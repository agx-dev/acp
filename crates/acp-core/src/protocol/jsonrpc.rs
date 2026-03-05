use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
    pub id: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    pub fn new(method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
            id: Some(serde_json::Value::Number(1.into())),
        }
    }

    pub fn notification(method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
            id: None,
        }
    }
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(id: Option<serde_json::Value>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    // Standard JSON-RPC errors.
    pub fn parse_error() -> Self {
        Self::new(-32700, "Parse error")
    }

    pub fn invalid_request() -> Self {
        Self::new(-32600, "Invalid request")
    }

    pub fn method_not_found(method: &str) -> Self {
        Self::new(-32601, format!("Method not found: {}", method))
    }

    pub fn invalid_params(msg: &str) -> Self {
        Self::new(-32602, format!("Invalid params: {}", msg))
    }

    pub fn internal_error(msg: &str) -> Self {
        Self::new(-32603, format!("Internal error: {}", msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_serialization() {
        let req = JsonRpcRequest::new("acp.memory.recall", serde_json::json!({"query": "test"}));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"acp.memory.recall\""));
    }

    #[test]
    fn response_success() {
        let resp = JsonRpcResponse::success(
            Some(serde_json::Value::Number(1.into())),
            serde_json::json!({"entries": []}),
        );
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn response_error() {
        let resp = JsonRpcResponse::error(
            Some(serde_json::Value::Number(1.into())),
            JsonRpcError::method_not_found("acp.foo"),
        );
        assert!(resp.result.is_none());
        assert_eq!(resp.error.as_ref().unwrap().code, -32601);
    }
}
