//! 频道 / 子频道数据结构——覆盖 `GUILD_CREATE` / `GUILD_UPDATE` / `GUILD_DELETE`
//! 与 `CHANNEL_CREATE` / `CHANNEL_UPDATE` / `CHANNEL_DELETE` 事件以及对应的 HTTP 响应。

use serde::{Deserialize, Serialize};

/// 频道（公会）信息。
///
/// `GUILD_*` 事件与 `GET /guilds/{guild_id}` 共用同一结构，缺失字段用 `Option` 兜底。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Guild {
    pub id: String,

    pub name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// 频道主 user_id。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,

    /// 当前用户是否为频道主（`GET /guilds` 回带）。
    #[serde(default)]
    pub owner: bool,

    /// 成员数（`GET /guilds` 回带）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_count: Option<u32>,

    /// 最大成员数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_members: Option<u32>,

    /// 频道描述。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// 机器人加入频道的时间（ISO 8601）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub joined_at: Option<String>,

    /// 操作人 user_id（仅 `GUILD_*` 事件携带，HTTP 响应为 `None`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub op_user_id: Option<String>,
}

/// 子频道信息。
///
/// 字段数超 5，在 [`crate::event::Event`] 中走 `Box`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Channel {
    pub id: String,

    /// 所属频道 ID。
    pub guild_id: String,

    pub name: String,

    /// 子频道类型（0=文字, 2=语音, 4=子频道分组, 10005=直播, 10006=应用, 10007=论坛）。
    /// serde 映射为 `type` 字段。
    #[serde(rename = "type")]
    pub type_: u32,

    /// 上级分组 ID（仅嵌套子频道携带）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,

    /// 操作人 user_id（仅 `CHANNEL_*` 事件携带）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub op_user_id: Option<String>,
}
