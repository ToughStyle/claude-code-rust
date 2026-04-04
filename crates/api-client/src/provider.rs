//! API提供者模块
//!
//! 提供多模型提供者支持，包括：
//! - Anthropic Claude
//! - OpenAI
//! - 自定义提供者

use crate::error::{ApiError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API提供者类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderType {
    /// Anthropic Claude
    Anthropic,
    /// OpenAI
    OpenAI,
    /// Azure OpenAI
    AzureOpenAI,
    /// Google Gemini
    Gemini,
    /// AWS Bedrock
    Bedrock,
    /// Vertex AI
    Vertex,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Anthropic => write!(f, "anthropic"),
            Self::OpenAI => write!(f, "openai"),
            Self::AzureOpenAI => write!(f, "azure"),
            Self::Gemini => write!(f, "gemini"),
            Self::Bedrock => write!(f, "bedrock"),
            Self::Vertex => write!(f, "vertex"),
        }
    }
}

/// 提供者配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// 提供者类型
    pub provider_type: ProviderType,
    /// 基础URL
    pub base_url: String,
    /// API密钥
    pub api_key: Option<String>,
    /// 组织ID (OpenAI)
    pub organization_id: Option<String>,
    /// 项目ID (Vertex)
    pub project_id: Option<String>,
    /// 区域 (AWS/GCP)
    pub region: Option<String>,
    /// 自定义头信息
    pub custom_headers: HashMap<String, String>,
    /// 超时配置（秒）
    pub timeout_seconds: u64,
    /// 最大重试次数
    pub max_retries: u32,
}

impl ProviderConfig {
    /// Anthropic默认配置
    pub fn anthropic(api_key: impl Into<String>) -> Self {
        Self {
            provider_type: ProviderType::Anthropic,
            base_url: "https://api.anthropic.com".to_string(),
            api_key: Some(api_key.into()),
            organization_id: None,
            project_id: None,
            region: None,
            custom_headers: HashMap::new(),
            timeout_seconds: 60,
            max_retries: 3,
        }
    }

    /// OpenAI默认配置
    pub fn openai(api_key: impl Into<String>) -> Self {
        Self {
            provider_type: ProviderType::OpenAI,
            base_url: "https://api.openai.com".to_string(),
            api_key: Some(api_key.into()),
            organization_id: None,
            project_id: None,
            region: None,
            custom_headers: HashMap::new(),
            timeout_seconds: 60,
            max_retries: 3,
        }
    }

    /// 设置组织ID
    pub fn with_organization(mut self, org_id: impl Into<String>) -> Self {
        self.organization_id = Some(org_id.into());
        self
    }

    /// 设置项目ID
    pub fn with_project(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    /// 设置区域
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// 添加自定义头
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_headers.insert(key.into(), value.into());
        self
    }

    /// 获取认证头
    pub fn auth_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        match self.provider_type {
            ProviderType::Anthropic => {
                if let Some(ref key) = self.api_key {
                    headers.insert("x-api-key".to_string(), key.clone());
                    headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
                    headers.insert("content-type".to_string(), "application/json".to_string());
                }
            }
            ProviderType::OpenAI => {
                if let Some(ref key) = self.api_key {
                    headers.insert("authorization".to_string(), format!("Bearer {}", key));
                    headers.insert("content-type".to_string(), "application/json".to_string());
                }
                if let Some(ref org) = self.organization_id {
                    headers.insert("openai-organization".to_string(), org.clone());
                }
            }
            _ => {}
        }

        // 添加自定义头
        headers.extend(self.custom_headers.clone());
        headers
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider_type: ProviderType::Anthropic,
            base_url: crate::CLAUDE_API_BASE_URL.to_string(),
            api_key: None,
            organization_id: None,
            project_id: None,
            region: None,
            custom_headers: HashMap::new(),
            timeout_seconds: 60,
            max_retries: 3,
        }
    }
}

/// API提供者
pub struct ApiProvider {
    /// 配置
    pub config: ProviderConfig,
    /// HTTP客户端
    client: reqwest::Client,
}

impl ApiProvider {
    /// 创建新的API提供者
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| ApiError::other(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    /// 获取提供者类型
    pub fn provider_type(&self) -> ProviderType {
        self.config.provider_type
    }

    /// 获取基础URL
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    /// 获取HTTP客户端
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// 构建完整URL
    pub fn build_url(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url, path)
    }

    /// 获取提供者名称
    pub fn name(&self) -> String {
        match self.config.provider_type {
            ProviderType::Anthropic => "Anthropic Claude".to_string(),
            ProviderType::OpenAI => "OpenAI".to_string(),
            ProviderType::AzureOpenAI => "Azure OpenAI".to_string(),
            ProviderType::Gemini => "Google Gemini".to_string(),
            ProviderType::Bedrock => "AWS Bedrock".to_string(),
            ProviderType::Vertex => "Google Vertex AI".to_string(),
        }
    }

    /// 检查是否配置了API密钥
    pub fn has_api_key(&self) -> bool {
        self.config.api_key.is_some()
    }

    /// 设置API密钥
    pub fn set_api_key(&mut self, key: impl Into<String>) {
        self.config.api_key = Some(key.into());
    }

    /// 清除API密钥
    pub fn clear_api_key(&mut self) {
        self.config.api_key = None;
    }
}

/// OAuth配置
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    /// 客户端ID
    pub client_id: String,
    /// 客户端密钥
    pub client_secret: Option<String>,
    /// 授权URL
    pub auth_url: String,
    /// 令牌URL
    pub token_url: String,
    /// 重定向URL
    pub redirect_url: String,
    /// 作用域
    pub scopes: Vec<String>,
}

impl OAuthConfig {
    /// 创建新的OAuth配置
    pub fn new(
        client_id: impl Into<String>,
        auth_url: impl Into<String>,
        token_url: impl Into<String>,
        redirect_url: impl Into<String>,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: None,
            auth_url: auth_url.into(),
            token_url: token_url.into(),
            redirect_url: redirect_url.into(),
            scopes: Vec::new(),
        }
    }

    /// 设置客户端密钥
    pub fn with_secret(mut self, secret: impl Into<String>) -> Self {
        self.client_secret = Some(secret.into());
        self
    }

    /// 添加作用域
    pub fn add_scope(mut self, scope: impl Into<String>) -> Self {
        self.scopes.push(scope.into());
        self
    }

    /// 构建授权URL
    pub fn authorization_url(&self, state: &str) -> String {
        let scopes = self.scopes.join(" ");
        format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
            self.auth_url, self.client_id, self.redirect_url, scopes, state
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_config_anthropic() {
        let config = ProviderConfig::anthropic("test-key");
        assert_eq!(config.provider_type, ProviderType::Anthropic);
        assert!(config.api_key.is_some());
        assert_eq!(config.base_url, "https://api.anthropic.com");

        let headers = config.auth_headers();
        assert!(headers.contains_key("x-api-key"));
        assert!(headers.contains_key("anthropic-version"));
    }

    #[test]
    fn test_provider_config_openai() {
        let config = ProviderConfig::openai("test-key").with_organization("test-org");
        assert_eq!(config.provider_type, ProviderType::OpenAI);

        let headers = config.auth_headers();
        assert!(headers.contains_key("authorization"));
        assert_eq!(headers.get("authorization").unwrap(), "Bearer test-key");
        assert_eq!(headers.get("openai-organization").unwrap(), "test-org");
    }

    #[test]
    fn test_oauth_config() {
        let config = OAuthConfig::new(
            "client-id",
            "https://auth.example.com/authorize",
            "https://auth.example.com/token",
            "https://app.example.com/callback",
        )
        .add_scope("read")
        .add_scope("write");

        let auth_url = config.authorization_url("random-state");
        assert!(auth_url.contains("client_id=client-id"));
        assert!(auth_url.contains("state=random-state"));
        assert!(auth_url.contains("scope=read+write"));
    }
}
