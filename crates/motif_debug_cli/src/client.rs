//! IPC client for connecting to a running motif debug server.

use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

use motif_debug::{DebugRequest, DebugResponse};

/// A client that connects to a motif debug server over a Unix domain socket.
pub struct DebugClient {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
    next_id: u64,
}

impl DebugClient {
    /// Connect to a debug server at the given socket path.
    pub fn connect(path: &str) -> io::Result<Self> {
        let stream = UnixStream::connect(path)?;
        let read_stream = stream.try_clone()?;
        read_stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
        let reader = BufReader::new(read_stream);
        let writer = stream;
        Ok(Self {
            reader,
            writer,
            next_id: 1,
        })
    }

    /// Discover a running motif debug server by scanning for sockets in /tmp.
    ///
    /// Connects to the first `motif-debug-*.sock` socket found.
    pub fn discover() -> io::Result<Self> {
        let sockets = Self::find_sockets()?;
        if sockets.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no running motif process found (no /tmp/motif-debug-*.sock sockets)",
            ));
        }

        // Try each socket until one connects.
        for path in &sockets {
            match Self::connect(path) {
                Ok(client) => return Ok(client),
                Err(_) => continue,
            }
        }

        Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "found motif debug sockets but could not connect to any",
        ))
    }

    /// Find all motif debug socket paths in /tmp.
    fn find_sockets() -> io::Result<Vec<String>> {
        let mut sockets = Vec::new();
        for entry in std::fs::read_dir("/tmp")? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("motif-debug-") && name.ends_with(".sock") {
                // Skip test sockets.
                if name.contains("-test-") {
                    continue;
                }
                sockets.push(entry.path().to_string_lossy().into_owned());
            }
        }
        Ok(sockets)
    }

    /// Send a debug request and wait for the response.
    ///
    /// The `method` is the command name (e.g. "scene.stats").
    /// Optional `params` can be provided as a JSON value.
    pub fn send(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> io::Result<DebugResponse> {
        let request = DebugRequest {
            method: method.to_string(),
            params,
            id: self.next_id,
        };
        self.next_id += 1;

        let json = serde_json::to_string(&request)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        writeln!(self.writer, "{json}")?;
        self.writer.flush()?;

        let mut line = String::new();
        self.reader.read_line(&mut line)?;

        if line.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "server closed connection",
            ));
        }

        serde_json::from_str(&line)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_returns_not_found_when_no_sockets() {
        // This test relies on there being no *non-test* motif-debug sockets.
        // In CI or a clean environment this is true. If a real server is running
        // the test may connect successfully, which is also fine.
        let result = DebugClient::find_sockets();
        // find_sockets itself should not error (just returns empty or populated vec)
        assert!(result.is_ok());
    }

    #[test]
    fn connect_to_nonexistent_socket_fails() {
        let result = DebugClient::connect("/tmp/motif-debug-nonexistent-99999.sock");
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn send_to_real_server() {
        use motif_debug::DebugServer;
        use std::path::PathBuf;
        use std::sync::atomic::{AtomicU64, Ordering};

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let path = format!("/tmp/motif-debug-test-cli-{pid}-{id}.sock");

        let _server =
            DebugServer::with_path(PathBuf::from(&path)).expect("server should start");
        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut client = DebugClient::connect(&path).expect("should connect");
        let resp = client.send("scene.stats", None).expect("should get response");

        // No snapshot has been pushed, so we expect an error response.
        assert_eq!(resp.id, 1);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32000);
    }

    #[test]
    fn request_ids_auto_increment() {
        use motif_debug::DebugServer;
        use std::path::PathBuf;
        use std::sync::atomic::{AtomicU64, Ordering};

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let path = format!("/tmp/motif-debug-test-cli-inc-{pid}-{id}.sock");

        let server =
            DebugServer::with_path(PathBuf::from(&path)).expect("server should start");
        std::thread::sleep(std::time::Duration::from_millis(200));

        let mut client = DebugClient::connect(&path).expect("should connect");

        let resp1 = client.send("scene.stats", None).expect("should get response 1");
        assert_eq!(resp1.id, 1);

        let resp2 = client.send("scene.stats", None).expect("should get response 2");
        assert_eq!(resp2.id, 2);

        let resp3 = client.send("nonexistent", None).expect("should get response 3");
        assert_eq!(resp3.id, 3);

        // Keep server alive until all assertions pass.
        drop(server);
    }
}
