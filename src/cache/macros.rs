/// 巨集模組：生成快取操作方法，減少程式碼重複
///
/// 此模組使用 Rust 巨集在編譯時生成快取相關的方法，
/// 保持零運行時開銷的同時大幅減少程式碼重複。
/// 為特定資料類型生成完整的快取操作方法集
///
/// 此巨集會生成以下方法：
/// - get_{type_name}() - 獲取單筆資料
/// - set_{type_name}() - 設置單筆資料
/// - set_{type_name}_arc() - 設置 Arc 包裝的資料  
/// - get_or_compute_{type_name}() - 獲取或計算資料
/// - get_{type_name}_batch() - 批量獲取資料
/// - warm_{type_name}_cache() - 預熱快取
/// - set_{type_name}_batch() - 批量設置資料
/// - set_{type_name}_batch_optimized() - 優化的批量設置
/// - set_{type_name}_batch_pipeline() - Pipeline 批量設置
/// - get_{type_name}_batch_buffered() - 使用緩衝區的批量獲取
///
/// # 參數
/// * `$data_type` - 資料類型 (如 MinuteBar, DbTick)
/// * `$cache_field` - 對應的快取欄位名稱 (如 minute_bars_cache, ticks_cache)
/// * `$type_name` - 方法名稱中使用的類型名稱 (如 minute_bars, ticks)
/// * `$display_name` - 用於監控指標的顯示名稱 (如 "minute_bars", "ticks")
macro_rules! impl_cache_methods {
    ($data_type:ty, $cache_field:ident, $type_name:ident, $display_name:expr) => {
        paste::paste! {
            /// 獲取資料 - 高性能版本
            ///
            /// 使用 u64 hash 作為內存快取鍵，提升查找性能。
            /// 直接返回 Arc<Vec<T>>，避免深拷貝整個 Vec。
            pub async fn [<get_ $type_name>](
                &self,
                key: &str,
            ) -> Result<Option<Arc<Vec<$data_type>>>, CacheError> {
                self.get_cached_data::<$data_type>(key, &self.$cache_field).await
            }

            /// 設置資料 - 高性能版本
            ///
            /// # 快取一致性策略
            /// - 先更新 Redis（持久化層）
            /// - 只有在 Redis 更新成功後才更新內存快取
            /// - 如果 Redis 更新失敗，內存快取保持不變
            pub async fn [<set_ $type_name>](
                &self,
                key: &str,
                data: &Vec<$data_type>,
            ) -> Result<(), CacheError> {
                self.set_cached_data::<$data_type>(key, data, &self.$cache_field).await
            }

            /// 設置資料 (Arc 版本) - 避免不必要的複製
            ///
            /// 當調用者已經擁有 Arc<Vec<T>> 時，可以使用此方法避免額外的 clone。
            ///
            /// # 快取一致性策略
            /// - 先更新 Redis（持久化層）
            /// - 只有在 Redis 更新成功後才更新內存快取
            /// - 如果 Redis 更新失敗，內存快取保持不變
            pub async fn [<set_ $type_name _arc>](
                &self,
                key: &str,
                data: Arc<Vec<$data_type>>,
            ) -> Result<(), CacheError> {
                self.set_cached_data_arc::<$data_type>(key, data, &self.$cache_field).await
            }

            /// 獲取或計算資料 - 使用 Arc 避免不必要的複製
            ///
            /// 如果快取命中，返回共享的數據；如果需要計算，返回新創建的數據。
            /// 這可以顯著減少記憶體分配和複製操作。
            ///
            /// # 快取一致性策略
            /// - 先更新 Redis（持久化層）
            /// - 只有在 Redis 更新成功後才更新內存快取
            pub async fn [<get_or_compute_ $type_name>]<F>(
                &self,
                key: &str,
                compute: F,
            ) -> Result<Arc<Vec<$data_type>>, CacheError>
            where
                F: FnOnce() -> Vec<$data_type>,
            {
                let hash = Self::hash_key(key);

                // 如果快取命中，返回 Arc（使用 hash）
                if let Some(arc_data) = self.$cache_field.get(&hash).await {
                    self.record_metric::<$data_type>(MetricType::Hit { layer: "memory" }, None);
                    return Ok(arc_data);
                }

                // 嘗試從 Redis 獲取
                match self.redis_cache.get::<_, Vec<$data_type>>(key).await {
                    Ok(data) => {
                        self.record_metric::<$data_type>(MetricType::Hit { layer: "redis" }, None);
                        let arc_data = Arc::new(data);
                        self.$cache_field.insert(hash, arc_data.clone()).await;

                        // 更新 hash 映射
                        self.key_mapping.write().await.insert(hash, key.to_string());

                        Ok(arc_data)
                    }
                    Err(CacheError::CacheMiss(_)) => {
                        // 計算新數據
                        self.record_metric::<$data_type>(MetricType::Miss, None);
                        let data = compute();
                        let arc_data = Arc::new(data);

                        // 先嘗試更新 Redis
                        match self
                            .redis_cache
                            .set(key, &*arc_data, Some(self.cache_ttl))
                            .await
                        {
                            Ok(_) => {
                                // Redis 更新成功後，更新內存快取和映射
                                self.$cache_field.insert(hash, arc_data.clone()).await;
                                self.key_mapping.write().await.insert(hash, key.to_string());
                                Ok(arc_data)
                            }
                            Err(e) => {
                                // Redis 更新失敗，不更新內存快取
                                self.record_metric::<$data_type>(
                                    MetricType::Error {
                                        operation: "set_in_compute",
                                    },
                                    None,
                                );
                                Err(e)
                            }
                        }
                    }
                    Err(e) => Err(e),
                }
            }

            /// 批量獲取資料 - 帶監控指標
            ///
            /// 直接返回 Arc<Vec<T>>，避免深拷貝整個 Vec。
            /// 適合需要高性能且只讀數據的場景。
            pub async fn [<get_ $type_name _batch>](
                &self,
                keys: &[String],
            ) -> Result<Vec<Option<Arc<Vec<$data_type>>>>, CacheError> {
                let start = Instant::now();

                if keys.is_empty() {
                    return Ok(Vec::new());
                }

                let mut results = vec![None; keys.len()];
                let mut missing_keys = Vec::new();
                let mut missing_indices = Vec::new();
                let mut missing_hashes = Vec::new();

                // 1. 先從內存快取批量獲取（使用 hash）
                for (idx, key) in keys.iter().enumerate() {
                    let hash = Self::hash_key(key);
                    if let Some(arc_data) = self.$cache_field.get(&hash).await {
                        results[idx] = Some(arc_data);
                        self.record_metric::<$data_type>(MetricType::Hit { layer: "memory" }, None);
                    } else {
                        missing_keys.push(key.clone());
                        missing_indices.push(idx);
                        missing_hashes.push(hash);
                    }
                }

                // 2. 如果有缺失的key，從Redis批量獲取
                if !missing_keys.is_empty() {
                    let redis_results = self
                        .redis_cache
                        .mget::<String, Vec<$data_type>>(&missing_keys)
                        .await?;

                    // 批量更新映射
                    let mut key_mapping = self.key_mapping.write().await;

                    for ((idx, data_opt), hash) in missing_indices
                        .iter()
                        .zip(redis_results)
                        .zip(missing_hashes.iter())
                    {
                        if let Some(data) = data_opt {
                            let arc_data = Arc::new(data);
                            // 更新內存快取（使用 hash）
                            self.$cache_field.insert(*hash, arc_data.clone()).await;
                            // 更新映射
                            key_mapping.insert(*hash, keys[*idx].clone());
                            results[*idx] = Some(arc_data);
                            self.record_metric::<$data_type>(MetricType::Hit { layer: "redis" }, None);
                        } else {
                            self.record_metric::<$data_type>(MetricType::Miss, None);
                        }
                    }
                }

                self.record_metric::<$data_type>(
                    MetricType::BatchOperation {
                        operation: "get",
                        count: keys.len(),
                    },
                    Some(start.elapsed()),
                );

                Ok(results)
            }

            /// 預熱快取
            pub async fn [<warm_ $type_name _cache>](&self, keys: Vec<String>) -> Result<(), CacheError> {
                let _ = self.[<get_ $type_name _batch>](&keys).await?;
                Ok(())
            }

            /// 批量設置資料 - 帶監控指標
            pub async fn [<set_ $type_name _batch>](
                &self,
                items: &[(String, Vec<$data_type>)],
            ) -> Result<(), CacheError> {
                let start = Instant::now();

                if items.is_empty() {
                    return Ok(());
                }

                // 1. 先收集所有需要的數據
                let updates: Vec<(u64, String, Arc<Vec<$data_type>>)> = items
                    .iter()
                    .map(|(key, data)| (Self::hash_key(key), key.clone(), Arc::new(data.clone())))
                    .collect();

                // 2. 批量更新內存快取（不需要持有鎖）
                for (hash, _, arc_data) in &updates {
                    self.$cache_field.insert(*hash, arc_data.clone()).await;
                }

                // 3. 最後一次性更新映射（最小化鎖持有時間）
                self.key_mapping
                    .write()
                    .await
                    .extend(updates.iter().map(|(hash, key, _)| (*hash, key.clone())));

                // 4. 批量更新 Redis 快取
                match self.redis_cache.mset(items, Some(self.cache_ttl)).await {
                    Ok(_) => {
                        CacheMetrics::record_batch_set($display_name, items.len(), start.elapsed());
                        Ok(())
                    }
                    Err(e) => {
                        CacheMetrics::record_batch_set_error($display_name);
                        Err(e)
                    }
                }
            }

            /// 優化的批量設置 - 減少複製操作
            ///
            /// 當調用者已經擁有 Arc<Vec<T>> 時，使用此方法可以避免額外的 clone。
            pub async fn [<set_ $type_name _batch_optimized>](
                &self,
                items: Vec<(String, Arc<Vec<$data_type>>)>,
            ) -> Result<(), CacheError> {
                let start = Instant::now();

                if items.is_empty() {
                    return Ok(());
                }

                // 1. 先收集所有需要的數據
                let updates: Vec<(u64, String)> = items
                    .iter()
                    .map(|(key, _)| (Self::hash_key(key), key.clone()))
                    .collect();

                // 2. 批量更新內存快取（不需要持有鎖）
                for ((_, arc_data), (hash, _)) in items.iter().zip(updates.iter()) {
                    self.$cache_field.insert(*hash, arc_data.clone()).await;
                }

                // 3. 最後一次性更新映射（最小化鎖持有時間）
                self.key_mapping.write().await.extend(updates);

                // 4. 準備 Redis 數據（只在必要時解引用）
                let redis_items: Vec<(String, &Vec<$data_type>)> =
                    items.iter().map(|(k, v)| (k.clone(), &**v)).collect();

                match self
                    .redis_cache
                    .mset(&redis_items, Some(self.cache_ttl))
                    .await
                {
                    Ok(_) => {
                        CacheMetrics::record_batch_set($display_name, items.len(), start.elapsed());
                        Ok(())
                    }
                    Err(e) => {
                        CacheMetrics::record_batch_set_error($display_name);
                        Err(e)
                    }
                }
            }

            /// 使用 Pipeline 批量設置 - 最大化 Redis 性能
            ///
            /// 使用 Redis Pipeline 技術一次性執行多個命令，大幅減少網路往返次數。
            /// 適合大批量數據寫入場景，性能比逐個設置或 MSET 更佳。
            pub async fn [<set_ $type_name _batch_pipeline>](
                &self,
                items: &[(String, Arc<Vec<$data_type>>)],
            ) -> Result<(), CacheError> {
                let start = Instant::now();

                if items.is_empty() {
                    return Ok(());
                }

                // 1. 先收集所有需要的數據
                let updates: Vec<(u64, String)> = items
                    .iter()
                    .map(|(key, _)| (Self::hash_key(key), key.clone()))
                    .collect();

                // 2. 批量更新內存快取（不需要持有鎖）
                for ((_, arc_data), (hash, _)) in items.iter().zip(updates.iter()) {
                    self.$cache_field.insert(*hash, arc_data.clone()).await;
                }

                // 3. 最後一次性更新映射（最小化鎖持有時間）
                self.key_mapping.write().await.extend(updates);

                // 4. 使用 CacheManager 的 Pipeline 批量設置
                let redis_items: Vec<(String, &Vec<$data_type>)> =
                    items.iter().map(|(k, v)| (k.clone(), &**v)).collect();

                match self
                    .redis_cache
                    .pipeline_mset(&redis_items, Some(self.cache_ttl))
                    .await
                {
                    Ok(_) => {
                        CacheMetrics::record_pipeline_set($display_name, items.len(), start.elapsed());
                        Ok(())
                    }
                    Err(e) => {
                        CacheMetrics::record_pipeline_set_error($display_name);
                        Err(e)
                    }
                }
            }

            /// 使用預分配緩衝區的批量獲取
            ///
            /// 通過重用緩衝區來減少記憶體分配，提升批量操作性能。
            pub async fn [<get_ $type_name _batch_buffered>](
                &self,
                keys: &[String],
                buffer: &mut CacheBuffer,
            ) -> Result<Vec<Option<Arc<Vec<$data_type>>>>, CacheError> {
                let start = Instant::now();

                buffer.clear();
                let mut results = vec![None; keys.len()];
                let mut missing_hashes = Vec::new();

                // 重用緩衝區來收集缺失的鍵（使用 hash）
                for (idx, key) in keys.iter().enumerate() {
                    let hash = Self::hash_key(key);
                    if let Some(arc_data) = self.$cache_field.get(&hash).await {
                        results[idx] = Some(arc_data);
                        self.record_metric::<$data_type>(MetricType::Hit { layer: "memory" }, None);
                    } else {
                        buffer.keys.push(key.clone());
                        buffer.indices.push(idx);
                        missing_hashes.push(hash);
                    }
                }

                // 如果有缺失的key，從Redis批量獲取
                if !buffer.keys.is_empty() {
                    let redis_results = self
                        .redis_cache
                        .mget::<String, Vec<$data_type>>(&buffer.keys)
                        .await?;

                    // 批量更新映射
                    let mut key_mapping = self.key_mapping.write().await;

                    for ((idx, data_opt), hash) in buffer
                        .indices
                        .iter()
                        .zip(redis_results)
                        .zip(missing_hashes.iter())
                    {
                        if let Some(data) = data_opt {
                            let arc_data = Arc::new(data);
                            // 更新內存快取（使用 hash）
                            self.$cache_field.insert(*hash, arc_data.clone()).await;
                            // 更新映射
                            key_mapping.insert(*hash, keys[*idx].clone());
                            results[*idx] = Some(arc_data);
                            self.record_metric::<$data_type>(MetricType::Hit { layer: "redis" }, None);
                        } else {
                            self.record_metric::<$data_type>(MetricType::Miss, None);
                        }
                    }
                }

                self.record_metric::<$data_type>(
                    MetricType::BatchOperation {
                        operation: "get",
                        count: keys.len(),
                    },
                    Some(start.elapsed()),
                );

                Ok(results)
            }
        }
    };
}

pub(crate) use impl_cache_methods;
