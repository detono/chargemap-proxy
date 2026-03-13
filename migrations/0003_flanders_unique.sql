-- migrations/0003_flanders_station_unique.sql

CREATE UNIQUE INDEX IF NOT EXISTS idx_stations_flanders_dedup
    ON stations (address_line1, postcode, operator_title, latitude, longitude);