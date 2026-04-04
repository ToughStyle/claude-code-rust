//! Plugin Marketplace API Client
//!
//! 连接真实插件市场API的客户端实现。

use crate::config::PluginMarketplaceConfig;
use crate::services::plugin_marketplace_error::{PluginMarketplaceError, Result};
use api_client::{ApiClient, ApiClientConfig};
use chrono::{DateTime, Utc};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// 缓存响应
#[derive(Debug, Clone)]
struct CachedResponse {
    data: serde_json::Value,
    expires_at: Instant,
    etag: Option<String>,
}

impl CachedResponse {
    fn new(data: serde_json::Value, ttl_seconds: u64, etag: Option<String>) -> Self {
        Self {
            data,
            expires_at: Instant::now() + Duration::from_secs(ttl_seconds),
            etag,
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// 插件市场API客户端
pub struct PluginMarketplaceClient {
    client: ApiClient,
    config: PluginMarketplaceConfig,
    cache: Arc<RwLock<LruCache<String, CachedResponse>>>,
    last_request_time: Arc<RwLock<Option<Instant>>>,
}

/// 插件搜索响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSearchResponse {
    pub plugins: Vec<MarketplacePlugin>,
    pub total: u32,
    pub page: u32,
    pub limit: u32,
    pub has_more: bool,
}

/// 市场插件详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplacePlugin {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub author_url: Option<String>,
    pub repository_url: Option<String>,
    pub documentation_url: Option<String>,
    pub icon_url: Option<String>,
    pub tags: Vec<String>,
    pub category: PluginCategory,
    pub downloads: u64,
    pub rating: f32,
    pub rating_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub license: String,
    pub is_official: bool,
    pub is_verified: bool,
    pub dependencies: Vec<String>,
    pub min_api_version: String,
    pub max_api_version: Option<String>,
    pub signature: Option<String>,
    pub source: String,
}

/// 插件类别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginCategory {
    Tools,
    Integrations,
    Themes,
    LanguageSupport,
    Productivity,
    Development,
    Other,
}

impl std::fmt::Display for PluginCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginCategory::Tools => write!(f, "Tools"),
            PluginCategory::Integrations => write!(f, "Integrations"),
            PluginCategory::Themes => write!(f, "Themes"),
            PluginCategory::LanguageSupport => write!(f, "Language Support"),
            PluginCategory::Productivity => write!(f, "Productivity"),
            PluginCategory::Development => write!(f, "Development"),
            PluginCategory::Other => write!(f, "Other"),
        }
    }
}

/// 插件下载包信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPackage {
    pub plugin_id: String,
    pub version: String,
    pub download_url: String,
    pub checksum: String,
    pub signature: String,
    pub size_bytes: u64,
    pub dependencies: Vec<String>,
    pub manifest: serde_json::Value,
}

/// 特色插件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturedPlugins {
    pub trending: Vec<MarketplacePlugin>,
    pub newest: Vec<MarketplacePlugin>,
    pub top_rated: Vec<MarketplacePlugin>,
    pub official: Vec<MarketplacePlugin>,
}

/// 类别信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub description: String,
    pub plugin_count: u32,
}

impl PluginMarketplaceClient {
    /// 创建新的插件市场客户端
    pub fn new(config: PluginMarketplaceConfig) -> Result<Self> {
        // 验证配置
        config.validate()
            .map_err(|e| PluginMarketplaceError::ConfigError(e))?;

        // 创建API客户端配置
        let client_config = ApiClientConfig {
            connect_timeout: Duration::from_secs(config.request_timeout_seconds as u64),
            read_timeout: Duration::from_secs(config.request_timeout_seconds as u64),
            retry_config: api_client::client::RetryConfig {
                max_retries: config.max_retries,
                initial_backoff: Duration::from_millis(100),
                max_backoff: Duration::from_secs(5),
                backoff_factor: 2.0,
                retry_status_codes: vec![429, 500, 502, 503, 504],
            },
            base_url: config.get_base_url(),
            default_model: api_client::types::ApiModel::Claude35Sonnet20241022,
            default_max_tokens: 1024,
            ..Default::default()
        };

        let client = ApiClient::new(&config.get_base_url(), client_config);

        // 设置API密钥
        let client = if let Some(api_key) = config.get_api_key() {
            client.with_api_key(&api_key)
        } else {
            client
        };

        // 创建缓存
        let cache_size = NonZeroUsize::new(config.max_cache_entries)
            .unwrap_or(NonZeroUsize::new(100).unwrap());
        let cache = LruCache::new(cache_size);

        Ok(Self {
            client,
            config,
            cache: Arc::new(RwLock::new(cache)),
            last_request_time: Arc::new(RwLock::new(None)),
        })
    }

    /// 搜索插件
    pub async fn search_plugins(
        &self,
        query: &str,
        page: u32,
        limit: u32,
    ) -> Result<PluginSearchResponse> {
        let cache_key = format!("search:{}:{}:{}", query, page, limit);

        // 检查缓存
        if let Some(cached) = self.get_from_cache(&cache_key).await {
            return serde_json::from_value(cached)
                .map_err(PluginMarketplaceError::from);
        }

        // 构建请求URL
        let url = format!("/plugins/search?q={}&page={}&limit={}",
            urlencoding::encode(query), page, limit);

        let response = self.make_request(&url).await?;

        // 解析响应
        let search_response: PluginSearchResponse = serde_json::from_value(response.clone())
            .map_err(PluginMarketplaceError::from)?;

        // 缓存结果
        self.set_cache(&cache_key, response, self.config.cache_ttl_seconds).await;

        Ok(search_response)
    }

    /// 获取插件详情
    pub async fn get_plugin(&self, plugin_id: &str) -> Result<MarketplacePlugin> {
        let cache_key = format!("plugin:{}", plugin_id);

        // 检查缓存
        if let Some(cached) = self.get_from_cache(&cache_key).await {
            return serde_json::from_value(cached)
                .map_err(PluginMarketplaceError::from);
        }

        let url = format!("/plugins/{}", plugin_id);
        let response = self.make_request(&url).await?;

        // 解析响应
        let plugin: MarketplacePlugin = serde_json::from_value(response.clone())
            .map_err(PluginMarketplaceError::from)?;

        // 检查插件源是否被允许
        if !self.config.is_source_allowed(&plugin.source) {
            return Err(PluginMarketplaceError::PermissionError(
                format!("Plugin source '{}' is not allowed", plugin.source)
            ));
        }

        // 缓存结果
        self.set_cache(&cache_key, response, self.config.cache_ttl_seconds).await;

        Ok(plugin)
    }

    /// 获取特色插件
    pub async fn get_featured(&self) -> Result<FeaturedPlugins> {
        let cache_key = "featured".to_string();

        // 检查缓存
        if let Some(cached) = self.get_from_cache(&cache_key).await {
            return serde_json::from_value(cached)
                .map_err(PluginMarketplaceError::from);
        }

        let url = "/plugins/featured".to_string();
        let response = self.make_request(&url).await?;

        // 解析响应
        let featured: FeaturedPlugins = serde_json::from_value(response.clone())
            .map_err(PluginMarketplaceError::from)?;

        // 缓存结果（使用较短的TTL，因为特色插件可能经常变化）
        self.set_cache(&cache_key, response, 60).await; // 1分钟

        Ok(featured)
    }

    /// 获取所有类别
    pub async fn get_categories(&self) -> Result<Vec<Category>> {
        let cache_key = "categories".to_string();

        // 检查缓存
        if let Some(cached) = self.get_from_cache(&cache_key).await {
            return serde_json::from_value(cached)
                .map_err(PluginMarketplaceError::from);
        }

        let url = "/categories".to_string();
        let response = self.make_request(&url).await?;

        // 解析响应
        let categories: Vec<Category> = serde_json::from_value(response.clone())
            .map_err(PluginMarketplaceError::from)?;

        // 缓存结果（类别不常变化，使用较长的TTL）
        self.set_cache(&cache_key, response, 3600).await; // 1小时

        Ok(categories)
    }

    /// 下载插件包
    pub async fn download_plugin(&self, plugin_id: &str, version: Option<&str>) -> Result<PluginPackage> {
        let url = match version {
            Some(v) => format!("/plugins/{}/versions/{}/download", plugin_id, v),
            None => format!("/plugins/{}/download", plugin_id),
        };

        let response = self.make_request(&url).await?;

        // 解析响应
        let package: PluginPackage = serde_json::from_value(response)
            .map_err(PluginMarketplaceError::from)?;

        Ok(package)
    }

    /// 验证插件签名
    pub async fn verify_signature(&self, plugin_id: &str, signature: &str) -> Result<bool> {
        if !self.config.verify_signatures {
            return Ok(true); // 如果配置不要求验证，则直接返回成功
        }

        let url = format!("/plugins/{}/verify", plugin_id);
        let body = serde_json::json!({
            "signature": signature,
        });

        let response = self.make_post_request(&url, &body).await?;

        let verification_result: serde_json::Value = serde_json::from_value(response)
            .map_err(PluginMarketplaceError::from)?;

        match verification_result.get("verified").and_then(|v| v.as_bool()) {
            Some(verified) => Ok(verified),
            None => Err(PluginMarketplaceError::SignatureVerificationFailed(
                "Invalid verification response".to_string()
            )),
        }
    }

    /// 获取市场统计信息
    pub async fn get_statistics(&self) -> Result<serde_json::Value> {
        let cache_key = "statistics".to_string();

        // 检查缓存
        if let Some(cached) = self.get_from_cache(&cache_key).await {
            return Ok(cached);
        }

        let url = "/statistics".to_string();
        let response = self.make_request(&url).await?;

        // 缓存结果（统计信息每小时更新一次）
        self.set_cache(&cache_key, response.clone(), 3600).await;

        Ok(response)
    }

    /// 清空缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// 获取缓存统计信息
    pub async fn get_cache_stats(&self) -> HashMap<String, usize> {
        let cache = self.cache.read().await;
        let mut stats = HashMap::new();
        stats.insert("size".to_string(), cache.len());
        stats.insert("capacity".to_string(), cache.cap().into_inner());
        stats
    }

    // 私有辅助方法

    /// 从缓存获取数据
    async fn get_from_cache(&self, key: &str) -> Option<serde_json::Value> {
        if self.config.offline_mode {
            // 离线模式：只使用缓存，即使过期也返回
            let cache = self.cache.read().await;
            return cache.peek(key).map(|cached| cached.data.clone());
        }

        let mut cache = self.cache.write().await;
        if let Some(cached) = cache.get(key) {
            if cached.is_expired() {
                // 缓存过期，移除
                cache.pop(key);
                None
            } else {
                Some(cached.data.clone())
            }
        } else {
            None
        }
    }

    /// 设置缓存
    async fn set_cache(&self, key: &str, data: serde_json::Value, ttl_seconds: u64) {
        let mut cache = self.cache.write().await;
        let cached_response = CachedResponse::new(data, ttl_seconds, None);
        cache.put(key.to_string(), cached_response);
    }

    /// 发送GET请求
    async fn make_request(&self, path: &str) -> Result<serde_json::Value> {
        self.rate_limit().await;

        let url = format!("{}{}", self.config.get_base_url(), path);

        if self.config.debug_logging {
            println!("[PluginMarketplace] GET {}", url);
        }

        match self.client.get(&url).send().await {
            Ok(response) => {
                let status = response.status();
                let text = response.text().await
                    .map_err(PluginMarketplaceError::NetworkError)?;

                if status.is_success() {
                    serde_json::from_str(&text)
                        .map_err(PluginMarketplaceError::from)
                } else {
                    Err(PluginMarketplaceError::from_http_status(
                        status.as_u16(),
                        text
                    ))
                }
            }
            Err(e) => {
                if self.config.debug_logging {
                    println!("[PluginMarketplace] Request failed: {}", e);
                }
                Err(PluginMarketplaceError::NetworkError(e))
            }
        }
    }

    /// 发送POST请求
    async fn make_post_request(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        self.rate_limit().await;

        let url = format!("{}{}", self.config.get_base_url(), path);

        if self.config.debug_logging {
            println!("[PluginMarketplace] POST {}", url);
        }

        match self.client.post(&url).json(body).send().await {
            Ok(response) => {
                let status = response.status();
                let text = response.text().await
                    .map_err(PluginMarketplaceError::NetworkError)?;

                if status.is_success() {
                    serde_json::from_str(&text)
                        .map_err(PluginMarketplaceError::from)
                } else {
                    Err(PluginMarketplaceError::from_http_status(
                        status.as_u16(),
                        text
                    ))
                }
            }
            Err(e) => {
                if self.config.debug_logging {
                    println!("[PluginMarketplace] Request failed: {}", e);
                }
                Err(PluginMarketplaceError::NetworkError(e))
            }
        }
    }

    /// 简单的速率限制（防止请求过频）
    async fn rate_limit(&self) {
        let mut last_request = self.last_request_time.write().await;
        if let Some(last) = *last_request {
            let elapsed = last.elapsed();
            if elapsed < Duration::from_millis(100) {
                // 最少100毫秒间隔
                tokio::time::sleep(Duration::from_millis(100) - elapsed).await;
            }
        }
        *last_request = Some(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_response() {
        let data = serde_json::json!({"test": "value"});
        let cached = CachedResponse::new(data.clone(), 1, None);

        assert!(!cached.is_expired());
        assert_eq!(cached.data, data);
    }

    #[test]
    fn test_plugin_category_display() {
        let category = PluginCategory::Tools;
        assert_eq!(category.to_string(), "Tools");

        let category = PluginCategory::LanguageSupport;
        assert_eq!(category.to_string(), "Language Support");
    }

    // 注意：由于需要真实的API端点，集成测试需要在实际环境中进行
    // 这里只测试数据结构和基本逻辑
}