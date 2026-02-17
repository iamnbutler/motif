//! Debug server that accepts commands over a Unix domain socket.
//!
//! The server runs on a background thread and does NOT block the render loop.
//! Scene state is shared via an `Arc<Mutex<Option<SceneSnapshot>>>`.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::protocol::{DebugRequest, DebugResponse};
use crate::screenshot;
use crate::snapshot::{ColorInfo, OverlayQuad, SceneSnapshot};

/// Shared state for debug overlays injected via the debug CLI.
///
/// Overlays persist across frames until explicitly cleared.
#[derive(Debug, Default)]
pub struct DebugOverlays {
    pub quads: Vec<OverlayQuad>,
    next_id: u64,
}

impl DebugOverlays {
    /// Add a new overlay quad. Returns the assigned ID.
    pub fn add_quad(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: ColorInfo,
        border_color: ColorInfo,
        border_width: f32,
        corner_radius: f32,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.quads.push(OverlayQuad {
            id,
            x,
            y,
            w,
            h,
            color,
            border_color,
            border_width,
            corner_radius,
        });
        id
    }

    /// Remove a specific overlay by ID. Returns true if it was found and removed.
    pub fn remove(&mut self, id: u64) -> bool {
        let len_before = self.quads.len();
        self.quads.retain(|q| q.id != id);
        self.quads.len() < len_before
    }

    /// Clear all overlays. Returns the number removed.
    pub fn clear(&mut self) -> usize {
        let count = self.quads.len();
        self.quads.clear();
        count
    }
}

/// A debug server that embeds in a running motif app.
///
/// Creates a Unix domain socket and handles debug commands on a background thread.
pub struct DebugServer {
    socket_path: PathBuf,
    snapshot: Arc<Mutex<Option<SceneSnapshot>>>,
    window_id: Arc<Mutex<Option<u32>>>,
    overlays: Arc<Mutex<DebugOverlays>>,
    _shutdown: Arc<Mutex<bool>>,
}

impl DebugServer {
    /// Start a new debug server. Creates a Unix domain socket at
    /// `/tmp/motif-debug-{pid}.sock` and begins accepting connections
    /// on a background thread.
    pub fn new() -> std::io::Result<Self> {
        let pid = std::process::id();
        let socket_path = PathBuf::from(format!("/tmp/motif-debug-{pid}.sock"));
        Self::with_path(socket_path)
    }

    /// Start a debug server bound to a specific socket path.
    ///
    /// Useful for tests or when the default path is not suitable.
    pub fn with_path(socket_path: PathBuf) -> std::io::Result<Self> {
        // Clean up any stale socket from a previous run.
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)?;
        }

        let listener = UnixListener::bind(&socket_path)?;
        listener.set_nonblocking(true)?;

        let snapshot: Arc<Mutex<Option<SceneSnapshot>>> = Arc::new(Mutex::new(None));
        let window_id: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let overlays: Arc<Mutex<DebugOverlays>> = Arc::new(Mutex::new(DebugOverlays::default()));
        let shutdown = Arc::new(Mutex::new(false));

        let server_snapshot = Arc::clone(&snapshot);
        let server_window_id = Arc::clone(&window_id);
        let server_overlays = Arc::clone(&overlays);
        let server_shutdown = Arc::clone(&shutdown);

        thread::spawn(move || {
            Self::accept_loop(
                listener,
                server_snapshot,
                server_window_id,
                server_overlays,
                server_shutdown,
            );
        });

        eprintln!("[motif-debug] listening on {}", socket_path.display());

        Ok(Self {
            socket_path,
            snapshot,
            window_id,
            overlays,
            _shutdown: shutdown,
        })
    }

    /// Update the shared scene snapshot. Called from the render loop each frame.
    pub fn update_scene(&self, snapshot: SceneSnapshot) {
        if let Ok(mut guard) = self.snapshot.lock() {
            *guard = Some(snapshot);
        }
    }

    /// Set the window ID for native screenshot capture.
    /// Call this once after creating the window.
    pub fn set_window_id(&self, id: u32) {
        if let Ok(mut guard) = self.window_id.lock() {
            *guard = Some(id);
        }
    }

    /// Return the socket path for this server.
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Return a clone of the current debug overlays.
    ///
    /// Call this each frame to paint overlay quads on top of the scene.
    pub fn overlays(&self) -> Vec<OverlayQuad> {
        self.overlays
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .quads
            .clone()
    }

    fn accept_loop(
        listener: UnixListener,
        snapshot: Arc<Mutex<Option<SceneSnapshot>>>,
        window_id: Arc<Mutex<Option<u32>>>,
        overlays: Arc<Mutex<DebugOverlays>>,
        shutdown: Arc<Mutex<bool>>,
    ) {
        loop {
            if *shutdown.lock().unwrap_or_else(|e| e.into_inner()) {
                break;
            }

            match listener.accept() {
                Ok((stream, _addr)) => {
                    // On macOS, accepted connections inherit the listener's
                    // non-blocking mode. Set them back to blocking so the
                    // handler can read lines synchronously.
                    let _ = stream.set_nonblocking(false);

                    let snap = Arc::clone(&snapshot);
                    let wid = Arc::clone(&window_id);
                    let ovl = Arc::clone(&overlays);
                    thread::spawn(move || {
                        Self::handle_connection(stream, snap, wid, ovl);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No pending connection -- sleep briefly to avoid busy-spinning.
                    thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => {
                    eprintln!("[motif-debug] accept error: {e}");
                    thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    }

    fn handle_connection(
        stream: std::os::unix::net::UnixStream,
        snapshot: Arc<Mutex<Option<SceneSnapshot>>>,
        window_id: Arc<Mutex<Option<u32>>>,
        overlays: Arc<Mutex<DebugOverlays>>,
    ) {
        let reader = BufReader::new(match stream.try_clone() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[motif-debug] failed to clone stream: {e}");
                return;
            }
        });
        let mut writer = stream;

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };

            if line.is_empty() {
                continue;
            }

            let request: DebugRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    let resp = DebugResponse::err(0, -32700, format!("Parse error: {e}"));
                    let _ = writeln!(writer, "{}", serde_json::to_string(&resp).unwrap());
                    continue;
                }
            };

            let response = Self::dispatch(&request, &snapshot, &window_id, &overlays);
            let _ = writeln!(writer, "{}", serde_json::to_string(&response).unwrap());
        }
    }

    fn dispatch(
        request: &DebugRequest,
        snapshot: &Arc<Mutex<Option<SceneSnapshot>>>,
        window_id: &Arc<Mutex<Option<u32>>>,
        overlays: &Arc<Mutex<DebugOverlays>>,
    ) -> DebugResponse {
        match request.method.as_str() {
            "scene.stats" => {
                let guard = snapshot.lock().unwrap_or_else(|e| e.into_inner());
                match guard.as_ref() {
                    Some(snap) => DebugResponse::ok(request.id, snap.stats()),
                    None => DebugResponse::err(
                        request.id,
                        -32000,
                        "No scene snapshot available yet",
                    ),
                }
            }
            "scene.quads" => {
                let guard = snapshot.lock().unwrap_or_else(|e| e.into_inner());
                match guard.as_ref() {
                    Some(snap) => DebugResponse::ok(request.id, snap.quads_json()),
                    None => DebugResponse::err(
                        request.id,
                        -32000,
                        "No scene snapshot available yet",
                    ),
                }
            }
            "scene.text_runs" => {
                let guard = snapshot.lock().unwrap_or_else(|e| e.into_inner());
                match guard.as_ref() {
                    Some(snap) => DebugResponse::ok(request.id, snap.text_runs_json()),
                    None => DebugResponse::err(
                        request.id,
                        -32000,
                        "No scene snapshot available yet",
                    ),
                }
            }
            "screenshot" => Self::handle_screenshot(request, window_id),
            "debug.draw_quad" => Self::handle_draw_quad(request, overlays),
            "debug.clear" => Self::handle_clear(request, overlays),
            "debug.remove" => Self::handle_remove(request, overlays),
            "debug.list" => Self::handle_list(request, overlays),
            _ => DebugResponse::err(
                request.id,
                -32601,
                format!("Method not found: {}", request.method),
            ),
        }
    }

    fn handle_draw_quad(
        request: &DebugRequest,
        overlays: &Arc<Mutex<DebugOverlays>>,
    ) -> DebugResponse {
        let params = match &request.params {
            Some(p) => p,
            None => {
                return DebugResponse::err(
                    request.id,
                    -32602,
                    "debug.draw_quad requires params: { x, y, w, h, color: [r,g,b,a] }",
                )
            }
        };

        let x = params.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let y = params.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let w = params.get("w").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
        let h = params.get("h").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;

        let color = match params.get("color").and_then(|v| v.as_array()) {
            Some(arr) if arr.len() >= 4 => ColorInfo {
                r: arr[0].as_f64().unwrap_or(1.0) as f32,
                g: arr[1].as_f64().unwrap_or(0.0) as f32,
                b: arr[2].as_f64().unwrap_or(0.0) as f32,
                a: arr[3].as_f64().unwrap_or(1.0) as f32,
            },
            _ => ColorInfo {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        };

        let border_color = match params.get("border_color").and_then(|v| v.as_array()) {
            Some(arr) if arr.len() >= 4 => ColorInfo {
                r: arr[0].as_f64().unwrap_or(0.0) as f32,
                g: arr[1].as_f64().unwrap_or(0.0) as f32,
                b: arr[2].as_f64().unwrap_or(0.0) as f32,
                a: arr[3].as_f64().unwrap_or(0.0) as f32,
            },
            _ => ColorInfo {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
        };

        let border_width = params
            .get("border_width")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let corner_radius = params
            .get("corner_radius")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;

        let mut guard = overlays.lock().unwrap_or_else(|e| e.into_inner());
        let id = guard.add_quad(x, y, w, h, color, border_color, border_width, corner_radius);

        DebugResponse::ok(request.id, serde_json::json!({ "id": id }))
    }

    fn handle_clear(
        request: &DebugRequest,
        overlays: &Arc<Mutex<DebugOverlays>>,
    ) -> DebugResponse {
        let mut guard = overlays.lock().unwrap_or_else(|e| e.into_inner());
        let count = guard.clear();
        DebugResponse::ok(request.id, serde_json::json!({ "cleared": count }))
    }

    fn handle_remove(
        request: &DebugRequest,
        overlays: &Arc<Mutex<DebugOverlays>>,
    ) -> DebugResponse {
        let params = match &request.params {
            Some(p) => p,
            None => {
                return DebugResponse::err(
                    request.id,
                    -32602,
                    "debug.remove requires params: { id: <u64> }",
                )
            }
        };

        let id = match params.get("id").and_then(|v| v.as_u64()) {
            Some(id) => id,
            None => {
                return DebugResponse::err(
                    request.id,
                    -32602,
                    "debug.remove requires an \"id\" parameter (u64)",
                )
            }
        };

        let mut guard = overlays.lock().unwrap_or_else(|e| e.into_inner());
        let removed = guard.remove(id);
        DebugResponse::ok(request.id, serde_json::json!({ "removed": removed }))
    }

    fn handle_list(
        request: &DebugRequest,
        overlays: &Arc<Mutex<DebugOverlays>>,
    ) -> DebugResponse {
        let guard = overlays.lock().unwrap_or_else(|e| e.into_inner());
        let json = serde_json::to_value(&guard.quads).unwrap_or(serde_json::Value::Array(vec![]));
        DebugResponse::ok(request.id, json)
    }

    fn handle_screenshot(
        request: &DebugRequest,
        window_id: &Arc<Mutex<Option<u32>>>,
    ) -> DebugResponse {
        let params = match &request.params {
            Some(p) => p,
            None => {
                return DebugResponse::err(
                    request.id,
                    -32602,
                    "screenshot requires params: { \"path\": \"/path/to/output.png\" }",
                )
            }
        };

        let path = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => {
                return DebugResponse::err(
                    request.id,
                    -32602,
                    "screenshot requires a \"path\" parameter",
                )
            }
        };

        let wid = window_id.lock().unwrap_or_else(|e| e.into_inner());
        let wid = match *wid {
            Some(id) => id,
            None => {
                return DebugResponse::err(
                    request.id,
                    -32000,
                    "No window ID set — call set_window_id() on the debug server",
                )
            }
        };

        match screenshot::capture_window_to_png(wid, path) {
            Ok(()) => DebugResponse::ok(
                request.id,
                serde_json::json!({ "path": path }),
            ),
            Err(e) => DebugResponse::err(
                request.id,
                -32000,
                format!("Failed to capture screenshot: {e}"),
            ),
        }
    }
}

impl Drop for DebugServer {
    fn drop(&mut self) {
        // Signal shutdown to the accept loop.
        if let Ok(mut guard) = self._shutdown.lock() {
            *guard = true;
        }
        // Clean up the socket file.
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
        eprintln!("[motif-debug] server stopped, socket removed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Generate a unique socket path for each test to avoid collisions.
    fn test_socket_path() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        PathBuf::from(format!("/tmp/motif-debug-test-{pid}-{id}.sock"))
    }

    #[test]
    fn server_creates_socket_and_cleans_up() {
        let path = test_socket_path();
        let server = DebugServer::with_path(path.clone()).expect("server should start");
        assert!(path.exists(), "socket file should exist");

        drop(server);
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(!path.exists(), "socket file should be removed on drop");
    }

    #[test]
    fn server_responds_to_scene_stats_without_snapshot() {
        let path = test_socket_path();
        let _server = DebugServer::with_path(path.clone()).expect("server should start");

        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut stream = UnixStream::connect(&path).expect("should connect");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();

        let request = r#"{"method":"scene.stats","params":null,"id":1}"#;
        writeln!(stream, "{request}").unwrap();

        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).unwrap();

        let resp: DebugResponse = serde_json::from_str(&response_line).unwrap();
        assert_eq!(resp.id, 1);
        assert!(resp.error.is_some(), "should error when no snapshot available");
        assert_eq!(resp.error.unwrap().code, -32000);
    }

    #[test]
    fn server_responds_to_scene_stats_with_snapshot() {
        let path = test_socket_path();
        let server = DebugServer::with_path(path.clone()).expect("server should start");

        // Provide a snapshot.
        use motif_core::{DevicePoint, DeviceRect, DeviceSize, Quad, Scene, Srgba};
        let mut scene = Scene::new();
        scene.push_quad(Quad::new(
            DeviceRect::new(DevicePoint::new(0.0, 0.0), DeviceSize::new(10.0, 10.0)),
            Srgba::new(1.0, 0.0, 0.0, 1.0),
        ));
        let snap = SceneSnapshot::from_scene(&scene, (800.0, 600.0), 2.0);
        server.update_scene(snap);

        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut stream = UnixStream::connect(&path).expect("should connect");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();

        let request = r#"{"method":"scene.stats","params":null,"id":42}"#;
        writeln!(stream, "{request}").unwrap();

        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).unwrap();

        let resp: DebugResponse = serde_json::from_str(&response_line).unwrap();
        assert_eq!(resp.id, 42);
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["quad_count"], 1);
        assert_eq!(result["text_run_count"], 0);
        assert_eq!(result["viewport_size"][0], 800.0);
        assert_eq!(result["scale_factor"], 2.0);
    }

    #[test]
    fn server_handles_unknown_method() {
        let path = test_socket_path();
        let _server = DebugServer::with_path(path.clone()).expect("server should start");

        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut stream = UnixStream::connect(&path).expect("should connect");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();

        let request = r#"{"method":"nonexistent.method","params":null,"id":5}"#;
        writeln!(stream, "{request}").unwrap();

        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).unwrap();

        let resp: DebugResponse = serde_json::from_str(&response_line).unwrap();
        assert_eq!(resp.id, 5);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32601);
    }

    #[test]
    fn server_handles_malformed_json() {
        let path = test_socket_path();
        let _server = DebugServer::with_path(path.clone()).expect("server should start");

        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut stream = UnixStream::connect(&path).expect("should connect");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();

        writeln!(stream, "not valid json").unwrap();

        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).unwrap();

        let resp: DebugResponse = serde_json::from_str(&response_line).unwrap();
        assert_eq!(resp.id, 0);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32700);
    }

    #[test]
    fn server_responds_to_scene_quads() {
        let path = test_socket_path();
        let server = DebugServer::with_path(path.clone()).expect("server should start");

        use motif_core::{DevicePoint, DeviceRect, DeviceSize, Quad, Scene, Srgba};
        let mut scene = Scene::new();
        scene.push_quad(Quad::new(
            DeviceRect::new(DevicePoint::new(10.0, 20.0), DeviceSize::new(100.0, 50.0)),
            Srgba::new(1.0, 0.0, 0.0, 1.0),
        ));
        let snap = SceneSnapshot::from_scene(&scene, (800.0, 600.0), 1.0);
        server.update_scene(snap);

        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut stream = UnixStream::connect(&path).expect("should connect");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();

        let request = r#"{"method":"scene.quads","params":null,"id":10}"#;
        writeln!(stream, "{request}").unwrap();

        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).unwrap();

        let resp: DebugResponse = serde_json::from_str(&response_line).unwrap();
        assert_eq!(resp.id, 10);
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        let arr = result.as_array().expect("should be an array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["bounds"]["x"], 10.0);
    }

    #[test]
    fn server_responds_to_scene_text_runs() {
        let path = test_socket_path();
        let server = DebugServer::with_path(path.clone()).expect("server should start");

        // Empty scene -- no text runs.
        use motif_core::Scene;
        let scene = Scene::new();
        let snap = SceneSnapshot::from_scene(&scene, (800.0, 600.0), 1.0);
        server.update_scene(snap);

        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut stream = UnixStream::connect(&path).expect("should connect");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();

        let request = r#"{"method":"scene.text_runs","params":null,"id":11}"#;
        writeln!(stream, "{request}").unwrap();

        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).unwrap();

        let resp: DebugResponse = serde_json::from_str(&response_line).unwrap();
        assert_eq!(resp.id, 11);
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        let arr = result.as_array().expect("should be an array");
        assert!(arr.is_empty());
    }

    #[test]
    fn server_screenshot_without_window_id_returns_error() {
        let path = test_socket_path();
        let server = DebugServer::with_path(path.clone()).expect("server should start");

        use motif_core::Scene;
        let scene = Scene::new();
        let snap = SceneSnapshot::from_scene(&scene, (100.0, 100.0), 1.0);
        server.update_scene(snap);
        // Note: NOT calling set_window_id()

        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut stream = UnixStream::connect(&path).expect("should connect");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();

        let request = r#"{"method":"screenshot","params":{"path":"/tmp/test.png"},"id":20}"#;
        writeln!(stream, "{request}").unwrap();

        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).unwrap();

        let resp: DebugResponse = serde_json::from_str(&response_line).unwrap();
        assert_eq!(resp.id, 20);
        assert!(resp.error.is_some(), "should error without window_id");
        assert_eq!(resp.error.unwrap().code, -32000);
    }

    #[test]
    fn server_screenshot_missing_params_returns_error() {
        let path = test_socket_path();
        let server = DebugServer::with_path(path.clone()).expect("server should start");

        use motif_core::Scene;
        let scene = Scene::new();
        let snap = SceneSnapshot::from_scene(&scene, (800.0, 600.0), 1.0);
        server.update_scene(snap);

        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut stream = UnixStream::connect(&path).expect("should connect");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();

        // Send without params.
        let request = r#"{"method":"screenshot","params":null,"id":21}"#;
        writeln!(stream, "{request}").unwrap();

        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).unwrap();

        let resp: DebugResponse = serde_json::from_str(&response_line).unwrap();
        assert_eq!(resp.id, 21);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32602);
    }

    #[test]
    fn server_screenshot_with_invalid_window_id_returns_error() {
        let path = test_socket_path();
        let server = DebugServer::with_path(path.clone()).expect("server should start");

        use motif_core::Scene;
        let scene = Scene::new();
        let snap = SceneSnapshot::from_scene(&scene, (320.0, 240.0), 1.0);
        server.update_scene(snap);
        server.set_window_id(999999); // Invalid window ID

        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut stream = UnixStream::connect(&path).expect("should connect");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();

        let request = r#"{"method":"screenshot","params":{"path":"/tmp/test-invalid.png"},"id":22}"#;
        writeln!(stream, "{request}").unwrap();

        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).unwrap();

        let resp: DebugResponse = serde_json::from_str(&response_line).unwrap();
        assert_eq!(resp.id, 22);
        assert!(resp.error.is_some(), "invalid window ID should error");
    }

    // --- Overlay tests ---

    /// Helper: send a request and read the response on an existing connection.
    fn send_request(stream: &mut UnixStream, request: &str) -> DebugResponse {
        writeln!(stream, "{request}").unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut response_line = String::new();
        reader.read_line(&mut response_line).unwrap();
        serde_json::from_str(&response_line).unwrap()
    }

    #[test]
    fn draw_quad_returns_id() {
        let path = test_socket_path();
        let _server = DebugServer::with_path(path.clone()).expect("server should start");
        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut stream = UnixStream::connect(&path).expect("should connect");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();

        let req = r#"{"method":"debug.draw_quad","params":{"x":10,"y":20,"w":100,"h":50,"color":[1,0,0,1]},"id":1}"#;
        let resp = send_request(&mut stream, req);
        assert_eq!(resp.id, 1);
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["id"], 0, "first overlay should get id 0");
    }

    #[test]
    fn draw_quad_ids_auto_increment() {
        let path = test_socket_path();
        let _server = DebugServer::with_path(path.clone()).expect("server should start");
        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut stream = UnixStream::connect(&path).expect("should connect");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();

        let req1 = r#"{"method":"debug.draw_quad","params":{"x":0,"y":0,"w":10,"h":10,"color":[1,0,0,1]},"id":1}"#;
        let resp1 = send_request(&mut stream, req1);
        assert_eq!(resp1.result.unwrap()["id"], 0);

        let req2 = r#"{"method":"debug.draw_quad","params":{"x":0,"y":0,"w":10,"h":10,"color":[0,1,0,1]},"id":2}"#;
        let resp2 = send_request(&mut stream, req2);
        assert_eq!(resp2.result.unwrap()["id"], 1);
    }

    #[test]
    fn clear_returns_count() {
        let path = test_socket_path();
        let _server = DebugServer::with_path(path.clone()).expect("server should start");
        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut stream = UnixStream::connect(&path).expect("should connect");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();

        // Add two overlays.
        let draw = r#"{"method":"debug.draw_quad","params":{"x":0,"y":0,"w":10,"h":10,"color":[1,0,0,1]},"id":1}"#;
        send_request(&mut stream, draw);
        send_request(&mut stream, &draw.replace("\"id\":1", "\"id\":2"));

        // Clear them.
        let clear = r#"{"method":"debug.clear","params":null,"id":3}"#;
        let resp = send_request(&mut stream, clear);
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["cleared"], 2);

        // Clear again should give 0.
        let clear2 = r#"{"method":"debug.clear","params":null,"id":4}"#;
        let resp2 = send_request(&mut stream, clear2);
        assert_eq!(resp2.result.unwrap()["cleared"], 0);
    }

    #[test]
    fn remove_specific_overlay() {
        let path = test_socket_path();
        let _server = DebugServer::with_path(path.clone()).expect("server should start");
        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut stream = UnixStream::connect(&path).expect("should connect");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();

        // Add two overlays (ids 0 and 1).
        let draw1 = r#"{"method":"debug.draw_quad","params":{"x":0,"y":0,"w":10,"h":10,"color":[1,0,0,1]},"id":1}"#;
        send_request(&mut stream, draw1);
        let draw2 = r#"{"method":"debug.draw_quad","params":{"x":20,"y":20,"w":30,"h":30,"color":[0,1,0,1]},"id":2}"#;
        send_request(&mut stream, draw2);

        // Remove overlay 0.
        let remove = r#"{"method":"debug.remove","params":{"id":0},"id":3}"#;
        let resp = send_request(&mut stream, remove);
        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap()["removed"], true);

        // Remove again should return false.
        let remove_again = r#"{"method":"debug.remove","params":{"id":0},"id":4}"#;
        let resp2 = send_request(&mut stream, remove_again);
        assert_eq!(resp2.result.unwrap()["removed"], false);

        // List should show only overlay 1.
        let list = r#"{"method":"debug.list","params":null,"id":5}"#;
        let resp3 = send_request(&mut stream, list);
        let arr = resp3.result.unwrap();
        let arr = arr.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], 1);
    }

    #[test]
    fn list_returns_all_overlays() {
        let path = test_socket_path();
        let _server = DebugServer::with_path(path.clone()).expect("server should start");
        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut stream = UnixStream::connect(&path).expect("should connect");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();

        // Empty list.
        let list = r#"{"method":"debug.list","params":null,"id":1}"#;
        let resp = send_request(&mut stream, list);
        let arr = resp.result.unwrap();
        assert_eq!(arr.as_array().unwrap().len(), 0);

        // Add overlays.
        let draw1 = r#"{"method":"debug.draw_quad","params":{"x":10,"y":20,"w":100,"h":50,"color":[1,0,0,0.5]},"id":2}"#;
        send_request(&mut stream, draw1);
        let draw2 = r#"{"method":"debug.draw_quad","params":{"x":30,"y":40,"w":200,"h":100,"color":[0,1,0,1],"border_color":[1,1,1,1],"border_width":2,"corner_radius":8},"id":3}"#;
        send_request(&mut stream, draw2);

        // List again.
        let list2 = r#"{"method":"debug.list","params":null,"id":4}"#;
        let resp2 = send_request(&mut stream, list2);
        let arr2 = resp2.result.unwrap();
        let arr2 = arr2.as_array().unwrap();
        assert_eq!(arr2.len(), 2);
        assert_eq!(arr2[0]["x"], 10.0);
        assert_eq!(arr2[0]["y"], 20.0);
        assert_eq!(arr2[1]["border_width"], 2.0);
        assert_eq!(arr2[1]["corner_radius"], 8.0);
    }

    #[test]
    fn overlays_accessor_returns_current_state() {
        let path = test_socket_path();
        let server = DebugServer::with_path(path.clone()).expect("server should start");
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Initially empty.
        assert!(server.overlays().is_empty());

        // Add overlay via socket.
        let mut stream = UnixStream::connect(&path).expect("should connect");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();

        let draw = r#"{"method":"debug.draw_quad","params":{"x":5,"y":10,"w":50,"h":25,"color":[1,0,0,1]},"id":1}"#;
        send_request(&mut stream, draw);

        // Accessor should now return the overlay.
        let overlays = server.overlays();
        assert_eq!(overlays.len(), 1);
        assert_eq!(overlays[0].x, 5.0);
        assert_eq!(overlays[0].y, 10.0);
        assert_eq!(overlays[0].w, 50.0);
        assert_eq!(overlays[0].h, 25.0);
    }
}
