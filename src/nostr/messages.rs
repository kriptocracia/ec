use serde::{Deserialize, Serialize};

/// Inbound message from a voter to the EC (JSON inside Gift Wrap rumor content).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
pub enum InboundMessage {
    Register {
        election_id: String,
        registration_token: String,
    },
    RequestToken {
        election_id: String,
        blinded_nonce: String,
    },
    CastVote {
        election_id: String,
        candidate_ids: Vec<u8>,
        h_n: String,
        token: String,
    },
}

/// Outbound message from the EC to a voter (JSON inside Gift Wrap rumor content).
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum OutboundMessage {
    Ok(OkResponse),
    Error(ErrorResponse),
}

#[derive(Debug, Clone, Serialize)]
pub struct OkResponse {
    pub status: &'static str,
    pub action: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blind_signature: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub status: &'static str,
    pub code: &'static str,
    pub message: String,
}

impl OutboundMessage {
    pub fn ok(action: &'static str) -> Self {
        Self::Ok(OkResponse {
            status: "ok",
            action,
            blind_signature: None,
        })
    }

    pub fn ok_with_signature(action: &'static str, blind_signature: String) -> Self {
        Self::Ok(OkResponse {
            status: "ok",
            action,
            blind_signature: Some(blind_signature),
        })
    }

    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self::Error(ErrorResponse {
            status: "error",
            code,
            message: message.into(),
        })
    }
}
