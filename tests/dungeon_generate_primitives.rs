//! Dungeon generation primitives & helpers.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use umoria::config::dungeon::objects::{OBJ_CLOSED_DOOR, OBJ_SECRET_DOOR};
use umoria::dungeon::{coord_walls_next_to, MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_generate::{
    chance_of_random_direction, dungeon_blank_entire_cave, dungeon_fill_empty_tiles_with,
    dungeon_floor_tile_for_level, dungeon_new_spot, dungeon_place_boundary_walls,
    dungeon_place_door, dungeon_place_stairs, dungeon_place_streamer_rock,
    dungeon_place_vault_trap, pick_correct_direction, set_corridors, set_floors, set_rooms,
};
use umoria::dungeon_tile::{
    Tile, MAX_CAVE_FLOOR, TILE_BLOCKED_FLOOR, TILE_BOUNDARY_WALL, TILE_CORR_FLOOR, TILE_DARK_FLOOR,
    TILE_GRANITE_WALL, TILE_LIGHT_FLOOR, TILE_MAGMA_WALL, TILE_NULL_WALL, TMP1_WALL, TMP2_WALL,
};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::treasure::{TV_CLOSED_DOOR, TV_OPEN_DOOR, TV_SECRET_DOOR};
use umoria::types::Coord_t;

fn setup_dungeon(height: i16, width: i16) {
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
    });
}

fn tile_at(coord: Coord_t) -> umoria::dungeon_tile::Tile {
    with_state(|s| s.dg.floor[coord.y as usize][coord.x as usize])
}

fn set_tile(coord: Coord_t, feature_id: u8) {
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize].feature_id = feature_id;
    });
}

fn fill_interior(feature_id: u8) {
    with_state_mut(|s| {
        for y in 1..s.dg.height - 1 {
            for x in 1..s.dg.width - 1 {
                s.dg.floor[y as usize][x as usize].feature_id = feature_id;
            }
        }
    });
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

fn count_feature(feature_id: u8) -> i32 {
    with_state(|s| {
        let mut count = 0;
        for y in 0..s.dg.height {
            for x in 0..s.dg.width {
                if s.dg.floor[y as usize][x as usize].feature_id == feature_id {
                    count += 1;
                }
            }
        }
        count
    })
}

// ---------------------------------------------------------------------------
// 1. dungeonFloorTileForLevel / pickCorrectDirection / chanceOfRandomDirection
// ---------------------------------------------------------------------------
#[test]
fn dungeon_floor_tile_for_level_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(66, 198);
    with_state_mut(|s| s.dg.current_level = 10);
    // seed 42: randomNumber(25) == 9; 10 <= 9 is false -> dark floor
    assert_eq!(dungeon_floor_tile_for_level(), TILE_DARK_FLOOR);
    assert_eq!(next_random_pair(25), (25, 23));
}

#[test]
fn dungeon_floor_tile_for_level_light_when_level_at_or_below_roll() {
    reset_for_new_game(Some(42));
    setup_dungeon(66, 198);
    with_state_mut(|s| s.dg.current_level = 1);
    assert_eq!(dungeon_floor_tile_for_level(), TILE_LIGHT_FLOOR);
    assert_eq!(next_random_pair(25), (25, 23));
}

#[test]
fn dungeon_floor_tile_for_level_dark_when_level_above_roll() {
    reset_for_new_game(Some(42));
    setup_dungeon(66, 198);
    with_state_mut(|s| s.dg.current_level = 25);
    assert_eq!(dungeon_floor_tile_for_level(), TILE_DARK_FLOOR);
    assert_eq!(next_random_pair(12), (12, 9));
}

#[test]
fn pick_correct_direction_no_tie_break_rng_when_one_axis_zero() {
    reset_for_new_game(Some(42));
    setup_dungeon(10, 10);
    let (v, h) = pick_correct_direction(Coord_t { y: 5, x: 5 }, Coord_t { y: 8, x: 5 });
    assert_eq!((v, h), (1, 0));
    assert_eq!(next_random_pair(12), (12, 2));
}

#[test]
fn pick_correct_direction_tie_break_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(10, 10);
    // randomNumber(2) == 2 -> zero horizontal
    let (v, h) = pick_correct_direction(Coord_t { y: 5, x: 5 }, Coord_t { y: 8, x: 10 });
    assert_eq!((v, h), (1, 0));
    assert_eq!(next_random_pair(12), (12, 9));
}

#[test]
fn pick_correct_direction_tie_break_vertical_seed1() {
    reset_for_new_game(Some(1));
    setup_dungeon(10, 10);
    // randomNumber(2) == 1 -> zero vertical
    let (v, h) = pick_correct_direction(Coord_t { y: 3, x: 3 }, Coord_t { y: 7, x: 8 });
    assert_eq!((v, h), (0, 1));
}

#[test]
fn chance_of_random_direction_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(10, 10);
    // randomNumber(4) == 2 -> vertical 1
    assert_eq!(chance_of_random_direction(), (1, 0));
    assert_eq!(next_random_pair(12), (12, 9));
}

#[test]
fn chance_of_random_direction_vertical_up_seed1() {
    reset_for_new_game(Some(1));
    setup_dungeon(10, 10);
    // randomNumber(4) == 4 -> horizontal -1
    assert_eq!(chance_of_random_direction(), (0, -1));
}

// ---------------------------------------------------------------------------
// 2. dungeonPlaceStreamerRock golden
// ---------------------------------------------------------------------------
#[test]
fn dungeon_place_streamer_rock_rng_and_grid_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 60);
    fill_interior(TILE_GRANITE_WALL);
    dungeon_place_boundary_walls();
    with_state_mut(|s| {
        s.dg.current_level = 5;
        s.game.treasure.current_id = 1;
    });

    dungeon_place_streamer_rock(TILE_MAGMA_WALL, 90);

    let magma = count_feature(TILE_MAGMA_WALL);
    assert!(magma > 0, "streamer should place magma cells");
    assert_eq!(next_random_pair(25), (25, 22));
}

// ---------------------------------------------------------------------------
// 3. dungeonPlaceDoor distribution/order
// ---------------------------------------------------------------------------
#[test]
fn dungeon_place_door_open_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    fill_interior(TILE_CORR_FLOOR);
    with_state_mut(|s| s.game.treasure.current_id = 1);

    let coord = Coord_t { y: 10, x: 10 };
    // door_type=randomNumber(3)=2, sub=randomNumber(12)=9 -> closed
    dungeon_place_door(coord);

    assert_eq!(tile_at(coord).feature_id, TILE_BLOCKED_FLOOR);
    let item = with_state(|s| s.game.treasure.list[1]);
    assert_eq!(item.id, OBJ_CLOSED_DOOR);
    assert_eq!(item.category_id, TV_CLOSED_DOOR);
    assert_eq!(next_random_pair(7), (7, 6));
}

#[test]
fn dungeon_place_door_broken_seed14() {
    reset_for_new_game(Some(14));
    setup_dungeon(20, 20);
    fill_interior(TILE_CORR_FLOOR);
    with_state_mut(|s| s.game.treasure.current_id = 1);

    let coord = Coord_t { y: 5, x: 5 };
    dungeon_place_door(coord);

    assert_eq!(tile_at(coord).feature_id, TILE_CORR_FLOOR);
    let misc = with_state(|s| s.game.treasure.list[1].misc_use);
    assert_eq!(misc, 1);
    assert_eq!(
        with_state(|s| s.game.treasure.list[1].category_id),
        TV_OPEN_DOOR
    );
}

#[test]
fn dungeon_place_door_locked_misc_use_seed12345() {
    reset_for_new_game(Some(12345));
    setup_dungeon(20, 20);
    fill_interior(TILE_CORR_FLOOR);
    with_state_mut(|s| s.game.treasure.current_id = 1);

    let coord = Coord_t { y: 8, x: 8 };
    dungeon_place_door(coord);

    let misc = with_state(|s| s.game.treasure.list[1].misc_use);
    assert!((11..=20).contains(&misc));
    assert_eq!(tile_at(coord).feature_id, TILE_BLOCKED_FLOOR);
}

#[test]
fn dungeon_place_door_stuck_misc_use_negative_seed15() {
    reset_for_new_game(Some(15));
    setup_dungeon(20, 20);
    fill_interior(TILE_CORR_FLOOR);
    with_state_mut(|s| s.game.treasure.current_id = 1);

    let coord = Coord_t { y: 6, x: 6 };
    dungeon_place_door(coord);

    let misc = with_state(|s| s.game.treasure.list[1].misc_use);
    assert_eq!(misc, -15);
}

#[test]
fn dungeon_place_door_secret_seed100() {
    reset_for_new_game(Some(100));
    setup_dungeon(20, 20);
    fill_interior(TILE_CORR_FLOOR);
    with_state_mut(|s| s.game.treasure.current_id = 1);

    let coord = Coord_t { y: 7, x: 7 };
    dungeon_place_door(coord);

    assert_eq!(tile_at(coord).feature_id, TILE_BLOCKED_FLOOR);
    assert_eq!(with_state(|s| s.game.treasure.list[1].id), OBJ_SECRET_DOOR);
    assert_eq!(
        with_state(|s| s.game.treasure.list[1].category_id),
        TV_SECRET_DOOR
    );
}

// ---------------------------------------------------------------------------
// 4. dungeonPlaceStairs search
// ---------------------------------------------------------------------------
#[test]
fn dungeon_place_stairs_down_finds_first_open_cell_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(30, 30);
    fill_interior(TILE_CORR_FLOOR);
    dungeon_place_boundary_walls();
    with_state_mut(|s| s.game.treasure.current_id = 1);

    dungeon_place_stairs(2, 1, 3);

    let mut stair_coord = None;
    with_state(|s| {
        for y in 1..s.dg.height - 1 {
            for x in 1..s.dg.width - 1 {
                if s.dg.floor[y as usize][x as usize].treasure_id != 0 {
                    stair_coord = Some(Coord_t {
                        y: y as i32,
                        x: x as i32,
                    });
                }
            }
        }
    });
    let stair = stair_coord.expect("stair should be placed");
    assert!(coord_walls_next_to(stair) >= 0);
    assert_eq!(next_random_pair(25), (25, 5));
}

// ---------------------------------------------------------------------------
// 5. dungeonPlaceVaultTrap retry
// ---------------------------------------------------------------------------
#[test]
fn dungeon_place_vault_trap_places_on_valid_floor_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    fill_interior(TILE_CORR_FLOOR);
    with_state_mut(|s| s.game.treasure.current_id = 1);

    let center = Coord_t { y: 10, x: 10 };
    dungeon_place_vault_trap(center, Coord_t { y: 2, x: 4 }, 1);

    let traps = with_state(|s| {
        let mut n = 0;
        for y in 0..s.dg.height {
            for x in 0..s.dg.width {
                if s.dg.floor[y as usize][x as usize].treasure_id != 0 {
                    n += 1;
                }
            }
        }
        n
    });
    assert_eq!(traps, 1);
    assert_eq!(next_random_pair(25), (25, 2));
}

// ---------------------------------------------------------------------------
// 6. Fill / boundary / newSpot / set* predicates
// ---------------------------------------------------------------------------
#[test]
fn dungeon_blank_entire_cave_zeros_grid() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    set_tile(Coord_t { y: 5, x: 5 }, TILE_GRANITE_WALL);
    dungeon_blank_entire_cave();
    assert_eq!(tile_at(Coord_t { y: 5, x: 5 }).feature_id, TILE_NULL_WALL);
}

#[test]
fn dungeon_fill_empty_tiles_with_replaces_tmp_and_null() {
    reset_for_new_game(None);
    setup_dungeon(8, 8);
    set_tile(Coord_t { y: 1, x: 1 }, TILE_NULL_WALL);
    set_tile(Coord_t { y: 2, x: 2 }, TMP1_WALL);
    set_tile(Coord_t { y: 3, x: 3 }, TMP2_WALL);
    set_tile(Coord_t { y: 4, x: 4 }, TILE_CORR_FLOOR);
    dungeon_fill_empty_tiles_with(TILE_GRANITE_WALL);
    assert_eq!(
        tile_at(Coord_t { y: 1, x: 1 }).feature_id,
        TILE_GRANITE_WALL
    );
    assert_eq!(
        tile_at(Coord_t { y: 2, x: 2 }).feature_id,
        TILE_GRANITE_WALL
    );
    assert_eq!(
        tile_at(Coord_t { y: 3, x: 3 }).feature_id,
        TILE_GRANITE_WALL
    );
    assert_eq!(tile_at(Coord_t { y: 4, x: 4 }).feature_id, TILE_CORR_FLOOR);
}

#[test]
fn dungeon_place_boundary_walls_sets_perimeter() {
    reset_for_new_game(None);
    setup_dungeon(6, 8);
    fill_interior(TILE_GRANITE_WALL);
    dungeon_place_boundary_walls();
    assert_eq!(
        tile_at(Coord_t { y: 0, x: 0 }).feature_id,
        TILE_BOUNDARY_WALL
    );
    assert_eq!(
        tile_at(Coord_t { y: 0, x: 7 }).feature_id,
        TILE_BOUNDARY_WALL
    );
    assert_eq!(
        tile_at(Coord_t { y: 5, x: 0 }).feature_id,
        TILE_BOUNDARY_WALL
    );
    assert_eq!(
        tile_at(Coord_t { y: 5, x: 7 }).feature_id,
        TILE_BOUNDARY_WALL
    );
    assert_eq!(
        tile_at(Coord_t { y: 1, x: 1 }).feature_id,
        TILE_GRANITE_WALL
    );
}

#[test]
fn dungeon_new_spot_finds_open_floor_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    fill_interior(TILE_CORR_FLOOR);
    set_tile(Coord_t { y: 10, x: 10 }, TILE_BLOCKED_FLOOR);

    let spot = dungeon_new_spot();
    assert_eq!(tile_at(spot).feature_id, TILE_CORR_FLOOR);
    assert!(spot.y >= 1 && spot.y <= 18);
    assert!(spot.x >= 1 && spot.x <= 18);
    assert_eq!(next_random_pair(25), (25, 11));
}

#[test]
fn set_predicate_truth_tables() {
    assert!(set_rooms(i32::from(TILE_DARK_FLOOR)));
    assert!(set_rooms(i32::from(TILE_LIGHT_FLOOR)));
    assert!(!set_rooms(i32::from(TILE_CORR_FLOOR)));

    assert!(set_corridors(i32::from(TILE_CORR_FLOOR)));
    assert!(set_corridors(i32::from(TILE_BLOCKED_FLOOR)));
    assert!(!set_corridors(i32::from(TILE_DARK_FLOOR)));

    assert!(set_floors(i32::from(TILE_CORR_FLOOR)));
    assert!(set_floors(i32::from(MAX_CAVE_FLOOR)));
    assert!(!set_floors(i32::from(TILE_GRANITE_WALL)));
}

#[test]
fn dungeon_place_vault_monster_calls_summon() {
    reset_for_new_game(Some(42));
    umoria::game_run::initialize_monster_levels();
    setup_dungeon(20, 20);
    fill_interior(TILE_CORR_FLOOR);
    with_state_mut(|s| {
        s.next_free_monster_id = i16::from(umoria::config::monsters::MON_MIN_INDEX_ID);
        s.py.pos = Coord_t { y: 1, x: 1 };
        s.dg.current_level = 5;
    });
    let before = with_state(|s| s.next_free_monster_id);
    umoria::dungeon_generate::dungeon_place_vault_monster(Coord_t { y: 10, x: 10 }, 1);
    let after = with_state(|s| s.next_free_monster_id);
    assert!(
        after > before,
        "vault monster summon should place at least one"
    );
}
