CREATE TABLE servers (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE, -- TODO: Improve user deletion handling
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE TRIGGER update_timestamp_servers BEFORE UPDATE
ON servers FOR EACH ROW EXECUTE PROCEDURE
set_updated_at_column();
