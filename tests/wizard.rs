//! `wizard` parity.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::config::dungeon::objects::OBJ_WIZARD;
use umoria::config::identification::{ID_KNOWN2, ID_STORE_BOUGHT};
use umoria::config::monsters::{self};
use umoria::data_creatures::CREATURES_LIST;
use umoria::data_treasure::GAME_OBJECTS;
use umoria::dungeon::{coord_in_bounds, MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_generate::treasure_linker;
use umoria::dungeon_tile::{Tile, TILE_GRANITE_WALL, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::helpers::sscanf_lx;
use umoria::monster::{Monster, MON_MAX_LEVELS, MON_TOTAL_ALLOCATIONS};
use umoria::player::{PlayerAttr, PLAYER_MAX_LEVEL};
use umoria::player_stats::player_initialize_base_experience_levels;
use umoria::types::{Coord_t, MAX_DUNGEON_OBJECTS, MESSAGE_HISTORY_SIZE, TREASURE_MAX_LEVELS};
use umoria::ui::panel_bounds_fields;
use umoria::ui_io::{
    ctrl_key, test_clear_getch_keys, test_push_getch_keys, test_set_ncurses_stub, ESCAPE,
};
use umoria::wizard::{
    enter_wizard_mode, wizard_character_adjustment, wizard_create_objects, wizard_cure_all,
    wizard_drop_random_items, wizard_gain_experience, wizard_generate_object, wizard_jump_level,
    wizard_light_up_dungeon, wizard_request_object_id, wizard_summon_monster,
};

fn init_treasure_levels() {
    with_state_mut(|state| {
        state.treasure_levels = [0; TREASURE_MAX_LEVELS as usize + 1];
        for i in 0..MAX_DUNGEON_OBJECTS as usize {
            let level = GAME_OBJECTS[i].depth_first_found as usize;
            state.treasure_levels[level] += 1;
        }
        for i in 1..=TREASURE_MAX_LEVELS as usize {
            state.treasure_levels[i] += state.treasure_levels[i - 1];
        }

        let mut indexes = [1i16; TREASURE_MAX_LEVELS as usize + 1];
        for i in 0..MAX_DUNGEON_OBJECTS as usize {
            let level = GAME_OBJECTS[i].depth_first_found as usize;
            let object_id = state.treasure_levels[level] - indexes[level];
            state.sorted_objects[object_id as usize] = i as i16;
            indexes[level] += 1;
        }
    });
}

fn prepare_object_tables() {
    treasure_linker();
    init_treasure_levels();
}

fn init_monster_levels() {
    with_state_mut(|state| {
        state.monster_levels = [0; MON_MAX_LEVELS as usize + 1];
        let endgame = monsters::MON_ENDGAME_MONSTERS as usize;
        for i in 0..CREATURES_LIST.len() - endgame {
            let level = CREATURES_LIST[i].level as usize;
            state.monster_levels[level] += 1;
        }
        for i in 1..=MON_MAX_LEVELS as usize {
            state.monster_levels[i] += state.monster_levels[i - 1];
        }
    });
}

fn reset_monster_slots() {
    with_state_mut(|s| {
        s.next_free_monster_id = i16::from(monsters::MON_MIN_INDEX_ID);
        s.monsters = [Monster::default(); MON_TOTAL_ALLOCATIONS as usize];
        s.hack_monptr = -1;
    });
}

fn setup_dungeon(height: i16, width: i16) {
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                s.dg.floor[y as usize][x as usize].feature_id = TILE_LIGHT_FLOOR;
            }
        }
    });
}

fn setup_panel(pos: Coord_t) {
    test_set_ncurses_stub(true);
    let bounds = panel_bounds_fields(0, 0);
    with_state_mut(|s| {
        s.py.pos = pos;
        s.dg.panel.row = 0;
        s.dg.panel.col = 0;
        s.dg.panel.top = bounds.top;
        s.dg.panel.bottom = bounds.bottom;
        s.dg.panel.left = bounds.left;
        s.dg.panel.right = bounds.right;
        s.dg.panel.row_prt = bounds.row_prt;
        s.dg.panel.col_prt = bounds.col_prt;
    });
}

fn setup_player_misc() {
    player_initialize_base_experience_levels();
    with_state_mut(|s| {
        s.py.misc.level = 1;
        s.py.misc.experience_factor = 100;
        s.py.base_hp_levels[0] = 10;
        s.py.base_exp_levels = [999_999; PLAYER_MAX_LEVEL as usize];
    });
}

fn push_keys_in_consume_order(keys: &[i32]) {
    let mut reversed = keys.to_vec();
    reversed.reverse();
    test_push_getch_keys(&reversed);
}

fn push_string_input(s: &str) {
    let mut keys: Vec<i32> = s.bytes().map(i32::from).collect();
    keys.push(i32::from(ctrl_key(b'J')));
    push_keys_in_consume_order(&keys);
}

fn push_confirm_no() {
    push_keys_in_consume_order(&[i32::from(b'n')]);
}

fn push_escape() {
    push_keys_in_consume_order(&[i32::from(ESCAPE)]);
}

fn push_input_sequence_then_escape(values: &[&str]) {
    let mut keys = Vec::new();
    for value in values {
        keys.extend(value.bytes().map(i32::from));
        keys.push(i32::from(ctrl_key(b'J')));
    }
    keys.push(i32::from(ESCAPE));
    push_keys_in_consume_order(&keys);
}

fn last_message() -> String {
    with_state(|s| {
        let idx = s.last_message_id.rem_euclid(MESSAGE_HISTORY_SIZE as i16) as usize;
        let msg = &s.messages[idx];
        let end = msg.iter().position(|&b| b == 0).unwrap_or(msg.len());
        String::from_utf8_lossy(&msg[..end]).into_owned()
    })
}

fn tile_at(coord: Coord_t) -> Tile {
    with_state(|s| s.dg.floor[coord.y as usize][coord.x as usize])
}

// ---------------------------------------------------------------------------
// 1. enterWizardMode
// ---------------------------------------------------------------------------
#[test]
fn enter_wizard_mode_denied_when_noscore_zero_and_not_confirmed() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    push_confirm_no();

    assert!(!enter_wizard_mode());
    with_state(|s| {
        assert_eq!(s.game.noscore, 0);
        assert!(!s.game.wizard_mode);
    });

    test_set_ncurses_stub(false);
    test_clear_getch_keys();
}

#[test]
fn enter_wizard_mode_succeeds_on_confirm() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    // print_message may require a -more- ack when a prior line is pending.
    with_state_mut(|s| s.message_ready_to_print = true);
    push_keys_in_consume_order(&[i32::from(b' '), i32::from(b'y')]);

    assert!(enter_wizard_mode());
    with_state(|s| {
        assert_eq!(s.game.noscore, 2);
        assert!(s.game.wizard_mode);
    });

    test_set_ncurses_stub(false);
    test_clear_getch_keys();
}

#[test]
fn enter_wizard_mode_auto_when_noscore_nonzero() {
    reset_for_new_game(None);
    with_state_mut(|s| s.game.noscore = 1);
    test_set_ncurses_stub(true);
    test_clear_getch_keys();

    assert!(enter_wizard_mode());
    with_state(|s| {
        assert_eq!(s.game.noscore, 3);
        assert!(s.game.wizard_mode);
    });

    test_set_ncurses_stub(false);
}

// ---------------------------------------------------------------------------
// 2. wizardCureAll
// ---------------------------------------------------------------------------
#[test]
fn wizard_cure_all_clamps_slow_image_and_cures_conditions() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.flags.blind = 5;
        s.py.flags.confused = 4;
        s.py.flags.poisoned = 3;
        s.py.flags.afraid = 2;
        s.py.flags.slow = 3;
        s.py.flags.image = 4;
        s.py.stats.used[PlayerAttr::A_STR as usize] = 10;
        s.py.stats.max[PlayerAttr::A_STR as usize] = 18;
    });

    wizard_cure_all();

    with_state(|s| {
        assert_eq!(s.py.flags.blind, 1);
        assert_eq!(s.py.flags.confused, 1);
        assert_eq!(s.py.flags.poisoned, 1);
        assert_eq!(s.py.flags.afraid, 1);
        assert_eq!(s.py.flags.slow, 1);
        assert_eq!(s.py.flags.image, 1);
        assert_eq!(s.py.stats.used[PlayerAttr::A_STR as usize], 18);
    });

    test_set_ncurses_stub(false);
    test_clear_getch_keys();
}

// ---------------------------------------------------------------------------
// 3. wizardDropRandomItems
// ---------------------------------------------------------------------------
#[test]
fn wizard_drop_random_items_uses_command_count_then_resets() {
    reset_for_new_game(Some(1));
    prepare_object_tables();
    setup_dungeon(20, 20);
    setup_panel(Coord_t { y: 10, x: 10 });
    with_state_mut(|s| {
        s.game.command_count = 3;
        s.game.treasure.current_id = 1;
    });

    wizard_drop_random_items();

    with_state(|s| assert_eq!(s.game.command_count, 0));
    let placed = with_state(|s| s.dg.floor.iter().flatten().any(|t| t.treasure_id != 0));
    assert!(placed);
}

#[test]
fn wizard_drop_random_items_defaults_to_one_when_no_command_count() {
    reset_for_new_game(Some(1));
    prepare_object_tables();
    setup_dungeon(20, 20);
    setup_panel(Coord_t { y: 10, x: 10 });
    with_state_mut(|s| {
        s.game.command_count = 0;
        s.game.treasure.current_id = 1;
    });

    wizard_drop_random_items();

    // `tries` defaults to 1 when command_count is 0, but C++
    // `dungeonPlaceRandomObjectNear` sets `i = 9` on each successful placement
    // inside `for (i = 0; i <= 10; i++)`, so the post-increment re-enters at
    // `i == 10` and can chain further placements until an attempt fails.
    // Seed 1 on this open floor places 8 objects for that single try.
    let count = with_state(|s| {
        s.dg.floor
            .iter()
            .flatten()
            .filter(|t| t.treasure_id != 0)
            .count()
    });
    assert_eq!(count, 8);
}

// ---------------------------------------------------------------------------
// 4. wizardJumpLevel
// ---------------------------------------------------------------------------
#[test]
fn wizard_jump_level_from_command_count_clamps_and_resets() {
    reset_for_new_game(None);
    with_state_mut(|s| s.game.command_count = 150);

    wizard_jump_level();

    with_state(|s| {
        assert_eq!(s.dg.current_level, 0);
        assert!(s.dg.generate_new_level);
        assert_eq!(s.game.command_count, 0);
    });
}

#[test]
fn wizard_jump_level_from_command_count_value() {
    reset_for_new_game(None);
    with_state_mut(|s| s.game.command_count = 12);

    wizard_jump_level();

    with_state(|s| {
        assert_eq!(s.dg.current_level, 12);
        assert!(s.dg.generate_new_level);
    });
}

#[test]
fn wizard_jump_level_prompt_clamps_above_99() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    push_string_input("150");

    wizard_jump_level();

    with_state(|s| {
        assert_eq!(s.dg.current_level, 99);
        assert!(s.dg.generate_new_level);
    });

    test_set_ncurses_stub(false);
    test_clear_getch_keys();
}

#[test]
fn wizard_jump_level_prompt_cancel_clears_message_line() {
    reset_for_new_game(None);
    with_state_mut(|s| s.dg.generate_new_level = false);
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    push_escape();

    wizard_jump_level();

    with_state(|s| {
        assert!(!s.dg.generate_new_level);
        assert_eq!(s.dg.current_level, 0);
    });

    test_set_ncurses_stub(false);
    test_clear_getch_keys();
}

// ---------------------------------------------------------------------------
// 5. wizardGainExperience
// ---------------------------------------------------------------------------
#[test]
fn wizard_gain_experience_from_command_count() {
    reset_for_new_game(None);
    setup_player_misc();
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.game.command_count = 500;
        s.py.misc.exp = 0;
    });

    wizard_gain_experience();

    with_state(|s| {
        assert_eq!(s.py.misc.exp, 500);
        assert_eq!(s.game.command_count, 0);
    });

    test_set_ncurses_stub(false);
}

#[test]
fn wizard_gain_experience_zero_becomes_one() {
    reset_for_new_game(None);
    setup_player_misc();
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.game.command_count = 0;
        s.py.misc.exp = 0;
    });

    wizard_gain_experience();

    with_state(|s| assert_eq!(s.py.misc.exp, 1));

    test_set_ncurses_stub(false);
}

#[test]
fn wizard_gain_experience_doubles_with_wrap() {
    reset_for_new_game(None);
    setup_player_misc();
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.game.command_count = 0;
        s.py.misc.exp = 1_400_000_000;
    });

    wizard_gain_experience();

    with_state(|s| assert_eq!(s.py.misc.exp, -1_494_967_296));

    test_set_ncurses_stub(false);
}

// ---------------------------------------------------------------------------
// 6. wizardSummonMonster
// ---------------------------------------------------------------------------
#[test]
fn wizard_summon_monster_rng_order_seed42() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    reset_monster_slots();
    setup_dungeon(30, 30);
    setup_panel(Coord_t { y: 15, x: 15 });
    with_state_mut(|s| s.dg.current_level = 5);

    reset_for_new_game(Some(42));
    init_monster_levels();
    reset_monster_slots();
    setup_dungeon(30, 30);
    setup_panel(Coord_t { y: 15, x: 15 });
    with_state_mut(|s| s.dg.current_level = 5);

    wizard_summon_monster();

    with_state(|s| assert!(s.next_free_monster_id > i16::from(monsters::MON_MIN_INDEX_ID)));
}

// ---------------------------------------------------------------------------
// 7. wizardLightUpDungeon
// ---------------------------------------------------------------------------
#[test]
fn wizard_light_up_dungeon_sets_three_by_three_neighborhood() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    setup_panel(Coord_t { y: 5, x: 5 });
    with_state_mut(|s| {
        for y in 3..7 {
            for x in 3..7 {
                s.dg.floor[y][x].feature_id = TILE_LIGHT_FLOOR;
            }
        }
        s.dg.floor[5][5].permanent_light = false;
    });

    wizard_light_up_dungeon();

    for y in 4..=6 {
        for x in 4..=6 {
            assert!(tile_at(Coord_t { y, x }).permanent_light);
        }
    }
    assert!(!tile_at(Coord_t { y: 5, x: 5 }).field_mark);
}

#[test]
fn wizard_light_up_dungeon_toggles_off_and_clears_field_mark() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    setup_panel(Coord_t { y: 5, x: 5 });
    with_state_mut(|s| {
        s.dg.floor[5][5].feature_id = TILE_LIGHT_FLOOR;
        s.dg.floor[5][5].permanent_light = true;
        s.dg.floor[5][5].field_mark = true;
    });

    wizard_light_up_dungeon();

    assert!(!tile_at(Coord_t { y: 5, x: 5 }).permanent_light);
    assert!(!tile_at(Coord_t { y: 5, x: 5 }).field_mark);
}

#[test]
fn wizard_light_up_dungeon_skips_non_floor_tiles() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    setup_panel(Coord_t { y: 5, x: 5 });
    with_state_mut(|s| {
        for y in 0..s.dg.height {
            for x in 0..s.dg.width {
                s.dg.floor[y as usize][x as usize].feature_id = TILE_GRANITE_WALL;
            }
        }
    });

    wizard_light_up_dungeon();

    assert!(!tile_at(Coord_t { y: 4, x: 4 }).permanent_light);
}

#[test]
fn wizard_light_up_dungeon_edge_floor_lights_in_bounds_neighbors() {
    // C++ writes yy/xx without bounds checks; Rust lights all in-array neighbors.
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    setup_panel(Coord_t { y: 0, x: 0 });
    with_state_mut(|s| {
        s.dg.floor[0][0].feature_id = TILE_LIGHT_FLOOR;
        s.dg.floor[0][0].permanent_light = false;
        s.py.pos = Coord_t { y: 0, x: 0 };
    });

    wizard_light_up_dungeon();

    assert!(tile_at(Coord_t { y: 0, x: 0 }).permanent_light);
    assert!(tile_at(Coord_t { y: 0, x: 1 }).permanent_light);
    assert!(tile_at(Coord_t { y: 1, x: 0 }).permanent_light);
    assert!(tile_at(Coord_t { y: 1, x: 1 }).permanent_light);
}

// ---------------------------------------------------------------------------
// 8. wizardCharacterAdjustment
// ---------------------------------------------------------------------------
#[test]
fn wizard_character_adjustment_searching_uses_prompt_length_bug() {
    reset_for_new_game(None);
    setup_player_misc();
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    let search_prompt_len = with_state(|s| {
        format!(
            "Current={}  (0-200) Searching = ",
            s.py.misc.chance_in_search
        )
        .len()
    });
    push_input_sequence_then_escape(&[
        "18", "18", "18", "18", "18", "18", "100", "50", "999", "77",
    ]);

    wizard_character_adjustment();
    with_state(|s| {
        assert_eq!(s.py.stats.max[PlayerAttr::A_STR as usize], 18);
        assert_eq!(s.py.misc.max_hp, 100);
        assert_eq!(s.py.misc.au, 999);
        assert_eq!(s.py.misc.chance_in_search, search_prompt_len as i16);
    });

    test_set_ncurses_stub(false);
    test_clear_getch_keys();
}

#[test]
fn wizard_character_adjustment_early_return_on_cancel() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    push_escape();

    wizard_character_adjustment();

    with_state(|s| assert_eq!(s.py.stats.max[PlayerAttr::A_STR as usize], 0));

    test_set_ncurses_stub(false);
    test_clear_getch_keys();
}

// ---------------------------------------------------------------------------
// 9. wizardRequestObjectId
// ---------------------------------------------------------------------------
#[test]
fn wizard_request_object_id_valid() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    push_string_input("42");

    let mut id = 0;
    assert!(wizard_request_object_id(
        &mut id,
        "Dungeon/Store object",
        0,
        366
    ));
    assert_eq!(id, 42);

    test_set_ncurses_stub(false);
    test_clear_getch_keys();
}

#[test]
fn wizard_request_object_id_out_of_range() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    push_string_input("999");

    let mut id = 0;
    assert!(!wizard_request_object_id(
        &mut id,
        "Dungeon/Store object",
        0,
        366
    ));
    assert_eq!(id, 0);

    test_set_ncurses_stub(false);
    test_clear_getch_keys();
}

#[test]
fn wizard_request_object_id_cancelled() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    push_escape();

    let mut id = 7;
    assert!(!wizard_request_object_id(&mut id, "Label", 0, 10));
    assert_eq!(id, 7);

    test_set_ncurses_stub(false);
    test_clear_getch_keys();
}

// ---------------------------------------------------------------------------
// 10. wizardGenerateObject — RNG order
// ---------------------------------------------------------------------------
#[test]
fn wizard_generate_object_rng_order_all_fail_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_panel(Coord_t { y: 10, x: 10 });
    with_state_mut(|s| {
        for y in 0..20 {
            for x in 0..20 {
                s.dg.floor[y][x].feature_id = TILE_GRANITE_WALL;
            }
        }
        s.game.treasure.current_id = 1;
    });

    test_set_ncurses_stub(true);
    test_clear_getch_keys();

    let baseline = {
        for _ in 0..10 {
            random_number(5);
            random_number(7);
        }
        random_number(100)
    };

    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_panel(Coord_t { y: 10, x: 10 });
    with_state_mut(|s| {
        for y in 0..20 {
            for x in 0..20 {
                s.dg.floor[y][x].feature_id = TILE_GRANITE_WALL;
            }
        }
        s.game.treasure.current_id = 1;
    });
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    push_string_input("5");

    wizard_generate_object();

    assert_eq!(random_number(100), baseline);
    assert_eq!(
        with_state(|s| {
            s.dg.floor
                .iter()
                .flatten()
                .filter(|t| t.treasure_id != 0)
                .count()
        }),
        0
    );

    test_set_ncurses_stub(false);
    test_clear_getch_keys();
}

#[test]
fn wizard_generate_object_places_item_on_success_seed1() {
    reset_for_new_game(Some(1));
    prepare_object_tables();
    setup_dungeon(20, 20);
    setup_panel(Coord_t { y: 10, x: 10 });
    with_state_mut(|s| {
        s.game.treasure.current_id = 1;
        s.dg.current_level = 5;
    });

    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    push_string_input("1");

    wizard_generate_object();

    let (coord, item_id) = with_state(|s| {
        for y in 0..s.dg.height {
            for x in 0..s.dg.width {
                let tid = s.dg.floor[y as usize][x as usize].treasure_id;
                if tid != 0 {
                    return (
                        Coord_t {
                            y: i32::from(y),
                            x: i32::from(x),
                        },
                        s.game.treasure.list[tid as usize].id,
                    );
                }
            }
        }
        panic!("no object placed");
    });
    assert!(coord_in_bounds(coord));
    assert_eq!(item_id, 1);
    with_state(|s| {
        for y in 0..s.dg.height {
            for x in 0..s.dg.width {
                let tid = s.dg.floor[y as usize][x as usize].treasure_id;
                if tid != 0 {
                    assert_eq!(
                        s.game.treasure.list[tid as usize].category_id,
                        umoria::data_treasure::GAME_OBJECTS[1].category_id
                    );
                    return;
                }
            }
        }
    });

    test_set_ncurses_stub(false);
    test_clear_getch_keys();
}

// ---------------------------------------------------------------------------
// 11. wizardCreateObjects
// ---------------------------------------------------------------------------
fn push_create_object_prompts(values: &[&str], confirm_yes: bool) {
    let mut keys = Vec::new();
    keys.push(i32::from(b' '));
    for value in values {
        keys.extend(value.bytes().map(i32::from));
        keys.push(i32::from(ctrl_key(b'J')));
    }
    keys.push(i32::from(if confirm_yes { b'y' } else { b'n' }));
    push_keys_in_consume_order(&keys);
}

#[test]
fn wizard_create_objects_allocate_places_forge() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    setup_panel(Coord_t { y: 5, x: 5 });
    with_state_mut(|s| {
        s.game.treasure.current_id = 1;
        s.message_ready_to_print = false;
    });

    test_clear_getch_keys();
    push_create_object_prompts(
        &[
            "1", "!", "2", "10", "3", "2", "6", "1", "2", "3", "4", "5", "ff", "100", "10",
        ],
        true,
    );

    wizard_create_objects();

    with_state(|s| {
        let tid = s.dg.floor[5][5].treasure_id;
        assert_ne!(tid, 0);
        let item = s.game.treasure.list[tid as usize];
        assert_eq!(item.id, OBJ_WIZARD);
        assert_eq!(item.category_id, 1);
        assert_eq!(item.sprite, b'!');
        assert_eq!(item.flags, 0xff);
        assert_eq!(item.identification, ID_KNOWN2 | ID_STORE_BOUGHT);
    });
    assert_eq!(last_message(), "Allocated.");

    test_set_ncurses_stub(false);
    test_clear_getch_keys();
}

#[test]
fn wizard_create_objects_aborted() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    setup_panel(Coord_t { y: 5, x: 5 });
    with_state_mut(|s| s.message_ready_to_print = false);
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    with_state_mut(|s| s.message_ready_to_print = false);
    push_create_object_prompts(
        &[
            "1", "!", "2", "10", "3", "2", "6", "1", "2", "3", "4", "5", "ff", "100", "10",
        ],
        false,
    );

    wizard_create_objects();

    assert_eq!(last_message(), "Aborted.");
    assert_eq!(with_state(|s| s.dg.floor[5][5].treasure_id), 0);

    test_set_ncurses_stub(false);
    test_clear_getch_keys();
}

#[test]
fn wizard_create_objects_invalid_hex_leaves_flags_zero() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    setup_panel(Coord_t { y: 5, x: 5 });
    with_state_mut(|s| {
        s.game.treasure.current_id = 1;
        s.message_ready_to_print = false;
    });
    test_clear_getch_keys();
    push_create_object_prompts(
        &[
            "1", "!", "2", "10", "3", "2", "6", "1", "2", "3", "4", "5", "xyzzy", "100", "10",
        ],
        true,
    );

    wizard_create_objects();

    with_state(|s| {
        let tid = s.dg.floor[5][5].treasure_id;
        assert_eq!(s.game.treasure.list[tid as usize].flags, 0);
    });

    test_set_ncurses_stub(false);
    test_clear_getch_keys();
}

#[test]
fn sscanf_lx_matches_wizard_hex_semantics() {
    let mut value = 99;
    assert_eq!(sscanf_lx("not-hex", &mut value), 0);
    assert_eq!(value, 99);
    assert_eq!(sscanf_lx("deadbeef", &mut value), 1);
    assert_eq!(value as u32, 0xdeadbeef_u32);
}
