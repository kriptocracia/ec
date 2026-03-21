use anyhow::Result;
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::types::{
    AuthorizedVoter, Candidate, Election, RegistrationToken, UsedNonce, Vote,
};

pub async fn create_election(pool: &SqlitePool, election: &Election, rsa_priv_key: &str) -> Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO elections (id, name, start_time, end_time, status, rules_id, rsa_pub_key, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
    )
    .bind(&election.id)
    .bind(&election.name)
    .bind(election.start_time)
    .bind(election.end_time)
    .bind(&election.status)
    .bind(&election.rules_id)
    .bind(&election.rsa_pub_key)
    .bind(election.created_at)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO election_keys (election_id, rsa_priv_key)
        VALUES (?1, ?2)
        "#,
    )
    .bind(&election.id)
    .bind(rsa_priv_key)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

pub async fn get_election_key(pool: &SqlitePool, election_id: &str) -> Result<Option<String>> {
    let key: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT rsa_priv_key
        FROM election_keys
        WHERE election_id = ?1
        "#,
    )
    .bind(election_id)
    .fetch_optional(pool)
    .await?;

    Ok(key.map(|(k,)| k))
}

pub async fn get_election(pool: &SqlitePool, id: &str) -> Result<Option<Election>> {
    let election = sqlx::query_as::<_, Election>(
        r#"
        SELECT id, name, start_time, end_time, status, rules_id, rsa_pub_key, created_at
        FROM elections
        WHERE id = ?1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(election)
}

pub async fn list_elections(pool: &SqlitePool) -> Result<Vec<Election>> {
    let elections = sqlx::query_as::<_, Election>(
        r#"
        SELECT id, name, start_time, end_time, status, rules_id, rsa_pub_key, created_at
        FROM elections
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(elections)
}

pub async fn add_candidate(pool: &SqlitePool, candidate: &Candidate) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO candidates (id, election_id, name)
        VALUES (?1, ?2, ?3)
        "#,
    )
    .bind(candidate.id)
    .bind(&candidate.election_id)
    .bind(&candidate.name)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_candidates_for_election(
    pool: &SqlitePool,
    election_id: &str,
) -> Result<Vec<Candidate>> {
    let candidates = sqlx::query_as::<_, Candidate>(
        r#"
        SELECT id, election_id, name
        FROM candidates
        WHERE election_id = ?1
        ORDER BY id ASC
        "#,
    )
    .bind(election_id)
    .fetch_all(pool)
    .await?;

    Ok(candidates)
}

pub async fn insert_registration_tokens(
    tx: &mut Transaction<'_, Sqlite>,
    election_id: &str,
    tokens: &[String],
) -> Result<()> {
    for token in tokens {
        sqlx::query(
            r#"
            INSERT INTO registration_tokens (token, election_id)
            VALUES (?1, ?2)
            "#,
        )
        .bind(token)
        .bind(election_id)
        .execute(tx.as_mut())
        .await?;
    }

    Ok(())
}

/// Atomically consume a registration token, marking it as used by the given voter.
pub async fn consume_registration_token(
    tx: &mut Transaction<'_, Sqlite>,
    token: &str,
    voter_pubkey: &str,
) -> Result<u64> {
    let result = sqlx::query(
        r#"
        UPDATE registration_tokens
        SET used = 1, voter_pubkey = ?1, used_at = unixepoch()
        WHERE token = ?2 AND used = 0
        "#,
    )
    .bind(voter_pubkey)
    .bind(token)
    .execute(tx.as_mut())
    .await?;

    Ok(result.rows_affected())
}

pub async fn authorize_voter(
    tx: &mut Transaction<'_, Sqlite>,
    election_id: &str,
    voter_pubkey: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO authorized_voters (voter_pubkey, election_id)
        VALUES (?1, ?2)
        "#,
    )
    .bind(voter_pubkey)
    .bind(election_id)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

pub async fn get_authorized_voter(
    pool: &SqlitePool,
    election_id: &str,
    voter_pubkey: &str,
) -> Result<Option<AuthorizedVoter>> {
    let voter = sqlx::query_as::<_, AuthorizedVoter>(
        r#"
        SELECT voter_pubkey, election_id, registered_at, token_issued
        FROM authorized_voters
        WHERE election_id = ?1 AND voter_pubkey = ?2
        "#,
    )
    .bind(election_id)
    .bind(voter_pubkey)
    .fetch_optional(pool)
    .await?;

    Ok(voter)
}

pub async fn mark_token_issued(
    tx: &mut Transaction<'_, Sqlite>,
    election_id: &str,
    voter_pubkey: &str,
) -> Result<u64> {
    let result = sqlx::query(
        r#"
        UPDATE authorized_voters
        SET token_issued = 1
        WHERE election_id = ?1 AND voter_pubkey = ?2 AND token_issued = 0
        "#,
    )
    .bind(election_id)
    .bind(voter_pubkey)
    .execute(tx.as_mut())
    .await?;

    Ok(result.rows_affected())
}

pub async fn is_nonce_used(
    pool: &SqlitePool,
    election_id: &str,
    h_n: &str,
) -> Result<bool> {
    let existing: Option<UsedNonce> = sqlx::query_as(
        r#"
        SELECT h_n, election_id, recorded_at
        FROM used_nonces
        WHERE election_id = ?1 AND h_n = ?2
        "#,
    )
    .bind(election_id)
    .bind(h_n)
    .fetch_optional(pool)
    .await?;

    Ok(existing.is_some())
}

pub async fn mark_nonce_used(
    pool: &SqlitePool,
    election_id: &str,
    h_n: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO used_nonces (h_n, election_id)
        VALUES (?1, ?2)
        "#,
    )
    .bind(h_n)
    .bind(election_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_vote(pool: &SqlitePool, vote: &Vote) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO votes (election_id, candidate_ids, recorded_at)
        VALUES (?1, ?2, ?3)
        "#,
    )
    .bind(&vote.election_id)
    .bind(&vote.candidate_ids)
    .bind(vote.recorded_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_votes_for_election(
    pool: &SqlitePool,
    election_id: &str,
) -> Result<Vec<Vote>> {
    let votes = sqlx::query_as::<_, Vote>(
        r#"
        SELECT id, election_id, candidate_ids, recorded_at
        FROM votes
        WHERE election_id = ?1
        ORDER BY id ASC
        "#,
    )
    .bind(election_id)
    .fetch_all(pool)
    .await?;

    Ok(votes)
}

