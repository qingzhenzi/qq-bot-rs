//! WebSocket 网关的线协议类型。
//!
//! `Payload` 是网关帧的通用信封——`d` 字段先按 `Value` 收，再由上层根据
//! `op` / `t` 解码到具体 data 结构。

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::intents::Intents;

/// 网关 OpCode——未知值反序列化失败是有意的，引入新 op 必须显式补，不静默吞。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum OpCode {
    /// 服务端向客户端推送事件。
    Dispatch = 0,

    /// 心跳——客户端定时上报，服务端也可能下发。
    Heartbeat = 1,

    /// 客户端鉴权（首次连接）。
    Identify = 2,

    /// 客户端断线重连，恢复 session。
    Resume = 6,

    /// 服务端通知客户端必须重新连接。
    Reconnect = 7,

    /// Identify / Resume 参数错误，session 不可用。
    InvalidSession = 9,

    /// 连接建立后服务端下发的第一帧，含心跳间隔。
    Hello = 10,

    /// 心跳上报后的 ACK。
    HeartbeatAck = 11,
}

/// 网关帧的通用信封：`{op, d, s?, t?}`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    /// 操作码。
    pub op: OpCode,

    /// 事件数据。Heartbeat 上报时是上次的 seq（数字或 null），所以保留 `Value`。
    #[serde(default)]
    pub d: Value,

    /// 事件序列号——只有 Dispatch 帧带，客户端要持久化以便 Resume。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s: Option<u64>,

    /// 事件类型名（`READY` / `AT_MESSAGE_CREATE` 等）——只有 Dispatch 帧带。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t: Option<String>,
}

/// `OpCode::Identify` 帧的 `d`。`Debug` 手写脱敏 `token`，避免凭证进日志。
#[derive(Clone, Serialize, Deserialize)]
pub struct IdentifyData {
    /// 鉴权 token，调用方拼好（`QQBot {access_token}`）。
    pub token: String,

    /// 订阅的事件类别位掩码。
    pub intents: Intents,

    /// 分片信息 `[shard_id, shard_count]`。
    pub shard: [u32; 2],

    /// 客户端属性（系统、设备、SDK 名等）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<Value>,
}

impl fmt::Debug for IdentifyData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IdentifyData")
            .field("token", &"***")
            .field("intents", &self.intents)
            .field("shard", &self.shard)
            .field("properties", &self.properties)
            .finish()
    }
}

/// `OpCode::Resume` 帧的 `d`。`Debug` 手写脱敏 `token`。
#[derive(Clone, Serialize, Deserialize)]
pub struct ResumeData {
    /// 鉴权 token。
    pub token: String,

    /// 上次连接拿到的 session_id。
    pub session_id: String,

    /// 客户端记录的最后 seq——服务端从这里之后补推漏掉的事件。
    pub seq: u64,
}

impl fmt::Debug for ResumeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResumeData")
            .field("token", &"***")
            .field("session_id", &self.session_id)
            .field("seq", &self.seq)
            .finish()
    }
}

/// `OpCode::Hello` 帧的 `d`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloData {
    /// 心跳间隔，毫秒。
    pub heartbeat_interval: u64,
}

/// Ready 事件中机器人自身的用户信息——字段集与通用 `User` 不同（带 `status`，
/// 不带 `union_*`），单独成型。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadyUser {
    /// 机器人用户 ID。
    pub id: String,

    /// 机器人昵称。
    pub username: String,

    /// 是否为 bot。
    #[serde(default)]
    pub bot: bool,

    /// 在线状态——协议未文档化具体取值，i32 透传。
    #[serde(default)]
    pub status: i32,

    /// 机器人头像 URL。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

/// `t = "READY"` 事件的 `d`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyData {
    /// 协议版本号。
    pub version: u32,

    /// 本次连接分配的 session_id，用于 Resume。
    pub session_id: String,

    /// 机器人自身信息。
    pub user: ReadyUser,

    /// 实际生效的 `[shard_id, shard_count]`，可能与 Identify 提交值不同。
    pub shard: [u32; 2],
}

/// 一帧 Dispatch（`op = 0`）。`data` 保留为 `Value`，由 [`crate::event`] 解码到强类型。
#[derive(Debug, Clone)]
pub struct DispatchEvent {
    /// 事件名（如 `"READY"` / `"AT_MESSAGE_CREATE"`）。
    pub event_type: String,

    /// 事件数据（原始 `d` 字段）。
    pub data: Value,

    /// 事件序列号——库内部自管，透出便于排查丢帧。
    pub seq: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_dispatch_decodes() {
        let raw = r#"{
            "op": 0,
            "s": 1,
            "t": "READY",
            "d": {
                "version": 1,
                "session_id": "sess-xyz",
                "user": {"id":"100","username":"my-bot","bot":true,"status":1,"avatar":"https://x"},
                "shard": [0, 1]
            }
        }"#;
        let p: Payload = serde_json::from_str(raw).unwrap();
        assert_eq!(p.op, OpCode::Dispatch);
        assert_eq!(p.t.as_deref(), Some("READY"));
        assert_eq!(p.s, Some(1));
        let ready: ReadyData = serde_json::from_value(p.d).unwrap();
        assert_eq!(ready.session_id, "sess-xyz");
        assert_eq!(ready.user.username, "my-bot");
        assert_eq!(ready.shard, [0, 1]);
    }

    #[test]
    fn identify_debug_masks_token() {
        let id = IdentifyData {
            token: "QQBot supersecret".to_owned(),
            intents: Intents::default_public(),
            shard: [0, 1],
            properties: None,
        };
        let s = format!("{id:?}");
        assert!(!s.contains("supersecret"), "{s}");
        assert!(s.contains("***"), "{s}");
    }

    #[test]
    fn resume_debug_masks_token() {
        let r = ResumeData {
            token: "QQBot supersecret".to_owned(),
            session_id: "sess-1".to_owned(),
            seq: 42,
        };
        let s = format!("{r:?}");
        assert!(!s.contains("supersecret"), "{s}");
        assert!(s.contains("sess-1"), "{s}");
    }
}
