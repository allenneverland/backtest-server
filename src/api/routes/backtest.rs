use axum::{
    routing::{post},
    Router,
};
use crate::api::handlers::backtest;

pub fn routes() -> Router {
    Router::new()
        .route("/backtest/create", post(backtest::create_backtest))
}