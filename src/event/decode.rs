use serde_json::{Error as JsonError, Value};
use thiserror::Error;

use crate::types::gateway::{DispatchEvent, ReadyData};
use crate::types::interaction::Interaction;
use crate::types::manage::{GroupManageEvent, UserManageEvent};
use crate::types::message::{C2cMessage, ChannelMessage, GroupMessage};

/// `Event::from_dispatch` 失败时连同 `event_type` 一起返回——避免调用方为
/// 错误日志预先 clone 事件名（每帧热路径）。
#[derive(Debug, Error)]
#[error("decode event {event_type:?} failed: {source}")]
pub struct DispatchDecodeError {
    /// 原始事件名（`t` 字段值）。
    pub event_type: String,

    /// schema 不匹配明细——`Display` 含哪个字段类型错或缺失。
    #[source]
    pub source: JsonError,
}

/// 派发到 handler 的强类型事件。
///
/// 消息类变体用 `Box` 包一层——`ChannelMessage` 等含多个 `String` / `Vec`，
/// 直接放 enum 会让整个 `Event` 被最大变体撑到 500B+，每次过 mpsc 都要搬。
/// `Box` 后整个 enum 退到 ~32B，仅一次堆分配。
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    /// 握手完成（`READY`）。
    Ready(ReadyData),

    /// Resume 成功（`RESUMED`）。
    Resumed,

    /// 频道 @ 机器人消息（`AT_MESSAGE_CREATE`）。
    AtMessageCreate(Box<ChannelMessage>),

    /// 群 @ 机器人消息（`GROUP_AT_MESSAGE_CREATE`）。
    GroupAtMessageCreate(Box<GroupMessage>),

    /// 私聊（C2C）消息（`C2C_MESSAGE_CREATE`）。
    C2cMessageCreate(Box<C2cMessage>),

    /// 用户添加机器人为好友（`FRIEND_ADD`）。
    FriendAdd(UserManageEvent),

    /// 用户从好友列表移除机器人（`FRIEND_DEL`）。
    FriendDel(UserManageEvent),

    /// 用户在资料卡关闭主动消息推送（`C2C_MSG_REJECT`）。
    C2cMsgReject(UserManageEvent),

    /// 用户重新打开主动消息推送（`C2C_MSG_RECEIVE`）。
    C2cMsgReceive(UserManageEvent),

    /// 机器人被加入群（`GROUP_ADD_ROBOT`）。
    GroupAddRobot(GroupManageEvent),

    /// 机器人被移出群（`GROUP_DEL_ROBOT`）。
    GroupDelRobot(GroupManageEvent),

    /// 群管理员关闭机器人主动消息（`GROUP_MSG_REJECT`）。
    GroupMsgReject(GroupManageEvent),

    /// 群管理员开启机器人主动消息（`GROUP_MSG_RECEIVE`）。
    GroupMsgReceive(GroupManageEvent),

    /// 按钮 / 快捷菜单交互（`INTERACTION_CREATE`）。须在 5s 内调
    /// [`crate::http::Bot::put_interaction_callback`] ACK。
    InteractionCreate(Box<Interaction>),

    /// 库未识别的事件——保留原始 payload。
    Unknown {
        /// 事件名（`t` 字段原值）。
        name: String,

        /// 事件数据。
        data: Value,

        /// 事件序列号。
        seq: u64,
    },
}

impl Event {
    /// 把网关原始 [`DispatchEvent`] 转成强类型 [`Event`]。
    ///
    /// - 已知事件名 + schema 匹配 → `Ok(Event::xxx)`
    /// - 已知事件名 + schema 不匹配 → `Err(DispatchDecodeError)`（含 `event_type`）
    /// - 未知事件名 → `Ok(Event::Unknown { ... })`
    pub fn from_dispatch(d: DispatchEvent) -> Result<Self, DispatchDecodeError> {
        let DispatchEvent {
            event_type,
            data,
            seq,
        } = d;
        let decoded: Result<Event, JsonError> = match event_type.as_str() {
            "READY" => serde_json::from_value(data).map(Event::Ready),
            "RESUMED" => Ok(Event::Resumed),
            "AT_MESSAGE_CREATE" => {
                serde_json::from_value(data).map(|m| Event::AtMessageCreate(Box::new(m)))
            }
            "GROUP_AT_MESSAGE_CREATE" => {
                serde_json::from_value(data).map(|m| Event::GroupAtMessageCreate(Box::new(m)))
            }
            "C2C_MESSAGE_CREATE" => {
                serde_json::from_value(data).map(|m| Event::C2cMessageCreate(Box::new(m)))
            }
            "FRIEND_ADD" => serde_json::from_value(data).map(Event::FriendAdd),
            "FRIEND_DEL" => serde_json::from_value(data).map(Event::FriendDel),
            "C2C_MSG_REJECT" => serde_json::from_value(data).map(Event::C2cMsgReject),
            "C2C_MSG_RECEIVE" => serde_json::from_value(data).map(Event::C2cMsgReceive),
            "GROUP_ADD_ROBOT" => serde_json::from_value(data).map(Event::GroupAddRobot),
            "GROUP_DEL_ROBOT" => serde_json::from_value(data).map(Event::GroupDelRobot),
            "GROUP_MSG_REJECT" => serde_json::from_value(data).map(Event::GroupMsgReject),
            "GROUP_MSG_RECEIVE" => serde_json::from_value(data).map(Event::GroupMsgReceive),
            "INTERACTION_CREATE" => {
                serde_json::from_value(data).map(|i| Event::InteractionCreate(Box::new(i)))
            }
            _ => {
                return Ok(Event::Unknown {
                    name: event_type,
                    data,
                    seq,
                });
            }
        };
        decoded.map_err(|source| DispatchDecodeError { event_type, source })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dispatch(name: &str, data: Value) -> DispatchEvent {
        DispatchEvent {
            event_type: name.to_owned(),
            data,
            seq: 1,
        }
    }

    #[test]
    fn unknown_event_falls_back() {
        let raw = json!({"foo": "bar"});
        let ev = Event::from_dispatch(dispatch("MADE_UP_EVENT", raw.clone())).unwrap();
        let Event::Unknown { name, data, seq } = ev else {
            panic!("expected Unknown");
        };
        assert_eq!(name, "MADE_UP_EVENT");
        assert_eq!(data, raw);
        assert_eq!(seq, 1);
    }

    #[test]
    fn group_message_decodes() {
        let raw = json!({
            "id": "msg-1",
            "group_openid": "GROUP_X",
            "author": {"member_openid": "MEMBER_Y"},
            "content": "hi",
            "timestamp": "2026-04-27T03:00:00+00:00"
        });
        let ev = Event::from_dispatch(dispatch("GROUP_AT_MESSAGE_CREATE", raw)).unwrap();
        let Event::GroupAtMessageCreate(m) = ev else {
            panic!("expected GroupAtMessageCreate");
        };
        assert_eq!(m.id, "msg-1");
        assert_eq!(m.group_openid, "GROUP_X");
        assert_eq!(m.author.member_openid, "MEMBER_Y");
    }

    #[test]
    fn known_event_with_bad_schema_errors() {
        let bad = json!({"missing_required_fields": true});
        let res = Event::from_dispatch(dispatch("AT_MESSAGE_CREATE", bad));
        assert!(res.is_err());
    }

    #[test]
    fn friend_add_decodes_with_scene() {
        let raw = json!({
            "timestamp": 1714464000,
            "openid": "OPENID-1",
            "scene": "Group",
            "scene_param": "campaign-42",
        });
        let ev = Event::from_dispatch(dispatch("FRIEND_ADD", raw)).unwrap();
        let Event::FriendAdd(e) = ev else {
            panic!("expected FriendAdd");
        };
        assert_eq!(e.openid, "OPENID-1");
        assert_eq!(e.scene.as_deref(), Some("Group"));
        assert_eq!(e.scene_param.as_deref(), Some("campaign-42"));
    }

    #[test]
    fn friend_del_decodes_without_scene() {
        let raw = json!({
            "timestamp": 1714464100,
            "openid": "OPENID-2",
        });
        let ev = Event::from_dispatch(dispatch("FRIEND_DEL", raw)).unwrap();
        let Event::FriendDel(e) = ev else {
            panic!("expected FriendDel");
        };
        assert_eq!(e.openid, "OPENID-2");
        assert!(e.scene.is_none());
    }

    #[test]
    fn interaction_create_group_button_decodes() {
        use crate::types::interaction::{ChatType, InteractionType};
        let raw = json!({
            "id": "IID-1",
            "type": 11,
            "chat_type": 1,
            "scene": "group",
            "timestamp": "2026-04-30T12:00:00+00:00",
            "version": 1,
            "group_openid": "GRP-1",
            "group_member_openid": "MBR-1",
            "data": {
                "type": 1,
                "resolved": {
                    "button_id": "btn-callback",
                    "button_data": "callback-payload-1",
                    "message_id": "MID-orig",
                }
            }
        });
        let ev = Event::from_dispatch(dispatch("INTERACTION_CREATE", raw)).unwrap();
        let Event::InteractionCreate(i) = ev else {
            panic!("expected InteractionCreate");
        };
        assert_eq!(i.id, "IID-1");
        assert_eq!(i.interaction_type, InteractionType::MessageButton);
        assert_eq!(i.chat_type, ChatType::Group);
        assert_eq!(i.group_openid.as_deref(), Some("GRP-1"));
        assert_eq!(i.group_member_openid.as_deref(), Some("MBR-1"));
        assert!(i.guild_id.is_none());
        assert_eq!(i.data.resolved.button_id, "btn-callback");
        assert_eq!(i.data.resolved.button_data, "callback-payload-1");
        assert_eq!(i.data.resolved.message_id.as_deref(), Some("MID-orig"));
    }
}
