//! 子频道权限管理 API。

use crate::error::HttpError;
use crate::http::Bot;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use tracing::info;

/// 子频道权限。
///
/// `permissions` 字段可能以字符串或数字形式返回，`string_or_number` 反序列化器兼容两者。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelPermissions {
    /// 子频道 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,

    /// 用户 ID（成员权限时回带）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// 身份组 ID（角色权限时回带）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_id: Option<String>,

    /// 权限值——可能为数字或字符串。
    #[serde(deserialize_with = "string_or_number")]
    pub permissions: String,
}

fn string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        _ => Err(D::Error::custom(
            "expected string or number for permissions",
        )),
    }
}

#[derive(Debug, Serialize)]
struct SetPermissionsRequest {
    /// 要添加的权限 bitfield 字符串。
    #[serde(skip_serializing_if = "Option::is_none")]
    add: Option<String>,

    /// 要移除的权限 bitfield 字符串。
    #[serde(skip_serializing_if = "Option::is_none")]
    remove: Option<String>,
}

impl Bot {
    /// `GET /channels/{channel_id}/permissions/role/{role_id}` —— 查询角色在子频道的权限。
    pub async fn get_channel_role_permissions(
        &self,
        channel_id: &str,
        role_id: &str,
    ) -> Result<ChannelPermissions, HttpError> {
        let path = format!("/channels/{channel_id}/permissions/role/{role_id}");
        info!(%channel_id, %role_id, "[获取角色子频道权限]");
        self.get_json(&path).await
    }

    /// `PUT /channels/{channel_id}/permissions/role/{role_id}` —— 设置角色在子频道的权限。
    ///
    /// `add` 和 `remove` 是权限 bitfield 字符串；至少传一个。
    pub async fn put_channel_role_permissions(
        &self,
        channel_id: &str,
        role_id: &str,
        add: Option<&str>,
        remove: Option<&str>,
    ) -> Result<(), HttpError> {
        let path = format!("/channels/{channel_id}/permissions/role/{role_id}");
        let body = SetPermissionsRequest {
            add: add.map(str::to_string),
            remove: remove.map(str::to_string),
        };
        info!(%channel_id, %role_id, "[设置角色子频道权限]");
        self.put_json_empty(&path, &body).await
    }

    /// `GET /channels/{channel_id}/permissions/member/{user_id}` —— 查询成员在子频道的权限。
    pub async fn get_channel_member_permissions(
        &self,
        channel_id: &str,
        user_id: &str,
    ) -> Result<ChannelPermissions, HttpError> {
        let path = format!("/channels/{channel_id}/permissions/member/{user_id}");
        info!(%channel_id, %user_id, "[获取成员子频道权限]");
        self.get_json(&path).await
    }

    /// `PUT /channels/{channel_id}/permissions/member/{user_id}` —— 设置成员在子频道的权限。
    pub async fn put_channel_member_permissions(
        &self,
        channel_id: &str,
        user_id: &str,
        add: Option<&str>,
        remove: Option<&str>,
    ) -> Result<(), HttpError> {
        let path = format!("/channels/{channel_id}/permissions/member/{user_id}");
        let body = SetPermissionsRequest {
            add: add.map(str::to_string),
            remove: remove.map(str::to_string),
        };
        info!(%channel_id, %user_id, "[设置成员子频道权限]");
        self.put_json_empty(&path, &body).await
    }
}
