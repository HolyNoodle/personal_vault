CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    acting_as_owner_id TEXT REFERENCES users(id),
    active_role TEXT NOT NULL,
    app_id TEXT NOT NULL,
    display_number INTEGER,
    state TEXT NOT NULL DEFAULT 'initializing',
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    terminated_at TEXT
);

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_state ON sessions(state);
