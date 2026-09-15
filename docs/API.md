# API Reference

Base URL: `http://localhost:3000`

## Health

### `GET /health`
Liveness probe.

**Response:**
```json
{
  "status": "ok",
  "version": "0.2.0",
  "uptime_seconds": 42
}
```

### `GET /api/v1/status`
Detailed service status.

**Response:**
```json
{
  "status": "ok",
  "version": "0.2.0",
  "uptime_seconds": 42,
  "network": "TESTNET",
  "contract_id": "C...",
  "soroban_rpc_url": "https://soroban-testnet.stellar.org:443",
  "horizon_url": "https://horizon-testnet.stellar.org",
  "entry_count": 5
}
```

## Entries

### `GET /api/v1/entries?start=0&limit=20`
Paginated list of audit entries.

**Query params:**
- `start` (default: 0) — starting entry index
- `limit` (default: 20, max: 100) — number of entries

**Response:**
```json
{
  "entries": [
    {
      "id": 0,
      "auditor": "G...",
      "subject": "G...",
      "category": "payment",
      "data_hash": "9f98196d...",
      "prev_hash": "00000000...",
      "entry_hash": "c38c2248...",
      "timestamp": 1789497417,
      "ledger": 1,
      "expires_at": 1789583817,
      "memo": "Testnet payment audit"
    }
  ],
  "count": 5,
  "start": 0,
  "limit": 20,
  "total": 5
}
```

### `GET /api/v1/entries/:id`
Single entry by ID.

**Response:** Single `AuditEntry` object (see above).

**Errors:** `404` if entry not found.

### `GET /api/v1/entries/count`
Total entry count.

**Response:** `{"count": 5}`

### `POST /api/v1/entries`
Record a new audit entry.

**Request body:**
```json
{
  "auditor": "G...",
  "subject": "G...",
  "category": "payment",
  "data_hash": "9f98196d...",
  "memo": "Audit note",
  "expires_at": 1789583817
}
```

**Response:** `{"success": true, "message": "Entry 5 recorded successfully"}`

## Verification

### `GET /api/v1/verify?from=0&to=4`
Verify hash-chain integrity for entries `[from, to]` inclusive.

**Response:**
```json
{
  "from": 0,
  "to": 4,
  "verified": true,
  "entries_checked": 5
}
```

## Admin

### `GET /api/v1/admin`
Get the admin address.

### `GET /api/v1/auditors/:addr`
Check if an address is an auditor.

**Response:** `{"address": "G...", "is_auditor": true}`

### `POST /api/v1/admin/grant`
Grant auditor role.

**Body:** `{"admin": "G...", "address": "G..."}`

### `POST /api/v1/admin/revoke`
Revoke auditor role.

**Body:** `{"admin": "G...", "address": "G..."}`

### `POST /api/v1/admin/pause`
Pause recording.

### `POST /api/v1/admin/unpause`
Resume recording.

### `GET /api/v1/admin/paused`
Check pause status.

**Response:** `{"paused": false}`

## Error Format

All errors return JSON:
```json
{
  "error": "not_found",
  "code": 404,
  "message": "Entry 999 not found"
}
```

| Code | Error | Description |
|---|---|---|
| 400 | `bad_request` | Invalid input parameters |
| 404 | `not_found` | Entry not found |
| 503 | `service_unavailable` | Contract not configured |
| 500 | `internal_error` | Server error |
