use serde_json::{Error as JsonError, Value};
use thiserror::Error;

use crate::types::audio::AudioEvent;
use crate::types::audit::AuditEvent;
use crate::types::forum::ForumThreadEvent;
use crate::types::gateway::{DispatchEvent, ReadyData};
use crate::types::guild::{Channel, Guild};
use crate::types::interaction::Interaction;
use crate::types::manage::{GroupManageEvent, UserManageEvent};
use crate::types::member::{GuildMemberAddEvent, GuildMemberRemoveEvent, GuildMemberUpdateEvent};
use crate::types::message::{C2cMessage, ChannelMessage, GroupMessage};
use crate::types::message_delete::MessageDeleteEvent;
use crate::types::reaction::ReactionEvent;

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

    /// 原始事件数据——解码失败时用于排查。
    pub raw_data: Value,
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

    /// 群内所有消息（`GROUP_MESSAGE_CREATE`）。
    GroupMessageCreate(Box<GroupMessage>),

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

    /// 频道创建（`GUILD_CREATE`）。
    GuildCreate(Guild),

    /// 频道信息更新（`GUILD_UPDATE`）。
    GuildUpdate(Guild),

    /// 频道解散或机器人被移除（`GUILD_DELETE`）。
    GuildDelete(Guild),

    /// 子频道创建（`CHANNEL_CREATE`）。
    ChannelCreate(Box<Channel>),

    /// 子频道信息更新（`CHANNEL_UPDATE`）。
    ChannelUpdate(Box<Channel>),

    /// 子频道删除（`CHANNEL_DELETE`）。
    ChannelDelete(Box<Channel>),

    /// 成员加入频道（`GUILD_MEMBER_ADD`）。
    GuildMemberAdd(Box<GuildMemberAddEvent>),

    /// 成员信息更新（`GUILD_MEMBER_UPDATE`）。
    GuildMemberUpdate(Box<GuildMemberUpdateEvent>),

    /// 成员被移出频道（`GUILD_MEMBER_REMOVE`）。
    GuildMemberRemove(Box<GuildMemberRemoveEvent>),

    /// 频道消息表态添加（`MESSAGE_REACTION_ADD`）。
    ReactionAdd(Box<ReactionEvent>),

    /// 频道消息表态移除（`MESSAGE_REACTION_REMOVE`）。
    ReactionRemove(Box<ReactionEvent>),

    /// 频道私信创建（`DIRECT_MESSAGE_CREATE`）。
    DirectMessageCreate(Box<ChannelMessage>),

    /// 频道私信/公开消息删除（`DIRECT_MESSAGE_DELETE` / `PUBLIC_MESSAGE_DELETE`）。
    MessageDelete(Box<MessageDeleteEvent>),

    /// 消息审核通过（`MESSAGE_AUDIT_PASS`）。
    AuditPass(Box<AuditEvent>),

    /// 消息审核拒绝（`MESSAGE_AUDIT_REJECT`）。
    AuditReject(Box<AuditEvent>),

    /// 论坛帖子创建（`FORUM_THREAD_CREATE`）。
    ForumThreadCreate(Box<ForumThreadEvent>),

    /// 论坛帖子更新（`FORUM_THREAD_UPDATE`）。
    ForumThreadUpdate(Box<ForumThreadEvent>),

    /// 论坛帖子删除（`FORUM_THREAD_DELETE`）。
    ForumThreadDelete(Box<ForumThreadEvent>),

    /// 公域论坛帖子创建（`OPEN_FORUM_THREAD_CREATE`）。
    OpenForumThreadCreate(Box<ForumThreadEvent>),

    /// 公域论坛帖子更新（`OPEN_FORUM_THREAD_UPDATE`）。
    OpenForumThreadUpdate(Box<ForumThreadEvent>),

    /// 公域论坛帖子删除（`OPEN_FORUM_THREAD_DELETE`）。
    OpenForumThreadDelete(Box<ForumThreadEvent>),

    /// 论坛评论创建（`OPEN_FORUM_POST_CREATE`）——注意与 `OPEN_FORUM_THREAD_CREATE` 不同。
    ForumPostCreate(Box<ForumThreadEvent>),

    /// 论坛评论删除（`OPEN_FORUM_POST_DELETE`）。
    ForumPostDelete(Box<ForumThreadEvent>),

    /// 音频开始播放（`AUDIO_START`）。
    AudioStart(Box<AudioEvent>),

    /// 音频结束播放（`AUDIO_FINISH`）。
    AudioFinish(Box<AudioEvent>),

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
        let data_clone = data.clone();
        let decoded: Result<Event, JsonError> = match event_type.as_str() {
            "READY" => serde_json::from_value(data).map(Event::Ready),
            "RESUMED" => Ok(Event::Resumed),
            "AT_MESSAGE_CREATE" => {
                serde_json::from_value(data).map(|m| Event::AtMessageCreate(Box::new(m)))
            }
            "GROUP_AT_MESSAGE_CREATE" => {
                serde_json::from_value(data).map(|m| Event::GroupAtMessageCreate(Box::new(m)))
            }
            "GROUP_MESSAGE_CREATE" => {
                serde_json::from_value(data).map(|m| Event::GroupMessageCreate(Box::new(m)))
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
            "GUILD_CREATE" => serde_json::from_value(data).map(Event::GuildCreate),
            "GUILD_UPDATE" => serde_json::from_value(data).map(Event::GuildUpdate),
            "GUILD_DELETE" => serde_json::from_value(data).map(Event::GuildDelete),
            "CHANNEL_CREATE" => {
                serde_json::from_value(data).map(|c| Event::ChannelCreate(Box::new(c)))
            }
            "CHANNEL_UPDATE" => {
                serde_json::from_value(data).map(|c| Event::ChannelUpdate(Box::new(c)))
            }
            "CHANNEL_DELETE" => {
                serde_json::from_value(data).map(|c| Event::ChannelDelete(Box::new(c)))
            }
            "GUILD_MEMBER_ADD" => {
                serde_json::from_value(data).map(|e| Event::GuildMemberAdd(Box::new(e)))
            }
            "GUILD_MEMBER_UPDATE" => {
                serde_json::from_value(data).map(|e| Event::GuildMemberUpdate(Box::new(e)))
            }
            "GUILD_MEMBER_REMOVE" => {
                serde_json::from_value(data).map(|e| Event::GuildMemberRemove(Box::new(e)))
            }
            "MESSAGE_REACTION_ADD" => {
                serde_json::from_value(data).map(|e| Event::ReactionAdd(Box::new(e)))
            }
            "MESSAGE_REACTION_REMOVE" => {
                serde_json::from_value(data).map(|e| Event::ReactionRemove(Box::new(e)))
            }
            "DIRECT_MESSAGE_CREATE" => {
                serde_json::from_value(data).map(|m| Event::DirectMessageCreate(Box::new(m)))
            }
            "DIRECT_MESSAGE_DELETE" => {
                serde_json::from_value(data).map(|e| Event::MessageDelete(Box::new(e)))
            }
            "PUBLIC_MESSAGE_DELETE" => {
                serde_json::from_value(data).map(|e| Event::MessageDelete(Box::new(e)))
            }
            "MESSAGE_AUDIT_PASS" => {
                serde_json::from_value(data).map(|e| Event::AuditPass(Box::new(e)))
            }
            "MESSAGE_AUDIT_REJECT" => {
                serde_json::from_value(data).map(|e| Event::AuditReject(Box::new(e)))
            }
            "FORUM_THREAD_CREATE" => {
                serde_json::from_value(data).map(|e| Event::ForumThreadCreate(Box::new(e)))
            }
            "OPEN_FORUM_THREAD_CREATE" => {
                serde_json::from_value(data).map(|e| Event::OpenForumThreadCreate(Box::new(e)))
            }
            "FORUM_THREAD_UPDATE" => {
                serde_json::from_value(data).map(|e| Event::ForumThreadUpdate(Box::new(e)))
            }
            "OPEN_FORUM_THREAD_UPDATE" => {
                serde_json::from_value(data).map(|e| Event::OpenForumThreadUpdate(Box::new(e)))
            }
            "FORUM_THREAD_DELETE" => {
                serde_json::from_value(data).map(|e| Event::ForumThreadDelete(Box::new(e)))
            }
            "OPEN_FORUM_THREAD_DELETE" => {
                serde_json::from_value(data).map(|e| Event::OpenForumThreadDelete(Box::new(e)))
            }
            "OPEN_FORUM_POST_CREATE" => {
                serde_json::from_value(data).map(|e| Event::ForumPostCreate(Box::new(e)))
            }
            "OPEN_FORUM_POST_DELETE" => {
                serde_json::from_value(data).map(|e| Event::ForumPostDelete(Box::new(e)))
            }
            "AUDIO_START" => serde_json::from_value(data).map(|e| Event::AudioStart(Box::new(e))),
            "AUDIO_FINISH" => serde_json::from_value(data).map(|e| Event::AudioFinish(Box::new(e))),
            _ => {
                return Ok(Event::Unknown {
                    name: event_type,
                    data,
                    seq,
                });
            }
        };
        decoded.map_err(|source| DispatchDecodeError {
            event_type,
            source,
            raw_data: data_clone,
        })
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

    // ─── Guild / Channel / Member event tests ───

    #[test]
    fn guild_create_decodes() {
        let raw = json!({
            "id": "G-001",
            "name": "My Guild",
            "owner_id": "U-OWNER",
            "op_user_id": "U-OP"
        });
        let ev = Event::from_dispatch(dispatch("GUILD_CREATE", raw)).unwrap();
        let Event::GuildCreate(g) = ev else {
            panic!("expected GuildCreate");
        };
        assert_eq!(g.id, "G-001");
        assert_eq!(g.name, "My Guild");
        assert_eq!(g.owner_id.as_deref(), Some("U-OWNER"));
    }

    #[test]
    fn guild_update_decodes() {
        let raw = json!({
            "id": "G-001",
            "name": "Updated Guild",
            "op_user_id": "U-OP"
        });
        let ev = Event::from_dispatch(dispatch("GUILD_UPDATE", raw)).unwrap();
        let Event::GuildUpdate(g) = ev else {
            panic!("expected GuildUpdate");
        };
        assert_eq!(g.name, "Updated Guild");
    }

    #[test]
    fn guild_delete_decodes() {
        let raw = json!({
            "id": "G-001",
            "name": "Old Guild",
            "op_user_id": "U-OP"
        });
        let ev = Event::from_dispatch(dispatch("GUILD_DELETE", raw)).unwrap();
        let Event::GuildDelete(g) = ev else {
            panic!("expected GuildDelete");
        };
        assert_eq!(g.id, "G-001");
    }

    #[test]
    fn channel_create_decodes() {
        let raw = json!({
            "id": "CH-1",
            "guild_id": "G-001",
            "name": "general",
            "type": 0,
            "op_user_id": "U-OP"
        });
        let ev = Event::from_dispatch(dispatch("CHANNEL_CREATE", raw)).unwrap();
        let Event::ChannelCreate(c) = ev else {
            panic!("expected ChannelCreate");
        };
        assert_eq!(c.id, "CH-1");
        assert_eq!(c.guild_id, "G-001");
        assert_eq!(c.type_, 0);
    }

    #[test]
    fn channel_update_decodes() {
        let raw = json!({
            "id": "CH-1",
            "guild_id": "G-001",
            "name": "general-renamed",
            "type": 0
        });
        let ev = Event::from_dispatch(dispatch("CHANNEL_UPDATE", raw)).unwrap();
        let Event::ChannelUpdate(c) = ev else {
            panic!("expected ChannelUpdate");
        };
        assert_eq!(c.name, "general-renamed");
    }

    #[test]
    fn channel_delete_decodes() {
        let raw = json!({
            "id": "CH-1",
            "guild_id": "G-001",
            "name": "obsolete",
            "type": 0,
            "op_user_id": "U-OP"
        });
        let ev = Event::from_dispatch(dispatch("CHANNEL_DELETE", raw)).unwrap();
        let Event::ChannelDelete(c) = ev else {
            panic!("expected ChannelDelete");
        };
        assert_eq!(c.id, "CH-1");
    }

    #[test]
    fn guild_member_add_decodes() {
        let raw = json!({
            "guild_id": "G-001",
            "joined_at": "2026-04-27T03:00:00+00:00",
            "user": {
                "id": "U-NEW",
                "username": "new_user",
                "bot": false
            },
            "nick": "Newbie"
        });
        let ev = Event::from_dispatch(dispatch("GUILD_MEMBER_ADD", raw)).unwrap();
        let Event::GuildMemberAdd(e) = ev else {
            panic!("expected GuildMemberAdd");
        };
        assert_eq!(e.guild_id, "G-001");
        assert_eq!(e.user.id, "U-NEW");
        assert_eq!(e.nick.as_deref(), Some("Newbie"));
    }

    #[test]
    fn guild_member_update_decodes() {
        let raw = json!({
            "guild_id": "G-001",
            "user": {
                "id": "U-1",
                "username": "updated_user",
                "bot": false
            },
            "nick": "Renamed",
            "roles": ["role-admin"],
            "op_user_id": "U-ADMIN"
        });
        let ev = Event::from_dispatch(dispatch("GUILD_MEMBER_UPDATE", raw)).unwrap();
        let Event::GuildMemberUpdate(e) = ev else {
            panic!("expected GuildMemberUpdate");
        };
        assert_eq!(e.guild_id, "G-001");
        assert_eq!(e.user.username, "updated_user");
        assert_eq!(e.roles, vec!["role-admin"]);
        assert_eq!(e.op_user_id.as_deref(), Some("U-ADMIN"));
    }

    #[test]
    fn guild_member_remove_decodes() {
        let raw = json!({
            "guild_id": "G-001",
            "user": {
                "id": "U-LEFT",
                "username": "leaving_user",
                "bot": false
            },
            "op_user_id": "U-KICKER"
        });
        let ev = Event::from_dispatch(dispatch("GUILD_MEMBER_REMOVE", raw)).unwrap();
        let Event::GuildMemberRemove(e) = ev else {
            panic!("expected GuildMemberRemove");
        };
        assert_eq!(e.guild_id, "G-001");
        assert_eq!(e.user.id, "U-LEFT");
        assert_eq!(e.op_user_id.as_deref(), Some("U-KICKER"));
    }

    // ─── Reaction / DM / Audit / Forum / Audio event tests ───

    #[test]
    fn reaction_add_decodes() {
        let raw = json!({
            "guild_id": "G-001",
            "channel_id": "CH-1",
            "user_id": "U-REACT",
            "message_id": "MID-1",
            "target": { "id": "128077", "type": 1 }
        });
        let ev = Event::from_dispatch(dispatch("MESSAGE_REACTION_ADD", raw)).unwrap();
        let Event::ReactionAdd(e) = ev else {
            panic!("expected ReactionAdd");
        };
        assert_eq!(e.guild_id, "G-001");
        assert_eq!(e.user_id, "U-REACT");
        assert_eq!(e.target.id, "128077");
        assert_eq!(e.target.type_, 1);
    }

    #[test]
    fn reaction_remove_decodes() {
        let raw = json!({
            "guild_id": "G-001",
            "channel_id": "CH-1",
            "user_id": "U-UNDO",
            "message_id": "MID-1",
            "target": { "id": "custom-emoji-id", "type": 2 }
        });
        let ev = Event::from_dispatch(dispatch("MESSAGE_REACTION_REMOVE", raw)).unwrap();
        let Event::ReactionRemove(e) = ev else {
            panic!("expected ReactionRemove");
        };
        assert_eq!(e.target.type_, 2);
        assert_eq!(e.user_id, "U-UNDO");
    }

    #[test]
    fn direct_message_create_decodes() {
        let raw = json!({
            "id": "DM-MSG-1",
            "guild_id": "G-DM",
            "channel_id": "CH-DM",
            "author": { "id": "U-SENDER", "username": "sender", "bot": false },
            "content": "private message",
            "timestamp": "2026-06-01T00:00:00+00:00"
        });
        let ev = Event::from_dispatch(dispatch("DIRECT_MESSAGE_CREATE", raw)).unwrap();
        let Event::DirectMessageCreate(m) = ev else {
            panic!("expected DirectMessageCreate");
        };
        assert_eq!(m.id, "DM-MSG-1");
        assert_eq!(m.guild_id, "G-DM");
        assert_eq!(m.content, "private message");
    }

    #[test]
    fn message_delete_decodes() {
        let raw = json!({
            "guild_id": "G-001",
            "channel_id": "CH-1",
            "message_id": "MID-DEL",
            "op_user_id": "U-MOD"
        });
        let ev = Event::from_dispatch(dispatch("PUBLIC_MESSAGE_DELETE", raw)).unwrap();
        let Event::MessageDelete(e) = ev else {
            panic!("expected MessageDelete");
        };
        assert_eq!(e.guild_id, "G-001");
        assert_eq!(e.message_id, "MID-DEL");
        assert_eq!(e.op_user_id.as_deref(), Some("U-MOD"));
    }

    #[test]
    fn audit_pass_decodes() {
        let raw = json!({
            "guild_id": "G-001",
            "channel_id": "CH-1",
            "message_id": "MID-APPROVED",
            "audit_result": 0
        });
        let ev = Event::from_dispatch(dispatch("MESSAGE_AUDIT_PASS", raw)).unwrap();
        let Event::AuditPass(e) = ev else {
            panic!("expected AuditPass");
        };
        assert_eq!(e.guild_id, "G-001");
        assert_eq!(e.audit_result, 0);
        assert!(e.reason.is_none());
    }

    #[test]
    fn audit_reject_decodes_with_reason() {
        let raw = json!({
            "guild_id": "G-001",
            "channel_id": "CH-1",
            "message_id": "MID-REJECTED",
            "audit_result": 1,
            "reason": "contains prohibited content"
        });
        let ev = Event::from_dispatch(dispatch("MESSAGE_AUDIT_REJECT", raw)).unwrap();
        let Event::AuditReject(e) = ev else {
            panic!("expected AuditReject");
        };
        assert_eq!(e.audit_result, 1);
        assert_eq!(e.reason.as_deref(), Some("contains prohibited content"));
    }

    #[test]
    fn forum_thread_create_decodes() {
        let raw = json!({
            "guild_id": "G-001",
            "channel_id": "CH-FORUM",
            "author_id": "U-AUTHOR",
            "thread_id": "THREAD-1",
            "title": "Welcome post",
            "content": "Hello forum!",
            "timestamp": "2026-06-13T10:00:00+00:00"
        });
        let ev = Event::from_dispatch(dispatch("FORUM_THREAD_CREATE", raw)).unwrap();
        let Event::ForumThreadCreate(e) = ev else {
            panic!("expected ForumThreadCreate");
        };
        assert_eq!(e.thread_id.as_deref(), Some("THREAD-1"));
        assert_eq!(e.title.as_deref(), Some("Welcome post"));
        assert_eq!(e.author_id, "U-AUTHOR");
    }

    #[test]
    fn forum_thread_delete_decodes() {
        let raw = json!({
            "guild_id": "G-001",
            "channel_id": "CH-FORUM",
            "author_id": "U-AUTHOR",
            "thread_id": "THREAD-OBSOLETE",
            "timestamp": "2026-06-13T12:00:00+00:00"
        });
        let ev = Event::from_dispatch(dispatch("FORUM_THREAD_DELETE", raw)).unwrap();
        let Event::ForumThreadDelete(e) = ev else {
            panic!("expected ForumThreadDelete");
        };
        assert_eq!(e.thread_id.as_deref(), Some("THREAD-OBSOLETE"));
    }

    #[test]
    fn open_forum_thread_create_decodes() {
        let raw = json!({
            "guild_id": "G-001",
            "channel_id": "CH-FORUM",
            "author_id": "U-AUTHOR",
            "thread_id": "THREAD-OPEN",
            "title": "Open forum post",
            "timestamp": "2026-06-13T16:50:00+08:00"
        });
        let ev = Event::from_dispatch(dispatch("OPEN_FORUM_THREAD_CREATE", raw)).unwrap();
        let Event::OpenForumThreadCreate(e) = ev else {
            panic!("expected OpenForumThreadCreate from OPEN_FORUM_THREAD_CREATE");
        };
        assert_eq!(e.thread_id.as_deref(), Some("THREAD-OPEN"));
        assert_eq!(e.title.as_deref(), Some("Open forum post"));
    }

    #[test]
    fn open_forum_thread_without_id() {
        // OPEN_FORUM_* 公域事件只含 author_id / channel_id / guild_id
        let raw = json!({
            "guild_id": "G-001",
            "channel_id": "CH-FORUM",
            "author_id": "U-AUTHOR"
        });
        let ev = Event::from_dispatch(dispatch("OPEN_FORUM_THREAD_CREATE", raw)).unwrap();
        let Event::OpenForumThreadCreate(e) = ev else {
            panic!("expected OpenForumThreadCreate");
        };
        assert_eq!(e.author_id, "U-AUTHOR");
        assert!(e.thread_id.is_none(), "OPEN_FORUM_* 的 thread_id 应为 None");
        assert!(e.timestamp.is_none(), "OPEN_FORUM_* 的 timestamp 应为 None");
    }

    #[test]
    fn audio_start_decodes() {
        let raw = json!({
            "guild_id": "G-001",
            "channel_id": "CH-VOICE",
            "op_user_id": "U-DJ",
            "audio_type": 1,
            "status": 1,
            "timestamp": "2026-06-13T14:00:00+00:00"
        });
        let ev = Event::from_dispatch(dispatch("AUDIO_START", raw)).unwrap();
        let Event::AudioStart(e) = ev else {
            panic!("expected AudioStart");
        };
        assert_eq!(e.guild_id, "G-001");
        assert_eq!(e.op_user_id.as_deref(), Some("U-DJ"));
    }

    #[test]
    fn audio_finish_decodes() {
        let raw = json!({
            "guild_id": "G-001",
            "channel_id": "CH-VOICE",
            "op_user_id": "U-DJ",
            "audio_type": 1,
            "status": 0,
            "timestamp": "2026-06-13T15:00:00+00:00"
        });
        let ev = Event::from_dispatch(dispatch("AUDIO_FINISH", raw)).unwrap();
        let Event::AudioFinish(e) = ev else {
            panic!("expected AudioFinish");
        };
        assert_eq!(e.status, 0);
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
