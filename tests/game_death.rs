//! Death handling (`game_death`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use std::path::Path;

use umoria::game::{
    reset_for_new_game, test_exit_program_called, test_reset_exit_program_called,
    test_set_skip_process_exit, with_state, with_state_mut,
};
use umoria::game_death::{
    end_game, test_display_equipment_called, test_display_inventory_range,
    test_identify_side_effect_count, test_kingly, test_kingly_called, test_print_crown,
    test_print_crown_called, test_print_tomb, test_print_tomb_called,
    test_recalculate_bonuses_count, test_reset_death_hooks, tomb_center_col,
};
use umoria::game_files::{
    test_output_player_character_call_count, test_output_player_character_last_path,
    test_reset_output_player_character_hooks, test_set_output_player_character_results,
};
use umoria::game_save::test_set_forced_seed_byte;
use umoria::inventory::PLAYER_INVENTORY_SIZE;
use umoria::player::{player_set_gender, PLAYER_MAX_LEVEL};
use umoria::scores::{
    test_character_saved_at_record, test_reset_score_test_hooks, test_set_scores_path,
};
use umoria::ui_io::{
    ctrl_key, test_clear_getch_keys, test_erase_lines, test_flush_input_buffer_count,
    test_push_getch_keys, test_put_strings_peek, test_set_eof_flag, test_set_ncurses_stub,
    test_set_ui_detail_capture, test_wait_continue_lines, ESCAPE,
};

fn setup_death_harness() {
    test_set_skip_process_exit(true);
    test_set_ncurses_stub(true);
    test_set_ui_detail_capture(true);
    test_set_eof_flag(0);
    test_clear_getch_keys();
    test_reset_exit_program_called();
    test_reset_death_hooks();
    test_reset_output_player_character_hooks();
    test_reset_score_test_hooks();
    reset_for_new_game(None);
}

fn set_name(name: &str) {
    with_state_mut(|state| {
        let bytes = name.as_bytes();
        for (slot, byte) in state.py.misc.name.iter_mut().enumerate() {
            *byte = if slot < bytes.len() { bytes[slot] } else { 0 };
        }
    });
}

fn set_died_from(text: &str) {
    with_state_mut(|state| {
        let bytes = text.as_bytes();
        for (slot, byte) in state.game.character_died_from.iter_mut().enumerate() {
            *byte = if slot < bytes.len() { bytes[slot] } else { 0 };
        }
    });
}

fn setup_nonwinner_tomb() {
    setup_death_harness();
    with_state_mut(|state| {
        state.dg.game_turn = 5;
        state.game.total_winner = false;
        state.py.misc.class_id = 0;
        state.py.misc.level = 5;
        state.py.misc.exp = 1234;
        state.py.misc.au = 567;
        state.dg.current_level = 12;
        state.py.misc.gender = true;
        // Generated characters always have non-zero ability bases.
        state.py.misc.bth = 10;
        state.py.misc.bth_with_bows = 10;
        state.py.misc.saving_throw = 10;
        state.py.misc.disarm = 10;
        state.py.misc.chance_in_search = 10;
    });
    set_name("Bob");
    set_died_from("Poison");
}

fn find_put(row: i32, col: i32) -> Option<String> {
    test_put_strings_peek()
        .into_iter()
        .find(|(y, x, _)| *y == row && *x == col)
        .map(|(_, _, text)| text)
}

fn enter_key() -> i32 {
    i32::from(ctrl_key(b'J'))
}

#[test]
fn test_tomb_layout_nonwinner() {
    setup_nonwinner_tomb();
    test_push_getch_keys(&[i32::from(ESCAPE)]);

    test_print_tomb();

    assert_eq!(find_put(6, tomb_center_col(3)), Some("Bob".to_string()));
    assert_eq!(
        find_put(8, tomb_center_col("Veteran(1st)".len())),
        Some("Veteran(1st)".to_string())
    );
    assert_eq!(
        find_put(10, tomb_center_col("Warrior".len())),
        Some("Warrior".to_string())
    );
    assert_eq!(find_put(11, 30), Some("5".to_string()));
    assert_eq!(
        find_put(12, tomb_center_col("1234 Exp".len())),
        Some("1234 Exp".to_string())
    );
    assert_eq!(
        find_put(13, tomb_center_col("567 Au".len())),
        Some("567 Au".to_string())
    );
    assert_eq!(find_put(14, 34), Some("12".to_string()));
    assert_eq!(
        find_put(16, tomb_center_col("Poison".len())),
        Some("Poison".to_string())
    );
    assert!(test_put_strings_peek()
        .iter()
        .any(|(y, _, text)| *y == 17 && !text.is_empty()));
    assert_eq!(
        find_put(23, 0),
        Some("(ESC to abort, return to print on screen, or file name)".to_string())
    );
    assert_eq!(find_put(22, 0), Some("Character record?".to_string()));
}

#[test]
fn test_tomb_centering_math() {
    for len in [0usize, 1, 2, 11, 25, 52] {
        assert_eq!(tomb_center_col(len), (26usize.wrapping_sub(len / 2)) as i32);
    }
}

#[test]
fn test_tomb_winner_titles() {
    setup_death_harness();
    with_state_mut(|state| {
        state.dg.game_turn = 1;
        state.game.total_winner = true;
        state.py.misc.gender = true;
    });
    set_name("Win");
    test_push_getch_keys(&[i32::from(ESCAPE)]);

    test_print_tomb();

    assert_eq!(
        find_put(8, tomb_center_col("Magnificent".len())),
        Some("Magnificent".to_string())
    );
    assert_eq!(
        find_put(10, tomb_center_col("*King*".len())),
        Some("*King*".to_string())
    );

    setup_death_harness();
    with_state_mut(|state| {
        state.dg.game_turn = 1;
        state.game.total_winner = true;
        state.py.misc.gender = false;
    });
    set_name("Win");
    test_push_getch_keys(&[i32::from(ESCAPE)]);

    test_print_tomb();

    assert_eq!(
        find_put(10, tomb_center_col("*Queen*".len())),
        Some("*Queen*".to_string())
    );
}

#[test]
fn test_retry_file_success() {
    setup_nonwinner_tomb();
    test_set_output_player_character_results(&[true]);
    test_push_getch_keys(&[enter_key(), b'x' as i32]);

    test_print_tomb();

    assert_eq!(test_output_player_character_call_count(), 1);
    assert_eq!(test_output_player_character_last_path(), "x");
    assert_eq!(test_flush_input_buffer_count(), 1);
}

#[test]
fn test_retry_file_fail_then_success() {
    setup_nonwinner_tomb();
    test_set_output_player_character_results(&[false, true]);
    test_push_getch_keys(&[enter_key(), b'x' as i32, enter_key(), b'x' as i32]);

    test_print_tomb();

    assert_eq!(test_output_player_character_call_count(), 2);
    assert_eq!(test_flush_input_buffer_count(), 2);
    assert_eq!(
        test_identify_side_effect_count(),
        2 * PLAYER_INVENTORY_SIZE as u32
    );
    assert_eq!(test_recalculate_bonuses_count(), 2);
}

#[test]
fn test_retry_esc_abort() {
    setup_nonwinner_tomb();
    test_push_getch_keys(&[i32::from(ESCAPE)]);

    test_print_tomb();

    assert_eq!(test_output_player_character_call_count(), 0);
    assert_eq!(test_identify_side_effect_count(), 0);
    assert_eq!(test_recalculate_bonuses_count(), 0);
}

#[test]
fn test_print_to_screen_empty() {
    setup_nonwinner_tomb();
    test_push_getch_keys(&[enter_key(), enter_key()]);

    test_print_tomb();

    assert!(test_display_equipment_called());
    assert_eq!(test_display_inventory_range(), Some((0, -1)));
}

#[test]
fn test_print_to_screen_esc_skip() {
    setup_nonwinner_tomb();
    test_push_getch_keys(&[enter_key(), i32::from(ESCAPE)]);

    test_print_tomb();

    assert!(!test_display_equipment_called());
    assert_eq!(test_display_inventory_range(), None);
}

#[test]
fn test_identify_all_inventory() {
    setup_nonwinner_tomb();
    test_set_output_player_character_results(&[true]);
    test_push_getch_keys(&[enter_key(), b'f' as i32]);

    test_print_tomb();

    assert_eq!(
        test_identify_side_effect_count(),
        PLAYER_INVENTORY_SIZE as u32
    );
    assert_eq!(test_recalculate_bonuses_count(), 1);
}

#[test]
fn test_crown_king() {
    setup_death_harness();
    player_set_gender(true);
    test_push_getch_keys(&[b' ' as i32]);

    test_print_crown();

    assert_eq!(find_put(17, 45), Some("King!".to_string()));
    assert_eq!(test_wait_continue_lines(), vec![23]);
}

#[test]
fn test_crown_queen() {
    setup_death_harness();
    player_set_gender(false);
    test_push_getch_keys(&[b' ' as i32]);

    test_print_crown();

    assert_eq!(find_put(17, 45), Some("Queen!".to_string()));
    assert_eq!(test_wait_continue_lines(), vec![23]);
}

#[test]
fn test_kingly_mutations() {
    setup_death_harness();
    with_state_mut(|state| {
        state.dg.current_level = 9;
        state.py.misc.level = 3;
        state.py.misc.au = 100;
        state.py.misc.max_exp = 10;
        state.py.misc.exp = 10;
        state.py.misc.gender = true;
    });
    test_push_getch_keys(&[b' ' as i32]);

    test_kingly();

    assert!(test_print_crown_called());
    assert_eq!(with_state(|state| state.dg.current_level), 0);
    assert_eq!(
        c_str(&with_state(|state| state.game.character_died_from)),
        "Ripe Old Age"
    );
    assert_eq!(
        with_state(|state| state.py.misc.level),
        3 + u16::from(PLAYER_MAX_LEVEL)
    );
    assert_eq!(with_state(|state| state.py.misc.au), 100 + 250_000);
    assert_eq!(with_state(|state| state.py.misc.max_exp), 10 + 5_000_000);
    assert_eq!(
        with_state(|state| state.py.misc.exp),
        with_state(|state| state.py.misc.max_exp)
    );
}

#[test]
fn test_endgame_winner_flow() {
    setup_death_harness();
    with_state_mut(|state| {
        state.dg.game_turn = 2;
        state.game.total_winner = true;
        state.game.character_generated = true;
        state.game.character_saved = true;
        state.game.noscore = 1;
    });
    test_push_getch_keys(&[i32::from(ESCAPE), b' ' as i32, i32::from(ESCAPE)]);

    end_game();

    assert!(test_kingly_called());
    assert!(test_print_tomb_called());
    assert!(test_exit_program_called());
}

#[test]
fn test_endgame_tomb_suppressed_when_saved() {
    setup_death_harness();
    with_state_mut(|state| {
        state.dg.game_turn = -1;
        state.game.character_generated = true;
        state.game.character_saved = true;
        state.game.noscore = 1;
    });

    end_game();

    assert!(!test_print_tomb_called());
    assert!(!test_kingly_called());
    assert!(test_exit_program_called());
}

#[test]
fn test_endgame_save_on_death() {
    setup_death_harness();
    let dir = std::env::temp_dir().join(format!("umoria_death544_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let save_path = dir.join("game.sav");

    with_state_mut(|state| {
        state.dg.game_turn = -1;
        state.game.character_generated = true;
        state.game.character_saved = false;
        state.game.noscore = 1;
        state.config_save_game = save_path.to_string_lossy().into_owned();
    });

    test_set_forced_seed_byte(Some(0x2A));
    test_set_scores_path(Some(dir.join("scores.dat").as_path()));

    end_game();

    assert!(save_path.is_file());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_endgame_score_trick() {
    setup_death_harness();
    with_state_mut(|state| {
        state.dg.game_turn = -1;
        state.game.character_generated = true;
        state.game.character_saved = true;
        state.game.noscore = 1;
    });
    test_set_scores_path(Some(Path::new("scores.dat")));

    end_game();

    assert_eq!(test_character_saved_at_record(), Some(false));
    assert!(test_exit_program_called());
}

#[test]
fn test_endgame_exit() {
    setup_death_harness();
    with_state_mut(|state| {
        state.dg.game_turn = -1;
        state.game.character_generated = false;
    });

    end_game();

    assert!(test_erase_lines().contains(&(23, 0)));
    assert!(test_exit_program_called());
}

fn c_str(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}
