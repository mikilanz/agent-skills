# Scaffolding a new service

The skill ships a starter in `assets/starter/` (next to this `references/` folder). It contains the
infrastructure: the actix-web bootstrap with CORS and request logging, `ConfigService`, one Diesel r2d2
pool, embedded migrations (with Diesel's `updated_at` helper), a Redis connection manager, a shared
reqwest client, the cron scheduler, the `GeneralModules` app state, gateway-claims and RS256 JWT
extractors, a role guard, the error contract, the mapper traits, the list query parser and whitelisted
filter/sort/pagination builder, body/multipart helpers, `RedisService`, `RequestService`, a
`/health` endpoint, Swagger, a Dockerfile, docker-compose, and Cloud Build + GKE templates. It builds
with no clippy warnings and passes its tests as shipped. It contains no business resources;
templates.md has a complete one.

## Tools

```bash
# Rust stable (rustup), plus the libpq headers for Diesel
sudo apt-get install -y libpq-dev pkg-config        # macOS: brew install libpq && brew link --force libpq
cargo install diesel_cli --no-default-features --features postgres
```

## Steps

1. **Copy the starter** into the target directory. Resolve the skill's own folder; it's wherever this file lives.
   ```bash
   cp -r <skill-dir>/assets/starter/. <target-dir>/
   ```
2. **Rename the crate.** The package is `rust-api-starter` and the library is `rust_api_starter`. Use the
   repository name in kebab-case (package/binary) and snake_case (library):
   ```bash
   cd <target-dir>
   grep -rl 'rust-api-starter\|rust_api_starter\|Rust API Starter' --exclude-dir=target . \
     | xargs sed -i 's/rust-api-starter/<name-kebab>/g; s/rust_api_starter/<name_snake>/g; s/Rust API Starter/<Display Name>/g'
   ```
   On macOS use `sed -i ''`. This touches `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `src/lib.rs` and the `Dockerfile`.
3. **Env**: `cp .env.example .env`. `docker compose up -d db redis` gives a local Postgres and Redis
   matching the example values. `ENV=staging` makes the binary load `.env.staging` instead.
4. **Trim what isn't needed.** Ask the user if it's unclear:
   - **No database** (pure integration/proxy service):
     - Remove `diesel` and `diesel_migrations` from `Cargo.toml`.
     - Delete `migrations/`, `diesel.toml`, `src/migrations/`, `src/dataproviders/`, `src/delivery/query/`, `src/utils/filter.rs` and `src/utils/diesel_pagination.rs`.
     - Reduce `src/utils/db.rs` to `pub type DbError = Box<dyn std::error::Error + Send + Sync>;`.
     - Drop the deleted modules from `src/lib.rs`, `src/utils/mod.rs` and `src/delivery/mod.rs`.
     - In `run()`, remove the pool, `run_migrations` and the `pool` field. Remove `DbPool` and `get_pool_connection_db` from `ConfigService`, `ErrorHandling::repository_error` with its diesel imports, and the database check in `health_controller`.
     - Remove `DATABASE_*` from `.env.example`, the `db` service from `docker-compose.yml`, and `libpq` plus `COPY migrations` from the `Dockerfile` (keep `ca-certificates`).
   - **No Redis**: remove `redis` from `Cargo.toml`, `get_redis_connection`, the `redis` field, `usecases/redis_service.rs`, the Redis check in `health_controller`, `REDIS_URL`, and the `redis` compose service.
   - **No cron jobs**: remove `tokio-cron-scheduler` and the `scheduler` field / start.
   - **No uploads**: remove `actix-multipart`, `multipart_to_files` and `public/`. If you do, also drop `COPY public` from the Dockerfile.
   - **Not behind an API gateway**: use `JwtMiddleware` instead of `ClaimGuardMiddleware` in handlers (same `Claims`), set `JWKS_N` / `JWKS_E` / `JWT_AUDIENCE`, and delete the `cloudendpoint*.json` step from your workflow.
5. **Deployment templates**: in `cloudbuild.yaml`, `cloudbuild-version.yaml` and `cloudbuild-app.yaml`, replace `<region>`, `<project>`, `<repository>`, `<cluster>`, `<namespace>` and `<service>`, or delete them if the project deploys differently. Create the Kubernetes Secret from the real env file; never put secret values in the manifest.
6. **Verify**:
   ```bash
   cargo fmt --check && cargo clippy --all-targets && cargo test
   cargo run                                   # applies migrations, listens on APP_PORT
   curl localhost:8000/health                  # {"code":200,"message":"ok","error":false}
   open http://localhost:8000/api/             # Swagger UI (trailing slash)
   ```
7. **Add features** with the workflow in `SKILL.md` and the example in `templates.md`: `diesel migration generate create_<table>` first, then entity → mappers → repository → query whitelist → DTOs → service → controller → wiring.
8. Initialize git and make the first commit (`chore: initial scaffold`) only if the user asks. Commit `Cargo.lock` and `schema.rs`; `target/` and `.env*` (except `.env.example`) are ignored.

## What's in the starter

```
Cargo.toml  Cargo.lock  rustfmt.toml  diesel.toml  .env.example  .gitignore  .dockerignore
Dockerfile  docker-compose.yml  cloudbuild.yaml  cloudbuild-version.yaml  cloudbuild-app.yaml
migrations/00000000000000_diesel_initial_setup/   diesel_manage_updated_at() helper
public/assets/files/.gitkeep                      upload scratch folder
src/
  main.rs                                    env file, stdout logger, bind APP_PORT, run()
  lib.rs                                     ApiDoc + run(): pool → migrations → redis → client → scheduler → state → routes
  config/config_service.rs                   ConfigService::{get_env, get_env_or, get_pool_connection_db, get_redis_connection}, DbPool
  migrations/migration_table.rs              MIGRATIONS (embedded), run_migrations(&pool)
  application/
    error/error_handling.rs                  ErrorHandling (application/not_found/internal/upstream/unauthorized/forbidden/repository)
    guards/claim_guard.rs                    ClaimGuardMiddleware (+ decode_userinfo, tests)
    guards/jwt_guard.rs                      JwtMiddleware (RS256, JWKS_N/JWKS_E)
    guards/role_guard.rs                     role_guard(&claims, &[roles]) (+ tests)
    mappers/{api,db,http}_mapper.rs          mapper traits
    modules/general_modules.rs               GeneralModules
  dataproviders/
    schema/schema.rs                         generated by `diesel migration run`
    entities/ mappers/{api,db}/ repositories/{abstract_repository/}   empty mod.rs files to fill
  delivery/
    domain/default.rs                        DefaultHttpResponseDTO, QueryFilter, QueryRequest
    domain/error.rs                          ApiError, ErrorPresenter, CustomErrorResponse, QueryError (+ tests)
    domain/request_domain.rs                 RequestOptionDomain
    enums/status_enum.rs                     StatusEnum (active | inactive)
    interfaces/general_interface.rs          Claims, role constants, scoped_company_code (+ tests)
    query/default_query.rs                   DefaultQuery (Swagger params for list endpoints)
    rest/health_controller.rs                GET /health
  usecases/redis_service.rs                  RedisService (get/set/delete/delete_pattern/ping)
  usecases/request_service.rs                RequestService (request/request_json/file_upload)
  utils/
    db.rs                                    DbError, run_blocking
    convert.rs                               body_payload, query_transform, multipart_to_files, client_ip (+ tests)
    filter.rs                                QueryColumn, ColumnKind, criteria parsing, WHERE/ORDER BY builders
    diesel_pagination.rs                     Paginate / Paginated (+ SQL tests)
    mutex.rs                                 LockResultExt::ignore_poison
```

Don't put real secrets, tokens or connection strings with passwords in `.env.example`,
`docker-compose.yml` (beyond the throwaway local `postgres/postgres`) or the deployment manifest.
