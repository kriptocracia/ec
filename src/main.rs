use anyhow::Result;
use nostr_sdk::prelude::{Client, Keys};
use secrecy::ExposeSecret;
use sqlx::sqlite::SqlitePoolOptions;
use tracing_subscriber::EnvFilter;

use ec::config::Config;
use ec::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config = Config::load()?;

    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.db_path)
        .await?;

    // Run pending migrations
    sqlx::migrate!("./migrations").run(&db).await?;

    // Initialize Nostr keys and client from the configured secret key.
    let nostr_sk = config.nostr_private_key.expose_secret();
    let keys = Keys::parse(nostr_sk)?;
    let nostr_client = Client::builder().signer(keys.clone()).build();
    let ec_nostr_keys = keys;

    let _state = AppState {
        db,
        nostr_client,
        ec_nostr_keys,
        config,
    };

    tracing::info!("EC daemon initialized (Phase 1 foundation)");

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}
