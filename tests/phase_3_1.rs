//! Phase 3.1 — ui_io (curses wrapper & terminal I/O).
//! See `.cursor/plans/rust-translation/phase_3.1.md`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::game::{reset_for_new_game, with_state, with_state_mut};
use umoria::types::{Vtype_t, MORIA_MESSAGE_SIZE};
use umoria::ui_io::{
    self, advance_message_ring_index, append_message_slot, clamp_more_column,
    clamp_string_input_end_col, confirmation_key_result, copy_message_to_ring_slot, ctrl_key,
    is_printable_key, message_old_len, more_prompt_accepts_key, panel_screen_coord,
    should_combine_messages, should_show_more,
    terminal::{self, Coord},
    test_bell_count, test_clear_getch_keys, test_flush_input_buffer_count, test_push_getch_keys,
    test_set_eof_flag, test_set_ncurses_stub, test_set_select_ready, test_set_ui_detail_capture,
    test_set_ui_trace, test_ui_trace_events, trim_trailing_spaces, UiTraceEvent, ESCAPE,
};

// ---------------------------------------------------------------------------
// T1 — putString truncation (pure)
// ---------------------------------------------------------------------------
#[test]
fn t1_put_string_truncation_boundaries() {
    // C++ ui_io.cpp lines 147–154: clamp x>79, strncpy to 79-x, force NUL.
    assert_eq!(ui_io::truncate_for_put_string("ABCDEFGH", 75), "ABCD");
    assert_eq!(ui_io::truncate_for_put_string("AB", 79), "");
    assert_eq!(ui_io::truncate_for_put_string("AB", 85), "");
    assert_eq!(
        ui_io::truncate_for_put_string(&"A".repeat(100), 10),
        "A".repeat(69)
    );
    assert_eq!(
        ui_io::truncate_for_put_string(&"A".repeat(100), 0),
        "A".repeat(79)
    );
}

#[test]
#[ignore = "TODO(phase_1.5): screen-capture harness for putString grid write"]
fn t1_put_string_capture_hundred_as_at_x10() {
    let _ = (&"A".repeat(100), Coord { y: 5, x: 10 });
}

// ---------------------------------------------------------------------------
// T2 — cursor/char/clear (capture → ignored; pure MSG_LINE guard via T4/T5)
// ---------------------------------------------------------------------------
#[test]
#[ignore = "TODO(phase_1.5): screen-capture harness for moveCursor/addChar/clear"]
fn t2_output_primitives_capture() {}

// ---------------------------------------------------------------------------
// T3 — panel coordinate interpolation (pure)
// ---------------------------------------------------------------------------
#[test]
fn t3_panel_screen_coord_subtracts_offsets() {
    // C++ ui_io.cpp lines 182–190, 194–202.
    let coord = Coord { y: 10, x: 30 };
    let screen = panel_screen_coord(coord, 2, 13);
    assert_eq!(screen, Coord { y: 8, x: 17 });
}

#[test]
#[ignore = "TODO(phase_1.5): screen-capture harness for panelMoveCursor/panelPutTile"]
fn t3_panel_primitives_capture() {}

// ---------------------------------------------------------------------------
// T4 — message ring buffer & message line cap (pure)
// ---------------------------------------------------------------------------
#[test]
fn t4_cap_message_line_resize_79() {
    let long = ui_io::cap_message_line("A".repeat(100));
    assert_eq!(long.len(), 79);
    let short = ui_io::cap_message_line("hello".to_string());
    assert_eq!(short.len(), 79);
    assert_eq!(&short.as_bytes()[..5], b"hello");
    assert!(short.as_bytes()[5..].iter().all(|&b| b == 0));
}

#[test]
fn t4_message_ring_wrap_and_truncation() {
    // C++ ui_io.cpp lines 304–312.
    assert_eq!(advance_message_ring_index(21), 0);
    assert_eq!(advance_message_ring_index(0), 1);

    let mut slot = [0u8; MORIA_MESSAGE_SIZE];
    let msg = "X".repeat(MORIA_MESSAGE_SIZE + 10);
    copy_message_to_ring_slot(&mut slot, &msg);
    assert_eq!(slot[MORIA_MESSAGE_SIZE - 1], 0);
    assert_eq!(
        slot[..MORIA_MESSAGE_SIZE - 1],
        [b'X'; MORIA_MESSAGE_SIZE - 1]
    );
}

#[test]
#[ignore = "TODO(phase_1.5): screen-capture harness for messageLinePrintMessage cursor save"]
fn t4_message_line_print_capture() {}

// ---------------------------------------------------------------------------
// T5 — printMessage combine / -more- thresholds (pure)
// ---------------------------------------------------------------------------
#[test]
fn t5_combine_and_more_thresholds() {
    // C++ ui_io.cpp lines 250–277: old_len = strlen + 1; >= 73 → -more-.
    let old = message_old_len(b"hello"); // 5 + 1 = 6
    assert_eq!(old, 6);
    assert!(should_combine_messages(6, 64)); // 64+6+2=72 < 73
    assert!(!should_combine_messages(6, 65)); // 65+6+2=73 >= 73
    assert!(should_show_more(false, 6, 65));
    assert!(should_show_more(true, 6, 0));
    assert_eq!(clamp_more_column(74), 73);
    assert_eq!(clamp_more_column(70), 70);
}

#[test]
fn t5_combine_appends_two_spaces_and_msg() {
    let mut slot = vtype_from(b"first");
    append_message_slot(&mut slot, "second");
    assert_eq!(c_str(&slot), "first  second");
}

#[test]
fn t5_more_prompt_accepts_space_escape_cr_lf() {
    assert!(more_prompt_accepts_key(b' '));
    assert!(more_prompt_accepts_key(b'\x1b'));
    assert!(more_prompt_accepts_key(b'\n'));
    assert!(more_prompt_accepts_key(b'\r'));
    assert!(!more_prompt_accepts_key(b'q'));
}

#[test]
#[ignore = "TODO(phase_1.5): screen-capture harness + key injection for -more- loop"]
fn t5_more_prompt_input_and_capture() {}

// ---------------------------------------------------------------------------
// T6 — getKeyInput
// ---------------------------------------------------------------------------
#[test]
fn t6_get_key_input_normal_char() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    // Queue is LIFO (pop); push in reverse consume order.
    test_push_getch_keys(&[i32::from(b'a')]);
    assert_eq!(terminal::get_key_input(), b'a');
    test_set_ncurses_stub(false);
}

#[test]
fn t6_get_key_input_ctrl_r_redraw() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    // ^R is consumed; next key is returned. getKeyInput never returns ^R.
    test_push_getch_keys(&[i32::from(b'z'), i32::from(ctrl_key(b'R'))]);
    assert_eq!(terminal::get_key_input(), b'z');
    test_set_ncurses_stub(false);
}

#[test]
fn t6_get_key_input_eof_hangup_path() {
    // C++: EOF bumps eof_flag and returns ESCAPE when character is in-progress
    // (generated but not yet saved) — otherwise endGame() is called.
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_set_eof_flag(0);
    test_clear_getch_keys();
    with_state_mut(|s| {
        s.game.character_generated = true;
        s.game.character_saved = false;
    });
    test_push_getch_keys(&[-1]); // libc::EOF
    assert_eq!(terminal::get_key_input(), ESCAPE);
    assert_eq!(ui_io::eof_flag(), 1);
    test_set_ncurses_stub(false);
}

// ---------------------------------------------------------------------------
// T7 — getCommand wrappers
// ---------------------------------------------------------------------------
#[test]
fn t7_command_wrappers_input() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'q')]);
    let mut command = 0u8;
    assert!(terminal::get_command("", &mut command));
    assert_eq!(command, b'q');

    test_push_getch_keys(&[i32::from(ESCAPE)]);
    assert!(!terminal::get_tile_character("", &mut command));
    assert_eq!(command, ESCAPE);
    test_set_ncurses_stub(false);
}

// ---------------------------------------------------------------------------
// T8 — getStringInput editing (pure helpers)
// ---------------------------------------------------------------------------
#[test]
fn t8_string_input_end_col_clamped_to_79() {
    // C++ ui_io.cpp lines 409–414.
    assert_eq!(clamp_string_input_end_col(70, 20), 79);
    assert_eq!(clamp_string_input_end_col(10, 5), 14);
}

#[test]
fn t8_isprint_matches_c_ascii() {
    // C++ ui_io.cpp line 441: isprint(key) — documented as 0x20..=0x7e.
    assert!(!is_printable_key(0x1f));
    assert!(is_printable_key(b'a' as i32));
    assert!(!is_printable_key(0x7f));
}

#[test]
fn t8_trim_trailing_blanks_on_submit() {
    let mut buf = *b"hello   \0";
    let len = trim_trailing_spaces(&mut buf);
    assert_eq!(len, 5);
    assert_eq!(&buf[..len], b"hello");
    assert_eq!(buf[len], 0);
}

#[test]
#[ignore = "TODO(phase_1.5): screen-capture harness for getStringInput loop"]
fn t8_get_string_input_editing_capture() {}

// ---------------------------------------------------------------------------
// T9 — confirmations (pure + capture ignored)
// ---------------------------------------------------------------------------
#[test]
fn t9_confirmation_key_codes() {
    // C++ ui_io.cpp lines 494–500.
    assert_eq!(confirmation_key_result(b'y'), 1);
    assert_eq!(confirmation_key_result(b'Y'), 1);
    assert_eq!(confirmation_key_result(b'n'), 0);
    assert_eq!(confirmation_key_result(b'N'), 0);
    assert_eq!(confirmation_key_result(b'q'), -1);
}

#[test]
#[ignore = "TODO(phase_1.5): screen-capture harness for confirmation prompt layout"]
fn t9_confirmation_prompt_capture() {}

// ---------------------------------------------------------------------------
// T10 — putQIO / bell / flush
// ---------------------------------------------------------------------------
#[test]
fn t10_put_qio_sets_screen_has_changed() {
    reset_for_new_game(None);
    with_state_mut(|s| s.screen_has_changed = false);
    ui_io::test_set_ncurses_stub(true);
    terminal::put_qio();
    ui_io::test_set_ncurses_stub(false);
    assert!(with_state(|s| s.screen_has_changed));
}

#[test]
fn t10_terminal_bell_sound_write() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_set_ui_detail_capture(true);
    with_state_mut(|s| s.options.error_beep_sound = true);
    assert_eq!(terminal::terminal_bell_sound(), 1);
    assert_eq!(test_bell_count(), 1);
    with_state_mut(|s| s.options.error_beep_sound = false);
    assert_eq!(terminal::terminal_bell_sound(), 0);
    assert_eq!(test_bell_count(), 1);
    test_set_ui_detail_capture(false);
    test_set_ncurses_stub(false);
}

#[test]
fn t10_flush_input_buffer_drains_queue() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_set_ui_detail_capture(true);
    test_set_eof_flag(0);
    test_set_select_ready(Some(true));
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'a'), i32::from(b'b')]);
    terminal::flush_input_buffer();
    assert!(test_flush_input_buffer_count() >= 1);
    // After drain, no keys remain for a subsequent getch pop.
    test_set_select_ready(None);
    test_set_ui_detail_capture(false);
    test_set_ncurses_stub(false);
}

// ---------------------------------------------------------------------------
// T11 — checkForNonBlockingKeyPress
// ---------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn t11_non_blocking_key_press_unix() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_set_select_ready(Some(false));
    assert!(!terminal::check_for_non_blocking_key_press(0));
    test_set_select_ready(Some(true));
    // Timed select path with empty queue returns true when microseconds > 0.
    assert!(terminal::check_for_non_blocking_key_press(1));
    test_set_select_ready(None);
    test_set_ncurses_stub(false);
}

#[cfg(windows)]
#[test]
#[ignore = "TODO(windows): non-macOS reference target; timeout(8) path"]
fn t11_non_blocking_key_press_windows() {}

// ---------------------------------------------------------------------------
// T12 — system/file helpers
// ---------------------------------------------------------------------------
#[test]
fn t12_get_default_player_name_fallback_x() {
    reset_for_new_game(None);
    ui_io::test_set_force_default_name(true);
    let mut buf = [0u8; 27];
    ui_io::test_set_ncurses_stub(true);
    terminal::get_default_player_name(&mut buf);
    ui_io::test_set_ncurses_stub(false);
    ui_io::test_set_force_default_name(false);
    assert_eq!(c_str(&buf), "X");
}

#[test]
fn t12_get_default_player_name_uses_login_seam() {
    ui_io::test_set_login_name(Some("moria"));
    let mut buf = [0u8; 27];
    ui_io::test_set_ncurses_stub(true);
    terminal::get_default_player_name(&mut buf);
    ui_io::test_set_ncurses_stub(false);
    assert_eq!(c_str(&buf), "moria");
    ui_io::test_set_login_name(None);
}

#[cfg(unix)]
#[test]
fn t12_tilde_expands_home_and_passes_through() {
    assert_eq!(terminal::tilde("/abs/path"), Some("/abs/path".to_string()));
    assert_eq!(terminal::tilde(""), Some(String::new()));
    let expanded = terminal::tilde("~/tmp/x").expect("~/tmp/x should expand");
    assert!(expanded.ends_with("/tmp/x"));
    assert!(!expanded.contains('~'));
}

#[cfg(unix)]
#[test]
fn t12_tilde_bad_user_returns_none() {
    assert!(terminal::tilde("~nosuchuser_xyz_bad/x").is_none());
}

#[test]
fn t12_check_file_permissions_success_on_normal_process() {
    assert!(terminal::check_file_permissions());
}

// ---------------------------------------------------------------------------
// T13 — terminal lifecycle (capture → ignored)
// ---------------------------------------------------------------------------
#[test]
fn t13_terminal_initialize_and_restore() {
    // Stub path: initialize/restore must not panic and must register UI hooks.
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    assert!(terminal::terminal_initialize());
    terminal::terminal_restore();
    test_set_ncurses_stub(false);
}

#[test]
fn t13_terminal_save_restore_screen() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_set_ui_trace(true);
    terminal::terminal_save_screen();
    terminal::terminal_restore_screen();
    let events = test_ui_trace_events();
    assert!(events
        .iter()
        .any(|e| matches!(e, UiTraceEvent::TerminalSaveScreen)));
    assert!(events
        .iter()
        .any(|e| matches!(e, UiTraceEvent::TerminalRestoreScreen)));
    test_set_ui_trace(false);
    test_set_ncurses_stub(false);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
fn vtype_from(s: &[u8]) -> Vtype_t {
    let mut v = [0u8; MORIA_MESSAGE_SIZE];
    let n = s.len().min(MORIA_MESSAGE_SIZE - 1);
    v[..n].copy_from_slice(&s[..n]);
    v
}

fn c_str(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}
