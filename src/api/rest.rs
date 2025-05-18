// src/api/rest.rs
use axum::{Router, middleware};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::{
    trace::{TraceLayer, DefaultMakeSpan, DefaultOnResponse},
    cors::CorsLayer,
    compression::CompressionLayer,
    timeout::TimeoutLayer,
};
use std::time::Duration;
use anyhow::Result;
use tracing::info;

use crate::config::{ServerConfig, RestApiConfig};
use super::{
    auth::{ApiAuth, auth_middleware},
    routes::api_routes,  // 導入模組化的路由
};

pub struct RestApi {
    server_config: ServerConfig,
    api_config: RestApiConfig,
}

impl RestApi {
    pub fn new(server_config: ServerConfig, api_config: RestApiConfig) -> Self {
        Self {
            server_config,
            api_config,
        }
    }
    
    pub async fn start(self) -> Result<()> {
        // 初始化認證
        let auth = ApiAuth::new(
            self.api_config.api_key.clone(),
            self.api_config.secret_key.clone(),
        );
        
        // 建立應用
        let app = self.build_app(auth)?;
        
        // 解析地址
        let addr = SocketAddr::from((
            self.server_config.host.parse::<std::net::IpAddr>()?,
            self.server_config.port,
        ));
        
        info!("Starting REST API server on {}", addr);
        
        // 啟動服務器
        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        
        Ok(())
    }

    fn build_app(&self, auth: ApiAuth) -> Result<Router> {
        // 使用模組化的路由
        let api_router = api_routes();

        // 建立應用並逐層添加中間件
        let app = Router::new()
            .nest(&self.api_config.base_path, api_router)
            // 追蹤層，取代自定義錯誤處理中間件
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().include_headers(true))
                    .on_response(DefaultOnResponse::new().include_headers(true))
            )
            // CORS
            .layer(self.build_cors_layer())
            // 壓縮
            .layer(CompressionLayer::new())
            // 超時設置
            .layer(TimeoutLayer::new(Duration::from_secs(self.api_config.request_timeout)))
            // 認證層
            .layer(middleware::from_fn_with_state(auth.clone(), auth_middleware));

        Ok(app)
    }
    
    fn build_cors_layer(&self) -> CorsLayer {
        let cors = CorsLayer::new()
            .allow_methods(vec![
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
            ])
            .allow_headers(vec![
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
                axum::http::HeaderName::from_static("x-api-key"),
                axum::http::HeaderName::from_static("x-timestamp"),
                axum::http::HeaderName::from_static("x-signature"),
            ]);
        
        // 根據配置設置允許的來源
        if self.api_config.cors_allow_all {
            cors.allow_origin(tower_http::cors::Any)
        } else {
            cors.allow_origin(
                self.api_config.cors_origins
                    .iter()
                    .map(|s| s.parse().unwrap())
                    .collect::<Vec<_>>()
            )
        }
    }
}
