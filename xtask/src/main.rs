use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};
use serde_json::{Value, json};

#[derive(Parser)]
#[command(
    name = "xtask",
    about = "A tool to manage the foodservice microservices infrastructure."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install project dependencies
    Deps,
    /// Setup docker compose
    ComposeUp,
    /// Run tests
    Test,
    /// Bring up the compose stack and run a small end-to-end smoke test
    Smoke,
}

struct ComposeGuard;

impl Drop for ComposeGuard {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["compose", "down"])
            .status();
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Deps => {
            println!("Installing project dependencies...");
            let status = Command::new("cargo")
                .arg("install")
                .arg("sqlx-cli")
                .status()
                .expect("Failed to install sqlx-cli");

            if !status.success() {
                eprintln!("Failed to install dependencies. Please check the error messages above.");
                std::process::exit(1);
            }
            println!("Dependencies installed successfully!");
        }
        Commands::Test => {
            println!("Running tests...");
            let status = Command::new("cargo")
                .args(["test", "--no-fail-fast", "--color", "always", "--workspace"])
                .status()
                .expect("Failed to run tests");

            if !status.success() {
                eprintln!("Tests failed. Please check the error messages above.");
                std::process::exit(1);
            }
            println!("All tests passed successfully!");
        }
        Commands::ComposeUp => {
            println!("Setting up Docker Compose environment...");
            let status = Command::new("docker")
                .args(["compose", "up", "-d"])
                .status()
                .expect("Failed to set up Docker Compose environment");

            if !status.success() {
                eprintln!(
                    "Failed to set up Docker Compose environment. Please check the error messages above."
                );
                std::process::exit(1);
            }
            println!("Docker Compose environment set up successfully!");
        }
        Commands::Smoke => {
            if let Err(err) = smoke().await {
                eprintln!("Smoke test failed: {err:#}");
                std::process::exit(1);
            }
        }
    }
}

async fn smoke() -> anyhow::Result<()> {
    println!("Starting compose stack...");
    let status = Command::new("docker")
        .args(["compose", "up", "-d", "--build"])
        .status()
        .expect("Failed to start compose stack");
    if !status.success() {
        anyhow::bail!("docker compose up failed");
    }
    let _guard = ComposeGuard;

    let http = reqwest::Client::new();

    wait_for_http_ok(&http, "http://127.0.0.1:3001/health").await?;
    wait_for_http_ok(&http, "http://127.0.0.1:3000/").await?;

    let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let food_name = format!("SmokeMilk{suffix}");

    let add = graphql(
        &http,
        json!({
            "query": format!(
                r#"mutation {{
                    addFood(name: "{food_name}", qty: 2) {{
                        id
                        name
                        qty
                    }}
                }}"#
            )
        }),
    )
    .await?;

    let added = add
        .get("data")
        .and_then(|data| data.get("addFood"))
        .ok_or_else(|| anyhow::anyhow!("missing addFood payload: {add}"))?;
    let added_id = added
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing addFood.id: {add}"))?;
    let added_name = added
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing addFood.name: {add}"))?;
    let added_qty = added
        .get("qty")
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow::anyhow!("missing addFood.qty: {add}"))?;

    if added_name != food_name || added_qty != 2 {
        anyhow::bail!("unexpected addFood response: {add}");
    }

    let listed = graphql(
        &http,
        json!({
            "query": r#"query {
                listFood {
                    id
                    name
                    qty
                }
            }"#
        }),
    )
    .await?;
    let list = listed
        .get("data")
        .and_then(|data| data.get("listFood"))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("missing listFood payload: {listed}"))?;

    let listed_item = list.iter().find(|item| {
        item.get("id").and_then(Value::as_str) == Some(added_id)
            && item.get("name").and_then(Value::as_str) == Some(food_name.as_str())
            && item.get("qty").and_then(Value::as_i64) == Some(2)
    });
    if listed_item.is_none() {
        anyhow::bail!("added item not visible via listFood: {listed}");
    }

    let deleted = graphql(
        &http,
        json!({
            "query": format!(
                r#"mutation {{
                    deleteFood(id: "{added_id}") {{
                        id
                        name
                        qty
                    }}
                }}"#
            )
        }),
    )
    .await?;
    let deleted_item = deleted
        .get("data")
        .and_then(|data| data.get("deleteFood"))
        .ok_or_else(|| anyhow::anyhow!("missing deleteFood payload: {deleted}"))?;
    if deleted_item.get("id").and_then(Value::as_str) != Some(added_id) {
        anyhow::bail!("unexpected deleteFood response: {deleted}");
    }

    let listed_after_delete = graphql(
        &http,
        json!({
            "query": r#"query {
                listFood {
                    id
                    name
                    qty
                }
            }"#
        }),
    )
    .await?;
    let remaining = listed_after_delete
        .get("data")
        .and_then(|data| data.get("listFood"))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("missing listFood payload after delete: {listed_after_delete}"))?;

    if remaining
        .iter()
        .any(|item| item.get("id").and_then(Value::as_str) == Some(added_id))
    {
        anyhow::bail!("deleted item still visible after delete: {listed_after_delete}");
    }

    println!("Compose smoke test passed.");
    Ok(())
}

async fn wait_for_http_ok(http: &reqwest::Client, url: &str) -> anyhow::Result<()> {
    for _ in 0..30 {
        if let Ok(response) = http.get(url).send().await {
            if response.status().is_success() {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    anyhow::bail!("timed out waiting for {url}");
}

async fn graphql(http: &reqwest::Client, body: Value) -> anyhow::Result<Value> {
    let response = http
        .post("http://127.0.0.1:3000/")
        .json(&body)
        .send()
        .await?
        .error_for_status()?;

    let json: Value = response.json().await?;
    if let Some(errors) = json.get("errors") {
        if errors.as_array().is_some_and(|errors| !errors.is_empty()) {
            anyhow::bail!("graphql errors: {json}");
        }
    }

    Ok(json)
}
