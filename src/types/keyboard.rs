//! 内联键盘（按钮）。外层由 [`crate::types::payloads::KeyboardPayload`] 包装。
//!
//! `style` / `type`（动作类别 / 权限类别）服务端时不时加新值——这里直接 `u8`
//! 透传，遇未知值不报错。调用方按官方文档的取值表填即可。

use serde::{Deserialize, Serialize};

/// 一组键盘——多行按钮。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Keyboard {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<KeyboardRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct KeyboardRow {
    /// 同一行最多 5 个（库不强校验）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buttons: Vec<Button>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Button {
    /// 按钮 ID——同一消息内唯一，回调里靠它定位。
    pub id: String,

    pub render_data: RenderData,

    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RenderData {
    pub label: String,

    pub visited_label: String,

    /// 样式：`0` 灰色线框，`1` 蓝色线框，其余按服务端文档。
    pub style: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Action {
    /// `0` 跳转 URL，`1` 回调（触发 INTERACTION_CREATE），`2` 指令。
    #[serde(rename = "type")]
    pub action_type: u8,

    pub permission: Permission,

    /// 跳转时是 URL，回调 / 指令时是自定义 data。
    pub data: String,

    /// 客户端不支持本 action 时的 toast 文案——v2 协议**必填**。
    pub unsupport_tips: String,

    /// 仅指令按钮（`action_type = 2`）：点击后是否引用原消息。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply: Option<bool>,

    /// 仅指令按钮：点击后是否自动发送（仅单聊有效）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enter: Option<bool>,

    /// 仅指令按钮：设为 `1` 弹起手 Q 选图器。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Permission {
    /// `0` 指定用户、`1` 仅管理者、`2` 所有人、`3` 指定身份组（仅频道可用）。
    #[serde(rename = "type")]
    pub permission_type: u8,

    /// `permission_type = 3` 时使用，仅频道可用。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub specify_role_ids: Vec<String>,

    /// `permission_type = 0` 时使用。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub specify_user_ids: Vec<String>,
}
