use axum::{
    routing::{get, post},
    Router,
};
use crate::api::handlers::data;

pub fn routes() -> Router {
    Router::new()
        .route("/data/symbols", get(data::list_symbols))
        .route("/data/symbols/{symbol}", get(data::get_symbol_data))
        .route("/data/import_folder", post(data::import_folder))
}