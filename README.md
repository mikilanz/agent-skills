# Agent Skills

A collection of [Agent Skills](https://www.skills.sh/) for coding agents such as Claude Code, Cursor, Codex and others.
Each skill lives in its own folder under [`skills/`](skills/) and can be installed or cloned on its own.

## Skills

| Skill | Description |
|---|---|
| [nestjs-api-builder](skills/nestjs-api-builder/) | Build or extend a layered NestJS REST API: Fastify, TypeORM + Postgres with raw-SQL migrations, RxJS services, Swagger, CQRS, Redis, Kafka and gateway-claims auth. Includes a type-checked starter project. |
| [golang-api-builder](skills/golang-api-builder/) | Build or extend a layered Go REST API: Gin, pgx/v5 + sqlc + golang-migrate on Postgres, Redis cache-aside, slog, swaggo Swagger, gateway-claims auth with role guards and tenant scoping, and a dynamic filter/sort/pagination contract. Includes a starter project that builds and passes its tests. |
| [rust-api-builder](skills/rust-api-builder/) | Build or extend a layered Rust REST API: actix-web 4, Diesel 2 + r2d2 on Postgres with embedded migrations, utoipa Swagger, mockable repository traits, gateway-claims auth with role guards and tenant scoping, a whitelisted filter/sort/pagination contract, Redis, reqwest integrations and cron jobs. Includes a starter project that builds and passes its tests. |

## Install with the skills CLI

```bash
# pick from all skills in this repo
npx skills add mikilanz/agent-skills

# install a single skill
npx skills add mikilanz/agent-skills --skill nestjs-api-builder

# or point at the skill folder directly
npx skills add https://github.com/mikilanz/agent-skills/tree/main/skills/nestjs-api-builder

# list the skills without installing
npx skills add mikilanz/agent-skills --list
```

Add `-a claude-code` (or another agent) to target a specific agent, and `-g` to install globally.

## Get one skill folder without the rest of the repo

Copy only the folder, with no git history:

```bash
npx degit mikilanz/agent-skills/skills/nestjs-api-builder nestjs-api-builder
```

Or use a sparse checkout to keep it as a git working copy you can pull updates into:

```bash
git clone --depth 1 --filter=blob:none --sparse https://github.com/mikilanz/agent-skills.git
cd agent-skills
git sparse-checkout set skills/nestjs-api-builder
```

To install it manually for Claude Code, copy the folder into `~/.claude/skills/` (all projects) or `.claude/skills/` (one project).

## Adding a skill

1. Create `skills/<skill-name>/SKILL.md`. The folder name must match the `name` field.
2. Start the file with YAML frontmatter:

   ```markdown
   ---
   name: skill-name
   description: What the skill does and when the agent should use it.
   ---
   ```

   `name` is lowercase with hyphens. Keep `description` under 1024 characters.
3. Put supporting files next to it: `references/` for docs the agent reads on demand, `assets/` for templates and files it copies, and `scripts/` for executables.
4. Keep the skill self-contained. Don't reference files outside its own folder, so it still works when installed or cloned on its own.
5. Add a row to the table above.
