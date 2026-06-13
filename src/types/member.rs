//! 频道成员事件 DTO——覆盖 `GUILD_MEMBER_ADD` / `GUILD_MEMBER_UPDATE` /
//! `GUILD_MEMBER_REMOVE`。
//!
//! 三个事件的 schema 不同，各自用独立结构体以保持类型安全。

use crate::types::user::User;
use serde::{Deserialize, Serialize};

/// `GUILD_MEMBER_ADD` 事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuildMemberAddEvent {
    /// 加入的频道 ID。
    pub guild_id: String,

    /// 用户加入时间（ISO 8601）。
    pub joined_at: String,

    /// 操作人 user_id（邀请人，仅邀请场景有值）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub op_user_id: Option<String>,

    /// 加入的用户。
    pub user: User,

    /// 用户在频道内的昵称。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nick: Option<String>,

    /// 用户在该频道下的身份组。
    #[serde(default)]
    pub roles: Vec<String>,
}

/// `GUILD_MEMBER_UPDATE` 事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuildMemberUpdateEvent {
    /// 频道 ID。
    pub guild_id: String,

    /// 被更新的用户。
    pub user: User,

    /// 更新后的昵称。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nick: Option<String>,

    /// 更新后的身份组。
    #[serde(default)]
    pub roles: Vec<String>,

    /// 加入时间（ISO 8601）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub joined_at: Option<String>,

    /// 操作人 user_id。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub op_user_id: Option<String>,
}

/// `GUILD_MEMBER_REMOVE` 事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuildMemberRemoveEvent {
    /// 频道 ID。
    pub guild_id: String,

    /// 操作人 user_id（手动移除时有值）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub op_user_id: Option<String>,

    /// 被移除的用户。
    pub user: User,
}

/// 频道成员分页——`GET /guilds/{guild_id}/members` 响应。
///
/// API 返回 `{"data": [...], "next": "..."}` 形态。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuildMemberPage {
    /// 本次返回的成员列表（对应 API 的 `data` 字段）。
    pub data: Vec<GuildMemberEntry>,

    /// 翻页游标——`None` 表示最后一页。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

/// `GuildMemberPage` 中的单个成员条目。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuildMemberEntry {
    pub user: User,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nick: Option<String>,

    #[serde(default)]
    pub roles: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub joined_at: Option<String>,

    /// 用户是否在语音频道中开启 deaf（闭音）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deaf: Option<bool>,

    /// 用户是否被禁言。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mute: Option<bool>,

    /// 用户是否处于"待审核"状态。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<bool>,
}
