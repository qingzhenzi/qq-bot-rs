//! 频道公告 API。

use crate::error::HttpError;
use crate::http::Bot;
use serde::{Deserialize, Serialize};
use tracing::info;

/// 频道公告。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Announce {
    /// 公告 ID。
    pub announce_id: String,

    /// 频道 ID。
    pub guild_id: String,

    /// 子频道 ID。
    pub channel_id: String,

    /// 公告内容。
    pub content: String,

    /// 创建时间（ISO 8601）。
    pub create_time: String,
}

#[derive(Debug, Serialize)]
struct CreateAnnounceRequest<'a> {
    content: &'a str,
}

impl Bot {
    /// `GET /guilds/{guild_id}/announces` —— 获取频道公告列表。
    pub async fn get_announces(&self, guild_id: &str) -> Result<Vec<Announce>, HttpError> {
        let path = format!("/guilds/{guild_id}/announces");
        info!(%guild_id, "[获取公告列表]");
        self.get_json(&path).await
    }

    /// `POST /guilds/{guild_id}/announces` —— 创建频道公告。
    pub async fn create_announce(
        &self,
        guild_id: &str,
        channel_id: &str,
        content: &str,
    ) -> Result<Announce, HttpError> {
        let path = format!("/guilds/{guild_id}/announces?channel_id={channel_id}");
        let body = CreateAnnounceRequest { content };
        info!(%guild_id, %channel_id, "[创建公告]");
        self.post_json(&path, &body).await
    }

    /// `DELETE /guilds/{guild_id}/announces/{announce_id}` —— 删除公告。
    pub async fn delete_announce(
        &self,
        guild_id: &str,
        announce_id: &str,
    ) -> Result<(), HttpError> {
        let path = format!("/guilds/{guild_id}/announces/{announce_id}");
        info!(%guild_id, %announce_id, "[删除公告]");
        self.delete_empty(&path).await
    }
}
