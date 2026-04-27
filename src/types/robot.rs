use serde::{Deserialize, Serialize};

/// `GET /users/@me` 返回的机器人自身信息。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Robot {
    pub id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

/// `GET /gateway/bot` 返回的 WebSocket 接入信息。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WsGateway {
    pub url: String,
}
