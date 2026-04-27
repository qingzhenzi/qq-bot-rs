//! 强类型事件 + 处理器 trait。
//!
//! [`Gateway`](crate::gateway::Gateway) 输出 raw [`crate::gateway::DispatchEvent`]
//! （`data: Value`）；本模块两步升级到可直接 `match` 的 [`Event`]：
//! [`decode`] 解码，[`handler`] 派发。未识别事件名走 [`Event::Unknown`]，
//! 服务端加新事件不会挂。

mod decode;
mod handler;

pub use decode::Event;
pub use handler::{EventHandler, dispatch_to};
