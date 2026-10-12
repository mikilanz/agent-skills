# Architecture

## Layers and dependency direction

```
delivery/rest  (controllers: routes, extractors, guards, Swagger)
      │  builds per request, passes &Claims
      ▼
usecases       (services: rules, tenancy, orchestration, outbound calls, cache)
      │  &dyn <X>RepositoryAbstract
      ▼
dataproviders  (repository traits + Diesel implementations, entities, mappers, schema)
      │
      ▼
PostgreSQL

application    (cross-cutting: GeneralModules state, guards, error handling, mapper traits)
config         (ConfigService: env, pool, redis)
utils          (run_blocking, query parsing, pagination/filter builder, body/multipart helpers)
delivery/domain, delivery/interfaces, delivery/query, delivery/enums   (shared types)
```

- Controllers never touch Diesel or `GeneralModules.pool`; they call a service.
- Services depend on repository **traits**, so tests swap in `Mock<X>RepositoryAbstract`.
- Repositories return entities (`<X>Entity`), not DTOs. Exceptions: `DefaultHttpResponseDTO` from deletes, and create/update DTOs as inputs.
- Mappers are the only place model ↔ entity ↔ response DTO conversion happens (`DbMapper`, `ApiMapper` traits in `application/mappers`).
- External payload shapes (third-party request/response bodies) live in `delivery/interfaces`, not `delivery/domain`.

## Request lifecycle

1. actix `Logger` and `Cors` middleware.
2. Scope match (`/api/v1/<feature>`), then route match inside the controller's `routes()`.
3. Extractors run in signature order: `web::Data<GeneralModules>`, `web::Path`, `HttpRequest`,
   `web::Payload` / `web::Bytes` / `Multipart`, then the claim extractor. A missing or invalid claim
   header short-circuits with 401 before the body is read.
4. Handler: `role_guard(&claim, ROLES)?` → `body_payload::<DTO>(payload).await?` → `XService::new(&ctx.x_repository).method(..., &claim).await`.
5. Service: validation → tenant scope → repository → `ErrorHandling::*` on failure.
6. Repository: `run_blocking` borrows a pooled connection on actix's blocking thread pool and runs Diesel there.
7. Handler maps the entity with `XApiMapper::to_api` and responds; errors become `CustomErrorResponse`, which actix renders as `ErrorPresenter` JSON.

## App state: `GeneralModules`

One `web::Data<GeneralModules>` is created in `run()` and cloned into every worker. It holds:

| Field | Type | Notes |
|---|---|---|
| `app_name` | `String` | |
| `pool` | `DbPool` (`r2d2::Pool<ConnectionManager<PgConnection>>`) | `DATABASE_POOL_SIZE`, default 5; total connections = pool size × replicas |
| `config_service` | `ConfigService` | zero-sized; reads env on demand |
| `redis` | `redis::aio::ConnectionManager` | multiplexed, reconnecting; clone per command |
| `http_client` | `reqwest::Client` | connection pooling, 30s timeout |
| `scheduler` | `JobScheduler` | started in `run()` |
| `<x>_repository` | `<X>Repository { pool }` | one per table, `pool.clone()` |

Everything in it is a cheap handle (`Arc` inside). Add a field when a new shared handle is needed
(another pool, a client for a different upstream), not for per-request data. Shared mutable state
(`Arc<Mutex<HashMap<..>>>`) is allowed for process-local resources that can't be serialized (e.g. live
browser/WebDriver sessions); lock with `.lock().ignore_poison()`, keep the critical section free of
`.await`, and remember it's per replica.

## Auth model

The service normally sits behind Google Cloud Endpoints (ESPv2) or Google API Gateway. The gateway
validates the user's JWT (issuer + JWKS in the spec's `securityDefinitions`) and forwards the verified
payload base64url-encoded in `X-Endpoint-API-UserInfo` (Endpoints) or `X-Apigateway-Api-Userinfo`
(API Gateway). Sometimes the payload is nested as a JSON string under `claims`.

`ClaimGuardMiddleware` (an actix `FromRequest` extractor) decodes either form into `Claims`
(`companyCode`, `companyKey`, `role`, `username`, `email`, `sub`, …) and keeps the raw JSON in
`userinfo`. It derefs to `Claims`, so `claim.company_code` and `&claim` work wherever `&Claims` is
expected. A missing or undecodable header is a 401.

`JwtMiddleware` is the alternative for routes the gateway doesn't authenticate: it verifies
`Authorization: Bearer <jwt>` with RS256 using `JWKS_N` / `JWKS_E` (the `n` / `e` of the issuer's JWKS
key) and optional `JWT_AUDIENCE`, and yields the same `Claims`. If the issuer rotates keys, fetch the
JWKS (`JWKS_URL`) at boot, cache it in `GeneralModules`, and pick the key by `kid`; ask the user
before adding that.

Roles are plain strings with constants in `delivery/interfaces/general_interface.rs`: `superadmin`,
`admin`, `employee`, `user`. `role_guard(&claim, &[...])?` returns 403 for any other role.

Tenancy: data rows carry `company_code`. `claims.scoped_company_code(requested)` returns the caller's
own company for every role except superadmin, who gets `requested` (`None` = all companies). Services
pass that scope to repositories, which put it in the `WHERE` of every read, update and delete. A
superadmin narrows a list with `?company_code.equal=<code>` (the column must be in the whitelist).

Consequences:
- The service must not be reachable except through the gateway, or anyone can forge the header.
- `claims.username` goes into `created_by`; `claims.sub` is the user id.
- Public routes (webhooks, health) simply omit the extractor. Verify webhook authenticity yourself (shared token, signature header).

## Response and error contract

| Case | Status | Body |
|---|---|---|
| Single resource | 200 | response DTO (camelCase) |
| List | 200 | bare JSON array; total in `x-total-count` |
| Delete / action | 200 | `{"code":200,"message":"success","error":false}` (`DefaultHttpResponseDTO`) |
| Any error | 4xx/5xx | `{"code":404,"error":"Product not found","message":"404 Not Found"}` (`ErrorPresenter`) |

In `ErrorPresenter`, `error` is the specific message and `message` is the HTTP reason phrase.

Flow: services return `ApiError { code, message, error }` (the `error` cause is only logged).
`CustomErrorResponse::map_io_error` (also `From<ApiError>`, so `?` works) keeps the status, uses
`message` for 4xx and replaces it with a generic text for 5xx.

| Source | Status | Built by |
|---|---|---|
| bad JSON body | 400 (413 if > 2 MiB) | `body_payload` |
| missing/invalid claims | 401 | `ClaimGuardMiddleware` / `JwtMiddleware` |
| role not allowed | 403 | `role_guard` |
| business validation | 400 | `ErrorHandling::application_error` |
| bad list query (unknown field/criteria, wrong type) | 400 with the reason | `QueryError` → `repository_error` |
| `diesel::result::Error::NotFound`, 0 rows deleted | 404 | `repository_error` |
| unique violation | 409 | `repository_error` |
| FK / not-null / check violation | 400 | `repository_error` |
| upstream unreachable / non-2xx / unparsable | 502 | `ErrorHandling::upstream_error` (`RequestService`) |
| anything else | 500 | `repository_error` / `internal_error` |

## List query-string contract

Parsed by `utils::convert::query_transform`, validated and turned into SQL by `Paginate`:

| Param | Meaning | Default |
|---|---|---|
| `page` | zero-based page | `0` |
| `size` | page size, clamped to 1..=1000 | `10` |
| `sort` | `<column>,ASC` or `<column>,DESC` (one key) | `id,DESC` |
| `<column>.<criteria>=<value>` | filter; repeat for more (ANDed) | |

Other keys without a `.` are ignored (cache busters, `pagination=true`).

Criteria: `equal`, `notEqual`, `greaterThan`, `greaterThanOrEqual`, `lessThan`, `lessThanOrEqual`
(the legacy spellings `greatherThan` / `greatherThanOrEqual` are accepted), `includes` (case-sensitive
substring), `contains` (substring where spaces match anything: `foo bar` → `%foo%bar%`), `in` / `notIn`
(comma-separated), `between` (`a,b`, inclusive). `%` and `_` in values are matched literally.

Column names are the snake_case DB column names, declared per resource:

```rust
pub const PRODUCT_QUERY_COLUMNS: &[QueryColumn] = &[
    QueryColumn::new("id", ColumnKind::Integer),
    QueryColumn::new("name", ColumnKind::Text),
    QueryColumn::new("created_at", ColumnKind::Timestamptz),
];
```

`ColumnKind` decides parsing and binding: `Integer` (int4), `BigInt` (int8), `Text`, `Bool`
(`true/false/1/0`), `Double`, `Timestamptz` (RFC 3339 or `YYYY-MM-DD`), `Uuid`. An unknown column,
unknown criteria or unparsable value is a 400 that names the problem. To filter on a type not listed
(numeric, date), add a `ColumnKind` + `FilterValue` variant and its `push_bind_param` arm in
`utils/filter.rs`.

Generated SQL (values are binds):

```sql
SELECT *, COUNT(*) OVER () FROM (<inner select, incl. tenant filter>) t
WHERE CAST(t."name" AS TEXT) LIKE $1 AND t."stock" > $2 AND t."id" IN ($3, $4)
ORDER BY t."name" ASC LIMIT $5 OFFSET $6
```

Services add server-side filters with `QueryRequest::with_filter(key, criteria, value)` (jobs,
service-to-service calls); because filters are ANDed, a client filter can only narrow them.

## Database

- **Pool**: one `DbPool` built in `run()` (`DATABASE_URL`, `DATABASE_POOL_SIZE`). `Pool::build` connects eagerly, so a wrong URL fails at startup.
- **Blocking**: `run_blocking(&pool, move |conn| ...)` = `web::block` + `pool.get()`. It returns `Result<R, DbError>` where `DbError = Box<dyn Error + Send + Sync>`. Everything the closure uses must be owned (`move`).
- **Migrations**: `migrations/<version>_<name>/{up,down}.sql`, created with `diesel migration generate`. The `00000000000000_diesel_initial_setup` migration provides `diesel_manage_updated_at('<table>')`, which installs a trigger that keeps `updated_at` current. `run_migrations(&pool)` applies pending ones at boot from the copy embedded at compile time.
- **Schema**: `diesel.toml` prints the schema to `src/dataproviders/schema/schema.rs` on every `diesel migration run`. Commit it with the migration.
- **Queries**: reference columns as `<table>::<column>`; select with `X::as_select()` and return with `X::as_returning()` so struct/column mismatches fail at compile time.
- **Transactions**: `conn.transaction::<_, DbError, _>(|conn| { ...; Ok(v) })` inside the `run_blocking` closure. Returning `Err` rolls back.
- **Bulk insert**: `insert_into(t).values(&Vec<NewX>)` in chunks of ≤ 1000 rows (Postgres caps a statement at 65535 bind parameters).
- **Upsert**: `.on_conflict(t::company_code).do_update().set((...))`.
- **Raw SQL** (rare): `diesel::sql_query("... $1 ...").bind::<Text, _>(v).load::<XByName>(conn)` with a `QueryableByName` struct; never `format!` input into it.

## Caching

Redis through `RedisService::new(&ctx.redis)`: `get` → `Option<String>`, `set(key, value, Some(ttl_secs))`,
`delete`, `delete_pattern` (SCAN, not KEYS), `ping`.

- Cache-aside in the controller or service: one key-builder function shared by read and invalidate, keys `<feature>:<kind>:<scope>:<params>`, a TTL constant, invalidation on every write (tenant scope and the superadmin `all` scope).
- Cache failures are logged with `log::warn!` and ignored; the request falls through to the database.
- Tokens from other systems: `token:<system>:<scope>`, TTL a little shorter than the token's lifetime.

## Background work

| Need | Use |
|---|---|
| Fire-and-forget after responding (audit log, notification) | `actix_web::rt::spawn(async move { ... })` with a cloned `web::Data<GeneralModules>`; no `Send` needed |
| CPU-heavy or blocking work inside a request | `web::block(move || ...)` |
| Recurring job | `Job::new_async("<sec> <min> <hour> <dom> <mon> <dow>", ...)` registered in `register_jobs(ctx)`; the future must be `Send` |
| Long-lived per-tenant resource (sessions, sockets) | a handle in `GeneralModules` behind `Arc<Mutex<..>>`; per replica |

Jobs act as the system: build a `Claims { role: superadmin, username: "system" }` and call service
`query_*` methods. Store job ids returned by `scheduler.add` if they must be removed later. With
several replicas, take a redis lock (`SET <job>:lock <id> NX EX <ttl>`) for jobs that must run once.

## Outbound HTTP

`RequestService::new(&ctx.http_client)`:
- `request(RequestOptionDomain { url, method, payload, headers }) -> Result<String, ApiError>`. `payload` is a JSON string (empty for none). With `Content-Type: application/x-www-form-urlencoded` the JSON object is sent as a form.
- `request_json::<T>(...)` parses the body.
- `file_upload(url, field, bytes, filename, mime)` sends a multipart request.
- A non-2xx status or a transport error becomes a 502 `ApiError`. The upstream body is logged, never returned to the client.

Platform integrations follow the same pattern (templates.md has verified code):
- **Machine token**: `AuthService::get_machine_token()` does a password grant against `AUTH_ENGINE_URL/v1/oauth/tokens` with Basic `AUTH_BASIC_KEY`, and caches the token in redis.
- **Audit log**: `ActivityLogService::spawn(ctx.clone(), ActivityLogDomain::new(&claim, client_ip(&request), "CREATE", "<entity>", id))` posts to `LOG_ENGINE_URL/activity-logs` with the machine token.
- **Proxy**: a catch-all `#[route("/{path:.*}", method = ...)]` forwards to `UPSTREAM_URL` with server-side credentials, an allowlist of client headers, and `..` segments rejected.
- **User lookup**: the query engine exposes other services' data at `QUERY_ENGINE_URL/proxy/<companyCode>/services/<service>/api/<resource>?<filters>`, authenticated with the machine token.

## Files

- Uploads: `Multipart` extractor → `multipart_to_files(payload, "public/assets/files")` (sanitized name, uuid prefix, 20 MiB cap) → process (inside `web::block` for blocking parsers such as calamine) → delete the file.
- Static files, if served: `actix_files::Files::new("/public/assets", "public/assets")` (add `actix-files`). The Dockerfile copies `public/`.
- HTML responses (rare, e.g. a QR page): render a template (`handlebars`) and return `HttpResponse::Ok().content_type("text/html; charset=utf-8")` with `Cache-Control: no-cache, no-store`.

## API gateway spec

Projects behind Cloud Endpoints keep a Swagger 2.0 file (`cloudendpoint.json`, often also
`cloudendpoint-prod.json`) with `x-google-*` OAuth2 `securityDefinitions`. Each route needs:

```json
"/api/v1/product/{product_id}": {
  "get": {
    "tags": ["product_controller"],
    "operationId": "product_controller_get_product",
    "parameters": [
      { "name": "product_id", "in": "path", "required": true, "type": "integer" },
      { "name": "x-endpoint-api-userinfo", "in": "header", "required": true, "type": "string" }
    ],
    "responses": { "200": { "description": "Succesfully" } }
  }
}
```

- `operationId` must be unique across the file; `<controller>_<handler>` keeps it so.
- Catch-all proxies use `/api/v1/<scope>/{url=**}`.
- Public routes (webhooks, health) need `"security": []` on the operation.
- Update the spec in the same change as the route. Ask before editing it, since deploying it is a separate step.

## Deployment

- **Docker**: multi-stage. The builder (`rust:<ver>-bookworm` + `libpq-dev`) caches dependencies with a stub build, then copies `src` and `migrations`. The runtime is `debian:bookworm-slim` + `libpq5` + `ca-certificates`, and copies `public/`.
- **Cloud Build**: `cloudbuild.yaml` (branch → build, push to Artifact Registry, `gke-deploy` the manifest) and `cloudbuild-version.yaml` (git tag → production image).
- **GKE**: `cloudbuild-app.yaml` has a Deployment (port 8080, readiness probe `/health`) and a ClusterIP Service. Non-secret env goes in `env`; secrets come from `envFrom.secretRef`.
- **Health**: `GET /health` checks Postgres (`SELECT 1`) and Redis (`PING`), and returns 503 if either fails.
- **Logs**: env_logger to stdout, level from `RUST_LOG` (default `info`).

## Working in an older project

Projects started from the earlier version of this layout share its folders and names but differ in
versions and helpers. Follow the project's existing style for new code, but don't copy its unsafe helpers:

| Older code | Do instead |
|---|---|
| `diesel 1.4`: `#[table_name = "x"]`, `&conn`, `#[macro_use] extern crate diesel`, `table!` | Stay on 1.4 inside that project (`get_result(&conn)`); don't mix 2.x syntax. Upgrading is its own task: ask first |
| Diesel calls directly in `async fn` | wrap in `web::block` (copy `utils/db.rs::run_blocking`, adapting the connection type) |
| `utils/filter.rs::query_db_filter` formatting `key = 'value'` into SQL | port this skill's `utils/filter.rs` + `utils/diesel_pagination.rs` (whitelist + binds) before adding list endpoints |
| `body_payload` / `query_transform` that `expect()` | return `Result` (400) as in the starter's `utils/convert.rs` |
| `CustomErrorReponse::map_io_error` matching only 400/401/403 | `StatusCode::from_u16` + generic message for 5xx |
| `#[async_trait(?Send)]` repository traits | keep for existing ones; new traits `Send + Sync` + `#[async_trait]` |
| `reqwest::Client::new()` per call, `redis::Client` per call | a shared client / `ConnectionManager` in `GeneralModules` |
| `ClaimGuardMiddleware { userinfo: Value }` read with `claim["companyCode"].as_str().unwrap()` | add typed `Claims` (`serde(default)`) next to `userinfo` |
| `JwtMiddleware` with a literal secret, or `token.replace("Bearer ", "")` with `unwrap()` | RS256 with `JWKS_N` / `JWKS_E` from env, `strip_prefix("Bearer ")` |
| `println!` logging, `#![allow(unused)]` at the crate root | `log` macros; remove the allows when touching that file if the user agrees |
| Migrations read from disk at runtime (`run_pending_migrations(&conn)`), errors only printed | `embed_migrations!` + fail startup; the Dockerfile must copy `migrations/` |
| Secrets in `.env-example`, `docker-compose.yml`, `cloudbuild-app.yaml` | placeholders + Kubernetes Secret; tell the user so they can rotate the exposed values |
