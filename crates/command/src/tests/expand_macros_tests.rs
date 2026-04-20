#![allow(dead_code)]
use crate::prelude::*;

#[test]
fn command_new() {
    // Arrange
    let request = DelayRequest::default();
    let handler = Arc::new(DelayHandler);
    let handler = CommandHandler::Delay(handler);

    // Act
    let command = Command::new(request, handler);

    // Assert
    assert!(matches!(command, Command::Delay(_, _)));
}

#[test]
fn command_display() {
    // Arrange
    let request = DelayRequest::default();
    let handler = Arc::new(DelayHandler);
    let command = Command::Delay(request, handler);

    // Act
    let display = command.to_string();

    // Assert
    assert_eq!(display, "Delay 50 ms");
}

#[tokio::test]
async fn command_execute() {
    // Arrange
    let request = DelayRequest::default();
    let handler = Arc::new(DelayHandler);
    let command = Command::Delay(request, handler);
    let _logger = init_test_logger();

    // Act
    let response = command.execute().await;

    // Assert
    assert!(matches!(response, Ok(CommandSuccess::Delay(()))));
}

#[tokio::test]
async fn take_completed() {
    // Arrange
    let services = ServiceBuilder::new().with_commands().build();
    let runner = services
        .get_async::<CommandRunner<CommandInfo>>()
        .await
        .expect("should be able to get runner");
    let _logger = init_test_logger();
    runner
        .queue_request(DelayRequest::new("A".to_owned(), 10))
        .await
        .expect("should be able to queue request");
    runner
        .queue_request(DelayRequest::new("B".to_owned(), 10))
        .await
        .expect("should be able to queue request");
    runner.start(2).await;
    runner.drain().await;

    // Act
    let results = runner.take_completed::<DelayRequest>().await;

    // Assert
    assert_eq!(results.len(), 2);
    for (_request, result) in &results {
        assert!(result.is_ok());
    }
}

#[test]
fn macro_expansion() {
    // Act
    use std::process::Command as ShellCommand;
    let output = ShellCommand::new("cargo")
        .args([
            "expand",
            "--all-features",
            "--release",
            "--locked",
            "-p",
            "studiole-command",
            "--lib",
            "--tests",
            "tests::expand_macros",
        ])
        .output()
        .expect("cargo-expand should be installed");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid UTF-8");

    // Assert
    assert!(
        output.status.success(),
        "cargo expand failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    insta::assert_snapshot!(stdout);
}
