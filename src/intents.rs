//! 网关订阅的事件类别位掩码——机器人在 Identify 帧里上报这个 u32 声明
//! 想接收哪些事件，网关只推送已订阅类别的 Dispatch。

use bitflags::bitflags;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

bitflags! {
    /// 事件订阅位掩码。
    ///
    /// `serde` 序列化为裸 u32——bitflags 默认的字符串序列化与网关线协议不兼容，
    /// 单独手写 `Serialize` / `Deserialize`。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Intents: u32 {
        /// 频道事件：guild create/update/delete、channel create/update/delete。
        const GUILDS = 1 << 0;

        /// 频道成员事件：member add/update/remove。
        const GUILD_MEMBERS = 1 << 1;

        /// 频道消息事件（**仅私域机器人可用**）：覆盖频道内全部消息，不只是 @机器人。
        const GUILD_MESSAGES = 1 << 9;

        /// 消息互动（表态）事件：reaction add/remove。
        const GUILD_MESSAGE_REACTIONS = 1 << 10;

        /// 私信事件：direct message create/delete。
        const DIRECT_MESSAGE = 1 << 12;

        /// 开放论坛事件：thread/post/reply create/delete。
        const OPEN_FORUM_EVENT = 1 << 18;

        /// 音视频/直播子频道成员进出事件。
        const AUDIO_OR_LIVE_CHANNEL_MEMBER = 1 << 19;

        /// 公域群 / C2C 消息事件：group at message、c2c message、好友/群增删等。
        const PUBLIC_MESSAGES = 1 << 25;

        /// 互动事件：interaction create（按钮、菜单等）。
        const INTERACTION = 1 << 26;

        /// 消息审核事件：audit pass / reject。
        const MESSAGE_AUDIT = 1 << 27;

        /// 论坛事件（**仅私域机器人可用**）。
        const FORUMS = 1 << 28;

        /// 音频事件：start/finish/on_mic/off_mic。
        const AUDIO_ACTION = 1 << 29;

        /// 公域消息事件：on_at_message_create、on_public_message_delete。
        const PUBLIC_GUILD_MESSAGES = 1 << 30;
    }
}

impl Intents {
    /// 默认推荐订阅集——所有公域事件，不含 `GUILD_MESSAGES` / `FORUMS`（需私域权限）。
    pub fn default_public() -> Self {
        Self::GUILDS
            | Self::GUILD_MEMBERS
            | Self::GUILD_MESSAGE_REACTIONS
            | Self::DIRECT_MESSAGE
            | Self::OPEN_FORUM_EVENT
            | Self::AUDIO_OR_LIVE_CHANNEL_MEMBER
            | Self::PUBLIC_MESSAGES
            | Self::INTERACTION
            | Self::MESSAGE_AUDIT
            | Self::AUDIO_ACTION
            | Self::PUBLIC_GUILD_MESSAGES
    }

    /// 仅群聊 + 私聊——不订阅任何频道 / 公会事件。网关不会推送 `OPEN_FORUM_*`、
    /// `GUILD_*`、`CHANNEL_*` 等事件，日志干净。
    pub fn group_and_c2c_only() -> Self {
        Self::PUBLIC_MESSAGES | Self::INTERACTION
    }
}

impl Serialize for Intents {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.bits().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Intents {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // 截断未知 bit——服务端若回发我们没定义的 bit，不影响已识别集合。
        let bits = u32::deserialize(deserializer)?;
        Ok(Self::from_bits_truncate(bits))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_as_u32() {
        let mask = Intents::DIRECT_MESSAGE | Intents::PUBLIC_GUILD_MESSAGES;
        let json = serde_json::to_value(mask).unwrap();
        assert_eq!(json.as_u64().unwrap() as u32, mask.bits());
    }

    #[test]
    fn deserializes_from_u32() {
        let bits = Intents::GUILDS.bits();
        let v: Intents = serde_json::from_value(serde_json::json!(bits)).unwrap();
        assert_eq!(v, Intents::GUILDS);
    }
}
