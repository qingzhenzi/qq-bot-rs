use serde::{Deserialize, Serialize};

use crate::types::user::{Member, User};

/// 频道消息（`AT_MESSAGE_CREATE` / `MESSAGE_CREATE`）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelMessage {
    pub id: String,

    pub channel_id: String,

    pub guild_id: String,

    /// 文本内容（@ 前缀也会出现在这里）。
    pub content: String,

    pub author: User,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<Member>,

    #[serde(default)]
    pub mentions: Vec<User>,

    #[serde(default)]
    pub attachments: Vec<Attachment>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_reference: Option<MessageReference>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,

    /// 子频道维度的序号（QQ 协议是字符串）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq_in_channel: Option<String>,

    /// ISO 8601。
    pub timestamp: String,

    /// 消息编辑时间（ISO 8601）。仅 `GET /channels/{id}/messages/{mid}` 与
    /// `PATCH` 后回带；Gateway 事件不携带。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_timestamp: Option<String>,
}

/// 群里 @ 机器人的消息——无 `channel_id` / `guild_id`，用 `group_openid` +
/// `author.member_openid` 标识。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupMessage {
    pub id: String,

    pub group_openid: String,

    pub author: GroupMessageAuthor,

    pub content: String,

    #[serde(default)]
    pub attachments: Vec<Attachment>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_reference: Option<MessageReference>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_seq: Option<u64>,

    /// ISO 8601。
    pub timestamp: String,
}

/// 群消息发送者——只有 `member_openid`，无昵称 / 头像（隐私边界）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupMessageAuthor {
    /// 群成员 openid——同一用户在不同群间不同。
    pub member_openid: String,
    /// QQ 昵称（部分事件可能为空）
    #[serde(default)]
    pub username: String,
}

/// 用户私聊（C2C）消息。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct C2cMessage {
    pub id: String,

    pub author: C2cMessageAuthor,

    pub content: String,

    #[serde(default)]
    pub attachments: Vec<Attachment>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_reference: Option<MessageReference>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_seq: Option<u64>,

    /// ISO 8601。
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct C2cMessageAuthor {
    /// 用户 openid（应用维度稳定）。
    pub user_openid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Attachment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    pub url: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,

    /// 语音消息的自动语音识别（ASR）转文字结果
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asr_refer_text: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageReference {
    pub message_id: String,

    /// 取被引用消息失败时是否忽略——发送侧用，接收侧通常缺省。
    #[serde(default)]
    pub ignore_get_message_error: bool,
}
