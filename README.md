# Chargemap Proxy
[![Docker Version](https://img.shields.io/docker/v/detono/chargemap-proxy?style=flat-square)](https://hub.docker.com/r/detono/chargemap-proxy)
[![Build Status](https://img.shields.io/github/actions/workflow/status/detono/chargemap-proxy/deploy.yml?branch=main&style=flat-square)](https://github.com/detono/chargemap-proxy/actions)
[![Tests](https://img.shields.io/github/actions/workflow/status/detono/chargemap-proxy/deploy.yml?branch=main&label=tests&style=flat-square)](https://github.com/detono/chargemap-proxy/actions)
[![Codecov](https://img.shields.io/codecov/c/github/detono/chargemap-proxy?style=flat-square&logo=codecov)](https://app.codecov.io/gh/detono/chargemap-proxy)
[![Docker Pulls](https://img.shields.io/docker/pulls/detono/chargemap-proxy?style=flat-square)](https://hub.docker.com/r/detono/chargemap-proxy)
[![Image Size](https://img.shields.io/docker/image-size/detono/chargemap-proxy/latest?style=flat-square)](https://hub.docker.com/r/detono/chargemap-proxy)
[![Rust Version](https://img.shields.io/badge/rust-1.94.0-blue.svg?style=flat-square&logo=rust)](https://github.com/detono/chargemap-proxy)
![License](https://img.shields.io/github/license/detono/chargemap-proxy?style=flat-square)
[![Support Tono on Ko-fi](https://img.shields.io/badge/Support_Tono-Tea-BD8C5E?style=flat-square&logo=ko-fi&logoColor=white)](https://ko-fi.com/detono)

A lightweight, blisteringly fast Rust/Axum API that caches EV charging station data from [Open Charge Map](https://openchargemap.org), [OpenStreetMap](https://www.openstreetmap.org), and a local CSV dataset (e.g. [Flanders's official EV charger data](https://www.vlaanderen.be/datavindplaats/catalogus/laadpunten-voor-elektrische-voertuigen)) into a local SQLite database and serves it to your clients.

Built specifically to prevent mobile apps and frontends from hammering upstream APIs and hitting rate limits. A single background process fetches and merges fresh data from all three sources, while all your clients hit the lightning-fast local cache.

## Features

- **Built for Speed:** Written in Rust using Axum and Tokio.
- **Smart Caching:** Local SQLite database ensures instant responses and zero rate-limiting from upstream.
- **Multi-Source:** Merges data from OCM, OSM, and a local CSV — stations missing from one source are covered by another.
- **Deduplication:** Stations within 25m of each other across sources are automatically merged.
- **Incremental OCM Sync:** Uses `modifiedsince` to only fetch changed stations after the first sync.
- **Geospatial Filtering:** Query stations by bounding radius, connector type, and minimum power output.
- **Multi-Arch Support:** Available for `amd64` and `arm64`.

## Quick Start

The easiest way to get the proxy running is via Docker. You will need an Open Charge Map API key, and you'll need to define your own API key to secure your proxy endpoints.

### 1. Prepare your configuration files

Create a `config.toml` file to define your search radius and sync intervals (e.g., around Ghent):

```toml
[server]
port = 8082

[cache]
refresh_interval_seconds = 300  # OCM sync interval in seconds

[location]
name = "Ghent"
latitude = 51.0543
longitude = 3.7174
radius_km = 30

[ocm]
url = "https://api.openchargemap.io/v3/poi"

[osm]
url = "https://overpass.kumi.systems/api/interpreter"
```

Create an empty SQLite database file so Docker can mount it properly:

```bash
touch chargeapi.db
```

### 2. (Optional) Add a CSV dataset

The proxy supports ingesting a local CSV of EV chargers at startup. This is useful for official regional datasets that aren't fully covered by OCM or OSM.

The CSV must follow this column structure (the Flanders dataset format):

| # | Column | Description |
|---|--------|-------------|
| 1 | `laadpunt_teller` | Counter (ignored) |
| 2 | `uniek_identificatienummer` | Unique EVSE ID |
| 3 | `uitbater` | Operator name |
| 4 | `toegankelijkheid` | Accessibility |
| 5 | `kw` | Power in kW |
| 6 | `snelheid` | Speed (ignored) |
| 7 | `stroomtype` | Current type |
| 8 | `connector` | Connector type (IEC codes, semicolon-separated) |
| 9 | `adres` | Street address |
| 10 | `postcode` | Postal code |
| 11 | `gemeente` | Municipality |
| 12 | `provincie` | Province (ignored) |
| 13 | `latitude` | Latitude (WGS84) |
| 14 | `longitude` | Longitude (WGS84) |
| 15 | `vervoerregio` | Transport region (ignored) |
| 16 | `geometry` | Lambert geometry (ignored, lat/lon used instead) |

Any CSV following this structure will work — not just the Flanders dataset. Download the Flanders dataset [here](https://www.vlaanderen.be/datavindplaats/catalogus/laadpunten-voor-elektrische-voertuigen) (requires accepting terms on the page). Place it at `./data/chargers.csv` or configure a custom path via the `FLANDERS_CSV_PATH` environment variable.

> **Note:** The CSV is not synced automatically. To reload it, either restart the container or call `POST /admin/refresh`.

### 3. Run with Docker CLI

```bash
docker run -d \
  --name chargemap-proxy \
  -p 8082:8082 \
  -e DATABASE_URL=sqlite:./chargeapi.db \
  -e OCM_API_KEY=your_ocm_api_key \
  -e APP_API_KEY=your_app_api_key \
  -v $(pwd)/config.toml:/app/config.toml:ro \
  -v $(pwd)/chargeapi.db:/app/chargeapi.db \
  -v $(pwd)/data:/app/data:ro \
  detono/chargemap-proxy:latest
```

### Alternative: Docker Compose

```yaml
services:
  chargemap-proxy:
    image: detono/chargemap-proxy:latest
    container_name: chargemap-proxy
    ports:
      - "8082:8082"
    environment:
      - DATABASE_URL=sqlite:./chargeapi.db
      - OCM_API_KEY=your_ocm_api_key
      - APP_API_KEY=your_app_api_key
      # - FLANDERS_CSV_PATH=./data/chargers.csv  # default, override if needed
    volumes:
      - ./config.toml:/app/config.toml:ro
      - ./chargeapi.db:/app/chargeapi.db
      - ./data:/app/data:ro  # mount your CSV folder here
    restart: unless-stopped
```

## Authentication

All endpoints require the `x-api-key` header, matching the `APP_API_KEY` environment variable you provided to the container.

```bash
curl -H "x-api-key: your_app_api_key" http://localhost:8082/stations
```

## API Endpoints

### `GET /stations`

Returns all cached stations. Supports the following query parameters:

| Parameter | Type | Description |
|---|---|---|
| `lat` | float | Latitude for distance filtering |
| `lon` | float | Longitude for distance filtering |
| `radius_km` | float | Radius in km (requires lat + lon) |
| `min_power_kw` | float | Minimum connector power in kW |
| `connector_type` | string | Filter by connector type (e.g. `CCS`, `Type 2`, `CHAdeMO`) |
| `fast_charge_only` | bool | Only return fast charge connectors |
| `operational_only` | bool | Only return operational stations (default: `true`) |

Filters are combinable. When `lat`/`lon` are provided, results are sorted by distance ascending and a `distance_km` field is included.

**Examples:**

```bash
# Stations within 5km of a specific location
curl -H "x-api-key: key" "http://localhost:8082/stations?lat=51.0543&lon=3.7174&radius_km=5"

# Fast chargers only
curl -H "x-api-key: key" "http://localhost:8082/stations?fast_charge_only=true"

# Type 2 connectors with at least 22kW
curl -H "x-api-key: key" "http://localhost:8082/stations?connector_type=Type+2&min_power_kw=22"
```

### `GET /stations/{id}`

Returns a single station by its internal ID.

### `GET /health`

Returns `200 OK` if the service is running.

### `POST /admin/refresh`

Triggers an immediate cache refresh from all sources (OCM, OSM, and CSV). Returns `202 Accepted` and runs the sync in the background.

```bash
curl -X POST -H "x-api-key: your_app_api_key" http://localhost:8082/admin/refresh
```

## Data Sources & Sync Schedule

| Source | Sync frequency | Notes |
|--------|---------------|-------|
| Open Charge Map | Configurable (default: 5 min) | Uses `modifiedsince` after first sync |
| OpenStreetMap (Overpass) | Every 24 hours | Free public API, rate-limited |
| CSV dataset | On startup + manual refresh | Update by replacing the file and restarting or calling `/admin/refresh` |

Stations within 25m of each other across sources are automatically deduplicated — the richer data source wins on a field-by-field basis with OCM taking priority.
