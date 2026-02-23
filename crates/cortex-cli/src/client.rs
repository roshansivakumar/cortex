use std::path::Path;

use cortex_core::error::{CortexError, Result};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper_util::rt::TokioIo;

/// Read the port number from the socket file.
fn read_port(socket_path: &Path) -> Result<u16> {
    let content = std::fs::read_to_string(socket_path).map_err(|_| {
        CortexError::Config(
            "Cannot find running daemon. Is it running? Try: cortex start".to_string(),
        )
    })?;
    content
        .trim()
        .parse()
        .map_err(|_| CortexError::Config("Invalid port in socket file".to_string()))
}

/// Send a GET request to the daemon via TCP.
pub async fn get(socket_path: &Path, path: &str) -> Result<String> {
    let port = read_port(socket_path)?;

    let stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .map_err(|e| {
            CortexError::Config(format!(
                "Cannot connect to daemon on port {port}: {e}\nIs the daemon running? Try: cortex start"
            ))
        })?;

    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
        .await
        .map_err(|e| CortexError::Config(format!("HTTP handshake failed: {e}")))?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Connection error: {e}");
        }
    });

    let req = hyper::Request::builder()
        .method("GET")
        .uri(path)
        .header("Host", "localhost")
        .body(Full::new(Bytes::new()))
        .map_err(|e| CortexError::Config(format!("Request build error: {e}")))?;

    let res = sender
        .send_request(req)
        .await
        .map_err(|e| CortexError::Config(format!("Request failed: {e}")))?;

    let body = res
        .into_body()
        .collect()
        .await
        .map_err(|e| CortexError::Config(format!("Body read error: {e}")))?
        .to_bytes();

    String::from_utf8(body.to_vec())
        .map_err(|e| CortexError::Config(format!("Invalid UTF-8: {e}")))
}

/// Send a POST request to the daemon via TCP.
pub async fn post(socket_path: &Path, path: &str, body: &str) -> Result<String> {
    let port = read_port(socket_path)?;

    let stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .map_err(|e| {
            CortexError::Config(format!(
                "Cannot connect to daemon on port {port}: {e}\nIs the daemon running? Try: cortex start"
            ))
        })?;

    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
        .await
        .map_err(|e| CortexError::Config(format!("HTTP handshake failed: {e}")))?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Connection error: {e}");
        }
    });

    let req = hyper::Request::builder()
        .method("POST")
        .uri(path)
        .header("Host", "localhost")
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .map_err(|e| CortexError::Config(format!("Request build error: {e}")))?;

    let res = sender
        .send_request(req)
        .await
        .map_err(|e| CortexError::Config(format!("Request failed: {e}")))?;

    let response_body = res
        .into_body()
        .collect()
        .await
        .map_err(|e| CortexError::Config(format!("Body read error: {e}")))?
        .to_bytes();

    String::from_utf8(response_body.to_vec())
        .map_err(|e| CortexError::Config(format!("Invalid UTF-8: {e}")))
}
