//! 元信息端点——网关入口、机器人自身。

use crate::error::HttpError;
use crate::http::Bot;
use crate::types::robot::{Robot, WsGateway};

impl Bot {
    /// `GET /gateway/bot`。
    pub async fn get_gateway(&self) -> Result<WsGateway, HttpError> {
        self.get_json("/gateway/bot").await
    }

    /// `GET /users/@me`。
    pub async fn get_self(&self) -> Result<Robot, HttpError> {
        self.get_json("/users/@me").await
    }
}
