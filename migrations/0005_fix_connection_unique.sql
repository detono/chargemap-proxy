-- migrations/0005_fix_connection_unique.sql

DROP INDEX idx_connections_ocm_id;
CREATE UNIQUE INDEX idx_connections_ocm_id ON connections (ocm_connection_id);