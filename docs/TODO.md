# qq-bot-rs TODO

按优先级分组。每条带 **Why**（为什么要做）和 **Where**（落地位置 / 参考代码）。
完成项打勾留作时间线。

接口与协议以 [QQ 官方 v2 文档](https://bot.q.qq.com/wiki/develop/api-v2/) 为准。

---

## P0 — 稳定性与可回归

库已经能跑通 echo，但所有验证都靠真实网络 + 人工。继续加功能前先把测试基础设施补上，否则后续每个改动都赌运气。

- [x] **mock WebSocket 单测** (2026-04-27)
  - **Why**：网关重连 / Resume / op 7 / op 9 / 4004 等分支目前**只在真断网时能复现**，靠 example 永远验不到。一旦回归，下游用户先发现。
  - **落地**：`src/gateway/tests.rs`——本地 `tokio-tungstenite` 起 ws server + `wiremock` mock `/gateway/bot` 与 token 端点。`HttpClientBuilder` 加 `#[cfg(test)] pub(crate) fn api_base / token_endpoint` 让测试可指。覆盖：
    - Hello → Identify → READY 快路径
    - 服务端发 op 9 → 库 reset session_id → 下次 Identify
    - 服务端发 op 7（READY 后）→ 库尝试 Resume，携带正确 session_id + seq
    - 服务端 4004 close → `Err(AuthRejected { code: 4004 })` 不重试
    - 短心跳 interval（100ms）→ 客户端首帧是 Heartbeat，服务端回 ack 不报错
  - 仍待补：`is_permanent_close(4014)` 的 4014 case；`StreamEnded` 自然断开后 Resume 路径；HeartbeatAck 长时间不回的踢。

- [x] **HTTP mock 单测（wiremock）** (2026-04-27)
  - **Why**：token 刷新（成功 / 失败 / 过期边界）、`HttpError::Status` 的 body 透出——这些只能靠 mock 跑。
  - **落地**：`src/http/tests.rs` 用 wiremock 起本地 server。`HttpClientBuilder` 加 `#[cfg(test)] api_base` / `token_endpoint` 让测试可指（同 mock-ws 测试共用）。覆盖：token happy / TokenRejected / 缓存复用（expect(1) 校验只打一次）、ApiError 抽 `code` `message` `trace_id`、Status 兜底。
  - 仍待补：429 限流（属 P3）、`Retry-After` / 5xx 自动退避一次重试。

- [x] **HTTP 错误响应可读化** (2026-04-27)
  - **Why**：跑 `bad_creds` 时报 `missing field access_token` —— 实际原因是凭证错，但报错文案让用户摸不着头脑。
  - **落地**：`HttpError::TokenRejected { body }` 新变体。`ensure_token` 在 200 但解码 `AccessTokenResponse` 失败时不再透传 `Decode`，统一识别为 `TokenRejected`，body 含 QQ 的 `code` / `message`。

- [x] **`HttpError::Status` 解析嵌入式错误** (2026-04-27)
  - **Why**：QQ 部分接口非 2xx 响应体形如 `{"code": 11253, "message": "..."}`，目前我们把整段 body 塞 `Status.body` 里。可以再 best-effort 解出 `code` / `message` 让上层好处理。
  - **落地**：`HttpError::ApiError { status, code, message, trace_id, body }` 新变体。`decode_error_body` 先尝试 JSON 对象 + `code` 字段抽取，命中则 `ApiError`，不命中回退 `Status`。Display 里带 `trace_id`，方便日志 grep 对账服务端日志。

---

## P1 — 功能补全（用户最先会问的）

- [x] **富媒体消息**：图片 / 视频 / 语音 / 文件 (2026-04-27)
  - **落地**：`HttpClient::post_group_file` / `post_c2c_file` 上传 → 返 `Media { file_uuid, file_info, ttl }` → `OutgoingMessage::media(media)` 发送。`FileType` enum（Image/Video/Voice/File）锁线协议数值。`srv_send_msg` 参数透出（true 直发占主动配额，false 仅上传两步走）。

- [x] **Markdown / Ark / Embed / Keyboard 消息体** (2026-04-27)
  - **落地**：
    - `types/payloads.rs` —— `MarkdownPayload` / `ArkPayload` / `EmbedPayload` / `KeyboardPayload` / `Media` + 嵌套 DTO（`MarkdownParam`、`ArkKv`、`ArkObj`、`ArkObjKv`、`EmbedField`、`EmbedThumbnail`）
    - `types/keyboard.rs` —— `Keyboard` / `KeyboardRow` / `Button` / `RenderData` / `Action` / `Permission`。`Action.type` / `Permission.type` 走 `serde(rename = "type")`；`style` / `action_type` / `permission_type` 用 `u8` 透传（避免服务端加新值时反序列化炸）。
    - `OutgoingMessage::{markdown, ark, embed, media}` 构造方法 + `.with_keyboard(...)` / `.with_content(...)` builder
    - `OutgoingChannelMessage::{markdown, ark, embed}` + `.with_keyboard(...)` / `.quote(message_id, ignore_missing)`
  - 单测覆盖 serialize 形态（msg_type 选对、`type` rename、空字段不输出）。

- [x] **撤回消息** (2026-04-27)
  - **落地**：transport 层 `delete_empty(&path)`（pub(super)）。`HttpClient::delete_channel_message(cid, mid, hide_tip)` / `delete_group_message` / `delete_c2c_message`。失败走 `decode_error_body` → `ApiError`。

- [x] **主动消息** (2026-04-27)
  - **落地**：接口面早已覆盖（不调 `.reply_to(...)` 即主动）。`OutgoingMessage` 文档加被动 vs 主动区分 + 配额超额会以 `HttpError::ApiError` 透出的说明，开发期建议优先被动路径。

---

## P2 — 更多事件类型

每加一种事件就是 `event.rs` 加变体 + 必要的 DTO。`Event::Unknown` 兜底所以不紧急，但典型 bot 场景会用到。

按使用频率排：

- [ ] **频道事件**：`GUILD_CREATE` / `GUILD_UPDATE` / `GUILD_DELETE`
- [ ] **子频道事件**：`CHANNEL_CREATE` / `CHANNEL_UPDATE` / `CHANNEL_DELETE`
- [ ] **成员事件**：`GUILD_MEMBER_ADD` / `GUILD_MEMBER_UPDATE` / `GUILD_MEMBER_REMOVE`
  - **Why**：欢迎语 bot 必备。
- [ ] **互动事件**：`INTERACTION_CREATE`
  - **Why**：按钮、菜单、命令的回调入口，新一代 bot 主流交互方式。
- [ ] **直接消息事件**：`DIRECT_MESSAGE_CREATE` / `DIRECT_MESSAGE_DELETE`
- [ ] **消息审核事件**：`MESSAGE_AUDIT_PASS` / `MESSAGE_AUDIT_REJECT`
- [ ] **表情表态**：`MESSAGE_REACTION_ADD` / `MESSAGE_REACTION_REMOVE`
- [ ] **论坛事件**：`FORUM_THREAD_*` / `FORUM_POST_*` / `FORUM_REPLY_*` 等
- [ ] **音频事件**：`AUDIO_*`
- [ ] **群 / C2C 关系事件**：`FRIEND_ADD` / `FRIEND_DEL` / `GROUP_ADD_ROBOT` / `GROUP_DEL_ROBOT` / `*_MSG_REJECT` / `*_MSG_RECEIVE`
- [ ] **公开消息删除**：`PUBLIC_MESSAGE_DELETE`

---

## P2 — 更多 HTTP 资源

按 botpy `api.py` 切的领域，每个一文件 `impl HttpClient { ... }`：

- [ ] **`http/guilds.rs`**：get_guild、get_guild_members、ban / unban、roles
- [ ] **`http/channels.rs`**：列子频道、创建 / 更新 / 删除子频道、子频道权限
- [ ] **`http/users.rs`**：mute、kick（成员管理）
- [ ] **`http/messages.rs`** 扩充：撤回、置顶、查询历史、撤回 reaction
- [ ] **`http/forums.rs`**：主题 / 帖子 / 评论 CRUD
- [ ] **`http/audio.rs`**：start / pause / resume / stop
- [ ] **`http/announces.rs`**：频道公告
- [ ] **`http/schedules.rs`**：日程
- [ ] **`http/pins.rs`**：置顶消息
- [ ] **`http/reactions.rs`**：表态 add / remove / list

---

## P3 — 工程化

- [ ] **README.md**（顶层）
  - **Why**：`CLAUDE.md` 是给 Claude 的开发约定，不是给最终用户的入门文档。需要一份"安装 / 快速开始 / 最小 echo bot 示例 / 链接到 docs"的 README。
  - 用 `examples/run_bot.rs` 当 quick-start 的素材。

- [ ] **多分片支持**
  - **Why**：当前 `gateway/connection.rs` 写死 `SHARD: [0, 1]`。大型机器人一个分片连接不够，需要并发多个 ws session。
  - **Where**：refactor `Gateway::connect` 接受 `shard: Option<[u32; 2]>`，由 `Client` 顶层做 N 分片的 supervisor。botpy 参考：`connection.py::ConnectionSession`。

- [ ] **限流处理（429）**
  - **Where**：`http/client.rs::decode_response`，HTTP 429 + `Retry-After` 头自动退避重试 1 次。

- [ ] **`X-Tps-Trace-Id` 日志埋点**
  - **Why**：QQ 服务端排错时需要 trace_id；现在出错时丢了这个头。
  - **Where**：`http/client.rs::send_authed` 后读响应头 `X-Tps-trace-Id`，错误路径塞进 `HttpError::Status` / `ApiError`。

- [ ] **wire 类型迁出 `types/`**
  - **Why**：之前讨论过——`OpCode` / `Payload` / `IdentifyData` 等是 wire 内部，调用方用 `Event` / `DispatchEvent` 即可。
  - **Where**：搬到 `gateway/wire.rs`（私有 / `pub(crate)`），`types/gateway.rs` 删除。注意 `event.rs` 会继续用到 `ReadyData`——它属于"用户会读的数据"，留 `types/`。

- [ ] **CI 集成**
  - `cargo fmt --all -- --check` / `cargo clippy --all-targets -- -D warnings` / `cargo test --workspace`
  - 暂未上 GitHub，等真要发布再说。

- [ ] **`cargo-deny` / `cargo audit`**
  - 依赖树漏洞 / 许可证扫描；上 CI 之后再加。

- [ ] **doctests 真跑**
  - 现在 `prelude.rs` / `client.rs` / `types/message.rs` 的 doctest 都标了 `ignore` —— 写得有但没在 CI 里跑。需要把它们改成可运行（或加 `no_run` 区分意图）。

- [ ] **文档站构建检查**
  - `cargo doc --no-deps -D rustdoc::broken-intra-doc-links`，确保所有 `[`链接`]` 都解得到。

---

## 已决定**不做**

防止后续重新提出来又走一遍。

- ❌ **`BOT_SANDBOX` 等环境变量驱动配置** — 库不读自定义 env 名，调用方一行 `.sandbox(env::var("X").is_ok())` 自己接（详见 conversation 2026-04-27）。
- ❌ **`Credentials::Display`** — 故意不实现，杜绝隐式 `format!("{cred}")` 泄露。`Debug` 已脱敏（首尾 4 字符 + `***`）。
- ❌ **单实现 trait 包装 `Credentials` / `HttpClient`** — YAGNI；等真有第二个实现 / 测试需要 mock 时再抽。
- ❌ **`EventType` 单独 enum 中间层** — 等下批 typed event 一起规整时再决定；当前 `Event` sum 直接派发够用。
- ❌ **`qq-bot-rs-types` / `qq-bot-rs-api` workspace 拆 crate** — 单 crate 体量未到拆分阈值，过早分裂只徒增编译时间和路径。

---

## 时间线

- 2026-04-27：v0.1 骨架完工——凭证 / HTTP / 网关（Resume + 重连）/ typed event / Client builder / echo bot 端到端。详见各模块 module doc。
