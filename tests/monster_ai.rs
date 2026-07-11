//! Monster movement & AI core tests.
#![allow(
    clippy::int_plus_one,
    reason = "test assertions use inclusive bound comparisons"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use umoria::config::dungeon::objects::OBJ_CLOSED_DOOR;
use umoria::config::monsters::move_flags::{
    CM_20_RANDOM, CM_75_RANDOM, CM_ATTACK_ONLY, CM_MOVE_NORMAL, CM_MULTIPLY, CM_OPEN_DOOR,
    CM_PICKS_UP,
};
use umoria::config::monsters::{self, MON_MULTIPLY_ADJUST};
use umoria::config::treasure::OBJECTS_RUNE_PROTECTION;
use umoria::data_creatures::CREATURES_LIST;
use umoria::dungeon::{coord_distance_between, MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{
    Tile, MIN_CAVE_WALL, TILE_CORR_FLOOR, TILE_GRANITE_WALL, TILE_LIGHT_FLOOR,
};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::game_objects::popt;
use umoria::inventory::inventory_item_copy_to;
use umoria::monster::{
    glyph_of_warding_protection, make_move, monster_allowed_to_move, monster_attack_without_moving,
    monster_do_move, monster_get_move_direction, monster_move, monster_move_confused,
    monster_move_normally, monster_move_out_of_wall, monster_move_randomly, monster_move_undead,
    monster_moves_on_player, monster_multiply, monster_multiply_critter, monster_open_door,
    Monster, MON_TOTAL_ALLOCATIONS,
};
use umoria::types::Coord_t;
use umoria::ui::panel_bounds_fields;
use umoria::ui_io::test_set_ncurses_stub;

const GIANT_YELLOW_CENTIPEDE_ID: u16 = 9;
const GIANT_WHITE_RAT_ID: u16 = 53;
const SINGING_HAPPY_DRUNK_ID: u16 = 5;
const GREY_MUSHROOM_ID: u16 = 8;
const SKELETON_KOBOLD_ID: u16 = 44;

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

fn reset_monster_slots() {
    with_state_mut(|s| {
        s.next_free_monster_id = i16::from(monsters::MON_MIN_INDEX_ID);
        s.monsters = [Monster::default(); MON_TOTAL_ALLOCATIONS as usize];
        s.hack_monptr = -1;
        s.monster_multiply_total = 0;
        s.game.treasure.current_id = 1;
    });
}

fn place_monster(id: i32, creature_id: u16, hp: i16, coord: Coord_t, lit: bool) {
    with_state_mut(|s| {
        s.monsters[id as usize] = Monster {
            hp,
            sleep_count: 0,
            creature_id,
            pos: coord,
            distance_from_player: coord_distance_between(s.py.pos, coord) as u8,
            lit,
            ..Default::default()
        };
        s.dg.floor[coord.y as usize][coord.x as usize].creature_id = id as u8;
        if id + 1 >= i32::from(s.next_free_monster_id) {
            s.next_free_monster_id = (id + 1) as i16;
        }
    });
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

fn place_closed_door(coord: Coord_t, misc_use: i16) -> u8 {
    let tid = popt() as u8;
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize].feature_id = MIN_CAVE_WALL;
        s.dg.floor[coord.y as usize][coord.x as usize].treasure_id = tid;
        inventory_item_copy_to(
            OBJ_CLOSED_DOOR as i16,
            &mut s.game.treasure.list[tid as usize],
        );
        s.game.treasure.list[tid as usize].misc_use = misc_use;
    });
    tid
}

// --------------------------------------------------------------------------
// 1. Direction generation parity
// --------------------------------------------------------------------------
#[test]
fn monster_get_move_direction_same_tile_case0() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    with_state_mut(|s| s.py.pos = Coord_t { y: 10, x: 10 });
    place_monster(2, 0, 10, Coord_t { y: 10, x: 10 }, false);

    let mut dirs = [0i32; 9];
    monster_get_move_direction(2, &mut dirs);
    assert_eq!(dirs[..5], [9, 6, 8, 3, 7]);
}

#[test]
fn monster_get_move_direction_north_case10() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    with_state_mut(|s| s.py.pos = Coord_t { y: 10, x: 10 });
    place_monster(2, 0, 10, Coord_t { y: 5, x: 10 }, false);

    let mut dirs = [0i32; 9];
    monster_get_move_direction(2, &mut dirs);
    assert_eq!(dirs[..5], [2, 1, 3, 4, 6]);
}

#[test]
fn monster_get_move_direction_east_case5() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    with_state_mut(|s| s.py.pos = Coord_t { y: 10, x: 10 });
    place_monster(2, 0, 10, Coord_t { y: 10, x: 15 }, false);

    let mut dirs = [0i32; 9];
    monster_get_move_direction(2, &mut dirs);
    assert_eq!(dirs[..5], [4, 7, 1, 8, 2]);
}

#[test]
fn monster_move_confused_consumes_five_random_nine_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    with_state_mut(|s| s.py.pos = Coord_t { y: 10, x: 10 });
    place_monster(
        2,
        GIANT_YELLOW_CENTIPEDE_ID,
        10,
        Coord_t { y: 8, x: 10 },
        false,
    );

    let creature = CREATURES_LIST[GIANT_YELLOW_CENTIPEDE_ID as usize];
    let mut rcmove = 0;
    monster_move_confused(&creature, 2, &mut rcmove);
    assert_eq!(next_random_pair(9), (9, 3));
}

#[test]
fn monster_move_normally_random_branch_seed199() {
    reset_for_new_game(Some(199));
    setup_dungeon(20, 20);
    reset_monster_slots();
    with_state_mut(|s| s.py.pos = Coord_t { y: 10, x: 10 });
    place_monster(
        2,
        GIANT_YELLOW_CENTIPEDE_ID,
        10,
        Coord_t { y: 8, x: 10 },
        false,
    );

    let mut rcmove = 0;
    monster_move_normally(2, &mut rcmove);
    assert!((rcmove & CM_MOVE_NORMAL) != 0);
    assert_eq!(next_random_pair(9), (9, 4));
}

// --------------------------------------------------------------------------
// 2. monster_move dispatch + random-move gates
// --------------------------------------------------------------------------
#[test]
fn monster_move_75_random_short_circuits_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.py.flags.rest = 0;
    });
    place_monster(
        2,
        SINGING_HAPPY_DRUNK_ID,
        50,
        Coord_t { y: 8, x: 10 },
        false,
    );

    let mut rcmove = 0;
    monster_move(2, &mut rcmove);
    assert!((rcmove & CM_75_RANDOM) != 0);
    assert_eq!(next_random_pair(9), (9, 5));
}

#[test]
fn monster_move_normally_branch_when_only_move_normal() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    test_set_ncurses_stub(true);
    with_state_mut(|s| s.py.pos = Coord_t { y: 10, x: 10 });
    place_monster(
        2,
        GIANT_YELLOW_CENTIPEDE_ID,
        20,
        Coord_t { y: 8, x: 10 },
        false,
    );

    let mut rcmove = 0;
    monster_move(2, &mut rcmove);
    assert!((rcmove & CM_MOVE_NORMAL) != 0);
}

#[test]
fn monster_attack_without_moving_sets_attack_only_when_far() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    with_state_mut(|s| s.py.pos = Coord_t { y: 10, x: 10 });
    place_monster(2, GREY_MUSHROOM_ID, 20, Coord_t { y: 5, x: 5 }, false);
    with_state_mut(|s| s.monsters[2].distance_from_player = 7);

    let mut rcmove = 0;
    monster_attack_without_moving(2, &mut rcmove, 7);
    assert!((rcmove & CM_ATTACK_ONLY) != 0);
}

// --------------------------------------------------------------------------
// 3. make_move / allowed / moves_on_player
// --------------------------------------------------------------------------
#[test]
fn make_move_updates_grid_and_distance_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    with_state_mut(|s| s.py.pos = Coord_t { y: 10, x: 10 });
    place_monster(
        2,
        GIANT_YELLOW_CENTIPEDE_ID,
        20,
        Coord_t { y: 8, x: 10 },
        false,
    );

    let mut dirs = [0i32; 9];
    monster_get_move_direction(2, &mut dirs);
    let mut rcmove = 0;
    make_move(2, &dirs, &mut rcmove);

    with_state(|s| {
        assert_eq!(s.monsters[2].pos, Coord_t { y: 9, x: 10 });
        assert_eq!(s.dg.floor[8][10].creature_id, 0);
        assert_eq!(s.dg.floor[9][10].creature_id, 2);
        assert_eq!(s.monsters[2].distance_from_player, 1);
    });
}

#[test]
fn monster_allowed_to_move_picks_up_object() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    with_state_mut(|s| s.py.pos = Coord_t { y: 10, x: 10 });
    place_monster(
        2,
        GIANT_YELLOW_CENTIPEDE_ID,
        20,
        Coord_t { y: 8, x: 10 },
        false,
    );

    let tid = popt() as u8;
    let target = Coord_t { y: 7, x: 10 };
    with_state_mut(|s| {
        s.dg.floor[target.y as usize][target.x as usize].treasure_id = tid;
        inventory_item_copy_to(0, &mut s.game.treasure.list[tid as usize]);
    });

    let mut do_turn = false;
    let mut rcmove = 0;
    monster_allowed_to_move(2, CM_PICKS_UP, &mut do_turn, &mut rcmove, target);
    assert!(do_turn);
    assert!((rcmove & CM_PICKS_UP) != 0);
    with_state(|s| assert_eq!(s.dg.floor[7][10].treasure_id, 0));
}

#[test]
fn monster_moves_on_player_attacks_and_sets_turn() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.py.misc.current_hp = 500;
        s.py.misc.max_hp = 500;
        s.game.character_is_dead = false;
    });
    place_monster(2, GREY_MUSHROOM_ID, 20, Coord_t { y: 9, x: 10 }, true);

    let mut do_move = true;
    let mut do_turn = false;
    let mut rcmove = 0;
    monster_moves_on_player(
        2,
        1,
        CM_MOVE_NORMAL,
        &mut do_move,
        &mut do_turn,
        &mut rcmove,
        Coord_t { y: 10, x: 10 },
    );
    assert!(!do_move);
    assert!(do_turn);
}

// --------------------------------------------------------------------------
// 4. monster_open_door parity
// --------------------------------------------------------------------------
#[test]
fn monster_open_door_closed_with_open_flag_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    test_set_ncurses_stub(true);
    let door = Coord_t { y: 8, x: 10 };
    place_closed_door(door, 0);

    let mut do_turn = false;
    let mut do_move = false;
    let mut rcmove = 0;
    monster_open_door(
        door,
        50,
        CM_OPEN_DOOR,
        &mut do_turn,
        &mut do_move,
        &mut rcmove,
    );

    assert!(do_turn);
    assert!(!do_move);
    assert!((rcmove & CM_OPEN_DOOR) != 0);
    with_state(|s| {
        assert_eq!(s.dg.floor[8][10].feature_id, TILE_CORR_FLOOR);
    });
}

#[test]
fn monster_open_door_locked_force_math_int16_hp() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    test_set_ncurses_stub(true);
    let door = Coord_t { y: 8, x: 10 };
    place_closed_door(door, 5);

    let mut do_turn = false;
    let mut do_move = false;
    let mut rcmove = 0;
    monster_open_door(
        door,
        30,
        CM_OPEN_DOOR,
        &mut do_turn,
        &mut do_move,
        &mut rcmove,
    );

    let lhs_max = (30 + 1) * (50 + 5);
    assert_eq!(next_random_pair(lhs_max), (lhs_max, 358));
    with_state(|s| {
        assert_eq!(
            s.game.treasure.list[s.dg.floor[8][10].treasure_id as usize].misc_use,
            5
        );
    });
}

// --------------------------------------------------------------------------
// 5. glyph_of_warding_protection parity
// --------------------------------------------------------------------------
#[test]
fn glyph_of_warding_blocks_when_roll_fails_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    test_set_ncurses_stub(true);
    let coord = Coord_t { y: 8, x: 10 };
    let tid = popt() as u8;
    with_state_mut(|s| {
        s.dg.floor[8][10].treasure_id = tid;
        inventory_item_copy_to(0, &mut s.game.treasure.list[tid as usize]);
        s.game.treasure.list[tid as usize].category_id = umoria::treasure::TV_VIS_TRAP;
        s.game.treasure.list[tid as usize].sub_category_id = 99;
    });

    let mut do_move = true;
    let mut do_turn = false;
    glyph_of_warding_protection(
        GREY_MUSHROOM_ID,
        CM_MOVE_NORMAL,
        &mut do_move,
        &mut do_turn,
        coord,
    );

    assert!(!do_move);
    assert_eq!(
        next_random_pair(i32::from(OBJECTS_RUNE_PROTECTION)),
        (3000, 1473)
    );
    with_state(|s| {
        assert_eq!(s.game.treasure.list[tid as usize].sub_category_id, 99);
    });
}

#[test]
fn glyph_of_warding_breaks_when_roll_passes() {
    reset_for_new_game(Some(1));
    setup_dungeon(20, 20);
    test_set_ncurses_stub(true);
    let coord = Coord_t { y: 8, x: 10 };
    let tid = popt() as u8;
    with_state_mut(|s| {
        s.dg.floor[8][10].treasure_id = tid;
        inventory_item_copy_to(0, &mut s.game.treasure.list[tid as usize]);
        s.game.treasure.list[tid as usize].category_id = umoria::treasure::TV_VIS_TRAP;
        s.game.treasure.list[tid as usize].sub_category_id = 99;
    });

    let mut do_move = true;
    let mut do_turn = false;
    glyph_of_warding_protection(
        GREY_MUSHROOM_ID,
        CM_ATTACK_ONLY,
        &mut do_move,
        &mut do_turn,
        coord,
    );

    assert!(!do_move);
    assert!(do_turn);
    with_state(|s| assert_eq!(s.dg.floor[8][10].treasure_id, 0));
}

// --------------------------------------------------------------------------
// 6. monster_move_out_of_wall / undead
// --------------------------------------------------------------------------
#[test]
fn monster_move_out_of_wall_exits_adjacent_open_space_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    test_set_ncurses_stub(true);
    with_state_mut(|s| s.py.pos = Coord_t { y: 10, x: 10 });
    let wall = Coord_t { y: 8, x: 10 };
    with_state_mut(|s| {
        s.dg.floor[8][10].feature_id = TILE_GRANITE_WALL;
        s.dg.floor[7][10].feature_id = TILE_LIGHT_FLOOR;
    });
    place_monster(2, GIANT_YELLOW_CENTIPEDE_ID, 100, wall, false);

    let mut rcmove = 0;
    monster_move_out_of_wall(2, &mut rcmove);
    with_state(|s| assert_eq!(s.monsters[2].pos, Coord_t { y: 7, x: 9 }));
    assert_eq!(next_random_pair(9), (9, 9));
}

#[test]
fn monster_move_undead_reverses_direction_and_rolls_tail_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    with_state_mut(|s| s.py.pos = Coord_t { y: 10, x: 10 });
    place_monster(
        2,
        GIANT_YELLOW_CENTIPEDE_ID,
        20,
        Coord_t { y: 8, x: 10 },
        false,
    );

    let creature = CREATURES_LIST[GIANT_YELLOW_CENTIPEDE_ID as usize];
    let mut rcmove = 0;
    monster_move_undead(&creature, 2, &mut rcmove);
    assert_eq!(next_random_pair(9), (9, 4));
}

// --------------------------------------------------------------------------
// 7. monster_multiply / monster_multiply_critter
// --------------------------------------------------------------------------
#[test]
fn monster_multiply_places_adjacent_and_increments_total_seed777() {
    reset_for_new_game(Some(777));
    setup_dungeon(20, 20);
    reset_monster_slots();
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.py.flags.speed = 0;
    });
    let origin = Coord_t { y: 8, x: 10 };
    place_monster(2, GIANT_WHITE_RAT_ID, 10, origin, false);

    let _ = monster_multiply(origin, i32::from(GIANT_WHITE_RAT_ID), 2);
    with_state(|s| {
        assert_eq!(s.monster_multiply_total, 1);
        assert!(s.dg.floor.iter().flatten().any(|t| t.creature_id == 3));
    });
    assert_eq!(next_random_pair(3), (3, 1));
}

#[test]
fn monster_multiply_critter_places_neighbor_seed777() {
    reset_for_new_game(Some(777));
    setup_dungeon(20, 20);
    reset_monster_slots();
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.py.flags.speed = 0;
        s.py.flags.rest = 0;
    });
    let origin = Coord_t { y: 8, x: 10 };
    place_monster(2, GIANT_WHITE_RAT_ID, 10, origin, false);

    let mut rcmove = 0;
    monster_multiply_critter(2, &mut rcmove);
    with_state(|s| assert_eq!(s.monster_multiply_total, 1));
    assert_eq!(next_random_pair(i32::from(MON_MULTIPLY_ADJUST)), (7, 5));
}

// --------------------------------------------------------------------------
// 8. Integer semantics / confused undead routing
// --------------------------------------------------------------------------
#[test]
fn monster_do_move_confused_undead_routes_to_undead_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    test_set_ncurses_stub(true);
    with_state_mut(|s| s.py.pos = Coord_t { y: 10, x: 10 });
    place_monster(2, SKELETON_KOBOLD_ID, 20, Coord_t { y: 8, x: 10 }, false);
    with_state_mut(|s| s.monsters[2].confused_amount = 2);

    let mut rcmove = 0;
    assert!(monster_do_move(2, &mut rcmove));
    with_state(|s| assert_eq!(s.monsters[2].confused_amount, 1));
    assert_eq!(next_random_pair(9), (9, 3));
}

#[test]
fn make_move_skips_boundary_wall_directions() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    with_state_mut(|s| s.py.pos = Coord_t { y: 10, x: 10 });
    place_monster(
        2,
        GIANT_YELLOW_CENTIPEDE_ID,
        20,
        Coord_t { y: 1, x: 10 },
        false,
    );

    let dirs = [4, 4, 4, 4, 4, 0, 0, 0, 0];
    let mut rcmove = 0;
    make_move(2, &dirs, &mut rcmove);
    with_state(|s| assert_eq!(s.monsters[2].pos.y, 1));
}

#[test]
fn monster_move_randomly_sets_randomness_flag_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    with_state_mut(|s| s.py.pos = Coord_t { y: 10, x: 10 });
    place_monster(
        2,
        GIANT_YELLOW_CENTIPEDE_ID,
        20,
        Coord_t { y: 8, x: 10 },
        false,
    );

    let mut rcmove = 0;
    monster_move_randomly(2, &mut rcmove, CM_20_RANDOM);
    assert!((rcmove & CM_20_RANDOM) != 0);
    assert_eq!(next_random_pair(9), (9, 3));
}

#[test]
fn monster_multiply_sets_rcmove_when_visible() {
    reset_for_new_game(Some(777));
    setup_dungeon(20, 20);
    reset_monster_slots();
    test_set_ncurses_stub(true);
    let bounds = panel_bounds_fields(0, 0);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.dg.panel.row = 0;
        s.dg.panel.col = 0;
        s.dg.panel.top = bounds.top;
        s.dg.panel.bottom = bounds.bottom;
        s.dg.panel.left = bounds.left;
        s.dg.panel.right = bounds.right;
        for y in 6..=12 {
            for x in 8..=12 {
                s.dg.floor[y][x].permanent_light = true;
            }
        }
    });
    place_monster(2, GIANT_WHITE_RAT_ID, 10, Coord_t { y: 8, x: 10 }, false);
    let mut rcmove = 0;
    monster_multiply_critter(2, &mut rcmove);
    assert!((rcmove & CM_MULTIPLY) != 0);
}
