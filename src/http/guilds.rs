//! 频道（Guild）管理 API。

use crate::error::HttpError;
use crate::http::Bot;
use crate::types::guild::Guild;
use crate::types::member::{GuildMemberEntry, GuildMemberPage};
use serde::{Deserialize, Serialize};
use tracing::info;

/// API 权限标识。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiIdentify {
    pub path: String,
    pub method: String,
}

/// API 权限条目。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiPermissionEntry {
    pub api: ApiIdentify,
    /// 授权状态：1=已授权，2=待授权，3=拒绝。
    pub auth_status: u32,
}

/// API 权限列表响应。
#[derive(Debug, Clone, Deserialize)]
pub struct ApiPermissionList {
    #[serde(default)]
    pub permissions: Vec<ApiPermissionEntry>,
}

/// API 权限申请响应。
#[derive(Debug, Clone, Deserialize)]
pub struct ApiPermissionDemand {
    pub task_id: String,
    pub status: u32,
}

#[derive(Debug, Serialize)]
struct ApiPermissionDemandRequest<'a> {
    channel_id: &'a str,
    api_identify: ApiIdentifyRef<'a>,
    desc: &'a str,
}

#[derive(Debug, Serialize)]
struct ApiIdentifyRef<'a> {
    path: &'a str,
    method: &'a str,
}

#[derive(Debug, Serialize)]
struct PatchGuildRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<&'a str>,
}

impl Bot {
    /// `GET /guilds/{guild_id}` —— 获取频道信息。
    pub async fn get_guild(&self, guild_id: &str) -> Result<Guild, HttpError> {
        let path = format!("/guilds/{guild_id}");
        info!(%guild_id, "[获取频道]");
        self.get_json(&path).await
    }

    /// `DELETE /guilds/{guild_id}` —— 解散频道。须频道主权限。
    pub async fn delete_guild(&self, guild_id: &str) -> Result<(), HttpError> {
        let path = format!("/guilds/{guild_id}");
        info!(%guild_id, "[解散频道]");
        self.delete_empty(&path).await
    }

    /// `PATCH /guilds/{guild_id}` —— 更新频道信息（名称 / 描述 / 图标）。
    pub async fn patch_guild(
        &self,
        guild_id: &str,
        name: Option<&str>,
        description: Option<&str>,
        icon: Option<&str>,
    ) -> Result<Guild, HttpError> {
        let path = format!("/guilds/{guild_id}");
        let body = PatchGuildRequest {
            name,
            description,
            icon,
        };
        info!(%guild_id, "[更新频道]");
        self.patch_json(&path, &body).await
    }

    /// `GET /guilds/{guild_id}/members` —— 获取频道成员列表（分页）。
    ///
    /// API 返回 `{"data": [...], "next": "..."}` 形态。
    /// `after` 为上一页返回的 `next` 游标；`limit` 默认 100，最大 400。
    pub async fn get_guild_members(
        &self,
        guild_id: &str,
        limit: Option<u32>,
        after: Option<&str>,
    ) -> Result<GuildMemberPage, HttpError> {
        let mut path = format!("/guilds/{guild_id}/members");
        let mut has_qs = false;
        if let Some(l) = limit {
            path.push_str(&format!("?limit={l}"));
            has_qs = true;
        }
        if let Some(a) = after {
            path.push_str(if has_qs { "&after=" } else { "?after=" });
            path.push_str(a);
        }
        info!(%guild_id, "[获取频道成员列表]");
        self.get_json(&path).await
    }

    /// `DELETE /guilds/{guild_id}/members/{user_id}` —— 踢出频道成员。
    ///
    /// `add_blacklist` 为 `true` 时同时加入黑名单。
    /// `delete_history_days` 为 `Some(n)` 时同时删除该用户 `n` 天内的消息（0-7）。
    pub async fn delete_guild_member(
        &self,
        guild_id: &str,
        user_id: &str,
        add_blacklist: bool,
        delete_history_days: Option<u32>,
    ) -> Result<(), HttpError> {
        let mut path = format!(
            "/guilds/{guild_id}/members/{user_id}?add_blacklist={}",
            if add_blacklist { "true" } else { "false" }
        );
        if let Some(days) = delete_history_days {
            path.push_str(&format!("&delete_history_days={days}"));
        }
        info!(%guild_id, %user_id, add_blacklist, delete_history_days, "[踢出频道成员]");
        self.delete_empty(&path).await
    }

    /// `GET /guilds/{guild_id}/members/{user_id}` —— 获取单个成员信息。
    pub async fn get_guild_member(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<GuildMemberEntry, HttpError> {
        let path = format!("/guilds/{guild_id}/members/{user_id}");
        info!(%guild_id, %user_id, "[获取频道成员]");
        self.get_json(&path).await
    }

    /// `GET /guilds/{guild_id}/api_permissions` —— 查询频道 API 权限列表。
    pub async fn get_api_permissions(
        &self,
        guild_id: &str,
    ) -> Result<ApiPermissionList, HttpError> {
        let path = format!("/guilds/{guild_id}/api_permissions");
        info!(%guild_id, "[获取 API 权限列表]");
        self.get_json(&path).await
    }

    /// `POST /guilds/{guild_id}/api_permissions/demand` —— 申请 API 权限。
    pub async fn demand_api_permission(
        &self,
        guild_id: &str,
        channel_id: &str,
        api_path: &str,
        api_method: &str,
        desc: &str,
    ) -> Result<ApiPermissionDemand, HttpError> {
        let path = format!("/guilds/{guild_id}/api_permissions/demand");
        let body = ApiPermissionDemandRequest {
            channel_id,
            api_identify: ApiIdentifyRef {
                path: api_path,
                method: api_method,
            },
            desc,
        };
        info!(%guild_id, api_path, api_method, "[申请 API 权限]");
        self.post_json(&path, &body).await
    }
}
