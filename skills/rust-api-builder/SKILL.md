---
name: rust-api-builder
description: Build or extend a Rust backend REST API in a layered house style — actix-web 4, Diesel 2 + r2d2 on PostgreSQL with embedded Diesel migrations and blocking DB work moved off the async workers, utoipa Swagger, a shared GeneralModules app state, mockable repository traits (async-trait + mockall), gateway-forwarded user claims with role guards and companyCode tenant scoping, a whitelisted filter/sort/pagination query-string contract with an x-total-count header, Redis via redis-rs, reqwest outbound calls, tokio-cron-scheduler jobs, and Cloud Build + GKE deployment. Use when creating a new Rust API service, or when adding an endpoint, CRUD resource, table, migration, entity, mapper, repository, DTO, list filter, guard, cache, outbound integration or proxy, audit log, cron job, file import or env var to a Rust project laid out as src/application, src/dataproviders, src/delivery, src/usecases.
---

# Rust API Builder

A layered actix-web architecture for REST APIs: `controller → service → repository`, with all shared
handles in one `GeneralModules` app state. Use it to scaffold a new service, or to extend one that
already follows it.

- **New project:** follow [references/new-project.md](references/new-project.md). It copies the starter in `assets/starter/`, which builds with no clippy warnings and passes its tests as shipped.
- **Existing project:** first read the closest existing feature end to end (controller → service → repository trait → repository → entity → mappers → DTOs → schema → migration → `lib.rs` wiring), then copy its shape. The project's own conventions override this skill wherever they differ. Older projects in this style use Diesel 1.4, `?Send` traits and a few unsafe helpers; see "Working in an older project" in architecture.md before copying their utilities.

References:
- [references/architecture.md](references/architecture.md): layers, request lifecycle, app state, auth model, response and error contract, list query-string contract, database, caching, background work, outbound HTTP, files, API gateway spec, deployment, and working in an older project
- [references/templates.md](references/templates.md): a complete, verified `products` resource covering every file type, plus snippets for env vars, cache-aside, machine tokens, audit logging, proxy endpoints, cron jobs, Excel import with a transactional bulk insert, and outbound calls

## Stack

| Concern | Choice |
|---|---|
| Language | Rust 2021 edition, stable toolchain (starter verified on 1.90) |
| HTTP | **actix-web 4**, `actix-cors`, `actix-multipart`; controllers use the route macros (`#[get("")]`) plus `pub fn routes(cfg)` per controller |
| DB | PostgreSQL via **Diesel 2** (`postgres`, `r2d2`, `chrono`, `uuid`) with one `r2d2` pool per process; blocking calls wrapped in `utils::db::run_blocking` (`web::block`) |
| Migrations | **diesel_migrations** `embed_migrations!("migrations")`, applied at boot; `diesel` CLI to create/run/redo and to regenerate `schema.rs` |
| Docs | **utoipa** `#[utoipa::path]` on every handler + `#[derive(OpenApi)] ApiDoc` in `lib.rs`; **utoipa-swagger-ui** at `/api/`, spec at `/api/openapi.json` |
| Cache / state | **redis** (redis-rs) `aio::ConnectionManager` wrapped by `RedisService` |
| Outbound HTTP | **reqwest**, one shared `Client` (30s timeout) wrapped by `RequestService` |
| Jobs | **tokio-cron-scheduler** `JobScheduler` in app state; 6-field cron (seconds first) |
| Auth | API gateway forwards claims (`ClaimGuardMiddleware`); RS256 `JwtMiddleware` (**jsonwebtoken**) for routes the gateway doesn't authenticate |
| Errors | `ApiError` (services) → `CustomErrorResponse` (`thiserror`, implements `ResponseError`) → `ErrorPresenter` JSON |
| Serialization | **serde** / serde_json, `#[serde(rename_all = "camelCase")]` DTOs; `serde_urlencoded` for query strings |
| Config | `dotenvy` (`.env`, or `.env.<ENV>` when `ENV` is set) read through `ConfigService` |
| Logging | `log` macros + **env_logger** to stdout; actix `Logger` middleware |
| Tests | built-in `#[test]` / `#[actix_web::test]`, **mockall** `automock` on repository traits, `diesel::debug_query` for SQL |
| Files | `calamine` for Excel import (add when needed); uploads under `public/assets/files` |
| Deploy | multi-stage Dockerfile (Debian slim + libpq), Cloud Build → Artifact Registry → GKE, Cloud Endpoints / API Gateway in front |

Don't introduce alternatives (axum/rocket, sqlx/sea-orm, deadpool, a DI container, tracing in place of
env_logger, config/figment, validator) unless the user asks.

## Folder map

```
Cargo.toml  Cargo.lock  diesel.toml  rustfmt.toml  Dockerfile  docker-compose.yml  .env.example
cloudbuild.yaml  cloudbuild-version.yaml  cloudbuild-app.yaml     Cloud Build + GKE manifest
cloudendpoint*.json                                              gateway spec (if behind Cloud Endpoints / API Gateway)
migrations/<version>_<name>/{up,down}.sql                        Diesel migrations (embedded at compile time)
public/assets/files/                                             temporary uploads
src/
  main.rs                       load .env / .env.<ENV>, init logger, bind APP_PORT, run()
  lib.rs                        ApiDoc, run(): pool → migrations → redis → http client → scheduler → GeneralModules → routes
  config/config_service.rs      ConfigService (get_env / get_env_or), DbPool, pool + redis builders
  migrations/migration_table.rs MIGRATIONS + run_migrations(&pool)
  application/
    error/error_handling.rs     ErrorHandling::{application_error, not_found_error, repository_error, upstream_error, ...}
    guards/                     claim_guard (gateway claims), jwt_guard (RS256), role_guard
    mappers/                    ApiMapper, DbMapper, HttpMapper traits
    modules/general_modules.rs  GeneralModules app state (pool, redis, http client, scheduler, repositories)
  dataproviders/
    schema/schema.rs            Diesel table! macros (generated)
    entities/<table>_entity.rs  <Model> (Queryable+Selectable), New<Model><'a> (Insertable), <Model>Entity
    mappers/db/<table>_mappers.rs   <Model>DbMapper: model ↔ entity
    mappers/api/<table>_mappers.rs  <Model>ApiMapper: entity → response DTO
    repositories/abstract_repository/<table>_repository_abstract.rs   trait (automock)
    repositories/<table>_repository.rs                                 Diesel implementation
  delivery/
    rest/<feature>_controller.rs    routes() + handlers with #[utoipa::path]
    domain/<feature>_domain.rs      request/response DTOs (ToSchema); default.rs, error.rs, request_domain.rs
    interfaces/                     external payload shapes, Claims + role constants (general_interface.rs)
    query/                          DefaultQuery (Swagger params) + <feature>_query.rs filter/sort whitelists
    enums/                          StatusEnum and other closed value sets
  usecases/<feature>_service.rs     business rules, tenancy, orchestration; redis_service, request_service
  utils/                            db (run_blocking, DbError), convert (body_payload, query_transform, multipart_to_files, client_ip),
                                    filter + diesel_pagination (whitelisted filters, Paginate), mutex (ignore_poison)
```

Files and modules are snake_case. Every folder has a `mod.rs` listing its modules; a new file isn't
compiled until it is added there.

## Workflow: adding a resource or endpoint

Work through these in order and skip the steps that don't apply:

1. **Read a sibling feature** in the project.
2. **Migration**: `diesel migration generate create_<table>`; write `up.sql` and a real `down.sql`; `diesel migration run` (regenerates `src/dataproviders/schema/schema.rs`) then `diesel migration redo`.
3. **Entity** `dataproviders/entities/<table>_entity.rs`: row struct, `New<X><'a>` insertable, `<X>Entity`.
4. **Mappers**: `<X>DbMapper` (model ↔ entity) and `<X>ApiMapper` (entity → response DTO).
5. **Repository trait** in `abstract_repository/` (`#[cfg_attr(test, automock)]`, `#[async_trait]`, `: Send + Sync`), then the implementation: every method is `run_blocking(&self.pool, move |conn| { ... }).await`, scoped by `company_code`.
6. **Query whitelist** `delivery/query/<feature>_query.rs` for any list endpoint.
7. **DTOs** `delivery/domain/<feature>_domain.rs`: `<X>ResponseDTO`, `<X>DTO` (create), `Update<X>DTO`, `Delete<X>DTO`, all `ToSchema` + camelCase.
8. **Service** `usecases/<feature>_service.rs`: `<X>Service<'a> { repository: &'a dyn <X>RepositoryAbstract }`, validation, tenant scope, `ErrorHandling::repository_error(...)`; unit tests with the mock.
9. **Controller** `delivery/rest/<feature>_controller.rs`: `routes(cfg)`, claim extractor + `role_guard` on every non-public handler, `#[utoipa::path]` on each.
10. **Wire**: `mod.rs` entries, repository field in `GeneralModules` built with `pool.clone()` in `lib.rs`, `ApiDoc` paths + schemas, `web::scope("/api/v1/<feature>").configure(<feature>_controller::routes)` before the Swagger service.
11. **Env vars**: read through `ConfigService`, add to `.env.example` (placeholders only) and to the deployment (`env` or the Secret).
12. **Gateway spec**: if the project has `cloudendpoint*.json` / an OpenAPI file for Cloud Endpoints or API Gateway, add every new path and method with a unique `operationId`, or the gateway blocks it. Ask before editing it.
13. **Verify**: `cargo fmt --check && cargo clippy --all-targets && cargo test && cargo build`, then run it and hit the endpoints (Swagger UI at `/api/`).

## Conventions

**Controllers** are thin: guard, parse, delegate, map, respond.
- `pub fn routes(cfg: &mut web::ServiceConfig)` chains `.service(handler)` for every handler; the scope prefix is set in `lib.rs`.
- Handler signature: `async fn <verb>_<feature>(ctx: web::Data<GeneralModules>, ..., claim: ClaimGuardMiddleware) -> Result<HttpResponse, CustomErrorResponse>`. Put `role_guard(&claim, ROLES)?;` first.
- Route shape: `GET ""` (list), `GET "/{<feature>_id}"`, `POST ""` (create), `PUT ""` (update, id in body), `DELETE ""` (delete, id in body), actions as sub-paths (`POST "/import"`). Path params: `web::Path<(T,)>` → `path.into_inner().0`.
- JSON bodies: `let body = body_payload::<XDTO>(payload).await?;` (400 on bad JSON, 413 when too large).
- Build the service per request: `XService::new(&ctx.x_repository)`, then `results.map_err(CustomErrorResponse::map_io_error).map(|entity| HttpResponse::Ok().json(XApiMapper::to_api(entity)))`.
- Lists return a bare JSON array with `.append_header(("x-total-count", total))`. Deletes and actions return `DefaultHttpResponseDTO { code: 200, message, error: false }`.
- `#[utoipa::path]` with the method, full path (including scope), `tag`, `params`, `request_body`, every response with `body = ...`, and `security(("authorization" = []))`; add the handler to `ApiDoc.paths` and its DTOs to `components(schemas(...))`.

**Services**:
- `pub struct XService<'a> { repository: &'a dyn XRepositoryAbstract, /* other &'a deps */ }` + `pub fn new(...)`. Dependencies are borrowed from `GeneralModules`; services hold no state.
- Methods are `async` and return `Result<T, ApiError>`. List methods take the `HttpRequest` and call `query_transform(request.query_string())`; offer a `query_<x>(QueryRequest, &Claims)` variant for other services and jobs.
- Tenancy: never trust a company code from the body or path. `claim.scoped_company_code(requested)`: superadmin may pick (None = all companies), everyone else is pinned to their claims. Pass the scope to the repository.
- Map repository errors with `ErrorHandling::repository_error("<fixed message>", e)` (404 / 409 / 400 / 500 by error kind); validation failures with `ErrorHandling::application_error(...)` (400). Never put `e.to_string()` of a DB or upstream error in the client message.
- Side effects that must not fail the request (audit logs, notifications) go through `actix_web::rt::spawn` with a cloned `web::Data<GeneralModules>`.

**Repositories**:
- `#[derive(Clone)] pub struct XRepository { pub pool: DbPool }`, built once in `lib.rs` with `pool.clone()`.
- Every method: `run_blocking(&self.pool, move |conn| { ...diesel...; Ok(...) }).await`. Move owned inputs into the closure; `?` converts Diesel errors into `DbError`.
- Lists: `table.select(X::as_select()).into_boxed()`, optional `.filter(company_code.eq(..))`, then `.paginate(&query, X_QUERY_COLUMNS)?.load_and_count_pages::<X>(conn)?`.
- Inserts/updates use `.returning(X::as_returning()).get_result(conn)?`; updates and deletes filter by id **and** tenant; zero deleted rows → `DieselError::NotFound`.
- Multi-statement writes: `conn.transaction::<_, DbError, _>(|conn| { ... })`; chunk bulk inserts (≤ 1000 rows).

**Entities and mappers**: `X` (row) / `NewX<'a>` (insert, borrows `&'a str`) / `XEntity` (domain, id named
`x_id`). `XDbMapper` implements `DbMapper<XEntity, X>`; `XApiMapper` implements `ApiMapper<XEntity, XResponseDTO, XDTO>`
(its `to_entity` is `unimplemented!()` when creates go through `NewX`).

**DTOs**: `#[derive(Serialize, Deserialize, Debug, ToSchema)]` + `#[serde(rename_all = "camelCase")]`. Request
DTOs carry only client-settable fields (`companyCode` optional, honoured for superadmin only); enums such
as `StatusEnum` instead of free strings; `Option<T>` for optional/nullable fields.

**SQL and migrations**: tables snake_case plural (`products`), columns snake_case. Every table:
`id serial4` (or `bigserial`) primary key `CONSTRAINT <table>_pk`, `company_code varchar NOT NULL` for
tenant data, `status varchar NOT NULL DEFAULT 'active'` with a `CHECK`, `created_by`, `created_at
timestamptz NOT NULL DEFAULT NOW()`, `updated_at timestamptz` + `SELECT diesel_manage_updated_at('<table>');`.
Constraints `<table>_unique`, `<table>_<col>_check`; indexes `idx_<table>_<cols>`. Use `IF NOT EXISTS`,
`NOT NULL` wherever the Rust field isn't `Option`, and `timestamptz` rather than `timestamp`.

**Config**: `ConfigService {}.get_env("KEY")` (required, panics with `KEY must be set`) or
`get_env_or("KEY", "default")`. Boot-time values are read in `run()` so a missing one fails at startup.

## Style

- `cargo fmt` with the starter's `rustfmt.toml` (`max_width = 120`); `cargo clippy --all-targets` clean. Don't add `#![allow(unused)]` / `#![allow(dead_code)]` at the crate root.
- Imports: one `use` per module path, grouped std → external crates → `crate::`, separated by blank lines. Import `crate::...` by full path (not `super::`).
- Naming: `XService`, `XRepository`, `XRepositoryAbstract`, `XDbMapper`, `XApiMapper`, `XResponseDTO` / `XDTO` / `UpdateXDTO` / `DeleteXDTO`, `<x>_controller.rs`, handlers `get_xs`, `get_x`, `create_x`, `update_x`, `delete_x`.
- Result variables are named for the step (`get_data`, `insert`, `update`, `delete`, `results`) and matched with `match ... { Ok(r) => Ok(r), Err(e) => Err(ErrorHandling::...) }` or `map_err`.
- Log with `log::{info,warn,error}!`, never `println!`. Messages are lowercase and include context (`"cannot cache machine token: {}"`).
- Comments are sparse and explain why. Match the language of the surrounding code.
- Commits use Conventional Commits (`feat: …`, `fix: …`, `chore: …`); branches `feature/…`, `hotfix/…`.

## Gotchas

- **Never splice query-string keys or values into SQL.** Lists go through `Paginate` with a column whitelist; values are typed bind parameters. Older projects have a `query_db_filter` that formats `key = 'value'` into the SQL, which is injectable: replace it with this skill's `utils/filter.rs` + `utils/diesel_pagination.rs` before adding a list endpoint there.
- Diesel is blocking. A Diesel call made directly inside an `async fn` blocks an actix worker thread for its whole duration; always use `run_blocking`.
- No `.unwrap()` / `.expect()` on anything from the request (body, headers, query, path, upload names). A panic kills the request with a bare 500 and no log context.
- Keep repository traits `Send + Sync` with `#[async_trait]`. `#[async_trait(?Send)]` makes every service future `!Send`, so cron jobs (`Job::new_async` requires `Send`) and `tokio::spawn` can't call them.
- Don't hold a `std::sync::Mutex` guard across `.await`. In-process state (a `Mutex<HashMap>` in `GeneralModules`) is per replica and lost on restart; anything shared across replicas belongs in Redis or Postgres.
- `Queryable` maps by position. `schema.rs` column order must match the struct unless you select with `X::as_select()` (do). The pagination wrapper is `SELECT * ... FROM (<inner>) t`, so a joined inner query must alias duplicate column names, or `t."id"` becomes ambiguous.
- Regenerate `schema.rs` from the database (`diesel migration run` / `diesel print-schema`); never hand-write it. A hand-written `VarChar` over a nullable column fails at runtime with "Unexpected null for non-null column".
- `embed_migrations!` reads `migrations/` at compile time: the Dockerfile must `COPY migrations` before `cargo build`. A failed migration must stop startup (the starter returns the error from `run()`).
- Write a real `down.sql` for every migration, and run `diesel migration redo` to prove it.
- Reuse the one pool; don't build a second pool for migrations or per request. Reuse the one `reqwest::Client` and the one redis `ConnectionManager` from `GeneralModules`.
- Never set `Content-Type` on a multipart request (reqwest adds the boundary). reqwest 0.13 has no `.form()` without a feature flag; `RequestService` url-encodes form payloads itself.
- Map every status, not just 400/401/403: `CustomErrorResponse::map_io_error` uses `StatusCode::from_u16`, and 5xx responses get a generic message. A `match` that only knows 400/401/403 turns 404, 409 and 502 into 500.
- Register specific routes before parameterised ones in `routes()` (`/import` before `/{id}`), and the `SwaggerUi` catch-all `/api/{_:.*}` after every scope. Swagger UI is at `/api/` (trailing slash).
- The `#[utoipa::path(path = ...)]` must include the scope prefix; utoipa doesn't know about `web::scope`.
- A route missing from the gateway spec is blocked in production. The gateway header is trusted blindly, so the service must only be reachable through the gateway (ClusterIP service, internal ingress).
- Verify JWTs with the issuer's public key (`JWKS_N` / `JWKS_E`, RS256) and strip the `Bearer ` prefix. Never hardcode a secret literal.
- Behind a load balancer `peer_addr()` is the proxy; use `client_ip(&request)` (`realip_remote_addr`).
- Strip directories from uploaded file names (`multipart_to_files` does); a raw `../` name writes outside the upload folder.
- Cron expressions have six fields, seconds first. Every replica runs every job; guard run-once jobs with a redis lock (`SET key value NX EX ttl`).
- `COUNT(*) OVER ()` is computed on the returned rows, so a page past the end reports `x-total-count: 0`.
- CORS must expose `x-total-count` (the starter does) or browsers can't read it.
- Don't commit real credentials in `.env.example`, `docker-compose.yml` or the Kubernetes manifest. Use placeholders, and put secrets in a Kubernetes Secret (`envFrom.secretRef`).
- `includes` / `contains` filters do `CAST(col AS TEXT) LIKE`, which can't use a btree index; add a `pg_trgm` GIN index if a large table is searched that way.
