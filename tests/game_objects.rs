//! `game_objects` tests.
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

use umoria::config::dungeon::objects::OBJ_NOTHING;
use umoria::data_treasure::GAME_OBJECTS;
use umoria::dungeon::{DungeonObject, MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::game_objects::{item_bigger_than_chest, item_get_random_object_id, popt, pusht};
use umoria::inventory::inventory_item_copy_to;
use umoria::treasure::{
    TV_BOW, TV_CHEST, TV_DIGGING, TV_HAFTED, TV_HARD_ARMOR, TV_MISC, TV_NOTHING, TV_POLEARM,
    TV_SOFT_ARMOR, TV_STAFF, TV_SWORD, TV_VIS_TRAP,
};
use umoria::types::{Coord_t, LEVEL_MAX_OBJECTS, MAX_DUNGEON_OBJECTS, TREASURE_MAX_LEVELS};
use umoria::ui_io::test_set_ncurses_stub;

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

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

fn count_treasure_tiles() -> i32 {
    with_state(|s| {
        let mut n = 0;
        for y in 0..s.dg.height {
            for x in 0..s.dg.width {
                if s.dg.floor[y as usize][x as usize].treasure_id != 0 {
                    n += 1;
                }
            }
        }
        n
    })
}

fn obj(category_id: u8, weight: u16) -> DungeonObject {
    DungeonObject {
        category_id,
        weight,
        ..DungeonObject::default()
    }
}

// --------------------------------------------------------------------------
// 1. itemGetRandomObjectId RNG-order golden capture
// --------------------------------------------------------------------------
#[test]
fn item_get_random_object_id_level_zero_seed42() {
    reset_for_new_game(Some(42));
    init_treasure_levels();
    assert_eq!(item_get_random_object_id(0, false), 13);
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn item_get_random_object_id_level_one_seed42() {
    reset_for_new_game(Some(42));
    init_treasure_levels();
    assert_eq!(item_get_random_object_id(1, false), 45);
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn item_get_random_object_id_level_five_seed42() {
    reset_for_new_game(Some(42));
    init_treasure_levels();
    assert_eq!(item_get_random_object_id(5, false), 78);
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn item_get_random_object_id_level_ten_seed42() {
    reset_for_new_game(Some(42));
    init_treasure_levels();
    assert_eq!(item_get_random_object_id(10, false), 95);
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn item_get_random_object_id_level_fifty_seed42() {
    reset_for_new_game(Some(42));
    init_treasure_levels();
    assert_eq!(item_get_random_object_id(50, false), 284);
    assert_eq!(next_random_pair(100), (100, 74));
}

#[test]
fn item_get_random_object_id_level_fifty_one_clamps_seed42() {
    reset_for_new_game(Some(42));
    init_treasure_levels();
    assert_eq!(item_get_random_object_id(51, false), 284);
    assert_eq!(next_random_pair(100), (100, 74));
}

#[test]
fn item_get_random_object_id_seed777_level10() {
    reset_for_new_game(Some(777));
    init_treasure_levels();
    assert_eq!(item_get_random_object_id(10, false), 143);
    assert_eq!(next_random_pair(100), (100, 93));
}

#[test]
fn item_get_random_object_id_must_be_small_retries_seed2_level30() {
    reset_for_new_game(Some(2));
    init_treasure_levels();
    assert_eq!(item_get_random_object_id(30, false), 197);
    reset_for_new_game(Some(2));
    init_treasure_levels();
    assert_eq!(item_get_random_object_id(30, true), 120);
    assert_eq!(next_random_pair(100), (100, 76));
}

// --------------------------------------------------------------------------
// 2. popt parity
// --------------------------------------------------------------------------
#[test]
fn popt_returns_current_id_and_increments() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| s.game.treasure.current_id = 5);
    assert_eq!(popt(), 5);
    with_state(|s| assert_eq!(s.game.treasure.current_id, 6));
}

#[test]
fn popt_triggers_compact_at_capacity_seed42() {
    reset_for_new_game(Some(42));
    init_treasure_levels();
    setup_dungeon(20, 20);
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.game.treasure.current_id = i16::from(LEVEL_MAX_OBJECTS);
        for tid in 1..=174i32 {
            inventory_item_copy_to(378, &mut s.game.treasure.list[tid as usize]);
            let y = 1 + (tid % 18) as usize;
            let x = 1 + (tid / 18) as usize;
            s.dg.floor[y][x].treasure_id = tid as u8;
        }
    });
    assert_eq!(count_treasure_tiles(), 174);
    assert_eq!(popt(), 174);
    with_state(|s| assert_eq!(s.game.treasure.current_id, 175));
    assert_eq!(count_treasure_tiles(), 173);
    assert_eq!(next_random_pair(100), (100, 2));
}

// --------------------------------------------------------------------------
// 3. pusht parity
// --------------------------------------------------------------------------
#[test]
fn pusht_moves_last_slot_and_updates_grid_seed42() {
    reset_for_new_game(Some(42));
    init_treasure_levels();
    setup_dungeon(20, 20);
    with_state_mut(|s| {
        s.game.treasure.current_id = 3;
        s.py.pos = Coord_t { y: 10, x: 10 };
        inventory_item_copy_to(378, &mut s.game.treasure.list[2]);
        s.dg.floor[5][5].treasure_id = 2;
    });
    pusht(1);
    with_state(|s| {
        assert_eq!(s.game.treasure.current_id, 2);
        assert_eq!(s.dg.floor[5][5].treasure_id, 1);
        assert_eq!(s.game.treasure.list[1].category_id, TV_VIS_TRAP);
        assert_eq!(s.game.treasure.list[2].id, OBJ_NOTHING);
    });
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn pusht_top_slot_skips_grid_move() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.game.treasure.current_id = 3;
        inventory_item_copy_to(378, &mut s.game.treasure.list[2]);
    });
    pusht(2);
    with_state(|s| {
        assert_eq!(s.game.treasure.current_id, 2);
        assert_eq!(s.game.treasure.list[2].id, OBJ_NOTHING);
        assert_eq!(s.game.treasure.list[2].category_id, TV_NOTHING);
    });
}

// --------------------------------------------------------------------------
// 4. itemBiggerThanChest boundary checks
// --------------------------------------------------------------------------
#[test]
fn item_bigger_than_chest_always_large_categories() {
    for category in [
        TV_CHEST,
        TV_BOW,
        TV_POLEARM,
        TV_HARD_ARMOR,
        TV_SOFT_ARMOR,
        TV_STAFF,
    ] {
        assert!(
            item_bigger_than_chest(&obj(category, 0)),
            "category {category} should be too big"
        );
    }
}

#[test]
fn item_bigger_than_chest_weight_boundary_for_blade_types() {
    assert!(!item_bigger_than_chest(&obj(TV_SWORD, 150)));
    assert!(item_bigger_than_chest(&obj(TV_SWORD, 151)));
    assert!(!item_bigger_than_chest(&obj(TV_HAFTED, 150)));
    assert!(item_bigger_than_chest(&obj(TV_HAFTED, 151)));
    assert!(!item_bigger_than_chest(&obj(TV_DIGGING, 150)));
    assert!(item_bigger_than_chest(&obj(TV_DIGGING, 151)));
}

#[test]
fn item_bigger_than_chest_default_false() {
    assert!(!item_bigger_than_chest(&obj(TV_MISC, 500)));
}

// --------------------------------------------------------------------------
// 5. Integer semantics
// --------------------------------------------------------------------------
#[test]
fn pusht_u8_equality_skips_move_when_top_slot() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.game.treasure.current_id = 1;
        inventory_item_copy_to(378, &mut s.game.treasure.list[0]);
    });
    pusht(0);
    with_state(|s| {
        assert_eq!(s.game.treasure.current_id, 0);
        assert_eq!(s.game.treasure.list[0].id, OBJ_NOTHING);
        assert_eq!(s.game.treasure.list[0].category_id, TV_NOTHING);
    });
}

#[test]
fn popt_i16_current_id_at_level_max_objects() {
    reset_for_new_game(Some(42));
    init_treasure_levels();
    setup_dungeon(10, 10);
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 5, x: 5 };
        s.game.treasure.current_id = i16::from(LEVEL_MAX_OBJECTS);
        inventory_item_copy_to(378, &mut s.game.treasure.list[1]);
        s.dg.floor[2][2].treasure_id = 1;
    });
    let id = popt();
    assert!(id >= 1);
    with_state(|s| assert!(i32::from(s.game.treasure.current_id) <= i32::from(LEVEL_MAX_OBJECTS)));
}
