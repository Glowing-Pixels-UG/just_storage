# API Reference

Complete API documentation for JustStorage object storage service.

## Base URL

```text
http://localhost:8080/v1
```

## Authentication

All endpoints (except `/health`) require authentication using one of:

### JWT Bearer Token

```bash
Authorization: Bearer <jwt_token>
```

### API Key

```bash
Authorization: ApiKey <api_key>
```

Configure keys via environment variables:

- `JWT_SECRET` - Secret for JWT token validation
- `API_KEYS` - Comma-separated list of valid API keys

**Development Mode**: Set `DISABLE_AUTH=true` to skip authentication.

---

## Endpoints

### Health Check

#### GET /health

Check service health (no authentication required).

**Response: 200 OK**

```json
{
  "status": "healthy"
}
```

---

### Upload Object

#### POST /v1/objects

Upload an object with streaming body. Uses two-phase commit for crash safety.

**Query Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `namespace` | string | Yes | Object namespace (e.g., 'models', 'kb', 'uploads') |
| `tenant_id` | string | Yes | Tenant identifier |
| `key` | string | No | Human-readable key for retrieval |
| `storage_class` | string | No | 'hot' or 'cold' (default: 'hot') |

**Request Headers:**

```text
Content-Type: application/octet-stream
Authorization: Bearer <token>
```

**Request Body:** Binary data (streaming)

**Response: 201 Created**

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "namespace": "models",
  "tenant_id": "acme-corp",
  "key": "llama-3.1-8b",
  "content_hash": "sha256:a3c5f1e2b4d6...",
  "size_bytes": 17179869184,
  "content_type": "application/octet-stream",
  "storage_class": "hot",
  "status": "COMMITTED",
  "created_at": "2025-12-11T10:30:00Z",
  "metadata": null
}
```

**Example:**

```bash
curl -X POST 'http://localhost:8080/v1/objects?namespace=models&tenant_id=acme&key=llama-3.1-8b&storage_class=hot' \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGc..." \
  -H "Content-Type: application/octet-stream" \
  --data-binary @model.bin
```

---

### Download Object by ID

#### GET /v1/objects/{id}

Download an object by its UUID.

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | UUID | Yes | Object ID |

**Query Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `tenant_id` | string | Yes | Tenant identifier for authorization |

**Response: 200 OK**

```text
Content-Type: application/octet-stream
Content-Length: 17179869184
X-Object-Id: 550e8400-e29b-41d4-a716-446655440000
X-Content-Hash: sha256:a3c5f1e2b4d6...

[binary data]
```

**Example:**

```bash
curl 'http://localhost:8080/v1/objects/550e8400-e29b-41d4-a716-446655440000?tenant_id=acme' \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGc..." \
  -o model.bin
```

---

### Download Object by Key

#### GET /v1/objects/by-key/{namespace}/{tenant_id}/{key}

Download an object by its human-readable key.

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `namespace` | string | Yes | Object namespace |
| `tenant_id` | string | Yes | Tenant identifier |
| `key` | string | Yes | Object key |

**Response: 200 OK**

Same as download by ID.

**Example:**

```bash
curl 'http://localhost:8080/v1/objects/by-key/models/acme/llama-3.1-8b' \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGc..." \
  -o model.bin
```

---

### List Objects

#### GET /v1/objects

List objects with pagination and filtering.

**Query Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `namespace` | string | Yes | Filter by namespace |
| `tenant_id` | string | Yes | Filter by tenant |
| `limit` | integer | No | Results per page (default: 50, max: 1000) |
| `offset` | integer | No | Pagination offset (default: 0) |

**Response: 200 OK**

```json
{
  "objects": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "namespace": "models",
      "tenant_id": "acme-corp",
      "key": "llama-3.1-8b",
      "content_hash": "sha256:a3c5f1e2b4d6...",
      "size_bytes": 17179869184,
      "storage_class": "hot",
      "status": "COMMITTED",
      "created_at": "2025-12-11T10:30:00Z"
    }
  ],
  "total": 127,
  "limit": 50,
  "offset": 0
}
```

**Example:**

```bash
curl 'http://localhost:8080/v1/objects?namespace=models&tenant_id=acme&limit=50&offset=0' \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGc..."
```

---

### Delete Object

#### DELETE /v1/objects/{id}

Delete an object. Physical deletion happens asynchronously via garbage collection.

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | UUID | Yes | Object ID |

**Query Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `tenant_id` | string | Yes | Tenant identifier for authorization |

**Response: 204 No Content**

No response body.

**Example:**

```bash
curl -X DELETE 'http://localhost:8080/v1/objects/550e8400-e29b-41d4-a716-446655440000?tenant_id=acme' \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGc..."
```

---

## Error Responses

All errors follow a consistent format:

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "Object not found",
    "details": "Object with ID 550e8400-e29b-41d4-a716-446655440000 does not exist"
  }
}
```

### HTTP Status Codes

| Code | Meaning | Common Causes |
|------|---------|---------------|
| 400 | Bad Request | Invalid parameters, malformed UUID |
| 401 | Unauthorized | Missing or invalid authentication |
| 403 | Forbidden | Tenant mismatch, insufficient permissions |
| 404 | Not Found | Object doesn't exist or tenant mismatch |
| 409 | Conflict | Key already exists for namespace/tenant |
| 500 | Internal Server Error | Database or storage failure |

---

## State Machine

Objects transition through these states:

```text
(none) → WRITING → COMMITTED → DELETING → DELETED
```

**State Descriptions:**

- **WRITING**: Upload in progress, object not visible
- **COMMITTED**: Upload complete, object visible and downloadable
- **DELETING**: Delete requested, object no longer visible
- **DELETED**: Physical file removed (internal state, not exposed in API)

**Important**: Only objects in `COMMITTED` state are visible in list/download operations.

---

## Content Addressing

All uploaded content is automatically deduplicated using SHA-256 content addressing:

1. File is hashed during upload
2. Content hash becomes the storage key
3. Multiple objects can reference the same physical file
4. Reference counting tracks usage
5. Files are only deleted when ref count reaches zero

**Benefits:**

- Automatic deduplication saves storage
- Integrity verification via hash comparison
- Immutable content (hash changes if content changes)

---

## Rate Limiting

Currently no rate limiting implemented. Plan to add:

- Per-tenant request limits
- Upload bandwidth throttling
- Concurrent operation limits

---

## Best Practices

### Uploads

- Use streaming for large files (>100MB)
- Set appropriate `storage_class` (`hot` for frequently accessed, `cold` for archives)
- Include `key` for human-readable access patterns
- Monitor upload failures and retry with exponential backoff

### Downloads

- Use `by-key` endpoint for stable URLs
- Cache downloads when possible (content is immutable once hash is known)
- Handle 404 gracefully (object may be in WRITING state temporarily)

### Deletes

- Deletion is async - file may exist briefly after DELETE returns
- Don't rely on immediate space reclamation
- GC runs every 60 seconds by default

### Keys

- Use hierarchical naming: `{version}/{model-name}` (e.g., `v1/llama-3.1-8b`)
- Keep keys under 255 characters
- Avoid special characters that need URL encoding

---

## Examples

### Complete Upload Workflow

```bash
# 1. Upload with key
OBJECT_ID=$(curl -X POST \
  'http://localhost:8080/v1/objects?namespace=models&tenant_id=acme&key=llama-3.1-8b' \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/octet-stream" \
  --data-binary @model.bin | jq -r '.id')

# 2. Download by ID
curl "http://localhost:8080/v1/objects/$OBJECT_ID?tenant_id=acme" \
  -H "Authorization: Bearer $TOKEN" \
  -o downloaded.bin

# 3. Verify hash matches
sha256sum model.bin downloaded.bin

# 4. Download by key
curl "http://localhost:8080/v1/objects/by-key/models/acme/llama-3.1-8b" \
  -H "Authorization: Bearer $TOKEN" \
  -o downloaded2.bin

# 5. Delete when done
curl -X DELETE "http://localhost:8080/v1/objects/$OBJECT_ID?tenant_id=acme" \
  -H "Authorization: Bearer $TOKEN"
```

### Pagination

```bash
# Page 1
curl 'http://localhost:8080/v1/objects?namespace=models&tenant_id=acme&limit=50&offset=0' \
  -H "Authorization: Bearer $TOKEN"

# Page 2
curl 'http://localhost:8080/v1/objects?namespace=models&tenant_id=acme&limit=50&offset=50' \
  -H "Authorization: Bearer $TOKEN"
```

---

## SDK Examples

### Rust

```rust
use reqwest::Client;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

async fn upload_file(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let file = File::open(path).await?;
    let stream = FramedRead::new(file, BytesCodec::new());

    let response = client
        .post("http://localhost:8080/v1/objects")
        .query(&[
            ("namespace", "models"),
            ("tenant_id", "acme"),
            ("key", "my-model"),
        ])
        .header("Authorization", "Bearer token")
        .body(reqwest::Body::wrap_stream(stream))
        .send()
        .await?;

    let json: serde_json::Value = response.json().await?;
    Ok(json["id"].as_str().unwrap().to_string())
}
```

### Python

```python
import requests

def upload_file(filepath: str, namespace: str, tenant_id: str, key: str) -> str:
    with open(filepath, 'rb') as f:
        response = requests.post(
            'http://localhost:8080/v1/objects',
            params={
                'namespace': namespace,
                'tenant_id': tenant_id,
                'key': key,
            },
            headers={'Authorization': f'Bearer {token}'},
            data=f
        )
    response.raise_for_status()
    return response.json()['id']

def download_file(object_id: str, tenant_id: str, output_path: str):
    response = requests.get(
        f'http://localhost:8080/v1/objects/{object_id}',
        params={'tenant_id': tenant_id},
        headers={'Authorization': f'Bearer {token}'},
        stream=True
    )
    response.raise_for_status()

    with open(output_path, 'wb') as f:
        for chunk in response.iter_content(chunk_size=8192):
            f.write(chunk)
```

---

## See Also

- [QUICKSTART.md](QUICKSTART.md) - Quick start guide
- [DESIGN.md](../DESIGN.md) - State machine and consistency model
- [OPERATIONS.md](OPERATIONS.md) - Operations manual
