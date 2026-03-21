use std::sync::Arc;

use nostr_sdk::prelude::{Client, Keys};
use sqlx::SqlitePool;

use crate::config::Config;

pub struct AppState {
    pub db: SqlitePool,
    pub nostr_client: Client,
    pub ec_nostr_keys: Keys,
    pub config: Config,
}

pub type SharedState = Arc<AppState>;
