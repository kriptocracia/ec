use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Stored ballot representation: ordered list of candidate IDs.
pub type Ballot = Vec<u8>;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Election {
    pub id: String,
    pub name: String,
    pub start_time: i64,
    pub end_time: i64,
    pub status: String,
    pub rules_id: String,
    pub rsa_pub_key: String,
    pub created_at: i64,
    pub results_published: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Candidate {
    pub id: u8,
    pub election_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RegistrationToken {
    pub token: String,
    pub election_id: String,
    pub used: i64,
    pub voter_pubkey: Option<String>,
    pub created_at: i64,
    pub used_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuthorizedVoter {
    pub voter_pubkey: String,
    pub election_id: String,
    pub registered_at: i64,
    pub token_issued: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UsedNonce {
    pub h_n: String,
    pub election_id: String,
    pub recorded_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Vote {
    pub id: i64,
    pub election_id: String,
    pub candidate_ids: String,
    pub recorded_at: i64,
}
