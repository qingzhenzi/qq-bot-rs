use serde::{Deserialize, Serialize};

/// Markdown 载荷。两种形态二选一：模板（`custom_template_id` + `params`）或
/// 原文（`content`）。
///
/// **支持矩阵**：v2 群 / c2c 原文 markdown 已对所有 bot 开放；v1 频道当前需
/// QQ 内部邀请激活。
///
/// `content` 里可嵌入 QQ 文本链 / 内联控件标签，SDK 不解析：
/// ```text
/// <qqbot-at-user id="100" />
/// <qqbot-at-everyone />
/// <qqbot-cmd-input text="help" show="/help" reference="false" />
/// <#channel_id>
/// <emoji:id>
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct MarkdownPayload {
    /// 需平台备案。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_template_id: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<MarkdownParam>,

    /// 原文 markdown——与 `custom_template_id` 互斥。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

impl MarkdownPayload {
    pub fn template(custom_template_id: impl Into<String>) -> Self {
        Self {
            custom_template_id: Some(custom_template_id.into()),
            params: Vec::new(),
            content: None,
        }
    }

    pub fn raw(content: impl Into<String>) -> Self {
        Self {
            custom_template_id: None,
            params: Vec::new(),
            content: Some(content.into()),
        }
    }

    pub fn param<I, S>(mut self, key: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.params.push(MarkdownParam {
            key: key.into(),
            values: values.into_iter().map(Into::into).collect(),
        });
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarkdownParam {
    /// 占位符名（与模板里的 `{{.foo}}` 中的 `foo` 对应）。
    pub key: String,

    /// 替换值——多值对应多次 substitution。
    pub values: Vec<String>,
}
