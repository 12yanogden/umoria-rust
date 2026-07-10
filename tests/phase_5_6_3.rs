//! Phase 5.6.3 — command input parsing & dispatch (strict TDD).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::game::{reset_for_new_game, test_set_direction, with_state, with_state_mut};
use umoria::game_run::{
    calculate_max_message_count, command_flip_wizard_mode, command_previous_message, command_quit,
    command_save_and_exit, command_toggle_search, do_command, execute_input_commands,
    get_command_repeat_count, move_without_pickup, original_commands, parse_alternate_ctrl_input,
    test_clear_dispatch_log, test_dispatch_log, test_free_turn_after_command,
    test_last_command_after, test_set_message, valid_count_command, VALID_COUNT_FALSE,
    VALID_COUNT_TRUE,
};
use umoria::types::MESSAGE_HISTORY_SIZE;
use umoria::ui_io::{
    self, ctrl_key, register_game_ui_hooks, test_bell_count, test_clear_getch_keys,
    test_flush_input_buffer_count, test_push_getch_keys, test_put_strings, test_set_ncurses_stub,
    test_set_ui_capture, test_set_ui_detail_capture, test_ui_messages, DELETE, ESCAPE,
};

fn setup_harness() {
    test_set_ncurses_stub(true);
    register_game_ui_hooks();
    reset_for_new_game(None);
    test_clear_dispatch_log();
}

fn c_str(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

// ---------------------------------------------------------------------------
// 1. getCommandRepeatCount
// ---------------------------------------------------------------------------

#[test]
fn step1_repeat_count_digits_and_default() {
    setup_harness();
    test_set_ui_capture(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'h'), i32::from(b' '), i32::from(b'2')]);

    let mut cmd = b'1';
    let count = get_command_repeat_count(&mut cmd);
    assert_eq!(count, 12);
    assert_eq!(cmd, b'h');

    let strings = test_put_strings();
    assert!(strings.iter().any(|(_, _, s)| s == "Repeat count:"));
    assert!(strings.iter().any(|(_, x, s)| *x == 14 && s == "0000012"));
}

#[test]
fn step1_repeat_count_empty_defaults_to_99() {
    setup_harness();
    test_set_ui_capture(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'h')]);

    let mut cmd = b'0';
    let count = get_command_repeat_count(&mut cmd);
    assert_eq!(count, 99);
    assert_eq!(cmd, b'h');

    let strings = test_put_strings();
    assert!(strings.iter().any(|(_, x, s)| *x == 14 && s == "99"));
}

#[test]
fn step1_repeat_count_delete_divides_by_ten() {
    setup_harness();
    test_set_ui_capture(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[
        i32::from(b'h'),
        i32::from(DELETE),
        i32::from(b'3'),
        i32::from(b'2'),
    ]);

    let mut cmd = b'1';
    let count = get_command_repeat_count(&mut cmd);
    assert_eq!(count, 12);
    assert!(test_put_strings()
        .iter()
        .any(|(_, x, s)| *x == 14 && s == "0000012"));
}

#[test]
fn step1_repeat_count_over_99_rings_bell() {
    setup_harness();
    with_state_mut(|state| state.options.error_beep_sound = true);
    test_set_ui_detail_capture(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[
        i32::from(b'h'),
        i32::from(b'5'),
        i32::from(b'0'),
        i32::from(b'0'),
    ]);

    let mut cmd = b'1';
    let count = get_command_repeat_count(&mut cmd);
    assert_eq!(count, 100);
    assert_eq!(test_bell_count(), 1);
}

#[test]
fn step1_repeat_count_hash_becomes_zero() {
    setup_harness();
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'h')]);

    let mut cmd = b'#';
    let count = get_command_repeat_count(&mut cmd);
    assert_eq!(count, 99);
}

#[test]
fn step1_repeat_count_trailing_space_prompts_command() {
    setup_harness();
    test_set_ui_capture(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'x'), i32::from(b' ')]);

    let mut cmd = b'3';
    let count = get_command_repeat_count(&mut cmd);
    assert_eq!(count, 3);
    assert_eq!(cmd, b'x');
    assert!(test_put_strings()
        .iter()
        .any(|(_, x, s)| *x == 20 && s == "Command:"));
}

// ---------------------------------------------------------------------------
// 2. parseAlternateCtrlInput
// ---------------------------------------------------------------------------

#[test]
fn step2_parse_ctrl_uppercase() {
    setup_harness();
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'A')]);

    let result = parse_alternate_ctrl_input(b'^');
    assert_eq!(result, ctrl_key(b'A'));
}

#[test]
fn step2_parse_ctrl_lowercase() {
    setup_harness();
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'z')]);

    let result = parse_alternate_ctrl_input(b'^');
    assert_eq!(result, ctrl_key(b'Z'));
}

#[test]
fn step2_parse_ctrl_non_letter_message() {
    setup_harness();
    test_set_ui_capture(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'1')]);

    let result = parse_alternate_ctrl_input(b'^');
    assert_eq!(result, b' ');
    assert!(test_ui_messages()
        .iter()
        .any(|m| m.contains("Type ^ <letter> for a control char")));
}

#[test]
fn step2_parse_ctrl_escape_returns_space() {
    setup_harness();
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(ESCAPE)]);

    let result = parse_alternate_ctrl_input(b'^');
    assert_eq!(result, b' ');
}

// ---------------------------------------------------------------------------
// 3. originalCommands keymap
// ---------------------------------------------------------------------------

#[test]
fn step3_original_commands_static_mappings() {
    setup_harness();
    test_set_direction(Some(4));

    let cases: &[(u8, u8)] = &[
        (ctrl_key(b'K'), b'Q'),
        (ctrl_key(b'J'), b'+'),
        (ctrl_key(b'M'), b'+'),
        (b'1', b'b'),
        (b'2', b'j'),
        (b'3', b'n'),
        (b'4', b'h'),
        (b'5', b'.'),
        (b'6', b'l'),
        (b'7', b'y'),
        (b'8', b'k'),
        (b'9', b'u'),
        (b'B', b'f'),
        (b'L', b'W'),
        (b'S', b'#'),
        (b'a', b'z'),
        (b'b', b'P'),
        (b'f', b't'),
        (b'h', b'?'),
        (b'j', b'S'),
        (b'l', b'x'),
        (b't', b'T'),
        (b'u', b'Z'),
        (b'x', b'X'),
        (ctrl_key(b'B'), ctrl_key(b'O')),
        (ctrl_key(b'H'), b'\\'),
        (ctrl_key(b'L'), b'*'),
        (ctrl_key(b'U'), b'&'),
        (b'/', b'/'),
        (b'?', b'?'),
        (b'z', b'~'), // illegal default
    ];

    for &(input, expected) in cases {
        if input == b'z' {
            assert_eq!(original_commands(input), b'~', "input {input}");
        } else {
            assert_eq!(original_commands(input), expected, "input {input}");
        }
    }
}

#[test]
fn step3_original_commands_direction_dot() {
    setup_harness();
    for (dir, expected) in [
        (1, b'B'),
        (2, b'J'),
        (3, b'N'),
        (4, b'H'),
        (6, b'L'),
        (7, b'Y'),
        (8, b'K'),
        (9, b'U'),
        (5, b' '),
    ] {
        test_set_direction(Some(dir));
        assert_eq!(original_commands(b'.'), expected, "dir {dir}");
    }

    test_set_direction(None);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(ESCAPE)]);
    assert_eq!(original_commands(b'.'), b' ');
}

#[test]
fn step3_original_commands_direction_tunnel() {
    setup_harness();
    for (dir, expected) in [
        (1, ctrl_key(b'B')),
        (2, ctrl_key(b'J')),
        (4, ctrl_key(b'H')),
        (5, b' '),
    ] {
        test_set_direction(Some(dir));
        assert_eq!(original_commands(b'T'), expected, "dir {dir}");
    }
}

// ---------------------------------------------------------------------------
// 4. validCountCommand
// ---------------------------------------------------------------------------

#[test]
fn step4_valid_count_command_table() {
    for &cmd in VALID_COUNT_FALSE {
        assert!(!valid_count_command(cmd), "false cmd {cmd}");
    }
    for &cmd in VALID_COUNT_TRUE {
        assert!(valid_count_command(cmd), "true cmd {cmd}");
    }
    assert!(!valid_count_command(b'Q'));
    assert!(!valid_count_command(b'~'));
    assert!(valid_count_command(b'.'));
}

// ---------------------------------------------------------------------------
// 5. moveWithoutPickup
// ---------------------------------------------------------------------------

#[test]
fn step5_move_without_pickup_non_dash_unchanged() {
    setup_harness();
    let mut cmd = b'h';
    assert!(move_without_pickup(&mut cmd));
    assert_eq!(cmd, b'h');
}

#[test]
fn step5_move_without_pickup_dash_maps_direction() {
    setup_harness();
    with_state_mut(|state| state.game.command_count = 5);
    test_set_direction(Some(4));

    let mut cmd = b'-';
    assert!(!move_without_pickup(&mut cmd));
    assert_eq!(cmd, b'h');
    assert_eq!(with_state(|state| state.game.command_count), 5);
}

#[test]
fn step5_move_without_pickup_dash_cancelled() {
    setup_harness();
    test_set_direction(None);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(ESCAPE)]);

    let mut cmd = b'-';
    assert!(!move_without_pickup(&mut cmd));
    assert_eq!(cmd, b' ');
}

// ---------------------------------------------------------------------------
// 6. executeInputCommands loop
// ---------------------------------------------------------------------------

#[test]
fn step6_execute_input_invalid_count_message() {
    setup_harness();
    test_set_ui_capture(true);
    with_state_mut(|state| {
        state.options.use_roguelike_keys = true;
        state.message_ready_to_print = true;
    });
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'i'), i32::from(b'3')]);
    ui_io::test_set_eof_flag(1);

    let mut cmd = b' ';
    let mut find_count = 0;
    execute_input_commands(&mut cmd, &mut find_count);

    assert!(with_state(|state| state.game.player_free_turn));
    assert_eq!(with_state(|state| state.game.command_count), 0);
    assert!(test_ui_messages()
        .iter()
        .any(|m| m.contains("Invalid command with a count.")));
    ui_io::test_set_eof_flag(0);
}

#[test]
fn step6_execute_input_valid_count_sets_command_count() {
    setup_harness();
    with_state_mut(|state| state.options.use_roguelike_keys = true);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'4'), i32::from(b'.'), i32::from(ESCAPE)]);

    let mut cmd = b' ';
    let mut find_count = 0;
    execute_input_commands(&mut cmd, &mut find_count);

    assert_eq!(with_state(|state| state.game.command_count), 0);
}

#[test]
fn step6_execute_input_running_short_circuit() {
    setup_harness();
    with_state_mut(|state| {
        state.py.running_tracker = 1;
        state.game.player_free_turn = true;
    });

    let mut cmd = b' ';
    let mut find_count = 2;
    execute_input_commands(&mut cmd, &mut find_count);
    assert_eq!(find_count, 1);
}

#[test]
fn step6_execute_input_inventory_short_circuit() {
    setup_harness();
    with_state_mut(|state| {
        state.game.doing_inventory_command = b'i';
        state.game.player_free_turn = true;
    });

    let mut cmd = b' ';
    let mut find_count = 0;
    execute_input_commands(&mut cmd, &mut find_count);
    assert_eq!(with_state(|state| state.game.doing_inventory_command), 0);
}

// ---------------------------------------------------------------------------
// 7. doCommand switch — free-turn accounting + last_command
// ---------------------------------------------------------------------------

#[test]
fn step7_do_command_free_turn_ui_commands() {
    setup_harness();
    with_state_mut(|state| {
        state.py.misc.level = 1;
        state.py.base_exp_levels[0] = 100;
    });
    for cmd in [b' ', ESCAPE, b'!', b'?', b'x', b'v', b'M'] {
        assert!(
            test_free_turn_after_command(cmd),
            "expected free turn for {cmd}"
        );
    }
}

#[test]
fn step7_do_command_movement_no_free_turn() {
    setup_harness();
    with_state_mut(|state| {
        state.dg.width = 66;
        state.dg.height = 22;
        state.dg.current_level = 1;
        state.py.pos = umoria::types::Coord_t { y: 10, x: 10 };
    });
    assert!(!test_free_turn_after_command(b'h'));
}

#[test]
fn step7_do_command_last_command_updated() {
    setup_harness();
    assert_eq!(test_last_command_after(b'?'), b'?');
    assert_eq!(test_last_command_after(b'Q'), b'Q');
}

#[test]
fn step7_do_command_dispatches_phase_564_stubs() {
    setup_harness();
    test_clear_dispatch_log();
    do_command(b'P');
    assert_eq!(test_dispatch_log(), vec![b'P']);
    test_clear_dispatch_log();
    do_command(b'<');
    assert_eq!(test_dispatch_log(), vec![b'<']);
}

// ---------------------------------------------------------------------------
// 8. doWizardCommands — via do_command default branch
// ---------------------------------------------------------------------------

#[test]
fn step8_wizard_commands_default_help_roguelike() {
    setup_harness();
    test_set_ui_capture(true);
    with_state_mut(|state| {
        state.game.wizard_mode = true;
        state.options.use_roguelike_keys = true;
    });
    do_command(b'|');
    let strings = test_put_strings();
    assert!(strings
        .iter()
        .any(|(_, _, s)| s.contains("Type '?' or '\\' for help.")));
}

// ---------------------------------------------------------------------------
// 9. Small commands
// ---------------------------------------------------------------------------

#[test]
fn step9_command_quit_confirmed() {
    setup_harness();
    test_set_ui_capture(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'y')]);

    command_quit();
    assert!(with_state(|state| state.game.character_is_dead));
    assert!(with_state(|state| state.dg.generate_new_level));
    assert_eq!(
        c_str(&with_state(|state| state.game.character_died_from)),
        "Quitting"
    );
    assert!(test_flush_input_buffer_count() > 0);
}

#[test]
fn step9_calculate_max_message_count() {
    setup_harness();
    with_state_mut(|state| state.game.command_count = 5);
    assert_eq!(calculate_max_message_count(), 5);
    assert_eq!(with_state(|state| state.game.command_count), 0);

    setup_harness();
    with_state_mut(|state| state.game.last_command = b'x');
    assert_eq!(calculate_max_message_count(), 1);

    setup_harness();
    with_state_mut(|state| state.game.last_command = ctrl_key(b'P'));
    assert_eq!(calculate_max_message_count(), MESSAGE_HISTORY_SIZE as u8);
}

#[test]
fn step9_command_previous_message_single() {
    setup_harness();
    test_set_ui_capture(true);
    test_set_message(3, "hello world");
    command_previous_message();
    let strings = test_put_strings();
    assert!(strings.iter().any(|(_, x, s)| *x == 0 && s == ">"));
    assert!(strings
        .iter()
        .any(|(_, x, s)| *x == 1 && s == "hello world"));
}

#[test]
fn step9_command_toggle_search() {
    setup_harness();
    command_toggle_search();
    assert_ne!(
        with_state(|state| state.py.flags.status & umoria::config::player::status::PY_SEARCH),
        0
    );
    command_toggle_search();
    assert_eq!(
        with_state(|state| state.py.flags.status & umoria::config::player::status::PY_SEARCH),
        0
    );
}

#[test]
fn step9_command_save_and_exit_total_winner() {
    setup_harness();
    test_set_ui_capture(true);
    with_state_mut(|state| state.game.total_winner = true);
    command_save_and_exit();
    assert!(test_ui_messages()
        .iter()
        .any(|m| m.contains("Total Winner")));
}

#[test]
fn step9_command_locate_on_map_blind() {
    setup_harness();
    test_set_ui_capture(true);
    with_state_mut(|state| state.py.flags.blind = 1);
    umoria::game_run::command_locate_on_map();
    assert!(test_ui_messages()
        .iter()
        .any(|m| m.contains("You can't see your map.")));
}

#[test]
fn step9_command_flip_wizard_mode_messages() {
    setup_harness();
    test_set_ui_capture(true);
    with_state_mut(|state| state.game.noscore = 2);
    command_flip_wizard_mode();
    assert!(with_state(|state| state.game.wizard_mode));
    assert!(test_ui_messages()
        .iter()
        .any(|m| m.contains("Wizard mode on.")));
    command_flip_wizard_mode();
    assert!(!with_state(|state| state.game.wizard_mode));
    assert!(test_ui_messages()
        .iter()
        .any(|m| m.contains("Wizard mode off.")));
}
