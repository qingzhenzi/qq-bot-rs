use serde::{Deserialize, Serialize};

/// Ark 模板消息——需服务端预先备案模板。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArkPayload {
    pub template_id: i32,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kv: Vec<ArkKv>,
}

/// `value` 与 `obj` 互斥。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ArkKv {
    pub key: String,

    /// 标量值（字符串）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// 数组形态字段值。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub obj: Vec<ArkObj>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ArkObj {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub obj_kv: Vec<ArkObjKv>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArkObjKv {
    pub key: String,

    pub value: String,
}
