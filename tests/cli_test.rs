use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use std::io::Write;

#[test]
fn test_cli_hash_command() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "Test content").unwrap();

    let mut cmd = Command::cargo_bin("codio-cdn").unwrap();
    cmd.arg("hash")
       .arg(file.path())
       .assert()
       .success()
       .stdout(predicate::str::contains("CID:"))
       .stdout(predicate::str::contains("Qm"));
}

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("codio-cdn").unwrap();
    cmd.arg("--help")
       .assert()
       .success()
       .stdout(predicate::str::contains("codio-cdn"))
       .stdout(predicate::str::contains("publish"));
}

#[test]
fn test_cli_invalid_cid() {
    let mut cmd = Command::cargo_bin("codio-cdn").unwrap();
    cmd.arg("get")
       .arg("invalid-cid")
       .assert()
       .failure();
}

#[test]
fn test_cli_hash_nonexistent_file() {
    let mut cmd = Command::cargo_bin("codio-cdn").unwrap();
    cmd.arg("hash")
       .arg("/nonexistent/file.txt")
       .assert()
       .failure();
}

#[test]
fn test_cli_hash_empty_file() {
    let file = NamedTempFile::new().unwrap();
    // Don't write anything - file is empty

    let mut cmd = Command::cargo_bin("codio-cdn").unwrap();
    cmd.arg("hash")
       .arg(file.path())
       .assert()
       .success()
       .stdout(predicate::str::contains("CID:"))
       .stdout(predicate::str::contains("Size: 0 bytes"));
}

#[test]
fn test_cli_hash_deterministic() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "Deterministic test").unwrap();

    // Run hash command twice
    let mut cmd1 = Command::cargo_bin("codio-cdn").unwrap();
    let output1 = cmd1.arg("hash")
                      .arg(file.path())
                      .output()
                      .unwrap();

    let mut cmd2 = Command::cargo_bin("codio-cdn").unwrap();
    let output2 = cmd2.arg("hash")
                      .arg(file.path())
                      .output()
                      .unwrap();

    // Should produce the same CID
    assert_eq!(output1.stdout, output2.stdout);
}

#[test]
fn test_cli_verbose_flag() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "Verbose test").unwrap();

    let mut cmd = Command::cargo_bin("codio-cdn").unwrap();
    cmd.arg("--verbose")
       .arg("hash")
       .arg(file.path())
       .assert()
       .success()
       .stdout(predicate::str::contains("CID:"));
}

#[test]
fn test_cli_subcommand_required() {
    let mut cmd = Command::cargo_bin("codio-cdn").unwrap();
    cmd.assert()
       .failure();
}

#[test]
fn test_cli_get_help() {
    let mut cmd = Command::cargo_bin("codio-cdn").unwrap();
    cmd.arg("get")
       .arg("--help")
       .assert()
       .success()
       .stdout(predicate::str::contains("Retrieve content by CID"));
}

#[test]
fn test_cli_publish_help() {
    let mut cmd = Command::cargo_bin("codio-cdn").unwrap();
    cmd.arg("publish")
       .arg("--help")
       .assert()
       .success()
       .stdout(predicate::str::contains("Publish content and get CID"));
}

#[test]
fn test_cli_hash_help() {
    let mut cmd = Command::cargo_bin("codio-cdn").unwrap();
    cmd.arg("hash")
       .arg("--help")
       .assert()
       .success()
       .stdout(predicate::str::contains("Show CID for content"));
}
