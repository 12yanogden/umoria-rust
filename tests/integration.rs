//! Final assembly & end-to-end integration gates.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use common::{golden_root, load_manifest, repo_root, verify_manifest};
use umoria::config::files;
use umoria::entry::{run_with_args, test_setup_entry_harness, test_take_stderr, test_take_stdout};
use umoria::scores::{test_reset_highscore_fp, test_set_scores_path};
use umoria::version::{CURRENT_VERSION_MAJOR, CURRENT_VERSION_MINOR, CURRENT_VERSION_PATCH};

static HARNESS_COUNTER: AtomicU64 = AtomicU64::new(0);

fn write_temp_scores_file() -> PathBuf {
    let id = HARNESS_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("umoria-int-{}-{}.dat", std::process::id(), id));
    fs::write(&path, [1u8, 2, 3]).expect("temp scores file");
    path
}

fn setup_lib_harness() -> PathBuf {
    test_setup_entry_harness();
    test_reset_highscore_fp();
    let path = write_temp_scores_file();
    test_set_scores_path(Some(&path));
    assert!(umoria::scores::initialize_score_file());
    path
}

fn umoria_binary() -> PathBuf {
    repo_root().join("target/debug/umoria")
}

// Step 0 — Wire the binary (build gate)

#[test]
fn test_crate_builds_and_binary_exists() {
    let status = Command::new("cargo")
        .args(["build", "--bin", "umoria"])
        .current_dir(repo_root())
        .status()
        .expect("cargo build should run");
    assert!(status.success(), "cargo build must produce umoria binary");
    assert!(
        umoria_binary().is_file(),
        "runnable umoria binary must exist at {}",
        umoria_binary().display()
    );
}

#[test]
fn test_main_rs_dispatch_reaches_start_moria() {
    let path = setup_lib_harness();
    let args = vec!["umoria".into(), "-n".into(), "-s".into(), "42".into()];
    let code = run_with_args(&args);
    assert_eq!(code, 0);
    assert_eq!(
        umoria::entry::test_start_moria_args(),
        Some((42, true, false))
    );
    let _ = fs::remove_file(&path);
}

#[test]
fn test_data_dir_resolved() {
    let data_dir = repo_root().join("data");
    for name in [
        "help.txt",
        "welcome.txt",
        "splash.txt",
        "versions.txt",
        "scores.dat",
    ] {
        let path = data_dir.join(name);
        assert!(
            path.is_file(),
            "runtime data file must exist at {}",
            path.display()
        );
    }
    assert_eq!(files::scores, "scores.dat");
    assert_eq!(files::save_game, "game.sav");
}

#[test]
fn test_golden_manifest_integrity() {
    let manifest = load_manifest().expect("manifest.json should parse");
    verify_manifest(&manifest).expect("golden files should match manifest hashes");
    assert!(golden_root().join("save/newchar_seed42.sav").is_file());
}

// Step 1 — CLI parity via assembled entry module (headless harness)

#[test]
fn test_cli_version_via_entry_harness() {
    let path = setup_lib_harness();
    let args = vec!["umoria".into(), "-v".into()];
    let code = run_with_args(&args);
    assert_eq!(code, 0);
    assert_eq!(
        test_take_stdout(),
        format!("{CURRENT_VERSION_MAJOR}.{CURRENT_VERSION_MINOR}.{CURRENT_VERSION_PATCH}\n")
    );
    assert!(test_take_stderr().is_empty());
    let _ = fs::remove_file(&path);
}

#[test]
fn test_cli_bad_seed_via_entry_harness() {
    let path = setup_lib_harness();
    let args = vec!["umoria".into(), "-s".into(), "0".into()];
    let code = run_with_args(&args);
    assert_eq!(code, 255);
    assert_eq!(
        test_take_stdout(),
        "Game seed must be a decimal number between 1 and 2147483647\n"
    );
    let _ = fs::remove_file(&path);
}

#[test]
fn test_cli_help_via_entry_harness() {
    let path = setup_lib_harness();
    let args = vec!["umoria".into(), "-h".into()];
    let code = run_with_args(&args);
    assert_eq!(code, 0);
    let stdout = test_take_stdout();
    assert!(stdout.contains("Robert A. Koeneke's classic dungeon crawler."));
    assert!(stdout.contains("GPL-3.0-or-later"));
    let _ = fs::remove_file(&path);
}

#[test]
fn test_cli_score_file_failure_via_entry_harness() {
    test_setup_entry_harness();
    umoria::entry::test_set_force_score_init_fail(true);
    let args = vec!["umoria".into(), "-v".into()];
    let code = run_with_args(&args);
    assert_eq!(code, 1);
    assert_eq!(
        test_take_stderr(),
        format!("Can't open score file '{}'\n", files::scores)
    );
}
