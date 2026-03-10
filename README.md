# Chargemap Proxy
[![Docker Version](https://img.shields.io/docker/v/detono/chargemap-proxy?style=flat-square)](https://hub.docker.com/r/detono/chargemap-proxy)
[![Build Status](https://img.shields.io/github/actions/workflow/status/detono/chargemap-proxy/deploy.yml?branch=main&style=flat-square)](https://github.com/detono/chargemap-proxy/actions)
[![Docker Pulls](https://img.shields.io/docker/pulls/detono/chargemap-proxy?style=flat-square)](https://hub.docker.com/r/detono/chargemap-proxy)
[![Image Size](https://img.shields.io/docker/image-size/detono/chargemap-proxy/latest?style=flat-square)](https://hub.docker.com/r/detono/chargemap-proxy)

A lightweight, blisteringly fast Rust/Axum API that caches EV charging station data from [Open Charge Map](https://openchargemap.org) into a local SQLite database and serves it to your clients.

Built specifically to prevent mobile apps and frontends from hammering OCM's API and hitting rate limits. A single background process fetches fresh data every few minutes, while all your clients hit the lightning-fast local cache.

## Features
* **Built for Speed:** Written in Rust using Axum and Tokio.
* **Smart Caching:** Local SQLite database ensures instant responses and zero rate-limiting from upstream.
* **Geospatial Filtering:** Query stations by bounding radius, connector type, and minimum power output.
* **Multi-Arch Support:** Available for `amd64` and `arm64`.

## Quick Start

The easiest way to get the proxy running is via Docker. You will need an Open Charge Map API key, and you'll need to define your own API key to secure your proxy endpoints.

### 1. Prepare your configuration files

Create a `config.toml` file to define your search radius and sync intervals (e.g., around Ghent):
```toml
[server]
port = 8082

[cache]
refresh_interval_seconds = 300

[location]
name = "Ghent"
latitude = 51.0543
longitude = 3.7174
radius_km = 160

[filters]
connector_types = []
networks = []
```

Create an empty SQLite database file so Docker can mount it properly:
```bash
touch chargeapi.db
```

### 2. Run with Docker CLI

```bash
docker run -d \
  --name chargemap-proxy \
  -p 8082:8082 \
  -e DATABASE_URL=sqlite:./chargeapi.db \
  -e OCM_API_KEY=your_ocm_api_key \
  -e APP_API_KEY=your_app_api_key \
  -v $(pwd)/config.toml:/app/config.toml:ro \
  -v $(pwd)/chargeapi.db:/app/chargeapi.db \
  detono/chargemap-proxy:latest
```

### Alternative: Docker Compose

If you prefer `docker-compose.yml`:
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
    volumes:
      - ./config.toml:/app/config.toml:ro
      - ./chargeapi.db:/app/chargeapi.db
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
| `connector_type` | string | Filter by connector (e.g. `CCS`, `Type 2`, `CHAdeMO`) |
| `fast_charge_only` | bool | Only return fast charge connectors |
| `operational_only` | bool | Only return operational stations (default: `true`) |

Filters are combinable. When `lat`/`lon` are provided, results are sorted by distance ascending and a `distance_km` field is included.

**Examples:**
```bash
# Stations within 5km of a specific location
curl -H "x-api-key: key" "http://localhost:8082/stations?lat=51.0543&lon=3.7174&radius_km=5"

# Fast chargers only
curl -H "x-api-key: key" "http://localhost:8082/stations?fast_charge_only=true"
```

### `GET /stations/:id`
Returns a single station by its OCM ID.

### `POST /admin/refresh`
Triggers an immediate cache refresh from OCM. Returns `202 Accepted` and runs the sync in the background.