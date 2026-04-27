//! 富消息载荷——按种类拆分（markdown / keyboard / ark / embed / media）。
//!
//! 字段全部 `pub`，构造好后塞进
//! [`crate::types::message::OutgoingMessage`] /
//! [`crate::types::message::OutgoingChannelMessage`]。

mod ark;
mod embed;
mod keyboard;
mod markdown;
mod media;

pub use ark::{ArkKv, ArkObj, ArkObjKv, ArkPayload};
pub use embed::{EmbedField, EmbedPayload, EmbedThumbnail};
pub use keyboard::KeyboardPayload;
pub use markdown::{MarkdownParam, MarkdownPayload};
pub use media::{FileType, Media};
