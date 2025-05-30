//! Redis存儲模組
//!
//! 此模組提供Redis數據存儲和訪問功能。包括基本的Redis客戶端、連接池管理，
//! 以及特定業務操作如快取、發布/訂閱、任務佇列和分散式鎖的實現。

pub mod client;
pub mod operations;
pub mod pool;

#[cfg(test)]
pub mod test_config;

pub use client::*;
pub use operations::cache::*;
pub use pool::*;
#[cfg(test)]
mod tests {
    use super::pool::RedisPool;

    #[test]
    fn test_module_exports() {
        // 確保重要的導出可用
        async fn _ensure_redis_client_works(client: &super::client::Client) {
            let _ = client.test_connection().await;
        }

        async fn _ensure_redis_pool_works<P: RedisPool>(pool: &P) {
            let _ = pool.check_health().await;
            let _ = pool.pool_size();
        }
    }
}
