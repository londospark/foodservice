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
    /// Setup the database
    ///     
    ///     This command will set up the database for the project, including creating necessary tables and seeding initial data.
    DbSetup,
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
        Commands::DbSetup => todo!(),
    }
}
