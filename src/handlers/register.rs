use nostr_sdk::prelude::PublicKey;
use sqlx::SqlitePool;

use crate::nostr::messages::OutboundMessage;

/// Handle a voter registration request.
///
/// Flow:
/// 1. Validate the election exists and accepts registrations (open or in_progress).
/// 2. Atomically consume the registration token (fails if already used or invalid).
/// 3. Authorize the voter's pubkey for this election.
///
/// The token consumption and voter authorization happen in a single transaction
/// to prevent race conditions.
pub async fn handle(
    pool: &SqlitePool,
    sender: &PublicKey,
    election_id: &str,
    registration_token: &str,
) -> OutboundMessage {
    match handle_inner(pool, sender, election_id, registration_token).await {
        Ok(()) => OutboundMessage::ok("register-confirmed"),
        Err(e) => {
            let msg = e.to_string();
            // Extract error code from the message format "CODE: description"
            if let Some((code, description)) = msg.split_once(": ") {
                OutboundMessage::error(error_code(code), description)
            } else {
                tracing::error!(error = %e, "Unexpected error in register handler");
                OutboundMessage::error("INTERNAL_ERROR", "An unexpected error occurred")
            }
        }
    }
}

async fn handle_inner(
    pool: &SqlitePool,
    sender: &PublicKey,
    election_id: &str,
    registration_token: &str,
) -> anyhow::Result<()> {
    // 1. Validate election exists and is accepting registrations
    let election = crate::db::get_election(pool, election_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("ELECTION_NOT_FOUND: Election does not exist"))?;

    if election.status != "open" && election.status != "in_progress" {
        anyhow::bail!("ELECTION_CLOSED: Election is not accepting registrations");
    }

    // 2+3. Consume token and authorize voter in a single transaction
    let voter_hex = sender.to_hex();
    let mut tx = pool.begin().await?;

    let rows =
        crate::db::consume_registration_token(&mut tx, registration_token, &voter_hex).await?;

    if rows == 0 {
        // Token was not consumed — either invalid or already used
        anyhow::bail!("INVALID_TOKEN: Registration token is invalid or already used");
    }

    crate::db::authorize_voter(&mut tx, election_id, &voter_hex).await?;

    tx.commit().await?;

    tracing::info!(
        election_id = %election_id,
        "Voter registered successfully"
    );

    Ok(())
}

/// Map known error codes to static string references.
/// This avoids lifetime issues with dynamically extracted codes.
fn error_code(code: &str) -> &'static str {
    match code {
        "ELECTION_NOT_FOUND" => "ELECTION_NOT_FOUND",
        "ELECTION_CLOSED" => "ELECTION_CLOSED",
        "INVALID_TOKEN" => "INVALID_TOKEN",
        _ => "INTERNAL_ERROR",
    }
}
