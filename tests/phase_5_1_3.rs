//! Phase 5.1.3 — loadGame (strict TDD).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use std::fs;

use common::{byte_diff, load_manifest, read_golden_bytes};
use umoria::dungeon::MAX_HEIGHT;
use umoria::game::{
    reset_for_new_game, test_exit_program_called, test_reset_exit_program_called, with_state,
    with_state_mut,
};
use umoria::game_save::{
    self, from_save_file, load_game, save_char, set_from_save_file, set_start_time, set_xor_byte,
    test_buffer_bytes, test_buffer_inject, test_build_options_l, test_load_save_from_bytes,
    test_reset_buffer, test_reset_store_maintenance_count, test_set_forced_seed_byte,
    test_set_unix_time, test_store_maintenance_count, wr_byte, wr_long, wr_short,
};
use umoria::inventory::PlayerEquipment;
use umoria::monster::MON_MAX_CREATURES;
use umoria::recall::Recall;
use umoria::ui_io::{self, test_push_getch_keys};

fn setup_load_harness() {
    ui_io::test_set_ncurses_stub(true);
    ui_io::test_set_ui_capture(true);
    test_reset_exit_program_called();
    test_reset_store_maintenance_count();
    reset_for_new_game(None);
    with_state_mut(|state| {
        state.dg.game_turn = -1;
        state.config_save_game = "game.sav".to_string();
    });
}

fn setup_buffer_save() {
    ui_io::test_set_ncurses_stub(true);
    test_reset_buffer();
    reset_for_new_game(None);
    with_state_mut(|state| {
        state.game.character_saved = false;
        state.dg.game_turn = 1;
        state.py.pack.heaviness = 0;
    });
}

fn c_str(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn write_header(seed: u8, maj: u8, min: u8, patch: u8) {
    test_set_forced_seed_byte(Some(seed));
    set_xor_byte(0);
    wr_byte(maj).unwrap();
    set_xor_byte(0);
    wr_byte(min).unwrap();
    set_xor_byte(0);
    wr_byte(patch).unwrap();
    set_xor_byte(0);
    wr_byte(seed).unwrap();
}

fn append_options_and_empty_body(l: u32) {
    wr_short(0xFFFF).unwrap();
    wr_long(l).unwrap();
}

#[test]
fn test_load_cpp_living_save() {
    let manifest = load_manifest().expect("manifest");
    let entry = manifest
        .goldens
        .iter()
        .find(|g| g.id == "save_newchar_seed42")
        .expect("save_newchar_seed42 golden");
    let golden = read_golden_bytes(entry);

    setup_load_harness();
    test_buffer_inject(&golden);
    let mut generate = true;
    assert!(load_game(&mut generate));
    assert!(!generate);
    assert_eq!(from_save_file(), 1);
    assert!(with_state(|state| state.dg.game_turn >= 0));
    assert!(with_state(|state| state.game.character_generated));
    assert_eq!(
        c_str(&with_state(|state| state.game.character_died_from)),
        "(alive and well)"
    );
    assert!(with_state(|state| state.dg.current_level >= 0));
    assert!(with_state(|state| state.game.magic_seed != 0));
}

#[test]
fn test_roundtrip_byte_stable() {
    setup_buffer_save();
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));
    let first = test_buffer_bytes();

    setup_load_harness();
    test_load_save_from_bytes(&first).expect("reload save");

    with_state_mut(|state| {
        state.game.character_saved = false;
        state.dg.game_turn = 1;
    });
    test_reset_buffer();
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));
    assert!(byte_diff(&first, &test_buffer_bytes()).is_none());
}

#[test]
fn test_missing_file() {
    setup_load_harness();
    let dir = std::env::temp_dir().join(format!("umoria_load513_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let missing = dir.join("missing.sav");
    with_state_mut(|state| state.config_save_game = missing.to_string_lossy().into_owned());

    let before_turn = with_state(|state| state.dg.game_turn);
    let mut generate = false;
    assert!(!load_game(&mut generate));
    assert!(generate);
    assert_eq!(with_state(|state| state.dg.game_turn), before_turn);
    let messages = ui_io::test_ui_messages();
    assert!(messages
        .iter()
        .any(|m| m.contains("Save file does not exist.")));
    assert!(!test_exit_program_called());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_alive_guard() {
    setup_load_harness();
    let dir = std::env::temp_dir().join(format!("umoria_load514_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("game.sav");
    fs::write(&path, [0u8; 8]).unwrap();
    with_state_mut(|state| {
        state.config_save_game = path.to_string_lossy().into_owned();
        state.dg.game_turn = 0;
    });

    let mut generate = false;
    assert!(!load_game(&mut generate));
    let messages = ui_io::test_ui_messages();
    assert!(messages
        .iter()
        .any(|m| m.contains("IMPOSSIBLE! Attempt to restore while still alive!")));
    assert!(messages
        .iter()
        .any(|m| m.contains("Please try again without that save file.")));
    assert!(test_exit_program_called());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_version_validation() {
    setup_load_harness();
    test_reset_buffer();
    write_header(0x2A, 1, 0, 0);
    append_options_and_empty_body(0);
    let bytes = test_buffer_bytes();
    test_buffer_inject(&bytes);

    let mut generate = true;
    assert!(!load_game(&mut generate));
    let messages = ui_io::test_ui_messages();
    assert!(messages
        .iter()
        .any(|m| m.contains("Sorry. This save file is from a different version of umoria.")));
    assert!(messages
        .iter()
        .any(|m| m.contains("Error during reading of file.")));
    assert!(test_exit_program_called());
}

#[test]
fn test_monster_memory_decode_and_bounds() {
    setup_buffer_save();
    with_state_mut(|state| {
        state.creature_recall[7] = Recall {
            movement: 42,
            spells: 99,
            kills: 3,
            deaths: 1,
            defenses: 5,
            wake: 2,
            ignore: 1,
            attacks: [1, 2, 3, 4],
        };
    });
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));

    setup_load_harness();
    test_load_save_from_bytes(&test_buffer_bytes()).expect("recall decode");
    let recall = with_state(|state| state.creature_recall[7]);
    assert_eq!(recall.movement, 42);
    assert_eq!(recall.spells, 99);
    assert_eq!(recall.kills, 3);

    setup_load_harness();
    test_reset_buffer();
    write_header(0x2A, 5, 7, 15);
    wr_short(MON_MAX_CREATURES).unwrap();
    test_buffer_inject(&test_buffer_bytes());
    let mut generate = true;
    assert!(!load_game(&mut generate));
    assert!(ui_io::test_ui_messages()
        .iter()
        .any(|m| m.contains("Error during reading of file.")));
}

#[test]
fn test_options_decode_mirrors_encode() {
    reset_for_new_game(None);
    with_state_mut(|state| {
        state.options.run_cut_corners = true;
        state.options.run_examine_corners = true;
        state.options.display_counts = true;
        state.options.error_beep_sound = false;
    });
    let encoded = test_build_options_l();

    setup_buffer_save();
    with_state_mut(|state| {
        state.options.run_cut_corners = true;
        state.options.run_examine_corners = true;
        state.options.display_counts = true;
        state.options.error_beep_sound = false;
    });
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));

    setup_load_harness();
    reset_for_new_game(None);
    test_load_save_from_bytes(&test_buffer_bytes()).expect("options load");
    assert_eq!(test_build_options_l(), encoded);
    assert!(with_state(|state| state.options.run_cut_corners));
    assert!(with_state(|state| state.options.run_examine_corners));
    assert!(with_state(|state| state.options.display_counts));
    assert!(!with_state(|state| state.options.error_beep_sound));
}

#[test]
fn test_retired_and_resurrect_gates() {
    setup_buffer_save();
    with_state_mut(|state| state.game.total_winner = true);
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));
    let retired = test_buffer_bytes();

    setup_load_harness();
    with_state_mut(|state| state.game.to_be_wizard = true);
    test_buffer_inject(&retired);
    let mut generate = true;
    assert!(load_game(&mut generate));
    let messages = ui_io::test_ui_messages();
    assert!(messages
        .iter()
        .any(|m| m.contains("Sorry, this character is retired from moria.")));
    assert!(messages
        .iter()
        .any(|m| m.contains("You can not resurrect a retired character.")));

    setup_buffer_save();
    with_state_mut(|state| state.game.character_is_dead = true);
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));
    let dead = test_buffer_bytes();

    setup_load_harness();
    with_state_mut(|state| state.game.to_be_wizard = true);
    test_push_getch_keys(&[b'y' as i32]);
    test_buffer_inject(&dead);
    let mut generate = true;
    assert!(load_game(&mut generate));
    assert!(ui_io::test_ui_messages()
        .iter()
        .any(|m| m.contains("Attempting a resurrection!")));
}

#[test]
fn test_dead_spirit_memory_only() {
    setup_buffer_save();
    with_state_mut(|state| state.game.character_is_dead = true);
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));

    setup_load_harness();
    test_buffer_inject(&test_buffer_bytes());
    let mut generate = true;
    assert!(!load_game(&mut generate));
    assert!(generate);
    assert_eq!(with_state(|state| state.dg.game_turn), -1);
    assert!(ui_io::test_ui_messages()
        .iter()
        .any(|m| m.contains("Restoring Memory of a departed spirit...")));
    assert_eq!(from_save_file(), 1);
}

#[test]
fn test_resurrection_setup() {
    setup_buffer_save();
    with_state_mut(|state| {
        state.py.misc.current_hp = -5;
        state.py.flags.food = -10;
        state.py.flags.poisoned = 3;
        state.dg.game_turn = 10;
    });
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));
    let full = test_buffer_bytes();
    let end = game_save::test_player_body_end_offset(&full).expect("body end");
    let truncated = full[..end].to_vec();

    setup_load_harness();
    with_state_mut(|state| state.game.to_be_wizard = true);
    test_buffer_inject(&truncated);
    let mut generate = true;
    assert!(load_game(&mut generate));
    with_state_mut(|state| {
        assert_eq!(state.py.misc.current_hp, 0);
        assert_eq!(state.py.misc.current_hp_fraction, 0);
        assert_eq!(state.py.flags.food, 0);
        assert_eq!(state.py.flags.poisoned, 1);
        assert_eq!(state.dg.current_level, 0);
        assert!(state.game.character_generated);
        assert!(!state.game.to_be_wizard);
        assert_eq!(state.game.noscore & 0x1, 0x1);
    });
}

#[test]
fn test_bounds_checks_goto_error() {
    setup_buffer_save();
    with_state_mut(|state| state.py.pack.unique_items = PlayerEquipment::Wield as i16 + 1);
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));
    setup_load_harness();
    test_buffer_inject(&test_buffer_bytes());
    let mut generate = true;
    assert!(!load_game(&mut generate));
    assert!(ui_io::test_ui_messages()
        .iter()
        .any(|m| m.contains("Error during reading of file.")));
}

#[test]
fn test_creature_treasure_coord_bounds() {
    setup_buffer_save();
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));
    let mut bytes = test_buffer_bytes();

    setup_load_harness();
    test_buffer_inject(&bytes);
    let mut generate = true;
    assert!(load_game(&mut generate));

    setup_load_harness();
    let pos = bytes.len().saturating_sub(32);
    bytes.splice(pos..pos, [MAX_HEIGHT + 1, 0, 1, 2]);
    test_buffer_inject(&bytes);
    let mut generate = true;
    assert!(!load_game(&mut generate));
}

#[test]
fn test_rle_cave_decode() {
    let manifest = load_manifest().expect("manifest");
    let entry = manifest
        .goldens
        .iter()
        .find(|g| g.id == "save_newchar_seed42")
        .expect("golden");
    let golden = read_golden_bytes(entry);

    setup_load_harness();
    test_load_save_from_bytes(&golden).expect("golden load");
    let tile = with_state(|state| state.dg.floor[1][1]);
    assert!(tile.feature_id != 0 || tile.permanent_light || tile.temporary_light);

    setup_load_harness();
    test_reset_buffer();
    write_header(0x2A, 5, 7, 15);
    append_options_and_empty_body(0);
    with_state_mut(|state| {
        state.dg.game_turn = 1;
        copy_cstr_into(&mut state.py.misc.name, "Tester");
    });
    // Force a truncated RLE by loading valid golden then corrupting is easier:
    let mut corrupt = golden.clone();
    let tail = corrupt.len().saturating_sub(4);
    corrupt[tail] = 0xFF;
    corrupt[tail + 1] = 0xFF;
    test_buffer_inject(&corrupt);
    let mut generate = true;
    assert!(!load_game(&mut generate));
}

fn copy_cstr_into(dest: &mut [u8], src: &str) {
    for (index, byte) in dest.iter_mut().enumerate() {
        *byte = if index < src.len() {
            src.as_bytes()[index]
        } else {
            0
        };
    }
}

#[test]
fn test_store_aging() {
    setup_buffer_save();
    test_set_unix_time(Some(1_000_000));
    set_start_time(900_000);
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));
    let bytes = test_buffer_bytes();

    setup_load_harness();
    test_set_unix_time(Some(1_000_000 + 5 * 86_400));
    test_buffer_inject(&bytes);
    let mut generate = true;
    assert!(load_game(&mut generate));
    assert_eq!(test_store_maintenance_count(), 5);

    setup_load_harness();
    test_set_unix_time(Some(500_000));
    test_buffer_inject(&bytes);
    let mut generate = true;
    assert!(load_game(&mut generate));
    assert_eq!(test_store_maintenance_count(), 0);
}

#[test]
fn test_scoreboard_and_noscore_messages() {
    setup_buffer_save();
    ui_io::test_set_panic_save(true);
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));

    setup_load_harness();
    test_buffer_inject(&test_buffer_bytes());
    let mut generate = true;
    assert!(load_game(&mut generate));
    assert!(ui_io::test_ui_messages().iter().any(|m| {
        m.contains("This game is from a panic save.  Score will not be added to scoreboard.")
    }));

    setup_buffer_save();
    with_state_mut(|state| state.game.noscore = 0);
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));

    setup_load_harness();
    test_buffer_inject(&test_buffer_bytes());
    let mut generate = true;
    assert!(load_game(&mut generate));
    let messages = ui_io::test_ui_messages();
    assert!(!messages.iter().any(|m| {
        m.contains("This character is already on the scoreboard; it will not be scored again.")
    }));

    setup_buffer_save();
    with_state_mut(|state| state.game.noscore = 1);
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));
    setup_load_harness();
    test_buffer_inject(&test_buffer_bytes());
    let mut generate = true;
    assert!(load_game(&mut generate));
    assert!(ui_io::test_ui_messages()
        .iter()
        .any(|m| { m.contains("This save file cannot be used to get on the score board.") }));
}

#[test]
fn test_version_mismatch_accepted_message() {
    let manifest = load_manifest().expect("manifest");
    let entry = manifest
        .goldens
        .iter()
        .find(|g| g.id == "save_newchar_seed42")
        .expect("golden");
    let golden = read_golden_bytes(entry);
    let mut bytes = golden.clone();
    bytes[0] = 5;
    bytes[1] = 2;
    bytes[2] = 2;

    setup_load_harness();
    test_buffer_inject(&bytes);
    let mut generate = true;
    assert!(load_game(&mut generate));
    assert!(ui_io::test_ui_messages()
        .iter()
        .any(|m| { m.contains("Save file version 5.2 accepted on game version 5.7.") }));
}

#[test]
fn test_alive_and_well_reset() {
    setup_buffer_save();
    with_state_mut(|state| {
        state.game.character_died_from[0] = b'x';
        state.game.character_died_from[1] = 0;
    });
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));

    setup_load_harness();
    set_from_save_file(0);
    test_buffer_inject(&test_buffer_bytes());
    let mut generate = true;
    assert!(load_game(&mut generate));
    assert_eq!(from_save_file(), 1);
    assert_eq!(
        c_str(&with_state(|state| state.game.character_died_from)),
        "(alive and well)"
    );
    assert!(with_state(|state| state.game.character_generated));
}
