use serde::{Deserialize, Serialize};

use crate::error::HttpError;
use crate::http::Bot;

#[derive(Debug, Serialize)]
struct GenerateUrlLinkRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    callback_data: Option<&'a str>,
}

// 实测形态：服务端把 url 包在 `{retcode, msg, data: {url}}` 三层壳里——文档
// 只列了 `{url}` 裸字段，对不上。retcode != 0 时升级为 ApiError 透出业务码。
#[derive(Debug, Deserialize)]
struct GenerateUrlLinkResponse {
    retcode: i64,
    msg: String,
    #[serde(default)]
    data: Option<GenerateUrlLinkData>,
}

#[derive(Debug, Deserialize)]
struct GenerateUrlLinkData {
    url: String,
}

impl Bot {
    /// `POST /v2/generate_url_link`——生成机器人分享链接。
    ///
    /// `callback_data` ≤ 32 字符，用户走链接添加机器人时回带到 `FRIEND_ADD`
    /// 的 `scene_param`，便于做转化追踪。
    pub async fn generate_url_link(
        &self,
        callback_data: Option<&str>,
    ) -> Result<String, HttpError> {
        let body = GenerateUrlLinkRequest { callback_data };
        let resp: GenerateUrlLinkResponse = self.post_json("/v2/generate_url_link", &body).await?;
        match resp.data {
            Some(d) if resp.retcode == 0 => Ok(d.url),
            _ => Err(HttpError::ApiError {
                status: 200,
                code: resp.retcode,
                message: resp.msg.clone(),
                trace_id: None,
                body: format!(r#"{{"retcode":{},"msg":{:?}}}"#, resp.retcode, resp.msg),
            }),
        }
    }
}
