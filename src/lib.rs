//! `qq-bot-rs`：QQ 机器人 SDK 的 Rust 实现——async/await + trait + 所有权风格。

pub mod auth;
pub mod client;
pub mod error;
pub mod event;
pub mod gateway;
pub mod http;
pub mod intents;
pub mod prelude;
pub mod types;

pub use auth::Credentials;
pub use client::{Client, ClientBuilder};
pub use error::{AuthError, BotError, ClientBuildError, GatewayError, HttpError};
pub use event::{Event, EventHandler, dispatch_to};
pub use gateway::{DispatchEvent, Gateway};
pub use http::{
    Announce, ApiIdentify, ApiPermissionDemand, ApiPermissionEntry, ApiPermissionList, Bot,
    BotBuilder, ChannelPermissions, DmSession, EmojiType, ForumPost, ForumThread,
    InteractionCallbackCode, PinMessage, ReactionUsersPage, Role, RoleMemberPage, RolePage,
    Schedule, VoiceMember,
};
pub use intents::Intents;
pub use types::member::{GuildMemberEntry, GuildMemberPage};
