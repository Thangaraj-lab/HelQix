-- Who started each run, so a signed-in user can list their own runs (GET /v1/runs). A run id is a hash of the request, so two users who make the same
-- request get the same run: each of them is recorded as an owner of it. A run made from a project is also visible to that project's owner.
-- Runs from before this migration have no owner and are visible to the service key only (and to a project's owner when linked to one).
CREATE TABLE IF NOT EXISTS helqix_run_owners (
    run_id TEXT NOT NULL CHECK (char_length(run_id) BETWEEN 1 AND 80),
    owner TEXT NOT NULL CHECK (char_length(owner) BETWEEN 1 AND 128),
    PRIMARY KEY (run_id, owner)
);
CREATE INDEX IF NOT EXISTS helqix_run_owners_owner_idx ON helqix_run_owners (owner);
