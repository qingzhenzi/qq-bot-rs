//! 表情表态事件 DTO——覆盖 `MESSAGE_REACTION_ADD` / `MESSAGE_REACTION_REMOVE`。

use serde::{Deserialize, Serialize};

/// 表态事件——添加/移除表情表态共用。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReactionEvent {
    /// 所属频道 ID。
    pub guild_id: String,

    /// 子频道 ID。
    pub channel_id: String,

    /// 表态用户 ID。
    pub user_id: String,

    /// 被表态的消息 ID。
    pub message_id: String,

    /// 表态的目标表情。
    pub target: ReactionTarget,
}

/// 表态目标——标识被操作的 emoji。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReactionTarget {
    /// emoji ID（系统表情为数字字符串，自定义表情为 emoji 的 id）。
    pub id: String,

    /// emoji 类型：`1` = 系统表情，`2` = 自定义 emoji。
    #[serde(rename = "type")]
    pub type_: u8,
}
