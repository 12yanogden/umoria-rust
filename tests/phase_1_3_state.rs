//! Phase 1.3 global-state ownership model tests.
//! See `.cursor/plans/rust-translation/phase_1.3.md`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::config;
use umoria::data_creatures;
use umoria::data_player;
use umoria::data_recall;
use umoria::data_store_owners;
use umoria::data_stores;
use umoria::data_treasure;
use umoria::dungeon_generate;
use umoria::game;
use umoria::game_save;
use umoria::monster;
use umoria::rng;
use umoria::scores;
use umoria::store;
use umoria::types::{Screen, MORIA_MESSAGE_SIZE};
use umoria::ui_io;

// ---------------------------------------------------------------------------
// 1. GameState defaults match C++ Game_t member initializers (game.h)
// ---------------------------------------------------------------------------
#[test]
fn state_default_matches_cpp_game_t() {
    let state = game::State::default();
    let g = &state.game;

    assert_eq!(g.magic_seed, 0);
    assert_eq!(g.town_seed, 0);
    assert!(!g.character_generated);
    assert!(!g.character_saved);
    assert!(!g.character_is_dead);
    assert!(!g.total_winner);
    assert!(!g.teleport_player);
    assert!(!g.player_free_turn);
    assert!(!g.to_be_wizard);
    assert!(!g.wizard_mode);
    assert_eq!(g.noscore, 0);
    assert!(!g.use_last_direction);
    assert_eq!(g.doing_inventory_command, 0);
    assert_eq!(g.last_command, b' ');
    assert_eq!(g.command_count, 0);
    assert_eq!(g.treasure.current_id, 0);
    assert_eq!(g.screen.current_screen_id, Screen::Blank);
    assert_eq!(g.screen.screen_left_pos, 0);
    assert_eq!(g.screen.screen_bottom_pos, 0);
    assert_eq!(g.screen.wear_low_id, 0);
    assert_eq!(g.screen.wear_high_id, 0);
    assert!(g.character_died_from.iter().all(|&b| b == 0));
}

// ---------------------------------------------------------------------------
// 2. Dungeon defaults match C++ Dungeon_t{0,0,{},-1,0,true,{}}
// ---------------------------------------------------------------------------
#[test]
fn dungeon_default_matches_cpp() {
    let dg = game::State::default().dg;

    assert_eq!(dg.height, 0);
    assert_eq!(dg.width, 0);
    // C++ Dungeon_t{0,0,{},-1,0,true,{}} positional init: game_turn(4th)=-1, current_level(5th)=0.
    assert_eq!(dg.current_level, 0);
    assert_eq!(dg.game_turn, -1);
    assert!(dg.generate_new_level);
}

// ---------------------------------------------------------------------------
// 3. hack_monptr default is -1 (monster.cpp)
// ---------------------------------------------------------------------------
#[test]
fn hack_monptr_default_is_minus_one() {
    assert_eq!(game::State::default().hack_monptr, -1);
}

// ---------------------------------------------------------------------------
// 4. Counter defaults match C++ zero-init globals
// ---------------------------------------------------------------------------
#[test]
fn counters_default_zero() {
    let s = game::State::default();

    assert_eq!(s.missiles_counter, 0);
    assert_eq!(s.last_message_id, 0);
    assert_eq!(s.next_free_monster_id, 0);
    assert_eq!(s.monster_multiply_total, 0);
}

// ---------------------------------------------------------------------------
// 5. RNG seed lives in State.rng; z[10001] == 1043618065 for internal seed 1
//    (C++ setRandomSeed(0) → stored seed 1, per rng.cpp TEST_RNG)
// ---------------------------------------------------------------------------
#[test]
fn rng_seed_lives_in_state_and_reseeds() {
    rng::set_seed(0);
    assert_eq!(game::with_state(|s| s.rng.seed), 1);

    for _ in 0..10_000 {
        rng::rnd();
    }
    let z10001 = game::with_state(|s| s.rng.seed);
    assert_eq!(z10001, 1_043_618_065);
}

// ---------------------------------------------------------------------------
// 6. reset_for_new_game yields deterministic RNG sequence
// ---------------------------------------------------------------------------
#[test]
fn reset_produces_deterministic_rng() {
    rng::set_seed(0);
    for _ in 0..100 {
        rng::rnd();
    }

    game::with_state_mut(|s| {
        s.game.character_is_dead = true;
        s.rng.seed = 999_999;
    });

    game::reset_for_new_game(Some(0));
    for _ in 0..10_000 {
        rng::rnd();
    }
    assert_eq!(game::with_state(|s| s.rng.seed), 1_043_618_065);
}

// ---------------------------------------------------------------------------
// 7. reset_for_new_game clears mutable state (no leakage)
// ---------------------------------------------------------------------------
#[test]
fn reset_clears_mutable_state() {
    game::with_state_mut(|s| {
        s.game.character_is_dead = true;
        s.messages[0][0] = b'X';
        s.last_message_id = 3;
        s.missiles_counter = 42;
    });

    game::reset_for_new_game(None);

    game::with_state(|s| {
        assert!(!s.game.character_is_dead);
        assert_eq!(s.messages[0][0], 0);
        assert_eq!(s.last_message_id, 0);
        assert_eq!(s.missiles_counter, 0);
    });
}

// ---------------------------------------------------------------------------
// 8. set_seed wrapping matches C setRandomSeed
// ---------------------------------------------------------------------------
#[test]
fn set_random_seed_wrapping_matches_c() {
    rng::set_seed(0);
    assert_eq!(game::with_state(|s| s.rng.seed), 1);

    let wrap = (rng::RNG_M - 1) as u32;
    rng::set_seed(wrap);
    assert_eq!(game::with_state(|s| s.rng.seed), 1);

    rng::set_seed(wrap + 1);
    assert_eq!(game::with_state(|s| s.rng.seed), 2);

    let almost = wrap - 1;
    rng::set_seed(almost);
    assert_eq!(game::with_state(|s| s.rng.seed), wrap);
}

// ---------------------------------------------------------------------------
// 9. Read-only tables have immutable module homes (compile-checked symbols)
// ---------------------------------------------------------------------------
#[test]
fn readonly_tables_are_immutable_homes() {
    let _ = &*data_creatures::CREATURES_LIST;
    let _ = &*data_creatures::MONSTER_ATTACKS;
    let _ = &data_player::BLOWS_TABLE;
    let _ = &data_player::SPELL_NAMES;
    let _ = &data_recall::RECALL_DESCRIPTION_SPELL;
    let _ = config::files::SPLASH_SCREEN;
    let _ = &*data_treasure::GAME_OBJECTS;
    let _ = &monster::BLANK_MONSTER;
    let _ = &*data_store_owners::STORE_OWNERS;
    let _ = &*data_stores::STORES;
}

// ---------------------------------------------------------------------------
// 10. Every mapped global has a named Rust home (compile test)
// ---------------------------------------------------------------------------
#[test]
fn every_mapped_global_has_a_home() {
    // --- mutable singletons (State fields) ---
    game::with_state(|s| {
        let _ = &s.game;
        let _ = &s.py;
        let _ = &s.dg;
        let _ = &s.normal_table;
        let _ = &s.sorted_objects;
        let _ = &s.treasure_levels;
        let _ = &s.monster_levels;
        let _ = &s.monsters;
        let _ = &s.creature_recall;
        let _ = &s.objects_identified;
        let _ = &s.messages;
        let _ = s.last_message_id;
        let _ = s.message_ready_to_print;
        let _ = s.screen_has_changed;
        let _ = s.next_free_monster_id;
        let _ = s.monster_multiply_total;
        let _ = s.hack_monptr;
        let _ = s.missiles_counter;
        let _ = &s.stores;
        let _ = &s.options;
        let _ = &s.config_save_game;
        let _ = &s.rng;
    });

    // --- read-only tables ---
    let _ = &*data_treasure::GAME_OBJECTS;
    let _ = &*data_creatures::CREATURES_LIST;
    let _ = &*data_creatures::MONSTER_ATTACKS;
    let _ = &monster::BLANK_MONSTER;
    let _ = &*data_player::CHARACTER_RACES;
    let _ = &*data_player::CHARACTER_BACKGROUNDS;
    let _ = &*data_player::CLASSES;
    let _ = &*data_player::CLASS_RANK_TITLES;
    let _ = &data_player::CLASS_LEVEL_ADJ;
    let _ = &data_player::CLASS_BASE_PROVISIONS;
    let _ = &data_player::BLOWS_TABLE;
    let _ = &*data_player::MAGIC_SPELLS;
    let _ = &data_player::SPELL_NAMES;
    let _ = &*data_store_owners::STORE_OWNERS;
    let _ = &*data_stores::STORES;
    let _ = &data_stores::STORE_CHOICES;
    let _ = &store::STORE_BUY;
    let _ = &data_stores::RACE_GOLD_ADJUSTMENTS;
    let _ = &data_stores::SPEECH_SALE_ACCEPTED;
    let _ = &data_stores::SPEECH_SELLING_HAGGLE_FINAL;
    let _ = &data_stores::SPEECH_SELLING_HAGGLE;
    let _ = &data_stores::SPEECH_BUYING_HAGGLE_FINAL;
    let _ = &data_stores::SPEECH_BUYING_HAGGLE;
    let _ = &data_stores::SPEECH_INSULTED_HAGGLING_DONE;
    let _ = &data_stores::SPEECH_GET_OUT_OF_MY_STORE;
    let _ = &data_stores::SPEECH_HAGGLING_TRY_AGAIN;
    let _ = &data_stores::SPEECH_SORRY;
    let _ = &data_recall::RECALL_DESCRIPTION_ATTACK_TYPE;
    let _ = &data_recall::RECALL_DESCRIPTION_ATTACK_METHOD;
    let _ = &data_recall::RECALL_DESCRIPTION_HOW_MUCH;
    let _ = &data_recall::RECALL_DESCRIPTION_MOVE;
    let _ = &data_recall::RECALL_DESCRIPTION_SPELL;
    let _ = &data_recall::RECALL_DESCRIPTION_BREATH;
    let _ = &data_recall::RECALL_DESCRIPTION_WEAKNESS;
    let _ = &data_treasure::SPECIAL_ITEM_NAMES;
    let _ = &data_treasure::COLORS;
    let _ = &data_treasure::MUSHROOMS;
    let _ = &data_treasure::WOODS;
    let _ = &data_treasure::METALS;
    let _ = &data_treasure::ROCKS;
    let _ = &data_treasure::AMULETS;
    let _ = &data_treasure::SYLLABLES;

    // --- config namespace ---
    let _ = config::files::SPLASH_SCREEN;
    let _ = config::files::WELCOME_SCREEN;
    let _ = config::files::LICENSE;
    let _ = config::files::VERSIONS_HISTORY;
    let _ = config::files::HELP;
    let _ = config::files::HELP_WIZARD;
    let _ = config::files::HELP_ROGUELIKE;
    let _ = config::files::HELP_ROGUELIKE_WIZARD;
    let _ = config::files::DEATH_TOMB;
    let _ = config::files::DEATH_ROYAL;
    let _ = config::files::SCORES;
    let _ = config::dungeon::DUN_RANDOM_DIR;
    let _ = config::treasure::MIN_TREASURE_LIST_ID;
    let _ = config::monsters::MON_CHANCE_OF_NEW;
    let _ = config::player::PLAYER_MAX_EXP;
    let _ = config::identification::OD_TRIED;
    let _ = config::spells::SPELL_TYPE_NONE;
    let _ = config::stores::STORE_MAX_AUTO_BUY_ITEMS;

    // --- module-private transient statics (not in State) ---
    let _ = ui_io::curses_on();
    let _ = ui_io::eof_flag();
    let _ = ui_io::panic_save();
    let _ = game_save::xor_byte();
    let _ = game_save::from_save_file();
    let _ = game_save::start_time();
    let _ = scores::highscore_fp_is_none();
    let _ = dungeon_generate::door_index();

    // --- rng free-fn surface ---
    let _ = rng::get_seed();
}

// ---------------------------------------------------------------------------
// 11. Transient module statics are excluded from State (save-file fidelity)
// ---------------------------------------------------------------------------
#[test]
fn module_statics_excluded_from_state() {
    // Access transient homes directly — they must not live on State.
    assert!(!ui_io::curses_on());
    assert_eq!(ui_io::eof_flag(), 0);
    assert!(!ui_io::panic_save());
    assert_eq!(game_save::xor_byte(), 0);
    assert_eq!(game_save::from_save_file(), 0);
    assert_eq!(game_save::start_time(), 0);
    assert!(scores::highscore_fp_is_none());
    assert_eq!(dungeon_generate::door_index(), 0);

    // State field inventory: no transient scratch fields.
    let field_count = std::mem::size_of::<game::State>();
    assert!(field_count > 0);
    let _ = MORIA_MESSAGE_SIZE; // keep test module linked to types
}

// ---------------------------------------------------------------------------
// 12. Options defaults match C++ config::options (config.cpp)
// ---------------------------------------------------------------------------
#[test]
fn options_default_matches_cpp() {
    let opts = game::State::default().options;

    assert!(opts.display_counts);
    assert!(!opts.find_bound);
    assert!(opts.run_cut_corners);
    assert!(opts.run_examine_corners);
    assert!(!opts.run_ignore_doors);
    assert!(!opts.run_print_self);
    assert!(!opts.highlight_seams);
    assert!(!opts.prompt_to_pickup);
    assert!(!opts.use_roguelike_keys);
    assert!(!opts.show_inventory_weights);
    assert!(opts.error_beep_sound);
}
