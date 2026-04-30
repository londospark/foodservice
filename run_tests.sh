#!/bin/bash

# Install cargo-llvm-cov if not already installed
if ! command -v cargo-llvm-cov &> /dev/null; then
    echo "Installing cargo-llvm-cov..."
    cargo install cargo-llvm-cov
fi

# Run tests with coverage
echo "Running tests with coverage..."
cargo llvm-cov --workspace --ignore-filename-regex '.*\.d' --lcov --output-path lcov.info

# Check if the lcov.info file was generated successfully
if [ -f lcov.info ]; then
    echo "Code coverage report generated successfully."
else
    echo "Failed to generate code coverage report."
    exit 1
fi
