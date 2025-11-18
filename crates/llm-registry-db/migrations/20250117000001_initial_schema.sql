-- Initial schema for LLM Registry PostgreSQL database
-- Migration: 20250117000001_initial_schema

-- Enable necessary extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Assets table: Core registry for all assets (models, pipelines, datasets, etc.)
CREATE TABLE assets (
    -- Primary identifier (ULID format as VARCHAR)
    id VARCHAR(26) PRIMARY KEY,

    -- Asset identification
    name VARCHAR(255) NOT NULL,
    version VARCHAR(50) NOT NULL,
    asset_type VARCHAR(50) NOT NULL,

    -- Status tracking
    status VARCHAR(50) NOT NULL DEFAULT 'active',

    -- Storage information
    storage_backend VARCHAR(50) NOT NULL,
    storage_uri TEXT NOT NULL,
    storage_path TEXT,
    size_bytes BIGINT,

    -- Integrity verification
    checksum_algorithm VARCHAR(50) NOT NULL,
    checksum_value VARCHAR(128) NOT NULL,

    -- Digital signature (optional)
    signature_algorithm VARCHAR(50),
    signature_value TEXT,
    signature_key_id VARCHAR(255),

    -- Metadata
    description TEXT,
    license VARCHAR(100),
    content_type VARCHAR(100),

    -- Provenance tracking
    author VARCHAR(255),
    source_repo TEXT,
    commit_hash VARCHAR(64),
    build_id VARCHAR(255),

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deprecated_at TIMESTAMPTZ,

    -- Flexible metadata storage (annotations)
    metadata JSONB DEFAULT '{}'::jsonb,

    -- Constraints
    UNIQUE(name, version),
    CHECK (name != ''),
    CHECK (version != ''),
    CHECK (size_bytes IS NULL OR size_bytes >= 0)
);

-- Indexes for efficient queries
CREATE INDEX idx_assets_name ON assets(name);
CREATE INDEX idx_assets_name_version ON assets(name, version);
CREATE INDEX idx_assets_type ON assets(asset_type);
CREATE INDEX idx_assets_status ON assets(status);
CREATE INDEX idx_assets_deprecated ON assets(deprecated_at) WHERE deprecated_at IS NULL;
CREATE INDEX idx_assets_created_at ON assets(created_at DESC);
CREATE INDEX idx_assets_updated_at ON assets(updated_at DESC);
CREATE INDEX idx_assets_metadata ON assets USING GIN(metadata);
CREATE INDEX idx_assets_author ON assets(author);
CREATE INDEX idx_assets_storage_backend ON assets(storage_backend);

-- Asset tags table: Many-to-many relationship for tags
CREATE TABLE asset_tags (
    asset_id VARCHAR(26) NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    tag VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY(asset_id, tag),
    CHECK (tag != '')
);

-- Index for tag-based queries
CREATE INDEX idx_asset_tags_tag ON asset_tags(tag);
CREATE INDEX idx_asset_tags_asset_id ON asset_tags(asset_id);

-- Asset dependencies table: Tracks dependencies between assets
CREATE TABLE asset_dependencies (
    asset_id VARCHAR(26) NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    dependency_id VARCHAR(26) NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    dependency_type VARCHAR(50) NOT NULL DEFAULT 'runtime',
    version_constraint VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY(asset_id, dependency_id),
    -- Prevent self-referencing dependencies
    CHECK (asset_id != dependency_id)
);

-- Indexes for dependency graph traversal
CREATE INDEX idx_asset_dependencies_asset ON asset_dependencies(asset_id);
CREATE INDEX idx_asset_dependencies_dependency ON asset_dependencies(dependency_id);
CREATE INDEX idx_asset_dependencies_type ON asset_dependencies(dependency_type);

-- Registry events table: Event sourcing and audit log
CREATE TABLE registry_events (
    -- Event identifier (ULID format)
    event_id VARCHAR(26) PRIMARY KEY,

    -- Event classification
    event_type VARCHAR(50) NOT NULL,

    -- Related asset (nullable for system-level events)
    asset_id VARCHAR(26),

    -- Event timing
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Actor information
    actor VARCHAR(255) NOT NULL,

    -- Event payload (JSON)
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,

    -- Additional metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    -- Constraints
    CHECK (event_type != ''),
    CHECK (actor != '')
);

-- Indexes for event queries
CREATE INDEX idx_registry_events_asset ON registry_events(asset_id);
CREATE INDEX idx_registry_events_timestamp ON registry_events(timestamp DESC);
CREATE INDEX idx_registry_events_type ON registry_events(event_type);
CREATE INDEX idx_registry_events_actor ON registry_events(actor);
CREATE INDEX idx_registry_events_payload ON registry_events USING GIN(payload);

-- Function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to update updated_at on assets table
CREATE TRIGGER update_assets_updated_at
    BEFORE UPDATE ON assets
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Create a view for active (non-deprecated) assets
CREATE VIEW active_assets AS
SELECT * FROM assets
WHERE deprecated_at IS NULL;

-- Create a view for asset statistics
CREATE VIEW asset_statistics AS
SELECT
    asset_type,
    COUNT(*) as total_count,
    COUNT(CASE WHEN deprecated_at IS NULL THEN 1 END) as active_count,
    COUNT(CASE WHEN deprecated_at IS NOT NULL THEN 1 END) as deprecated_count,
    SUM(size_bytes) as total_size_bytes,
    MIN(created_at) as first_created,
    MAX(created_at) as last_created
FROM assets
GROUP BY asset_type;

-- Comments for documentation
COMMENT ON TABLE assets IS 'Core registry table for all assets (models, pipelines, datasets, policies, etc.)';
COMMENT ON TABLE asset_tags IS 'Tags associated with assets for categorization and search';
COMMENT ON TABLE asset_dependencies IS 'Dependency relationships between assets';
COMMENT ON TABLE registry_events IS 'Event sourcing and audit log for all registry operations';

COMMENT ON COLUMN assets.id IS 'ULID identifier for the asset';
COMMENT ON COLUMN assets.metadata IS 'Flexible key-value metadata stored as JSONB';
COMMENT ON COLUMN assets.status IS 'Current lifecycle status: active, deprecated, archived, deleted';
COMMENT ON COLUMN registry_events.payload IS 'Event-specific data stored as JSONB';
