---
name: golang-api-builder
description: Build or extend a Go backend REST API in a layered (ports-and-adapters) house style — Gin behind an API gateway, PostgreSQL through pgx/v5 with sqlc-generated queries and golang-migrate SQL migrations, Redis cache-aside via go-redis, log/slog JSON logging, swaggo Swagger annotations, gateway-forwarded user claims with role guards and tenant (companyCode) scoping, a standard query-string filter/sort/pagination contract with an x-total-count header, and testify handler tests. Use when creating a new Go API service, or when adding an endpoint, CRUD resource, table, migration, sqlc query, DTO, dynamic list filter, Redis cache, outbound HTTP integration, background job, worker or env var to a Go project laid out as cmd/api, internal/adapter, internal/domain, internal/usecase.
---

# Go API Builder

A layered Go architecture for REST APIs: `handler → service → repository`, wired by hand in
`cmd/api/main.go`. Use it to scaffold a new service, or to extend one that already follows it.

- **New project:** follow [references/new-project.md](references/new-project.md). It copies the ready-made starter in `assets/starter/`, which builds, vets and passes its tests as shipped.
- **Existing project:** first read the closest existing feature end to end (handler → dto → service → domain → repository → query.sql → migration → main.go wiring), then copy its shape. The project's own conventions override this skill wherever they differ.

References:
- [references/architecture.md](references/architecture.md): layers and dependency direction, request lifecycle, auth model, response and error contract, list query-string contract, caching, background work, outbound HTTP, API gateway spec, deployment
- [references/templates.md](references/templates.md): a complete, verified example feature covering every file type, plus snippets for env vars, outbound integrations, transactions, bulk insert and background jobs

## Stack

| Concern | Choice |
|---|---|
| Language | Go 1.25+ (uses `sync.WaitGroup.Go`, `min`, range-over-int) |
| HTTP | **Gin** (`gin-gonic/gin`), `gin-contrib/graceful` for shutdown, `gin-contrib/cors`, `gin-contrib/slog` request logs |
| DB | PostgreSQL via **pgx/v5** `pgxpool`; **sqlc** generates typed queries per repository; dynamic list queries built with `utils.FilterData` on the raw pool |
| Migrations | **golang-migrate** SQL files `YYYYMMDDHHMMSS_name.up.sql` / `.down.sql`, applied at boot when `AUTO_DATABASE_MIGRATION=ON` |
| Cache | **go-redis/v9**, cache-aside with explicit keys and TTLs, SCAN-based invalidation |
| Config | `joho/godotenv` + `os.Getenv`, a `sync.Once` singleton `config.Config` with UPPER_SNAKE fields |
| Logging | stdlib **log/slog** JSON handler, logger injected into every constructor |
| Docs | **swaggo/swag** annotations on handlers → `api/docs`, served at `/swagger/index.html` |
| Validation | Gin `binding:"..."` tags (go-playground/validator) on request DTOs |
| IDs | `id BIGSERIAL` internal key + `uuid UUID DEFAULT gen_random_uuid()` public key (`google/uuid` to parse) |
| Tests | `testing` + **testify** (`assert`, `mock`), handlers tested with `gin.CreateTestContext` and a mocked service interface |
| Deploy | multi-stage Dockerfile (alpine), Cloud Build, Kubernetes, Google Cloud Endpoints / API Gateway in front |

Don't introduce alternatives (an ORM such as GORM, another router, a DI framework, zap/logrus, viper,
`database/sql` for app queries, a different migration tool) unless the user asks.

## Folder map

```
cmd/
  api/main.go                       config → logger → pgxpool → migrations → redis → DI → router → graceful run
  worker/main.go                    optional background process sharing internal/
api/docs/                           swag output (generated, committed)
internal/
  config/config.go                  Config struct + InitConfig/GetConfig
  database/migrations/              golang-migrate files (the schema source of truth for sqlc)
  domain/<context>/<entity>.go      entity struct (json tags) + <Entity>Repository interface + pagination types
  domain/auth/claims.go             gateway claims, role constants, ScopedCompanyCode
  domain/general/                   DefaultHttpResponse, HttpOptions, sentinel errors (ErrNotFound, …)
  adapter/http/<feature>/
    handler.go                      Handler, NewHandler, RegisterRoutes, one swag-annotated method per route
    handler_test.go                 MockService + table of handler tests
    dto/request.go  dto/response.go
  adapter/http/swagger/ health/     infra handlers
  adapter/repository/<table-kebab>/
    sqlc.yaml  query.sql            sqlc input
    model/                          sqlc output (generated, DO NOT EDIT)
    repository.go                   implements the domain interface, maps model ↔ domain
  application/middleware/           ClaimsMiddleware, RoleGuard, QueryPaginationMiddleware (+ GetClaims, GetQueryRequest)
  usecase/<feature>/service.go      ServiceInterface + Service (business rules, tenancy, cache, side effects)
  usecase/http/service.go           the one outbound HTTP client (MakeRequest / MakeJSONRequest)
  utils/                            filter.go (dynamic SQL), errors.go, logger.go, mappers.go (generic Map)
```

Directories are kebab-case (`order-item`); the Go package is the name without dashes
(`package orderitem`). Files are kebab-case (`claims-middleware.go`, `http-response.go`).

## Workflow: adding a resource or endpoint

Work through these in order and skip the steps that don't apply:

1. **Read a sibling feature** in the project.
2. **Migration**: `migrate create -ext sql -dir internal/database/migrations -format 20060102150405 create_<table>_tb`. The timestamp must sort after every existing file. Write both `up` and `down`.
3. **sqlc**: `adapter/repository/<table-kebab>/sqlc.yaml` + `query.sql`, then run `sqlc generate` in that folder. Never edit `model/`.
4. **Domain**: `domain/<context>/<entity>.go` with the entity struct and its `Repository` interface (all methods take `ctx context.Context` first).
5. **Repository**: implements the interface, maps sqlc rows to domain structs, returns errors through `utils.TranslateDBError`. Dynamic lists use `utils.FilterData` with a column whitelist.
6. **DTOs**: `dto/request.go` with `binding` tags, `dto/response.go` with the public shape.
7. **Service**: `ServiceInterface` + `Service` + `NewService(rdb, repos..., logger)`. Tenancy, cache and side effects live here.
8. **Handler**: `RegisterRoutes` with `middleware.RoleGuard(...)` per route, swag comment on every method.
9. **Wire** in `cmd/api/main.go`: repository → service → handler in the `var (...)` DI block, then `xHandler.RegisterRoutes(apiV1)`.
10. **Env vars**: field in `Config`, `getEnvOrFatal`/`getEnvOrDefault` in `InitConfig`, `.env.example`, and the deployment manifest.
11. **Docs**: `swag init -g cmd/api/main.go -o api/docs` (then `swag fmt` if the project uses it).
12. **Gateway spec**: if the project deploys behind Cloud Endpoints / API Gateway with a static OpenAPI file (`cloudendpoint*.json`, `openapi*.yaml`), add each new path and method there with a unique `operationId`, or the gateway blocks it. Ask before editing it.
13. **Tests**: handler test with a `MockService` (update the mock whenever `ServiceInterface` changes).
14. **Verify**: `go mod tidy && gofmt -l . && go vet ./... && go build ./... && go test ./...`.

## Conventions

**Handlers** are thin: auth, bind, delegate, map to DTO, respond.
- Struct `Handler{svc usecase.ServiceInterface; logger *slog.Logger}`; constructor `NewHandler(svc, logger)`. Depend on the interface so tests can mock it.
- `RegisterRoutes(rg *gin.RouterGroup)` creates `rg.Group("/<feature>")` and registers each route as `group.METHOD("path", middleware.RoleGuard(roles...), h.method)`. Handler methods are unexported camelCase.
- Route shape: `GET ""` (list, paginated), `GET "/:uuid"` (single), `POST ""` (create), `PUT ""` (update, `uuid` in body), `DELETE ""` (delete, `uuid` in body). Actions are verb-ish sub-paths: `POST "publish"`, `PUT "user/mark-read"`.
- Every method starts by reading claims; a missing claim returns 401 `DefaultHttpResponse`. Bind with `c.ShouldBindJSON(&req)`; a bind error returns 400 with `err.Error()` as the message.
- On a service error respond `status := utils.HttpStatusFromError(err)` with a fixed `"Failed to <action> <thing>"` message. Don't leak raw DB errors.
- Success: the response DTO (or `[]DTO` via `utils.Map`) with 200. Lists set `c.Header("x-total-count", ...)` and return a bare JSON array. Actions and deletes return `DefaultHttpResponse{StatusCode: 200, Message: "success …", Ok: true}`.
- Pass `c` (the `*gin.Context`) as the `context.Context` to the service.

**Services**:
- `ServiceInterface` declared above `Service` in the same file; `NewService(rdb, <deps>..., logger)` returns `*Service`. Depend on domain repository interfaces, not concrete repositories.
- Tenancy: never trust a company code from the body. Use `claim.ScopedCompanyCode(requested)`: superadmin may pick, everyone else is pinned. Pass the scope down to the repository and filter on it in SQL.
- Log every error once, where it is handled: `s.logger.Error("failed to <action>", slog.String("error", err.Error()), <context attrs>)`, then return it. Don't swallow errors and return `nil`.
- Cache-aside with Redis: one key-builder function shared by get and set, `<feature>:<kind>:<scope>:<params>` keys, a TTL constant, and invalidation on every write. Cache failures are logged, never returned.
- Work that outlives the request runs in a goroutine with `context.WithoutCancel(ctx)`; batch it with a bounded worker pool (`sync.WaitGroup.Go` + channels).

**Repositories**:
- Struct `<Entity>Repository{queries queries.Queries; db *pgxpool.Pool; logger *slog.Logger}`; `NewRepository(dbpool, logger)` calls `queries.New(dbpool)`. Import the sqlc package as `queries "<module>/internal/adapter/repository/<table>/model"`.
- Map with a private `map<Entity>(row queries.<Entity>) *domain.<Entity>` helper: `pgtype.Text → .String`, `pgtype.Timestamptz → .Time`, `pgtype.UUID → .String()`, JSONB `[]byte → json.Unmarshal`.
- Parse UUIDs with `uuid.Parse` → `pgtype.UUID{Bytes: u, Valid: true}`; an invalid UUID is `general.ErrBadRequest`.
- Updates and deletes are scoped by tenant in the `WHERE`; use `:one` + `RETURNING *` for updates and `:execrows` for deletes, and turn "no row" into `general.ErrNotFound`.
- Dynamic lists: `utils.FilterData(query.Filters, columns)` + `utils.BuildSortClause(query.Sort, columns)` + `LIMIT ? OFFSET ?`, `utils.ReplacePlaceholders`, `pgx.CollectRows(rows, pgx.RowToStructByName[queries.X])`, and a `COUNT(*)` with the same WHERE.

**SQL and migrations**: table names snake_case singular (`order_item`), column names
camelCase and always double-quoted (`"companyCode"`). Every table has `"id" BIGSERIAL PRIMARY KEY`,
`"uuid" UUID NOT NULL DEFAULT gen_random_uuid()`, `"createdAt" TIMESTAMPTZ NOT NULL DEFAULT NOW()`;
mutable tables add `"updatedAt"` (set it in the UPDATE query) and `"createdBy" VARCHAR(100)`. Enums are
`VARCHAR` + `CHECK (... IN (...))`; free-form payloads are `JSONB`. Constraints are named
`<table>_<cols>_unique`, indexes `idx_<table>_<cols>`. Use `CREATE ... IF NOT EXISTS`.

**DTOs**: `json` tags in camelCase matching the column names; `binding:"required"` etc. on request
fields (Gin ignores `validate:` tags). Server-set fields (`createdBy`, tenant) are not in request DTOs
except an optional `companyCode` honoured for superadmin only.

**Domain**: plain structs with `json` tags, no framework imports except `utils.QueryRequest` in list
signatures. Pagination results are `<Entity>Pagination{Data, Total, Page, Size}`.

**Config**: add the field under a `// Section` comment, read it in `InitConfig` with
`getEnvOrFatal` (required) or `getEnvOrDefault` (optional). Read config with `config.GetConfig()`.

## Style

- `gofmt` everything. Import groups: stdlib, then this module, then third-party, separated by blank lines.
- Import aliases: `domain` for the feature's own domain package, `<x>Domain` for others (`authDomain`), `usecase` for the service package inside a handler, `queries` for sqlc models, `httpUsecase` for the outbound client, and in `main.go` `<feature>Handler`, `<feature>Repo`, `<feature>Usecase`.
- Error variables are named after the call: `errCache`, `errMarshal`, `errUpdate`. Log messages are lowercase `"failed to …"`; attributes use `slog.String/Int/Int64`.
- Swag comment block on every handler: `// Title`, `//`, then tab-aligned `@Summary`, `@Schemes`, `@Description`, `@Tags`, `@Accept json`, `@Produce json`, `@Param x-endpoint-api-userinfo header string true "..."`, `@Param`/`@Success`/`@Failure ... {object} general.DefaultHttpResponse`, `@Security BearerAuth`, `@Router /api/v1/... [method]`.
- Comments are sparse and explain why. Match the language of the surrounding code.
- Commits use Conventional Commits: `feat: …`, `fix: …`, `chore: …`, `refactor: …`. Branches `feature/…`, `hotfix/…`.

## Gotchas

- **Never interpolate a query-string key into SQL.** Filter and sort keys come from the client; `FilterData` / `BuildSortClause` only accept keys present in the repository's column whitelist. If an existing project's `FilterData` takes a bare alias and quotes whatever key arrives, that is a SQL injection hole: add a whitelist before exposing a new list endpoint.
- Values from the query string are strings. pgx sends a Go string as text for any column type, so `stock.greaterThan=10` works, but a bad value (`stock.equal=abc`) is a Postgres `22P02` error → map it to 400, not 500.
- `ReplacePlaceholders` rewrites every `?`, so don't build dynamic SQL that contains the jsonb `?` / `?|` operators; use `jsonb_exists()` or put that part in sqlc.
- `in` / `notIn` values arrive as `[]string`; a `FilterData` that only type-switches on `[]interface{}` silently formats them as `"[a b]"`. `between` with fewer than two values must be skipped, not emit an empty condition.
- A goroutine launched from a handler must not use `c` or the request context: Gin recycles `*gin.Context` and the context is cancelled when the response is written. Use `context.WithoutCancel(ctx)` and copy what you need.
- Use the **same** key-builder for cache get and set, include every input that changes the result (filters, sort, page, size) in the key, and invalidate both the tenant's keys and any superadmin/global keys on write.
- `graceful.Default()` wraps `gin.Default()`, which adds Gin's text logger and recovery; use `graceful.New(gin.New(), ...)` with your own slog + recovery middleware.
- Check `redis.ParseURL` / `pgxpool.New` errors before touching the result.
- Close every `http.Response.Body` and check the status code; prefer `MakeJSONRequest`. Never cache a token or payload before confirming it decoded.
- Tenant checks belong in SQL (`WHERE "uuid" = @uuid AND (@company_code::text = '' OR "companyCode" = @company_code)`). Update/delete by uuid alone lets one company modify another's rows.
- Keep sqlc's `schema:` pointed at `internal/database/migrations` (sqlc reads golang-migrate files and skips `.down.sql`). A separate hand-maintained `schema.sql` drifts from the real schema. If the project already uses one, update it in the same change as the migration.
- sqlc names positional casts `Column3`; use named params (`@filters::jsonb`, `sqlc.narg(x)`) to get readable struct fields.
- Use `TIMESTAMPTZ`, not `TIMESTAMP`, so times survive timezone changes; compare times in UTC.
- A route registered in `RegisterRoutes` but missing from the gateway spec is blocked in production; a route in the spec but missing from `RegisterRoutes` returns 404. Check both, plus the swag annotation.
- `go build ./cmd/api` writes a binary named `api`, which collides with the `api/` docs directory; build with `-o bin/api`.
- Adding a method to `ServiceInterface` breaks every `MockService` in `handler_test.go` until you add it there too.
- CORS `AllowMethods` must include every verb you route (PUT, PATCH) and `ExposeHeaders` must include `x-total-count`, or browsers can't read it.
