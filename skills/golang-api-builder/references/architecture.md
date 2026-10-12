# Architecture

## Layers and dependency direction

```
cmd/api/main.go ──wires──▶ adapter/http/<feature>  (Handler)
                               │ depends on usecase.ServiceInterface
                               ▼
                           usecase/<feature>        (Service)
                               │ depends on domain.<Entity>Repository interfaces,
                               │ *redis.Client, *httpUsecase.Service, other services
                               ▼
                           adapter/repository/<table>  (implements the domain interface)
                               │ uses sqlc model/ + *pgxpool.Pool
                               ▼
                           PostgreSQL
```

- `domain/` is the center: entity structs, repository interfaces, sentinel errors. It imports nothing
  from `adapter/` or `usecase/` (the one allowed exception is `utils.QueryRequest` in list signatures).
- `usecase/` holds business rules: tenancy, validation that needs data, cache, orchestration of several
  repositories, outbound calls, background work. Services may import their feature's request DTOs
  (`adapter/http/<feature>/dto`) as input types; this is the house trade-off that avoids a second set
  of input structs.
- `adapter/http/` translates HTTP ↔ service calls. No SQL, no Redis, no business rules.
- `adapter/repository/` translates domain ↔ SQL. No HTTP, no cache, no cross-repository logic.
- `application/middleware/` is Gin plumbing shared by every route.
- `utils/` is dependency-light helpers (`Map`, dynamic SQL, error mapping, logger setup).

There is no DI framework. `main.go` builds everything in one `var (...)` block, in dependency order:
shared clients → repositories → services → handlers. A service needed by several services (for
example an integration service that fetches a machine token) is constructed once and passed as a
concrete `*Service`.

## Request lifecycle

```
Gin engine (graceful.New(gin.New()))
  ginSlog.SetLogger            one JSON access-log line per request (skips /health)
  gin.Recovery                 panic → 500
  cors.New(...)
  group ""                     /swagger/*any, /health   (no auth)
  group "api/v1"
    ClaimsMiddleware           decode gateway user-info header → c.Set("userInfo", Claims) or 401
    QueryPaginationMiddleware  parse page/size/sort/filters → c.Set("queryRequest", QueryRequest)
    <feature> group
      RoleGuard(roles...)      claims.Role in roles or 403
      handler method           GetClaims → ShouldBindJSON → svc.X(c, claim, ...) → DTO → c.JSON
```

## Auth model

The service sits behind Google Cloud Endpoints (ESPv2) or Google API Gateway. The gateway validates
the user's JWT (issuer + JWKS configured in the gateway spec's `securityDefinitions`) and forwards
the verified payload base64url-encoded in `X-Endpoint-API-UserInfo` (Endpoints) or
`X-Apigateway-Api-Userinfo` (API Gateway). `ClaimsMiddleware` decodes it into `auth.Claims`
(`companyCode`, `role`, `username`, `sub`, …). The service never sees or verifies the raw token.

Consequences:
- The service must not be reachable except through the gateway (ClusterIP service, internal ingress),
  or anyone can forge the header.
- Roles are plain strings: `superadmin`, `admin`, `employee`, `user`. Use the constants in
  `domain/auth`. Superadmin is cross-tenant; everyone else is pinned to `claims.CompanyCode`.
- `claims.Sub` is the user id (for per-user data such as inboxes); `claims.Username` goes in
  `createdBy`.

If a project is **not** behind such a gateway, replace `ClaimsMiddleware` with JWT verification
(for example `github.com/golang-jwt/jwt/v5` + a JWKS fetcher such as `github.com/MicahParks/keyfunc/v3`)
that fills the same `auth.Claims` and stores it under the same context key, so handlers and
`RoleGuard` stay unchanged. Ask the user before choosing a library.

## Response and error contract

| Case | Status | Body |
|---|---|---|
| Single resource | 200 | response DTO object |
| List | 200 | bare JSON array of DTOs; total in the `x-total-count` header |
| Action / delete | 200 | `{"statusCode":200,"message":"success …","ok":true}` |
| Missing / bad claims | 401 | `{"statusCode":401,"message":"…","ok":false}` |
| Role not allowed | 403 | same shape |
| Bind / validation error | 400 | same shape, `message` = validator error text |
| Domain error | from `utils.HttpStatusFromError` | same shape, fixed `"Failed to …"` message |

Domain errors are sentinels in `domain/general/errors.go`, wrapped with `fmt.Errorf("%w: …", ErrX)`:

| Sentinel | Status | Typical source |
|---|---|---|
| `ErrNotFound` | 404 | `pgx.ErrNoRows`, `:execrows` returned 0, tenant mismatch |
| `ErrDuplicate` | 409 | unique violation `23505` |
| `ErrBadRequest` | 400 | FK / check violation, invalid uuid, bad filter value `22P02`, business validation |
| `ErrForbidden` | 403 | business-level permission check in a service |
| anything else | 500 | |

Repositories return `utils.TranslateDBError(err)`, so services and handlers never import pgx.

## List query-string contract

Parsed once by `QueryPaginationMiddleware` for every `api/v1` route and read with
`middleware.GetQueryRequest(c)`:

| Param | Meaning | Default |
|---|---|---|
| `page` | zero-based page | `0` |
| `size` | page size, capped at 1000 | `10` |
| `pagination` | `true` when the client wants paging; parsed and passed through | `false` |
| `sort` | `<field>,asc` or `<field>,desc`; repeat the param for more keys | none (repository default) |
| `<field>.<criteria>` | filter; first value wins | |

Criteria: `equal`, `notEqual`, `greaterThan`, `greaterThanOrEqual`, `lessThan`, `lessThanOrEqual`,
`contains` (case-sensitive substring), `includes` (case-insensitive, spaces match anything:
`foo bar` → `%foo%bar%`), `in` / `notIn` (comma-separated), `between` (`a,b`, inclusive).

Field names are the camelCase column names. Each repository declares which ones are allowed:

```go
var productColumns = utils.Columns("p", "uuid", "companyCode", "name", "stock", "status", "createdAt")
```

Unknown keys are silently ignored. For joins, merge maps or add entries by hand, for example key
`jobStatus` → expression `bj."status"`. Tenant scoping is applied by the service appending an `equal` filter on
`companyCode`; since all filters are ANDed, a client-supplied `companyCode` filter can only narrow it.

## Caching

Redis is cache-aside and best effort:
- Read: `rdb.Get(ctx, key).Result()`; `redis.Nil` is a miss; other errors are logged and treated as a miss.
- Write: `json.Marshal` the domain result, `rdb.Set(ctx, key, value, ttl)`; errors are logged.
- Invalidate on every write with `rdb.Scan(ctx, 0, pattern, 100).Iterator()` + `Del`, covering the
  tenant's keys and any superadmin/global keys.
- Counters that must stay exact (unread counts, quotas) are recomputed from the DB and `Set` after the
  write rather than incremented blindly.
- Key format `<feature>:<kind>:<scope>:<params>`; for list queries hash the whole `QueryRequest`.

## Background work

- In-process async (fan-out after a request): the service starts a goroutine with
  `context.WithoutCancel(ctx)`, records progress in a job table (`<thing>_job` with
  `status` `pending|processing|completed|failed`, `total…`, `processed…`, `failed…`, `startedAt`,
  `completedAt`, `createdBy`) and processes batches with a fixed worker pool. The API returns 200
  immediately.
- Scheduled work: rows with `status = 'pending'` and a `scheduledTime`, processed by an endpoint
  restricted to `superadmin` that a scheduler (Cloud Scheduler, a CronJob) calls, or by the
  `cmd/worker` process. Mark a row `processing` before starting so overlapping runs don't repeat it.
- `cmd/worker/main.go` is a separate binary for long-running consumers; it shares `internal/` and stops
  when its signal context is cancelled.

## Outbound HTTP

All external calls go through `usecase/http.Service` (`MakeRequest` returns the raw response;
`MakeJSONRequest` checks for 2xx, decodes and closes). A per-integration service in
`usecase/<integration>/service.go` builds URLs from `config` base URLs, adds auth headers, and pages
through upstream lists using their `x-total-count` header. Machine tokens are cached in Redis for a
little less than their lifetime.

## API gateway spec

Production traffic passes through a static Swagger 2.0 file (`cloudendpoint.json`,
`cloudendpoint-prod.json`, …) deployed with `gcloud endpoints services deploy <file>`. Every route
needs an entry there:

```json
"/api/v1/product": {
  "get": {
    "operationId": "Product_get_list",
    "description": "Retrieve products",
    "consumes": ["application/json"],
    "produces": ["application/json"],
    "tags": ["Product"],
    "summary": "Retrieve products",
    "parameters": [
      { "type": "string", "description": "x-endpoint-api-userinfo", "name": "x-endpoint-api-userinfo", "in": "header", "required": true }
    ],
    "responses": {
      "200": { "description": "OK" },
      "401": { "description": "Unauthorized", "schema": { "$ref": "#/definitions/general.DefaultHttpResponse" } }
    }
  }
}
```

- Path parameters use `{uuid}` and need a `"in": "path"` parameter.
- `operationId` must be unique across the file: `<Tag>_<verb>_<thing>`.
- The `security` block (top-level `oauth2` with `x-google-issuer`, `x-google-jwks_uri`,
  `x-google-audiences`) applies to every path; public paths such as `/swagger` and `/health` override
  it with `"security": [{}]`.
- Definitions referenced with `$ref` must exist under `definitions`. Copy them from the regenerated
  `api/docs/swagger.json`.
- There is usually one file per environment; update all of them in the same change.

## Deployment

- `Dockerfile`: `golang:1.25-alpine` build stage (`CGO_ENABLED=0`), `alpine` runtime stage that copies
  the binary and `internal/database` (migrations are read from `file://internal/database/migrations`
  relative to `/`).
- Cloud Build: one config builds and pushes the image tagged with the branch and restarts the
  workload (`kubectl rollout restart`); another builds release images on tags; another deploys the
  gateway spec.
- Kubernetes runs the image with env vars from a ConfigMap/Secret; `/health` is the probe.
- Versioning: `APP_VERSION` env var, shown in Swagger.
