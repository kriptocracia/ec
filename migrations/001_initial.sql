-- Initial schema for Criptocracia EC

CREATE TABLE IF NOT EXISTS elections (
    id TEXT PRIMARY KEY NOT NULL,         -- nanoid
    name TEXT NOT NULL,
    start_time INTEGER NOT NULL,          -- unix timestamp
    end_time INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',  -- open | in_progress | finished | cancelled
    rules_id TEXT NOT NULL,               -- references a file in rules/{rules_id}.toml
    rsa_pub_key TEXT NOT NULL,            -- DER base64
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE IF NOT EXISTS candidates (
    id INTEGER NOT NULL,                  -- u8, candidate number within election
    election_id TEXT NOT NULL REFERENCES elections(id),
    name TEXT NOT NULL,
    PRIMARY KEY (id, election_id)
);

CREATE TABLE IF NOT EXISTS registration_tokens (
    token TEXT PRIMARY KEY NOT NULL,      -- random base64url, 32 bytes
    election_id TEXT NOT NULL REFERENCES elections(id),
    used INTEGER NOT NULL DEFAULT 0,      -- 0 = unused, 1 = used
    voter_pubkey TEXT,                    -- hex pubkey, set when token is consumed
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    used_at INTEGER
);

CREATE TABLE IF NOT EXISTS authorized_voters (
    voter_pubkey TEXT NOT NULL,           -- hex nostr pubkey
    election_id TEXT NOT NULL REFERENCES elections(id),
    registered_at INTEGER NOT NULL DEFAULT (unixepoch()),
    token_issued INTEGER NOT NULL DEFAULT 0,  -- 1 = already got blind sig, can't get another
    PRIMARY KEY (voter_pubkey, election_id)
);

CREATE TABLE IF NOT EXISTS used_nonces (
    h_n TEXT NOT NULL,                    -- SHA256(nonce) hex
    election_id TEXT NOT NULL REFERENCES elections(id),
    recorded_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (h_n, election_id)
);

CREATE TABLE IF NOT EXISTS votes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    election_id TEXT NOT NULL REFERENCES elections(id),
    -- JSON array of candidate ids, ordered by voter preference.
    -- Plurality example: [3]
    -- STV example:       [3, 1, 4, 2]
    -- Stored as TEXT to support both single-choice and ranked ballots.
    -- NO voter identity. Ever.
    candidate_ids TEXT NOT NULL,
    recorded_at INTEGER NOT NULL DEFAULT (unixepoch())
);

