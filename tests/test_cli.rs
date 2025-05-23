#[cfg(test)]
extern crate assert_cmd;
extern crate predicates;

use assert_cmd::prelude::*;
use predicates::prelude::*;

use std::process::Command;

#[test]
fn test_cli() {
    let mut cmd = Command::cargo_bin("codeinput").expect("Calling binary failed");
    cmd.assert().failure();
}

#[test]
fn test_version() {
    let expected_version = "codeinput 0.0.1-beta\n";
    let mut cmd = Command::cargo_bin("codeinput").expect("Calling binary failed");
    cmd.arg("--version").assert().stdout(expected_version);
}
