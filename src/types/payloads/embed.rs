use serde::{Deserialize, Serialize};

/// Embed 卡片——比 ark 简单的纯展示型。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EmbedPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// 通知栏弹窗提示。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<EmbedThumbnail>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<EmbedField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbedThumbnail {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbedField {
    pub name: String,
}
