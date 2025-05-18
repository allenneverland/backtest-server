// src/api/routes/system.rs
use axum::{
    routing::get,
    Router,
};
use crate::api::handlers::system;

pub fn routes() -> Router {
    Router::new()
        .route("/system/health", get(system::health))
}