// Generic HTTP server using Axum
// Works on any cloud provider or self-hosted environment

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Import our generic handler
use polaroid_serverless::{PolaroidHandler, ServerlessHandler, ServerlessRequest, ServerlessResponse};

/// Convert axum::Request to ServerlessRequest
async fn to_serverless_request(
    req: axum::extract::Request,
) -> ServerlessRequest {
    use axum::body::Body;
    use axum::http::request::Parts;
    
    let (parts, body) = req.into_parts();
    let Parts {
        method,
        uri,
        headers,
        ..
    } = parts;

    // Extract headers
    let header_map: std::collections::HashMap<String, String> = headers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    // Extract query params
    let query_params: std::collections::HashMap<String, String> = uri
        .query()
        .map(|q| {
            url::form_urlencoded::parse(q.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_default();

    // Read body
    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .unwrap_or_default();

    ServerlessRequest {
        method: method.to_string(),
        path: uri.path().to_string(),
        headers: header_map,
        body: body_bytes.to_vec(),
        query_params,
    }
}

/// Convert ServerlessResponse to axum::Response
fn from_serverless_response(resp: ServerlessResponse) -> Response {
    let mut response = Response::builder().status(resp.status_code);

    for (key, value) in resp.headers {
        response = response.header(key, value);
    }

    response.body(axum::body::Body::from(resp.body)).unwrap()
}

/// Generic handler endpoint
async fn handle_request(
    State(handler): State<Arc<dyn ServerlessHandler>>,
    req: axum::extract::Request,
) -> Response {
    let serverless_req = to_serverless_request(req).await;

    match handler.handle_request(serverless_req).await {
        Ok(resp) => from_serverless_response(resp),
        Err(e) => {
            tracing::error!("Handler error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("{{\"error\": \"{}\"}}", e),
            )
                .into_response()
        }
    }
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        serde_json::json!({
            "status": "healthy",
            "version": env!("CARGO_PKG_VERSION")
        })
        .to_string(),
    )
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "polaroid_serverless=debug,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create handler
    let handler: Arc<dyn ServerlessHandler> = Arc::new(PolaroidHandler::new());

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/*path", post(handle_request))
        .route("/api/*path", get(handle_request))
        .layer(CorsLayer::permissive())
        .with_state(handler);

    // Get port from environment (cloud-agnostic)
    // Azure Functions uses FUNCTIONS_CUSTOMHANDLER_PORT, others use PORT
    let port = std::env::var("FUNCTIONS_CUSTOMHANDLER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .or_else(|| std::env::var("PORT").ok().and_then(|p| p.parse().ok()))
        .unwrap_or(8080);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("🚀 Polaroid HTTP server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
