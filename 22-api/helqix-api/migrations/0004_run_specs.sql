-- K4: what each stored run was made from. The frames stay in the frame store; this is the metadata the engine needs
-- to freeze and branch a run, and the index that lists a project's runs. A run id is a hash of its request, so a
-- repeated request is the same row (the insert does nothing).
CREATE TABLE IF NOT EXISTS helqix_run_specs (
    run_id TEXT PRIMARY KEY CHECK (char_length(run_id) BETWEEN 1 AND 80),
    project_id UUID,
    revision INTEGER CHECK (revision IS NULL OR revision >= 1),
    parent_run_id TEXT,
    parent_index INTEGER CHECK (parent_index IS NULL OR parent_index >= 0),
    params JSONB NOT NULL CHECK (jsonb_typeof(params) = 'object'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS helqix_run_specs_project_idx ON helqix_run_specs (project_id, created_at DESC) WHERE project_id IS NOT NULL;
