//! 库的公开错误类型。
//!
//! 各领域有自己的细粒度错误（[`AuthError`] / [`HttpError`] / [`GatewayError`] /
//! [`ClientBuildError`]）；顶层 [`BotError`] 用 `From` 把它们聚成一个 enum，
//! 便于跨层方法写 `Result<T, BotError>` 靠 `?` 自动桥接。

mod auth;
mod client;
mod gateway;
mod http;

pub use auth::AuthError;
pub use client::ClientBuildError;
pub use gateway::GatewayError;
pub use http::HttpError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BotError {
    /// 凭证加载失败。
    #[error(transparent)]
    Auth(#[from] AuthError),

    /// HTTP 路径失败。
    #[error(transparent)]
    Http(#[from] HttpError),

    /// 网关连接 / 握手 / 帧处理失败。
    #[error(transparent)]
    Gateway(#[from] GatewayError),

    /// `Client::builder()` 缺必填项。
    #[error(transparent)]
    ClientBuild(#[from] ClientBuildError),
}
