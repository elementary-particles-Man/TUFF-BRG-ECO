use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum Message {
    StreamFragment {
        id: String,
        ts: String,
        payload: StreamFragmentPayload,
    },
    JudgeResult {
        id: String,
        ts: String,
        payload: JudgeResultPayload,
    },
    ControlCommand {
        id: String,
        ts: String,
        payload: ControlCommandPayload,
    },
    Auth {
        id: String,
        ts: String,
        payload: AuthPayload,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamFragmentPayload {
    pub conversation_id: String,
    pub sequence_number: u64,
    pub url: String,
    pub selector: String,
    pub fragment: String,
    pub context: StreamContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamContext {
    pub page_title: String,
    pub locale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeResultPayload {
    pub status: VerificationStatus,
    pub reason: String,
    pub confidence: f32,
    pub claim: String,
    pub evidence_count: u32,
    pub abstract_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
    Smoke,
    GrayBlack,
    GrayMid,
    GrayWhite,
    White,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlCommandPayload {
    pub command: ControlCommand,
    pub trigger: ControlTrigger,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_override: Option<ManualOverrideMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ControlCommand {
    Stop,
    Continue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum ControlTrigger {
    SmokeDetected,
    LowConfidence,
    ManualOverride,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualOverrideMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstract_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthPayload {
    pub token: String,
}
