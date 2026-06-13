//! 频道身份组（Role）管理 API。

use crate::error::HttpError;
use crate::http::Bot;
use crate::types::member::GuildMemberEntry;
use serde::{Deserialize, Serialize};
use tracing::info;

/// 频道身份组。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub color: u32,

    /// 是否在成员列表中单独展示。
    #[serde(default)]
    pub hoist: bool,

    /// 当前身份组人数。
    #[serde(default)]
    pub member_count: u32,

    /// 身份组人数上限。
    #[serde(default)]
    pub member_limit: u32,

    /// 身份组描述。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// 角色成员列表分页响应。
#[derive(Debug, Clone, Deserialize)]
pub struct RoleMemberPage {
    #[serde(default)]
    pub data: Vec<GuildMemberEntry>,

    /// 下一页游标；`None` 或空字符串表示已到末尾。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

/// 身份组列表响应。
#[derive(Debug, Clone, Deserialize)]
pub struct RolePage {
    #[serde(default)]
    pub roles: Vec<Role>,

    /// 频道可创建的身份组数量上限（字符串形式）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_num_limit: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateRoleRequest<'a> {
    name: &'a str,

    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    hoist: Option<bool>,
}

#[derive(Debug, Serialize)]
struct PatchRoleRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    hoist: Option<bool>,
}

impl Bot {
    /// `GET /guilds/{guild_id}/roles` —— 获取身份组列表。
    pub async fn get_roles(&self, guild_id: &str) -> Result<RolePage, HttpError> {
        let path = format!("/guilds/{guild_id}/roles");
        info!(%guild_id, "[获取身份组列表]");
        self.get_json(&path).await
    }

    /// `POST /guilds/{guild_id}/roles` —— 创建身份组。
    pub async fn create_role(
        &self,
        guild_id: &str,
        name: &str,
        color: Option<u32>,
        hoist: Option<bool>,
    ) -> Result<Role, HttpError> {
        let path = format!("/guilds/{guild_id}/roles");
        let body = CreateRoleRequest { name, color, hoist };
        info!(%guild_id, name, "[创建身份组]");
        self.post_json(&path, &body).await
    }

    /// `PATCH /guilds/{guild_id}/roles/{role_id}` —— 更新身份组。
    pub async fn patch_role(
        &self,
        guild_id: &str,
        role_id: &str,
        name: Option<&str>,
        color: Option<u32>,
        hoist: Option<bool>,
    ) -> Result<Role, HttpError> {
        let path = format!("/guilds/{guild_id}/roles/{role_id}");
        let body = PatchRoleRequest { name, color, hoist };
        info!(%guild_id, %role_id, "[更新身份组]");
        self.patch_json(&path, &body).await
    }

    /// `DELETE /guilds/{guild_id}/roles/{role_id}` —— 删除身份组。
    pub async fn delete_role(&self, guild_id: &str, role_id: &str) -> Result<(), HttpError> {
        let path = format!("/guilds/{guild_id}/roles/{role_id}");
        info!(%guild_id, %role_id, "[删除身份组]");
        self.delete_empty(&path).await
    }

    /// `PUT /guilds/{guild_id}/members/{user_id}/roles/{role_id}` —— 给成员添加身份组。
    pub async fn put_member_role(
        &self,
        guild_id: &str,
        user_id: &str,
        role_id: &str,
    ) -> Result<(), HttpError> {
        let path = format!("/guilds/{guild_id}/members/{user_id}/roles/{role_id}");
        info!(%guild_id, %user_id, %role_id, "[添加成员身份组]");
        self.put_empty(&path).await
    }

    /// `DELETE /guilds/{guild_id}/members/{user_id}/roles/{role_id}` —— 移除成员身份组。
    pub async fn delete_member_role(
        &self,
        guild_id: &str,
        user_id: &str,
        role_id: &str,
    ) -> Result<(), HttpError> {
        let path = format!("/guilds/{guild_id}/members/{user_id}/roles/{role_id}");
        info!(%guild_id, %user_id, %role_id, "[移除成员身份组]");
        self.delete_empty(&path).await
    }

    /// `GET /guilds/{guild_id}/roles/{role_id}/members` —— 获取拥有该身份组的成员列表（分页）。
    ///
    /// `start_index` 为上一页返回的 `next` 游标；`limit` 默认 100。
    pub async fn get_role_members(
        &self,
        guild_id: &str,
        role_id: &str,
        start_index: Option<&str>,
        limit: Option<u32>,
    ) -> Result<RoleMemberPage, HttpError> {
        let mut path = format!("/guilds/{guild_id}/roles/{role_id}/members");
        let mut has_qs = false;
        if let Some(si) = start_index {
            path.push_str(&format!("?start_index={si}"));
            has_qs = true;
        }
        if let Some(l) = limit {
            path.push_str(if has_qs { "&limit=" } else { "?limit=" });
            path.push_str(&l.to_string());
        }
        info!(%guild_id, %role_id, "[获取身份组成员]");
        self.get_json(&path).await
    }
}
