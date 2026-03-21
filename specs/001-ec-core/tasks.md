# Tasks: EC Core Daemon

## Phase 1 — Foundation
- [x] Initialize Rust project, Cargo.toml (versions above), `.sqlx/` offline mode
- [x] Copy `rules/plurality.toml` and `rules/stv.toml` into repo
- [x] Write SQLite migrations (`001_initial.sql`) — include `rules_id` in elections, `candidate_ids TEXT` in votes
- [x] Implement `types.rs` structs with serde (include `Ballot = Vec<u8>`)
- [x] Implement `config.rs` — hybrid config system
- [x] Add `ec.toml` example file to repo root (non-secret defaults)
- [x] Add `.env.example` to repo root (secret vars template, never commit `.env`)
- [x] Implement `state.rs` AppState + SharedState
- [x] Implement `db.rs` with all query functions
- [x] Write `main.rs` startup (DB connect + migrations, Nostr client init, AppState, tracing)
- [x] **Verify `cargo build` passes clean**

## Phase 2 — Rules & Counting Engine
- [x] Implement `rules/types.rs`: `ElectionRules` and all sub-structs (serde + toml)
- [x] Implement `rules/mod.rs`: `load_rules(rules_id, rules_dir) -> Result<ElectionRules>`
- [x] Implement `counting/mod.rs`: `CountingAlgorithm` trait, `CountResult`, `algorithm_for()`
- [x] Implement `counting/plurality.rs`: `PluralityAlgorithm` — count single-choice ballots
- [x] Implement `counting/stv.rs`: `StvAlgorithm` — weighted inclusive Gregory, Droop quota
- [x] Write `tests/counting_plurality_test.rs`: 5 ballots, verify winner
- [x] Write `tests/counting_stv_test.rs`: 10 ranked ballots, 2 seats, verify elected

## Phase 3 — Cryptography
- [x] Implement `crypto.rs`: `generate_keypair()`, `blind_sign()`, `verify_signature()`
- [x] Nonce is `[u8; 32]` (rand 0.10), NOT BigUint
- [x] Write crypto roundtrip integration test

## Phase 4 — Nostr
- [x] Implement `nostr/publisher.rs`: `publish_election_event()`, `publish_result_event()`
- [x] Implement `nostr/listener.rs`: Gift Wrap subscription, message dispatch
- [x] Implement `nostr/messages.rs`: inbound/outbound types (`candidate_ids` as array)

## Phase 5 — Message Handlers
- [ ] Implement `handlers/register.rs` (atomic token consumption)
- [ ] Implement `handlers/request_token.rs` (blind sign + remove from authorized)
- [ ] Implement `handlers/cast_vote.rs` (validate ballot vs rules + verify token + nonce + record)
- [ ] Write `tests/ballot_validation_test.rs`
- [ ] Write integration tests for all 3 handlers

## Phase 6 — gRPC Admin API
- [ ] Write `proto/admin.proto` (`AddElection` includes `rules_id`)
- [ ] Configure `build.rs` for tonic-build
- [ ] Implement `grpc/admin.rs`: all service methods
- [ ] Implement `GenerateRegistrationTokens`

## Phase 7 — Scheduler
- [ ] Implement `scheduler.rs`: 30s tick, status transitions
- [ ] On `Finished`: load rules → `algorithm_for(rules_id)?.count()` → publish result

## Phase 8 — Polish
- [ ] `tracing` instrumentation throughout
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` clean
- [ ] README
- [ ] Docker Compose (ec + nostr relay)
