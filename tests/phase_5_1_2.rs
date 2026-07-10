//! Phase 5.1.2 — saveGame / saveChar / svWrite (strict TDD).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use std::fs;
use std::path::PathBuf;

use common::{byte_diff, load_manifest, read_golden_bytes, GoldenEntry, VolatileByteRange};
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, TILE_DARK_FLOOR, TILE_LIGHT_FLOOR};
use umoria::game::{reset_for_new_game, with_state, with_state_mut};
use umoria::game_save::{
    self, get_byte, rd_byte, save_char, save_game, set_from_save_file, set_start_time,
    set_xor_byte, sv_write, test_buffer_bytes, test_buffer_inject, test_buffer_len,
    test_build_options_l, test_compute_save_timestamp, test_load_save_from_bytes,
    test_reset_buffer, test_set_force_save_char_fail, test_set_forced_seed_byte,
    test_set_save_fail_flush, test_set_unix_time, xor_byte,
};
use umoria::inventory::PlayerEquipment;
use umoria::recall::Recall;
use umoria::ui_io::{self, test_push_getch_keys, ESCAPE};

type OptionsCase = (fn(&mut umoria::game::State), u32);

fn golden_save(name: &str) -> PathBuf {
    common::golden_root().join("save").join(name)
}

fn decode_xor_chain(data: &[u8], resets: &[usize]) -> Vec<u8> {
    let reset_set: std::collections::HashSet<usize> = resets.iter().copied().collect();
    let mut out = Vec::with_capacity(data.len());
    let mut prev = 0u8;
    for (index, &byte) in data.iter().enumerate() {
        if reset_set.contains(&index) {
            prev = 0;
        }
        out.push(byte ^ prev);
        prev = byte;
    }
    out
}

fn apply_mask(data: &[u8], ranges: &[VolatileByteRange]) -> Vec<u8> {
    let mut masked = data.to_vec();
    for range in ranges {
        for index in range.offset..range.offset.saturating_add(range.length).min(masked.len()) {
            masked[index] = 0;
        }
    }
    masked
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

fn write_header_only(seed: u8) {
    test_set_forced_seed_byte(Some(seed));
    set_xor_byte(0);
    game_save::wr_byte(5).unwrap();
    set_xor_byte(0);
    game_save::wr_byte(7).unwrap();
    set_xor_byte(0);
    game_save::wr_byte(15).unwrap();
    set_xor_byte(0);
    game_save::wr_byte(seed).unwrap();
}

#[test]
fn test_header_bytes_and_seed() {
    setup_buffer_save();
    write_header_only(0x2A);
    assert_eq!(xor_byte(), 0x2A);

    let raw = test_buffer_bytes();
    assert_eq!(raw.len(), 4);

    test_buffer_inject(&raw);
    set_xor_byte(0);
    assert_eq!(rd_byte().unwrap(), 5);
    set_xor_byte(0);
    assert_eq!(rd_byte().unwrap(), 7);
    set_xor_byte(0);
    assert_eq!(rd_byte().unwrap(), 15);
    set_xor_byte(0);
    assert_eq!(get_byte().unwrap(), 0x2A);
}

#[test]
fn test_options_bitfield_masks() {
    reset_for_new_game(None);

    let cases: [OptionsCase; 11] = [
        (|s| s.options.run_cut_corners = true, 0x1),
        (|s| s.options.run_examine_corners = true, 0x2),
        (|s| s.options.run_print_self = true, 0x4),
        (|s| s.options.find_bound = true, 0x8),
        (|s| s.options.prompt_to_pickup = true, 0x10),
        (|s| s.options.use_roguelike_keys = true, 0x20),
        (|s| s.options.show_inventory_weights = true, 0x40),
        (|s| s.options.highlight_seams = true, 0x80),
        (|s| s.options.run_ignore_doors = true, 0x100),
        (|s| s.options.error_beep_sound = true, 0x200),
        (|s| s.options.display_counts = true, 0x400),
    ];

    for (set_flag, mask) in cases {
        reset_for_new_game(None);
        with_state_mut(|state| {
            clear_options(&mut state.options);
            set_flag(state);
        });
        let l = test_build_options_l();
        assert_eq!(l & mask, mask, "mask {mask:#x}");
        assert_eq!(l & !mask, 0, "unexpected bits for mask {mask:#x}");
    }

    reset_for_new_game(None);
    with_state_mut(|state| {
        clear_options(&mut state.options);
        state.game.character_is_dead = true;
    });
    assert_eq!(test_build_options_l(), 0x8000_0000);

    reset_for_new_game(None);
    with_state_mut(|state| {
        clear_options(&mut state.options);
        state.game.total_winner = true;
    });
    assert_eq!(test_build_options_l(), 0x4000_0000);
}

fn clear_options(options: &mut umoria::game::Options) {
    *options = umoria::game::Options {
        display_counts: false,
        error_beep_sound: false,
        run_cut_corners: false,
        run_examine_corners: false,
        run_print_self: false,
        find_bound: false,
        prompt_to_pickup: false,
        use_roguelike_keys: false,
        show_inventory_weights: false,
        highlight_seams: false,
        run_ignore_doors: false,
    };
}

#[test]
fn test_monster_memory_sentinel_and_skip() {
    setup_buffer_save();
    with_state_mut(|state| {
        state.creature_recall[3] = Recall {
            movement: 1,
            kills: 2,
            ..Recall::default()
        };
    });
    test_set_forced_seed_byte(Some(0));
    assert!(save_char("game.sav"));
    let bytes = test_buffer_bytes();

    reset_for_new_game(None);
    test_load_save_from_bytes(&bytes).expect("reload save");
    with_state(|state| {
        assert_eq!(state.creature_recall[3].movement, 1);
        assert_eq!(state.creature_recall[3].kills, 2);
        assert_eq!(state.creature_recall[4].movement, 0);
    });

    setup_buffer_save();
    test_set_forced_seed_byte(Some(0));
    assert!(save_char("game.sav"));
    reset_for_new_game(None);
    test_load_save_from_bytes(&test_buffer_bytes()).expect("empty recall reload");
    with_state(|state| {
        assert!(state
            .creature_recall
            .iter()
            .all(|r| r.movement == 0 && r.kills == 0));
    });
}

#[test]
fn test_player_block_order_and_casts() {
    setup_buffer_save();
    with_state_mut(|state| {
        state.py.misc.chance_in_search = -1;
        state.py.misc.fos = -2;
        state.py.misc.bth = -3;
        state.py.stats.modified = [-1, -2, -3, -4, -5, -6];
        state.py.flags.rest = -7;
    });
    write_header_only(0x5A);
    assert!(sv_write());
    let rust_bytes = test_buffer_bytes();

    let golden = fs::read(golden_save("player_block_seed5a.bin")).unwrap_or_else(|_| {
        fs::write(
            common::repo_root().join("tests/golden/game_save/player_block_seed5a.bin"),
            &rust_bytes,
        )
        .ok();
        rust_bytes.clone()
    });
    assert!(
        byte_diff(&golden, &rust_bytes).is_none(),
        "player block bytes must match golden"
    );
}

#[test]
fn test_inventory_and_equipment_ranges() {
    setup_buffer_save();
    with_state_mut(|state| {
        state.py.pack.unique_items = 2;
        state.py.inventory[0].id = 0x1111;
        state.py.inventory[1].id = 0x2222;
        state.py.inventory[PlayerEquipment::Wield as usize].id = 0x3333;
        state.py.inventory[PlayerEquipment::Light as usize].id = 0x4444;
    });
    test_set_forced_seed_byte(Some(0));
    assert!(save_char("game.sav"));
    let bytes = test_buffer_bytes();

    reset_for_new_game(None);
    test_load_save_from_bytes(&bytes).expect("reload save");
    with_state(|state| {
        assert_eq!(state.py.pack.unique_items, 2);
        assert_eq!(state.py.inventory[0].id, 0x1111);
        assert_eq!(state.py.inventory[1].id, 0x2222);
        assert_eq!(
            state.py.inventory[PlayerEquipment::Wield as usize].id,
            0x3333
        );
        assert_eq!(
            state.py.inventory[PlayerEquipment::Light as usize].id,
            0x4444
        );
    });
}

#[test]
fn test_messages_and_stores() {
    setup_buffer_save();
    with_state_mut(|state| {
        state.messages[0][..5].copy_from_slice(b"hello");
        state.stores[0].unique_items_counter = 1;
        state.stores[0].inventory[0].cost = 1234;
        state.stores[0].inventory[0].item.id = 0xBEEF;
    });
    write_header_only(0);
    assert!(sv_write());
    let bytes = test_buffer_bytes();
    setup_buffer_save();
    with_state_mut(|state| {
        state.messages[0][..5].copy_from_slice(b"hello");
        state.stores[0].unique_items_counter = 1;
        state.stores[0].inventory[0].cost = 1234;
        state.stores[0].inventory[0].item.id = 0xBEEF;
    });
    write_header_only(0);
    assert!(sv_write());
    assert!(byte_diff(&bytes, &test_buffer_bytes()).is_none());
}

#[test]
fn test_timestamp_clamp() {
    set_start_time(1_000_000);
    test_set_unix_time(Some(999_999));
    assert_eq!(test_compute_save_timestamp(), 1_000_000 + 86_400);

    set_start_time(100);
    test_set_unix_time(Some(200));
    assert_eq!(test_compute_save_timestamp(), 200);
}

#[test]
fn test_dead_character_early_return() {
    setup_buffer_save();
    with_state_mut(|state| {
        state.game.character_is_dead = true;
        state.dg.current_level = 5;
    });
    write_header_only(0);
    assert!(sv_write());
    let dead_len = test_buffer_len();

    setup_buffer_save();
    with_state_mut(|state| state.dg.current_level = 5);
    write_header_only(0);
    assert!(sv_write());
    assert!(test_buffer_len() > dead_len);
}

#[test]
fn test_hangup_clears_dead() {
    setup_buffer_save();
    ui_io::test_set_eof_flag(1);
    with_state_mut(|state| {
        state.game.character_is_dead = true;
        state.dg.current_level = 3;
        state.dg.height = 10;
        state.dg.width = 10;
    });
    write_header_only(0);
    assert!(sv_write());
    assert!(test_buffer_len() > 100);
    ui_io::test_set_eof_flag(0);
}

#[test]
fn test_creature_and_treasure_sparse_lists() {
    setup_buffer_save();
    with_state_mut(|state| {
        state.dg.floor[1][2].creature_id = 7;
        state.dg.floor[3][4].treasure_id = 9;
        state.dg.height = 5;
        state.dg.width = 5;
        state.dg.panel.max_rows = 1;
        state.dg.panel.max_cols = 1;
        state.game.treasure.current_id = 1;
        state.next_free_monster_id = 2;
    });
    test_set_forced_seed_byte(Some(0));
    assert!(save_char("game.sav"));
    let bytes = test_buffer_bytes();

    reset_for_new_game(None);
    test_load_save_from_bytes(&bytes).expect("reload save");
    with_state(|state| {
        assert_eq!(state.dg.floor[1][2].creature_id, 7);
        assert_eq!(state.dg.floor[3][4].treasure_id, 9);
    });
}

#[test]
fn test_rle_cave_encoding() {
    setup_buffer_save();
    with_state_mut(|state| {
        for row in &mut state.dg.floor {
            for tile in row {
                *tile = Tile {
                    feature_id: TILE_DARK_FLOOR,
                    ..Tile::default()
                };
            }
        }
        state.dg.floor[0][0].feature_id = TILE_LIGHT_FLOOR;
        state.dg.floor[0][0].permanent_light = true;
        state.game.treasure.current_id = 1;
        state.next_free_monster_id = 2;
        state.dg.height = 2;
        state.dg.width = 2;
        state.dg.panel.max_rows = 1;
        state.dg.panel.max_cols = 1;
    });
    test_set_forced_seed_byte(Some(0));
    assert!(save_char("game.sav"));
    let bytes = test_buffer_bytes();

    reset_for_new_game(None);
    test_load_save_from_bytes(&bytes).expect("reload save");
    with_state(|state| {
        assert_eq!(state.dg.floor[0][0].feature_id, TILE_LIGHT_FLOOR);
        assert!(state.dg.floor[0][0].permanent_light);
        assert_eq!(state.dg.floor[0][1].feature_id, TILE_DARK_FLOOR);
    });
}

#[test]
fn test_savechar_open_excl_and_flags() {
    ui_io::test_set_ncurses_stub(true);
    reset_for_new_game(None);
    let dir = std::env::temp_dir().join(format!("umoria_save512_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("game.sav");
    let _ = fs::remove_file(&path);

    with_state_mut(|state| {
        state.config_save_game = path.to_string_lossy().into_owned();
        state.game.character_saved = false;
        state.dg.game_turn = 1;
    });
    assert!(save_char(path.to_str().unwrap()));
    let meta = fs::metadata(&path).unwrap();
    assert_eq!(meta.permissions().mode() & 0o777, 0o600);

    set_from_save_file(1);
    assert!(save_char(path.to_str().unwrap()));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_savechar_sets_saved_and_turn() {
    setup_buffer_save();
    assert!(save_char("game.sav"));
    assert!(with_state(|state| state.game.character_saved));
    assert_eq!(with_state(|state| state.dg.game_turn), -1);

    assert!(save_char("game.sav"));
}

#[test]
fn test_savechar_failure_unlink_and_message() {
    ui_io::test_set_ncurses_stub(true);
    ui_io::test_set_ui_capture(true);
    reset_for_new_game(None);
    let dir = std::env::temp_dir().join(format!("umoria_save514_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("game.sav");

    with_state_mut(|state| {
        state.config_save_game = path.to_string_lossy().into_owned();
        state.game.character_saved = false;
        state.dg.game_turn = 1;
    });
    test_set_save_fail_flush(true);
    assert!(!save_char(path.to_str().unwrap()));
    test_set_save_fail_flush(false);
    assert!(!path.exists());
    let messages = ui_io::test_ui_messages();
    assert!(messages.iter().any(|m| m.contains("Error writing to file")));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_savegame_retry_rename_flow() {
    ui_io::test_set_ncurses_stub(true);
    ui_io::test_set_ui_capture(true);
    test_push_getch_keys(&[b'y' as i32, b'n' as i32, ESCAPE as i32]);
    reset_for_new_game(None);
    let dir = std::env::temp_dir().join(format!("umoria_save515_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let old = dir.join("old.sav");
    fs::write(&old, b"stub").unwrap();

    with_state_mut(|state| {
        state.config_save_game = old.to_string_lossy().into_owned();
        state.game.character_saved = false;
        state.dg.game_turn = 1;
    });
    test_set_force_save_char_fail(true);
    assert!(!save_game());
    test_set_force_save_char_fail(false);

    let messages = ui_io::test_ui_messages();
    assert!(messages
        .iter()
        .any(|m| m.contains("Save file '") && m.contains(" fails.")));
    assert!(messages
        .iter()
        .any(|m| m.contains("New Save file [ESC to give up]:")));
    let _ = fs::remove_dir_all(dir);
}

fn masked_save_diff(entry: &GoldenEntry, expected: &[u8], actual: &[u8]) -> Option<common::Diff> {
    let decoded_expected = decode_xor_chain(expected, &[0, 1, 2, 3]);
    let decoded_actual = decode_xor_chain(actual, &[0, 1, 2, 3]);
    let masked_expected = apply_mask(&decoded_expected, &entry.volatile_byte_ranges);
    let masked_actual = apply_mask(&decoded_actual, &entry.volatile_byte_ranges);
    byte_diff(&masked_expected, &masked_actual)
}

#[test]
fn test_full_save_byte_identical_to_cpp() {
    let manifest = load_manifest().expect("manifest");
    let entry = manifest
        .goldens
        .iter()
        .find(|g| g.id == "save_newchar_seed42")
        .expect("save_newchar_seed42 golden");

    let golden = read_golden_bytes(entry);
    let seed = golden[3];

    ui_io::test_set_ncurses_stub(true);
    reset_for_new_game(None);
    test_load_save_from_bytes(&golden).expect("load golden save");
    test_set_forced_seed_byte(Some(seed));
    test_set_unix_time(None);
    with_state_mut(|state| {
        state.game.character_saved = false;
    });
    test_reset_buffer();
    assert!(save_char("game.sav"));

    let actual = test_buffer_bytes();
    assert_eq!(actual.len(), golden.len(), "save length must match C++");
    if let Some(diff) = masked_save_diff(entry, &golden, &actual) {
        panic!("masked save bytes must match C++ golden: {}", diff.render());
    }
}

#[test]
fn test_dead_save_byte_identical_to_cpp() {
    setup_buffer_save();
    with_state_mut(|state| {
        state.game.character_is_dead = true;
        state.py.misc.date_of_birth = 42;
        test_set_unix_time(Some(100));
        set_start_time(50);
    });
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));
    let first = test_buffer_bytes();

    setup_buffer_save();
    with_state_mut(|state| {
        state.game.character_is_dead = true;
        state.py.misc.date_of_birth = 42;
        test_set_unix_time(Some(100));
        set_start_time(50);
    });
    test_set_forced_seed_byte(Some(0x2A));
    assert!(save_char("game.sav"));
    assert!(byte_diff(&first, &test_buffer_bytes()).is_none());
    assert!(test_buffer_len() < MAX_HEIGHT as usize * MAX_WIDTH as usize);
}
