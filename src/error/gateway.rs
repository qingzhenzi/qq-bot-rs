use thiserror::Error;

use crate::error::HttpError;
use crate::types::gateway::OpCode;

#[derive(Debug, Error)]
pub enum GatewayError {
    /// 拉网关入口、刷新 token 时的 HTTP 失败。
    #[error("http: {0}")]
    Http(#[from] HttpError),

    /// `tokio-tungstenite` 抛出的 WebSocket 协议层故障。
    #[error("websocket: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// JSON 编解码失败——通常是服务端 schema 与 SDK 不一致。
    #[error("frame decode failed: {0}")]
    Decode(#[from] serde_json::Error),

    /// 收到了不该出现在当前阶段的 OpCode。
    #[error("unexpected opcode at this stage: {0:?}")]
    UnexpectedOp(OpCode),

    /// 握手过程中连接被对端关闭。
    #[error("connection closed during handshake")]
    HandshakeClosed,

    /// 服务端报鉴权失败（关闭码 4004 等）。
    #[error("authentication rejected by server (code {code})")]
    AuthRejected {
        /// 服务端给出的 WebSocket 关闭码。
        code: u16,
    },

    /// 服务端返回的网关 URL 不是 `wss://`（或未指向 loopback 的 `ws://`）。
    /// 拒绝连接以避免凭证明文外发或被钓鱼端点接走。
    #[error("insecure or invalid gateway url: {0:?}")]
    InsecureGatewayUrl(String),

    /// `Hello` 帧给的 `heartbeat_interval` 超出合理范围（0 / 异常大），
    /// 直接照办会导致 CPU spin 或心跳永不发。
    #[error("invalid heartbeat interval from server: {0} ms")]
    InvalidHeartbeatInterval(u64),
}
