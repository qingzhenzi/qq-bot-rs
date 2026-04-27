//! 按钮交互 ACK。
//!
//! 网关收到 `INTERACTION_CREATE` 后**必须**在 5s 内调本接口，否则客户端按钮
//! 一直停在 loading。这里只回状态码——给用户的反馈消息要另行调
//! `post_*_message`。

use serde::Serialize;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::error::HttpError;
use crate::http::Bot;

#[derive(Debug, Serialize)]
struct InteractionCallback {
    code: InteractionCallbackCode,
}

/// 按钮回调结果码——决定客户端 loading 收尾及给用户的提示文案。
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum InteractionCallbackCode {
    /// 操作成功。
    Success = 0,

    /// 操作失败。
    Failure = 1,

    /// 操作频繁，触发频控。
    RateLimited = 2,

    /// 重复操作。
    Duplicate = 3,

    /// 没有权限。
    NoPermission = 4,

    /// 仅管理员可操作。
    AdminOnly = 5,
}

impl Bot {
    /// `PUT /interactions/{interaction_id}`。
    ///
    /// `interaction_id` 来自 `INTERACTION_CREATE` 事件的 `id` 字段。延迟过久
    /// 客户端按钮会一直 loading。
    pub async fn put_interaction_callback(
        &self,
        interaction_id: &str,
        code: InteractionCallbackCode,
    ) -> Result<(), HttpError> {
        let path = format!("/interactions/{interaction_id}");
        let body = InteractionCallback { code };
        self.put_json_empty(&path, &body).await
    }
}
