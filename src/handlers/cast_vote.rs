use base64::Engine;
use sqlx::SqlitePool;
use std::path::Path;

use crate::nostr::messages::OutboundMessage;
use crate::rules::{BallotMethod, ElectionRules};
use crate::types::Vote;

/// Handle a cast-vote request from an anonymous voter keypair.
///
/// Flow:
/// 1. Validate the election exists and is in_progress.
/// 2. Verify the blind RSA signature against the election's public key.
/// 3. Load election rules and validate the ballot structure.
/// 4. Atomically mark nonce as used and store the vote (NO voter identity — ever).
pub async fn handle(
    pool: &SqlitePool,
    election_id: &str,
    candidate_ids: &[u8],
    h_n: &str,
    token: &str,
    rules_dir: &Path,
) -> OutboundMessage {
    match handle_inner(pool, election_id, candidate_ids, h_n, token, rules_dir).await {
        Ok(()) => OutboundMessage::ok("vote-recorded"),
        Err(e) => {
            let msg = e.to_string();
            if let Some((code, description)) = msg.split_once(": ") {
                OutboundMessage::error(error_code(code), description)
            } else {
                tracing::error!(error = %e, "Unexpected error in cast-vote handler");
                OutboundMessage::error("INTERNAL_ERROR", "An unexpected error occurred")
            }
        }
    }
}

async fn handle_inner(
    pool: &SqlitePool,
    election_id: &str,
    candidate_ids: &[u8],
    h_n: &str,
    token_b64: &str,
    rules_dir: &Path,
) -> anyhow::Result<()> {
    // 1. Validate election exists and is in voting phase
    let election = crate::db::get_election(pool, election_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("ELECTION_NOT_FOUND: Election does not exist"))?;

    if election.status != "in_progress" {
        anyhow::bail!("ELECTION_CLOSED: Election is not accepting votes");
    }

    // 2. Verify the blind RSA signature
    // The token contains: base64(signature ++ msg_randomizer)
    // where msg_randomizer is the last 32 bytes
    let token_bytes = base64::engine::general_purpose::STANDARD
        .decode(token_b64)
        .map_err(|_| anyhow::anyhow!("INVALID_TOKEN: Malformed token (not valid base64)"))?;

    if token_bytes.len() <= 32 {
        anyhow::bail!("INVALID_TOKEN: Token too short");
    }

    let (signature, msg_randomizer) = token_bytes.split_at(token_bytes.len() - 32);

    // The message being verified is the nonce hash (h_n) as raw bytes
    let h_n_bytes = hex::decode(h_n)
        .map_err(|_| anyhow::anyhow!("INVALID_TOKEN: Malformed nonce hash (not valid hex)"))?;

    crate::crypto::verify_signature(&election.rsa_pub_key, signature, msg_randomizer, &h_n_bytes)
        .map_err(|_| anyhow::anyhow!("INVALID_TOKEN: Signature verification failed"))?;

    // 3. Load rules and validate ballot
    let rules = crate::rules::load_rules(&election.rules_id, rules_dir).map_err(|_| {
        anyhow::anyhow!(
            "UNKNOWN_RULES: Unknown election rules '{}'",
            election.rules_id
        )
    })?;

    let candidates = crate::db::get_candidates_for_election(pool, election_id).await?;
    let valid_ids: Vec<u8> = candidates
        .iter()
        .map(|c| {
            u8::try_from(c.id).map_err(|_| {
                anyhow::anyhow!("INTERNAL_ERROR: Candidate ID {} exceeds u8 range", c.id)
            })
        })
        .collect::<anyhow::Result<_>>()?;

    validate_ballot(candidate_ids, &rules, &valid_ids)?;

    // 4+5. Atomically mark nonce as used and record vote in a single transaction.
    // This prevents both TOCTOU double-voting and orphaned nonces if vote insert fails.
    let candidate_ids_json = serde_json::to_string(candidate_ids)?;
    let vote = Vote {
        id: 0, // auto-increment
        election_id: election_id.to_string(),
        candidate_ids: candidate_ids_json,
        recorded_at: chrono::Utc::now().timestamp(),
    };

    let mut tx = pool.begin().await?;

    let was_new = crate::db::try_mark_nonce_used(&mut tx, election_id, h_n).await?;
    if !was_new {
        anyhow::bail!("NONCE_ALREADY_USED: This voting token has already been used");
    }

    crate::db::insert_vote_tx(&mut tx, &vote).await?;
    tx.commit().await?;

    tracing::info!(
        election_id = %election_id,
        "Vote recorded successfully"
    );

    Ok(())
}

/// Validate a ballot against election rules and valid candidates.
pub fn validate_ballot(
    candidate_ids: &[u8],
    rules: &ElectionRules,
    election_candidates: &[u8],
) -> anyhow::Result<()> {
    if candidate_ids.len() > u8::MAX as usize {
        anyhow::bail!("BALLOT_INVALID: Too many choices");
    }
    let n = candidate_ids.len() as u8;

    if n < rules.ballot.min_choices {
        anyhow::bail!(
            "BALLOT_INVALID: Too few choices ({n}, minimum {})",
            rules.ballot.min_choices
        );
    }

    if rules.ballot.max_choices > 0 && n > rules.ballot.max_choices {
        anyhow::bail!(
            "BALLOT_INVALID: Too many choices ({n}, maximum {})",
            rules.ballot.max_choices
        );
    }

    for &id in candidate_ids {
        if !election_candidates.contains(&id) {
            anyhow::bail!("INVALID_CANDIDATE: Candidate {id} is not in this election");
        }
    }

    if rules.ballot.method == BallotMethod::Ranked {
        let mut seen = std::collections::HashSet::new();
        for &id in candidate_ids {
            if !seen.insert(id) {
                anyhow::bail!("BALLOT_INVALID: Duplicate candidate {id} in ranked ballot");
            }
        }
    }

    Ok(())
}

fn error_code(code: &str) -> &'static str {
    match code {
        "ELECTION_NOT_FOUND" => "ELECTION_NOT_FOUND",
        "ELECTION_CLOSED" => "ELECTION_CLOSED",
        "NONCE_ALREADY_USED" => "NONCE_ALREADY_USED",
        "INVALID_TOKEN" => "INVALID_TOKEN",
        "INVALID_CANDIDATE" => "INVALID_CANDIDATE",
        "BALLOT_INVALID" => "BALLOT_INVALID",
        "UNKNOWN_RULES" => "UNKNOWN_RULES",
        _ => "INTERNAL_ERROR",
    }
}
