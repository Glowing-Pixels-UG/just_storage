# Scanner Service Configuration

This document describes the configuration for the scanner service to connect to JustStorage.

## Configuration Values

The scanner service is configured with the following environment variables:

```bash
SCANNER_STORAGE_BASE_URL=https://storage.bk.glpx.pro
SCANNER_STORAGE_API_KEY=sk_canon_BB6YR79iGWyVmCOn65KIoSJrR-bIh7w21vIgUeb7fzB6M_dSleZWwA
SCANNER_STORAGE_JWT_TOKEN=
SCANNER_STORAGE_NAMESPACE=canon-scanner-documents
SCANNER_STORAGE_TENANT_ID=2f161fa2-06f6-46a4-88dc-3894b94ec6ee
```

## Configuration File

The configuration is stored in `scanner-config.env` in the project root.

## Authentication

The scanner service uses API key authentication. The API key has been added to JustStorage secrets and is active.

**API Key**: `sk_canon_BB6YR79iGWyVmCOn65KIoSJrR-bIh7w21vIgUeb7fzB6M_dSleZWwA`

To authenticate, include the API key in the request header:
```bash
Authorization: ApiKey sk_canon_BB6YR79iGWyVmCOn65KIoSJrR-bIh7w21vIgUeb7fzB6M_dSleZWwA
```

## Namespace and Tenant

- **Namespace**: `canon-scanner-documents`
- **Tenant ID**: `2f161fa2-06f6-46a4-88dc-3894b94ec6ee`

All scanner documents will be stored under this namespace and tenant combination.

## Usage Examples

### Upload a Document

```bash
curl -X POST "https://storage.bk.glpx.pro/v1/objects?namespace=canon-scanner-documents&tenant_id=2f161fa2-06f6-46a4-88dc-3894b94ec6ee&key=document-001" \
  -H "Authorization: ApiKey sk_canon_BB6YR79iGWyVmCOn65KIoSJrR-bIh7w21vIgUeb7fzB6M_dSleZWwA" \
  -H "Content-Type: application/octet-stream" \
  --data-binary @document.pdf
```

### List Documents

```bash
curl "https://storage.bk.glpx.pro/v1/objects?namespace=canon-scanner-documents&tenant_id=2f161fa2-06f6-46a4-88dc-3894b94ec6ee" \
  -H "Authorization: ApiKey sk_canon_BB6YR79iGWyVmCOn65KIoSJrR-bIh7w21vIgUeb7fzB6M_dSleZWwA"
```

### Search Documents

```bash
curl -X POST "https://storage.bk.glpx.pro/v1/objects/search" \
  -H "Authorization: ApiKey sk_canon_BB6YR79iGWyVmCOn65KIoSJrR-bIh7w21vIgUeb7fzB6M_dSleZWwA" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "canon-scanner-documents",
    "tenant_id": "2f161fa2-06f6-46a4-88dc-3894b94ec6ee",
    "key_contains": "invoice",
    "limit": 50
  }'
```

### Download by Key

```bash
curl "https://storage.bk.glpx.pro/v1/objects/by-key/canon-scanner-documents/2f161fa2-06f6-46a4-88dc-3894b94ec6ee/document-001" \
  -H "Authorization: ApiKey sk_canon_BB6YR79iGWyVmCOn65KIoSJrR-bIh7w21vIgUeb7fzB6M_dSleZWwA" \
  -o document.pdf
```

## Verification

The API key has been tested and is working. You can verify it with:

```bash
curl -H "Authorization: ApiKey sk_canon_BB6YR79iGWyVmCOn65KIoSJrR-bIh7w21vIgUeb7fzB6M_dSleZWwA" \
  https://storage.bk.glpx.pro/health
```

Expected response:
```json
{"service":"activestorage","status":"healthy","version":"0.1.0"}
```

