use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::warn;

use super::decode::Event;
use crate::http::Bot;
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

/// 事件处理器——实现感兴趣的方法即可，其它默认 no-op。
///
/// 默认 [`EventHandler::on_event`] 按变体派发到下方 typed 方法；重写它可做
/// 统一前置（日志、metrics、限流）。
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// 顶层入口——默认按变体派发。重写做统一前置时记得自己继续派发。
    async fn on_event(&self, bot: &Bot, event: Event) {
        match event {
            Event::Ready(r) => self.on_ready(bot, r).await,
            Event::Resumed => self.on_resumed(bot).await,
            Event::AtMessageCreate(m) => self.on_at_message_create(bot, *m).await,
            Event::GroupAtMessageCreate(m) => self.on_group_at_message_create(bot, *m).await,
            Event::GroupMessageCreate(m) => self.on_group_message_create(bot, *m).await,
            Event::C2cMessageCreate(m) => self.on_c2c_message_create(bot, *m).await,
            Event::FriendAdd(e) => self.on_friend_add(bot, e).await,
            Event::FriendDel(e) => self.on_friend_del(bot, e).await,
            Event::C2cMsgReject(e) => self.on_c2c_msg_reject(bot, e).await,
            Event::C2cMsgReceive(e) => self.on_c2c_msg_receive(bot, e).await,
            Event::GroupAddRobot(e) => self.on_group_add_robot(bot, e).await,
            Event::GroupDelRobot(e) => self.on_group_del_robot(bot, e).await,
            Event::GroupMsgReject(e) => self.on_group_msg_reject(bot, e).await,
            Event::GroupMsgReceive(e) => self.on_group_msg_receive(bot, e).await,
            Event::InteractionCreate(i) => self.on_interaction_create(bot, *i).await,
            Event::GuildCreate(g) => self.on_guild_create(bot, g).await,
            Event::GuildUpdate(g) => self.on_guild_update(bot, g).await,
            Event::GuildDelete(g) => self.on_guild_delete(bot, g).await,
            Event::ChannelCreate(c) => self.on_channel_create(bot, *c).await,
            Event::ChannelUpdate(c) => self.on_channel_update(bot, *c).await,
            Event::ChannelDelete(c) => self.on_channel_delete(bot, *c).await,
            Event::GuildMemberAdd(e) => self.on_guild_member_add(bot, *e).await,
            Event::GuildMemberUpdate(e) => self.on_guild_member_update(bot, *e).await,
            Event::GuildMemberRemove(e) => self.on_guild_member_remove(bot, *e).await,
            Event::ReactionAdd(e) => self.on_reaction_add(bot, *e).await,
            Event::ReactionRemove(e) => self.on_reaction_remove(bot, *e).await,
            Event::DirectMessageCreate(m) => self.on_direct_message_create(bot, *m).await,
            Event::MessageDelete(e) => self.on_message_delete(bot, *e).await,
            Event::AuditPass(e) => self.on_audit_pass(bot, *e).await,
            Event::AuditReject(e) => self.on_audit_reject(bot, *e).await,
            Event::ForumThreadCreate(e) => self.on_forum_thread_create(bot, *e).await,
            Event::ForumThreadUpdate(e) => self.on_forum_thread_update(bot, *e).await,
            Event::ForumThreadDelete(e) => self.on_forum_thread_delete(bot, *e).await,
            Event::OpenForumThreadCreate(e) => self.on_open_forum_thread_create(bot, *e).await,
            Event::OpenForumThreadUpdate(e) => self.on_open_forum_thread_update(bot, *e).await,
            Event::OpenForumThreadDelete(e) => self.on_open_forum_thread_delete(bot, *e).await,
            Event::ForumPostCreate(e) => self.on_forum_post_create(bot, *e).await,
            Event::ForumPostDelete(e) => self.on_forum_post_delete(bot, *e).await,
            Event::AudioStart(e) => self.on_audio_start(bot, *e).await,
            Event::AudioFinish(e) => self.on_audio_finish(bot, *e).await,
            Event::Unknown { name, data, seq } => self.on_unknown(bot, name, data, seq).await,
        }
    }

    /// 握手完成。
    async fn on_ready(&self, _bot: &Bot, _ready: ReadyData) {}

    /// 重连成功，会话延续。
    async fn on_resumed(&self, _bot: &Bot) {}

    /// 频道 @ 机器人消息。
    async fn on_at_message_create(&self, _bot: &Bot, _msg: ChannelMessage) {}

    /// 群 @ 机器人消息。
    async fn on_group_at_message_create(&self, _bot: &Bot, _msg: GroupMessage) {}

    /// 群内所有消息（非 @ 也能收到）。
    async fn on_group_message_create(&self, _bot: &Bot, _msg: GroupMessage) {}

    /// 私聊（C2C）消息。
    async fn on_c2c_message_create(&self, _bot: &Bot, _msg: C2cMessage) {}

    /// 用户添加机器人为好友。
    async fn on_friend_add(&self, _bot: &Bot, _event: UserManageEvent) {}

    /// 用户从好友列表移除机器人。
    async fn on_friend_del(&self, _bot: &Bot, _event: UserManageEvent) {}

    /// 用户关闭主动消息推送。
    async fn on_c2c_msg_reject(&self, _bot: &Bot, _event: UserManageEvent) {}

    /// 用户重新开启主动消息推送。
    async fn on_c2c_msg_receive(&self, _bot: &Bot, _event: UserManageEvent) {}

    /// 机器人被加入群。
    async fn on_group_add_robot(&self, _bot: &Bot, _event: GroupManageEvent) {}

    /// 机器人被移出群。
    async fn on_group_del_robot(&self, _bot: &Bot, _event: GroupManageEvent) {}

    /// 群管理员关闭机器人主动消息。
    async fn on_group_msg_reject(&self, _bot: &Bot, _event: GroupManageEvent) {}

    /// 群管理员开启机器人主动消息。
    async fn on_group_msg_receive(&self, _bot: &Bot, _event: GroupManageEvent) {}

    /// 按钮 / 快捷菜单交互——**必须** ACK：
    /// [`crate::http::Bot::put_interaction_callback`]。
    async fn on_interaction_create(&self, _bot: &Bot, _interaction: Interaction) {}

    /// 库未识别的事件。
    async fn on_unknown(&self, _bot: &Bot, _name: String, _data: Value, _seq: u64) {}

    /// 频道创建。
    async fn on_guild_create(&self, _bot: &Bot, _guild: Guild) {}

    /// 频道信息更新。
    async fn on_guild_update(&self, _bot: &Bot, _guild: Guild) {}

    /// 频道解散或机器人被移除。
    async fn on_guild_delete(&self, _bot: &Bot, _guild: Guild) {}

    /// 子频道创建。
    async fn on_channel_create(&self, _bot: &Bot, _channel: Channel) {}

    /// 子频道信息更新。
    async fn on_channel_update(&self, _bot: &Bot, _channel: Channel) {}

    /// 子频道删除。
    async fn on_channel_delete(&self, _bot: &Bot, _channel: Channel) {}

    /// 成员加入频道。
    async fn on_guild_member_add(&self, _bot: &Bot, _event: GuildMemberAddEvent) {}

    /// 成员信息更新（昵称 / 身份组变更等）。
    async fn on_guild_member_update(&self, _bot: &Bot, _event: GuildMemberUpdateEvent) {}

    /// 成员被移出频道。
    async fn on_guild_member_remove(&self, _bot: &Bot, _event: GuildMemberRemoveEvent) {}

    /// 频道消息被添加表情表态。
    async fn on_reaction_add(&self, _bot: &Bot, _event: ReactionEvent) {}

    /// 频道消息的表态被移除。
    async fn on_reaction_remove(&self, _bot: &Bot, _event: ReactionEvent) {}

    /// 频道私信消息。
    async fn on_direct_message_create(&self, _bot: &Bot, _msg: ChannelMessage) {}

    /// 频道消息被删除（私信或公开频道）。
    async fn on_message_delete(&self, _bot: &Bot, _event: MessageDeleteEvent) {}

    /// 消息审核通过。
    async fn on_audit_pass(&self, _bot: &Bot, _event: AuditEvent) {}

    /// 消息审核拒绝。
    async fn on_audit_reject(&self, _bot: &Bot, _event: AuditEvent) {}

    /// 私域论坛帖子创建（`FORUM_THREAD_CREATE`）。公域走 [`Self::on_open_forum_thread_create`]。
    async fn on_forum_thread_create(&self, _bot: &Bot, _event: ForumThreadEvent) {}

    /// 论坛帖子更新。
    async fn on_forum_thread_update(&self, _bot: &Bot, _event: ForumThreadEvent) {}

    /// 论坛帖子删除。
    async fn on_forum_thread_delete(&self, _bot: &Bot, _event: ForumThreadEvent) {}

    /// 公域论坛帖子创建（`OPEN_FORUM_THREAD_CREATE`）。
    async fn on_open_forum_thread_create(&self, _bot: &Bot, _event: ForumThreadEvent) {}

    /// 公域论坛帖子更新（`OPEN_FORUM_THREAD_UPDATE`）。
    async fn on_open_forum_thread_update(&self, _bot: &Bot, _event: ForumThreadEvent) {}

    /// 公域论坛帖子删除（`OPEN_FORUM_THREAD_DELETE`）。
    async fn on_open_forum_thread_delete(&self, _bot: &Bot, _event: ForumThreadEvent) {}

    /// 论坛评论创建（`OPEN_FORUM_POST_CREATE`）。
    async fn on_forum_post_create(&self, _bot: &Bot, _event: ForumThreadEvent) {}

    /// 论坛评论删除（`OPEN_FORUM_POST_DELETE`）。
    async fn on_forum_post_delete(&self, _bot: &Bot, _event: ForumThreadEvent) {}

    /// 音频开始播放。
    async fn on_audio_start(&self, _bot: &Bot, _event: AudioEvent) {}

    /// 音频结束播放。
    async fn on_audio_finish(&self, _bot: &Bot, _event: AudioEvent) {}
}

/// 把 dispatch 通道驱动到 handler，阻塞直到通道关闭。
/// 单帧解码失败只 `warn` 不退出——一帧坏帧不该把整个 bot 停掉。
pub async fn dispatch_to<H: EventHandler + ?Sized>(
    handler: &H,
    bot: &Bot,
    events: &mut mpsc::Receiver<DispatchEvent>,
) {
    while let Some(raw) = events.recv().await {
        match Event::from_dispatch(raw) {
            Ok(event) => handler.on_event(bot, event).await,
            Err(e) => {
                warn!(event_type = %e.event_type, error = %e.source, "decode event failed");
            }
        }
    }
}
