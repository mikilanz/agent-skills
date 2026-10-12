---
name: nestjs-api-builder
description: Build or extend a NestJS backend API in a layered house style — Fastify, TypeORM + Postgres with hand-written SQL migrations, RxJS Observable services, class-validator DTOs, Swagger, CQRS for async side effects, Redis, Kafka, Joi-validated env, gateway-claims auth with role pipes, and a standard query-string filter/sort/pagination contract. Use when creating a new NestJS API service, or when adding an endpoint, CRUD resource, entity, migration, DTO, query filter, module, CQRS command/event, outbound HTTP integration or env var to a NestJS project laid out as src/application, src/delivery, src/use-case, src/data-provider.
---

# NestJS API Builder

A layered NestJS architecture for REST APIs. Use it to scaffold a new service, or to extend one
that already follows it.

- **New project:** follow [references/new-project.md](references/new-project.md). It copies the ready-made starter in `assets/starter/`, which type-checks.
- **Existing project:** first read the closest existing feature end to end (controller → service → DTO → query → entity → module), then copy its shape. The project's own conventions override this skill wherever they differ.

References:
- [references/architecture.md](references/architecture.md): layers, request lifecycle, response and error contract, query-string filtering, global modules
- [references/templates.md](references/templates.md): a complete, type-checked example feature covering every file type, plus snippets for env vars, validators, errors and tests

## Stack

| Concern | Choice |
|---|---|
| Framework | NestJS 10 on **Fastify** (`@nestjs/platform-fastify`) |
| DB | TypeORM 0.3 + `pg`, `synchronize: false`, **raw-SQL migrations** |
| Validation | `class-validator` + `class-transformer`, global `ValidationPipe({ transform: true })` and a trim pipe |
| Env | `@nestjs/config` + **Joi** schema (`src/config/custom-validation-function.ts`) |
| Async style | **RxJS Observables** from services and controllers (`from(promise).pipe(...)`) |
| Docs | `@nestjs/swagger` at `/api`; every route and DTO field decorated |
| Side effects | `@nestjs/cqrs`: Command → AggregateRoot model → Event → EventsHandler |
| HTTP out | `@nestjs/axios` `HttpService`, optional `https-proxy-agent` |
| Cache | `redis` v4 client wrapped in `CacheService` |
| Messaging | `kafkajs` producer in `KafkaService`, sent via `SendKafkaMessageCommand` |
| Misc | `moment`, `uuid`, `@nestjs/terminus` health, `@nestjs/throttler` |
| Tooling | pnpm, Node 20+, Jest + ts-jest, ESLint + Prettier (`singleQuote`, `trailingComma: all`) |

Don't introduce alternatives (Express, Prisma, `synchronize: true`, async/await-only services,
cache-manager) unless the user asks.

## Folder map

```
src/
  main.ts                      Fastify bootstrap: global pipes and interceptors, CORS, Swagger, URI versioning
  app.module.ts                imports GeneralModule + feature modules; spreads CQRS handler arrays into providers; ContextMiddleware
  config/                      ConfigService (extends Nest's, typed getters), Joi validate(), `configService` singleton for migrations
  application/                 Nest wiring only, no business logic
    modules/<feature>.module.ts    + barrel modules/index.ts
    guards/ pipes/ interceptors/ decorators/ errors/ validators/ middleware/ indicators/
  data-provider/entities/      TypeORM entities extending DefaultEntity, request Context + barrel index.ts
  delivery/                    the API contract
    http/<feature>.controller.ts   + barrel http/index.ts
    dto/<feature>/<feature>.dto.ts
    query/<feature>.query.ts       @Expose() whitelist for filtering and sorting
    interfaces/<topic>.interface.ts
    enums/<topic>.enum.ts          + barrel enums/index.ts
  use-case/                    business logic
    <feature>/<feature>.service.ts + barrel use-case/index.ts
    cqrs/{commands,events}/{actions,handlers}/<area>/   cqrs/models/
  subscribers/                 TypeORM EntitySubscribers (audit trail) + barrel _index.ts
  migrations/<timestamp>-<PascalName>.ts
  utils/                       <name>.util.ts, constants/<topic>.constant.ts, enum/ (infra enums such as FilterCriteria)
```

**Hard rule:** never declare interfaces, enums, constants, DTOs or helpers inline in a service, handler
or controller. Put them in the matching folder above, and extend an existing topic file when one fits.

## Workflow: adding a resource or endpoint

Work through these in order and skip the steps that don't apply:

1. **Read a sibling feature** in the project.
2. **Migration** `migrations/<ts>-<Name>.ts`: raw SQL against `${configService.get('DB_SCHEMA')}.<table>`.
   `<ts>` must be **greater than the largest existing prefix**. Don't assume `Date.now()` is the largest, because projects sometimes have future-dated prefixes.
3. **Entity** extending `DefaultEntity`. Register it in `entities/index.ts` and in the TypeORM module (both `forRootAsync.entities` and `forFeature`).
4. **Enums / constants / interfaces** in their folders.
5. **DTOs**: `Create…DTO`, `Update…DTO` (with `uuid`), `Delete…DTO` (`uuid`), `BulkDelete…DTO` (`uuids`).
6. **Query class** extending `DefaultQuery`, with one `@Expose()` per filterable or sortable field.
7. **Service** returning Observables. Add it to the `use-case/index.ts` barrel.
8. **Controller**. Add it to the `delivery/http/index.ts` barrel.
9. **Module**. Add it to the `modules/index.ts` barrel and to `AppModule.imports`.
10. **Env vars**: Joi schema, `.env.example`, `docker-compose.yaml`, and optionally a typed `ConfigService` getter.
11. **Async side effects** (external APIs, messaging, notifications) go through CQRS, not inline in the request path.
12. **Audit trail** for user-editable tables: add a subscriber, provide it in the feature module and export it from `subscribers/_index.ts`.
13. **Gateway spec**: if the project deploys behind an API gateway with a static OpenAPI/Swagger file (Google Cloud Endpoints, API Gateway), new routes must also be added there, or they're blocked. Look for such a file and ask the user before editing it.
14. **Verify**: `npx tsc --noEmit -p tsconfig.build.json`, then the relevant `jest` spec.

## Conventions

**Controllers** are thin. They only do auth, role checks, tenant scoping and delegation:
- `@UseGuards(new AuthGuard())` (an instance, at class or method level), `@ApiTags('…')`, `@Controller('kebab-path')`.
- Each authenticated method has `@ApiHeader({ name: 'x-apigateway-api-userinfo', required: true })`, `@ApiOperation({ summary })`, and one or more `@ApiResponse({ status: HttpStatus.X, type, description })`.
- Role check as a param decorator: `@ClaimInfo(new ClaimRoleValidationPipe([ClaimRole.SUPERADMIN])) _: Claim`. Name the parameter `claim` when you use it.
- Tenant scoping: `dto.companyCode = claim?.role === ClaimRole.SUPERADMIN ? companyCode : claim?.companyCode;` and `dto.actionPerformedBy = claim?.username;`. Never trust a tenant id from the body.
- Route shape: `POST ''` (or `/:companyCode`), `GET '/all'` (paginated), `GET '/single/:uuid'`, `PUT ''` (uuid in body), `DELETE ''` (uuid in body), `DELETE '/bulk'` (uuids).
- Paginated list: `@UseInterceptors(HeadersInterceptor)` + `@QueryFilterPagination<XQuery>({ classConstructor: XQuery }, new QueryParamValidationPipe()) query: QueryRequest` returning `Observable<CustomResponseDTO<T[]>>`.
- The return type is always `Observable<…>`.

**Services**:
- `private readonly logger: Logger = new Logger(XService.name);`, and `@InjectRepository(XEntity) private readonly xRepository: Repository<XEntity>`.
- `from(this.repo.op(...)).pipe(map | switchMap | mergeMap, catchError((err) => throwError(() => err)))`.
- Not found: throw `NotFoundException(\`X with UUID "${uuid}" not found\`)` inside the operator.
- Unique violation: `err?.code === '23505' && err?.constraint?.includes(constraintName)` → `BadRequestException('Duplicate entry: …')`. Export the constraint name from the migration.
- List: `findAndCount(typeORMQuery(query))` → `{ data, headers: { 'x-total-count': count.toString() } }`.
- Delete: return `DefaultHttpResponseDTO({ statusCode: HttpStatus.OK, messages: 'success', ok: true })`.
- Parallel reads use `forkJoin`. Aggregates use `createQueryBuilder().getRawMany<T>()`.
- Side effects: `tap(saved => this.commandBus.execute(new XCommand(...)))` after the save.

**Errors**: the global `ErrorsInterceptor` returns
`{ statusCode, errors, messages: ['Error ID: <ts>'], ok: false }` with the matching HTTP status. Throw Nest's
built-in exceptions. A custom error class goes in `application/errors/` and only needs an interceptor branch for extra behavior such as alerting.

**DTOs**: `@ApiProperty()` on every field (`{ required: false }` for optional fields, `{ example: v4() }` for uuids,
`{ enum: Object.values(E) }` for enums) plus class-validator decorators. Fields set by the server
(`companyCode`, `actionPerformedBy`) have no decorators.

**Entities**: table names in snake_case and plural, column names in camelCase (double-quoted in SQL). Relations use an explicit
`xxxId` column + `@ManyToOne` + `@JoinColumn({ name: 'xxxId' })`. Use `timestamptz` for dates and `simple-array` for string lists.

**Migrations**: hand-written SQL with `IF NOT EXISTS` guards, `uuid` defaulting to `uuid_generate_v4()`,
`createdAt` / `updatedAt` as `TIMESTAMPTZ NOT NULL DEFAULT NOW()`, and constraints named `<table>_pk` / `<table>_<col>_unique`.
A `down` for create-table only logs a warning. A `down` for add-column drops the column.

## Style

- Imports: absolute `src/...` across layers. Relative `../dto/...` is fine inside `delivery/`.
- Barrels use `import { X } from './x';` then `export { X, ... };`, not `export * from`.
- 4-space indent and single quotes in new files. In an existing project, **match the file you're editing**, including CRLF line endings (`grep -lI $'\r' <file>`).
- Before running `lint` / `format` scripts that use `--fix` / `--write` over all of `src`, check whether the project is already formatted. If it isn't, they create a huge unrelated diff, so format only the files you touched (`npx prettier --write <files>`).
- Comments are sparse and explain *why*. Write them in the language the surrounding code uses.
- Commits use Conventional Commits: `feat: …`, `fix: …`, `chore: …`, `refactor: …`. Branches: `feature/…`, `hotfix/…`, `refactor/…`.
- Releases bump the `APP_VERSION` default in the Joi schema (`chore: version x.y.z`).

## Gotchas

- `QueryFilterPagination` only builds filters and sorts for keys `@Expose()`d on the query class. Others are silently dropped.
- `HeadersInterceptor` expects `{ data, headers }`. A bare array produces an empty body.
- Infra modules are `@Global()` (Config, TypeOrm, Http, Cache, Kafka, plus any shared clients), so their services can be injected anywhere. Feature modules export their service. Import the feature module rather than re-providing its service. Use `forwardRef` only for real cycles.
- `strictNullChecks` is off. Use optional chaining (`a?.b`) defensively.
- Migrations load from `dist/migrations/*.js` and run only when `DB_MIGRATION_RUNS_AUTO=true`, so a build must come first.
- TypeORM subscribers fire on `save()` / `remove()`, not on the `update()` / `delete()` shortcuts.
- `Context.getUser()` / `Context.getIPAddress()` (set by `ContextMiddleware`) give non-request code the caller. In an existing project, check that its `Context` uses `AsyncLocalStorage`. A static field leaks between concurrent requests.
- Check that `sortTransform` (writes `{ key, order }`) and `typeORMQuery` (reads it) agree on the field name. A mismatch makes every `?sort=` request throw 500. The starter's versions are consistent.
