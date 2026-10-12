# Scaffolding a new service

The skill ships a ready starter in `assets/starter/` (next to this `references/` folder). It contains all
the infrastructure: Fastify bootstrap, config + Joi, global modules, auth guard and claim decorator, role pipe,
query/filter/sort/pagination decorator and utilities, error/headers/logging interceptors, request context
(AsyncLocalStorage), base entity, Redis cache service, Kafka producer + CQRS command, health check, an init
migration, Dockerfile and docker-compose. It type-checks as shipped.

## Steps

1. **Copy the starter** into the target directory. Resolve the skill's own folder; it's wherever this file lives.
   ```bash
   cp -r <skill-dir>/assets/starter/. <target-dir>/
   ```
2. **Rename**: `package.json` `name`, the Swagger title in `src/main.ts`, and the image/service names in `docker-compose.yaml`.
3. **Install**: `pnpm install`. Use `npm`/`yarn` only if the user prefers them, and then adjust the Dockerfile.
4. **Trim unused infrastructure**. Ask the user which pieces they need if it's unclear:
   - No Redis: delete `use-case/cache/`, `application/modules/cache.module.ts` and `application/indicators/redis-health.indicator.ts`. Remove them from the barrels, from `GeneralModule` and `HealthModule`, from the health check, from `getRedisConfig` and from the Joi `REDIS_*` keys.
   - No Kafka: delete `use-case/kafka/`, `kafka.module.ts` and the `send-kafka-message` command/event/model/handlers. Remove the handler arrays from `AppModule`, `getKafkaConfig`, the Joi `KAFKA_*` keys and the `kafkajs` dependency.
   - No CQRS at all: also remove `CqrsModule.forRoot()` and the `use-case/cqrs/` folder.
   - Not behind an API gateway: replace `AuthGuard` and `ClaimInfo` with JWT verification (see architecture.md, "Auth model").
5. **Env**: `cp .env.example .env` and fill it in. `docker compose up -d db redis` gives a local Postgres and Redis.
6. **Verify**: `npx tsc --noEmit -p tsconfig.build.json`, then `pnpm start:dev`. Check `GET /health` and `GET /api` (Swagger).
7. **Add features** with the workflow in `SKILL.md` and the example in `templates.md`.
8. Initialize git and make the first commit (`chore: initial scaffold`) only if the user asks.

## What's in the starter

```
package.json  tsconfig.json  tsconfig.build.json  nest-cli.json  .prettierrc  .eslintrc.js
.env.example  .gitignore  Dockerfile  docker-compose.yaml
src/
  main.ts  app.module.ts
  config/            config.service.ts  custom-validation-function.ts  index.ts
  migrations/        1700000000000-Init.ts            (uuid-ossp + schema)
  application/
    modules/         config  typeorm  http  cache  kafka  health  general  index
    guards/          auth.guard.ts
    decorators/      claim.decorator.ts  query-pagination-param.decorator.ts
    pipes/           claim-role-validation  query-param-validation  general-sanitization
    interceptors/    errors  headers  logging
    middleware/      context.middleware.ts
    indicators/      redis-health.indicator.ts
    errors/          (empty barrel)
  data-provider/entities/  default.entity.ts  context.entity.ts
  delivery/
    dto/             default.dto.ts  custom-response.dto.ts  claim.dto.ts
    interfaces/      query-request.interface.ts
    enums/           sort-order.enum.ts
    query/           default.query.ts
    http/            health/health.controller.ts
  use-case/
    cache/cache.service.ts  kafka/kafka.service.ts
    cqrs/            send-kafka-message command, event, model, handlers + index arrays
  subscribers/       _index.ts (empty barrel)
  utils/             typeorm.util.ts  sort.util.ts  filter.util.ts  transform.util.ts
                     enum/filter.enum.ts  constants/general.constant.ts
```

The starter's Jest config maps `src/...` imports (`moduleNameMapper`) and uses the project root as `rootDir`, so
specs can use the same absolute imports as the app.

Don't put real secrets, webhook URLs or API keys in Joi defaults. Use `.required()` or an empty default.
