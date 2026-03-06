-- migrations/0001_initial.sql

CREATE TABLE IF NOT EXISTS stations (
    -- OCM identifiers
    id                      INTEGER PRIMARY KEY,  -- OCM ID (e.g. 478964)
    uuid                    TEXT NOT NULL UNIQUE,

    -- Operator / usage
    operator_id             INTEGER,
    operator_title          TEXT,
    usage_type_id           INTEGER,
    usage_type_title        TEXT,
    usage_cost              TEXT,                 -- free text e.g. "0,55 EUR/kWh"

    -- Status
    status_type_id          INTEGER,
    is_operational          INTEGER,              -- SQLite bool (0/1)

    -- Location
    address_title           TEXT,
    address_line1           TEXT,
    town                    TEXT,
    state_or_province       TEXT,
    postcode                TEXT,
    country_iso             TEXT,
    latitude                REAL NOT NULL,
    longitude               REAL NOT NULL,
    access_comments         TEXT,
    related_url             TEXT,
    contact_telephone       TEXT,

    -- Meta
    number_of_points        INTEGER,
    general_comments        TEXT,
    is_recently_verified    INTEGER,
    date_last_verified      TEXT,
    date_last_status_update TEXT,
    date_created            TEXT,

    -- Full raw JSON blob (so we never lose data / easy to re-parse)
    raw_json                TEXT NOT NULL,

    -- Cache bookkeeping
    cached_at               TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS connections (
    id                  INTEGER PRIMARY KEY,   -- OCM connection ID
    station_id          INTEGER NOT NULL REFERENCES stations(id) ON DELETE CASCADE,

    connection_type_id  INTEGER,
    connection_type     TEXT,                  -- e.g. "Type 2 (Socket Only)"
    formal_name         TEXT,                  -- e.g. "IEC 62196-2 Type 2"

    level_id            INTEGER,
    level_title         TEXT,
    is_fast_charge      INTEGER,               -- bool

    current_type_id     INTEGER,
    current_type        TEXT,                  -- e.g. "AC (Three-Phase)"

    amps                REAL,
    voltage             REAL,
    power_kw            REAL,
    quantity            INTEGER,

    status_type_id      INTEGER,
    is_operational      INTEGER,

    comments            TEXT
);

CREATE INDEX IF NOT EXISTS idx_stations_location
    ON stations (latitude, longitude);

CREATE INDEX IF NOT EXISTS idx_stations_country
    ON stations (country_iso);

CREATE INDEX IF NOT EXISTS idx_connections_station
    ON connections (station_id);