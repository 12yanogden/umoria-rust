//! Main entry point.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use umoria::config::files;
use umoria::entry::{
    expected_help_output, parse_game_seed, run_with_args, test_entry_trace_events,
    test_reset_entry_hooks, test_set_capture_stderr, test_set_capture_stdout, test_set_entry_trace,
    test_set_force_permissions_fail, test_set_force_score_init_fail,
    test_set_force_terminal_init_fail, test_set_skip_start_moria, test_setup_entry_harness,
    test_start_moria_args, test_take_stderr, test_take_stdout, USAGE_INSTRUCTIONS,
};
use umoria::game::{test_exit_program_called, with_state};
use umoria::scores::{test_reset_highscore_fp, test_set_scores_path};
use umoria::version::{CURRENT_VERSION_MAJOR, CURRENT_VERSION_MINOR, CURRENT_VERSION_PATCH};

static TEMP_SCORES_COUNTER: AtomicU64 = AtomicU64::new(0);

fn write_temp_scores_file() -> PathBuf {
    let id = TEMP_SCORES_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path =
        std::env::temp_dir().join(format!("umoria-phase57-{}-{}.dat", std::process::id(), id));
    fs::write(&path, [1u8, 2, 3]).expect("temp scores file");
    path
}

fn setup_harness() -> PathBuf {
    test_reset_entry_hooks();
    test_set_capture_stdout(true);
    test_set_capture_stderr(true);
    test_set_skip_start_moria(true);
    test_set_entry_trace(true);
    umoria::game::test_set_skip_process_exit(true);
    umoria::game::test_reset_exit_program_called();
    umoria::ui_io::test_set_ncurses_stub(true);
    umoria::ui_io::register_game_ui_hooks();
    umoria::game::reset_for_new_game(None);
    test_reset_highscore_fp();
    let path = write_temp_scores_file();
    test_set_scores_path(Some(&path));
    assert!(umoria::scores::initialize_score_file());
    path
}

fn teardown_scores(path: &PathBuf) {
    test_reset_highscore_fp();
    let _ = fs::remove_file(path);
}

fn run(args: &[&str]) -> u8 {
    let owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    run_with_args(&owned)
}

// ---------------------------------------------------------------------------
// Step 1 — parseGameSeed
// ---------------------------------------------------------------------------

#[test]
fn step1_parse_game_seed_accepts_valid_range() {
    let mut seed = 0;
    assert!(parse_game_seed("1", &mut seed));
    assert_eq!(seed, 1);

    assert!(parse_game_seed("2147483647", &mut seed));
    assert_eq!(seed, 2_147_483_647);
}

#[test]
fn step1_parse_game_seed_rejects_zero_and_negative() {
    let mut seed = 99;
    assert!(!parse_game_seed("0", &mut seed));
    assert!(!parse_game_seed("-1", &mut seed));
    assert!(!parse_game_seed("-5", &mut seed));
    assert_eq!(seed, 99);
}

#[test]
fn step1_parse_game_seed_rejects_non_numeric_and_garbage() {
    let mut seed = 0;
    assert!(!parse_game_seed("abc", &mut seed));
    assert!(!parse_game_seed("12x", &mut seed));
    assert!(!parse_game_seed("", &mut seed));
}

#[test]
fn step1_parse_game_seed_truncates_large_decimal_like_c_strtol() {
    let mut seed = 0;
    assert!(parse_game_seed("9999999999999", &mut seed));
    assert_eq!(seed, 1_316_134_911);
}

// ---------------------------------------------------------------------------
// Step 2 — startup guards + version/usage/error output
// ---------------------------------------------------------------------------

#[test]
fn step2_version_flag_prints_exact_version() {
    let path = setup_harness();
    let code = run(&["umoria", "-v"]);
    assert_eq!(code, 0);
    assert_eq!(
        test_take_stdout(),
        format!("{CURRENT_VERSION_MAJOR}.{CURRENT_VERSION_MINOR}.{CURRENT_VERSION_PATCH}\n")
    );
    assert!(test_take_stderr().is_empty());
    teardown_scores(&path);
}

#[test]
fn step2_help_unknown_and_lone_dash_match_default_output() {
    let path = setup_harness();
    let expected = expected_help_output();

    for args in [
        &["umoria", "-h"][..],
        &["umoria", "-z"][..],
        &["umoria", "-"][..],
    ] {
        test_set_capture_stdout(true);
        let code = run(args);
        assert_eq!(code, 0, "args={args:?}");
        assert_eq!(test_take_stdout(), expected, "args={args:?}");
        assert!(test_take_stderr().is_empty(), "args={args:?}");
    }
    teardown_scores(&path);
}

#[test]
fn step2_bad_seed_values_print_error_and_exit_255() {
    let path = setup_harness();
    let expected_err = "Game seed must be a decimal number between 1 and 2147483647\n";

    for args in [
        &["umoria", "-s", "0"][..],
        &["umoria", "-s", "-1"][..],
        &["umoria", "-s", "abc"][..],
        &["umoria", "-s", "12x"][..],
    ] {
        test_set_capture_stdout(true);
        let code = run(args);
        assert_eq!(code, 255, "args={args:?}");
        assert_eq!(test_take_stdout(), expected_err, "args={args:?}");
        assert!(test_take_stderr().is_empty(), "args={args:?}");
    }
    teardown_scores(&path);
}

#[test]
fn step2_score_file_init_failure_prints_stderr_and_exits_1() {
    test_setup_entry_harness();
    test_set_force_score_init_fail(true);
    let code = run(&["umoria", "-v"]);
    assert_eq!(code, 1);
    assert_eq!(
        test_take_stderr(),
        format!("Can't open score file '{}'\n", files::scores)
    );
    assert!(test_take_stdout().is_empty());
    test_reset_entry_hooks();
}

#[test]
fn step2_permissions_failure_exits_1_without_message() {
    let path = setup_harness();
    test_set_force_permissions_fail(true);
    let code = run(&["umoria", "-v"]);
    assert_eq!(code, 1);
    assert!(test_take_stdout().is_empty());
    assert!(test_take_stderr().is_empty());
    teardown_scores(&path);
    test_reset_entry_hooks();
}

#[test]
fn step2_terminal_init_failure_exits_1_without_message() {
    let path = setup_harness();
    test_set_force_terminal_init_fail(true);
    let code = run(&["umoria", "-v"]);
    assert_eq!(code, 1);
    assert!(test_take_stdout().is_empty());
    assert!(test_take_stderr().is_empty());
    teardown_scores(&path);
    test_reset_entry_hooks();
}

// ---------------------------------------------------------------------------
// Step 3 — arg loop + flag state + save-file/startMoria hand-off
// ---------------------------------------------------------------------------

#[test]
fn step3_flags_set_new_game_roguelike_and_wizard() {
    let path = setup_harness();
    let code = run(&["umoria", "-n", "-r", "-w", "SAVE"]);
    assert_eq!(code, 0);
    assert_eq!(test_start_moria_args(), Some((0, true, true)));
    assert!(with_state(|state| state.game.to_be_wizard));
    assert_eq!(with_state(|state| state.config_save_game.clone()), "SAVE");
    teardown_scores(&path);
}

#[test]
fn step3_seed_and_save_file_hand_off() {
    let path = setup_harness();
    let code = run(&["umoria", "-s", "42", "SAVE"]);
    assert_eq!(code, 0);
    assert_eq!(test_start_moria_args(), Some((42, false, false)));
    assert_eq!(with_state(|state| state.config_save_game.clone()), "SAVE");
    teardown_scores(&path);
}

#[test]
fn step3_trailing_bare_s_ignored() {
    let path = setup_harness();
    let code = run(&["umoria", "-n", "-s"]);
    assert_eq!(code, 0);
    assert_eq!(test_start_moria_args(), Some((0, true, false)));
    assert_eq!(
        with_state(|state| state.config_save_game.clone()),
        files::save_game
    );
    teardown_scores(&path);
}

#[test]
fn step3_default_save_game_without_positional_arg() {
    let path = setup_harness();
    let code = run(&["umoria"]);
    assert_eq!(code, 0);
    assert_eq!(test_start_moria_args(), Some((0, false, false)));
    assert_eq!(
        with_state(|state| state.config_save_game.clone()),
        files::save_game
    );
    teardown_scores(&path);
}

#[test]
fn step3_d_calls_show_scores_then_exit_program() {
    let path = setup_harness();
    let code = run(&["umoria", "-d"]);
    assert_eq!(code, 0);
    assert_eq!(
        test_entry_trace_events(),
        vec!["show_scores_screen", "exit_program"]
    );
    assert!(test_exit_program_called());
    assert!(test_start_moria_args().is_none());
    teardown_scores(&path);
}

#[test]
fn step3_usage_instructions_literal_matches_cpp() {
    assert!(USAGE_INSTRUCTIONS.starts_with('\n'));
    assert!(USAGE_INSTRUCTIONS.contains("-n           Force start of new game"));
    assert!(!USAGE_INSTRUCTIONS.contains("-w"));
    assert!(USAGE_INSTRUCTIONS.ends_with("-h           Display this message\n"));
}
