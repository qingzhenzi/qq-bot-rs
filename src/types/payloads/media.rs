use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// `POST /v2/{groups|users}/.../files` 必填字段。`File` 暂未对开发者开放。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum FileType {
    /// 图片：png / jpg。
    Image = 1,

    /// 视频：mp4。
    Video = 2,

    /// 语音：silk。
    Voice = 3,

    /// 文件——暂未开放。
    File = 4,
}

/// 富媒体载荷——由 [`crate::http::Bot::post_group_file`] / `post_c2c_file`
/// 上传后返回，直接塞进 [`crate::types::message::OutgoingMessage::media`] 发送。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Media {
    /// 文件 ID——发消息时用 `file_info` 而非 `file_uuid`，但保留供日志 / 排错。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_uuid: Option<String>,

    /// 发消息接口 `media.file_info` 字段使用的不透明字符串。
    pub file_info: String,

    /// 有效期（秒）；`0` 表示长期有效。
    #[serde(default)]
    pub ttl: u32,
}
