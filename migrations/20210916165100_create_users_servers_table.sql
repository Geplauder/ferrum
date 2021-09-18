CREATE TABLE users_servers (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE(user_id, server_id)
);

CREATE TRIGGER update_timestamp_users_servers BEFORE UPDATE
ON users_servers FOR EACH ROW EXECUTE PROCEDURE
set_updated_at_column();
