//! Wire protocol for the motif debug server.
//!
//! Uses JSON-RPC 2.0 style messages over newline-delimited JSON.

use serde::{Deserialize, Serialize};

/// A debug request from a client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DebugRequest {
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub id: u64,
}

/// A debug response sent back to the client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DebugResponse {
    pub result: Option<serde_json::Value>,
    pub error: Option<DebugError>,
    pub id: u64,
}

/// An error included in a debug response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DebugError {
    pub code: i32,
    pub message: String,
}

impl DebugResponse {
    /// Create a successful response with a JSON result.
    pub fn ok(id: u64, result: serde_json::Value) -> Self {
        Self {
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response.
    pub fn err(id: u64, code: i32, message: impl Into<String>) -> Self {
        Self {
            result: None,
            error: Some(DebugError {
                code,
                message: message.into(),
            }),
            id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_round_trip() {
        let req = DebugRequest {
            method: "scene.stats".into(),
            params: None,
            id: 1,
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: DebugRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, deserialized);
    }

    #[test]
    fn request_with_params_round_trip() {
        let req = DebugRequest {
            method: "scene.quads".into(),
            params: Some(json!({"filter": "visible"})),
            id: 42,
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: DebugRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, deserialized);
    }

    #[test]
    fn response_ok_round_trip() {
        let resp = DebugResponse::ok(1, json!({"quad_count": 10}));
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: DebugResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, deserialized);
        assert!(deserialized.error.is_none());
        assert_eq!(deserialized.result.unwrap()["quad_count"], 10);
    }

    #[test]
    fn response_err_round_trip() {
        let resp = DebugResponse::err(2, -32601, "Method not found");
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: DebugResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, deserialized);
        assert!(deserialized.result.is_none());
        let err = deserialized.error.unwrap();
        assert_eq!(err.code, -32601);
        assert_eq!(err.message, "Method not found");
    }

    #[test]
    fn request_deserializes_from_raw_json() {
        let raw = r#"{"method":"scene.stats","params":null,"id":7}"#;
        let req: DebugRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.method, "scene.stats");
        assert_eq!(req.id, 7);
        assert_eq!(req.params, None);
    }

    #[test]
    fn newline_delimited_stream() {
        let req1 = DebugRequest {
            method: "scene.stats".into(),
            params: None,
            id: 1,
        };
        let req2 = DebugRequest {
            method: "scene.quads".into(),
            params: None,
            id: 2,
        };
        let stream = format!(
            "{}\n{}\n",
            serde_json::to_string(&req1).unwrap(),
            serde_json::to_string(&req2).unwrap()
        );

        let requests: Vec<DebugRequest> = stream
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();

        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].method, "scene.stats");
        assert_eq!(requests[1].method, "scene.quads");
    }
}
