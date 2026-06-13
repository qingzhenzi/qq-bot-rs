//! 消息审核事件 DTO——覆盖 `MESSAGE_AUDIT_PASS` / `MESSAGE_AUDIT_REJECT`。

use serde::{Deserialize, Serialize};

/// 消息审核结果事件——通过/拒绝共用。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    /// 频道 ID。
    pub guild_id: String,

    /// 子频道 ID。
    pub channel_id: String,

    /// 被审核消息的 ID。
    pub message_id: String,

    /// 审核结果：`0` = 通过，`1` = 拒绝。
    pub audit_result: u8,

    /// 拒绝原因（仅拒绝时可能回填）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// 审核消息序列号。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq: Option<String>,
}
