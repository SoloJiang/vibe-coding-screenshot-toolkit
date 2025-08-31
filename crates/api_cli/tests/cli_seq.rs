use std::fs;
use std::process::Command;

#[test]
fn seq_persists_across_invocations() {
    // Use a temp dir
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("shots");
    fs::create_dir_all(&out).unwrap();
    // First invocation
    let status1 = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "api_cli",
            "--",
            "capture",
            "-d",
            out.to_str().unwrap(),
            "--mock",
        ])
        .status()
        .expect("run1");
    assert!(status1.success());
    // Second invocation
    let status2 = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "api_cli",
            "--",
            "capture",
            "-d",
            out.to_str().unwrap(),
            "--mock",
        ])
        .status()
        .expect("run2");
    assert!(status2.success());
    // Read history seq file
    let seq_file = out.join(".history").join("seq.txt");
    let txt = fs::read_to_string(&seq_file).expect("seq file");
    // basic format check
    let parts: Vec<&str> = txt.split_whitespace().collect();
    assert_eq!(parts.len(), 2);
    let v: u32 = parts[1].parse().unwrap();
    assert!(
        v >= 2,
        "sequence should be at least 2 after two runs, got {}",
        v
    );
}
