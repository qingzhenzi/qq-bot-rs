use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use reqwest::Client as ReqwestClient;
use reqwest::Response;
use reqwest::StatusCode;
use reqwest::header::AUTHORIZATION;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::json;
use tokio::sync::Mutex;
use tracing::{debug, warn};

use crate::auth::Credentials;
use crate::error::{AuthError, HttpError};

// 服务端把 expires_in（秒）以字符串形式返回（如 "7200"）。
#[derive(Debug, Clone, Deserialize)]
struct AccessTokenResponse {
    access_token: String,
    expires_in: String,
}

const PROD_API_BASE: &str = "https://api.sgroup.qq.com";
const SANDBOX_API_BASE: &str = "https://sandbox.api.sgroup.qq.com";
const TOKEN_EXCHANGE_URL: &str = "https://bots.qq.com/app/getAppAccessToken";

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);
const DEFAULT_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

// access_token 有效期约 7200s，留 60s 余量避免请求恰好命中失效边界。
const REFRESH_LEAD: Duration = Duration::from_secs(60);

// ── 速率限制（Token Bucket）──

const DEFAULT_RATE_CAPACITY: u32 = 5;
const DEFAULT_RATE_REFILL: f64 = 5.0;

/// Token Bucket 速率限制器——在发包前主动等待，避免触发 QQ API 429。
#[derive(Clone)]
pub(crate) struct RateLimiter {
    capacity: f64,
    refill_per_sec: f64,
    state: Arc<Mutex<RateState>>,
}

struct RateState {
    available: f64,
    last_refill: Instant,
}

impl RateLimiter {
    pub fn new(capacity: u32, refill_per_sec: f64) -> Self {
        Self {
            capacity: capacity as f64,
            refill_per_sec,
            state: Arc::new(Mutex::new(RateState {
                available: capacity as f64,
                last_refill: Instant::now(),
            })),
        }
    }

    /// 消耗一个 token，如果没有则等待补充到足够。
    pub async fn acquire(&self) {
        loop {
            let wait = {
                let mut s = self.state.lock().await;
                let now = Instant::now();
                let elapsed = now.saturating_duration_since(s.last_refill).as_secs_f64();
                if elapsed > 0.0 {
                    s.available = (s.available + elapsed * self.refill_per_sec).min(self.capacity);
                    s.last_refill = now;
                }
                if s.available >= 1.0 {
                    s.available -= 1.0;
                    return; // 有 token，直接放行
                }
                // 还需要等多久才有 1 个 token
                Duration::from_secs_f64((1.0 - s.available) / self.refill_per_sec)
            };
            tokio::time::sleep(wait).await;
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(DEFAULT_RATE_CAPACITY, DEFAULT_RATE_REFILL)
    }
}

/// QQ 机器人句柄。
#[derive(Clone)]
pub struct Bot {
    inner: Arc<Inner>,
}

struct Inner {
    http: ReqwestClient,
    creds: Credentials,
    api_base: String,
    token_endpoint: String,

    /// 缓存的 access_token——只在快路径读 / 刷新路径写，临界区不跨 await。
    token: Mutex<TokenState>,

    /// 刷新闸——确保同一时刻最多一个 token 端点请求在飞，
    /// 但不阻塞已经持有有效 token 的并发调用。
    refresh: Mutex<()>,

    /// 全局递增的出站消息序号——给 v2 group / c2c 消息自动注入 `msg_seq`，
    /// 绕过 QQ 服务端的 `(msg_id, msg_seq)` 去重逻辑。同 `msg_id` 下多条 reply
    /// 必须各自序号唯一，否则第二条起回 40054005「消息被去重」。`Relaxed` 顺序
    /// 足够：我们只要值唯一，不要求跨线程"先后"语义。
    next_msg_seq: AtomicU32,

    /// 速率限制器——发送前主动等待 token，避免触发 QQ API 429。
    rate_limiter: RateLimiter,
}

struct TokenState {
    access_token: String,
    expires_at: Instant,
}

impl Default for TokenState {
    fn default() -> Self {
        Self {
            access_token: String::new(),
            // 强制首次取用前刷新。
            expires_at: Instant::now(),
        }
    }
}

impl Bot {
    pub fn new(credentials: Credentials) -> Self {
        Self::builder().build(credentials)
    }

    /// 从 `QQ_BOT_APP_ID` / `QQ_BOT_APP_SECRET` 加载凭证。不读 `.env`——
    /// 调用方按需自行 `dotenvy::dotenv().ok()`。
    pub fn from_env() -> Result<Self, AuthError> {
        Credentials::from_env().map(Self::new)
    }

    pub fn builder() -> BotBuilder {
        BotBuilder::default()
    }

    /// 当前可用 access_token——`gateway` 拼 Identify 帧用。
    pub async fn access_token(&self) -> Result<String, HttpError> {
        self.inner.ensure_token().await
    }

    pub fn app_id(&self) -> &str {
        self.inner.creds.app_id()
    }

    /// 取下一个全局递增的 `msg_seq`——`post_*_message` 在调用方未显式 `reply_seq`
    /// 时自动注入；外部一般不用直接调，留 pub 给需要严格控序的高级场景。
    pub fn next_msg_seq(&self) -> u32 {
        self.inner.next_msg_seq.fetch_add(1, Ordering::Relaxed)
    }

    pub(super) async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, HttpError> {
        let url = format!("{}{path}", self.inner.api_base);
        debug!(method = "GET", %url, "request");
        let req = self.inner.http.get(&url);
        let resp = self.send_idempotent_with_retry("GET", &url, req).await?;
        decode_response(resp).await
    }

    /// POST 不走 429 自动重试——非幂等，重复提交可能造成副作用。
    pub(super) async fn post_json<B, T>(&self, path: &str, body: &B) -> Result<T, HttpError>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let url = format!("{}{path}", self.inner.api_base);
        debug!(method = "POST", %url, "request");
        let req = self.inner.http.post(&url).json(body);
        let resp = self.send_authed(req).await?;
        decode_response(resp).await
    }

    pub(super) async fn delete_empty(&self, path: &str) -> Result<(), HttpError> {
        let url = format!("{}{path}", self.inner.api_base);
        debug!(method = "DELETE", %url, "request");
        let req = self.inner.http.delete(&url);
        let resp = self.send_idempotent_with_retry("DELETE", &url, req).await?;
        let trace_id = Self::extract_trace_id(&resp);
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(decode_error_body(status.as_u16(), body, trace_id));
        }
        Ok(())
    }

    pub(super) async fn put_empty(&self, path: &str) -> Result<(), HttpError> {
        let url = format!("{}{path}", self.inner.api_base);
        debug!(method = "PUT", %url, "request");
        let req = self.inner.http.put(&url);
        let resp = self.send_idempotent_with_retry("PUT", &url, req).await?;
        let trace_id = Self::extract_trace_id(&resp);
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(decode_error_body(status.as_u16(), body, trace_id));
        }
        Ok(())
    }

    pub(super) async fn put_json_empty<B>(&self, path: &str, body: &B) -> Result<(), HttpError>
    where
        B: Serialize + ?Sized,
    {
        let url = format!("{}{path}", self.inner.api_base);
        debug!(method = "PUT", %url, "request");
        let req = self.inner.http.put(&url).json(body);
        let resp = self.send_idempotent_with_retry("PUT", &url, req).await?;
        let trace_id = Self::extract_trace_id(&resp);
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(decode_error_body(status.as_u16(), body, trace_id));
        }
        Ok(())
    }

    pub(super) async fn patch_json_empty<B>(&self, path: &str, body: &B) -> Result<(), HttpError>
    where
        B: Serialize + ?Sized,
    {
        let url = format!("{}{path}", self.inner.api_base);
        debug!(method = "PATCH", %url, "request");
        let req = self.inner.http.patch(&url).json(body);
        let resp = self.send_idempotent_with_retry("PATCH", &url, req).await?;
        let trace_id = Self::extract_trace_id(&resp);
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(decode_error_body(status.as_u16(), body, trace_id));
        }
        Ok(())
    }

    pub(super) async fn patch_json<B, T>(&self, path: &str, body: &B) -> Result<T, HttpError>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let url = format!("{}{path}", self.inner.api_base);
        debug!(method = "PATCH", %url, "request");
        let req = self.inner.http.patch(&url).json(body);
        let resp = self.send_idempotent_with_retry("PATCH", &url, req).await?;
        decode_response(resp).await
    }

    async fn send_authed(&self, req: reqwest::RequestBuilder) -> Result<Response, HttpError> {
        self.inner.rate_limiter.acquire().await;
        let token = self.inner.ensure_token().await?;
        let resp = req
            .header(AUTHORIZATION, format!("QQBot {token}"))
            .header("X-Union-Appid", self.inner.creds.app_id())
            .send()
            .await?;
        Ok(resp)
    }

    /// 幂等请求（GET/PUT/DELETE/PATCH）走一次 429 自动退避重试。
    ///
    /// 首次 429 读 `Retry-After`（默认 5s）sleep 后重试一次；
    /// 第二次仍 429 则透出 `ApiError`。非 429 错不重试。
    async fn send_idempotent_with_retry(
        &self,
        method: &str,
        url: &str,
        req: reqwest::RequestBuilder,
    ) -> Result<Response, HttpError> {
        // try_clone 仅在 body 为 stream 时失败——本库只用 .json()，总是可 clone。
        let retry_req = req.try_clone().expect("request should be cloneable");
        let resp = self.send_authed(req).await?;

        if resp.status() != StatusCode::TOO_MANY_REQUESTS {
            return Ok(resp);
        }

        let retry_secs = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(5);

        // 丢弃 429 的响应体——重试时会重新发。
        warn!(
            method,
            %url,
            retry_secs,
            "HTTP 429 — retrying after {retry_secs}s"
        );
        tokio::time::sleep(Duration::from_secs(retry_secs.min(15))).await;

        self.send_authed(retry_req).await
    }

    /// 从响应头提取 `X-Tps-Trace-Id`。
    fn extract_trace_id(resp: &Response) -> Option<String> {
        resp.headers()
            .get("X-Tps-Trace-Id")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    }
}

impl fmt::Debug for Bot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bot")
            .field("api_base", &self.inner.api_base)
            .field("app_id", &self.inner.creds.app_id())
            .finish_non_exhaustive()
    }
}

impl Inner {
    // Double-checked locking + 单飞 refresh：
    //  - 快路径只读 `token`，临界区不跨 await，已有效 token 的并发调用不互相阻塞。
    //  - 慢路径走 `refresh` 闸，第二个进来的会看到刚被前一个写好的新 token，免发请求。
    async fn ensure_token(&self) -> Result<String, HttpError> {
        if let Some(t) = self.cached_token().await {
            return Ok(t);
        }

        let _refresh = self.refresh.lock().await;
        if let Some(t) = self.cached_token().await {
            return Ok(t);
        }

        let resp = self
            .http
            .post(&self.token_endpoint)
            .json(&json!({
                "appId": self.creds.app_id(),
                "clientSecret": self.creds.app_secret(),
            }))
            .send()
            .await?;
        let trace_id = Bot::extract_trace_id(&resp);
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(decode_error_body(status.as_u16(), body, trace_id));
        }
        // QQ 在凭证错时也回 200——把"200 + 没有 access_token"映射为鉴权拒绝，
        // 文案比 schema 错友好。
        let parsed: AccessTokenResponse = serde_json::from_str(&body)
            .map_err(|_| HttpError::TokenRejected { body: body.clone() })?;
        let secs: u64 = parsed
            .expires_in
            .parse()
            .map_err(|_| HttpError::InvalidExpiresIn(parsed.expires_in.clone()))?;
        let mut state = self.token.lock().await;
        state.access_token = parsed.access_token.clone();
        state.expires_at = Instant::now() + Duration::from_secs(secs);
        Ok(parsed.access_token)
    }

    async fn cached_token(&self) -> Option<String> {
        let state = self.token.lock().await;
        let still_fresh = !state.access_token.is_empty()
            && state.expires_at.saturating_duration_since(Instant::now()) > REFRESH_LEAD;
        still_fresh.then(|| state.access_token.clone())
    }
}

async fn decode_response<T: DeserializeOwned>(resp: Response) -> Result<T, HttpError> {
    let trace_id = Bot::extract_trace_id(&resp);
    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        return Err(decode_error_body(status.as_u16(), body, trace_id));
    }
    serde_json::from_str(&body).map_err(|source| HttpError::Decode { body, source })
}

// 非 2xx 响应若是 `{code, message, ...}` 形态升级为 ApiError，否则裸 body 走 Status。
// `header_trace_id` 来自响应头 `X-Tps-Trace-Id`，优先于 body 中的 `trace_id`。
fn decode_error_body(status: u16, body: String, header_trace_id: Option<String>) -> HttpError {
    if let Ok(v) = serde_json::from_str::<Value>(&body)
        && let Some(code) = v.get("code").and_then(Value::as_i64)
    {
        let message = v
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        // 优先用响应头 trace_id（经网关注入的更可靠），body 中的用作 fallback。
        let trace_id = header_trace_id
            .or_else(|| v.get("trace_id").and_then(Value::as_str).map(str::to_owned));
        return HttpError::ApiError {
            status,
            code,
            message,
            trace_id,
            body,
        };
    }
    HttpError::Status { status, body }
}

/// [`Bot`] builder。`credentials` 走 [`Self::build`] 必填——把"忘传凭证"前移到编译期。
pub struct BotBuilder {
    timeout: Duration,
    is_sandbox: bool,
    user_agent: String,
    api_base_override: Option<String>,
    token_endpoint_override: Option<String>,
    rate_capacity: Option<u32>,
    rate_refill: Option<f64>,
}

impl Default for BotBuilder {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            is_sandbox: false,
            user_agent: DEFAULT_USER_AGENT.to_owned(),
            api_base_override: None,
            token_endpoint_override: None,
            rate_capacity: None,
            rate_refill: None,
        }
    }
}

impl BotBuilder {
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn sandbox(mut self, on: bool) -> Self {
        self.is_sandbox = on;
        self
    }

    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = ua.into();
        self
    }

    #[cfg(test)]
    pub(crate) fn api_base(mut self, url: impl Into<String>) -> Self {
        self.api_base_override = Some(url.into());
        self
    }

    #[cfg(test)]
    pub(crate) fn token_endpoint(mut self, url: impl Into<String>) -> Self {
        self.token_endpoint_override = Some(url.into());
        self
    }

    /// 配置速率限制。`capacity` = 桶容量（最大突发请求数），`refill_per_sec` = 每秒补充数。
    /// 默认 5 容量、5/秒 补充。
    pub fn rate_limit(mut self, capacity: u32, refill_per_sec: f64) -> Self {
        self.rate_capacity = Some(capacity);
        self.rate_refill = Some(refill_per_sec);
        self
    }

    pub fn build(self, credentials: Credentials) -> Bot {
        let api_base = self.api_base_override.unwrap_or_else(|| {
            if self.is_sandbox {
                SANDBOX_API_BASE.to_owned()
            } else {
                PROD_API_BASE.to_owned()
            }
        });
        let token_endpoint = self
            .token_endpoint_override
            .unwrap_or_else(|| TOKEN_EXCHANGE_URL.to_owned());
        let rate_limiter = match (self.rate_capacity, self.rate_refill) {
            (Some(cap), Some(refill)) => RateLimiter::new(cap, refill),
            _ => RateLimiter::default(),
        };
        let http = ReqwestClient::builder()
            .timeout(self.timeout)
            .user_agent(self.user_agent)
            .build()
            // 仅 TLS 初始化失败会 Err，本配置不会触发。
            .expect("reqwest client config is always buildable");
        Bot {
            inner: Arc::new(Inner {
                http,
                creds: credentials,
                api_base,
                token_endpoint,
                token: Mutex::new(TokenState::default()),
                refresh: Mutex::new(()),
                next_msg_seq: AtomicU32::new(1),
                rate_limiter,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_token_response_decodes_string_expires_in() {
        let raw = r#"{"access_token":"xyz","expires_in":"7200"}"#;
        let r: AccessTokenResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(r.access_token, "xyz");
        assert_eq!(r.expires_in.parse::<u64>().unwrap(), 7200);
    }

    #[test]
    fn debug_does_not_leak_secret() {
        let creds = Credentials::new("12345", "topsecret");
        let client = Bot::new(creds);
        let s = format!("{client:?}");
        assert!(s.contains("12345"), "{s}");
        assert!(!s.contains("topsecret"), "{s}");
    }
}
