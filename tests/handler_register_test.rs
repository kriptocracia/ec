use nostr_sdk::prelude::*;
use secrecy::SecretString;
use sqlx::SqlitePool;

use ec::db;
use ec::handlers::register;
use ec::types::Election;

async fn setup_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

fn test_election(status: &str) -> Election {
    Election {
        id: "test-election-1".to_string(),
        name: "Test Election".to_string(),
        start_time: 1000,
        end_time: 2000,
        status: status.to_string(),
        rules_id: "plurality".to_string(),
        rsa_pub_key: "dummy-pk".to_string(),
        created_at: 1000,
        results_published: 0,
    }
}

async fn seed_election(pool: &SqlitePool, status: &str) {
    let election = test_election(status);
    db::create_election(pool, &election, &SecretString::new("dummy-sk".into()))
        .await
        .unwrap();
}

async fn seed_token(pool: &SqlitePool, election_id: &str, token: &str) {
    let mut tx = pool.begin().await.unwrap();
    db::insert_registration_tokens(&mut tx, election_id, &[token.to_string()])
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn register_success() {
    let pool = setup_db().await;
    seed_election(&pool, "open").await;
    seed_token(&pool, "test-election-1", "valid-token-abc").await;

    let keys = Keys::generate();
    let response = register::handle(
        &pool,
        &keys.public_key(),
        "test-election-1",
        "valid-token-abc",
    )
    .await;

    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["action"], "register-confirmed");

    // Verify voter is now authorized
    let voter = db::get_authorized_voter(&pool, "test-election-1", &keys.public_key().to_hex())
        .await
        .unwrap();
    assert!(voter.is_some());
}

#[tokio::test]
async fn register_in_progress_election() {
    let pool = setup_db().await;
    seed_election(&pool, "in_progress").await;
    seed_token(&pool, "test-election-1", "valid-token-abc").await;

    let keys = Keys::generate();
    let response = register::handle(
        &pool,
        &keys.public_key(),
        "test-election-1",
        "valid-token-abc",
    )
    .await;

    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["action"], "register-confirmed");

    // Verify voter is now authorized
    let voter = db::get_authorized_voter(&pool, "test-election-1", &keys.public_key().to_hex())
        .await
        .unwrap();
    assert!(voter.is_some());
}

#[tokio::test]
async fn register_invalid_token() {
    let pool = setup_db().await;
    seed_election(&pool, "open").await;

    let keys = Keys::generate();
    let response =
        register::handle(&pool, &keys.public_key(), "test-election-1", "bad-token").await;

    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["status"], "error");
    assert_eq!(json["code"], "INVALID_TOKEN");
}

#[tokio::test]
async fn register_already_used_token() {
    let pool = setup_db().await;
    seed_election(&pool, "open").await;
    seed_token(&pool, "test-election-1", "valid-token-abc").await;

    let keys1 = Keys::generate();
    register::handle(
        &pool,
        &keys1.public_key(),
        "test-election-1",
        "valid-token-abc",
    )
    .await;

    // Second registration with same token should fail
    let keys2 = Keys::generate();
    let response = register::handle(
        &pool,
        &keys2.public_key(),
        "test-election-1",
        "valid-token-abc",
    )
    .await;

    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["status"], "error");
    assert_eq!(json["code"], "INVALID_TOKEN");
}

#[tokio::test]
async fn register_election_not_found() {
    let pool = setup_db().await;

    let keys = Keys::generate();
    let response = register::handle(&pool, &keys.public_key(), "nonexistent", "some-token").await;

    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["status"], "error");
    assert_eq!(json["code"], "ELECTION_NOT_FOUND");
}

#[tokio::test]
async fn register_election_finished() {
    let pool = setup_db().await;
    seed_election(&pool, "finished").await;
    seed_token(&pool, "test-election-1", "valid-token-abc").await;

    let keys = Keys::generate();
    let response = register::handle(
        &pool,
        &keys.public_key(),
        "test-election-1",
        "valid-token-abc",
    )
    .await;

    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["status"], "error");
    assert_eq!(json["code"], "ELECTION_CLOSED");
}
