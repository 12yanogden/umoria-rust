//! Port of `src/main.cpp` — CLI entry point, argument parsing, and startup guards.

use std::cell::{Cell, RefCell};
use std::io::{self, Write};

use crate::config::files;
use crate::game::{
    exit_program, reset_for_new_game, test_reset_exit_program_called, test_set_skip_process_exit,
    with_state_mut,
};
use crate::game_files::initialize_score_file;
use crate::game_run::start_moria;
use crate::helpers::string_to_number;
use crate::scores::show_scores_screen;
use crate::ui_io::{self, terminal};
use crate::version::{CURRENT_VERSION_MAJOR, CURRENT_VERSION_MINOR, CURRENT_VERSION_PATCH};

/// C++ `usage_instructions` raw string in main.cpp lines 13–27 (byte-for-byte).
pub const USAGE_INSTRUCTIONS: &str = "\
\nUsage:\n    umoria [OPTIONS] SAVEGAME\n\nSAVEGAME is an optional save game filename (default: game.sav)\n\nOptions:\n    -n           Force start of new game\n    -r           Enable classic roguelike keys on startup (default: disabled, or save game settings)\n    -d           Display high scores and exit\n    -s NUMBER    Game Seed, as a decimal number (max: 2147483647)\n\n    -v           Print version info and exit\n    -h           Display this message\n";

const SEED_ERROR: &str = "Game seed must be a decimal number between 1 and 2147483647\n";

thread_local! {
    static TEST_CAPTURE_STDOUT: Cell<bool> = const { Cell::new(false) };
    static TEST_STDOUT: RefCell<String> = const { RefCell::new(String::new()) };
    static TEST_CAPTURE_STDERR: Cell<bool> = const { Cell::new(false) };
    static TEST_STDERR: RefCell<String> = const { RefCell::new(String::new()) };
    static TEST_FORCE_SCORE_INIT_FAIL: Cell<bool> = const { Cell::new(false) };
    static TEST_FORCE_PERMISSIONS_FAIL: Cell<bool> = const { Cell::new(false) };
    static TEST_FORCE_TERMINAL_INIT_FAIL: Cell<bool> = const { Cell::new(false) };
    static TEST_SKIP_START_MORIA: Cell<bool> = const { Cell::new(false) };
    static TEST_START_MORIA_ARGS: RefCell<Option<(u32, bool, bool)>> = const { RefCell::new(None) };
    static TEST_ENTRY_TRACE: Cell<bool> = const { Cell::new(false) };
    static TEST_TRACE_EVENTS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

fn trace(event: &'static str) {
    if TEST_ENTRY_TRACE.with(std::cell::Cell::get) {
        TEST_TRACE_EVENTS.with(|events| events.borrow_mut().push(event));
    }
}

fn stdout_print(text: &str) {
    if TEST_CAPTURE_STDOUT.with(std::cell::Cell::get) {
        TEST_STDOUT.with(|out| out.borrow_mut().push_str(text));
        return;
    }
    let _ = write!(io::stdout(), "{text}");
    let _ = io::stdout().flush();
}

fn stderr_print(text: &str) {
    if TEST_CAPTURE_STDERR.with(std::cell::Cell::get) {
        TEST_STDERR.with(|out| out.borrow_mut().push_str(text));
        return;
    }
    let _ = write!(io::stderr(), "{text}");
    let _ = io::stderr().flush();
}

fn print_help_and_license() {
    terminal::terminal_restore();
    stdout_print("Robert A. Koeneke's classic dungeon crawler.\n");
    stdout_print(&format!(
        "Umoria {CURRENT_VERSION_MAJOR}.{CURRENT_VERSION_MINOR}.{CURRENT_VERSION_PATCH} is released under a GPL-3.0-or-later license.\n"
    ));
    stdout_print(USAGE_INSTRUCTIONS);
}

/// C++ `parseGameSeed` in main.cpp lines 109–122.
pub fn parse_game_seed(arg: &str, seed: &mut u32) -> bool {
    let mut value = 0i32;
    if !string_to_number(arg, &mut value) {
        return false;
    }
    if value <= 0 {
        return false;
    }
    *seed = value as u32;
    true
}

/// C++ `main` body — returns the process exit status (0, 1, or 255).
pub fn run_with_args(args: &[String]) -> u8 {
    let mut seed = 0u32;
    let mut new_game = false;
    let mut roguelike_keys = false;

    if TEST_FORCE_SCORE_INIT_FAIL.with(std::cell::Cell::get) || !initialize_score_file() {
        stderr_print(&format!("Can't open score file '{}'\n", files::scores));
        return 1;
    }

    if TEST_FORCE_PERMISSIONS_FAIL.with(std::cell::Cell::get) || !terminal::check_file_permissions()
    {
        return 1;
    }

    if TEST_FORCE_TERMINAL_INIT_FAIL.with(std::cell::Cell::get) || !terminal::terminal_initialize()
    {
        return 1;
    }

    let mut index = 1usize;
    while index < args.len() {
        let arg = &args[index];
        if !arg.starts_with('-') {
            break;
        }

        let flag = arg.as_bytes().get(1).copied().unwrap_or(0);
        match flag as char {
            'v' => {
                terminal::terminal_restore();
                stdout_print(&format!(
                    "{CURRENT_VERSION_MAJOR}.{CURRENT_VERSION_MINOR}.{CURRENT_VERSION_PATCH}\n"
                ));
                return 0;
            }
            'n' => {
                new_game = true;
            }
            'r' => {
                roguelike_keys = true;
            }
            'd' => {
                trace("show_scores_screen");
                show_scores_screen();
                trace("exit_program");
                exit_program();
                return 0;
            }
            's' => {
                if index + 1 >= args.len() {
                    // Trailing bare `-s` — no NUMBER provided.
                } else {
                    index += 1;
                    if !parse_game_seed(&args[index], &mut seed) {
                        terminal::terminal_restore();
                        stdout_print(SEED_ERROR);
                        return 255;
                    }
                }
            }
            'w' => {
                with_state_mut(|state| state.game.to_be_wizard = true);
            }
            _ => {
                print_help_and_license();
                return 0;
            }
        }
        index += 1;
    }

    if index < args.len() {
        with_state_mut(|state| state.config_save_game.clone_from(&args[index]));
    }

    if TEST_SKIP_START_MORIA.with(std::cell::Cell::get) {
        TEST_START_MORIA_ARGS
            .with(|slot| *slot.borrow_mut() = Some((seed, new_game, roguelike_keys)));
        return 0;
    }

    start_moria(seed, new_game, roguelike_keys);
    0
}

pub fn expected_help_output() -> String {
    format!(
        "Robert A. Koeneke's classic dungeon crawler.\nUmoria {CURRENT_VERSION_MAJOR}.{CURRENT_VERSION_MINOR}.{CURRENT_VERSION_PATCH} is released under a GPL-3.0-or-later license.\n{USAGE_INSTRUCTIONS}"
    )
}

#[doc(hidden)]
pub fn test_reset_entry_hooks() {
    TEST_CAPTURE_STDOUT.with(|c| c.set(false));
    TEST_STDOUT.with(|out| *out.borrow_mut() = String::new());
    TEST_CAPTURE_STDERR.with(|c| c.set(false));
    TEST_STDERR.with(|out| *out.borrow_mut() = String::new());
    TEST_FORCE_SCORE_INIT_FAIL.with(|c| c.set(false));
    TEST_FORCE_PERMISSIONS_FAIL.with(|c| c.set(false));
    TEST_FORCE_TERMINAL_INIT_FAIL.with(|c| c.set(false));
    TEST_SKIP_START_MORIA.with(|c| c.set(false));
    TEST_START_MORIA_ARGS.with(|slot| *slot.borrow_mut() = None);
    TEST_ENTRY_TRACE.with(|c| c.set(false));
    TEST_TRACE_EVENTS.with(|events| events.borrow_mut().clear());
}

#[doc(hidden)]
pub fn test_set_capture_stdout(capture: bool) {
    TEST_CAPTURE_STDOUT.with(|c| c.set(capture));
    if capture {
        TEST_STDOUT.with(|out| *out.borrow_mut() = String::new());
    }
}

#[doc(hidden)]
pub fn test_take_stdout() -> String {
    TEST_STDOUT.with(|out| std::mem::take(&mut *out.borrow_mut()))
}

#[doc(hidden)]
pub fn test_set_capture_stderr(capture: bool) {
    TEST_CAPTURE_STDERR.with(|c| c.set(capture));
    if capture {
        TEST_STDERR.with(|out| *out.borrow_mut() = String::new());
    }
}

#[doc(hidden)]
pub fn test_take_stderr() -> String {
    TEST_STDERR.with(|out| std::mem::take(&mut *out.borrow_mut()))
}

#[doc(hidden)]
pub fn test_set_force_score_init_fail(fail: bool) {
    TEST_FORCE_SCORE_INIT_FAIL.with(|c| c.set(fail));
}

#[doc(hidden)]
pub fn test_set_force_permissions_fail(fail: bool) {
    TEST_FORCE_PERMISSIONS_FAIL.with(|c| c.set(fail));
}

#[doc(hidden)]
pub fn test_set_force_terminal_init_fail(fail: bool) {
    TEST_FORCE_TERMINAL_INIT_FAIL.with(|c| c.set(fail));
}

#[doc(hidden)]
pub fn test_set_skip_start_moria(skip: bool) {
    TEST_SKIP_START_MORIA.with(|c| c.set(skip));
}

#[doc(hidden)]
pub fn test_start_moria_args() -> Option<(u32, bool, bool)> {
    TEST_START_MORIA_ARGS.with(|slot| *slot.borrow())
}

#[doc(hidden)]
pub fn test_set_entry_trace(trace: bool) {
    TEST_ENTRY_TRACE.with(|c| c.set(trace));
    if trace {
        TEST_TRACE_EVENTS.with(|events| events.borrow_mut().clear());
    }
}

#[doc(hidden)]
pub fn test_entry_trace_events() -> Vec<&'static str> {
    TEST_TRACE_EVENTS.with(|events| events.borrow().clone())
}

#[doc(hidden)]
pub fn test_setup_entry_harness() {
    test_reset_entry_hooks();
    test_set_capture_stdout(true);
    test_set_capture_stderr(true);
    test_set_skip_start_moria(true);
    test_set_entry_trace(true);
    test_set_skip_process_exit(true);
    test_reset_exit_program_called();
    ui_io::test_set_ncurses_stub(true);
    ui_io::register_game_ui_hooks();
    reset_for_new_game(None);
}
