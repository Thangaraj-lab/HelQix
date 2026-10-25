-- K2: staged model builds. The op log is the source of truth for a build; events are the
-- append-only record the UI streams; a commit creates an immutable model revision.
CREATE TABLE IF NOT EXISTS helqix_builds (
    id UUID PRIMARY KEY,
    project_id UUID NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('staged', 'committed')),
    op_cursor INTEGER NOT NULL DEFAULT 0 CHECK (op_cursor >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS helqix_builds_project_idx ON helqix_builds (project_id, created_at DESC);

CREATE TABLE IF NOT EXISTS helqix_build_ops (
    build_id UUID NOT NULL REFERENCES helqix_builds (id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    op JSONB NOT NULL CHECK (jsonb_typeof(op) = 'object'),
    PRIMARY KEY (build_id, position)
);

CREATE TABLE IF NOT EXISTS helqix_build_events (
    build_id UUID NOT NULL REFERENCES helqix_builds (id) ON DELETE CASCADE,
    seq BIGINT NOT NULL CHECK (seq > 0),
    event JSONB NOT NULL CHECK (jsonb_typeof(event) = 'object'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (build_id, seq)
);

CREATE TABLE IF NOT EXISTS helqix_model_revisions (
    project_id UUID NOT NULL,
    number INTEGER NOT NULL CHECK (number > 0),
    build_id UUID NOT NULL UNIQUE REFERENCES helqix_builds (id),
    op_count BIGINT NOT NULL,
    model_sha256 TEXT NOT NULL CHECK (char_length(model_sha256) = 64),
    committed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, number)
);

-- Connected component/material sources. Empty until a source is connected.
CREATE TABLE IF NOT EXISTS helqix_sources (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    as_of TEXT,
    example BOOLEAN NOT NULL DEFAULT false
);
