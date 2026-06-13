//! 频道消息置顶 API。

use crate::error::HttpError;
use crate::http::Bot;
use serde::{Deserialize, Serialize};
use tracing::info;

/// 频道置顶消息。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PinMessage {
    /// 频道 ID。
    pub guild_id: String,

    /// 子频道 ID。
    pub channel_id: String,

    /// 置顶消息 ID 列表。
    pub message_ids: Vec<String>,
}

impl Bot {
    /// `GET /channels/{channel_id}/pins` —— 获取子频道置顶消息列表。
    pub async fn get_pin_messages(&self, channel_id: &str) -> Result<PinMessage, HttpError> {
        let path = format!("/channels/{channel_id}/pins");
        info!(%channel_id, "[获取置顶消息]");
        self.get_json(&path).await
    }

    /// `PUT /channels/{channel_id}/pins/{message_id}` —— 置顶消息。
    pub async fn put_pin_message(
        &self,
        channel_id: &str,
        message_id: &str,
    ) -> Result<(), HttpError> {
        let path = format!("/channels/{channel_id}/pins/{message_id}");
        info!(%channel_id, %message_id, "[置顶消息]");
        self.put_empty(&path).await
    }

    /// `DELETE /channels/{channel_id}/pins/{message_id}` —— 取消置顶。
    pub async fn delete_pin_message(
        &self,
        channel_id: &str,
        message_id: &str,
    ) -> Result<(), HttpError> {
        let path = format!("/channels/{channel_id}/pins/{message_id}");
        info!(%channel_id, %message_id, "[取消置顶]");
        self.delete_empty(&path).await
    }
}
