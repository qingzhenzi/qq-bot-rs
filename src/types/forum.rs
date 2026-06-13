//! 论坛事件 DTO——覆盖 `FORUM_THREAD_CREATE` / `FORUM_THREAD_UPDATE` /
//! `FORUM_THREAD_DELETE`。

use serde::{Deserialize, Serialize};

/// 论坛帖子事件——创建/更新/删除共用。
///
/// `FORUM_THREAD_*`（私域）含 `thread_id` / `title` / `content` / `timestamp`，
/// `OPEN_FORUM_THREAD_*`（公域）仅含 `author_id` / `channel_id` / `guild_id`，
/// 因此 `thread_id` / `timestamp` 为可选字段。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForumThreadEvent {
    /// 频道 ID。
    pub guild_id: String,

    /// 子频道 ID。
    pub channel_id: String,

    /// 帖子发起人 user_id。
    pub author_id: String,

    /// 帖子 ID——`OPEN_FORUM_*` 事件不携带。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,

    /// 帖子标题。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// 帖子内容。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// 事件时间戳（ISO 8601）——`OPEN_FORUM_*` 事件不携带。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}
