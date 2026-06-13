//! 音频控制 API。

use crate::error::HttpError;
use crate::http::Bot;
use serde::Serialize;
use tracing::info;

/// 音频控制请求。
#[derive(Debug, Serialize)]
struct AudioControlRequest<'a> {
    /// 音频控制事件类型。
    /// - `0` = Start
    /// - `1` = Pause
    /// - `2` = Resume
    /// - `3` = Stop
    /// - `4` = Seek
    audio_type: u32,

    /// 音频控制状态。
    status: u32,

    /// 拉取进度（毫秒），仅 Seek 时带。
    #[serde(skip_serializing_if = "Option::is_none")]
    current_seek: Option<u32>,

    /// 目标音轨标识。
    #[serde(skip_serializing_if = "Option::is_none")]
    target_id: Option<&'a str>,

    /// 状态说明文本。
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<&'a str>,
}

impl Bot {
    /// `POST /channels/{channel_id}/audio` —— 发送音频控制指令。
    ///
    /// `audio_type`：`0` = Start, `1` = Pause, `2` = Resume, `3` = Stop, `4` = Seek。
    /// - `current_seek`：拉取进度（毫秒），仅 Seek 时设置。
    /// - `target_id`：目标音轨标识。
    /// - `text`：状态说明文本。
    pub async fn post_audio_control(
        &self,
        channel_id: &str,
        audio_type: u32,
        status: u32,
        current_seek: Option<u32>,
        target_id: Option<&str>,
        text: Option<&str>,
    ) -> Result<(), HttpError> {
        let path = format!("/channels/{channel_id}/audio");
        let body = AudioControlRequest {
            audio_type,
            status,
            current_seek,
            target_id,
            text,
        };
        info!(%channel_id, audio_type, status, "[音频控制]");
        self.post_json(&path, &body).await
    }
}
