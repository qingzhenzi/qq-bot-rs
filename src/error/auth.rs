//! 凭证加载错误——仅针对凭证从外部输入（环境变量等）的失败。
//! token 交换、API 鉴权失败这类发生在 HTTP 路径上的归 [`crate::error::HttpError`]。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    /// 必需的环境变量未设置或不可读。
    #[error("missing required environment variable: {0}")]
    MissingEnv(String),
}
