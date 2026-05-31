//! CLI smoke coverage for local operator workflows.

use std::{path::Path, process::Command};

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
