# Criptocracia EC — Electoral Commission Daemon

> **EXPERIMENTAL SOFTWARE — NOT FOR PRODUCTION USE.**  
> This code has **not** been audited and **must not** be used for real‑world elections or any scenario where safety, legality, or physical risk is at stake.

## Overview

Criptocracia EC (`ec`) is the **Electoral Commission daemon** for the Criptocracia voting system — an experimental, trustless, open-source electronic voting platform.  
It is responsible for managing elections, issuing anonymous voting tokens via blind RSA signatures, receiving votes over Nostr, and publishing verifiable results.

## Status

- **Experimental**: protocol and implementation are still evolving.  
- **No guarantees**: no formal security review, no backward-compatibility promises.  
- **Do not use** for production elections, public institutions, or high-stakes governance.

## Tech Stack

- **Language**: Rust 1.94 (edition 2024)
- **Async runtime**: `tokio` 1.50 (multi-thread)
- **Transport**: Nostr with **NIP-59 Gift Wrap** (`nostr-sdk` 0.44.1)
- **Blind signatures**: `blind-rsa-signatures` 0.17.1
- **Database**: SQLite via `sqlx` 0.8.6
- **Admin API**: gRPC via `tonic` 0.14.5 / `prost` 0.14.3
- **Config / secrets**: `toml`, `dotenvy`, `secrecy`

## Configuration

Configuration is split between a versioned **`ec.toml`** file (non-secrets) and **environment variables** (secrets + overrides).

### 1. Non-secret config (`ec.toml`)

Example `ec.toml` (already included in the repo):

```toml
# ec.toml — operator configuration, safe to version control
# No secrets here. Ever.

relay_url  = "wss://relay.mostro.network"
grpc_bind  = "127.0.0.1:50051"
rules_dir  = "./rules"
log_level  = "info"
db_path    = "./ec.db"
```

These values can be overridden by environment variables:

- `RELAY_URL`
- `GRPC_BIND`
- `RULES_DIR`
- `LOG_LEVEL`
- `DATABASE_URL` (overrides `db_path`)

### 2. Secret config (environment variables only)

Secrets are **never** stored in `ec.toml`. They are loaded only from environment variables and kept in memory as `SecretString`:

- `NOSTR_PRIVATE_KEY` — hex-encoded Nostr private key for the EC identity (**required**)
- `EC_DB_PASSWORD` — optional password for encrypting per-election RSA keys stored in the database (not yet implemented)

For local development, you can use `.env` (loaded by `dotenvy`) from the provided template:

```bash
cp .env.example .env
edit .env           # fill in your secrets
```

`.env` is **gitignored** and must never be committed.

### 3. Config precedence

From highest to lowest priority:

1. Environment variables (`RELAY_URL`, `GRPC_BIND`, `RULES_DIR`, `LOG_LEVEL`, `DATABASE_URL`, secrets)
2. `ec.toml` values
3. Hardcoded defaults in `Config` (if `ec.toml` is missing)

## Running Locally

### 1. Prerequisites

- Rust toolchain compatible with Rust 1.94 (e.g. via `rustup`)  
- OpenSSL CLI available on your PATH (for key generation)  
- SQLite (or let `sqlx` create `ec.db` on first run)

### 2. Configure secrets and non-secrets

1. Edit `ec.toml` if you want to change relay, ports, or DB path.
2. Create `.env` from the example:

```bash
cp .env.example .env
```

Then set at least:

```bash
NOSTR_PRIVATE_KEY=your_hex_private_key_here
# Optional:
# EC_DB_PASSWORD=...
# RELAY_URL=wss://your-relay.example
```

### 3. Run migrations and start the daemon

The EC daemon uses `sqlx` with SQLite and runs migrations at startup.

```bash
cargo build
cargo run
```

By default, this will:

- Load `.env` (if present) and `ec.toml`
- Connect to the SQLite database at `db_path` / `DATABASE_URL`
- Run migrations from `./migrations`
- Initialize Nostr client and EC identity
- Start the scheduler (30s tick: election status transitions + vote counting + result publishing)
- Start the Nostr listener for Gift Wrap voter messages
- Bind the gRPC admin API to `grpc_bind` (default `127.0.0.1:50051`)

## High-Level Architecture

- Single Rust binary with three main surfaces:
  - **Nostr listener/publisher** (NIP-59 Gift Wrap) for voter communication
  - **gRPC admin API** for operators (local, non-voter)
  - **Scheduler** that drives election state transitions and counting
- All persistent state lives in SQLite; counting is pluggable via a trait-based engine.

## Related Projects

- Criptocracia MVP repository (original prototype):  
  `https://github.com/kriptocracia/criptocracia`

Again: **this repository is experimental and unaudited**. Use it only for research, testing, and education.***
