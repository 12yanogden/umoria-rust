//! `player_move` parity.
#![allow(
    clippy::int_plus_one,
    reason = "test assertions mirror C++ inclusive bound comparisons"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::config::dungeon::objects::{OBJ_GOLD_LIST, OBJ_NOTHING, OBJ_TRAP_LIST};
use umoria::config::monsters::MON_ENDGAME_MONSTERS;
use umoria::config::player::status::PY_SEARCH;
use umoria::data_creatures::CREATURES_LIST;
use umoria::data_treasure::GAME_OBJECTS;
use umoria::dungeon::{dungeon_set_trap, MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, TILE_GRANITE_WALL, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::game_objects::popt;
use umoria::inventory::{inventory_item_copy_to, PLAYER_INVENTORY_SIZE};
use umoria::monster::{Monster, MON_MAX_CREATURES, MON_MAX_LEVELS, MON_TOTAL_ALLOCATIONS};
use umoria::player::PlayerAttr;
use umoria::player_move::player_move;
use umoria::treasure::TV_FOOD;
use umoria::types::{Coord_t, MESSAGE_HISTORY_SIZE};
use umoria::ui::panel_bounds_fields;
use umoria::ui_io::test_set_ncurses_stub;

const POS: Coord_t = Coord_t { y: 10, x: 10 };
const NORTH: i32 = 8;
const GREY_MUSHROOM_ID: u16 = 8;
const RATION_OBJ: u16 = 21;

fn init_monster_levels() {
    with_state_mut(|s| {
        s.monster_levels = [0; MON_MAX_LEVELS as usize + 1];
        let endgame = MON_ENDGAME_MONSTERS as usize;
        for i in 0..MON_MAX_CREATURES as usize - endgame {
            let level = CREATURES_LIST[i].level as usize;
            s.monster_levels[level] += 1;
        }
        for i in 1..=MON_MAX_LEVELS as usize {
            s.monster_levels[i] += s.monster_levels[i - 1];
        }
    });
}

fn open_floor_north_of_player() {
    set_tile(
        Coord_t { y: 9, x: 10 },
        Tile {
            feature_id: TILE_LIGHT_FLOOR,
            ..Tile::default()
        },
    );
}

/// Seed 77 with fos=10 skips `player_search` (randomNumber(10) != 1).
fn disable_auto_search() {
    with_state_mut(|s| {
        s.py.misc.fos = 10;
        s.py.misc.chance_in_search = 50;
    });
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

fn message_text(id: i16) -> String {
    with_state(|s| {
        let idx = id.rem_euclid(MESSAGE_HISTORY_SIZE as i16) as usize;
        let msg = &s.messages[idx];
        let end = msg.iter().position(|&b| b == 0).unwrap_or(msg.len());
        String::from_utf8_lossy(&msg[..end]).into_owned()
    })
}

fn setup_dungeon() {
    with_state_mut(|s| {
        s.dg.height = 20;
        s.dg.width = 20;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
        for y in 1..19 {
            for x in 1..19 {
                s.dg.floor[y][x].feature_id = TILE_LIGHT_FLOOR;
            }
        }
        s.game.treasure.current_id = 1;
        s.dg.current_level = 10;
    });
}

fn setup_player() {
    test_set_ncurses_stub(true);
    let bounds = panel_bounds_fields(0, 0);
    with_state_mut(|s| {
        s.py.pos = POS;
        s.py.misc.current_hp = 500;
        s.py.misc.max_hp = 500;
        s.py.misc.level = 10;
        s.py.misc.class_id = 1;
        s.py.misc.au = 0;
        s.py.misc.fos = 0;
        s.py.misc.chance_in_search = 100;
        s.py.stats.used[PlayerAttr::A_STR as usize] = 18;
        s.py.pack.unique_items = 0;
        s.py.pack.weight = 0;
        s.py.pack.heaviness = 0;
        s.py.flags.blind = 0;
        s.py.flags.confused = 0;
        s.py.flags.paralysis = 0;
        s.py.flags.slow = 0;
        s.py.flags.free_action = false;
        s.py.flags.status = 0;
        s.py.running_tracker = 0;
        s.dg.panel.row = 0;
        s.dg.panel.col = 0;
        s.dg.panel.top = bounds.top;
        s.dg.panel.bottom = bounds.bottom;
        s.dg.panel.left = bounds.left;
        s.dg.panel.right = bounds.right;
        s.dg.panel.row_prt = bounds.row_prt;
        s.dg.panel.col_prt = bounds.col_prt;
        s.dg.floor[POS.y as usize][POS.x as usize].creature_id = 1;
        s.dg.floor[POS.y as usize][POS.x as usize].temporary_light = true;
        for i in 0..PLAYER_INVENTORY_SIZE as usize {
            inventory_item_copy_to(OBJ_NOTHING as i16, &mut s.py.inventory[i]);
        }
    });
    init_monster_levels();
}

fn set_tile(coord: Coord_t, tile: Tile) {
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize] = tile;
    });
}

fn place_treasure_at(coord: Coord_t, obj_id: u16) {
    let tid = popt() as u8;
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize].treasure_id = tid;
        inventory_item_copy_to(obj_id as i16, &mut s.game.treasure.list[tid as usize]);
    });
}

fn place_trap_at(coord: Coord_t, trap_index: i32) {
    set_tile(
        coord,
        Tile {
            feature_id: TILE_LIGHT_FLOOR,
            ..Tile::default()
        },
    );
    dungeon_set_trap(coord, trap_index);
}

fn reset_monster_slots() {
    with_state_mut(|s| {
        s.next_free_monster_id = 2;
        s.monsters = [Monster::default(); MON_TOTAL_ALLOCATIONS as usize];
    });
}

fn place_monster(id: i32, creature_id: u16, hp: i16, coord: Coord_t, lit: bool) {
    with_state_mut(|s| {
        s.monsters[id as usize] = Monster {
            hp,
            creature_id,
            pos: coord,
            lit,
            ..Default::default()
        };
        s.dg.floor[coord.y as usize][coord.x as usize].creature_id = id as u8;
        if lit {
            s.dg.floor[coord.y as usize][coord.x as usize].permanent_light = true;
        }
        if id + 1 >= i32::from(s.next_free_monster_id) {
            s.next_free_monster_id = (id + 1) as i16;
        }
    });
}

// ---------------------------------------------------------------------------
// 1. Confusion random-move RNG order
// ---------------------------------------------------------------------------

#[test]
fn confused_move_consumes_random_four_then_nine_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon();
    setup_player();
    disable_auto_search();
    with_state_mut(|s| s.py.flags.confused = 5);
    open_floor_north_of_player();
    set_tile(
        Coord_t { y: 9, x: 11 },
        Tile {
            feature_id: TILE_LIGHT_FLOOR,
            ..Tile::default()
        },
    );

    player_move(NORTH, false);

    with_state(|s| assert_eq!(s.py.pos, Coord_t { y: 9, x: 11 }));
    assert_eq!(next_random_pair(4), (4, 2));
    assert_eq!(next_random_pair(9), (9, 7));
}

#[test]
fn sit_direction_never_randomizes_even_when_confused() {
    reset_for_new_game(Some(42));
    setup_dungeon();
    setup_player();
    disable_auto_search();
    with_state_mut(|s| s.py.flags.confused = 5);

    player_move(5, false);

    with_state(|s| assert_eq!(s.py.pos, POS));
    assert_eq!(next_random_pair(4), (4, 1));
}

#[test]
fn sober_move_still_consumes_random_four_seed42() {
    // C++ playerRandomMovement always draws randomNumber(4) when dir != 5.
    reset_for_new_game(Some(42));
    setup_dungeon();
    setup_player();
    with_state_mut(|s| {
        // fos > 1 so the fos gate consumes one roll; PY_SEARCH off.
        s.py.misc.fos = 50;
        s.py.misc.chance_in_search = 0;
        s.py.flags.status = 0;
    });
    open_floor_north_of_player();

    player_move(NORTH, false);

    with_state(|s| assert_eq!(s.py.pos, Coord_t { y: 9, x: 10 }));
    // Draws: randomNumber(4) then randomNumber(50). Next randomNumber(4)
    // must not be the first seed42 draw (1) — that was already consumed.
    assert_ne!(next_random_pair(4), (4, 1));
}

// ---------------------------------------------------------------------------
// 2. Search gate — fos short-circuit / skip paths
// ---------------------------------------------------------------------------

#[test]
fn search_fos_le_one_skips_random_fos_roll() {
    reset_for_new_game(Some(99));
    setup_dungeon();
    setup_player();
    with_state_mut(|s| {
        s.py.misc.fos = 1;
        s.py.misc.chance_in_search = 100;
    });
    set_tile(
        Coord_t { y: 9, x: 10 },
        Tile {
            feature_id: TILE_LIGHT_FLOOR,
            ..Tile::default()
        },
    );

    player_move(NORTH, false);

    // After sober move, C++ always consumes randomNumber(4) before search.
    assert_eq!(next_random_pair(100), (100, 15));
}

#[test]
fn search_fos_gt_one_skips_search_when_roll_not_one_seed77() {
    reset_for_new_game(Some(77));
    setup_dungeon();
    setup_player();
    with_state_mut(|s| {
        s.py.misc.fos = 10;
        s.py.misc.chance_in_search = 50;
    });
    open_floor_north_of_player();

    player_move(NORTH, false);

    assert_eq!(next_random_pair(100), (100, 69));
}

#[test]
fn search_py_search_flag_triggers_search_without_fos_roll() {
    reset_for_new_game(Some(77));
    setup_dungeon();
    setup_player();
    with_state_mut(|s| {
        s.py.misc.fos = 50;
        s.py.flags.status |= PY_SEARCH;
        s.py.misc.chance_in_search = 50;
    });
    open_floor_north_of_player();

    player_move(NORTH, false);

    // Search ran (PY_SEARCH) without consuming randomNumber(50); next roll after
    // the always-drawn randomNumber(4) from playerRandomMovement.
    assert_eq!(next_random_pair(100), (100, 91));
}

// ---------------------------------------------------------------------------
// 3. Trap-type dispatch table
// ---------------------------------------------------------------------------

fn trap_test_setup(seed: u32) {
    reset_for_new_game(Some(seed));
    setup_dungeon();
    setup_player();
}

#[test]
fn trap_sleeping_gas_paralysis_and_message_seed55() {
    trap_test_setup(55);
    place_trap_at(Coord_t { y: 9, x: 10 }, 4);

    player_move(NORTH, false);

    with_state(|s| assert_eq!(s.py.flags.paralysis, 12));
    assert!(message_text(1).contains("white mist") || message_text(0).contains("white mist"));
    assert!(message_text(2).contains("fall asleep") || message_text(1).contains("fall asleep"));
    assert_eq!(next_random_pair(10), (10, 1));
}

#[test]
fn trap_sleeping_gas_free_action_gate() {
    reset_for_new_game(Some(55));
    setup_dungeon();
    setup_player();
    with_state_mut(|s| {
        s.py.flags.free_action = true;
    });
    place_trap_at(Coord_t { y: 9, x: 10 }, 4);

    player_move(NORTH, false);

    with_state(|s| assert_eq!(s.py.flags.paralysis, 0));
    assert!(message_text(1).contains("unaffected"));
}

#[test]
fn trap_sleeping_gas_existing_paralysis_short_circuit() {
    trap_test_setup(55);
    with_state_mut(|s| s.py.flags.paralysis = 3);
    place_trap_at(Coord_t { y: 9, x: 10 }, 4);

    player_move(NORTH, false);

    with_state(|s| assert_eq!(s.py.flags.paralysis, 3));
    assert_eq!(next_random_pair(10), (10, 8));
}

#[test]
fn trap_blind_gas_counter_math_seed60() {
    trap_test_setup(60);
    place_trap_at(Coord_t { y: 9, x: 10 }, 14);

    player_move(NORTH, false);

    with_state(|s| assert_eq!(s.py.flags.blind, 59));
    assert_eq!(next_random_pair(50), (50, 13));
}

#[test]
fn trap_confuse_gas_counter_math_seed61() {
    trap_test_setup(61);
    place_trap_at(Coord_t { y: 9, x: 10 }, 15);

    player_move(NORTH, false);

    with_state(|s| assert_eq!(s.py.flags.confused, 27));
    assert_eq!(next_random_pair(15), (15, 13));
}

#[test]
fn trap_teleport_sets_flag() {
    reset_for_new_game(None);
    setup_dungeon();
    setup_player();
    place_trap_at(Coord_t { y: 9, x: 10 }, 7);

    player_move(NORTH, false);

    with_state(|s| assert!(s.game.teleport_player));
}

#[test]
fn trap_summon_monster_rng_count_seed62() {
    trap_test_setup(62);
    place_trap_at(Coord_t { y: 9, x: 10 }, 10);

    player_move(NORTH, false);

    assert_eq!(next_random_pair(3), (3, 2));
}

// ---------------------------------------------------------------------------
// 4. Pickup vs no-pickup / gold
// ---------------------------------------------------------------------------

#[test]
fn gold_auto_pickup_without_do_pickup_flag() {
    reset_for_new_game(None);
    setup_dungeon();
    setup_player();
    let dest = Coord_t { y: 9, x: 10 };
    set_tile(
        dest,
        Tile {
            feature_id: TILE_LIGHT_FLOOR,
            ..Tile::default()
        },
    );
    place_treasure_at(dest, OBJ_GOLD_LIST);
    with_state_mut(|s| {
        s.game.treasure.list[s.dg.floor[dest.y as usize][dest.x as usize].treasure_id as usize]
            .cost = 42;
    });

    player_move(NORTH, false);

    with_state(|s| {
        assert_eq!(s.py.misc.au, 42);
        assert_eq!(s.dg.floor[dest.y as usize][dest.x as usize].treasure_id, 0);
    });
}

#[test]
fn do_pickup_carries_item_once() {
    reset_for_new_game(None);
    setup_dungeon();
    setup_player();
    disable_auto_search();
    let dest = Coord_t { y: 9, x: 10 };
    open_floor_north_of_player();
    place_treasure_at(dest, RATION_OBJ);

    player_move(NORTH, true);

    with_state(|s| {
        assert_eq!(s.py.pack.unique_items, 1);
        assert_eq!(s.py.inventory[0].category_id, TV_FOOD);
        assert_eq!(s.dg.floor[dest.y as usize][dest.x as usize].treasure_id, 0);
    });
}

#[test]
fn no_pickup_leaves_floor_item() {
    reset_for_new_game(None);
    setup_dungeon();
    setup_player();
    let dest = Coord_t { y: 9, x: 10 };
    open_floor_north_of_player();
    place_treasure_at(dest, RATION_OBJ);

    player_move(NORTH, false);

    with_state(|s| {
        assert_eq!(s.py.pack.unique_items, 0);
        assert_ne!(s.dg.floor[dest.y as usize][dest.x as usize].treasure_id, 0);
    });
}

// ---------------------------------------------------------------------------
// 5. Movement / attack routing
// ---------------------------------------------------------------------------

#[test]
fn open_floor_move_updates_position() {
    reset_for_new_game(None);
    setup_dungeon();
    setup_player();
    set_tile(
        Coord_t { y: 9, x: 10 },
        Tile {
            feature_id: TILE_LIGHT_FLOOR,
            ..Tile::default()
        },
    );

    player_move(NORTH, false);

    with_state(|s| assert_eq!(s.py.pos, Coord_t { y: 9, x: 10 }));
}

#[test]
fn lit_monster_tile_attacks_without_moving() {
    reset_for_new_game(None);
    setup_dungeon();
    setup_player();
    reset_monster_slots();
    let dest = Coord_t { y: 9, x: 10 };
    set_tile(
        dest,
        Tile {
            feature_id: TILE_LIGHT_FLOOR,
            creature_id: 2,
            ..Tile::default()
        },
    );
    place_monster(2, GREY_MUSHROOM_ID, 500, dest, true);
    with_state_mut(|s| {
        s.py.misc.bth = 40;
        s.py.misc.level = 10;
    });

    player_move(NORTH, false);

    with_state(|s| {
        assert_eq!(s.py.pos, POS);
        assert_eq!(s.monsters[2].sleep_count, 0);
    });
}

#[test]
fn wall_tile_sets_player_free_turn() {
    reset_for_new_game(None);
    setup_dungeon();
    setup_player();
    set_tile(
        Coord_t { y: 9, x: 10 },
        Tile {
            feature_id: TILE_GRANITE_WALL,
            ..Tile::default()
        },
    );

    player_move(NORTH, false);

    with_state(|s| {
        assert_eq!(s.py.pos, POS);
        assert!(s.game.player_free_turn);
    });
}

// ---------------------------------------------------------------------------
// 6. Integer semantics — i16 status counter +=
// ---------------------------------------------------------------------------

#[test]
fn status_counter_i16_wrap_on_blind_gas() {
    trap_test_setup(60);
    with_state_mut(|s| s.py.flags.blind = 32_000);
    place_trap_at(Coord_t { y: 9, x: 10 }, 14);

    player_move(NORTH, false);

    with_state(|s| assert_eq!(s.py.flags.blind, 32_059));
}

#[test]
fn trap_open_pit_dice_roll_before_effect_seed63() {
    trap_test_setup(63);
    place_trap_at(Coord_t { y: 9, x: 10 }, 0);

    player_move(NORTH, false);

    let trap_damage = GAME_OBJECTS[OBJ_TRAP_LIST as usize].damage;
    assert_eq!(trap_damage.dice, 2);
    with_state(|s| assert!(s.py.misc.current_hp < 500));
    assert_eq!(next_random_pair(i32::from(trap_damage.sides)), (6, 2));
}
