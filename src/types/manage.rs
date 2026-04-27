//! 用户 / 群管理事件 DTO——同一域内多个事件 schema 一致，共用一个 DTO，由
//! [`crate::event::Event`] 的变体名区分语义。归在 intent `1<<25`（`PUBLIC_MESSAGES`）下。

use serde::{Deserialize, Serialize};

/// 覆盖 `FRIEND_ADD` / `FRIEND_DEL` / `C2C_MSG_REJECT` / `C2C_MSG_RECEIVE`。
/// `scene` / `scene_param` 仅 `FRIEND_ADD` 走分享链接添加时回填。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserManageEvent {
    pub timestamp: i64,

    pub openid: String,

    /// 添加来源场景。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,

    /// `generate_url_link` 上报的 callback_data 原值——开发者侧追踪转化用。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene_param: Option<String>,
}

/// 覆盖 `GROUP_ADD_ROBOT` / `GROUP_DEL_ROBOT` / `GROUP_MSG_REJECT` / `GROUP_MSG_RECEIVE`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupManageEvent {
    pub timestamp: i64,

    pub group_openid: String,

    pub op_member_openid: String,
}
