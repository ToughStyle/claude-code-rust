//! 错误类型定义

use thiserror::Error;

/// 工具系统错误
#[derive(Debug, Error)]
pub enum ToolError {
    /// 工具未找到
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// 工具执行错误
    #[error("Tool execution error: {0}")]
    ExecutionError(String),

    /// 权限错误
    #[error("Permission denied: {0}")]
    PermissionError(String),

    /// 输入验证错误
    #[error("Input validation error: {0}")]
    ValidationError(String),

    /// IO错误
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON序列化/反序列化错误
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// API工具调用错误
    #[error("API tool call error: {0}")]
    ApiToolCallError(String),

    /// 其他错误
    #[error("Other error: {0}")]
    Other(String),
}

impl ToolError {
    /// 创建API工具调用错误
    pub fn api_tool_call(message: impl Into<String>) -> Self {
        Self::ApiToolCallError(message.into())
    }

    /// 创建工具未找到错误
    pub fn tool_not_found(name: impl Into<String>) -> Self {
        Self::ToolNotFound(name.into())
    }

    /// 创建执行错误
    pub fn execution(message: impl Into<String>) -> Self {
        Self::ExecutionError(message.into())
    }
}

/// 工具系统结果类型
pub type Result<T> = std::result::Result<T, ToolError>;