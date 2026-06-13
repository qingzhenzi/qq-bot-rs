//! 业务代码常用类型一站式导入：`use qq_bot_rs::prelude::*;`。
//!
//! 低频 / 高级类型走显式路径取（`qq_bot_rs::http::BotBuilder`、
//! `qq_bot_rs::types::gateway::OpCode` 等）。

pub use crate::auth::Credentials;
pub use crate::client::Client;
pub use crate::error::{AuthError, BotError, ClientBuildError, GatewayError, HttpError};
pub use crate::event::{Event, EventHandler};
pub use crate::gateway::{DispatchEvent, Gateway};
pub use crate::http::{
    ApiPermissionDemand, Bot, ChannelPermissions, InteractionCallbackCode, Role, RolePage,
};
pub use crate::intents::Intents;
pub use crate::types::message::{
    C2cMessage, ChannelMessage, GroupMessage, OutgoingChannelMessage, OutgoingMessage, SentMessage,
};
pub use crate::types::payloads::{
    ArkPayload, EmbedPayload, FileType, KeyboardPayload, MarkdownPayload, Media,
};
pub use crate::types::robot::{Robot, WsGateway};
