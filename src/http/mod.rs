//! REST API 客户端。
//!
//! 业务方法跨文件展开同一 `impl Bot { ... }`：transport / token 在 `client.rs`，
//! 各资源（messages / interactions / reactions / users / dms / meta）一文件一组。

mod client;
mod dms;
mod interactions;
mod messages;
mod meta;
mod reactions;
#[cfg(test)]
mod tests;
mod users;

pub use client::{Bot, BotBuilder};
pub use dms::DmSession;
pub use interactions::InteractionCallbackCode;
pub use reactions::{EmojiType, ReactionUsersPage};
