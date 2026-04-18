# foodservice

`foodservice` is a Rust workspace for tracking what food is available in the house, organized as small services plus shared library crates. Today the project has a runnable GraphQL gateway, a PostgreSQL-backed inventory crate, and a placeholder inventory service binary that has not been wired into a network service yet.

## Workspace layout

| Path | Purpose |
| --- | --- |
| `crates/gateway` | GraphQL schema, resolvers, and gateway-focused tests |
| `crates/inventory` | Inventory persistence logic, SQL migrations, and database tests |
| `services/gateway-svc` | Axum binary that serves the GraphQL endpoint on port `3000` |
| `services/inventory-svc` | Placeholder binary for a future standalone inventory service |

## Patterns currently used

### 1. Cargo workspace with shared dependencies

The repository uses a top-level Cargo workspace to keep versions and shared dependencies centralized. Common crates like `axum`, `async-graphql`, `tokio`, `sqlx`, and `uuid` are declared once in the root `Cargo.toml` and then reused by member crates.

### 2. Library crates for business logic, service crates for delivery

The codebase separates domain logic from transport/runtime concerns:

- `crates/*` holds reusable logic.
- `services/*` holds binaries that turn that logic into long-running processes.

That pattern is already visible in the gateway path: the GraphQL schema lives in `crates/gateway`, while the HTTP server lives in `services/gateway-svc`.

### 3. GraphQL at the edge

The current external interface is GraphQL:

- `async-graphql` defines the schema, queries, and mutations.
- `axum` hosts the HTTP server.
- `async-graphql-axum` wires the schema into the router.
- The gateway serves an embedded Apollo sandbox at `/` for interactive testing.

At the moment the gateway uses in-memory example data for reads and mutations rather than calling the inventory crate.

### 4. PostgreSQL-backed inventory logic

The inventory crate uses `sqlx` with PostgreSQL. Its current behavior is intentionally small and focused:

- `food_items` is the backing table.
- `name` is unique.
- adding the same item twice increments quantity instead of creating duplicates.

The schema migration uses `uuidv7()` as the default ID generator, which means **PostgreSQL 18 or newer is required** for the provided setup.

### 5. Tests close to the code they verify

The workspace already leans on crate-local tests:

- `crates/gateway` has GraphQL tests that execute the schema directly.
- `crates/inventory` uses `#[sqlx::test]` to run against temporary PostgreSQL databases with migrations applied.

This makes `cargo test` the main validation path for the whole workspace.

## Requirements

- Rust toolchain with `cargo`
- Docker with Docker Compose support
- PostgreSQL **18+** if you are not using Docker

## Environment

Copy the example environment file before running database-backed tests:

```bash
cp .env.example .env
```

The workspace currently relies on `DATABASE_URL` for the inventory crate and SQLx-powered tests.

## Start PostgreSQL with Docker

A ready-to-use Compose file is included at the repository root. Start PostgreSQL 18 with:

```bash
docker compose up -d postgres
```

Stop it with:

```bash
docker compose down
```

If you want to remove the local database volume as well:

```bash
docker compose down -v
```

## Getting the project running

1. Copy the example env file.
2. Start PostgreSQL: `docker compose up -d postgres`
3. Run the test suite: `cargo test`
4. Start the gateway service: `cargo run -p gateway-svc`
5. Open `http://127.0.0.1:3000`

## What runs today

### GraphQL gateway

Run:

```bash
cargo run -p gateway-svc
```

Then open `http://127.0.0.1:3000` and try:

```graphql
query {
  health
  listFood {
    id
    name
    qty
  }
}
```

Or:

```graphql
mutation {
  addFood(name: "Milk", qty: 2) {
    id
    name
    qty
  }
}
```

### Inventory crate

The inventory code is exercised through tests right now rather than a standalone HTTP service:

```bash
cargo test -p inventory_svc --lib
```

Those tests require a reachable PostgreSQL 18+ instance and use the `DATABASE_URL` from your environment.

## Database notes

- Schema files live under `crates/inventory_svc/migrations`
- The current table is `food_items`
- Repeated inserts for the same food name are merged by increasing quantity
- PostgreSQL 18+ is required because the migration uses `uuidv7()`

## Current limitations

- `services/inventory-svc` is still a placeholder binary
- The gateway does not yet call the PostgreSQL-backed inventory crate
- There is no end-to-end service-to-service flow yet; the repository currently shows the architectural direction and the first service boundaries
