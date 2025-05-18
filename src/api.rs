// api.rs - API服務模組，宣告子模組
//
// API服務模組提供外部接口，使用戶能夠與系統交互，實現：
// - RESTful API接口
// - WebSocket實時數據流
// - 用戶認證和授權
// - API路由和處理器


/// REST API實現
pub mod rest;
/// 認證和授權系統
pub mod auth;
/// API路由定義
pub mod routes;
/// API處理器模組
pub mod handlers;
