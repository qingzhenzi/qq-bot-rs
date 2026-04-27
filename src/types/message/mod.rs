//! 消息相关的协议 DTO——按入 / 出方向拆：
//!
//! - [`inbound`]：服务端推送给我们的消息事件 DTO；
//! - [`outgoing`]：客户端组装发送的消息 + 发送响应。

mod inbound;
mod outgoing;

pub use inbound::{
    Attachment, C2cMessage, C2cMessageAuthor, ChannelMessage, GroupMessage, GroupMessageAuthor,
    MessageReference,
};
pub use outgoing::{MessageType, OutgoingChannelMessage, OutgoingMessage, SentMessage};
