# qq-bot-rs

QQ 机器人 SDK 的 Rust 实现——async/await + trait + 所有权风格。

接口与协议以 [QQ 官方 v2 文档](https://bot.q.qq.com/wiki/develop/api-v2/) 为准。

## 快速上手

`.env` 里填好 `QQ_BOT_APP_ID` / `QQ_BOT_APP_SECRET`：

```rust
use async_trait::async_trait;
use qq_bot_rs::prelude::*;

struct MyBot;

#[async_trait]
impl EventHandler for MyBot {
    async fn on_group_at_message_create(&self, bot: &Bot, msg: GroupMessage) {
        let reply = OutgoingMessage::text("hello").reply_to(&msg.id);
        let _ = bot.post_group_message(&msg.group_openid, &reply).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), BotError> {
    dotenvy::dotenv().ok();
    Client::from_env()?
        .intents(Intents::default_public())
        .handler(MyBot)
        .run()
        .await
}
```

完整 demo（含 markdown / 键盘 / 撤回 / 表态 / 私信 / 按钮回调）见
[`examples/run_bot.rs`](examples/run_bot.rs)。

## 已覆盖能力

- **消息**：群 / c2c（v2）+ 频道子频道 / 频道私信（v1）的发送 + 撤回；
  text / markdown / ark / embed / media / keyboard 全形态
- **互动**：按钮回调（`INTERACTION_CREATE`）+ ACK，含 5s 时效约束
- **表态**：频道消息 PUT / DELETE / GET 表情表态（含 cookie 分页）
- **私信**：`POST /users/@me/dms` 创建会话 + 后续 `/dms/{guild_id}/...` 发送
- **分享链接**：`generate_url_link` 含 `callback_data` 转化追踪
- **事件**：14 个强类型变体（READY / RESUMED / 三类消息 / FRIEND_*\* /
  C2C_MSG_*\* / GROUP_*_ROBOT / GROUP_MSG_*\* / INTERACTION_CREATE）+
  forward-compat 的 `Event::Unknown` 兜底
- **网关**：握手 / 心跳 / Resume / 自动重连，4004 永久错不重试退避

## 设计要点

- 事件 handler 通过入参拿到 `&Bot`，不必自己持有副本
- access_token 缓存 + 提前刷新对调用方透明（`Bot::access_token()` 仅给
  `gateway` 拼 Identify 帧用）
- 凭证有手写 `Debug` 脱敏，**不**实现 `Display`
- 错误按领域分（`AuthError` / `HttpError` / `GatewayError` /
  `ClientBuildError`），顶层 `BotError` 用 `From` 聚合
- `Intents` 是 `bitflags!` 上的 u32，自定义 serde 把它序列化成裸数字
- `HttpError::Decode` 嵌入响应原文，schema 不齐时不必盲飞

## 开发

```sh
cargo check                              # 编译验证
cargo clippy --all-targets -- -D warnings
cargo test
cargo run --example run_bot              # 端到端联调
```

## License

[MIT](LICENSE)
