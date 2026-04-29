# Agent Instructions for Foodservice Repository

## Repository Overview
This is a Rust-based microservices repository. It uses `cargo` for build management and `xtask` for custom automation.

## Development Workflow
- **Environment**: Use Docker Compose for local infrastructure (Postgres).
- **Testing**: 
  - Run tests for a specific crate: `cargo test -p <crate_name>`
  - Set database URL: `$env:DATABASE_URL="postgres://postgres:postgres@localhost:5432/foodservice"`
  - Use `cargo-llvm-cov` for coverage analysis.
- **Automation**: Use `xtask` for development tasks.

## Key Conventions
- **SQL**: Ensure all SQL queries in `sqlx` macros are syntactically correct (e.g., use `RETURNING`, not `RETting`).
- **Error Handling**: Use `anyhow::Result` for application-level error handling.
- **Naming**: Follow standard Rust naming conventions (snake_case for functions/variables, PascalCase for types).

## Common Commands
- `cargo test -p inventory_svc`: Run inventory service tests.
- `cargo test -p <crate>`: Run tests for other crates.
- `cargo build`: Build the entire workspace.
