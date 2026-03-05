use acp_core::*;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::server::AcpServer;

/// Serve ACP over stdin/stdout (standard MCP transport).
pub async fn serve_stdio(server: &AcpServer) -> Result<(), AcpError> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    tracing::info!("ACP server ready (stdio transport)");

    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| AcpError::Internal(e.to_string()))?
    {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let error_response = JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                    id: None,
                };
                let json = serde_json::to_string(&error_response).unwrap();
                stdout
                    .write_all(json.as_bytes())
                    .await
                    .map_err(|e| AcpError::Internal(e.to_string()))?;
                stdout
                    .write_all(b"\n")
                    .await
                    .map_err(|e| AcpError::Internal(e.to_string()))?;
                stdout
                    .flush()
                    .await
                    .map_err(|e| AcpError::Internal(e.to_string()))?;
                continue;
            }
        };

        let response = server.handle_request(request).await;

        let json =
            serde_json::to_string(&response).map_err(|e| AcpError::Internal(e.to_string()))?;
        stdout
            .write_all(json.as_bytes())
            .await
            .map_err(|e| AcpError::Internal(e.to_string()))?;
        stdout
            .write_all(b"\n")
            .await
            .map_err(|e| AcpError::Internal(e.to_string()))?;
        stdout
            .flush()
            .await
            .map_err(|e| AcpError::Internal(e.to_string()))?;
    }

    Ok(())
}
