use serde::{Deserialize, Serialize};

use crate::error::HttpError;
use crate::http::Bot;

#[derive(Debug, Serialize)]
struct CreateDmRequest<'a> {
    recipient_id: &'a str,
    source_guild_id: &'a str,
}

/// 私信会话——`guild_id` 配合 [`crate::http::Bot::post_dm_message`] 发文 / 撤回。
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DmSession {
    /// 私信 guild ID——发送 / 撤回都用它。
    pub guild_id: String,

    /// 私信子频道 ID（当前发送接口走 `/dms/{guild_id}/...` 不用它）。
    pub channel_id: String,

    /// 创建时间（ISO 8601）。
    #[serde(default)]
    pub create_time: Option<String>,
}

impl Bot {
    /// `POST /users/@me/dms`——创建私信会话。`@me` 是字面量，服务端按 token 解析机器人身份。
    ///
    /// `recipient_id` 必须是 `source_guild_id` 的成员，否则服务端拒。返回的
    /// [`DmSession::guild_id`] 传给 [`Self::post_dm_message`] 即可发私信。
    pub async fn create_dm(
        &self,
        recipient_id: &str,
        source_guild_id: &str,
    ) -> Result<DmSession, HttpError> {
        let body = CreateDmRequest {
            recipient_id,
            source_guild_id,
        };
        self.post_json("/users/@me/dms", &body).await
    }
}
