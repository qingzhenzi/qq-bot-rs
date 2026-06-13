use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use super::inbound::MessageReference;
use crate::types::payloads::{ArkPayload, EmbedPayload, KeyboardPayload, MarkdownPayload, Media};

/// 流式消息状态
#[derive(Debug, Clone, Serialize)]
pub struct StreamState {
    /// 1 = 生成中, 10 = 结束
    pub state: u8,

    /// 流式消息 ID，首条为 None，后续用返回的 id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// 分片索引，从 0 开始递增
    pub index: u32,

    /// 是否重新生成（终结消息时为 true）
    #[serde(default)]
    pub reset: bool,
}

/// v2（群 / c2c）发送时必填的 `msg_type`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum MessageType {
    /// 纯文本。
    Text = 0,

    /// 图文混排。
    MixedImage = 1,

    /// markdown。
    Markdown = 2,

    /// ark 模板。
    Ark = 3,

    /// embed。
    Embed = 4,

    /// 富媒体（图 / 视频 / 语音 / 文件）。
    Media = 7,
}

/// 出站消息（v2 群 / c2c 共用）。
///
/// 起手用 [`OutgoingMessage::text`] / `markdown` / `ark` / `embed` / `media`
/// 选定主体，再链式 setter 加可选项：
///
/// ```ignore
/// let msg = OutgoingMessage::text("hello").reply_to("MSG_ID_FROM_EVENT");
/// bot.post_group_message("GROUP_OPENID", &msg).await?;
/// ```
///
/// **被动 vs 主动**：调 `.reply_to(msg_id)` / `.reply_to_event(event_id)` 之一
/// 是被动回复（5 分钟有效，无配额）；都不调即主动消息——QQ 对每应用 / 群每天
/// 有严格配额，超额按 [`crate::error::HttpError::ApiError`] 透出。开发期建议优先
/// 走被动回复。
///
/// **没有 quote API**：v2 群 / c2c 协议**不支持** `message_reference`。`reply_to`
/// 只用来：(1) 5 分钟内免主动配额发消息；(2) 后端日志关联原消息——**不是**给
/// 用户看的引用展现。客户端不会把被动回复渲染成引用样式（只是普通消息）。
/// 真正的"引用消息"卡片只在 v1 频道有效，走 [`OutgoingChannelMessage::quote`]。
#[derive(Debug, Clone, Serialize)]
pub struct OutgoingMessage {
    msg_type: MessageType,

    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    msg_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    msg_seq: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    event_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    markdown: Option<MarkdownPayload>,

    #[serde(skip_serializing_if = "Option::is_none")]
    keyboard: Option<KeyboardPayload>,

    #[serde(skip_serializing_if = "Option::is_none")]
    ark: Option<ArkPayload>,

    #[serde(skip_serializing_if = "Option::is_none")]
    embed: Option<EmbedPayload>,

    #[serde(skip_serializing_if = "Option::is_none")]
    media: Option<Media>,

    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<StreamState>,
}

impl OutgoingMessage {
    fn empty(msg_type: MessageType) -> Self {
        Self {
            msg_type,
            content: None,
            msg_id: None,
            msg_seq: None,
            event_id: None,
            markdown: None,
            keyboard: None,
            ark: None,
            embed: None,
            media: None,
            stream: None,
        }
    }

    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            ..Self::empty(MessageType::Text)
        }
    }

    pub fn markdown(payload: MarkdownPayload) -> Self {
        Self {
            markdown: Some(payload),
            ..Self::empty(MessageType::Markdown)
        }
    }

    pub fn ark(payload: ArkPayload) -> Self {
        Self {
            ark: Some(payload),
            ..Self::empty(MessageType::Ark)
        }
    }

    /// **协议限制**：embed 按官方文档**只在频道（含频道私信）支持**，v2 群 / c2c
    /// 不支持——客户端会 fallback 成"表情"占位。频道发 embed 走
    /// [`OutgoingChannelMessage::embed`]。
    pub fn embed(payload: EmbedPayload) -> Self {
        Self {
            embed: Some(payload),
            ..Self::empty(MessageType::Embed)
        }
    }

    pub fn media(media: Media) -> Self {
        Self {
            media: Some(media),
            ..Self::empty(MessageType::Media)
        }
    }

    /// 标记为对某条入站消息的被动回复。
    pub fn reply_to(mut self, msg_id: impl Into<String>) -> Self {
        self.msg_id = Some(msg_id.into());
        self
    }

    /// 标记为对某事件的被动回复（GUILD_MEMBER_ADD 之类，没有 msg_id 时用）。
    pub fn reply_to_event(mut self, event_id: impl Into<String>) -> Self {
        self.event_id = Some(event_id.into());
        self
    }

    /// 显式设置回复序号。同一 `msg_id` 下唯一——多次回复同一条消息时递增。
    pub fn reply_seq(mut self, seq: u32) -> Self {
        self.msg_seq = Some(seq);
        self
    }

    /// 当前 `msg_seq`——`None` 表示调用方未显式设置，`post_*_message` 会自动注入
    /// 一个全局递增值避免 QQ 的 `(msg_id, msg_seq)` 去重。
    pub fn msg_seq(&self) -> Option<u32> {
        self.msg_seq
    }

    /// 附加 inline 键盘——可与 markdown / text 等任一主体并存。
    pub fn with_keyboard(mut self, keyboard: KeyboardPayload) -> Self {
        self.keyboard = Some(keyboard);
        self
    }

    /// 补一段文本——markdown / media 主体为主时偶尔需要附带说明。
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// 获取文本内容（如果主体是文本）。
    pub fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }

    /// 消息类型（text / markdown / ark / embed / media 等）。
    pub fn msg_type(&self) -> MessageType {
        self.msg_type
    }

    /// 纯文本内容的字节长度——非文本主体返回 0。
    pub fn content_length(&self) -> usize {
        self.content.as_deref().map(|s| s.len()).unwrap_or(0)
    }

    /// 设置流式状态——用于流式 markdown 发送。
    pub fn with_stream(mut self, stream: StreamState) -> Self {
        self.stream = Some(stream);
        self
    }
}

/// 频道（v1）出站消息——schema 与 v2 不同（无 `msg_type`），单独成型。
#[derive(Debug, Clone, Serialize)]
pub struct OutgoingChannelMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    msg_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    event_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    markdown: Option<MarkdownPayload>,

    #[serde(skip_serializing_if = "Option::is_none")]
    keyboard: Option<KeyboardPayload>,

    #[serde(skip_serializing_if = "Option::is_none")]
    ark: Option<ArkPayload>,

    #[serde(skip_serializing_if = "Option::is_none")]
    embed: Option<EmbedPayload>,

    #[serde(skip_serializing_if = "Option::is_none")]
    message_reference: Option<MessageReference>,
}

impl OutgoingChannelMessage {
    fn empty() -> Self {
        Self {
            content: None,
            msg_id: None,
            event_id: None,
            markdown: None,
            keyboard: None,
            ark: None,
            embed: None,
            message_reference: None,
        }
    }

    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            ..Self::empty()
        }
    }

    pub fn markdown(payload: MarkdownPayload) -> Self {
        Self {
            markdown: Some(payload),
            ..Self::empty()
        }
    }

    pub fn ark(payload: ArkPayload) -> Self {
        Self {
            ark: Some(payload),
            ..Self::empty()
        }
    }

    pub fn embed(payload: EmbedPayload) -> Self {
        Self {
            embed: Some(payload),
            ..Self::empty()
        }
    }

    pub fn reply_to(mut self, msg_id: impl Into<String>) -> Self {
        self.msg_id = Some(msg_id.into());
        self
    }

    pub fn reply_to_event(mut self, event_id: impl Into<String>) -> Self {
        self.event_id = Some(event_id.into());
        self
    }

    pub fn with_keyboard(mut self, keyboard: KeyboardPayload) -> Self {
        self.keyboard = Some(keyboard);
        self
    }

    /// `ignore_missing = true`：原消息已被删也照样发，引用展示为空。
    pub fn quote(mut self, message_id: impl Into<String>, ignore_missing: bool) -> Self {
        self.message_reference = Some(MessageReference {
            message_id: message_id.into(),
            ignore_get_message_error: ignore_missing,
        });
        self
    }

    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// 获取文本内容（如果主体是文本）。
    pub fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }

    /// 纯文本内容的字节长度——非文本主体返回 0。
    pub fn content_length(&self) -> usize {
        self.content.as_deref().map(|s| s.len()).unwrap_or(0)
    }
}

/// 发送消息后的成功响应。
#[derive(Debug, Clone, Deserialize)]
pub struct SentMessage {
    /// 服务端为本次发送分配的消息 ID。
    pub id: String,

    /// ISO 8601。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::keyboard::{Action, Button, Keyboard, KeyboardRow, Permission, RenderData};
    use serde_json::Value;

    #[test]
    fn outgoing_text_serialization_is_minimal() {
        let v: Value = serde_json::to_value(OutgoingMessage::text("hi").reply_to("MID")).unwrap();
        assert_eq!(v["msg_type"], 0);
        assert_eq!(v["content"], "hi");
        assert_eq!(v["msg_id"], "MID");
        assert!(v.get("markdown").is_none());
        assert!(v.get("keyboard").is_none());
        assert!(v.get("ark").is_none());
    }

    #[test]
    fn outgoing_markdown_with_keyboard_serializes_both() {
        let md = MarkdownPayload::template("tmpl-1").param("name", ["alice"]);
        let kb = KeyboardPayload::inline(Keyboard {
            rows: vec![KeyboardRow {
                buttons: vec![Button {
                    id: "btn-1".into(),
                    render_data: RenderData {
                        label: "click".into(),
                        visited_label: "ok".into(),
                        style: 1,
                    },
                    action: Action {
                        action_type: 1,
                        permission: Permission {
                            permission_type: 2,
                            ..Default::default()
                        },
                        data: "callback-data".into(),
                        unsupport_tips: "客户端版本太低，请升级".into(),
                        reply: None,
                        enter: None,
                        anchor: None,
                    },
                }],
            }],
        });
        let msg = OutgoingMessage::markdown(md)
            .with_keyboard(kb)
            .reply_to("MID");
        let v: Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(v["msg_type"], 2);
        assert_eq!(v["markdown"]["custom_template_id"], "tmpl-1");
        assert_eq!(v["markdown"]["params"][0]["key"], "name");
        assert_eq!(
            v["keyboard"]["content"]["rows"][0]["buttons"][0]["id"],
            "btn-1"
        );
        assert_eq!(
            v["keyboard"]["content"]["rows"][0]["buttons"][0]["action"]["type"], 1,
            "Action.action_type 必须 rename 为 type"
        );
        assert!(v.get("ark").is_none());
    }

    #[test]
    fn outgoing_media_uses_file_info() {
        let media = Media {
            file_uuid: Some("uuid-x".into()),
            file_info: "opaque-info".into(),
            ttl: 0,
        };
        let v: Value = serde_json::to_value(OutgoingMessage::media(media)).unwrap();
        assert_eq!(v["msg_type"], 7);
        assert_eq!(v["media"]["file_info"], "opaque-info");
        assert_eq!(v["media"]["file_uuid"], "uuid-x");
    }

    #[test]
    fn outgoing_channel_no_msg_type() {
        let v: Value = serde_json::to_value(OutgoingChannelMessage::text("hi")).unwrap();
        assert!(v.get("msg_type").is_none(), "v1 不应有 msg_type: {v}");
    }

    #[test]
    fn outgoing_channel_quote_serializes_reference() {
        let v: Value =
            serde_json::to_value(OutgoingChannelMessage::text("re").quote("MID-orig", true))
                .unwrap();
        assert_eq!(v["message_reference"]["message_id"], "MID-orig");
        assert_eq!(v["message_reference"]["ignore_get_message_error"], true);
    }
}
