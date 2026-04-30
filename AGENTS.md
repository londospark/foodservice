export DATABASE_URL="postgres://postgres:postgres@localhost:5432/foodservice"

# Code Coverage Setup

To set up code coverage for the project, follow these steps:

1. **Install `cargo-llvm-cov`**:
   ```sh
   cargo install cargo-llvm-cov
   ```

2. **Run tests with coverage**:
   ```sh
   cargo llvm-cov --workspace --ignore-filename-regex '.*\.d' --lcov --output-path lcov.info
   ```

This will generate a `lcov.info` file in the current directory, which you can use to analyze code coverage.
