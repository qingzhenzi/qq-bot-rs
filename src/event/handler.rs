use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::warn;

use super::decode::Event;
use crate::http::Bot;
use crate::types::gateway::{DispatchEvent, ReadyData};
use crate::types::interaction::Interaction;
use crate::types::manage::{GroupManageEvent, UserManageEvent};
use crate::types::message::{C2cMessage, ChannelMessage, GroupMessage};

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
