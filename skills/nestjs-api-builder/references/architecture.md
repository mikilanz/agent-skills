# Architecture reference

## Layers

| Layer | Folder | Responsibility | May import |
|---|---|---|---|
| Wiring | `src/application` | Nest modules, guards, pipes, interceptors, decorators, error classes, custom validators, middleware, health indicators | everything |
| Config | `src/config` | `ConfigService extends NestConfigService` with typed getters, Joi `validate()`, `configService` singleton for non-DI code (migrations) | — |
| Delivery | `src/delivery` | Controllers, DTOs, query classes, interfaces, enums: the API contract | use-case, data-provider, application |
| Use case | `src/use-case` | Services (business logic), CQRS commands/events/models/handlers, infra clients (cache, kafka, outbound HTTP) | data-provider, delivery (dto/enums/interfaces), utils, config |
| Data | `src/data-provider/entities` | TypeORM entities, request `Context` | delivery enums/dto (sparingly) |
| Persistence hooks | `src/subscribers` | `EntitySubscriberInterface` audit trail | use-case |
| Schema | `src/migrations` | Raw-SQL `MigrationInterface` classes, plus exported constraint-name constants | config singleton |
| Shared | `src/utils` | Pure helpers (`*.util.ts`), `constants/`, `enum/` (infra enums like `FilterCriteria`) | delivery dto/interfaces/enums |

## Bootstrap (`src/main.ts`)

1. `NestFactory.create<NestFastifyApplication>(AppModule, new FastifyAdapter({ logger: false }), { logger: ['log', 'error'] })`
2. Global pipes: `GeneralSanitizationPipe` (trims all strings), then `ValidationPipe({ transform: true })`.
3. Global interceptors: `ClassSerializerInterceptor`, `LoggingInterceptor` (timing), `ErrorsInterceptor`.
4. `enableCors({ origin: true, credentials: true })`, `enableVersioning({ type: VersioningType.URI })`.
5. `useContainer(app.select(AppModule), { fallbackOnErrors: true })` so class-validator constraints can inject providers.
6. Swagger at `/api`, versioned with `configService.getAppVersion()`.
7. `app.listen(port, '0.0.0.0')`.

`AppModule` imports `GeneralModule` (Config, TypeOrm, Http, Cache, Kafka, Throttler, Health, `CqrsModule.forRoot()`)
and then every feature module. It spreads the CQRS handler arrays into `providers` and applies `ContextMiddleware` to all routes.

## Auth model

The service sits behind an API gateway (Google Cloud Endpoints / API Gateway / similar) that verifies the JWT
and forwards the claims as a base64-encoded JSON header: `x-apigateway-api-userinfo` (or `x-endpoint-api-userinfo`).

- `AuthGuard` checks that the header is present.
- `@ClaimInfo()` decodes it into `Claim` (`companyCode`, `role`, `username`, `email`, `sub`, …). An invalid header returns 401.
- `ClaimRoleValidationPipe([roles])` returns 401 when the caller's role isn't listed.
- Roles: `superadmin`, `admin`, `developer`, `user`, `employee`. Typical sets: writes `[SUPERADMIN]`, tenant reads `[SUPERADMIN, ADMIN, EMPLOYEE]`.
- Tenancy: non-superadmins are always scoped to `claim.companyCode`. A superadmin may target any tenant through the `/:companyCode` path param.

If the service isn't behind a gateway, add a `JwtGuard` (`@nestjs/jwt` + `jwks-rsa`) that verifies `Authorization: Bearer` and sets `request.user`.
Keep `@ClaimInfo` reading from that instead.

## Request lifecycle

1. `ContextMiddleware`: `Context.run(req, next)` (AsyncLocalStorage).
2. Guard → global pipes → param decorators and their pipes.
3. Controller → service (Observable).
4. `HeadersInterceptor` (list routes) copies `res.headers` onto the reply and returns `res.data`.
5. `ClassSerializerInterceptor`, then `LoggingInterceptor`.
6. On error, `ErrorsInterceptor` sets the HTTP status and returns the error body.

## Response contract

| Case | Body | Headers |
|---|---|---|
| Single entity | the entity | — |
| List | array (`data`) | `x-total-count` |
| Action / delete | `{ statusCode, messages, ok: true, result? }` (`DefaultHttpResponseDTO`) | — |
| Error | `{ statusCode, errors, messages: ['Error ID: <timestamp>'], ok: false }` | status = `statusCode` |

The error id is also logged, so a client report can be matched to the log line.

## Query-string contract (`QueryFilterPagination`)

```
GET /widget/all?page=0&size=20&sort=createdAt,desc&name.contains=foo&type.in=basic,premium&createdAt.between=2026-01-01,2026-02-01
```

- `page` is 0-based. `size` defaults to `DEFAULT_NUMBER_OF_SIZE` (20).
- `sort=<field>,<asc|desc>` (repeatable). The field must be `@Expose()`d. A badly formatted value returns 400.
- `<field>.<criteria>=<value>`, where criteria come from `FilterCriteria`: `equal, notEqual, greatherThanOrEqual, lessThanOrEqual, greatherThan, lessThan, includes, contains, in (comma list), between (two dates)`. The `greather` spelling is part of the contract. Keep it.
- Nested relation keys work: `account.companyCode.equal=abc` (expose `account` on the query class and make sure the relation exists).
- `expose` / `exclude` are parsed into lists for services that want field projection.
- In the service: `typeORMQuery(query)` → `{ where, order, skip, take }`. Use `filterDataORM(query.filter)` for `where` only.
  Merge extra conditions with `options.where = { ...options.where, companyCode }`.

## Global infrastructure (injectable anywhere)

| Service | Purpose |
|---|---|
| `ConfigService` | `get('ENV')` + typed getters (`getDatabaseConfig`, `getRedisConfig`, `getKafkaConfig`, …) |
| `HttpService` | axios wrapped as Observables, with an optional proxy agent |
| `CacheService` | Redis `get<T>`, `set(key, value, ttl?)`, `delete`, `exists`, `pingRedis` (JSON-serialized) |
| `KafkaService` | `sendMessage(topic, string)`, `sendMessagesBatch(topic, objects)`. Usually called through `SendKafkaMessageCommand` |
| `CommandBus` | dispatches CQRS commands |

Projects typically add more global clients the same way: their own `@Global()` module exporting one
service. Common ones are an auth-token client for service-to-service calls, an activity-log client used by subscribers, and a
chat/webhook notifier for operational alerts.

## Deployment shape

- Multi-stage Dockerfile: install with pnpm, `pnpm run build`, run `node dist/main`, expose 8080.
- Health at `GET /health` (Terminus: database + Redis, plus pings to critical upstreams).
- CI builds the image and pushes it to a registry. Gateway specs are deployed separately when routes change.
