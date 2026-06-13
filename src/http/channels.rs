//! 子频道（Channel）管理 API。

use crate::error::HttpError;
use crate::http::Bot;
use crate::types::guild::Channel;
use crate::types::member::GuildMemberEntry;
use serde::{Deserialize, Serialize};
use tracing::info;

/// 子频道创建请求。
#[derive(Debug, Clone, Serialize)]
struct CreateChannelRequest<'a> {
    name: &'a str,

    /// 子频道类型（0=文字, 2=语音, 4=分组, 10005=直播, 10006=应用, 10007=论坛）。
    #[serde(rename = "type")]
    type_: u32,

    /// 父频道 ID（嵌套子频道用）。
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_id: Option<&'a str>,
}

/// 子频道更新请求。
#[derive(Debug, Clone, Serialize)]
struct PatchChannelRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    parent_id: Option<&'a str>,
}

/// 语音频道成员。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceMember {
    /// QQ 内部用户 ID。
    #[serde(default)]
    pub uid: String,

    /// 成员信息。
    pub member: GuildMemberEntry,
}

impl Bot {
    /// `GET /guilds/{guild_id}/channels` —— 获取频道下的子频道列表。
    pub async fn get_channels(&self, guild_id: &str) -> Result<Vec<Channel>, HttpError> {
        let path = format!("/guilds/{guild_id}/channels");
        info!(%guild_id, "[获取子频道列表]");
        self.get_json(&path).await
    }

    /// `GET /channels/{channel_id}` —— 获取子频道详情。
    pub async fn get_channel(&self, channel_id: &str) -> Result<Channel, HttpError> {
        let path = format!("/channels/{channel_id}");
        info!(%channel_id, "[获取子频道]");
        self.get_json(&path).await
    }

    /// `POST /guilds/{guild_id}/channels` —— 创建子频道。
    pub async fn create_channel(
        &self,
        guild_id: &str,
        name: &str,
        type_: u32,
        parent_id: Option<&str>,
    ) -> Result<Channel, HttpError> {
        let path = format!("/guilds/{guild_id}/channels");
        let body = CreateChannelRequest {
            name,
            type_,
            parent_id,
        };
        info!(%guild_id, name, type_, "[创建子频道]");
        self.post_json(&path, &body).await
    }

    /// `PATCH /channels/{channel_id}` —— 更新子频道。
    pub async fn patch_channel(
        &self,
        channel_id: &str,
        name: Option<&str>,
        position: Option<u32>,
        parent_id: Option<&str>,
    ) -> Result<Channel, HttpError> {
        let path = format!("/channels/{channel_id}");
        let body = PatchChannelRequest {
            name,
            position,
            parent_id,
        };
        info!(%channel_id, "[更新子频道]");
        self.patch_json(&path, &body).await
    }

    /// `DELETE /channels/{channel_id}` —— 删除子频道。
    pub async fn delete_channel(&self, channel_id: &str) -> Result<(), HttpError> {
        let path = format!("/channels/{channel_id}");
        info!(%channel_id, "[删除子频道]");
        self.delete_empty(&path).await
    }

    /// `GET /channels/{channel_id}/voice/members` —— 获取语音频道成员列表。
    pub async fn get_voice_members(&self, channel_id: &str) -> Result<Vec<VoiceMember>, HttpError> {
        let path = format!("/channels/{channel_id}/voice/members");
        info!(%channel_id, "[获取语音频道成员]");
        self.get_json(&path).await
    }
}
