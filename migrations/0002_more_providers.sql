-- migrations/0002_more_providers.sql

-- SQLite can't ALTER column constraints, so we rename + recreate stations and connections

-- 0. FK Checks ON
PRAGMA foreign_keys = ON;

-- 1. Rename existing tables
ALTER TABLE connections RENAME TO connections_old;
ALTER TABLE stations RENAME TO stations_old;

-- 2. New stations table — generic PK, OCM fields nullable
CREATE TABLE stations (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,

    -- OCM-specific (nullable for OSM/Flanders rows)
    ocm_id                  INTEGER UNIQUE,
    uuid                    TEXT UNIQUE,
    operator_id             INTEGER,
    usage_type_id           INTEGER,
    status_type_id          INTEGER,
    is_recently_verified    INTEGER,
    date_last_verified      TEXT,
    date_last_status_update TEXT,
    date_created            TEXT,
    general_comments        TEXT,
    related_url             TEXT,
    contact_telephone       TEXT,
    access_comments         TEXT,
    state_or_province       TEXT,
    country_iso             TEXT,

    -- Common fields (all sources)
    operator_title          TEXT,
    usage_type_title        TEXT,
    usage_cost              TEXT,
    is_operational          INTEGER,
    address_title           TEXT,
    address_line1           TEXT,
    town                    TEXT,
    postcode                TEXT,
    latitude                REAL NOT NULL,
    longitude               REAL NOT NULL,
    number_of_points        INTEGER,

    -- Source tracking
    primary_source          TEXT NOT NULL DEFAULT 'ocm',  -- 'ocm', 'osm', 'flanders'

    cached_at               TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 3. New connections table — auto PK, OCM id nullable
CREATE TABLE connections (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    ocm_connection_id   INTEGER,               -- nullable, OCM only
    station_id          INTEGER NOT NULL REFERENCES stations(id) ON DELETE CASCADE,

    connection_type_id  INTEGER,
    connection_type     TEXT,
    formal_name         TEXT,
    level_id            INTEGER,
    level_title         TEXT,
    is_fast_charge      INTEGER,
    current_type_id     INTEGER,
    current_type        TEXT,
    amps                REAL,
    voltage             REAL,
    power_kw            REAL,
    quantity            INTEGER,
    status_type_id      INTEGER,
    is_operational      INTEGER,
    comments            TEXT
);

-- 4. Migrate existing data
INSERT INTO stations (
    ocm_id, uuid, operator_id, usage_type_id, status_type_id,
    is_recently_verified, date_last_verified, date_last_status_update,
    date_created, general_comments, related_url, contact_telephone,
    access_comments, state_or_province, country_iso,
    operator_title, usage_type_title, usage_cost, is_operational,
    address_title, address_line1, town, postcode,
    latitude, longitude, number_of_points, primary_source, cached_at
)
SELECT
    id, uuid, operator_id, usage_type_id, status_type_id,
    is_recently_verified, date_last_verified, date_last_status_update,
    date_created, general_comments, related_url, contact_telephone,
    access_comments, state_or_province, country_iso,
    operator_title, usage_type_title, usage_cost, is_operational,
    address_title, address_line1, town, postcode,
    latitude, longitude, number_of_points, 'ocm', cached_at
FROM stations_old;

INSERT INTO connections (
    ocm_connection_id, station_id, connection_type_id, connection_type,
    formal_name, level_id, level_title, is_fast_charge, current_type_id,
    current_type, amps, voltage, power_kw, quantity,
    status_type_id, is_operational, comments
)
SELECT
    c.id, s.id, c.connection_type_id, c.connection_type,
    c.formal_name, c.level_id, c.level_title, c.is_fast_charge, c.current_type_id,
    c.current_type, c.amps, c.voltage, c.power_kw, c.quantity,
    c.status_type_id, c.is_operational, c.comments
FROM connections_old c
JOIN stations s ON s.ocm_id = c.station_id;

-- 5. Source tracking tables
CREATE TABLE sync_state (
    source         TEXT PRIMARY KEY,  -- 'ocm', 'osm', 'flanders'
    last_synced_at TEXT NOT NULL
);

CREATE TABLE station_sources (
    station_id  INTEGER NOT NULL REFERENCES stations(id) ON DELETE CASCADE,
    source      TEXT NOT NULL,
    source_id   TEXT NOT NULL,
    raw_json    TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen   TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (source, source_id)
);

-- Backfill station_sources for existing OCM rows
INSERT INTO station_sources (station_id, source, source_id)
SELECT id, 'ocm', CAST(ocm_id AS TEXT)
FROM stations
WHERE ocm_id IS NOT NULL;

-- 6. Recreate indexes
CREATE INDEX IF NOT EXISTS idx_stations_location ON stations (latitude, longitude);
CREATE INDEX IF NOT EXISTS idx_stations_country  ON stations (country_iso);
CREATE INDEX IF NOT EXISTS idx_connections_station ON connections (station_id);
CREATE INDEX IF NOT EXISTS idx_station_sources_station_id ON station_sources (station_id);

-- 7. Drop old tables
DROP TABLE connections_old;
DROP TABLE stations_old;