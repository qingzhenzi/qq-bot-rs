//! 频道禁言管理 API——全员禁言 / 单成员禁言 / 解除。

use crate::error::HttpError;
use crate::http::Bot;
use serde::Serialize;
use tracing::info;

/// 禁言请求——`mute_end_timestamp`（绝对秒）/ `mute_seconds`（相对秒）二选一。
/// 服务端 `mute_end_timestamp` 优先；传 `"0"` 表示解除禁言。
#[derive(Debug, Serialize)]
struct MuteRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    mute_end_timestamp: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    mute_seconds: Option<&'a str>,
}

/// 批量禁言请求——与单成员禁言同一路径，body 多 `user_ids` 数组。
#[derive(Debug, Serialize)]
struct MultiMuteRequest<'a> {
    user_ids: &'a [&'a str],

    #[serde(skip_serializing_if = "Option::is_none")]
    mute_end_timestamp: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    mute_seconds: Option<&'a str>,
}

impl Bot {
    /// `PATCH /guilds/{guild_id}/mute` —— 全员禁言。
    ///
    /// `mute_end_timestamp`（秒级 Unix 时间戳字符串）与 `mute_seconds`
    /// 二选一；都传时服务端以 `mute_end_timestamp` 为准。
    pub async fn mute_guild(
        &self,
        guild_id: &str,
        mute_end_timestamp: Option<&str>,
        mute_seconds: Option<&str>,
    ) -> Result<(), HttpError> {
        let path = format!("/guilds/{guild_id}/mute");
        let body = MuteRequest {
            mute_end_timestamp,
            mute_seconds,
        };
        info!(%guild_id, "[全员禁言]");
        self.patch_json_empty(&path, &body).await
    }

    /// `PATCH /guilds/{guild_id}/mute` —— 取消全员禁言。
    pub async fn unmute_guild(&self, guild_id: &str) -> Result<(), HttpError> {
        let path = format!("/guilds/{guild_id}/mute");
        let body = MuteRequest {
            mute_end_timestamp: Some("0"),
            mute_seconds: None,
        };
        info!(%guild_id, "[取消全员禁言]");
        self.patch_json_empty(&path, &body).await
    }

    /// `PATCH /guilds/{guild_id}/members/{user_id}/mute` —— 禁言单个成员。
    pub async fn mute_member(
        &self,
        guild_id: &str,
        user_id: &str,
        mute_end_timestamp: Option<&str>,
        mute_seconds: Option<&str>,
    ) -> Result<(), HttpError> {
        let path = format!("/guilds/{guild_id}/members/{user_id}/mute");
        let body = MuteRequest {
            mute_end_timestamp,
            mute_seconds,
        };
        info!(%guild_id, %user_id, "[禁言成员]");
        self.patch_json_empty(&path, &body).await
    }

    /// `PATCH /guilds/{guild_id}/members/{user_id}/mute` —— 解除单个成员禁言。
    pub async fn unmute_member(&self, guild_id: &str, user_id: &str) -> Result<(), HttpError> {
        let path = format!("/guilds/{guild_id}/members/{user_id}/mute");
        let body = MuteRequest {
            mute_end_timestamp: Some("0"),
            mute_seconds: None,
        };
        info!(%guild_id, %user_id, "[解除成员禁言]");
        self.patch_json_empty(&path, &body).await
    }

    /// `PATCH /guilds/{guild_id}/mute` —— 批量禁言多名成员。
    ///
    /// 与全员禁言同一路径，但 body 含 `user_ids` 数组时服务端按批量处理。
    pub async fn mute_multi_member(
        &self,
        guild_id: &str,
        user_ids: &[&str],
        mute_end_timestamp: Option<&str>,
        mute_seconds: Option<&str>,
    ) -> Result<(), HttpError> {
        let path = format!("/guilds/{guild_id}/mute");
        let body = MultiMuteRequest {
            user_ids,
            mute_end_timestamp,
            mute_seconds,
        };
        info!(%guild_id, count = user_ids.len(), "[批量禁言]");
        self.patch_json_empty(&path, &body).await
    }

    /// `PATCH /guilds/{guild_id}/mute` —— 批量解除多名成员禁言。
    pub async fn unmute_multi_member(
        &self,
        guild_id: &str,
        user_ids: &[&str],
    ) -> Result<(), HttpError> {
        let path = format!("/guilds/{guild_id}/mute");
        let body = MultiMuteRequest {
            user_ids,
            mute_end_timestamp: Some("0"),
            mute_seconds: None,
        };
        info!(%guild_id, count = user_ids.len(), "[批量解除禁言]");
        self.patch_json_empty(&path, &body).await
    }
}
