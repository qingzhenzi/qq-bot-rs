//! `INTERACTION_CREATE` 事件 DTO。
//!
//! 事件 schema 跨场景（频道 / 群 / c2c）共享同一壳，差异通过 `chat_type` 与
//! 不同 openid 字段表达——这些字段都是 `Option`，调用方按 `chat_type` 取。
//! 收到事件须在 5s 内调 [`crate::http::Bot::put_interaction_callback`] ACK。

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// 一次按钮 / 快捷菜单交互。`id` 是 ACK 用的 `interaction_id`，**不**是消息 ID。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Interaction {
    /// 平台事件 ID——`PUT /interactions/{id}` ACK 用。
    pub id: String,

    /// 互动类型——按钮 / 快捷菜单。
    #[serde(rename = "type")]
    pub interaction_type: InteractionType,

    /// 互动场景——频道 / 群 / c2c。
    pub chat_type: ChatType,

    /// 字符串场景标识："c2c" / "group" / "guild"——与 `chat_type` 冗余。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,

    /// 触发时间戳（RFC 3339）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,

    /// 事件协议版本号——目前固定为 1。
    #[serde(default)]
    pub version: i32,

    /// 频道 ID（仅 `chat_type = Guild`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<String>,

    /// 子频道 ID（仅 `chat_type = Guild`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,

    /// 用户 openid（仅 `chat_type = C2c`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_openid: Option<String>,

    /// 群 openid（仅 `chat_type = Group`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_openid: Option<String>,

    /// 群成员 openid（仅 `chat_type = Group`，标识点击者）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_member_openid: Option<String>,

    /// 互动数据——按钮 ID / 按钮 data 等。
    pub data: InteractionData,
}

/// `non_exhaustive` 兼容未来扩展。
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum InteractionType {
    /// 消息按钮点击。
    MessageButton = 11,

    /// 单聊快捷菜单。
    QuickMenu = 12,
}

/// 互动场景——决定哪组 openid 字段会被填。
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum ChatType {
    /// 频道。
    Guild = 0,

    /// 群聊。
    Group = 1,

    /// 单聊。
    C2c = 2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InteractionData {
    /// 文档暂未列出可选值，i32 透传。
    #[serde(rename = "type", default)]
    pub data_type: i32,

    /// 按钮 / 菜单 resolved 内容。
    pub resolved: InteractionResolved,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InteractionResolved {
    /// 发送时设置的按钮 `id`。
    #[serde(default)]
    pub button_id: String,

    /// 发送时设置的按钮 `data`——通常是开发者自定义 payload。
    #[serde(default)]
    pub button_data: String,

    /// 操作用户 ID（仅频道场景）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// 功能 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_id: Option<String>,

    /// 触发互动的消息 ID——按钮所在那条消息的 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
}
