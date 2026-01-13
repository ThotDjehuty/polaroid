# REST Exec API (QuestDB-like)

Polaroid now exposes a small HTTP API designed to mirror the *shape* and ergonomics of QuestDB’s `/exec` endpoint.

## Why this exists

- Easy integrations with tools that expect a simple REST query endpoint.
- A bridge to the long-term architecture:
  - Time-series and metadata via QuestDB
  - Distributed SQL execution via DataFusion + Ballista

## Server configuration

Environment variables:
- `POLAROID_HTTP_BIND_ADDRESS` (default: `0.0.0.0:9000`)
- `POLAROID_QUESTDB_HTTP_URL` (optional): e.g. `http://questdb:9000`
  - if set, Polaroid will proxy `/exec?query=...` to QuestDB

Start the server (gRPC + HTTP in the same process):

```bash
cd polaroid
cargo run -p polaroid-grpc
```

## Endpoints

### `GET /ping`
Health check.

Response:
- `200 OK` with body `ok`

### `GET /exec`

Two modes:

#### 1) Polaroid handle mode (expose DataFrame)

Request:
- `/exec?handle=<handle>&fmt=json&limit=1000`

Behavior:
- Loads the DataFrame referenced by `handle` from the Polaroid server.
- Returns a QuestDB-like JSON payload with `columns` and `dataset`.

Notes:
- `fmt=json` is currently required.
- `limit` defaults to `1000`.

Example (curl):

```bash
curl "http://localhost:9000/exec?handle=$HANDLE&fmt=json&limit=100"
```

Example response shape:

```json
{
  "query": "handle:<handle>",
  "columns": [{"name": "col", "type": "LONG"}],
  "dataset": [[1], [2]],
  "count": 2
}
```

#### 2) QuestDB proxy mode (SQL)

Request:
- `/exec?query=<sql>&fmt=json`

Behavior:
- If `POLAROID_QUESTDB_HTTP_URL` (or `QUESTDB_HTTP_URL`) is set, Polaroid proxies the request to `${QUESTDB}/exec?query=...&fmt=json`.
- This makes Polaroid a single entrypoint for time-series SQL + Polaroid handles.

If QuestDB is not configured:
- returns `412 Failed Precondition` with a hint to set `POLAROID_QUESTDB_HTTP_URL`.

Example (curl):

```bash
export POLAROID_QUESTDB_HTTP_URL="http://localhost:9000"   # or your QuestDB host
curl "http://localhost:9000/exec?query=select%201&fmt=json"
```

Example (Python):

```python
import os
import requests

base = os.environ.get("POLAROID_HTTP_URL", "http://localhost:9000")

print(requests.get(f"{base}/ping").text)

# QuestDB proxy mode (requires POLAROID_QUESTDB_HTTP_URL/QUESTDB_HTTP_URL set)
r = requests.get(f"{base}/exec", params={"query": "select 42", "fmt": "json"})
print(r.status_code)
print(r.text)
```

## Roadmap

- Local SQL execution: DataFusion plan execution on a single node.
- Distributed SQL execution: submit DataFusion plans to Ballista scheduler.
- Output formats: Arrow IPC and CSV.
- Auth: token/mTLS and per-tenant handle namespaces.
