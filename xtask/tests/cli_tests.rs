mod cli_tests {
    use assert_cmd::Command;

    #[tokio::test]
    async fn test_xtask_help() {
        let mut cmd = Command::cargo_bin("xtask").unwrap();
        cmd.arg("--help");

        let output = cmd.output().unwrap();

        assert!(output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stdout)
                .contains("A tool to manage the foodservice microservices infrastructure.")
        );
    }

    #[tokio::test]
    async fn test_xtask_deps_help() {
        let mut cmd = Command::cargo_bin("xtask").unwrap();
        cmd.arg("deps").arg("--help");

        let output = cmd.output().unwrap();

        assert!(output.status.success());
    }

    #[tokio::test]
    async fn test_xtask_command_parsing() {
        // Verifies that the subcommand parsing works by checking if it accepts valid subcommands
        // even if we don't run the full heavy logic, we test the command structure.
        let mut cmd = Command::cargo_bin("xtask").unwrap();
        cmd.arg("compose-up");

        let output = cmd.output().unwrap();

        // We check for the printed message to ensure it reached the correct branch
        assert!(output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stdout)
                .contains("Setting up Docker Compose environment...")
        );
    }
}
