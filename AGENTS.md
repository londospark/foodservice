# Project Overview
This project is a microservice architecture built with Rust, utilizing Docker Compose for orchestration.

## Architecture
The system consists of three main services:
1. **`postgres`**: A PostgreSQL 18+ database running on port 5432.
2. **`inventory`**: The core inventory service, built using a Rust binary, accessible on port 3001. It relies on the database for persistent storage.
3. **`gateway`**: An API Gateway/entrypoint service, built using a Rust binary, accessible on port 3000. It routes client requests to the `inventory` service.

## Environment
*   **Database:** Requires PostgreSQL 18+
*   **Database URL (Example):** `postgres://postgres:postgres@127.0.0.1:5432/foodservice` (Defined in `.env.example`)

## Source Code
*   **Core Codebase:** The main source logic and crates reside in the `binaries/` and `crates/` folders.
*   **Testing & Setup:** The `xtask/` directory is used for project setup and testing helpers, following common Rust patterns. Services are built using Dockerfiles located in `binaries/inventory_bin/` and `binaries/gateway_bin/`.

## Local Development and Testing Guide

### Environment Setup
To run tests locally, you need to set up the environment variables and the Docker Compose services:
1. **Create `.env` file:** Copy the example file (`.env.example`) to `.env` and fill in the necessary configuration (e.g., the `DATABASE_URL`).
2. **Run Docker Compose:** Execute the `compose.yaml` to bring up the `postgres`, `inventory`, and `gateway` services.

### Running Tests
Once the environment (Docker Compose and `.env` file) is set up, testing is straightforward. Simply run `cargo test` to execute the tests located in the appropriate test directories.