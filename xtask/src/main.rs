use clap::{Parser, Subcommand};

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
    ///     
    ///     This command will install all the necessary dependencies for the project using Cargo.
    Deps,
    ///
    /// Setup docker compose
    ///     
    ///     This command will set up the Docker Compose environment for the project, allowing you to easily manage and run the various services that make up the foodservice microservices infrastructure.
    ComposeUp,
    ///Run tests
    ///     
    ///     This command will run all the tests for the project, ensuring that everything is working as expected.
    Test,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Deps => {
            println!("Installing project dependencies...");
            let status = std::process::Command::new("cargo")
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
            let status = std::process::Command::new("cargo")
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
            let status = std::process::Command::new("docker-compose")
                .args(["up", "-d"])
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
    }
}
