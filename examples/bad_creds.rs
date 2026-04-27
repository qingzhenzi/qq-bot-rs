//! 验证 4004 / 永久鉴权失败的退出路径。
//!
//! 故意传入错误的 `QQ_BOT_APP_SECRET`，期望 `Gateway::connect` 在握手阶段立刻 `Err`
//! （HTTP 401 由 token 交换抛出），**不**进入重连循环。

use qq_bot_rs::prelude::*;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,qq_bot_rs=debug")),
        )
        .init();

    dotenvy::dotenv().ok();

    let app_id = std::env::var("QQ_BOT_APP_ID").expect("QQ_BOT_APP_ID required for this example");
    let creds = Credentials::new(app_id, "intentionally-wrong-secret-for-test");

    let http = Bot::new(creds);
    let intents = Intents::default_public();

    match Gateway::connect(http, intents).await {
        Ok(_) => panic!("expected connect to fail with bad creds"),
        Err(e) => {
            println!("got expected error: {e}");
            if let GatewayError::Http(http_err) = e {
                println!("inner: {http_err}");
            }
        }
    }
}
