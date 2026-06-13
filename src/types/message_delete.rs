//! 直接消息 / 公开消息删除事件 DTO。

use serde::{Deserialize, Serialize};

/// `DIRECT_MESSAGE_DELETE` / `PUBLIC_MESSAGE_DELETE` 共用。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageDeleteEvent {
    /// 频道 ID。
    pub guild_id: String,

    /// 子频道 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,

    /// 被删除的消息 ID。
    pub message_id: String,

    /// 操作人 user_id。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub op_user_id: Option<String>,
}
