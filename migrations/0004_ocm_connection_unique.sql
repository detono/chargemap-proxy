-- migrations/0004_ocm_connection_unique.sql

CREATE UNIQUE INDEX IF NOT EXISTS idx_connections_ocm_id
    ON connections (ocm_connection_id)
    WHERE ocm_connection_id IS NOT NULL;