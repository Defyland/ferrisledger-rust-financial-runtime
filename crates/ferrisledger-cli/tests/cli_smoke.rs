//! CLI smoke coverage for local operator workflows.

use std::{
    path::Path,
    process::{Command, Stdio},
};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_ferrisledger")
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(binary())
        .args(args)
        .output()
        .expect("cli process")
}

fn assert_success(output: std::process::Output) -> String {
    assert!(
        output.status.success(),
        "expected command to succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("stdout utf8")
}

fn store_path(path: &Path) -> String {
    path.join("events.jsonl").display().to_string()
}

fn spawn_open_account_process(
    store: &str,
    account_id: &str,
    account_holder_name: &str,
    correlation_id: &str,
) -> std::process::Child {
    Command::new(binary())
        .arg("open-account")
        .arg("--store-path")
        .arg(store)
        .arg("--tenant-id")
        .arg("tenant_001")
        .arg("--account-id")
        .arg(account_id)
        .arg("--account-holder-name")
        .arg(account_holder_name)
        .arg("--correlation-id")
        .arg(correlation_id)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn cli process")
}

#[test]
fn verify_empty_store_outputs_integrity_summary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_path(dir.path());

    let stdout = assert_success(run(&["verify", "--store-path", &store]));
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("json");

    assert_eq!(value["records"], 0);
    assert_eq!(value["streams"], 0);
}

#[test]
fn open_deposit_and_replay_account_from_cli() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_path(dir.path());

    assert_success(run(&[
        "open-account",
        "--store-path",
        &store,
        "--tenant-id",
        "tenant_001",
        "--account-id",
        "account_001",
        "--account-holder-name",
        "Ada Lovelace",
        "--correlation-id",
        "corr_cli_open",
    ]));
    assert_success(run(&[
        "deposit",
        "--store-path",
        &store,
        "--tenant-id",
        "tenant_001",
        "--account-id",
        "account_001",
        "--amount-cents",
        "2500",
        "--idempotency-key",
        "cli_deposit_001",
        "--correlation-id",
        "corr_cli_deposit",
    ]));

    let stdout = assert_success(run(&[
        "replay",
        "--store-path",
        &store,
        "--tenant-id",
        "tenant_001",
        "--account-id",
        "account_001",
    ]));
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("json");

    assert_eq!(value["balance"]["cents"], 2500);
    assert_eq!(value["pending_pix_out"]["cents"], 0);
    assert_eq!(value["version"], 2);
}

#[test]
fn concurrent_cli_processes_share_one_locked_store() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_path(dir.path());
    let process_count = 8;
    let mut children = Vec::new();

    for index in 0..process_count {
        let account_id = format!("account_proc_{index:03}");
        let holder_name = format!("Process User {index:03}");
        let correlation_id = format!("corr_proc_{index:03}");
        let child = spawn_open_account_process(&store, &account_id, &holder_name, &correlation_id);
        children.push((index, child));
    }

    for (index, child) in children {
        let output = child.wait_with_output().expect("wait cli process");
        assert!(
            output.status.success(),
            "process {index} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = assert_success(run(&["verify", "--store-path", &store]));
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("json");

    assert_eq!(value["records"], process_count);
    assert_eq!(value["streams"], process_count);
}

#[test]
fn serve_rejects_weak_api_key_before_binding() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_path(dir.path());
    let output = run(&[
        "serve",
        "--bind",
        "127.0.0.1:0",
        "--store-path",
        &store,
        "--api-key",
        "short",
    ]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("configuration error"));
}
