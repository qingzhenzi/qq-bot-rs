use serde::{Deserialize, Serialize};

use crate::types::keyboard::Keyboard;

/// 键盘载荷——`id` 引用已备案模板，或 `content` 直接 inline 一份，二选一。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct KeyboardPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<Keyboard>,
}

impl KeyboardPayload {
    pub fn template(id: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            content: None,
        }
    }

    pub fn inline(keyboard: Keyboard) -> Self {
        Self {
            id: None,
            content: Some(keyboard),
        }
    }
}
