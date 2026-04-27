//! 顶层 [`Client`]——把 [`Bot`] + [`Gateway`] + [`EventHandler`] 串起来。

use std::fmt;
use std::sync::Arc;

use crate::auth::Credentials;
use crate::error::{AuthError, BotError, ClientBuildError};
use crate::event::{EventHandler, dispatch_to};
use crate::gateway::Gateway;
use crate::http::Bot;
use crate::intents::Intents;

pub struct Client {
    bot: Bot,
    intents: Intents,
    handler: Arc<dyn EventHandler>,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("bot", &self.bot)
            .field("intents", &self.intents)
            .finish_non_exhaustive()
    }
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// 从 `QQ_BOT_APP_ID` / `QQ_BOT_APP_SECRET` 加载凭证，返回已注入 [`Bot`]
    /// 的 builder。不读 `.env`——按需在程序入口自行 `dotenvy::dotenv().ok()`。
    pub fn from_env() -> Result<ClientBuilder, AuthError> {
        Ok(ClientBuilder::default().bot(Bot::from_env()?))
    }

    pub fn bot(&self) -> &Bot {
        &self.bot
    }

    /// 启动 bot：建网关连接、跑 dispatch、收到流末尾或永久错时返回。
    ///
    /// 不自动捕获 Ctrl-C / SIGTERM——调用方按需在外层 `tokio::select!` 加退出
    /// 条件，再用 [`Gateway::shutdown`] 干净停。
    pub async fn run(self) -> Result<(), BotError> {
        let bot_for_dispatch = self.bot.clone();
        let (gateway, mut events) = Gateway::connect(self.bot, self.intents).await?;
        dispatch_to(self.handler.as_ref(), &bot_for_dispatch, &mut events).await;
        gateway.shutdown().await?;
        Ok(())
    }
}

/// [`Client`] builder。
///
/// 必填：`credentials(c)` 或 `bot(b)`（二选一）+ `handler(...)`。
/// 可选：`intents`，默认 [`Intents::default_public`]。
#[derive(Default)]
pub struct ClientBuilder {
    bot: Option<Bot>,
    credentials: Option<Credentials>,
    intents: Option<Intents>,
    handler: Option<Arc<dyn EventHandler>>,
}

impl ClientBuilder {
    /// 直接传 [`Bot`]——适合需要自定义 timeout / sandbox 的场景。
    /// 与 [`Self::credentials`] 二选一；都设时此项优先。
    pub fn bot(mut self, bot: Bot) -> Self {
        self.bot = Some(bot);
        self
    }

    pub fn credentials(mut self, creds: Credentials) -> Self {
        self.credentials = Some(creds);
        self
    }

    pub fn intents(mut self, intents: Intents) -> Self {
        self.intents = Some(intents);
        self
    }

    pub fn handler<H: EventHandler + 'static>(mut self, handler: H) -> Self {
        self.handler = Some(Arc::new(handler));
        self
    }

    pub fn build(self) -> Result<Client, ClientBuildError> {
        let bot = match (self.bot, self.credentials) {
            (Some(b), _) => b,
            (None, Some(c)) => Bot::new(c),
            (None, None) => return Err(ClientBuildError::MissingBot),
        };
        let handler = self.handler.ok_or(ClientBuildError::MissingHandler)?;
        let intents = self.intents.unwrap_or_else(Intents::default_public);
        Ok(Client {
            bot,
            intents,
            handler,
        })
    }

    /// `build()? + run().await` 的语法糖。
    pub async fn run(self) -> Result<(), BotError> {
        self.build()?.run().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct DummyHandler;

    #[async_trait]
    impl EventHandler for DummyHandler {}

    #[test]
    fn build_requires_bot_or_credentials() {
        let err = Client::builder().handler(DummyHandler).build().unwrap_err();
        assert!(matches!(err, ClientBuildError::MissingBot));
    }

    #[test]
    fn build_requires_handler() {
        let err = Client::builder()
            .credentials(Credentials::new("123", "abcdEFGHwxyz"))
            .build()
            .unwrap_err();
        assert!(matches!(err, ClientBuildError::MissingHandler));
    }

    #[test]
    fn build_uses_default_intents() {
        let client = Client::builder()
            .credentials(Credentials::new("123", "abcdEFGHwxyz"))
            .handler(DummyHandler)
            .build()
            .unwrap();
        assert_eq!(client.intents, Intents::default_public());
    }
}
