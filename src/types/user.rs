use serde::{Deserialize, Serialize};

/// 频道用户。字段在不同事件中出现的子集不同，可能缺失的一律 `Option`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: String,

    pub username: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,

    #[serde(default)]
    pub bot: bool,

    /// 跨频道的 union openid（仅部分事件回填）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub union_openid: Option<String>,

    /// union 账号 ID（仅部分事件回填）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub union_user_account: Option<String>,
}

/// 频道成员（用户 + 该频道下的成员属性）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Member {
    /// 关联的用户对象——某些事件下服务端不回填。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nick: Option<String>,

    #[serde(default)]
    pub roles: Vec<String>,

    /// 加入频道的时间（ISO 8601）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub joined_at: Option<String>,
}
