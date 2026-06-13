//! HTTP 路径的错误识别测试：token 拒绝、API 业务错误码、Status 兜底。
//!
//! 全部走 wiremock 起本地 HTTP server，不打真网。涉及业务接口选 `get_self`
//! （`GET /users/@me`）作为代理——是最薄的一个，断言失败时排错信息最少。

use serde_json::json;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::Credentials;
use crate::error::HttpError;
use crate::http::{Bot, ChannelPermissions, EmojiType, InteractionCallbackCode};
use crate::types::message::OutgoingChannelMessage;
use crate::types::payloads::FileType;

/// 起 mock server + 拼出 `Bot`，token 端点指 `/token`，业务 base 指 server 自身。
async fn make_client_against(server: &MockServer) -> Bot {
    let creds = Credentials::new("test-app-id", "test-app-secret");
    Bot::builder()
        .api_base(server.uri())
        .token_endpoint(format!("{}/token", server.uri()))
        .build(creds)
}

/// 注册一个能正常颁发 token 的端点——大多数测试只关心业务接口的行为。
async fn mount_happy_token(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "valid-token",
            "expires_in": "7200",
        })))
        .mount(server)
        .await;
}

/// **Token Happy Path**：valid 200 → `access_token()` 返回原值。
#[tokio::test]
async fn token_happy_path() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    let http = make_client_against(&server).await;

    let token = http.access_token().await.expect("token issued");
    assert_eq!(token, "valid-token");
}

/// **TokenRejected**：QQ 在凭证错时回 200 但响应体形如 `{"code": ..., "message": ...}`，
/// 没有 `access_token`。库要识别为 `TokenRejected` 而非"missing field"的 Decode 错。
#[tokio::test]
async fn token_endpoint_200_without_access_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 100007,
            "message": "appid not exist",
        })))
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let err = http.access_token().await.unwrap_err();
    match err {
        HttpError::TokenRejected { body } => {
            assert!(
                body.contains("appid not exist"),
                "body should preserve server message: {body}"
            );
        }
        other => panic!("expected TokenRejected, got {other:?}"),
    }
}

/// **Token 过期 / 缓存命中**：连续两次 `access_token()` 调用——
/// 第二次必须复用缓存，不再打 token 端点。
#[tokio::test]
async fn token_caches_within_expiry() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        // expect: token 端点最多被打 1 次。
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "cached-token",
            "expires_in": "7200",
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let t1 = http.access_token().await.unwrap();
    let t2 = http.access_token().await.unwrap();
    assert_eq!(t1, t2);
    // server.drop 时校验 expect(1)；显式触发让断言失败时报点更准。
    server.verify().await;
}

/// **ApiError 识别**：业务接口非 2xx 且 body 形如 `{"code", "message", "trace_id"}` →
/// `ApiError { code, message, trace_id, .. }`，不是 raw `Status`。
#[tokio::test]
async fn api_error_extracts_code_and_trace_id() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("GET"))
        .and(path("/users/@me"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "code": 40054005,
            "message": "消息被去重，请检查请求msgseq",
            "err_code": 40054005,
            "trace_id": "abc-trace-id-123",
        })))
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let err = http.get_self().await.unwrap_err();
    match err {
        HttpError::ApiError {
            status,
            code,
            message,
            trace_id,
            body,
        } => {
            assert_eq!(status, 400);
            assert_eq!(code, 40054005);
            assert_eq!(message, "消息被去重，请检查请求msgseq");
            assert_eq!(trace_id.as_deref(), Some("abc-trace-id-123"));
            assert!(
                body.contains("err_code"),
                "raw body should include err_code: {body}"
            );
        }
        other => panic!("expected ApiError, got {other:?}"),
    }
}

/// **Status 兜底**：非 2xx 但 body 不是可识别业务错误结构（如纯 HTML 5xx 网关页）→ Status。
#[tokio::test]
async fn non_json_5xx_falls_back_to_status() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("GET"))
        .and(path("/users/@me"))
        .respond_with(
            ResponseTemplate::new(503).set_body_string("<html>upstream gateway timeout</html>"),
        )
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let err = http.get_self().await.unwrap_err();
    match err {
        HttpError::Status { status, body } => {
            assert_eq!(status, 503);
            assert!(body.contains("upstream gateway timeout"));
        }
        other => panic!("expected Status, got {other:?}"),
    }
}

/// **撤回频道消息**：path + `hidetip` 查询参数都对，2xx 返回 `Ok(())`。
#[tokio::test]
async fn delete_channel_message_passes_hide_tip() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("DELETE"))
        .and(path("/channels/CID/messages/MID"))
        .and(query_param("hidetip", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.delete_channel_message("CID", "MID", true)
        .await
        .expect("delete ok");
    server.verify().await;
}

/// **撤回群消息**：path 命中 v2 group 形态。
#[tokio::test]
async fn delete_group_message_hits_v2_path() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("DELETE"))
        .and(path("/v2/groups/GID/messages/MID"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.delete_group_message("GID", "MID").await.expect("ok");
    server.verify().await;
}

/// **上传群富媒体**：请求 body 三字段都对，响应 `file_info` / `file_uuid` / `ttl` 透传到 `Media`。
#[tokio::test]
async fn post_group_file_uploads_and_returns_media() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("POST"))
        .and(path("/v2/groups/GID/files"))
        .and(body_json(json!({
            "file_type": 1,
            "url": "https://cdn/example.png",
            "srv_send_msg": false,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "file_uuid": "uuid-1",
            "file_info": "opaque-info-blob",
            "ttl": 600,
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let media = http
        .post_group_file("GID", FileType::Image, "https://cdn/example.png", false)
        .await
        .expect("upload ok");
    assert_eq!(media.file_uuid.as_deref(), Some("uuid-1"));
    assert_eq!(media.file_info, "opaque-info-blob");
    assert_eq!(media.ttl, 600);
    server.verify().await;
}

/// **撤回失败透出 ApiError**：服务端拒绝时不能静默吞。
#[tokio::test]
async fn delete_failure_surfaces_api_error() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("DELETE"))
        .and(path("/v2/users/UID/messages/MID"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "code": 11253,
            "message": "no permission",
        })))
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let err = http.delete_c2c_message("UID", "MID").await.unwrap_err();
    match err {
        HttpError::ApiError { code, .. } => assert_eq!(code, 11253),
        other => panic!("expected ApiError, got {other:?}"),
    }
}

/// **按钮 ACK Happy**：PUT /interactions/{id}，body `{"code":0}` 命中
/// （`InteractionCallbackCode::Success` 必须序列化为整数 0），200 → Ok(())。
#[tokio::test]
async fn put_interaction_callback_serializes_code_as_int() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("PUT"))
        .and(path("/interactions/IID-1"))
        .and(body_json(json!({ "code": 0 })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.put_interaction_callback("IID-1", InteractionCallbackCode::Success)
        .await
        .expect("ack ok");
    server.verify().await;
}

/// **按钮 ACK 非零码**：`Duplicate = 3` 序列化为 3——枚举到线协议的映射是核心契约。
#[tokio::test]
async fn put_interaction_callback_duplicate_code_is_three() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("PUT"))
        .and(path("/interactions/IID-2"))
        .and(body_json(json!({ "code": 3 })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.put_interaction_callback("IID-2", InteractionCallbackCode::Duplicate)
        .await
        .expect("ack ok");
    server.verify().await;
}

/// **频道私信发送**：POST /dms/{guild_id}/messages 走 v1 路径（不是 /v2/users），
/// 复用 `OutgoingChannelMessage`，响应 `id` 透出 `SentMessage`。
#[tokio::test]
async fn post_dm_message_hits_v1_dms_path() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("POST"))
        .and(path("/dms/DM-GID/messages"))
        .and(body_json(json!({ "content": "hi" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "MID-dm-1",
            "timestamp": "2026-04-30T12:00:00+00:00",
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let sent = http
        .post_dm_message("DM-GID", &OutgoingChannelMessage::text("hi"))
        .await
        .expect("send ok");
    assert_eq!(sent.id, "MID-dm-1");
    server.verify().await;
}

/// **加表态**：path 拼出 `/reactions/{type}/{id}`——`EmojiType::System` 取值 1，
/// emoji_id 原样进 path。
#[tokio::test]
async fn put_channel_reaction_path_contains_type_and_id() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("PUT"))
        .and(path("/channels/CID/messages/MID/reactions/1/4"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.put_channel_reaction("CID", "MID", EmojiType::System, "4")
        .await
        .expect("react ok");
    server.verify().await;
}

/// **撤表态**：DELETE 同 path 形态——`EmojiType::Emoji` 取值 2。
#[tokio::test]
async fn delete_channel_reaction_path_uses_emoji_type_2() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("DELETE"))
        .and(path("/channels/CID/messages/MID/reactions/2/129315"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.delete_channel_reaction("CID", "MID", EmojiType::Emoji, "129315")
        .await
        .expect("undo ok");
    server.verify().await;
}

/// **拉表态用户列表（首页）**：cookie 传 None 时不带查询串；返回的
/// `users / cookie / is_end` 都解析出来。
#[tokio::test]
async fn list_channel_reaction_users_first_page_parses_pagination() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("GET"))
        .and(path("/channels/CID/messages/MID/reactions/1/4"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "users": [
                { "id": "U1", "username": "alice" },
                { "id": "U2", "username": "bob" }
            ],
            "cookie": "next-cursor-1",
            "is_end": false,
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let page = http
        .list_channel_reaction_users("CID", "MID", EmojiType::System, "4", None)
        .await
        .expect("list ok");
    assert_eq!(page.users.len(), 2);
    assert_eq!(page.users[0].id, "U1");
    assert_eq!(page.cookie, "next-cursor-1");
    assert!(!page.is_end);
    server.verify().await;
}

/// **拉表态用户列表（带 cookie）**：含 `+` / `=` 等会破坏 query 串的字符
/// 必须 percent-encode 后再发出去，服务端 query_param 匹配的是解码后的原值。
#[tokio::test]
async fn list_channel_reaction_users_url_encodes_cookie() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("GET"))
        .and(path("/channels/CID/messages/MID/reactions/1/4"))
        .and(query_param("cookie", "abc+def=ghi&xyz"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "users": [],
            "cookie": "",
            "is_end": true,
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let page = http
        .list_channel_reaction_users(
            "CID",
            "MID",
            EmojiType::System,
            "4",
            Some("abc+def=ghi&xyz"),
        )
        .await
        .expect("list ok");
    assert!(page.is_end);
    server.verify().await;
}

/// **生成分享链接（含 callback_data）**：响应实测是 `{retcode, msg, data:{url}}` 包
/// 三层壳——SDK 必须按这个形态解，并把 url 从 data 里挑出来。
#[tokio::test]
async fn generate_url_link_with_callback_data() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("POST"))
        .and(path("/v2/generate_url_link"))
        .and(body_json(json!({ "callback_data": "campaign-42" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "retcode": 0,
            "msg": "success",
            "data": {
                "url": "https://qun.qq.com/qunpro/robot/qunshare?xxx",
            }
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let url = http
        .generate_url_link(Some("campaign-42"))
        .await
        .expect("ok");
    assert!(url.starts_with("https://qun.qq.com/"), "got: {url}");
    server.verify().await;
}

/// **生成分享链接（无 callback_data）**：None 不应序列化进 body——只回 `{}`。
#[tokio::test]
async fn generate_url_link_omits_none_callback_data() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("POST"))
        .and(path("/v2/generate_url_link"))
        .and(body_json(json!({})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "retcode": 0,
            "msg": "success",
            "data": { "url": "https://example/share" }
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let _ = http.generate_url_link(None).await.expect("ok");
    server.verify().await;
}

/// **retcode != 0 → ApiError**：服务端返回 200 但业务层拒绝（接口未开通等），
/// 必须按 ApiError 透出 retcode + msg，不让调用方误以为成功。
#[tokio::test]
async fn generate_url_link_nonzero_retcode_surfaces_api_error() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("POST"))
        .and(path("/v2/generate_url_link"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "retcode": 11253,
            "msg": "permission denied",
        })))
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let err = http.generate_url_link(None).await.unwrap_err();
    match err {
        HttpError::ApiError { code, message, .. } => {
            assert_eq!(code, 11253);
            assert_eq!(message, "permission denied");
        }
        other => panic!("expected ApiError, got {other:?}"),
    }
}

/// **2xx 但 schema 不匹配 → Decode 错带原始 body**：QQ 文档跟实际返回字段对不
/// 齐时，错误必须带回响应原文，否则上层完全看不到服务端到底回了啥。
#[tokio::test]
async fn decode_failure_preserves_response_body() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("POST"))
        .and(path("/v2/generate_url_link"))
        // 故意把字段名写错，模拟"文档说有 url 实际没有"。
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "share_url": "https://example/share",
            "code": 0,
        })))
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let err = http.generate_url_link(None).await.unwrap_err();
    match err {
        HttpError::Decode { body, source } => {
            assert!(
                body.contains("share_url"),
                "body should retain server response: {body}"
            );
            // Display 把 body 也带上——光看错误消息就能定位字段名问题。
            let display = format!("{}", HttpError::Decode { body, source });
            assert!(display.contains("share_url"), "display: {display}");
        }
        other => panic!("expected Decode, got {other:?}"),
    }
}

/// **创建私信会话**：路径 `/users/@me/dms`（`@me` 字面量），body 含 recipient_id /
/// source_guild_id；响应 guild_id / channel_id / create_time 透出到 DmSession。
#[tokio::test]
async fn create_dm_uses_at_me_path() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("POST"))
        .and(path("/users/@me/dms"))
        .and(body_json(json!({
            "recipient_id": "USER-1",
            "source_guild_id": "GUILD-1",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "guild_id": "DM-GUILD-1",
            "channel_id": "DM-CHAN-1",
            "create_time": "2026-04-30T12:00:00+00:00",
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let session = http.create_dm("USER-1", "GUILD-1").await.expect("ok");
    assert_eq!(session.guild_id, "DM-GUILD-1");
    assert_eq!(session.channel_id, "DM-CHAN-1");
    assert!(session.create_time.is_some());
    server.verify().await;
}

/// **ApiError 错误描述含 trace_id**：方便 grep 日志时定位。
#[tokio::test]
async fn api_error_display_includes_trace_id() {
    let err = HttpError::ApiError {
        status: 400,
        code: 11253,
        message: "permission denied".into(),
        trace_id: Some("trace-xyz".into()),
        body: "{}".into(),
    };
    let s = err.to_string();
    assert!(s.contains("11253"), "code in display: {s}");
    assert!(s.contains("trace_id=trace-xyz"), "trace_id in display: {s}");
}

// ─── Guilds / Channels API tests ───

/// **get_guild**：`GET /guilds/{guild_id}` 解析出 Guild。
#[tokio::test]
async fn get_guild_parses_response() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("GET"))
        .and(path("/guilds/G-001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "G-001",
            "name": "My Guild",
            "owner_id": "U-OWNER",
            "member_count": 42,
            "max_members": 500,
            "description": "A test guild",
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let guild = http.get_guild("G-001").await.expect("get ok");
    assert_eq!(guild.id, "G-001");
    assert_eq!(guild.name, "My Guild");
    assert_eq!(guild.owner_id.as_deref(), Some("U-OWNER"));
    assert_eq!(guild.member_count, Some(42));
    server.verify().await;
}

/// **get_guild_members**：GET 带 limit + after 分页参数，响应解析为 `GuildMemberPage`。
#[tokio::test]
async fn get_guild_members_with_pagination() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("GET"))
        .and(path("/guilds/G-001/members"))
        .and(query_param("limit", "50"))
        .and(query_param("after", "cursor-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [
                {
                    "user": { "id": "U1", "username": "alice", "bot": false },
                    "nick": "Alice",
                    "roles": [],
                    "joined_at": "2026-04-27T00:00:00+00:00"
                },
                {
                    "user": { "id": "U2", "username": "bob", "bot": false },
                    "nick": "Bob",
                    "roles": ["admin"],
                    "joined_at": "2026-05-01T00:00:00+00:00"
                }
            ],
            "next": "cursor-2"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let page = http
        .get_guild_members("G-001", Some(50), Some("cursor-1"))
        .await
        .expect("list ok");
    assert_eq!(page.data.len(), 2);
    assert_eq!(page.data[0].user.username, "alice");
    assert_eq!(page.data[1].nick.as_deref(), Some("Bob"));
    assert_eq!(page.data[1].roles, vec!["admin"]);
    assert_eq!(page.next.as_deref(), Some("cursor-2"));
    server.verify().await;
}

/// **delete_guild_member**：DELETE 路径带 add_blacklist 查询参数。
#[tokio::test]
async fn delete_guild_member_with_blacklist() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("DELETE"))
        .and(path("/guilds/G-001/members/U-BAD"))
        .and(query_param("add_blacklist", "true"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.delete_guild_member("G-001", "U-BAD", true, None)
        .await
        .expect("kick ok");
    server.verify().await;
}

/// **get_channels**：GET 频道子频道列表。
#[tokio::test]
async fn get_channels_returns_list() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("GET"))
        .and(path("/guilds/G-001/channels"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": "CH-1",
                "guild_id": "G-001",
                "name": "general",
                "type": 0
            },
            {
                "id": "CH-2",
                "guild_id": "G-001",
                "name": "voice",
                "type": 2,
                "parent_id": "CH-1"
            }
        ])))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let channels = http.get_channels("G-001").await.expect("list ok");
    assert_eq!(channels.len(), 2);
    assert_eq!(channels[0].name, "general");
    assert_eq!(channels[0].type_, 0);
    assert_eq!(channels[1].parent_id.as_deref(), Some("CH-1"));
    server.verify().await;
}

/// **create_channel**：POST 创建子频道，body 含 name + type。
#[tokio::test]
async fn create_channel_posts_with_type() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("POST"))
        .and(path("/guilds/G-001/channels"))
        .and(body_json(json!({
            "name": "new-channel",
            "type": 0
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "CH-NEW",
            "guild_id": "G-001",
            "name": "new-channel",
            "type": 0
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let channel = http
        .create_channel("G-001", "new-channel", 0, None)
        .await
        .expect("create ok");
    assert_eq!(channel.id, "CH-NEW");
    assert_eq!(channel.name, "new-channel");
    server.verify().await;
}

/// **patch_channel**：PATCH 更新子频道名称。
#[tokio::test]
async fn patch_channel_hits_patch_verb() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("PATCH"))
        .and(path("/channels/CH-1"))
        .and(body_json(json!({ "name": "renamed" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "CH-1",
            "guild_id": "G-001",
            "name": "renamed",
            "type": 0
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let channel = http
        .patch_channel("CH-1", Some("renamed"), None, None)
        .await
        .expect("patch ok");
    assert_eq!(channel.name, "renamed");
    server.verify().await;
}

/// **delete_channel**：DELETE 子频道。
#[tokio::test]
async fn delete_channel_hits_path() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("DELETE"))
        .and(path("/channels/CH-1"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.delete_channel("CH-1").await.expect("delete ok");
    server.verify().await;
}

/// **delete_guild**：DELETE 解散频道。
#[tokio::test]
async fn delete_guild_hits_path() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("DELETE"))
        .and(path("/guilds/G-001"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.delete_guild("G-001").await.expect("delete ok");
    server.verify().await;
}

// ─── delete_guild_member with delete_history_days ───

/// **踢出成员并删除历史消息**：`delete_history_days` 出现在查询串中。
#[tokio::test]
async fn delete_guild_member_with_history_days() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("DELETE"))
        .and(path("/guilds/G-001/members/U-BAD"))
        .and(query_param("add_blacklist", "false"))
        .and(query_param("delete_history_days", "7"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.delete_guild_member("G-001", "U-BAD", false, Some(7))
        .await
        .expect("delete ok");
    server.verify().await;
}

// ─── get_guild_member ───

/// **获取单个成员**：`GET /guilds/{guild_id}/members/{user_id}` 返回 `GuildMemberEntry`。
#[tokio::test]
async fn get_guild_member_returns_entry() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("GET"))
        .and(path("/guilds/G-001/members/U-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "user": { "id": "U-1", "username": "alice", "bot": false },
            "nick": "Alice",
            "roles": ["admin"],
            "joined_at": "2026-04-27T00:00:00+00:00"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let member = http.get_guild_member("G-001", "U-1").await.expect("get ok");
    assert_eq!(member.user.id, "U-1");
    assert_eq!(member.nick.as_deref(), Some("Alice"));
    assert_eq!(member.roles, vec!["admin"]);
    server.verify().await;
}

// ─── Roles API tests ───

/// **获取身份组列表**：`GET /guilds/{guild_id}/roles` → `RolePage`。
#[tokio::test]
async fn get_roles_returns_role_list() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("GET"))
        .and(path("/guilds/G-001/roles"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "roles": [
                { "id": "R-1", "name": "Admin", "color": 3447003, "hoist": true, "member_count": 3, "member_limit": 10 },
                { "id": "R-2", "name": "Member", "color": 0, "hoist": false, "member_count": 42, "member_limit": 100 }
            ],
            "role_num_limit": "5"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let page = http.get_roles("G-001").await.expect("list ok");
    assert_eq!(page.roles.len(), 2);
    assert_eq!(page.roles[0].name, "Admin");
    assert_eq!(page.roles[0].color, 3447003);
    assert!(page.roles[0].hoist);
    assert_eq!(page.roles[1].member_count, 42);
    assert_eq!(page.role_num_limit.as_deref(), Some("5"));
    server.verify().await;
}

/// **创建身份组**：`POST /guilds/{guild_id}/roles` 带 body，返回 `Role`。
#[tokio::test]
async fn create_role_posts_with_body() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("POST"))
        .and(path("/guilds/G-001/roles"))
        .and(body_json(json!({
            "name": "Moderator",
            "color": 16776960,
            "hoist": true
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "R-NEW",
            "name": "Moderator",
            "color": 16776960,
            "hoist": true,
            "member_count": 0,
            "member_limit": 50
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let role = http
        .create_role("G-001", "Moderator", Some(16776960), Some(true))
        .await
        .expect("create ok");
    assert_eq!(role.id, "R-NEW");
    assert_eq!(role.name, "Moderator");
    assert_eq!(role.color, 16776960);
    server.verify().await;
}

/// **更新身份组**：`PATCH /guilds/{guild_id}/roles/{role_id}`。
#[tokio::test]
async fn patch_role_updates_name() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("PATCH"))
        .and(path("/guilds/G-001/roles/R-1"))
        .and(body_json(json!({ "name": "Super Admin" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "R-1",
            "name": "Super Admin",
            "color": 3447003,
            "hoist": true,
            "member_count": 3,
            "member_limit": 10
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let role = http
        .patch_role("G-001", "R-1", Some("Super Admin"), None, None)
        .await
        .expect("patch ok");
    assert_eq!(role.name, "Super Admin");
    server.verify().await;
}

/// **删除身份组**：`DELETE /guilds/{guild_id}/roles/{role_id}`。
#[tokio::test]
async fn delete_role_hits_path() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("DELETE"))
        .and(path("/guilds/G-001/roles/R-1"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.delete_role("G-001", "R-1").await.expect("delete ok");
    server.verify().await;
}

/// **添加成员身份组**：`PUT /guilds/{guild_id}/members/{user_id}/roles/{role_id}`。
#[tokio::test]
async fn put_member_role_hits_path() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("PUT"))
        .and(path("/guilds/G-001/members/U-1/roles/R-1"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.put_member_role("G-001", "U-1", "R-1")
        .await
        .expect("put ok");
    server.verify().await;
}

/// **移除成员身份组**：`DELETE /guilds/{guild_id}/members/{user_id}/roles/{role_id}`。
#[tokio::test]
async fn delete_member_role_hits_path() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("DELETE"))
        .and(path("/guilds/G-001/members/U-1/roles/R-1"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.delete_member_role("G-001", "U-1", "R-1")
        .await
        .expect("delete ok");
    server.verify().await;
}

// ─── Mutes API tests ───

/// **全员禁言**：`PATCH /guilds/{guild_id}/mute` body 含 `mute_seconds`。
#[tokio::test]
async fn mute_guild_with_seconds() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("PATCH"))
        .and(path("/guilds/G-001/mute"))
        .and(body_json(json!({ "mute_seconds": "60" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.mute_guild("G-001", None, Some("60"))
        .await
        .expect("mute ok");
    server.verify().await;
}

/// **取消全员禁言**：`PATCH /guilds/{guild_id}/mute` body 含 `mute_end_timestamp: "0"`。
#[tokio::test]
async fn unmute_guild_sends_zero() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("PATCH"))
        .and(path("/guilds/G-001/mute"))
        .and(body_json(json!({ "mute_end_timestamp": "0" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.unmute_guild("G-001").await.expect("unmute ok");
    server.verify().await;
}

/// **禁言单个成员**：`PATCH /guilds/{guild_id}/members/{user_id}/mute` body 含 `mute_end_timestamp`。
#[tokio::test]
async fn mute_member_with_timestamp() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("PATCH"))
        .and(path("/guilds/G-001/members/U-1/mute"))
        .and(body_json(json!({ "mute_end_timestamp": "1750000000" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.mute_member("G-001", "U-1", Some("1750000000"), None)
        .await
        .expect("mute ok");
    server.verify().await;
}

/// **解除单个成员禁言**：`PATCH /guilds/{guild_id}/members/{user_id}/mute` body 含 `mute_end_timestamp: "0"`。
#[tokio::test]
async fn unmute_member_sends_zero() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("PATCH"))
        .and(path("/guilds/G-001/members/U-1/mute"))
        .and(body_json(json!({ "mute_end_timestamp": "0" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.unmute_member("G-001", "U-1").await.expect("unmute ok");
    server.verify().await;
}

// ─── Permissions API tests ───

/// **获取角色子频道权限**：`GET /channels/{id}/permissions/role/{role_id}`。
#[tokio::test]
async fn get_channel_role_permissions_parses() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("GET"))
        .and(path("/channels/CH-1/permissions/role/R-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "channel_id": "CH-1",
            "role_id": "R-1",
            "permissions": "114"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let perms = http
        .get_channel_role_permissions("CH-1", "R-1")
        .await
        .expect("get ok");
    assert_eq!(perms.channel_id.as_deref(), Some("CH-1"));
    assert_eq!(perms.role_id.as_deref(), Some("R-1"));
    assert_eq!(perms.permissions, "114");
    server.verify().await;
}

/// **设置角色子频道权限**：`PUT /channels/{id}/permissions/role/{role_id}` body 含 add/remove。
#[tokio::test]
async fn put_channel_role_permissions_sends_add_remove() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("PUT"))
        .and(path("/channels/CH-1/permissions/role/R-1"))
        .and(body_json(json!({ "add": "1", "remove": "2" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.put_channel_role_permissions("CH-1", "R-1", Some("1"), Some("2"))
        .await
        .expect("put ok");
    server.verify().await;
}

/// **获取成员子频道权限**：`GET /channels/{id}/permissions/member/{user_id}`。
#[tokio::test]
async fn get_channel_member_permissions_parses() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("GET"))
        .and(path("/channels/CH-1/permissions/member/U-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "channel_id": "CH-1",
            "user_id": "U-1",
            "permissions": "114"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let perms = http
        .get_channel_member_permissions("CH-1", "U-1")
        .await
        .expect("get ok");
    assert_eq!(perms.channel_id.as_deref(), Some("CH-1"));
    assert_eq!(perms.user_id.as_deref(), Some("U-1"));
    assert_eq!(perms.permissions, "114");
    server.verify().await;
}

/// **设置成员子频道权限**：`PUT /channels/{id}/permissions/member/{user_id}`。
#[tokio::test]
async fn put_channel_member_permissions_sends_add_remove() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("PUT"))
        .and(path("/channels/CH-1/permissions/member/U-1"))
        .and(body_json(json!({ "add": "8" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.put_channel_member_permissions("CH-1", "U-1", Some("8"), None)
        .await
        .expect("put ok");
    server.verify().await;
}

/// **permissions 字段兼容数字**：`serde` 能将 `"permissions": 114` (数字) 转为 String。
#[tokio::test]
async fn channel_permissions_string_or_number() {
    let raw = json!({
        "channel_id": "CH-1",
        "user_id": "U-1",
        "permissions": 114
    });
    let p: ChannelPermissions = serde_json::from_value(raw).expect("deserialize");
    assert_eq!(p.permissions, "114");
}

// ─── Voice members test ───

/// **获取语音频道成员**：`GET /channels/{id}/voice/members`。
#[tokio::test]
async fn get_voice_members_returns_list() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("GET"))
        .and(path("/channels/CH-1/voice/members"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "uid": "UID-1",
                "member": {
                    "user": { "id": "U-1", "username": "alice", "bot": false },
                    "nick": "Alice",
                    "roles": [],
                    "joined_at": "2026-04-27T00:00:00+00:00"
                }
            },
            {
                "uid": "UID-2",
                "member": {
                    "user": { "id": "U-2", "username": "bob", "bot": false },
                    "roles": []
                }
            }
        ])))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let members = http.get_voice_members("CH-1").await.expect("list ok");
    assert_eq!(members.len(), 2);
    assert_eq!(members[0].uid, "UID-1");
    assert_eq!(members[0].member.user.username, "alice");
    assert_eq!(members[1].uid, "UID-2");
    server.verify().await;
}

// ─── Message query/edit tests ───

/// **获取频道消息**：`GET /channels/{id}/messages/{msg_id}` → `ChannelMessage`。
#[tokio::test]
async fn get_channel_message_returns_message() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("GET"))
        .and(path("/channels/CH-1/messages/MID-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "MID-1",
            "channel_id": "CH-1",
            "guild_id": "G-001",
            "content": "hello",
            "author": { "id": "U-1", "username": "alice", "bot": false },
            "timestamp": "2026-06-13T00:00:00+00:00",
            "edited_timestamp": "2026-06-13T01:00:00+00:00"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let msg = http
        .get_channel_message("CH-1", "MID-1")
        .await
        .expect("get ok");
    assert_eq!(msg.id, "MID-1");
    assert_eq!(msg.content, "hello");
    assert_eq!(msg.author.username, "alice");
    assert_eq!(
        msg.edited_timestamp.as_deref(),
        Some("2026-06-13T01:00:00+00:00")
    );
    server.verify().await;
}

/// **编辑频道消息**：`PATCH /channels/{id}/messages/{msg_id}` → `ChannelMessage`。
#[tokio::test]
async fn patch_channel_message_updates_content() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("PATCH"))
        .and(path("/channels/CH-1/messages/MID-1"))
        .and(body_json(json!({ "content": "edited text" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "MID-1",
            "channel_id": "CH-1",
            "guild_id": "G-001",
            "content": "edited text",
            "author": { "id": "U-BOT", "username": "mybot", "bot": true },
            "timestamp": "2026-06-13T00:00:00+00:00",
            "edited_timestamp": "2026-06-13T01:30:00+00:00"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let msg = http
        .patch_channel_message("CH-1", "MID-1", Some("edited text"), None)
        .await
        .expect("patch ok");
    assert_eq!(msg.content, "edited text");
    assert!(msg.edited_timestamp.is_some());
    server.verify().await;
}

// ─── API Permissions tests ───

/// **获取 API 权限列表**：`GET /guilds/{guild_id}/api_permissions` → `ApiPermissionList`。
#[tokio::test]
async fn get_api_permissions_parses_list() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("GET"))
        .and(path("/guilds/G-001/api_permissions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "permissions": [
                {
                    "api": { "path": "/guilds/{guild_id}/members/{user_id}", "method": "GET" },
                    "auth_status": 1
                },
                {
                    "api": { "path": "/guilds/{guild_id}/members", "method": "GET" },
                    "auth_status": 2
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let list = http.get_api_permissions("G-001").await.expect("get ok");
    assert_eq!(list.permissions.len(), 2);
    assert_eq!(list.permissions[0].auth_status, 1);
    assert_eq!(
        list.permissions[0].api.path,
        "/guilds/{guild_id}/members/{user_id}"
    );
    assert_eq!(list.permissions[1].auth_status, 2);
    server.verify().await;
}

/// **申请 API 权限**：`POST /guilds/{guild_id}/api_permissions/demand`。
#[tokio::test]
async fn demand_api_permission_sends_body() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("POST"))
        .and(path("/guilds/G-001/api_permissions/demand"))
        .and(body_json(json!({
            "channel_id": "CH-1",
            "api_identify": { "path": "/guilds/{guild_id}/members", "method": "GET" },
            "desc": "need member list for welcome feature"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "task_id": "TASK-1",
            "status": 1
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let demand = http
        .demand_api_permission(
            "G-001",
            "CH-1",
            "/guilds/{guild_id}/members",
            "GET",
            "need member list for welcome feature",
        )
        .await
        .expect("demand ok");
    assert_eq!(demand.task_id, "TASK-1");
    assert_eq!(demand.status, 1);
    server.verify().await;
}

// ─── Batch mute tests ───

/// **批量禁言**：`PATCH /guilds/{guild_id}/mute` body 含 `user_ids` 数组。
#[tokio::test]
async fn mute_multi_member_with_user_ids() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("PATCH"))
        .and(path("/guilds/G-001/mute"))
        .and(body_json(json!({
            "user_ids": ["U-1", "U-2"],
            "mute_seconds": "120"
        })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.mute_multi_member("G-001", &["U-1", "U-2"], None, Some("120"))
        .await
        .expect("multi mute ok");
    server.verify().await;
}

/// **批量解除禁言**：`PATCH /guilds/{guild_id}/mute` body 含 `user_ids` + `mute_end_timestamp: "0"`。
#[tokio::test]
async fn unmute_multi_member_sends_zero() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("PATCH"))
        .and(path("/guilds/G-001/mute"))
        .and(body_json(json!({
            "user_ids": ["U-1", "U-2"],
            "mute_end_timestamp": "0"
        })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    http.unmute_multi_member("G-001", &["U-1", "U-2"])
        .await
        .expect("multi unmute ok");
    server.verify().await;
}

// ─── Role members test ───

/// **获取角色成员列表**：`GET /guilds/{guild_id}/roles/{role_id}/members` 分页返回。
#[tokio::test]
async fn get_role_members_returns_page() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("GET"))
        .and(path("/guilds/G-001/roles/R-1/members"))
        .and(query_param("start_index", "cursor-1"))
        .and(query_param("limit", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [
                {
                    "user": { "id": "U-1", "username": "alice", "bot": false },
                    "nick": "Alice",
                    "roles": ["R-1"],
                    "joined_at": "2026-04-27T00:00:00+00:00"
                },
                {
                    "user": { "id": "U-2", "username": "bob", "bot": false },
                    "nick": "Bob",
                    "roles": ["R-1"],
                    "joined_at": "2026-05-01T00:00:00+00:00"
                }
            ],
            "next": "cursor-2"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let page = http
        .get_role_members("G-001", "R-1", Some("cursor-1"), Some(50))
        .await
        .expect("get ok");
    assert_eq!(page.data.len(), 2);
    assert_eq!(page.data[0].user.username, "alice");
    assert_eq!(page.data[0].roles, vec!["R-1"]);
    assert_eq!(page.next.as_deref(), Some("cursor-2"));
    server.verify().await;
}

// ─── Patch guild test ───

/// **更新频道信息**：`PATCH /guilds/{guild_id}` body 含 name/description/icon。
#[tokio::test]
async fn patch_guild_updates_name() {
    let server = MockServer::start().await;
    mount_happy_token(&server).await;
    Mock::given(method("PATCH"))
        .and(path("/guilds/G-001"))
        .and(body_json(json!({ "name": "New Guild Name" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "G-001",
            "name": "New Guild Name",
            "owner_id": "U-OWNER",
            "description": "updated",
            "member_count": 42,
            "max_members": 500
        })))
        .expect(1)
        .mount(&server)
        .await;
    let http = make_client_against(&server).await;

    let guild = http
        .patch_guild("G-001", Some("New Guild Name"), None, None)
        .await
        .expect("patch ok");
    assert_eq!(guild.name, "New Guild Name");
    assert_eq!(guild.id, "G-001");
    server.verify().await;
}
