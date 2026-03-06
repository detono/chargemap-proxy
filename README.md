# chargemap-proxy

A lightweight Rust/Axum API that caches EV charging station data from [Open Charge Map](https://openchargemap.org) into a local SQLite database and serves it to clients.

Built to avoid hitting OCM's API rate limits from mobile clients — one server fetches every 5 minutes, all app clients hit the cache.

## Stack

- **Rust** + **Axum** — HTTP server
- **SQLx** + **SQLite** — local cache
- **reqwest** — OCM API client
- **Tokio** — async runtime + background sync loop

## Setup

### Prerequisites

- Rust (stable)
- `cargo-sqlx` CLI: `cargo install sqlx-cli --no-default-features --features sqlite`

### Environment

Create a `.env` file:
```env
DATABASE_URL=sqlite:./chargeapi.db
OCM_API_KEY=your_ocm_api_key
APP_API_KEY=your_app_api_key
```

### Configuration

Edit `config.toml`:
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

### Run
```bash
touch chargeapi.db
cargo sqlx migrate run
cargo run
```

## Authentication

All endpoints require an `x-api-key` header:
```bash
curl -H "x-api-key: your_APP_API_KEY" http://localhost:8082/stations
```

## Endpoints

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

Filters are combinable. When `lat`/`lon` are provided, results are sorted by distance ascending and a `distance_km` field is included in the response.

**Examples:**
```bash
# Stations within 5km of Ghent city centre
curl -H "x-api-key: key" "http://localhost:8082/stations?lat=51.0543&lon=3.7174&radius_km=5"

# Fast chargers only
curl -H "x-api-key: key" "http://localhost:8082/stations?fast_charge_only=true"

# CCS chargers with at least 50kW nearby
curl -H "x-api-key: key" "http://localhost:8082/stations?lat=51.0543&lon=3.7174&radius_km=10&connector_type=CCS&min_power_kw=50"
```

**Response shape:**
```json
[
  {
    "id": 462764,
    "name": "Shell Recharge Gentbrugge",
    "address": "Brusselsesteenweg 735, Gentbrugge, 9050",
    "latitude": 51.0286,
    "longitude": 3.7676,
    "operator": "Shell Recharge Solutions (BE)",
    "usage_cost": "Shell Recharge App: €0.69/kWh",
    "is_operational": true,
    "number_of_points": 6,
    "distance_km": 1.2,
    "connectors": [
      {
        "type_name": "CCS (Type 2)",
        "formal_name": "IEC 62196-3 Configuration FF",
        "power_kw": 300.0,
        "amps": 500.0,
        "voltage": 1000.0,
        "current_type": "DC",
        "is_fast_charge": true,
        "is_operational": true,
        "quantity": 1
      }
    ]
  }
]
```

### `GET /stations/:id`

Returns a single station by OCM ID.
```bash
curl -H "x-api-key: key" http://localhost:8082/stations/462764
```

### `POST /admin/refresh`

Triggers an immediate cache refresh from OCM. Returns `202 Accepted` and runs the sync in the background.
```bash
curl -X POST -H "x-api-key: key" http://localhost:8082/admin/refresh
```

## Data Source

Station data is fetched from the [Open Charge Map API](https://openchargemap.org/site/develop/api) and cached locally. The background sync job runs on the interval configured in `config.toml`. Data includes station location, operator, connectors, power levels, and operational status.
