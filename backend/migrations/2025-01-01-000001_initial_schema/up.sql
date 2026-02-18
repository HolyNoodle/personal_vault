CREATE TABLE users (
    id TEXT PRIMARY KEY NOT NULL,
    email TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    roles TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'active'
        CHECK(status IN ('active', 'suspended', 'deleted'))
);

CREATE TABLE webauthn_credentials (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id TEXT NOT NULL UNIQUE,
    public_key TEXT NOT NULL,
    sign_count INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
