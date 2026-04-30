use clap::{Parser, Subcommand};
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "xtask",
    about = "A tool to manage the foodservice microservices infrastructure."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    /// Install project dependencies
    Deps,
    /// Setup docker compose
    ComposeUp,
    /// Run tests
    Test,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandResult {
    Success,
    Failure(i32),
    Error(String),
}

pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> CommandResult;
}

pub struct RealCommandRunner;

impl CommandRunner for RealCommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> CommandResult {
        match Command::new(program).args(args).status() {
            Ok(status) => {
                if status.success() {
                    CommandResult::Success
                } else {
                    CommandResult::Failure(status.code().unwrap_or(1))
                }
            }
            Err(e) => CommandResult::Error(e.to_string()),
        }
    }
}

pub fn run_app<R: CommandRunner>(cli: Cli, runner: R) -> CommandResult {
    match cli.command {
        Commands::Deps => {
            println!("Installing project dependencies...");
            runner.run("cargo", &["install", "sqlx-cli"])
        }
        Commands::Test => {
            println!("Running tests...");
            runner.run(
                "cargo",
                &["test", "--no-fail-fast", "--color", "always", "--workspace"],
            )
        }
        Commands::ComposeUp => {
            println!("Setting up Docker Compose environment...");
            runner.run("docker-compose", &["up", "-d"])
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let runner = RealCommandRunner;
    let result = run_app(cli, runner);

    match result {
        CommandResult::Success => std::process::exit(0),
        CommandResult::Failure(code) => {
            eprintln!("Command failed with exit code: {}", code);
            std::process::exit(code);
        }
        CommandResult::Error(e) => {
            eprintln!("Failed to execute command: {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone, PartialEq)]
    struct CallInfo {
        program: String,
        args: Vec<String>,
    }

    struct MockCommandRunner {
        pub result: CommandResult,
        pub calls: Arc<Mutex<Vec<CallInfo>>>,
    }

    impl CommandRunner for MockCommandRunner {
        fn run(&self, program: &str, args: &[&str]) -> CommandResult {
            let mut calls = self.calls.lock().unwrap();
            calls.push(CallInfo {
                program: program.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
            });
            self.result.clone()
        }
    }

    #[test]
    fn test_run_app_deps_success() {
        let cli = Cli {
            command: Commands::Deps,
        };
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = MockCommandRunner {
            result: CommandResult::Success,
            calls: calls.clone(),
        };
        let result = run_app(cli, runner);
        assert_eq!(result, CommandResult::Success);
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].program, "cargo");
        assert_eq!(calls[0].args, vec!["install", "sqlx-cli"]);
    }

    #[test]
    fn test_run_app_deps_failure() {
        let cli = Cli {
            command: Commands::Deps,
        };
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = MockCommandRunner {
            result: CommandResult::Failure(42),
            calls: calls.clone(),
        };
        let result = run_app(cli, runner);
        assert_eq!(result, CommandResult::Failure(42));
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].program, "cargo");
        assert_eq!(calls[0].args, vec!["install", "sqlx-cli"]);
    }

    #[test]
    fn test_run_app_test_success() {
        let cli = Cli {
            command: Commands::Test,
        };
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = MockCommandRunner {
            result: CommandResult::Success,
            calls: calls.clone(),
        };
        let result = run_app(cli, runner);
        assert_eq!(result, CommandResult::Success);
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].program, "cargo");
        assert_eq!(
            calls[0].args,
            vec!["test", "--no-fail-fast", "--color", "always", "--workspace"]
        );
    }

    #[test]
    fn test_run_app_compose_up_success() {
        let cli = Cli {
            command: Commands::ComposeUp,
        };
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = MockCommandRunner {
            result: CommandResult::Success,
            calls: calls.clone(),
        };
        let result = run_app(cli, runner);
        assert_eq!(result, CommandResult::Success);
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].program, "docker-compose");
        assert_eq!(calls[0].args, vec!["up", "-d"]);
    }
}
