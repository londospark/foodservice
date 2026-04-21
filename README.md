# foodservice

`foodservice` is a Rust workspace for tracking household food inventory. Right now it contains:

- a runnable GraphQL gateway on port `3000`
- a PostgreSQL-backed inventory service crate
- a small placeholder inventory HTTP binary on port `3001`
- shared inventory DTOs and traits for wiring the services together later

## Workspace layout

| Path | Purpose |
| --- | --- |
| `crates/gateway` | GraphQL schema, resolvers, and gateway tests |
| `crates/inventory` | Shared inventory DTOs, trait definitions, and a placeholder client |
| `crates/inventory_svc` | PostgreSQL-backed `InventoryService` implementation and SQLx tests |
| `binaries/gateway_bin` | Axum server that exposes the GraphQL gateway on port `3000` |
| `binaries/inventory_bin` | Placeholder Axum HTTP server on port `3001` |
| `xtask` | Workspace automation crate |

## Current architecture

### GraphQL gateway

The external entry point today is the GraphQL gateway in `binaries/gateway_bin`.

- `async-graphql` defines the schema in `crates/gateway`
- `axum` serves the endpoint
- Apollo Sandbox is embedded at `/`

The gateway currently returns in-memory data for `listFood`, `addFood`, and `deleteFood`. It does not yet call the inventory client or the PostgreSQL-backed inventory service crate.

### Inventory domain

The shared inventory domain lives in `crates/inventory`.

- `dto.rs` defines `AddFoodItem` and `FoodItem`
- `traits.rs` defines the `InventoryService` trait
- `client.rs` contains a placeholder `reqwest`-based client implementation that is not yet wired to a real service

### PostgreSQL-backed inventory service crate

`crates/inventory_svc` contains the real persistence logic so far.

- it implements `InventoryService` against PostgreSQL
- it stores data in the `food_items` table
- repeated inserts for the same `name` increase `quantity` instead of creating duplicate rows
- the migration uses `uuidv7()`, so PostgreSQL `18+` is required

This is the most complete part of the system today, but it is still a library crate rather than a networked microservice.

### Inventory HTTP binary

`binaries/inventory_bin` starts an Axum server on port `3001`, but it is still a stub. It serves simple text responses and is not connected to PostgreSQL or `crates/inventory_svc`.

## Requirements

- Rust toolchain with `cargo`
- Docker with Docker Compose support
- PostgreSQL `18+` if you are not using Docker

## Local database

A ready-to-use Compose file is included at the repository root:

```bash
docker compose up -d postgres
```

Stop it with:

```bash
docker compose down
```

Remove the local database volume as well:

```bash
docker compose down -v
```

## Environment

Database-backed tests use `DATABASE_URL`.

Example:

```bash
export DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/foodservice
```

## Getting started

1. Start PostgreSQL: `docker compose up -d postgres`
2. Export `DATABASE_URL`
3. Run the workspace tests: `cargo test`
4. Start the GraphQL gateway: `cargo run -p gateway_bin`
5. Open `http://127.0.0.1:3000`

If you want to see the placeholder inventory HTTP server as well:

```bash
cargo run -p inventory_bin
```

Then open `http://127.0.0.1:3001`.

## What runs today

### Gateway example

Run:

```bash
cargo run -p gateway_bin
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

### Inventory service crate tests

The PostgreSQL-backed inventory logic is currently exercised through tests:

```bash
cargo test -p inventory_svc
```

Those tests require a reachable PostgreSQL `18+` instance and `DATABASE_URL` in the environment.

## Database notes

- migration files live under `crates/inventory_svc/migrations`
- the current table is `food_items`
- `name` is unique
- repeated inserts for the same food merge by increasing quantity
- PostgreSQL `18+` is required because the migration uses `uuidv7()`

## Current limitations

- the gateway still uses in-memory data instead of calling inventory over a service boundary
- `crates/inventory/client.rs` is still a placeholder
- `binaries/inventory_bin` is not wired to PostgreSQL or `crates/inventory_svc`
- there is no end-to-end gateway -> inventory service flow yet
