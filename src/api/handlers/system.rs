use axum::{
    extract::Json,
    response::IntoResponse,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
    version: String,
}

pub async fn health() -> impl IntoResponse {
    let health_response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    
    Json(health_response)
}
