-- Store private keys separately for potential encryption in the future
CREATE TABLE IF NOT EXISTS election_keys (
    election_id TEXT PRIMARY KEY NOT NULL REFERENCES elections(id),
    rsa_priv_key TEXT NOT NULL,           -- DER base64
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
