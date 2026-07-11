//! Room & vault builders in `dungeon_generate`.
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

use umoria::config::dungeon::objects::OBJ_SECRET_DOOR;
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_generate::{
    dungeon_build_room, dungeon_build_room_cross_shaped, dungeon_build_room_overlapping_rectangles,
    dungeon_build_room_with_inner_rooms, dungeon_place_four_small_rooms,
    dungeon_place_inner_pillars, dungeon_place_large_middle_pillar, dungeon_place_maze_inside_room,
    dungeon_place_random_secret_door, dungeon_place_treasure_vault, dungeon_place_vault,
};
use umoria::dungeon_tile::{
    Tile, TILE_BLOCKED_FLOOR, TILE_DARK_FLOOR, TILE_GRANITE_WALL, TILE_LIGHT_FLOOR, TMP1_WALL,
};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::treasure::TV_SECRET_DOOR;
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

// --------------------------------------------------------------------------
// 1. Per-builder golden grid + RNG
// --------------------------------------------------------------------------
#[test]
fn dungeon_build_room_rng_and_grid_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(66, 198);
    with_state_mut(|s| s.dg.current_level = 10);

    let coord = Coord_t { y: 30, x: 50 };
    dungeon_build_room(coord);

    assert_eq!(tile_at(coord).feature_id, TILE_DARK_FLOOR);
    assert!(tile_at(coord).perma_lit_room);
    assert_eq!(
        tile_at(Coord_t { y: 28, x: 40 }).feature_id,
        TILE_GRANITE_WALL
    );
    assert_eq!(
        tile_at(Coord_t { y: 32, x: 55 }).feature_id,
        TILE_GRANITE_WALL
    );
    assert_eq!(next_random_pair(12), (12, 6));
}

#[test]
fn dungeon_build_room_light_floor_seed1() {
    reset_for_new_game(Some(1));
    setup_dungeon(66, 198);
    with_state_mut(|s| s.dg.current_level = 1);

    dungeon_build_room(Coord_t { y: 30, x: 50 });

    assert_eq!(
        tile_at(Coord_t { y: 30, x: 50 }).feature_id,
        TILE_LIGHT_FLOOR
    );
    assert_eq!(next_random_pair(12), (12, 5));
}

#[test]
fn dungeon_build_room_overlapping_rectangles_rng_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(66, 198);
    with_state_mut(|s| s.dg.current_level = 10);

    dungeon_build_room_overlapping_rectangles(Coord_t { y: 30, x: 50 });

    let dark = count_feature(TILE_DARK_FLOOR);
    let granite = count_feature(TILE_GRANITE_WALL);
    assert!(dark > 50, "overlapping rects should paint floor");
    assert!(granite > 20, "overlapping rects should paint walls");
    assert_eq!(next_random_pair(12), (12, 9));
}

#[test]
fn dungeon_build_room_cross_shaped_middle_pillar_seed1() {
    reset_for_new_game(Some(1));
    setup_dungeon(66, 198);
    with_state_mut(|s| s.dg.current_level = 1);

    dungeon_build_room_cross_shaped(Coord_t { y: 30, x: 50 });

    assert_eq!(tile_at(Coord_t { y: 30, x: 50 }).feature_id, TMP1_WALL);
    assert_eq!(count_feature(TMP1_WALL), 9);
    assert_eq!(next_random_pair(12), (12, 2));
}

#[test]
fn dungeon_build_room_cross_shaped_treasure_vault_seed42() {
 // seed 42 / level 10: feature branch 2 (treasure vault). Vault paints 8
 // TMP1 walls; the secret door converts one to TILE_BLOCKED_FLOOR → 7.
    reset_for_new_game(Some(42));
    umoria::game_run::initialize_treasure_levels();
    umoria::game_run::initialize_monster_levels();
    setup_dungeon(66, 198);
    with_state_mut(|s| {
        s.dg.current_level = 10;
        s.game.treasure.current_id = 1;
        s.next_free_monster_id = i16::from(umoria::config::monsters::MON_MIN_INDEX_ID);
        s.py.pos = Coord_t { y: 1, x: 1 };
    });

    dungeon_build_room_cross_shaped(Coord_t { y: 30, x: 50 });

    assert_eq!(count_feature(TMP1_WALL), 7);
    assert!(with_state(|s| s.game.treasure.current_id) > 1);
    assert!(
        with_state(|s| s.next_free_monster_id)
            > i16::from(umoria::config::monsters::MON_MIN_INDEX_ID)
    );
}

#[test]
fn dungeon_build_room_with_inner_rooms_plain_seed4() {
 // seed 4 / level 10: InnerRoomTypes::Plain (randomNumber(5) == 1).
    assert_eq!(peek_inner_room_type_roll(4, 10), 1);

    reset_for_new_game(Some(4));
    umoria::game_run::initialize_treasure_levels();
    umoria::game_run::initialize_monster_levels();
    setup_dungeon(66, 198);
    with_state_mut(|s| {
        s.dg.current_level = 10;
        s.game.treasure.current_id = 1;
        s.next_free_monster_id = i16::from(umoria::config::monsters::MON_MIN_INDEX_ID);
        s.py.pos = Coord_t { y: 1, x: 1 };
    });

    dungeon_build_room_with_inner_rooms(Coord_t { y: 30, x: 50 });

 // seed 4 floor-tile roll yields light floor; plain branch leaves center as floor.
    assert_eq!(
        tile_at(Coord_t { y: 30, x: 50 }).feature_id,
        TILE_LIGHT_FLOOR
    );
    assert!(
        with_state(|s| s.next_free_monster_id)
            > i16::from(umoria::config::monsters::MON_MIN_INDEX_ID)
    );
}

// --------------------------------------------------------------------------
// 2. Helper builders
// --------------------------------------------------------------------------
#[test]
fn dungeon_place_vault_grid() {
    reset_for_new_game(None);
    setup_dungeon(40, 40);

    let coord = Coord_t { y: 20, x: 20 };
    dungeon_place_vault(coord);

    assert_eq!(tile_at(Coord_t { y: 19, x: 19 }).feature_id, TMP1_WALL);
    assert_eq!(tile_at(Coord_t { y: 19, x: 21 }).feature_id, TMP1_WALL);
    assert_eq!(tile_at(Coord_t { y: 21, x: 19 }).feature_id, TMP1_WALL);
    assert_eq!(tile_at(Coord_t { y: 21, x: 21 }).feature_id, TMP1_WALL);
    assert_eq!(tile_at(Coord_t { y: 19, x: 20 }).feature_id, TMP1_WALL);
    assert_eq!(tile_at(Coord_t { y: 21, x: 20 }).feature_id, TMP1_WALL);
    assert_eq!(tile_at(coord).feature_id, 0);
}

#[test]
fn dungeon_place_inner_pillars_core_only_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);

    let coord = Coord_t { y: 20, x: 20 };
    dungeon_place_inner_pillars(coord);

    assert_eq!(count_feature(TMP1_WALL), 9);
    assert_eq!(next_random_pair(12), (12, 9));
}

#[test]
fn dungeon_place_inner_pillars_with_wings_seed1() {
    reset_for_new_game(Some(1));
    setup_dungeon(40, 40);

    let coord = Coord_t { y: 20, x: 20 };
    dungeon_place_inner_pillars(coord);

    assert!(count_feature(TMP1_WALL) > 9);
    assert_eq!(next_random_pair(12), (12, 4));
}

#[test]
fn dungeon_place_maze_inside_room_checkerboard() {
    reset_for_new_game(None);
    setup_dungeon(40, 40);

    dungeon_place_maze_inside_room(22, 18, 15, 25);

    assert_eq!(tile_at(Coord_t { y: 18, x: 16 }).feature_id, 0);
    assert_eq!(tile_at(Coord_t { y: 18, x: 17 }).feature_id, TMP1_WALL);
    assert_eq!(tile_at(Coord_t { y: 19, x: 16 }).feature_id, TMP1_WALL);
    assert_eq!(tile_at(Coord_t { y: 19, x: 17 }).feature_id, 0);
}

#[test]
fn dungeon_place_large_middle_pillar_grid() {
    reset_for_new_game(None);
    setup_dungeon(40, 40);

    dungeon_place_large_middle_pillar(Coord_t { y: 20, x: 20 });

    assert_eq!(count_feature(TMP1_WALL), 9);
}

#[test]
fn dungeon_place_random_secret_door_south_wall_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    with_state_mut(|s| s.game.treasure.current_id = 1);

    let coord = Coord_t { y: 20, x: 20 };
    dungeon_place_random_secret_door(coord, 22, 18, 15, 25);

    let door = tile_at(Coord_t { y: 23, x: 20 });
    assert_eq!(door.feature_id, TILE_BLOCKED_FLOOR);
    assert_eq!(with_state(|s| s.game.treasure.list[1].id), OBJ_SECRET_DOOR);
    assert_eq!(
        with_state(|s| s.game.treasure.list[1].category_id),
        TV_SECRET_DOOR
    );
    assert_eq!(next_random_pair(12), (12, 9));
}

#[test]
fn dungeon_place_treasure_vault_rng_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    with_state_mut(|s| s.game.treasure.current_id = 1);

    let coord = Coord_t { y: 20, x: 20 };
    dungeon_place_treasure_vault(coord, 22, 18, 15, 25);

    assert_eq!(count_feature(TMP1_WALL), 7);
    assert_eq!(
        tile_at(Coord_t { y: 19, x: 20 }).feature_id,
        TILE_BLOCKED_FLOOR
    );
    assert_eq!(next_random_pair(7), (7, 2));
}

#[test]
fn dungeon_place_four_small_rooms_horizontal_doors_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    with_state_mut(|s| s.game.treasure.current_id = 1);

    let coord = Coord_t { y: 20, x: 20 };
    dungeon_place_four_small_rooms(coord, 22, 18, 15, 25);

    assert_eq!(tile_at(Coord_t { y: 20, x: 20 }).feature_id, TMP1_WALL);
    assert_eq!(tile_at(Coord_t { y: 18, x: 20 }).feature_id, TMP1_WALL);
    let doors = with_state(|s| {
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
    assert_eq!(doors, 4);
    assert_eq!(next_random_pair(12), (12, 4));
}

// --------------------------------------------------------------------------
// 3. Branch-coverage sweep — inner-room type (randomNumber(5))
// --------------------------------------------------------------------------
/// Replay the floor-tile + type rolls used by `dungeon_build_room_with_inner_rooms`.
fn peek_inner_room_type_roll(seed: u32, level: u8) -> i32 {
    reset_for_new_game(Some(seed));
    setup_dungeon(66, 198);
    with_state_mut(|s| s.dg.current_level = level as i16);
    let _ = umoria::dungeon_generate::dungeon_floor_tile_for_level();
    random_number(5)
}

#[test]
fn inner_room_type_branch_sweep_seeds() {
 // First seeds (level 10) where randomNumber(5) after floor-tile pick yields 1..=5.
    let expected: [(u32, i32); 5] = [(4, 1), (3, 2), (2, 3), (1, 4), (5, 5)];

    for (seed, want) in expected {
        assert_eq!(
            peek_inner_room_type_roll(seed, 10),
            want,
            "seed {seed} type roll"
        );

        reset_for_new_game(Some(seed));
        umoria::game_run::initialize_treasure_levels();
        umoria::game_run::initialize_monster_levels();
        setup_dungeon(66, 198);
        with_state_mut(|s| {
            s.dg.current_level = 10;
            s.game.treasure.current_id = 1;
            s.next_free_monster_id = i16::from(umoria::config::monsters::MON_MIN_INDEX_ID);
            s.py.pos = Coord_t { y: 1, x: 1 };
        });

        let coord = Coord_t { y: 30, x: 50 };
        dungeon_build_room_with_inner_rooms(coord);

 // Structural smoke per branch (grid heuristics are ambiguous for maze vs four-rooms).
        match want {
            1 => {
                assert_ne!(tile_at(coord).feature_id, TMP1_WALL);
                assert!(
                    with_state(|s| s.next_free_monster_id)
                        > i16::from(umoria::config::monsters::MON_MIN_INDEX_ID)
                );
            }
            2 => {
 // Vault ring; one wall may become a locked door (TILE_BLOCKED_FLOOR).
                assert!(count_feature(TMP1_WALL) >= 6);
                assert!(with_state(|s| s.game.treasure.current_id) > 1);
            }
            3 => assert_eq!(tile_at(coord).feature_id, TMP1_WALL),
            4 => assert!(count_feature(TMP1_WALL) > 80, "maze paints many TMP1 walls"),
            5 => {
                assert_eq!(tile_at(coord).feature_id, TMP1_WALL);
                assert_eq!(
                    tile_at(Coord_t {
                        y: coord.y,
                        x: coord.x - 1
                    })
                    .feature_id,
                    TMP1_WALL
                );
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn cross_shaped_feature_branch_sweep_seeds() {
 // level 5: seeds where feature randomNumber(4) == 1 (large middle pillar).
    let seeds_pillar = [1u32, 17];
    for seed in seeds_pillar {
        reset_for_new_game(Some(seed));
        setup_dungeon(66, 198);
        with_state_mut(|s| s.dg.current_level = 5);

        dungeon_build_room_cross_shaped(Coord_t { y: 30, x: 50 });
        assert!(
            count_feature(TMP1_WALL) >= 9,
            "seed {seed} should get middle pillar branch"
        );
    }
}
