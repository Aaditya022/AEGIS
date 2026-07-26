use hyper::{header, HeaderMap, Method, Uri};

#[derive(Debug, Clone, PartialEq)]
pub enum DetectedProtocol {
    Http,
    Https,
    Grpc,
    Mcp, // Model Context Protocol (Anthropic)
    A2a, // Agent-to-Agent (Google)
    Acp, // Agent Communication Protocol
    Anp, // Agent Network Protocol
    #[allow(dead_code)]
    Unknown,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProtocolInfo {
    pub protocol: DetectedProtocol,
    pub is_tls: bool,
    pub content_type: Option<String>,
    pub is_streaming: bool,
}

pub fn detect_protocol(method: &Method, uri: &Uri, headers: &HeaderMap) -> ProtocolInfo {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let is_tls = uri.scheme_str().is_some_and(|s| s == "https");

    let is_streaming = headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/event-stream") || s.contains("application/x-ndjson"))
        .unwrap_or(false);

    let protocol = if content_type.as_deref() == Some("application/grpc")
        || method == Method::POST && uri.path().starts_with('/') && uri.path().contains('.')
    {
        DetectedProtocol::Grpc
    } else if headers.get("mcp-version").is_some()
        || headers.get("x-mcp-version").is_some()
        || content_type.as_deref() == Some("application/vnd.mcp+json")
    {
        DetectedProtocol::Mcp
    } else if headers.get("a2a-version").is_some()
        || headers.get("x-a2a-version").is_some()
        || content_type.as_deref() == Some("application/vnd.a2a+json")
    {
        DetectedProtocol::A2a
    } else if content_type.as_deref() == Some("application/vnd.acp+json") {
        DetectedProtocol::Acp
    } else if content_type.as_deref() == Some("application/vnd.anp+json") {
        DetectedProtocol::Anp
    } else if is_tls {
        DetectedProtocol::Https
    } else {
        DetectedProtocol::Http
    };

    ProtocolInfo {
        protocol,
        is_tls,
        content_type,
        is_streaming,
    }
}
