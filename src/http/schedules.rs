//! 频道日程 API。

use crate::error::HttpError;
use crate::http::Bot;
use serde::{Deserialize, Serialize};
use tracing::info;

/// 频道日程。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Schedule {
    /// 日程 ID。
    pub schedule_id: String,

    /// 频道 ID。
    pub guild_id: String,

    /// 日程名称。
    pub name: String,

    /// 日程描述。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// 开始时间（ISO 8601）。
    pub start_time: String,

    /// 结束时间（ISO 8601）。
    pub end_time: String,

    /// 创建人 user_id。
    pub creator_id: String,
}

#[derive(Debug, Serialize)]
struct CreateScheduleRequest<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    start_time: &'a str,
    end_time: &'a str,
}

#[derive(Debug, Serialize)]
struct UpdateScheduleRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_time: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_time: Option<&'a str>,
}

impl Bot {
    /// `GET /guilds/{guild_id}/schedules` —— 获取日程列表。
    pub async fn get_schedules(&self, guild_id: &str) -> Result<Vec<Schedule>, HttpError> {
        let path = format!("/guilds/{guild_id}/schedules");
        info!(%guild_id, "[获取日程列表]");
        self.get_json(&path).await
    }

    /// `GET /guilds/{guild_id}/schedules/{schedule_id}` —— 获取日程详情。
    pub async fn get_schedule(
        &self,
        guild_id: &str,
        schedule_id: &str,
    ) -> Result<Schedule, HttpError> {
        let path = format!("/guilds/{guild_id}/schedules/{schedule_id}");
        info!(%guild_id, %schedule_id, "[获取日程]");
        self.get_json(&path).await
    }

    /// `POST /guilds/{guild_id}/schedules` —— 创建日程。
    pub async fn create_schedule(
        &self,
        guild_id: &str,
        name: &str,
        description: Option<&str>,
        start_time: &str,
        end_time: &str,
    ) -> Result<Schedule, HttpError> {
        let path = format!("/guilds/{guild_id}/schedules");
        let body = CreateScheduleRequest {
            name,
            description,
            start_time,
            end_time,
        };
        info!(%guild_id, name, "[创建日程]");
        self.post_json(&path, &body).await
    }

    /// `PATCH /guilds/{guild_id}/schedules/{schedule_id}` —— 更新日程。
    pub async fn update_schedule(
        &self,
        guild_id: &str,
        schedule_id: &str,
        name: Option<&str>,
        description: Option<&str>,
        start_time: Option<&str>,
        end_time: Option<&str>,
    ) -> Result<Schedule, HttpError> {
        let path = format!("/guilds/{guild_id}/schedules/{schedule_id}");
        let body = UpdateScheduleRequest {
            name,
            description,
            start_time,
            end_time,
        };
        info!(%guild_id, %schedule_id, "[更新日程]");
        self.patch_json(&path, &body).await
    }

    /// `DELETE /guilds/{guild_id}/schedules/{schedule_id}` —— 删除日程。
    pub async fn delete_schedule(
        &self,
        guild_id: &str,
        schedule_id: &str,
    ) -> Result<(), HttpError> {
        let path = format!("/guilds/{guild_id}/schedules/{schedule_id}");
        info!(%guild_id, %schedule_id, "[删除日程]");
        self.delete_empty(&path).await
    }
}
