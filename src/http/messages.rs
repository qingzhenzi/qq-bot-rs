//! 消息发送 / 撤回 / 富媒体上传。
//!
//! v1（频道）与 v2（群、c2c）schema 不同：v1 没有 `msg_type` 字段，所以分两个
//! DTO（[`OutgoingChannelMessage`] / [`OutgoingMessage`]），调用方按场景选。

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::Serialize;
use tracing::{debug, info};

use crate::error::HttpError;
use crate::http::Bot;
use crate::types::message::{ChannelMessage, OutgoingChannelMessage, OutgoingMessage, SentMessage};
use crate::types::payloads::{FileType, MarkdownPayload, Media};

/// 富媒体上传请求体——`url` 与 `file_data` 互斥，二选一。
///
/// - `url`：让 server 去拉，适合已有公网 URL 的素材
/// - `file_data`：base64 内联，适合本地生成（TTS 字节、临时截图）无现成 URL 的情况
#[derive(Debug, Serialize)]
struct PostFileRequest<'a> {
    file_type: FileType,

    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    file_data: Option<String>,

    // true：服务端拉到后直接发消息（占主动消息配额）；false：仅上传拿 file_info。
    srv_send_msg: bool,
}

impl Bot {
    /// `POST /v2/groups/{group_openid}/messages`。
    ///
    /// 被动回复在 [`OutgoingMessage`] 上调 `.reply_to(msg_id)`，5 分钟内有效；
    /// 主动消息有日配额限制。
    ///
    /// 调用方未显式 [`OutgoingMessage::reply_seq`] 时本方法会**自动注入**全局递增
    /// 的 `msg_seq`，避开 QQ 的 `(msg_id, msg_seq)` 去重——同 `msg_id` 下连发多条
    /// 否则会被去重 40054005。
    pub async fn post_group_message(
        &self,
        group_openid: &str,
        msg: &OutgoingMessage,
    ) -> Result<SentMessage, HttpError> {
        let path = format!("/v2/groups/{group_openid}/messages");
        let prepared = self.with_default_msg_seq(msg);
        info!(%group_openid, "[发送群消息]");
        debug!(msg_type = ?prepared.msg_type(), content_len = prepared.content_length(), "[消息内容]");
        self.post_json(&path, &prepared).await
    }

    /// `POST /v2/users/{openid}/messages`（c2c 私聊）。
    ///
    /// `msg_seq` 自动注入逻辑同 [`Self::post_group_message`]。
    pub async fn post_c2c_message(
        &self,
        openid: &str,
        msg: &OutgoingMessage,
    ) -> Result<SentMessage, HttpError> {
        let path = format!("/v2/users/{openid}/messages");
        let prepared = self.with_default_msg_seq(msg);
        info!(%openid, "[发送私聊消息]");
        debug!(msg_type = ?prepared.msg_type(), content_len = prepared.content_length(), "[消息内容]");
        self.post_json(&path, &prepared).await
    }

    /// 调用方没设 `msg_seq` 时填一个全局递增值；设过的尊重原值。
    fn with_default_msg_seq(&self, msg: &OutgoingMessage) -> OutgoingMessage {
        if msg.msg_seq().is_some() {
            return msg.clone();
        }
        msg.clone().reply_seq(self.next_msg_seq())
    }

    /// `POST /channels/{channel_id}/messages`（v1 频道）。
    pub async fn post_channel_message(
        &self,
        channel_id: &str,
        msg: &OutgoingChannelMessage,
    ) -> Result<SentMessage, HttpError> {
        let path = format!("/channels/{channel_id}/messages");
        info!(%channel_id, "[发送频道消息]");
        debug!(content_len = msg.content_length(), "[频道消息内容]");
        self.post_json(&path, msg).await
    }

    /// `POST /dms/{guild_id}/messages`（频道私信）。
    ///
    /// `guild_id` 不是用户所在公会，而是私信会话本身的 guild——QQ 把每段私信
    /// 当独立 guild 路由，由 [`Self::create_dm`] 或 DM 事件回带。
    pub async fn post_dm_message(
        &self,
        guild_id: &str,
        msg: &OutgoingChannelMessage,
    ) -> Result<SentMessage, HttpError> {
        let path = format!("/dms/{guild_id}/messages");
        info!(%guild_id, "[发送频道私信]");
        debug!(content_len = msg.content_length(), "[DM消息内容]");
        self.post_json(&path, msg).await
    }

    /// `DELETE /channels/{channel_id}/messages/{message_id}`。
    /// `hide_tip = true` 隐藏"消息已撤回"小灰条。
    pub async fn delete_channel_message(
        &self,
        channel_id: &str,
        message_id: &str,
        hide_tip: bool,
    ) -> Result<(), HttpError> {
        let path = format!(
            "/channels/{channel_id}/messages/{message_id}?hidetip={}",
            if hide_tip { "true" } else { "false" }
        );
        self.delete_empty(&path).await
    }

    /// `DELETE /v2/groups/{group_openid}/messages/{message_id}`。
    ///
    /// v2 群 / c2c 协议规定消息发送超 2 分钟即不可撤回，超时按
    /// [`HttpError::ApiError`] 透出。
    pub async fn delete_group_message(
        &self,
        group_openid: &str,
        message_id: &str,
    ) -> Result<(), HttpError> {
        let path = format!("/v2/groups/{group_openid}/messages/{message_id}");
        self.delete_empty(&path).await
    }

    /// `DELETE /v2/users/{openid}/messages/{message_id}`。同 [`Self::delete_group_message`] 受 2 分钟时效限制。
    pub async fn delete_c2c_message(
        &self,
        openid: &str,
        message_id: &str,
    ) -> Result<(), HttpError> {
        let path = format!("/v2/users/{openid}/messages/{message_id}");
        self.delete_empty(&path).await
    }

    /// `DELETE /dms/{guild_id}/messages/{message_id}?hidetip={hide_tip}`。
    pub async fn delete_dm_message(
        &self,
        guild_id: &str,
        message_id: &str,
        hide_tip: bool,
    ) -> Result<(), HttpError> {
        let path = format!(
            "/dms/{guild_id}/messages/{message_id}?hidetip={}",
            if hide_tip { "true" } else { "false" }
        );
        self.delete_empty(&path).await
    }

    /// 上传富媒体到群（URL 模式）。`media_url` 必须**外网可访问**——QQ 服务端去拉。
    /// `send_immediately = true` 时服务端拉到后直接发消息（占主动配额，不再返回 `file_info`）；
    /// 常规两步用法传 `false`，再把返回的 [`Media`] 塞进 [`OutgoingMessage::media`] 发送。
    ///
    /// 本地生成（TTS / 截图 / 临时文件）等没现成 URL 的场景用 [`Self::post_group_file_bytes`]。
    pub async fn post_group_file(
        &self,
        group_openid: &str,
        file_type: FileType,
        media_url: &str,
        send_immediately: bool,
    ) -> Result<Media, HttpError> {
        let path = format!("/v2/groups/{group_openid}/files");
        let body = PostFileRequest {
            file_type,
            url: Some(media_url),
            file_data: None,
            srv_send_msg: send_immediately,
        };
        self.post_json(&path, &body).await
    }

    /// 上传富媒体到群（base64 字节内联模式）——QQ 协议 `file_data` 字段。
    /// 适合本地字节无现成公网 URL 的场景；avoid 自己起 host / tunnel。
    /// 语义同 [`Self::post_group_file`]：`send_immediately = false` 拿 `file_info` 走两步发。
    pub async fn post_group_file_bytes(
        &self,
        group_openid: &str,
        file_type: FileType,
        bytes: &[u8],
        send_immediately: bool,
    ) -> Result<Media, HttpError> {
        let path = format!("/v2/groups/{group_openid}/files");
        let body = PostFileRequest {
            file_type,
            url: None,
            file_data: Some(BASE64.encode(bytes)),
            srv_send_msg: send_immediately,
        };
        self.post_json(&path, &body).await
    }

    /// 上传富媒体到 c2c（URL 模式）。语义同 [`Self::post_group_file`]。
    pub async fn post_c2c_file(
        &self,
        openid: &str,
        file_type: FileType,
        media_url: &str,
        send_immediately: bool,
    ) -> Result<Media, HttpError> {
        let path = format!("/v2/users/{openid}/files");
        let body = PostFileRequest {
            file_type,
            url: Some(media_url),
            file_data: None,
            srv_send_msg: send_immediately,
        };
        self.post_json(&path, &body).await
    }

    /// 上传富媒体到 c2c（base64 字节内联模式）。语义同 [`Self::post_group_file_bytes`]。
    pub async fn post_c2c_file_bytes(
        &self,
        openid: &str,
        file_type: FileType,
        bytes: &[u8],
        send_immediately: bool,
    ) -> Result<Media, HttpError> {
        let path = format!("/v2/users/{openid}/files");
        let body = PostFileRequest {
            file_type,
            url: None,
            file_data: Some(BASE64.encode(bytes)),
            srv_send_msg: send_immediately,
        };
        self.post_json(&path, &body).await
    }

    /// 流式发送群消息（markdown）。按行切分，逐块发送，最后终结。
    /// `max_chars_per_chunk` 控制每块最大字符数，默认 50。
    pub async fn post_group_message_streaming(
        &self,
        group_openid: &str,
        markdown_content: &str,
        max_chars_per_chunk: Option<usize>,
    ) -> Result<SentMessage, HttpError> {
        use crate::types::message::StreamState;
        use crate::types::payloads::MarkdownPayload;

        let max_chars = max_chars_per_chunk.unwrap_or(50);
        let chunks = split_by_lines(markdown_content, max_chars);
        let total = chunks.len();

        let mut stream_id: Option<String> = None;
        let mut msg_seq = self.next_msg_seq();

        // 逐块发送（state=1，生成中）
        for (i, chunk) in chunks.iter().enumerate() {
            let msg = OutgoingMessage::markdown(MarkdownPayload::raw(chunk.clone()))
                .reply_seq(msg_seq)
                .with_stream(StreamState {
                    state: 1,
                    id: stream_id.clone(),
                    index: i as u32,
                    reset: false,
                });

            let path = format!("/v2/groups/{group_openid}/messages");
            let result: SentMessage = self.post_json(&path, &msg).await?;
            stream_id = Some(result.id.clone());
            msg_seq = self.next_msg_seq();

            info!(
                "[流式群消息] 群: {}, 进度: {}/{}",
                group_openid,
                i + 1,
                total
            );
        }

        // 发送终结消息（state=10，reset=true，全量文本）
        let final_msg =
            OutgoingMessage::markdown(MarkdownPayload::raw(markdown_content.to_string()))
                .reply_seq(msg_seq)
                .with_stream(StreamState {
                    state: 10,
                    id: stream_id,
                    index: 1,
                    reset: true,
                });

        let path = format!("/v2/groups/{group_openid}/messages");
        let final_result: SentMessage = self.post_json(&path, &final_msg).await?;

        info!(
            "[流式群消息完成] 群: {}, 共 {} 块 + 1 终结",
            group_openid, total
        );

        Ok(final_result)
    }

    /// 流式发送私聊消息（markdown）。语义同 [`Self::post_group_message_streaming`]。
    pub async fn post_c2c_message_streaming(
        &self,
        openid: &str,
        markdown_content: &str,
        max_chars_per_chunk: Option<usize>,
    ) -> Result<SentMessage, HttpError> {
        use crate::types::message::StreamState;
        use crate::types::payloads::MarkdownPayload;

        let max_chars = max_chars_per_chunk.unwrap_or(50);
        let chunks = split_by_lines(markdown_content, max_chars);
        let total = chunks.len();

        let mut stream_id: Option<String> = None;
        let mut msg_seq = self.next_msg_seq();

        for (i, chunk) in chunks.iter().enumerate() {
            let msg = OutgoingMessage::markdown(MarkdownPayload::raw(chunk.clone()))
                .reply_seq(msg_seq)
                .with_stream(StreamState {
                    state: 1,
                    id: stream_id.clone(),
                    index: i as u32,
                    reset: false,
                });

            let path = format!("/v2/users/{openid}/messages");
            let result: SentMessage = self.post_json(&path, &msg).await?;
            stream_id = Some(result.id.clone());
            msg_seq = self.next_msg_seq();

            info!("[流式私聊消息] 用户: {}, 进度: {}/{}", openid, i + 1, total);
        }

        let final_msg =
            OutgoingMessage::markdown(MarkdownPayload::raw(markdown_content.to_string()))
                .reply_seq(msg_seq)
                .with_stream(StreamState {
                    state: 10,
                    id: stream_id,
                    index: 1,
                    reset: true,
                });

        let path = format!("/v2/users/{openid}/messages");
        let final_result: SentMessage = self.post_json(&path, &final_msg).await?;

        info!(
            "[流式私聊消息完成] 用户: {}, 共 {} 块 + 1 终结",
            openid, total
        );

        Ok(final_result)
    }

    /// `GET /channels/{channel_id}/messages/{message_id}` —— 获取指定频道消息。
    pub async fn get_channel_message(
        &self,
        channel_id: &str,
        message_id: &str,
    ) -> Result<ChannelMessage, HttpError> {
        let path = format!("/channels/{channel_id}/messages/{message_id}");
        info!(%channel_id, %message_id, "[获取频道消息]");
        self.get_json(&path).await
    }

    /// `PATCH /channels/{channel_id}/messages/{message_id}` —— 编辑频道消息。
    ///
    /// 仅已开通编辑消息权限的机器人可用；`content` 与 `markdown` 至少传一个。
    pub async fn patch_channel_message(
        &self,
        channel_id: &str,
        message_id: &str,
        content: Option<&str>,
        markdown: Option<&MarkdownPayload>,
    ) -> Result<ChannelMessage, HttpError> {
        #[derive(Debug, Serialize)]
        struct PatchChannelMessageRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            content: Option<&'a str>,

            #[serde(skip_serializing_if = "Option::is_none")]
            markdown: Option<&'a MarkdownPayload>,
        }

        let path = format!("/channels/{channel_id}/messages/{message_id}");
        let body = PatchChannelMessageRequest { content, markdown };
        info!(%channel_id, %message_id, "[编辑频道消息]");
        self.patch_json(&path, &body).await
    }
}

/// 按行切分文本，保证每个分片以 `\n` 结尾。
fn split_by_lines(text: &str, max_chars: usize) -> Vec<String> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut chunks = Vec::new();
    let mut current = String::new();

    for (i, line) in lines.iter().enumerate() {
        let piece = if i < lines.len() - 1 {
            format!("{}\n", line)
        } else {
            line.to_string()
        };

        if current.len() + piece.len() <= max_chars {
            current.push_str(&piece);
        } else {
            if !current.is_empty() {
                if !current.ends_with('\n') {
                    current.push('\n');
                }
                chunks.push(current);
            }
            current = piece;
        }
    }

    if !current.is_empty() {
        if !current.ends_with('\n') {
            current.push('\n');
        }
        chunks.push(current);
    }

    chunks
}
