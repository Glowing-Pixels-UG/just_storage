# JustStorage — Local Docker Compose

The canonical Compose stack lives at the **repository root** in
[`docker-compose.yml`](../../docker-compose.yml). Run it from there:

```bash
# from the repo root
docker compose up -d
docker compose logs -f just_storage
curl http://localhost:8080/health
```

This brings up PostgreSQL (with the schema applied) and the JustStorage service.

## Configuration

All configuration is via environment variables. The authoritative, exhaustive
template is [`rust/.env.example`](../../rust/.env.example) — every variable there
is actually read by the service (see `rust/src/config.rs`).

Authentication model (v1):

- `INTERNAL_ADMIN_TOKEN` bootstraps admin access; create DB-backed API keys
  through the API.
- `OIDC_*` enables OpenID Connect (optional).
- `DISABLE_AUTH=true` is **development only**.

> Note: `JWT_SECRET` and a static `API_KEYS` list are **not** used by the v1
> runtime; ignore any older references to them.

## Cloud platforms

For PaaS targets (Heroku, Fly.io, Railway, Render, DigitalOcean, CapRover) use the
generator CLI and see [`../README.md`](../README.md):

```bash
cd rust && cargo run --release --bin just-storage-deploy -- generate <platform>
```
