//! 网关重连 / Resume / op7 / op9 / 4004 等"只在真断网时复现"分支的回归测试。
//!
//! 起本地 `tokio-tungstenite` server 喂脚本帧 + `wiremock` mock `/gateway/bot`
//! 与 token 端点。每个测试用例通常起一个新的 mock 集合——隔离避免相互污染。

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::Credentials;
use crate::error::GatewayError;
use crate::gateway::Gateway;
use crate::http::Bot;
use crate::intents::Intents;

/// 解析帧到 JSON。文本帧之外都视为测试断言失败——脚本里没准备过别的。
fn parse_text(msg: Message) -> Value {
    match msg {
        Message::Text(t) => serde_json::from_str(t.as_str()).expect("client sent invalid json"),
        other => panic!("expected text frame, got {other:?}"),
    }
}

/// 起一个监听 `127.0.0.1:0` 的 TCP listener，返回 `(listener, ws://addr)`。
async fn bind_local_ws() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    (listener, format!("ws://{addr}"))
}

/// `accept` 一个 ws 连接并完成升级握手。
async fn accept_ws(listener: &TcpListener) -> tokio_tungstenite::WebSocketStream<TcpStream> {
    let (stream, _) = listener.accept().await.unwrap();
    accept_async(stream).await.unwrap()
}

/// 配 mock：`POST /token` 返回 access_token；`GET /gateway/bot` 返回指向给定 ws_url 的入口。
async fn setup_http_mock(ws_url: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "test-token",
            "expires_in": "7200",
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/gateway/bot"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "url": ws_url })))
        .mount(&server)
        .await;
    server
}

/// 用 mock 服务地址组装 Bot + 凭证。
fn make_http_client(mock_base: &str) -> Bot {
    let creds = Credentials::new("test-app-id", "test-app-secret");
    Bot::builder()
        .api_base(mock_base.to_owned())
        .token_endpoint(format!("{mock_base}/token"))
        .build(creds)
}

/// Hello 帧（含心跳 interval）。`interval_ms` 一般给得很大，避免测试期间真触发心跳。
fn hello_frame(interval_ms: u64) -> Message {
    Message::Text(
        json!({"op": 10, "d": {"heartbeat_interval": interval_ms}, "s": null, "t": null})
            .to_string()
            .into(),
    )
}

/// READY dispatch 帧——含 session_id，使 SDK 后续走 Resume。
fn ready_frame(session_id: &str, seq: u64) -> Message {
    Message::Text(
        json!({
            "op": 0,
            "s": seq,
            "t": "READY",
            "d": {
                "version": 1,
                "session_id": session_id,
                "user": {"id": "100", "username": "test-bot", "bot": true, "status": 1},
                "shard": [0, 1],
            }
        })
        .to_string()
        .into(),
    )
}

/// op 7（Reconnect）。
fn reconnect_frame() -> Message {
    Message::Text(
        json!({"op": 7, "d": null, "s": null, "t": null})
            .to_string()
            .into(),
    )
}

/// op 9（InvalidSession）。
fn invalid_session_frame() -> Message {
    Message::Text(
        json!({"op": 9, "d": false, "s": null, "t": null})
            .to_string()
            .into(),
    )
}

/// HeartbeatAck (op 11)。
fn heartbeat_ack_frame() -> Message {
    Message::Text(
        json!({"op": 11, "d": null, "s": null, "t": null})
            .to_string()
            .into(),
    )
}

/// **happy path**：Hello → Identify → READY → 关闭。
///
/// 验证 Identify 含正确字段、READY 透传给调用方、shutdown 干净退出。
#[tokio::test]
async fn happy_path_hello_identify_ready() {
    let (listener, ws_url) = bind_local_ws().await;
    let mock = setup_http_mock(&ws_url).await;
    let http = make_http_client(&mock.uri());

    // 服务端：Hello → 期望收到 Identify → 发 READY → 等客户端关闭。
    let server_task = tokio::spawn(async move {
        let mut ws = accept_ws(&listener).await;
        ws.send(hello_frame(60_000)).await.unwrap();
        let identify = parse_text(ws.next().await.unwrap().unwrap());
        assert_eq!(identify["op"], 2, "expected Identify op=2");
        assert_eq!(identify["d"]["token"], "QQBot test-token");
        assert_eq!(identify["d"]["shard"], json!([0, 1]));
        ws.send(ready_frame("sess-happy", 1)).await.unwrap();
        // 等客户端主动关——shutdown 时会发 close。
        while let Some(msg) = ws.next().await {
            if matches!(msg, Ok(Message::Close(_))) {
                break;
            }
        }
    });

    let (gateway, mut events) = Gateway::connect(http, Intents::default_public())
        .await
        .expect("initial connect");
    let event = events.recv().await.expect("ready event");
    assert_eq!(event.event_type, "READY");
    assert_eq!(event.data["session_id"], "sess-happy");
    assert_eq!(event.seq, 1);

    gateway.shutdown().await.expect("clean shutdown");
    server_task.await.unwrap();
}

/// **op 9**：服务端在 Identify 后发 op 9 → SDK 必须重置 session_id，
/// 下次连接走 Identify 而非 Resume。
#[tokio::test]
async fn op9_invalid_session_resets_and_reidentifies() {
    let (listener, ws_url) = bind_local_ws().await;
    let mock = setup_http_mock(&ws_url).await;
    let http = make_http_client(&mock.uri());

    // 用 channel 把第二次连接收到的握手帧透出来给主测试断言。
    let (probe_tx, mut probe_rx) = mpsc::channel::<Value>(2);

    let server_task = tokio::spawn(async move {
        // 第一次连接：Hello → READY（让 SDK 抓到 session_id）→ op9 → close。
        let mut ws = accept_ws(&listener).await;
        ws.send(hello_frame(60_000)).await.unwrap();
        let _identify = parse_text(ws.next().await.unwrap().unwrap());
        ws.send(ready_frame("sess-old", 1)).await.unwrap();
        ws.send(invalid_session_frame()).await.unwrap();
        // 客户端一收到 op9 就会结束本 session 准备重连——主动关掉这条。
        let _ = ws.close(None).await;

        // 第二次连接：Hello → 期望 Identify（不是 Resume）。
        let mut ws2 = accept_ws(&listener).await;
        ws2.send(hello_frame(60_000)).await.unwrap();
        let handshake = parse_text(ws2.next().await.unwrap().unwrap());
        probe_tx.send(handshake).await.unwrap();
        // 让 SDK 安静退出：服务端发 READY 后等 close。
        ws2.send(ready_frame("sess-new", 1)).await.unwrap();
        while let Some(msg) = ws2.next().await {
            if matches!(msg, Ok(Message::Close(_))) {
                break;
            }
        }
    });

    let (gateway, mut events) = Gateway::connect(http, Intents::default_public())
        .await
        .expect("initial connect");
    // 接掉第一次的 READY，避免事件 channel 阻塞 dispatch。
    let _first_ready = events.recv().await.expect("first ready event");

    let handshake = tokio::time::timeout(Duration::from_secs(5), probe_rx.recv())
        .await
        .expect("second handshake within timeout")
        .expect("probe channel alive");
    assert_eq!(
        handshake["op"], 2,
        "expected Identify (op 2) after op 9, got: {handshake}"
    );
    assert!(
        handshake["d"].get("session_id").is_none(),
        "Identify must not carry session_id (would imply Resume)"
    );

    // 第二次的 READY 也要捞掉——dispatch 任务在事件收完前不会 cleanly 退。
    let _second_ready = events.recv().await.expect("second ready event");

    gateway.shutdown().await.expect("clean shutdown");
    server_task.await.unwrap();
}

/// **op 7**：READY 后服务端发 op 7 → 下次连接必须发 Resume，
/// 携带原 session_id 与最后已知 seq。
#[tokio::test]
async fn op7_reconnect_triggers_resume() {
    let (listener, ws_url) = bind_local_ws().await;
    let mock = setup_http_mock(&ws_url).await;
    let http = make_http_client(&mock.uri());

    let (probe_tx, mut probe_rx) = mpsc::channel::<Value>(2);

    let server_task = tokio::spawn(async move {
        let mut ws = accept_ws(&listener).await;
        ws.send(hello_frame(60_000)).await.unwrap();
        let _identify = parse_text(ws.next().await.unwrap().unwrap());
        ws.send(ready_frame("sess-keep", 7)).await.unwrap();
        ws.send(reconnect_frame()).await.unwrap();
        let _ = ws.close(None).await;

        let mut ws2 = accept_ws(&listener).await;
        ws2.send(hello_frame(60_000)).await.unwrap();
        let handshake = parse_text(ws2.next().await.unwrap().unwrap());
        probe_tx.send(handshake).await.unwrap();
        // 让客户端能正常退出。RESUMED 帧不发也行——直接等 close。
        while let Some(msg) = ws2.next().await {
            if matches!(msg, Ok(Message::Close(_))) {
                break;
            }
        }
    });

    let (gateway, mut events) = Gateway::connect(http, Intents::default_public())
        .await
        .expect("initial connect");
    let _ready = events.recv().await.expect("ready event");

    let handshake = tokio::time::timeout(Duration::from_secs(5), probe_rx.recv())
        .await
        .expect("resume handshake within timeout")
        .expect("probe channel alive");
    assert_eq!(
        handshake["op"], 6,
        "expected Resume (op 6) after op 7, got: {handshake}"
    );
    assert_eq!(handshake["d"]["session_id"], "sess-keep");
    assert_eq!(handshake["d"]["seq"], 7);

    gateway.shutdown().await.expect("clean shutdown");
    server_task.await.unwrap();
}

/// **4004**：服务端 Identify 后用 4004 关闭 → supervisor 必须以 `AuthRejected` 退出，
/// **不**进入重连退避循环。
#[tokio::test]
async fn close_4004_returns_auth_rejected() {
    let (listener, ws_url) = bind_local_ws().await;
    let mock = setup_http_mock(&ws_url).await;
    let http = make_http_client(&mock.uri());

    let server_task = tokio::spawn(async move {
        let mut ws = accept_ws(&listener).await;
        ws.send(hello_frame(60_000)).await.unwrap();
        let _identify = parse_text(ws.next().await.unwrap().unwrap());
        ws.send(Message::Close(Some(CloseFrame {
            code: CloseCode::Library(4004),
            reason: "auth failed".into(),
        })))
        .await
        .unwrap();
        let _ = ws.close(None).await;
    });

    let (gateway, mut events) = Gateway::connect(http, Intents::default_public())
        .await
        .expect("initial connect");

    // 等 supervisor 自己以 AuthRejected 终结：它结束时 events_tx 被 drop，
    // recv 返回 None。直接调 shutdown 会和 close 帧的处理 race（select! 里
    // shutdown 信号可能先于 close 帧被命中），导致 supervisor 看不到 4004。
    let drained = tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("events channel closes within timeout");
    assert!(
        drained.is_none(),
        "expected channel close, got event {drained:?}"
    );

    let res = gateway.shutdown().await;
    match res {
        Err(GatewayError::AuthRejected { code }) => assert_eq!(code, 4004),
        other => panic!("expected AuthRejected(4004), got {other:?}"),
    }
    server_task.await.unwrap();
}

/// **heartbeat**：服务端给短 interval → SDK 应在 +interval 处发 Heartbeat → 服务端回 ack。
#[tokio::test]
async fn heartbeat_send_and_ack() {
    let (listener, ws_url) = bind_local_ws().await;
    let mock = setup_http_mock(&ws_url).await;
    let http = make_http_client(&mock.uri());

    let (probe_tx, mut probe_rx) = mpsc::channel::<Value>(4);

    let server_task = tokio::spawn(async move {
        let mut ws = accept_ws(&listener).await;
        // 100ms 心跳——测试 1s 内能稳定看到至少一次心跳。
        ws.send(hello_frame(100)).await.unwrap();
        let _identify = parse_text(ws.next().await.unwrap().unwrap());
        ws.send(ready_frame("sess-hb", 1)).await.unwrap();

        // 收 N 个客户端帧——前几个应是 Heartbeat (op 1)。
        while let Some(Ok(msg)) = ws.next().await {
            match msg {
                Message::Text(_) => {
                    let v = parse_text(msg);
                    let op = v["op"].as_u64().unwrap_or(0);
                    let _ = probe_tx.send(v).await;
                    if op == 1 {
                        // 回 ack。
                        ws.send(heartbeat_ack_frame()).await.unwrap();
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    let (gateway, mut events) = Gateway::connect(http, Intents::default_public())
        .await
        .expect("initial connect");
    let _ready = events.recv().await.expect("ready event");

    // 等首个心跳出现——若 ticker 实现错误（首 tick 被吃掉等），这里会超时。
    let first = tokio::time::timeout(Duration::from_secs(2), probe_rx.recv())
        .await
        .expect("client sent a frame within 2s")
        .expect("probe alive");
    assert_eq!(
        first["op"], 1,
        "first client frame after Identify should be Heartbeat, got: {first}"
    );

    gateway.shutdown().await.expect("clean shutdown");
    server_task.await.unwrap();
}
