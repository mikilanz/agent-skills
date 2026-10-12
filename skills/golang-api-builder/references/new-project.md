# Scaffolding a new service

The skill ships a ready starter in `assets/starter/` (next to this `references/` folder). It contains the
infrastructure: Gin bootstrap with graceful shutdown, slog JSON logging, CORS, config singleton,
pgxpool, golang-migrate auto-migration, Redis client, gateway-claims middleware, role guard,
query/filter/sort/pagination middleware and SQL builder, domain error mapping, the outbound HTTP
client, Swagger and health handlers, a worker entry point, an init migration, Dockerfile and
docker-compose. It builds, vets and passes its tests as shipped.

## Tools

Install once (pin versions the project already uses if it has any):

```bash
go install github.com/sqlc-dev/sqlc/cmd/sqlc@v1.30.0
go install github.com/swaggo/swag/cmd/swag@v1.16.6
go install -tags 'postgres' github.com/golang-migrate/migrate/v4/cmd/migrate@v4.19.1
```

## Steps

1. **Copy the starter** into the target directory. Resolve the skill's own folder; it's wherever this file lives.
   ```bash
   cp -r <skill-dir>/assets/starter/. <target-dir>/
   ```
2. **Rename the module.** The starter's module is `go-api-starter`. Use the repository name (a bare
   name such as `billing-service`, or a full path such as `github.com/org/billing-service`):
   ```bash
   cd <target-dir>
   grep -rl 'go-api-starter' --include='*.go' --include='go.mod' . | xargs sed -i 's#go-api-starter#<module>#g'
   ```
   On macOS use `sed -i ''`. Also change the `@title` / `@description` swag annotations and the
   `Starting Go API Starter...` log line in `cmd/api/main.go`.
3. **Dependencies**: `go mod tidy`.
4. **Regenerate docs**: `swag init -g cmd/api/main.go -o api/docs`.
5. **Trim what isn't needed.** Ask the user if it's unclear:
   - No Redis: remove the Redis block and `rdb` from `main.go`, the Redis check from `health/handler.go`, the `REDIS_*` config fields and env keys, and the `redis` service from `docker-compose.yaml`; then `go mod tidy`.
   - No worker: delete `cmd/worker/`.
   - Not behind an API gateway: replace `ClaimsMiddleware` with JWT verification (see architecture.md, "Auth model").
6. **Env**: `cp .env.example .env` and fill it in. `docker compose up -d db redis` gives a local Postgres and Redis matching the example values.
7. **Verify**:
   ```bash
   gofmt -l . && go vet ./... && go build -o bin/api ./cmd/api && go test ./...
   ./bin/api            # or: go run ./cmd/api
   curl localhost:8080/health
   open http://localhost:8080/swagger/index.html
   ```
8. **Add features** with the workflow in `SKILL.md` and the example in `templates.md`. The first
   repository needs `go get github.com/google/uuid` (or just `go mod tidy` after writing it), and the
   first handler test pulls in `testify/mock`.
9. Initialize git and make the first commit (`chore: initial scaffold`) only if the user asks.
   Generated code (`api/docs`, every `model/`) is committed.

## What's in the starter

```
go.mod  go.sum  .env.example  .gitignore  .dockerignore  Dockerfile  docker-compose.yaml
api/docs/                        swag output for the starter routes
cmd/
  api/main.go                    bootstrap, DI block, routes, graceful shutdown, runMigrations
  worker/main.go                 signal-aware worker skeleton
internal/
  config/config.go               Config + InitConfig/GetConfig + env helpers
  database/migrations/           20250101000000_init.{up,down}.sql (pgcrypto)
  domain/
    auth/claims.go               Claims, role constants, ScopedCompanyCode
    general/http-response.go     DefaultHttpResponse, HttpOptions
    general/errors.go            ErrNotFound, ErrDuplicate, ErrForbidden, ErrBadRequest
  application/middleware/
    claims-middleware.go         ClaimsMiddleware, GetClaims
    role-guard-middleware.go     RoleGuard
    query-pagination-middleware.go  QueryPaginationMiddleware, GetQueryRequest
    middleware_test.go
  adapter/http/
    health/handler.go            GET /health (db + redis ping)
    swagger/handler.go           GET /swagger/*any
  adapter/repository/            (created with the first repository: one folder per table)
  usecase/http/service.go        MakeRequest, MakeJSONRequest
  utils/
    filter.go  filter_test.go    QueryRequest types, Columns, FilterData, BuildSortClause, ReplacePlaceholders
    errors.go                    TranslateDBError, HttpStatusFromError
    logger.go                    InitLogging (JSON slog, sets default)
    mappers.go                   Map
```

Don't put real secrets, tokens or webhook URLs in `.env.example` or in `getEnvOrDefault` defaults.
Required secrets use `getEnvOrFatal`.
