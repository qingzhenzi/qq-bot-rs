//! WebSocket 网关——握手、心跳、断线重连 / Resume，dispatch 帧通过
//! [`mpsc`](tokio::sync::mpsc) 流给调用方；[`Gateway::shutdown`] 干净退出。

mod connection;
#[cfg(test)]
mod tests;

pub use crate::types::gateway::DispatchEvent;
pub use connection::Gateway;
