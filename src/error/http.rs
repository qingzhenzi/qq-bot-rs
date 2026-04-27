use serde_json::Error as JsonError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
    /// 底层网络 / TLS / 超时——`reqwest` 抛出的所有 transport 类故障。
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// 服务端非 2xx 且响应体含可识别的 `{code, message, ...}` 业务错误结构。
    /// QQ v2 习惯在 4xx 里给业务错误码（如 `40054005` 消息去重）。
    #[error("api error code={code} status={status} message={message}{}",
        trace_id.as_ref().map(|t| format!(" trace_id={t}")).unwrap_or_default())]
    ApiError {
        /// HTTP 状态码。
        status: u16,

        /// 业务错误码——QQ 的 `code` 字段。
        code: i64,

        /// 业务错误描述（可能含中文）。
        message: String,

        /// 服务端日志 trace id——便于报障。
        trace_id: Option<String>,

        /// 原始响应体——保留供上层进一步排查（如 `err_code` 等附加字段）。
        body: String,
    },

    /// 服务端非 2xx 但响应体不是 [`Self::ApiError`] 那种结构。
    #[error("api status {status}: {body}")]
    Status {
        /// HTTP 状态码。
        status: u16,

        /// 响应体。
        body: String,
    },

    /// access_token 端点回 2xx 但响应体不含可用的 `access_token`——
    /// 大概率 AppID / Secret 错（QQ 此情形也回 200）。
    #[error("token rejected by server: {body}")]
    TokenRejected {
        /// 服务端原始响应体。
        body: String,
    },

    /// 响应体非合法 JSON / 不符合期望 schema。
    /// `Display` 截前 512 字符避免日志刷屏；`body` 字段本身是全量。
    /// 不打 body 等于盲飞——QQ 文档对不齐字段名是常见原因。
    #[error(
        "response decode failed: {source} | body={:?}",
        body.chars().take(512).collect::<String>()
    )]
    Decode {
        /// 服务端响应原文。
        body: String,

        /// serde 解析失败原因。
        source: JsonError,
    },

    /// `getAppAccessToken` 返回的 `expires_in` 不是合法的整数秒。
    #[error("invalid expires_in from server: {0:?}")]
    InvalidExpiresIn(String),
}
