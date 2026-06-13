use std::time::Duration;

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use serde_json::json;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use tracing::{debug, info, warn};
use url::Url;

use crate::error::GatewayError;
use crate::http::Bot;
use crate::intents::Intents;
use crate::types::gateway::{DispatchEvent, HelloData, IdentifyData, OpCode, Payload, ResumeData};

const EVENT_CHANNEL_CAPACITY: usize = 32;

// 单分片足够 v1。
const SHARD: [u32; 2] = [0, 1];

const TOKEN_PREFIX: &str = "QQBot";

// 重连退避：起始 1s，每次失败翻倍，封顶 60s；成功握手后回到起始。
const BACKOFF_INIT: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(60);

const INVALID_SESSION_PAUSE: Duration = Duration::from_millis(500);

// 下限挡 0 / 极小值——会让 ticker spin。
// 上限挡极大值——心跳永不发，session 静默死。
const HEARTBEAT_MIN_MS: u64 = 50;
const HEARTBEAT_MAX_MS: u64 = 600_000;

/// 网关连接句柄——内部 supervisor 任务持续维持连接，握手 / 收发 / 重连
/// （优先 Resume，失败回退 Identify）全程对调用方透明。
pub struct Gateway {
    supervisor: JoinHandle<Result<(), GatewayError>>,
    shutdown: watch::Sender<bool>,
}

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Default)]
struct SessionContext {
    /// READY 帧里的 session_id；`Some` 则下次连接尝试 Resume。
    session_id: Option<String>,

    /// 收到的最后一个 dispatch 序列号——心跳和 Resume 都要用。
    last_seq: u64,
}

impl SessionContext {
    fn reset(&mut self) {
        self.session_id = None;
        self.last_seq = 0;
    }
}

enum SessionExit {
    Shutdown,
    /// op 7：服务端要求重连，session_id 仍然有效。
    ServerReconnect,
    /// op 9：session 失效，必须重置后 Identify。
    InvalidSession,
    /// WebSocket 流自然结束——尝试 Resume。
    StreamEnded,
}

struct ConnectedSession {
    sink: SplitSink<WsStream, Message>,
    stream: SplitStream<WsStream>,
    heartbeat_interval: u64,
}

impl Gateway {
    /// 建立首次连接并启动 supervisor。
    ///
    /// 首次握手在调用线同步完成——凭证错 / 网络不通会立刻 `Err`，不会无声坠入
    /// 重连循环。握手后 supervisor 接管，断线自动 Resume / 重连。
    pub async fn connect(
        http: Bot,
        intents: Intents,
    ) -> Result<(Self, mpsc::Receiver<DispatchEvent>), GatewayError> {
        let (events_tx, events_rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let ctx = SessionContext::default();
        let initial = connect_session(&http, intents, &ctx).await?;
        info!("gateway initial session connected");

        let supervisor = tokio::spawn(supervise(
            initial,
            ctx,
            http,
            intents,
            events_tx,
            shutdown_rx,
        ));

        Ok((
            Self {
                supervisor,
                shutdown: shutdown_tx,
            },
            events_rx,
        ))
    }

    /// 通知 supervisor 干净退出并 join。
    pub async fn shutdown(self) -> Result<(), GatewayError> {
        let _ = self.shutdown.send(true);
        match self.supervisor.await {
            Ok(res) => res,
            Err(je) if je.is_cancelled() => Ok(()),
            Err(je) => std::panic::resume_unwind(je.into_panic()),
        }
    }
}

async fn supervise(
    initial: ConnectedSession,
    mut ctx: SessionContext,
    http: Bot,
    intents: Intents,
    events_tx: mpsc::Sender<DispatchEvent>,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), GatewayError> {
    let mut current = Some(initial);
    let mut backoff = BACKOFF_INIT;

    loop {
        let session = match current.take() {
            Some(s) => s,
            None => {
                if *shutdown.borrow() {
                    return Ok(());
                }
                match connect_session(&http, intents, &ctx).await {
                    Ok(s) => {
                        backoff = BACKOFF_INIT;
                        s
                    }
                    Err(e) if is_permanent(&e) => {
                        warn!(error = %e, "permanent gateway error, giving up");
                        return Err(e);
                    }
                    Err(e) => {
                        warn!(error = %e, ?backoff, "connect failed, will retry");
                        if sleep_or_shutdown(backoff, &mut shutdown).await {
                            return Ok(());
                        }
                        backoff = (backoff * 2).min(BACKOFF_MAX);
                        continue;
                    }
                }
            }
        };

        match run_session_loop(session, &mut ctx, &events_tx, &mut shutdown).await {
            Ok(SessionExit::Shutdown) => return Ok(()),
            Ok(SessionExit::ServerReconnect) => {
                info!("server requested reconnect, will resume");
            }
            Ok(SessionExit::InvalidSession) => {
                info!("session invalid, will identify fresh");
                ctx.reset();
                if sleep_or_shutdown(INVALID_SESSION_PAUSE, &mut shutdown).await {
                    return Ok(());
                }
            }
            Ok(SessionExit::StreamEnded) => {
                info!("stream ended, will attempt resume");
            }
            Err(e) if is_permanent(&e) => {
                warn!(error = %e, "permanent gateway error, giving up");
                return Err(e);
            }
            Err(e) => {
                warn!(error = %e, ?backoff, "session error, reconnecting");
                if sleep_or_shutdown(backoff, &mut shutdown).await {
                    return Ok(());
                }
                backoff = (backoff * 2).min(BACKOFF_MAX);
            }
        }
    }
}

async fn connect_session(
    http: &Bot,
    intents: Intents,
    ctx: &SessionContext,
) -> Result<ConnectedSession, GatewayError> {
    let gateway = http.get_gateway().await?;
    validate_gateway_url(&gateway.url)?;
    debug!(url = %gateway.url, "connecting ws");
    let (ws, _) = connect_async(gateway.url.as_str()).await?;
    let (mut sink, mut stream) = ws.split();

    let hello = recv_payload(&mut stream).await?;
    if hello.op != OpCode::Hello {
        return Err(GatewayError::UnexpectedOp(hello.op));
    }
    let HelloData { heartbeat_interval } = serde_json::from_value(hello.d)?;
    if !(HEARTBEAT_MIN_MS..=HEARTBEAT_MAX_MS).contains(&heartbeat_interval) {
        return Err(GatewayError::InvalidHeartbeatInterval(heartbeat_interval));
    }

    let access_token = http.access_token().await?;
    let token_str = format!("{TOKEN_PREFIX} {access_token}");

    let frame = if let Some(session_id) = ctx.session_id.as_ref() {
        debug!(seq = ctx.last_seq, "sending resume");
        Payload {
            op: OpCode::Resume,
            d: serde_json::to_value(ResumeData {
                token: token_str,
                session_id: session_id.clone(),
                seq: ctx.last_seq,
            })?,
            s: None,
            t: None,
        }
    } else {
        debug!("sending identify");
        Payload {
            op: OpCode::Identify,
            d: serde_json::to_value(IdentifyData {
                token: token_str,
                intents,
                shard: SHARD,
                properties: None,
            })?,
            s: None,
            t: None,
        }
    };
    send_payload(&mut sink, &frame).await?;

    Ok(ConnectedSession {
        sink,
        stream,
        heartbeat_interval,
    })
}

async fn run_session_loop(
    mut session: ConnectedSession,
    ctx: &mut SessionContext,
    events_tx: &mpsc::Sender<DispatchEvent>,
    shutdown: &mut watch::Receiver<bool>,
) -> Result<SessionExit, GatewayError> {
    // QQ 协议要 Identify 鉴权完成后才认 heartbeat（READY 前发的会被静默丢弃）。
    // 主动消耗 interval 的"首 tick 即时返回"，否则首次心跳会被 select! 的随机
    // 调度吃掉，下一次心跳延后到 +2*period。
    let mut ticker = tokio::time::interval(Duration::from_millis(session.heartbeat_interval));
    // Skip 而非 Burst：supervisor 被抢占跨过多个间隔后，按下个槽位发一次，
    // 不补发——服务端不需要追账。
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    ticker.tick().await;

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                let _ = session.sink.close().await;
                return Ok(SessionExit::Shutdown);
            }
            _ = ticker.tick() => {
                send_heartbeat(&mut session.sink, ctx.last_seq).await?;
            }
            msg = session.stream.next() => match msg {
                Some(Ok(Message::Text(text))) => {
                    let payload: Payload = serde_json::from_str(text.as_str())?;
                    debug!(op = ?payload.op, t = ?payload.t, raw = ?payload.d, "ws payload received");
                    if let Some(s) = payload.s {
                        ctx.last_seq = s;
                    }
                    if let Some(exit) = handle_payload(payload, ctx, events_tx).await? {
                        return Ok(exit);
                    }
                }
                Some(Ok(Message::Close(frame))) => {
                    return Err(close_to_error(frame));
                }
                Some(Ok(_)) => {}
                Some(Err(e)) => return Err(e.into()),
                None => return Ok(SessionExit::StreamEnded),
            }
        }
    }
}

async fn handle_payload(
    payload: Payload,
    ctx: &mut SessionContext,
    events_tx: &mpsc::Sender<DispatchEvent>,
) -> Result<Option<SessionExit>, GatewayError> {
    match payload.op {
        OpCode::Dispatch => {
            let Some(event_type) = payload.t else {
                warn!("dispatch frame without event type, dropping");
                return Ok(None);
            };
            // READY 里的 session_id 决定后续能不能 Resume。
            if event_type == "READY"
                && let Some(sid) = payload.d.get("session_id").and_then(Value::as_str)
            {
                ctx.session_id = Some(sid.to_owned());
                info!(session_id = sid, "session id captured");
            }
            // Dispatch 帧按协议必带 `s`；缺了不当致命错——透出 0 给消费方记录，
            // 但 warn 出来，定位是服务端协议怪行还是 SDK schema 漂移。
            let seq = payload.s.unwrap_or_else(|| {
                warn!(%event_type, "dispatch frame missing seq, falling back to 0");
                0
            });
            let event = DispatchEvent {
                event_type,
                data: payload.d,
                seq,
            };
            if events_tx.send(event).await.is_err() {
                debug!("events receiver dropped, treating as shutdown");
                return Ok(Some(SessionExit::Shutdown));
            }
            Ok(None)
        }
        OpCode::HeartbeatAck => {
            debug!("heartbeat ack");
            Ok(None)
        }
        OpCode::Reconnect => Ok(Some(SessionExit::ServerReconnect)),
        OpCode::InvalidSession => Ok(Some(SessionExit::InvalidSession)),
        OpCode::Heartbeat => {
            // 协议允许服务端主动发心跳，实务中没见过；忽略。
            debug!("server-initiated heartbeat ignored");
            Ok(None)
        }
        unexpected @ (OpCode::Hello | OpCode::Identify | OpCode::Resume) => {
            Err(GatewayError::UnexpectedOp(unexpected))
        }
    }
}

async fn recv_payload(stream: &mut SplitStream<WsStream>) -> Result<Payload, GatewayError> {
    loop {
        match stream.next().await {
            Some(Ok(Message::Text(text))) => {
                return Ok(serde_json::from_str(text.as_str())?);
            }
            Some(Ok(Message::Close(frame))) => return Err(close_to_error(frame)),
            Some(Ok(_)) => continue,
            Some(Err(e)) => return Err(e.into()),
            None => return Err(GatewayError::HandshakeClosed),
        }
    }
}

async fn send_payload(
    sink: &mut SplitSink<WsStream, Message>,
    payload: &Payload,
) -> Result<(), GatewayError> {
    let body = serde_json::to_string(payload)?;
    sink.send(Message::Text(body.into())).await?;
    Ok(())
}

async fn send_heartbeat(
    sink: &mut SplitSink<WsStream, Message>,
    last_seq: u64,
) -> Result<(), GatewayError> {
    let frame = Payload {
        op: OpCode::Heartbeat,
        d: json!(last_seq),
        s: None,
        t: None,
    };
    debug!(seq = last_seq, "heartbeat send");
    send_payload(sink, &frame).await
}

/// `true` 表示 shutdown 触发。
async fn sleep_or_shutdown(d: Duration, shutdown: &mut watch::Receiver<bool>) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(d) => false,
        _ = shutdown.changed() => true,
    }
}

fn close_to_error(frame: Option<CloseFrame>) -> GatewayError {
    if let Some(f) = frame {
        let code = u16::from(f.code);
        if is_permanent_close(code) {
            return GatewayError::AuthRejected { code };
        }
    }
    GatewayError::HandshakeClosed
}

// 4004 鉴权失败、4014 intents 不允许——这两类不该重试。
fn is_permanent_close(code: u16) -> bool {
    matches!(code, 4004 | 4014)
}

fn is_permanent(e: &GatewayError) -> bool {
    matches!(
        e,
        GatewayError::AuthRejected { .. }
            | GatewayError::InsecureGatewayUrl(_)
            | GatewayError::InvalidHeartbeatInterval(_)
    )
}

/// 网关 URL 必须 `wss://`；放行 `ws://` 仅当 host 是 loopback——给本地测试 mock 留口。
/// 阻止 QQ API 误回（或被钓鱼端点污染）的明文 / 异端点 WebSocket 把 token 明文外发。
fn validate_gateway_url(raw: &str) -> Result<(), GatewayError> {
    let url = Url::parse(raw).map_err(|_| GatewayError::InsecureGatewayUrl(raw.to_owned()))?;
    match url.scheme() {
        "wss" => Ok(()),
        "ws" if is_loopback_host(url.host_str()) => Ok(()),
        _ => Err(GatewayError::InsecureGatewayUrl(raw.to_owned())),
    }
}

fn is_loopback_host(host: Option<&str>) -> bool {
    matches!(host, Some("127.0.0.1" | "localhost" | "::1" | "[::1]"))
}
