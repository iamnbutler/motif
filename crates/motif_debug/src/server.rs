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
use crate::snapshot::SceneSnapshot;

/// A debug server that embeds in a running motif app.
///
/// Creates a Unix domain socket and handles debug commands on a background thread.
pub struct DebugServer {
    socket_path: PathBuf,
    snapshot: Arc<Mutex<Option<SceneSnapshot>>>,
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
        let shutdown = Arc::new(Mutex::new(false));

        let server_snapshot = Arc::clone(&snapshot);
        let server_shutdown = Arc::clone(&shutdown);

        thread::spawn(move || {
            Self::accept_loop(listener, server_snapshot, server_shutdown);
        });

        eprintln!("[motif-debug] listening on {}", socket_path.display());

        Ok(Self {
            socket_path,
            snapshot,
            _shutdown: shutdown,
        })
    }

    /// Update the shared scene snapshot. Called from the render loop each frame.
    pub fn update_scene(&self, snapshot: SceneSnapshot) {
        if let Ok(mut guard) = self.snapshot.lock() {
            *guard = Some(snapshot);
        }
    }

    /// Return the socket path for this server.
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    fn accept_loop(
        listener: UnixListener,
        snapshot: Arc<Mutex<Option<SceneSnapshot>>>,
        shutdown: Arc<Mutex<bool>>,
    ) {
        loop {
            if *shutdown.lock().unwrap_or_else(|e| e.into_inner()) {
                break;
            }

            match listener.accept() {
                Ok((stream, _addr)) => {
                    let snap = Arc::clone(&snapshot);
                    thread::spawn(move || {
                        Self::handle_connection(stream, snap);
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

            let response = Self::dispatch(&request, &snapshot);
            let _ = writeln!(writer, "{}", serde_json::to_string(&response).unwrap());
        }
    }

    fn dispatch(
        request: &DebugRequest,
        snapshot: &Arc<Mutex<Option<SceneSnapshot>>>,
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
            _ => DebugResponse::err(
                request.id,
                -32601,
                format!("Method not found: {}", request.method),
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
}
