# AGENTS.md

Guidance for AI coding agents (VS Code Copilot, Cline, Cursor, etc.) using
DeepSeek in this repository.

## Golden rules

1. This is a Rust workspace that is built with Cargo. Always build/test after
   code changes before reporting them as done.
2. Do not edit generated files (e.g. `Cargo.lock`, `target/`) or SQL
   migrations unless the task explicitly requires it.
3. Preserve the clean architecture:

   `src/api` -> `src/application` -> `src/domain` -> `src/infrastructure`

   Do not introduce reverse dependencies.
4. Prefer existing patterns and helper functions over inventing new ones.
5. Run `cargo fmt --all` after every Rust edit before finishing a task.
6. The README still mentions Diesel and a `src/db`/`src/rest` layout. That is
   outdated. The real code uses `sqlx` and the layout below.

## How to use the VS Code tools

The workspace defines reusable tasks in `.vscode/tasks.json`.
Prefer them over raw shell commands so results are standardized and captured
by VS Code (menu: **Terminal → Run Task…**).

| VS Code task           | Exact equivalent command                                             | Purpose                       |
| ---------------------- | -------------------------------------------------------------------- | ----------------------------- |
| `rust: build`          | `cargo build`                                                        | Debug build                   |
| `rust: check`          | `cargo check`                                                        | Fast type/borrow check        |
| `rust: test`           | `cargo test`                                                         | Run all tests                 |
| `rust: test api`       | `cargo test --test api`                                              | Run API integration tests     |
| `rust: format check`   | `cargo fmt --all -- --check`                                         | Verify formatting (read-only) |
| `rust: clippy`         | `cargo clippy -- -D warnings`                                        | Lint, deny warnings           |
| `rust: run`            | `cargo run`                                                          | Start the API (port 3000)     |
| `infra: postgres up`   | `docker compose -f tests/infra/docker-compose.yaml up -d --wait`     | Start test infrastructure     |
| `infra: postgres down` | `docker compose -f tests/infra/docker-compose.yaml down`             | Stop test infrastructure      |

If a VS Code task cannot be used (for example in a non-VS Code environment),
use the exact equivalent command from the table.

## Commands

- Format code before finishing Rust work:
  `cargo fmt --all`
- Lint with warnings denied:
  `cargo clippy --all-targets --all-features -- -D warnings`
- Build:
  `cargo build`
- Run all tests:
  `cargo test`
- Run only the API integration test suite:
  `cargo test --test api -- --nocapture`
- Start the server:
  `cargo run`
  - Listens on `http://localhost:3000`
  - Health endpoint: `GET /health`
- Database migrations are plain SQL files in `migrations/` and are managed by
  `sqlx` (not Diesel). Do not create Diesel-style migration folders.

## Project layout

```
src/
  main.rs                   # Server setup and middleware configuration
  error.rs                  # AppError + error mapping
  api/                      # HTTP layer: handlers, DTOs, auth, routes
  application/              # Business logic / orchestration
  domain/                   # Domain types
  infrastructure/           # DB (sqlx), mail, external integrations
  mail/                     # Email templates + SMTP worker
migrations/                 # sqlx SQL migrations (*.up.sql / *.down.sql)
tests/api/                  # Integration tests, entry point: tests/api/mod.rs
email_templates/            # Email body/subject templates
openapi/swagger.yml         # OpenAPI 3.0 specification
```

## Integration test notes

- Test infrastructure is started with the `docker compose` command above.
- The test binary is defined in `Cargo.toml` as `name = "api"` with path
  `tests/api/mod.rs`.
- Tests require a running API instance and PostgreSQL. In CI, environment
  variables such as `DATABASE_URL`, `SMTP_HOST`, `EMAIL_TEMPLATES_DIR`,
  `TEAM_DEEPLINK_BASE_URL`, and `ADMIN_TEAM_LINK_BASE_URL` are set explicitly;
  mirror these when running tests locally.
- To debug tests, prefer:
  `cargo test --test api -- --nocapture`

## Style and conventions

- Rust edition 2021.
- Errors are centralized via `src/error.rs` (`thiserror` + `anyhow`).
- Request/response DTOs live in `src/api/models.rs` and are validated with the
  `validator` crate.
- Authentication uses JWT (`jsonwebtoken`) with Auth0 JWKS caching.
- Format with `cargo fmt`, lint with `cargo clippy`. Match surrounding code
  conventions instead of introducing different formatting or error handling.