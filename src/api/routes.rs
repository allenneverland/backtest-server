use axum::Router;

pub mod data;
pub mod backtest;
pub mod system;

pub fn api_routes() -> Router {
    Router::new()
        .merge(data::routes())
        .merge(backtest::routes())
        .merge(system::routes())
}