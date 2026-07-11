//! `game` lifecycle residuals.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::game::{
    abort_program_output, format_option_line, game_options_table, get_all_directions,
    get_direction_with_memory, get_random_direction, is_current_game_version,
    map_roguelike_keys_to_keypad, reset_for_new_game, test_pre_exit_sequence,
    test_reset_exit_program_called, test_set_capture_abort, test_set_direction,
    test_set_skip_process_exit, test_take_abort_output, valid_game_version, with_state,
    with_state_mut, Options,
};
use umoria::game_save::{test_apply_options_from_l, test_build_options_l};
use umoria::ui_io::{
    self, register_game_ui_hooks, terminal::Coord, test_bell_count, test_clear_getch_keys,
    test_move_cursors, test_push_getch_keys, test_put_strings, test_set_ncurses_stub,
    test_set_ui_capture, test_set_ui_detail_capture, ESCAPE,
};

fn setup_harness() {
    test_set_ncurses_stub(true);
    register_game_ui_hooks();
    reset_for_new_game(None);
}

// --------------------------------------------------------------------------
// Step 0 — game global coordination
// --------------------------------------------------------------------------

#[test]
fn step0_state_default_fields() {
    reset_for_new_game(None);
    with_state(|state| {
        assert!(!state.game.player_free_turn);
        assert!(!state.game.use_last_direction);
        assert_eq!(state.game.command_count, 0);
    });
}

#[test]
fn step0_lifecycle_signatures_compile() {
    let _ = valid_game_version as fn(u8, u8, u8) -> bool;
    let _ = is_current_game_version as fn(u8, u8, u8) -> bool;
    let _ = get_random_direction as fn() -> i32;
    let _ = map_roguelike_keys_to_keypad as fn(u8) -> u8;
    let _ = umoria::game::set_game_options as fn();
    let _ = get_direction_with_memory as fn(Option<&str>, &mut i32) -> bool;
    let _ = get_all_directions as fn(&str, &mut i32) -> bool;
    let _ = umoria::game::exit_program as fn();
    let _ = umoria::game::abort_program as fn(&str);
}

// --------------------------------------------------------------------------
// Step 1 — validGameVersion / isCurrentGameVersion
// --------------------------------------------------------------------------

#[test]
fn step1_valid_game_version_accepted() {
    for (maj, min, patch) in [
        (5, 2, 2),
        (5, 2, 3),
        (5, 3, 0),
        (5, 7, 0),
        (5, 7, 15),
        (5, 7, 255),
    ] {
        assert!(
            valid_game_version(maj, min, patch),
            "{maj}.{min}.{patch} should be valid"
        );
    }
}

#[test]
fn step1_valid_game_version_rejected() {
    for (maj, min, patch) in [
        (5, 2, 1),
        (5, 2, 0),
        (5, 1, 9),
        (5, 0, 0),
        (5, 8, 0),
        (4, 7, 0),
        (6, 0, 0),
        (0, 0, 0),
    ] {
        assert!(
            !valid_game_version(maj, min, patch),
            "{maj}.{min}.{patch} should be invalid"
        );
    }
}

#[test]
fn step1_is_current_game_version() {
    assert!(is_current_game_version(5, 7, 15));
    assert!(!is_current_game_version(5, 7, 14));
    assert!(!is_current_game_version(5, 6, 15));
    assert!(!is_current_game_version(4, 7, 15));
    assert!(!is_current_game_version(5, 7, 16));
}

// --------------------------------------------------------------------------
// Step 2 — getRandomDirection / mapRoguelikeKeysToKeypad
// --------------------------------------------------------------------------

#[test]
fn step2_map_roguelike_keys_to_keypad_mappings() {
    assert_eq!(map_roguelike_keys_to_keypad(b'h'), b'4');
    assert_eq!(map_roguelike_keys_to_keypad(b'y'), b'7');
    assert_eq!(map_roguelike_keys_to_keypad(b'k'), b'8');
    assert_eq!(map_roguelike_keys_to_keypad(b'u'), b'9');
    assert_eq!(map_roguelike_keys_to_keypad(b'l'), b'6');
    assert_eq!(map_roguelike_keys_to_keypad(b'n'), b'3');
    assert_eq!(map_roguelike_keys_to_keypad(b'j'), b'2');
    assert_eq!(map_roguelike_keys_to_keypad(b'b'), b'1');
    assert_eq!(map_roguelike_keys_to_keypad(b'.'), b'5');
}

#[test]
fn step2_map_roguelike_keys_passthrough() {
    for key in [b'x', b'0', b'5', b'H', ESCAPE] {
        assert_eq!(map_roguelike_keys_to_keypad(key), key);
    }
}

#[test]
fn step2_get_random_direction_golden_and_invariants() {
    const GOLDEN: [i32; 30] = [
        2, 9, 4, 7, 3, 1, 1, 4, 3, 8, 8, 2, 8, 2, 4, 7, 6, 8, 6, 3, 2, 2, 3, 4, 9, 4, 2, 2, 1, 7,
    ];

    reset_for_new_game(Some(42));
    for (idx, &expected) in GOLDEN.iter().enumerate() {
        let dir = get_random_direction();
        assert_eq!(dir, expected, "draw {idx}");
        assert!((1..=9).contains(&dir));
        assert_ne!(dir, 5);
    }

    for _ in 0..970 {
        let dir = get_random_direction();
        assert!((1..=9).contains(&dir));
        assert_ne!(dir, 5);
    }
}

// --------------------------------------------------------------------------
// Step 3 — game_options[] + setGameOptions
// --------------------------------------------------------------------------

const OPTION_PROMPTS: [&str; 11] = [
    "Running: cut known corners",
    "Running: examine potential corners",
    "Running: print self during run",
    "Running: stop when map sector changes",
    "Running: run through open doors",
    "Prompt to pick up objects",
    "Rogue like commands",
    "Show weights in inventory",
    "Highlight and notice mineral seams",
    "Beep for invalid character",
    "Display rest/repeat counts",
];

fn clear_options(options: &mut Options) {
    *options = Options {
        display_counts: false,
        find_bound: false,
        run_cut_corners: false,
        run_examine_corners: false,
        run_ignore_doors: false,
        run_print_self: false,
        highlight_seams: false,
        prompt_to_pickup: false,
        use_roguelike_keys: false,
        show_inventory_weights: false,
        error_beep_sound: false,
    };
}

#[test]
fn step3_game_options_table_integrity() {
    let table = game_options_table();
    assert_eq!(table.len(), 11);
    for (index, entry) in table.iter().enumerate() {
        assert_eq!(entry.prompt, OPTION_PROMPTS[index]);
        reset_for_new_game(None);
        with_state_mut(|state| {
            clear_options(&mut state.options);
            (entry.set)(&mut state.options, true);
            assert!((entry.get)(&state.options));
            (entry.set)(&mut state.options, false);
            assert!(!(entry.get)(&state.options));
        });
    }
}

#[test]
fn step3_format_option_line_layout() {
    let line_yes = format_option_line("Running: cut known corners", true);
    let line_no = format_option_line("Running: cut known corners", false);
    assert_eq!(line_yes.len(), line_no.len());
    assert!(line_yes.ends_with(": yes"));
    assert!(line_no.ends_with(": no "));
    assert_eq!(line_yes.len() - 5, 38);
    assert_eq!(line_no.len() - 5, 38);
}

type OptionMaskEntry = (fn(&mut Options), fn(&Options) -> bool, u32);

#[test]
fn step3_options_bitfield_roundtrip() {
    let masks: [OptionMaskEntry; 11] = [
        (|o| o.run_cut_corners = true, |o| o.run_cut_corners, 0x1),
        (
            |o| o.run_examine_corners = true,
            |o| o.run_examine_corners,
            0x2,
        ),
        (|o| o.run_print_self = true, |o| o.run_print_self, 0x4),
        (|o| o.find_bound = true, |o| o.find_bound, 0x8),
        (|o| o.prompt_to_pickup = true, |o| o.prompt_to_pickup, 0x10),
        (
            |o| o.use_roguelike_keys = true,
            |o| o.use_roguelike_keys,
            0x20,
        ),
        (
            |o| o.show_inventory_weights = true,
            |o| o.show_inventory_weights,
            0x40,
        ),
        (|o| o.highlight_seams = true, |o| o.highlight_seams, 0x80),
        (|o| o.run_ignore_doors = true, |o| o.run_ignore_doors, 0x100),
        (|o| o.error_beep_sound = true, |o| o.error_beep_sound, 0x200),
        (|o| o.display_counts = true, |o| o.display_counts, 0x400),
    ];

    for (set_flag, get_flag, mask) in masks {
        reset_for_new_game(None);
        with_state_mut(|state| {
            clear_options(&mut state.options);
            set_flag(&mut state.options);
        });
        let l = test_build_options_l();
        assert_eq!(l & mask, mask);
        assert_eq!(l & !mask, 0);

        reset_for_new_game(None);
        with_state_mut(|state| clear_options(&mut state.options));
        test_apply_options_from_l(l);
        assert!(with_state(|state| get_flag(&state.options)));
    }

    reset_for_new_game(None);
    with_state_mut(|state| {
        clear_options(&mut state.options);
        for entry in game_options_table() {
            (entry.set)(&mut state.options, true);
        }
    });
    let all_true = test_build_options_l();
    reset_for_new_game(None);
    with_state_mut(|state| clear_options(&mut state.options));
    test_apply_options_from_l(all_true);
    with_state(|state| {
        for entry in game_options_table() {
            assert!((entry.get)(&state.options));
        }
    });

    reset_for_new_game(None);
    with_state_mut(|state| clear_options(&mut state.options));
    assert_eq!(test_build_options_l(), 0);
    test_apply_options_from_l(0);
    with_state(|state| {
        for entry in game_options_table() {
            assert!(!(entry.get)(&state.options));
        }
    });

    reset_for_new_game(None);
    with_state_mut(|state| clear_options(&mut state.options));
    test_apply_options_from_l(0x555);
    with_state(|state| {
        assert!(state.options.run_cut_corners);
        assert!(!state.options.run_examine_corners);
        assert!(state.options.run_print_self);
        assert!(!state.options.find_bound);
        assert!(state.options.prompt_to_pickup);
    });
}

#[test]
fn step3_set_game_options_interactive_loop() {
    setup_harness();
    test_set_ui_capture(true);
    test_set_ui_detail_capture(true);
    test_clear_getch_keys();

    let before = with_state(|state| state.options.clone());

 // ESC immediately — no changes.
    test_push_getch_keys(&[i32::from(ESCAPE)]);
    umoria::game::set_game_options();
    assert_eq!(with_state(|state| state.options.clone()), before);

 // Navigate to idx 6, then y (use_roguelike_keys).
    setup_harness();
    test_set_ui_capture(true);
    test_set_ui_detail_capture(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[
        i32::from(ESCAPE),
        i32::from(b'y'),
        i32::from(b' '),
        i32::from(b' '),
        i32::from(b' '),
        i32::from(b' '),
        i32::from(b' '),
        i32::from(b' '),
    ]);
    umoria::game::set_game_options();
    assert!(with_state(|state| state.options.use_roguelike_keys));
    assert!(test_put_strings()
        .iter()
        .any(|(y, x, text)| text == "yes" && *y == 7 && *x == 40));

 // '-' at idx 0 wraps to 10; space at idx 10 wraps to 0.
    setup_harness();
    test_set_ui_detail_capture(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(ESCAPE), i32::from(b' '), i32::from(b'-')]);
    umoria::game::set_game_options();
    let moves = test_move_cursors();
    assert!(moves.contains(&Coord { y: 11, x: 40 }));
    assert!(moves.contains(&Coord { y: 1, x: 40 }));

 // '-' wraps to idx 10 (display_counts); n sets false.
    setup_harness();
    with_state_mut(|state| state.options.display_counts = true);
    test_set_ui_capture(true);
    test_set_ui_detail_capture(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(ESCAPE), i32::from(b'n'), i32::from(b'-')]);
    umoria::game::set_game_options();
    assert!(!with_state(|state| state.options.display_counts));
    assert!(test_put_strings()
        .iter()
        .any(|(y, x, text)| text == "no " && *y == 11 && *x == 40));

 // Unknown key → bell, no state change.
    setup_harness();
    let snapshot = with_state(|state| state.options.clone());
    test_set_ui_detail_capture(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(ESCAPE), i32::from(b'z')]);
    umoria::game::set_game_options();
    assert_eq!(with_state(|state| state.options.clone()), snapshot);
    assert_eq!(test_bell_count(), 1);

 // Initial row rendering uses %-38s layout.
    setup_harness();
    test_set_ui_capture(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(ESCAPE)]);
    umoria::game::set_game_options();
    let messages = ui_io::test_ui_messages();
    assert!(messages
        .iter()
        .any(|m| { m.starts_with("  ESC when finished") }));
    assert!(messages
        .iter()
        .any(|m| { m == &format_option_line("Running: cut known corners", true) }));
}

// --------------------------------------------------------------------------
// Step 4 — getDirectionWithMemory
// --------------------------------------------------------------------------

#[test]
fn step4_get_direction_with_memory() {
    setup_harness();
    test_set_direction(None);

    with_state_mut(|state| {
        state.game.use_last_direction = true;
        state.py.prev_dir = 6;
    });
    let mut dir = 0;
    assert!(get_direction_with_memory(None, &mut dir));
    assert_eq!(dir, 6);

    setup_harness();
    test_set_direction(None);
    with_state_mut(|state| state.game.use_last_direction = false);

    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'4')]);
    dir = 0;
    assert!(get_direction_with_memory(None, &mut dir));
    assert_eq!(dir, 4);
    assert_eq!(with_state(|state| state.py.prev_dir), 4);

    setup_harness();
    test_set_ui_detail_capture(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'4'), i32::from(b'5')]);
    dir = 0;
    assert!(get_direction_with_memory(None, &mut dir));
    assert_eq!(dir, 4);
    assert_eq!(test_bell_count(), 1);

    setup_harness();
    with_state_mut(|state| state.options.use_roguelike_keys = true);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'h')]);
    dir = 0;
    assert!(get_direction_with_memory(None, &mut dir));
    assert_eq!(dir, 4);

    setup_harness();
    with_state_mut(|state| state.game.command_count = 7);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'2')]);
    dir = 0;
    assert!(get_direction_with_memory(None, &mut dir));
    assert_eq!(with_state(|state| state.game.command_count), 7);

    setup_harness();
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(ESCAPE)]);
    dir = 0;
    assert!(!get_direction_with_memory(
        Some("Which direction?"),
        &mut dir
    ));
    assert!(with_state(|state| state.game.player_free_turn));
}

// --------------------------------------------------------------------------
// Step 5 — getAllDirections
// --------------------------------------------------------------------------

#[test]
fn step5_get_all_directions() {
    setup_harness();

    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'5')]);
    let mut dir = 0;
    assert!(get_all_directions("Look?", &mut dir));
    assert_eq!(dir, 5);

    setup_harness();
    let prev = with_state(|state| state.py.prev_dir);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'2')]);
    dir = 0;
    assert!(get_all_directions("Look?", &mut dir));
    assert_eq!(dir, 2);
    assert_eq!(with_state(|state| state.py.prev_dir), prev);

    setup_harness();
    with_state_mut(|state| state.options.use_roguelike_keys = true);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'.')]);
    dir = 0;
    assert!(get_all_directions("Look?", &mut dir));
    assert_eq!(dir, 5);

    setup_harness();
    with_state_mut(|state| state.game.command_count = 9);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'3')]);
    dir = 0;
    assert!(get_all_directions("Look?", &mut dir));
    assert_eq!(with_state(|state| state.game.command_count), 0);

    setup_harness();
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(ESCAPE)]);
    dir = 0;
    assert!(!get_all_directions("Look?", &mut dir));
    assert!(with_state(|state| state.game.player_free_turn));
}

// --------------------------------------------------------------------------
// Step 6 — exitProgram / abortProgram
// --------------------------------------------------------------------------

#[test]
fn step6_exit_program_pre_exit_sequence() {
    setup_harness();
    test_reset_exit_program_called();
    test_set_skip_process_exit(true);
    test_pre_exit_sequence();
    assert!(umoria::game::test_exit_program_called());
}

#[test]
fn step6_abort_program_message_bytes() {
    setup_harness();
    test_set_skip_process_exit(true);
    test_set_capture_abort(true);
    umoria::game::abort_program("test reason");
    assert_eq!(
        test_take_abort_output(),
        Some(abort_program_output("test reason"))
    );
    assert_eq!(
        abort_program_output("test reason"),
        "Program was manually aborted with the message:\ntest reason\n"
    );
}

#[test]
#[should_panic(expected = "with_state re-entered while with_state_mut")]
fn nested_with_state_inside_with_state_mut_panics_clearly() {
    reset_for_new_game(Some(1));
    with_state_mut(|_| {
        let _ = with_state(|s| s.py.misc.level);
    });
}

#[test]
#[should_panic(expected = "with_state_mut re-entered while game state is already borrowed")]
fn nested_with_state_mut_inside_with_state_panics_clearly() {
    reset_for_new_game(Some(1));
    with_state(|_| {
        with_state_mut(|s| s.py.misc.level = 1);
    });
}
