CREATE TABLE invitations (
    id UUID PRIMARY KEY,
    owner_id UUID REFERENCES users(id),
    invitee_email VARCHAR(255),
    token VARCHAR(64) UNIQUE NOT NULL,
    granted_paths JSON NOT NULL,
    status VARCHAR(20) NOT NULL,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE file_permissions (
    id UUID PRIMARY KEY,
    owner_id UUID REFERENCES users(id),
    client_id UUID REFERENCES users(id),
    path TEXT NOT NULL,
    access TEXT NOT NULL,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ
);

CREATE INDEX idx_file_permissions_client_revoked ON file_permissions (client_id, revoked_at);
CREATE INDEX idx_file_permissions_owner_client ON file_permissions (owner_id, client_id);
