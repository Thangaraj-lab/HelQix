CREATE TABLE IF NOT EXISTS helqix_resources (
    resource_type TEXT NOT NULL CHECK (resource_type IN (
        'realities', 'projects', 'branches', 'executions',
        'evidence', 'verification', 'searches'
    )),
    id UUID NOT NULL,
    project_id UUID,
    payload JSONB NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    version BIGINT NOT NULL DEFAULT 1 CHECK (version > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (resource_type, id)
);

CREATE INDEX IF NOT EXISTS helqix_resources_list_idx
    ON helqix_resources (resource_type, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS helqix_resources_payload_gin_idx
    ON helqix_resources USING GIN (payload jsonb_path_ops);

CREATE INDEX IF NOT EXISTS helqix_resources_project_idx
    ON helqix_resources (project_id, resource_type, created_at DESC, id DESC)
    WHERE project_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS helqix_idempotency_keys (
    resource_type TEXT NOT NULL CHECK (resource_type IN (
        'realities', 'projects', 'branches', 'executions',
        'evidence', 'verification', 'searches'
    )),
    idempotency_key TEXT NOT NULL CHECK (char_length(idempotency_key) BETWEEN 1 AND 200),
    fingerprint JSONB NOT NULL CHECK (jsonb_typeof(fingerprint) = 'object'),
    resource_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT (now() + interval '24 hours'),
    PRIMARY KEY (resource_type, idempotency_key)
);

CREATE INDEX IF NOT EXISTS helqix_idempotency_expiry_idx
    ON helqix_idempotency_keys (expires_at);
