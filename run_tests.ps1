# Check if .env file exists, if not create a default one
if (-Not (Test-Path ".\.env")) {
    echo "Creating default .env file..."
    New-Item -Path ".\.env" -ItemType File
}

# Start Docker Compose
echo "Starting Docker Compose..."
docker-compose up -d

# Wait for services to be ready
Start-Sleep -Seconds 30

# Run tests with coverage
echo "Running tests with coverage..."
cargo llvm-cov --workspace --ignore-filename-regex '.*\.d' --lcov --output-path lcov.info

# Check if the lcov.info file was generated successfully
if (Test-Path ".\lcov.info") {
    echo "Code coverage report generated successfully."
} else {
    echo "Failed to generate code coverage report."
    exit 1
}
