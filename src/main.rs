use std::sync::Arc;

use anyhow::Result;
use nostr_sdk::prelude::{Client, Keys};
use secrecy::ExposeSecret;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use tracing_subscriber::EnvFilter;

use ec::config::Config;
use ec::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config = Config::load()?;

    let options = SqliteConnectOptions::from_str(&config.db_path)?
        .create_if_missing(true);

    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    // Run pending migrations
    sqlx::migrate!("./migrations").run(&db).await?;

    // Initialize Nostr keys and client from the configured secret key.
    let nostr_sk = config.nostr_private_key.expose_secret();
    let keys = Keys::parse(nostr_sk)?;
    let nostr_client = Client::builder().signer(keys.clone()).build();
    nostr_client.add_relay(&config.relay_url).await?;
    nostr_client.connect().await;
    let ec_nostr_keys = keys;

    let state = Arc::new(AppState {
        db,
        nostr_client,
        ec_nostr_keys,
        config,
    });

    tracing::info!("EC daemon starting");

    // Spawn the scheduler (30s tick: status transitions + counting + result publishing).
    let scheduler_handle = tokio::spawn(ec::scheduler::run(
        state.db.clone(),
        state.nostr_client.clone(),
        state.config.rules_dir.clone(),
    ));

    // Spawn the Nostr Gift Wrap listener.
    let listener_handle = tokio::spawn(ec::nostr::listener::listen(state.clone()));

    tracing::info!("EC daemon running");

    // Wait for either task to finish (they run forever under normal operation).
    tokio::select! {
        res = scheduler_handle => {
            match res {
                Ok(()) => {
                    tracing::error!("Scheduler exited unexpectedly");
                    anyhow::bail!("Scheduler exited unexpectedly")
                }
                Err(join_err) => Err(join_err.into()),
            }
        }
        res = listener_handle => {
            match res {
                Ok(Ok(())) => {
                    tracing::error!("Nostr listener exited unexpectedly");
                    anyhow::bail!("Nostr listener exited unexpectedly")
                }
                Ok(Err(e)) => Err(e),
                Err(join_err) => Err(join_err.into()),
            }
        }
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}
