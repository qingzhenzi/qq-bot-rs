//! 机器人凭证：`AppID` + `AppSecret`。
//!
//! v2 协议用这对凭证通过 OAuth 类似流程换 access_token，再以
//! `Authorization: QQBot <access_token>` 调 API / 鉴权 WebSocket。
//! 本模块只保管原始凭证；token 交换在 `http` 层。

use std::env;
use std::fmt;

use crate::error::AuthError;

const ENV_APP_ID: &str = "QQ_BOT_APP_ID";
const ENV_APP_SECRET: &str = "QQ_BOT_APP_SECRET";

/// 机器人凭证。
///
/// `Debug` 手写脱敏 `app_secret`；故意不实现 `Display`，杜绝 `format!("{cred}")`
/// 这种隐式输出渠道。
#[derive(Clone)]
pub struct Credentials {
    app_id: String,
    app_secret: String,
}

impl Credentials {
    pub fn new(app_id: impl Into<String>, app_secret: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
            app_secret: app_secret.into(),
        }
    }

    /// 从 `QQ_BOT_APP_ID` / `QQ_BOT_APP_SECRET` 加载。不读 `.env`——按需在
    /// 程序入口自行 `dotenvy::dotenv()?`。
    pub fn from_env() -> Result<Self, AuthError> {
        Self::from_env_vars(ENV_APP_ID, ENV_APP_SECRET)
    }

    /// 从指定前缀的环境变量加载：`{PREFIX}_APP_ID` / `{PREFIX}_APP_SECRET`。
    /// 例如 `prefix = "QQ_BOT_0"` → `QQ_BOT_0_APP_ID` / `QQ_BOT_0_APP_SECRET`。
    pub fn from_env_with_prefix(prefix: &str) -> Result<Self, AuthError> {
        let id_var = format!("{}_APP_ID", prefix);
        let secret_var = format!("{}_APP_SECRET", prefix);
        Self::from_env_vars(&id_var, &secret_var)
    }

    /// 从带序号的环境变量加载：`QQ_BOT_APP_ID_{index}` / `QQ_BOT_APP_SECRET_{index}`。
    /// 常配合多账号配置使用，如 `Credentials::from_env_index(0)`。
    pub fn from_env_index(index: usize) -> Result<Self, AuthError> {
        let prefix = format!("QQ_BOT_{}", index);
        Self::from_env_with_prefix(&prefix)
    }

    fn from_env_vars(id_var: &str, secret_var: &str) -> Result<Self, AuthError> {
        let app_id = env::var(id_var).map_err(|_| AuthError::MissingEnv(id_var.to_owned()))?;
        let app_secret =
            env::var(secret_var).map_err(|_| AuthError::MissingEnv(secret_var.to_owned()))?;
        Ok(Self { app_id, app_secret })
    }

    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    /// AppSecret——敏感字段，调用方负责不要外传 / 写日志。
    pub fn app_secret(&self) -> &str {
        &self.app_secret
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("app_id", &self.app_id)
            .field("app_secret", &MaskedSecret(&self.app_secret))
            .finish()
    }
}

struct MaskedSecret<'a>(&'a str);

// 首尾各保留 4 字符——便于肉眼校对加载源（".env 写错了吗？"），
// 又不暴露完整长度信息。
const SECRET_KEEP: usize = 4;

impl fmt::Debug for MaskedSecret<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 套引号匹配 String 的 Debug 视觉风格。
        write!(f, "\"{}\"", mask_secret(self.0))
    }
}

// 不够长直接全脱敏——保留首尾会暴露过半。
fn mask_secret(secret: &str) -> String {
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() <= SECRET_KEEP * 2 + 1 {
        return "<redacted>".into();
    }
    let head: String = chars.iter().take(SECRET_KEEP).collect();
    let tail: String = chars.iter().rev().take(SECRET_KEEP).rev().collect();
    format!("{head}***{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_masks_long_secret() {
        let c = Credentials::new("12345", "abcdEFGHIJKLwxyz");
        let s = format!("{c:?}");
        assert!(s.contains("12345"), "{s}");
        assert!(!s.contains("abcdEFGHIJKLwxyz"), "{s}");
        assert!(s.contains("abcd"), "{s}");
        assert!(s.contains("wxyz"), "{s}");
        assert!(s.contains("***"), "{s}");
        assert!(!s.contains("EFGH"), "{s}");
    }

    #[test]
    fn debug_fully_redacts_short_secret() {
        let c = Credentials::new("12345", "abcdefghi");
        let s = format!("{c:?}");
        assert!(s.contains("redacted"), "{s}");
        assert!(!s.contains("abcd"), "{s}");
        assert!(!s.contains("ghi"), "{s}");
    }
}
