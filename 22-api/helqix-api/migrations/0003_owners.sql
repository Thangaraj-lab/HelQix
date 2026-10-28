-- Ownership: who created each resource. Rows created before this migration have no owner and are visible
-- only to the service key. A signed-in user sees only resources they own (and everything hanging off their projects).
CREATE TABLE IF NOT EXISTS helqix_owners (
    resource_id UUID PRIMARY KEY,
    owner TEXT NOT NULL CHECK (char_length(owner) BETWEEN 1 AND 128),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS helqix_owners_owner_idx ON helqix_owners (owner);
