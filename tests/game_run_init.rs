//! `startMoria` & game initialization.
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

use common::{load_manifest, read_golden_bytes};
use umoria::config::identification::{ID_SHOW_HIT_DAM, ID_STORE_BOUGHT};
use umoria::config::monsters::MON_ENDGAME_MONSTERS;
use umoria::data_creatures::CREATURES_LIST;
use umoria::data_player::CLASS_BASE_PROVISIONS;
use umoria::data_treasure::GAME_OBJECTS;
use umoria::game::{
    reset_for_new_game, seeds_initialize, set_test_unix_time, test_reset_exit_program_called,
    test_set_skip_process_exit, with_state, with_state_mut,
};
use umoria::game_run::{
    initialize_character_inventory, initialize_monster_levels, initialize_treasure_levels,
    player_initialize_player_light, price_adjust, price_adjust_cost, reset_dungeon_flags,
    start_moria, test_boot_events, test_play_dungeon_call_count, test_reset_boot_hooks,
    test_set_boot_stop_after, test_set_load_game_hook, test_set_play_dungeon_script,
    test_set_skip_change_character_name, test_set_skip_character_create, test_set_skip_end_game,
    test_set_skip_generate_cave, BootEvent, PlayDungeonScript,
};
use umoria::game_save::{test_set_force_save_char_fail, test_set_unix_time};
use umoria::inventory::{PlayerEquipment, PLAYER_INVENTORY_SIZE};
use umoria::scores::test_reset_score_test_hooks;
use umoria::scores::test_set_scores_path;
use umoria::store::COST_ADJUSTMENT;
use umoria::treasure::TV_SWORD;
use umoria::types::{MAX_DUNGEON_OBJECTS, MON_MAX_CREATURES, MON_MAX_LEVELS, TREASURE_MAX_LEVELS};
use umoria::ui_io::{
    test_clear_getch_keys, test_push_getch_keys, test_set_ncurses_stub, test_set_ui_capture,
    test_set_ui_trace, test_ui_trace_events, UiTraceEvent,
};

fn setup_harness() {
    test_set_ncurses_stub(true);
    test_set_ui_capture(true);
    test_set_ui_trace(true);
    test_clear_getch_keys();
    umoria::ui_io::test_set_eof_flag(0);
    test_reset_exit_program_called();
    test_reset_boot_hooks();
    test_reset_score_test_hooks();
    test_set_skip_process_exit(true);
    test_set_skip_end_game(true);
    test_set_skip_generate_cave(true);
    reset_for_new_game(None);
}

fn push_keys_in_consume_order(keys: &[i32]) {
    let mut reversed = keys.to_vec();
    reversed.reverse();
    test_push_getch_keys(&reversed);
}

fn c_str(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn reference_monster_levels() -> [i16; MON_MAX_LEVELS as usize + 1] {
    let mut levels = [0i16; MON_MAX_LEVELS as usize + 1];
    for i in 0..(MON_MAX_CREATURES - u16::from(MON_ENDGAME_MONSTERS)) as usize {
        let level = CREATURES_LIST[i].level as usize;
        levels[level] += 1;
    }
    for i in 1..=MON_MAX_LEVELS as usize {
        levels[i] += levels[i - 1];
    }
    levels
}

fn reference_treasure_tables() -> (
    [i16; TREASURE_MAX_LEVELS as usize + 1],
    [i16; MAX_DUNGEON_OBJECTS as usize],
) {
    let mut treasure_levels = [0i16; TREASURE_MAX_LEVELS as usize + 1];
    for i in 0..MAX_DUNGEON_OBJECTS as usize {
        let level = GAME_OBJECTS[i].depth_first_found as usize;
        treasure_levels[level] += 1;
    }
    for i in 1..=TREASURE_MAX_LEVELS as usize {
        treasure_levels[i] += treasure_levels[i - 1];
    }

    let mut sorted_objects = [0i16; MAX_DUNGEON_OBJECTS as usize];
    let mut indexes = [1i16; TREASURE_MAX_LEVELS as usize + 1];
    for i in 0..MAX_DUNGEON_OBJECTS as usize {
        let level = GAME_OBJECTS[i].depth_first_found as usize;
        let object_id = treasure_levels[level] - indexes[level];
        sorted_objects[object_id as usize] = i as i16;
        indexes[level] += 1;
    }
    (treasure_levels, sorted_objects)
}

fn tempfile_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("umoria_561_{label}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

#[test]
fn price_adjust_noop_when_cost_adjustment_is_100() {
    assert_eq!(COST_ADJUSTMENT, 100);
    let before: Vec<i32> = GAME_OBJECTS.iter().map(|o| o.cost).collect();
    price_adjust();
    let after: Vec<i32> = GAME_OBJECTS.iter().map(|o| o.cost).collect();
    assert_eq!(before, after);
}

#[test]
fn price_adjust_formula_round_half_up_and_boundaries() {
    assert_eq!(price_adjust_cost(0, 150), 0);
    assert_eq!(price_adjust_cost(1, 150), 2);
    assert_eq!(price_adjust_cost(33, 150), 50);
    assert_eq!(price_adjust_cost(34, 150), 51);
    assert_eq!(price_adjust_cost(100, 100), 100);
    assert_eq!(price_adjust_cost(999, 150), 1499);
    assert_eq!(price_adjust_cost(65535, 200), ((65535 * 200) + 50) / 100);

    for object in GAME_OBJECTS.iter() {
        let adjusted = price_adjust_cost(object.cost, 150);
        assert_eq!(adjusted, ((object.cost * 150) + 50) / 100);
    }
}

#[test]
fn initialize_monster_levels_matches_cpp_reference() {
    setup_harness();
    initialize_monster_levels();
    let expected = reference_monster_levels();
    with_state(|state| assert_eq!(state.monster_levels, expected));
}

#[test]
fn initialize_treasure_levels_matches_cpp_reference() {
    setup_harness();
    initialize_treasure_levels();
    let (expected_levels, expected_sorted) = reference_treasure_tables();
    with_state(|state| {
        assert_eq!(state.treasure_levels, expected_levels);
        assert_eq!(state.sorted_objects, expected_sorted);
    });
}

#[test]
fn initialize_character_inventory_provisions_and_spell_order() {
    setup_harness();
    with_state_mut(|state| {
        state.py.misc.class_id = 0;
        for slot in &mut state.py.inventory {
            slot.category_id = umoria::treasure::TV_MISC;
        }
    });

    initialize_character_inventory();

    with_state(|state| {
        for id in &state.py.flags.spells_learned_order {
            assert_eq!(*id, 99);
        }

        for &item_id in &CLASS_BASE_PROVISIONS[0] {
            let object = &GAME_OBJECTS[item_id as usize];
            let mut found = false;
            for (slot, item) in state
                .py
                .inventory
                .iter()
                .enumerate()
                .take(usize::from(PLAYER_INVENTORY_SIZE))
            {
                if slot >= PlayerEquipment::Wield as usize {
                    break;
                }
                if item.category_id == umoria::treasure::TV_NOTHING {
                    continue;
                }
                if item.id == item_id {
                    found = true;
                    assert_ne!(item.identification & ID_STORE_BOUGHT, 0);
                    if object.category_id == TV_SWORD {
                        assert_ne!(item.identification & ID_SHOW_HIT_DAM, 0);
                    }
                }
            }
            assert!(found, "missing provision item {item_id}");
        }
    });
}

#[test]
fn reset_dungeon_flags_and_player_light() {
    setup_harness();
    with_state_mut(|state| {
        state.game.command_count = 9;
        state.dg.generate_new_level = true;
        state.py.running_tracker = 4;
        state.game.teleport_player = true;
        state.monster_multiply_total = 7;
        state.py.pos.y = 10;
        state.py.pos.x = 11;
        state.dg.floor[10][11].creature_id = 0;
        state.py.inventory[PlayerEquipment::Light as usize].misc_use = 5;
        state.py.carrying_light = false;
    });

    reset_dungeon_flags();
    player_initialize_player_light();

    with_state(|state| {
        assert_eq!(state.game.command_count, 0);
        assert!(!state.dg.generate_new_level);
        assert_eq!(state.py.running_tracker, 0);
        assert!(!state.game.teleport_player);
        assert_eq!(state.monster_multiply_total, 0);
        assert_eq!(state.dg.floor[10][11].creature_id, 1);
        assert!(state.py.carrying_light);
    });
}

#[test]
fn start_moria_new_game_call_order() {
    setup_harness();
    test_set_skip_character_create(true);
    test_set_unix_time(Some(1_700_000_000));
    set_test_unix_time(Some(1_700_000_000));
    with_state_mut(|state| {
        state.game.noscore = 1;
        state.py.misc.class_id = 1;
    });

    start_moria(42, true, true);

    assert_eq!(
        test_boot_events(),
        vec![
            BootEvent::SetRoguelikeKeys,
            BootEvent::PriceAdjust,
            BootEvent::DisplaySplashScreen,
            BootEvent::SeedsInitialize,
            BootEvent::InitializeMonsterLevels,
            BootEvent::InitializeTreasureLevels,
            BootEvent::StoreInitializeOwners,
            BootEvent::PlayerInitializeBaseExperienceLevels,
            BootEvent::ZeroSpellCounters,
            BootEvent::CharacterCreate,
            BootEvent::SetDateOfBirth,
            BootEvent::InitializeCharacterInventory,
            BootEvent::SetFoodDefaults,
            BootEvent::MageManaBranch,
            BootEvent::SetDefaultPlayerFields,
            BootEvent::SetCharacterGenerated,
            BootEvent::MagicInitializeItemNames,
            BootEvent::BeginGameDisplay,
            BootEvent::GenerateCave,
            BootEvent::PlayDungeon,
            BootEvent::EndGame,
        ]
    );

    with_state(|state| {
        assert!(state.options.use_roguelike_keys);
        assert_eq!(state.py.flags.food, 7500);
        assert_eq!(state.py.flags.food_digested, 2);
        assert!(state.game.character_generated);
        assert_eq!(state.py.misc.date_of_birth, 1_700_000_000);
    });
}

#[test]
fn start_moria_new_game_rng_after_seeds_initialize() {
    setup_harness();
    seeds_initialize(42);
    let expected =
        with_state(|state| (state.game.magic_seed, state.game.town_seed, state.rng.seed));

    reset_for_new_game(None);
    test_reset_boot_hooks();
    test_set_boot_stop_after(Some(BootEvent::SeedsInitialize));

    start_moria(42, true, false);

    with_state(|state| {
        assert_eq!(state.game.magic_seed, expected.0);
        assert_eq!(state.game.town_seed, expected.1);
        assert_eq!(state.rng.seed, expected.2);
    });
}

#[test]
fn start_moria_new_game_begin_display_puts_help_string() {
    setup_harness();
    test_set_skip_character_create(true);
    start_moria(42, true, false);

    let help = test_ui_trace_events()
        .into_iter()
        .find_map(|event| match event {
            UiTraceEvent::PutString { text, y, x }
                if text == "Press ? for help" && y == 0 && x == 63 =>
            {
                Some(())
            }
            _ => None,
        });
    assert!(help.is_some());
}

#[test]
fn start_moria_load_branch_wizard_gate_and_generate_skip() {
    setup_harness();
    test_set_load_game_hook(true, false);
    with_state_mut(|state| {
        state.game.to_be_wizard = true;
        state.game.noscore = 0;
    });
    push_keys_in_consume_order(&[i32::from(b'n')]);

    start_moria(42, false, false);

    assert_eq!(
        test_boot_events(),
        vec![
            BootEvent::SetRoguelikeKeys,
            BootEvent::PriceAdjust,
            BootEvent::DisplaySplashScreen,
            BootEvent::SeedsInitialize,
            BootEvent::InitializeMonsterLevels,
            BootEvent::InitializeTreasureLevels,
            BootEvent::StoreInitializeOwners,
            BootEvent::PlayerInitializeBaseExperienceLevels,
            BootEvent::ZeroSpellCounters,
            BootEvent::LoadGame,
            BootEvent::EnterWizardMode,
            BootEvent::EndGame,
        ]
    );
    assert!(!test_boot_events().contains(&BootEvent::GenerateCave));
}

#[test]
fn start_moria_load_branch_restores_and_respects_generate() {
    setup_harness();
    test_set_load_game_hook(true, false);
    test_set_skip_change_character_name(true);
    with_state_mut(|state| state.py.misc.current_hp = -1);

    start_moria(42, false, false);

    assert!(test_boot_events().contains(&BootEvent::ChangeCharacterName));
    assert!(!test_boot_events().contains(&BootEvent::GenerateCave));
    with_state(|state| assert!(state.game.character_is_dead));
}

#[test]
fn start_moria_load_branch_generates_when_resurrection() {
    setup_harness();
    test_set_load_game_hook(true, true);
    test_set_skip_change_character_name(true);

    start_moria(42, false, false);

    assert!(test_boot_events().contains(&BootEvent::ChangeCharacterName));
    assert!(test_boot_events().contains(&BootEvent::GenerateCave));
}

#[test]
fn start_moria_eof_save_sets_died_from_and_exits() {
    setup_harness();
    test_set_skip_character_create(true);
    test_set_play_dungeon_script(PlayDungeonScript::SetEof);
    test_set_skip_generate_cave(false);

    let dir = tempfile_dir("eof");
    let save_path = dir.join("game.sav");
    with_state_mut(|state| {
        state.config_save_game = save_path.to_string_lossy().into_owned();
        state.game.noscore = 1;
    });
    test_set_scores_path(Some(dir.join("scores.dat").as_path()));

    start_moria(42, true, false);

    assert_eq!(test_play_dungeon_call_count(), 1);
    with_state(|state| {
        assert!(state.game.character_is_dead);
        assert_eq!(
            c_str(&state.game.character_died_from),
            "(end of input: saved)"
        );
    });
    assert!(save_path.is_file());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn start_moria_eof_save_failure_sets_unexpected_eof() {
    setup_harness();
    test_set_skip_character_create(true);
    test_set_play_dungeon_script(PlayDungeonScript::SetEof);
    test_set_force_save_char_fail(true);
    with_state_mut(|state| state.game.noscore = 1);

    start_moria(42, true, false);

    with_state(|state| {
        assert_eq!(c_str(&state.game.character_died_from), "unexpected eof");
    });
}

#[test]
fn start_moria_loop_generates_next_level_when_alive() {
    setup_harness();
    test_set_skip_character_create(true);
    test_set_play_dungeon_script(PlayDungeonScript::ContinueThenDead(2));
    test_set_skip_generate_cave(false);
    with_state_mut(|state| state.game.noscore = 1);

    start_moria(42, true, false);

    let generate_count = test_boot_events()
        .iter()
        .filter(|e| **e == BootEvent::GenerateCave)
        .count();
    assert_eq!(generate_count, 2);
    assert_eq!(test_play_dungeon_call_count(), 2);
}

#[test]
fn boot_playthrough_new_game_state_through_first_play_dungeon() {
    setup_harness();
    test_set_unix_time(Some(1_700_000_000));
    set_test_unix_time(Some(1_700_000_000));
    test_set_skip_character_create(true);
    test_set_play_dungeon_script(PlayDungeonScript::ContinueThenDead(1));

    let dir = tempfile_dir("boot");
    test_set_scores_path(Some(dir.join("scores.dat").as_path()));
    with_state_mut(|state| {
        state.game.noscore = 1;
        state.py.misc.class_id = 0;
    });

    start_moria(42, true, false);

    assert_eq!(test_play_dungeon_call_count(), 1);
    with_state(|state| {
        assert!(state.game.character_generated);
        assert_eq!(state.py.flags.food, 7500);
        assert_eq!(state.py.misc.date_of_birth, 1_700_000_000);
        assert_eq!(state.monster_levels, reference_monster_levels());
    });
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn boot_playthrough_loaded_game_from_golden_save() {
    setup_harness();
    test_set_skip_change_character_name(true);

    let manifest = load_manifest().expect("manifest");
    let entry = manifest
        .goldens
        .iter()
        .find(|g| g.id == "save_newchar_seed42")
        .expect("save_newchar_seed42 golden");
    let save_bytes = read_golden_bytes(entry);

    let dir = tempfile_dir("loadboot");
    let save_path = dir.join("game.sav");
    fs::write(&save_path, &save_bytes).expect("write save");
    test_set_scores_path(Some(dir.join("scores.dat").as_path()));

    with_state_mut(|state| {
        state.config_save_game = save_path.to_string_lossy().into_owned();
        state.game.noscore = 1;
        state.dg.game_turn = -1;
    });

    start_moria(42, false, false);

    assert!(test_boot_events().contains(&BootEvent::LoadGame));
    assert!(test_boot_events().contains(&BootEvent::ChangeCharacterName));
    with_state(|state| {
        assert!(state.game.character_generated);
    });
    let _ = fs::remove_dir_all(dir);
}
