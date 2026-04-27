//! 频道消息表情表态——仅频道作用域可用，v2 群 / c2c 没有这套接口。
//!
//! PUT / DELETE 是 idempotent；GET 用 `cookie` 翻页，到末尾时 `is_end = true`。

use serde::Deserialize;
use serde_repr::{Deserialize_repr, Serialize_repr};
use url::form_urlencoded;

use crate::error::HttpError;
use crate::http::Bot;
use crate::types::user::User;

/// 表情类型——type 与 id 必须配对，type 选错即便 id 对也找不到表情。
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum EmojiType {
    /// 系统表情——`id` 形如 `"4"`、`"38"`。
    System = 1,

    /// emoji unicode——`id` 为十进制码点字符串，如 `"129315"`。
    Emoji = 2,
}

impl EmojiType {
    fn as_path_segment(self) -> u8 {
        self as u8
    }
}

/// 表情表态用户列表的一页。
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ReactionUsersPage {
    /// 当前页用户。
    #[serde(default)]
    pub users: Vec<User>,

    /// 翻页游标——传给下次请求继续往后取。
    #[serde(default)]
    pub cookie: String,

    /// 是否已到末尾。
    #[serde(default)]
    pub is_end: bool,
}

impl Bot {
    /// `PUT /channels/{channel_id}/messages/{message_id}/reactions/{type}/{id}`——加表情。
    pub async fn put_channel_reaction(
        &self,
        channel_id: &str,
        message_id: &str,
        emoji_type: EmojiType,
        emoji_id: &str,
    ) -> Result<(), HttpError> {
        let t = emoji_type.as_path_segment();
        let path = format!("/channels/{channel_id}/messages/{message_id}/reactions/{t}/{emoji_id}");
        self.put_empty(&path).await
    }

    /// `DELETE /channels/{channel_id}/messages/{message_id}/reactions/{type}/{id}`——
    /// 撤回表情。只能撤机器人自己加过的。
    pub async fn delete_channel_reaction(
        &self,
        channel_id: &str,
        message_id: &str,
        emoji_type: EmojiType,
        emoji_id: &str,
    ) -> Result<(), HttpError> {
        let t = emoji_type.as_path_segment();
        let path = format!("/channels/{channel_id}/messages/{message_id}/reactions/{t}/{emoji_id}");
        self.delete_empty(&path).await
    }

    /// `GET /channels/{channel_id}/messages/{message_id}/reactions/{type}/{id}`——
    /// 拉表态用户列表（分页）。首次 `cookie` 传 `None`，后续传上一页返回的 `cookie` 直到 `is_end`。
    pub async fn list_channel_reaction_users(
        &self,
        channel_id: &str,
        message_id: &str,
        emoji_type: EmojiType,
        emoji_id: &str,
        cookie: Option<&str>,
    ) -> Result<ReactionUsersPage, HttpError> {
        let t = emoji_type.as_path_segment();
        let mut path =
            format!("/channels/{channel_id}/messages/{message_id}/reactions/{t}/{emoji_id}");
        if let Some(c) = cookie {
            // cookie 是不透明字符串，可能含 +/=/& 等会破坏查询串的字符。
            let encoded: String = form_urlencoded::byte_serialize(c.as_bytes()).collect();
            path.push_str("?cookie=");
            path.push_str(&encoded);
        }
        self.get_json(&path).await
    }
}
