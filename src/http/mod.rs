//! REST API 客户端。
//!
//! 业务方法跨文件展开同一 `impl Bot { ... }`：transport / token 在 `client.rs`，
//! 各资源（messages / interactions / reactions / users / dms / meta / guilds / channels）一文件一组。

mod announces;
mod audio;
mod channels;
mod client;
mod dms;
mod forums;
mod guilds;
mod interactions;
mod messages;
mod meta;
mod mutes;
mod permissions;
mod pins;
mod reactions;
mod roles;
mod schedules;
#[cfg(test)]
mod tests;
mod users;

pub use announces::Announce;
pub use channels::VoiceMember;
pub use client::{Bot, BotBuilder};
pub use dms::DmSession;
pub use forums::{ForumPost, ForumThread};
pub use guilds::{ApiIdentify, ApiPermissionDemand, ApiPermissionEntry, ApiPermissionList};
pub use interactions::InteractionCallbackCode;
pub use permissions::ChannelPermissions;
pub use pins::PinMessage;
pub use reactions::{EmojiType, ReactionUsersPage};
pub use roles::{Role, RoleMemberPage, RolePage};
pub use schedules::Schedule;
