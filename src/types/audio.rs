//! 音频事件 DTO——覆盖 `AUDIO_START` / `AUDIO_FINISH`。

use serde::{Deserialize, Serialize};

/// 音频频道事件——开始/结束共用。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioEvent {
    /// 频道 ID。
    pub guild_id: String,

    /// 子频道 ID。
    pub channel_id: String,

    /// 操作人 user_id。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub op_user_id: Option<String>,

    /// 音频类型。
    #[serde(default)]
    pub audio_type: u32,

    /// 音频状态。
    #[serde(default)]
    pub status: u32,

    /// 事件时间戳（ISO 8601）。
    pub timestamp: String,
}
