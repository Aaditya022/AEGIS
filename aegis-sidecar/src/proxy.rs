use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tracing::{debug, error, info, Instrument};

use crate::middleware::GovernanceOutcome;
use crate::AppState;

type ProxyResult = std::result::Result<Response<Full<Bytes>>, hyper::Error>;

pub struct Proxy;

impl Proxy {
    pub async fn run(listener: TcpListener, state: Arc<AppState>) {
        info!("Proxy listening");
        loop {
            let (stream, peer) = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                    error!(error = %e, "Accept error");
                    continue;
                }
            };

            let sidecar_id = state.config.read().await.sidecar_id.clone();
            let state = state.clone();
            tokio::spawn(async move {
                let span = tracing::info_span!(
                    "connection",
                    peer = %peer,
                    sidecar_id = %sidecar_id,
                );
                async {
                    let io = TokioIo::new(stream);
                    let svc = service_fn(move |req| handle_connection(req, state.clone()));
                    if let Err(e) = hyper::server::conn::http1::Builder::new()
                        .serve_connection(io, svc)
                        .with_upgrades()
                        .await
                    {
                        if !e.to_string().contains("connection closed") {
                            debug!(error = %e, "Connection error");
                        }
                    }
                }
                .instrument(span)
                .await
            });
        }
    }
}

async fn handle_connection(req: Request<Incoming>, state: Arc<AppState>) -> ProxyResult {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();
    let span = tracing::info_span!("request", method = %method, uri = %uri);

    async {
        let start = std::time::Instant::now();

        let _protocol = crate::protocol::detect_protocol(&method, &uri, &headers);

        let agent_id = match crate::middleware::extract_agent_id(&headers) {
            Some(id) => id,
            None => {
                state.metrics.inc_denied("missing_identity");
                return deny(
                    StatusCode::UNAUTHORIZED,
                    "missing agent identity",
                    "identity",
                );
            }
        };

        let env = std::env::var("AEGIS_ENV").unwrap_or_default();
        let trace_id = headers
            .get("x-aegis-trace-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let ctx = aegis_common::types::PolicyContext {
            agent_id: agent_id.clone(),
            operation: format!("{method} {uri}"),
            resource: uri.to_string(),
            environment: env.clone(),
            recursion_depth: 0,
            budget_consumed_usd: state.cost.current_cost(),
            trace_id: trace_id.clone(),
            extra: Default::default(),
        };

        match crate::middleware::run_governance(&state, &ctx, &headers).await {
            GovernanceOutcome::Allow => {
                let elapsed = start.elapsed();

                let upstream = match build_upstream_url(&method, &uri) {
                    Some(u) => u,
                    None => {
                        return Ok(Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Full::new(Bytes::from("no upstream target")))
                            .unwrap())
                    }
                };

                let body_bytes = req
                    .collect()
                    .await
                    .map(|b| b.to_bytes())
                    .unwrap_or_default();

                let hyper_client =
                    HyperClient::builder(hyper_util::rt::TokioExecutor::new()).build_http();

                let upstream_req = Request::builder()
                    .method(&method)
                    .uri(&upstream)
                    .body(Full::new(body_bytes))
                    .unwrap();

                match hyper_client.request(upstream_req).await {
                    Ok(up_resp) => {
                        let status = up_resp.status();
                        let upstream_body = up_resp
                            .collect()
                            .await
                            .map(|b| b.to_bytes())
                            .unwrap_or_default();
                        let body_str = String::from_utf8_lossy(&upstream_body).to_string();

                        state.metrics.record_request(
                            &format!("{method}"),
                            uri.path(),
                            elapsed,
                            status.as_u16(),
                        );
                        state.metrics.inc_allowed("policy");
                        state
                            .audit
                            .log_event(
                                &agent_id,
                                &format!("{method} {uri}"),
                                &uri.to_string(),
                                "ALLOW",
                                &trace_id,
                            )
                            .await;

                        let body_bytes = Bytes::from(body_str);
                        Ok(Response::builder()
                            .status(status)
                            .header("x-aegis-decision", "ALLOW")
                            .header("x-aegis-trace-id", &trace_id)
                            .body(Full::new(body_bytes))
                            .unwrap())
                    }
                    Err(e) => {
                        error!(error = %e, upstream = %upstream, "Upstream request failed");
                        Ok(Response::builder()
                            .status(StatusCode::BAD_GATEWAY)
                            .body(Full::new(Bytes::from(format!("upstream error: {e}"))))
                            .unwrap())
                    }
                }
            }
            GovernanceOutcome::Deny { reason, category } => {
                state.metrics.inc_denied(&category);
                state.metrics.record_request(
                    &format!("{method}"),
                    uri.path(),
                    start.elapsed(),
                    StatusCode::FORBIDDEN.as_u16(),
                );
                state
                    .audit
                    .log_event(
                        &agent_id,
                        &format!("{method} {uri}"),
                        &format!("{uri}"),
                        "DENY",
                        &trace_id,
                    )
                    .await;
                deny(StatusCode::FORBIDDEN, &reason, &category)
            }
            GovernanceOutcome::Escalate { reason, category } => {
                state.metrics.inc_denied(&category);
                state.metrics.record_request(
                    &format!("{method}"),
                    uri.path(),
                    start.elapsed(),
                    StatusCode::FORBIDDEN.as_u16(),
                );
                state
                    .audit
                    .log_event(
                        &agent_id,
                        &format!("{method} {uri}"),
                        &uri.to_string(),
                        "ESCALATE",
                        &trace_id,
                    )
                    .await;
                deny(StatusCode::FORBIDDEN, &reason, &category)
            }
        }
    }
    .instrument(span)
    .await
}

fn build_upstream_url(method: &Method, uri: &hyper::Uri) -> Option<String> {
    let host = uri.host()?;
    let port = uri
        .port_u16()
        .unwrap_or(if method == Method::CONNECT { 443 } else { 80 });
    let path = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
    Some(format!("http://{host}:{port}{path}"))
}

fn deny(status: StatusCode, reason: &str, category: &str) -> ProxyResult {
    let body = Bytes::from(
        serde_json::json!({
            "decision": "DENY",
            "reason": reason,
            "category": category,
        })
        .to_string(),
    );
    Ok(Response::builder()
        .status(status)
        .header("x-aegis-decision", "DENY")
        .header("x-aegis-reason", reason)
        .header("x-aegis-category", category)
        .body(Full::new(body))
        .unwrap())
}
