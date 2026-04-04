//! Plugin Marketplace Configuration
//!
//! 插件市场配置，包含API端点、缓存、重试等设置。

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 插件市场配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMarketplaceConfig {
    /// 市场API基础URL
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// API密钥（可选）
    pub api_key: Option<String>,

    /// 缓存TTL（秒）
    #[serde(default = "default_cache_ttl_seconds")]
    pub cache_ttl_seconds: u64,

    /// 最大重试次数
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// 请求超时（秒）
    #[serde(default = "default_request_timeout_seconds")]
    pub request_timeout_seconds: u32,

    /// 启用签名验证
    #[serde(default = "default_verify_signatures")]
    pub verify_signatures: bool,

    /// 允许的插件源
    #[serde(default = "default_allowed_sources")]
    pub allowed_sources: HashSet<String>,

    /// 自动更新检查间隔（小时）
    #[serde(default = "default_update_check_interval_hours")]
    pub update_check_interval_hours: u32,

    /// 最大缓存条目数
    #[serde(default = "default_max_cache_entries")]
    pub max_cache_entries: usize,

    /// 启用调试日志
    #[serde(default)]
    pub debug_logging: bool,

    /// 离线模式（仅使用缓存）
    #[serde(default)]
    pub offline_mode: bool,
}

fn default_base_url() -> String {
    "https://plugins.claude.ai/api/v1".to_string()
}

fn default_cache_ttl_seconds() -> u64 {
    300 // 5分钟
}

fn default_max_retries() -> u32 {
    3
}

fn default_request_timeout_seconds() -> u32 {
    30
}

fn default_verify_signatures() -> bool {
    true
}

fn default_allowed_sources() -> HashSet<String> {
    let mut sources = HashSet::new();
    sources.insert("official".to_string());
    sources.insert("verified".to_string());
    sources
}

fn default_update_check_interval_hours() -> u32 {
    24
}

fn default_max_cache_entries() -> usize {
    100
}

impl Default for PluginMarketplaceConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            api_key: None,
            cache_ttl_seconds: default_cache_ttl_seconds(),
            max_retries: default_max_retries(),
            request_timeout_seconds: default_request_timeout_seconds(),
            verify_signatures: default_verify_signatures(),
            allowed_sources: default_allowed_sources(),
            update_check_interval_hours: default_update_check_interval_hours(),
            max_cache_entries: default_max_cache_entries(),
            debug_logging: false,
            offline_mode: false,
        }
    }
}

impl PluginMarketplaceConfig {
    /// 获取API密钥，优先从环境变量读取
    pub fn get_api_key(&self) -> Option<String> {
        std::env::var("CLAUDE_PLUGIN_MARKETPLACE_API_KEY")
            .ok()
            .or_else(|| self.api_key.clone())
    }

    /// 获取基础URL，优先从环境变量读取
    pub fn get_base_url(&self) -> String {
        std::env::var("CLAUDE_PLUGIN_MARKETPLACE_BASE_URL")
            .unwrap_or_else(|_| self.base_url.clone())
    }

    /// 检查插件源是否被允许
    pub fn is_source_allowed(&self, source: &str) -> bool {
        self.allowed_sources.contains(source)
    }

    /// 验证配置
    pub fn validate(&self) -> Result<(), String> {
        if self.base_url.is_empty() {
            return Err("base_url cannot be empty".to_string());
        }

        if self.cache_ttl_seconds == 0 {
            return Err("cache_ttl_seconds must be greater than 0".to_string());
        }

        if self.request_timeout_seconds == 0 {
            return Err("request_timeout_seconds must be greater than 0".to_string());
        }

        if self.max_cache_entries == 0 {
            return Err("max_cache_entries must be greater than 0".to_string());
        }

        Ok(())
    }

    /// 创建调试模式的配置
    pub fn debug() -> Self {
        Self {
            debug_logging: true,
            max_retries: 1,
            request_timeout_seconds: 10,
            ..Default::default()
        }
    }

    /// 创建严格模式的配置
    pub fn strict() -> Self {
        let mut sources = HashSet::new();
        sources.insert("official".to_string());

        Self {
            verify_signatures: true,
            allowed_sources: sources,
            max_cache_entries: 50,
            ..Default::default()
        }
    }

    /// 创建宽松模式的配置
    pub fn permissive() -> Self {
        let mut sources = HashSet::new();
        sources.insert("official".to_string());
        sources.insert("verified".to_string());
        sources.insert("community".to_string());

        Self {
            verify_signatures: false,
            allowed_sources: sources,
            max_cache_entries: 200,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PluginMarketplaceConfig::default();

        assert_eq!(config.base_url, "https://plugins.claude.ai/api/v1");
        assert_eq!(config.cache_ttl_seconds, 300);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.request_timeout_seconds, 30);
        assert!(config.verify_signatures);
        assert!(config.allowed_sources.contains("official"));
        assert!(config.allowed_sources.contains("verified"));
        assert_eq!(config.update_check_interval_hours, 24);
        assert_eq!(config.max_cache_entries, 100);
        assert!(!config.debug_logging);
        assert!(!config.offline_mode);
    }

    #[test]
    fn test_validation() {
        let config = PluginMarketplaceConfig::default();
        assert!(config.validate().is_ok());

        let mut invalid_config = PluginMarketplaceConfig::default();
        invalid_config.base_url = String::new();
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_source_checking() {
        let config = PluginMarketplaceConfig::default();
        assert!(config.is_source_allowed("official"));
        assert!(config.is_source_allowed("verified"));
        assert!(!config.is_source_allowed("unknown"));
    }

    #[test]
    fn test_environment_variables() {
        std::env::set_var("CLAUDE_PLUGIN_MARKETPLACE_API_KEY", "test_key");
        std::env::set_var("CLAUDE_PLUGIN_MARKETPLACE_BASE_URL", "https://test.example.com");

        let config = PluginMarketplaceConfig::default();

        assert_eq!(config.get_api_key(), Some("test_key".to_string()));
        assert_eq!(config.get_base_url(), "https://test.example.com");

        // 清理环境变量
        std::env::remove_var("CLAUDE_PLUGIN_MARKETPLACE_API_KEY");
        std::env::remove_var("CLAUDE_PLUGIN_MARKETPLACE_BASE_URL");
    }

    #[test]
    fn test_config_variants() {
        let debug_config = PluginMarketplaceConfig::debug();
        assert!(debug_config.debug_logging);
        assert_eq!(debug_config.max_retries, 1);

        let strict_config = PluginMarketplaceConfig::strict();
        assert!(strict_config.verify_signatures);
        assert_eq!(strict_config.allowed_sources.len(), 1);

        let permissive_config = PluginMarketplaceConfig::permissive();
        assert!(!permissive_config.verify_signatures);
        assert_eq!(permissive_config.allowed_sources.len(), 3);
    }
}