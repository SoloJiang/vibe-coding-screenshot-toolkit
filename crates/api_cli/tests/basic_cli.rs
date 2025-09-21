use std::process::Command;

/// 测试CLI基础功能
#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(["run", "-q", "-p", "api_cli", "--", "--help"])
        .output()
        .expect("Failed to run CLI help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Screenshot Toolkit") || stdout.contains("交互式截图"));
}

#[test]
fn test_cli_version() {
    let output = Command::new("cargo")
        .args(["run", "-q", "-p", "api_cli", "--", "version"])
        .output()
        .expect("Failed to run CLI version");

    assert!(output.status.success());
}

#[test]
fn test_capture_interactive_help() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "api_cli",
            "--",
            "capture-interactive",
            "--help",
        ])
        .output()
        .expect("Failed to run capture-interactive help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("交互式框选截图"));
    assert!(stdout.contains("--clipboard"));
    assert!(stdout.contains("--out-dir"));
    assert!(stdout.contains("--template"));
}

/// 测试无效命令的错误处理
#[test]
fn test_invalid_command() {
    let output = Command::new("cargo")
        .args(["run", "-q", "-p", "api_cli", "--", "invalid-command"])
        .output()
        .expect("Failed to run invalid command");

    assert!(!output.status.success());
}
