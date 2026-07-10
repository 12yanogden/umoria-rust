//! Tunnels & intersection doors in `dungeon_generate`.
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

mod common;

use umoria::dungeon::{coord_corridor_walls_next_to, MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_generate::{
    door_index, doors_tk_at, dungeon_build_tunnel, dungeon_is_next_to,
    dungeon_place_door_if_next_to_two_walls, reset_door_queue,
};
use umoria::dungeon_tile::{
    Tile, TILE_BLOCKED_FLOOR, TILE_CORR_FLOOR, TILE_GRANITE_WALL, TILE_NULL_WALL, TMP2_WALL,
};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::rng::get_seed;
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

// ---------------------------------------------------------------------------
// 1. dungeonBuildTunnel golden
// ---------------------------------------------------------------------------
#[test]
fn dungeon_build_tunnel_null_wall_straight_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(30, 30);
    fill_interior(TILE_NULL_WALL);
    reset_door_queue();

    dungeon_build_tunnel(Coord_t { y: 15, x: 5 }, Coord_t { y: 15, x: 20 });

    assert_eq!(tile_at(Coord_t { y: 15, x: 5 }).feature_id, TILE_NULL_WALL);
    for x in 6..=20 {
        assert_eq!(
            tile_at(Coord_t { y: 15, x }).feature_id,
            TILE_CORR_FLOOR,
            "corridor at x={x}"
        );
    }
    assert_eq!(door_index(), 0);
    assert_eq!(next_random_pair(12), (12, 11));
}

#[test]
fn dungeon_build_tunnel_granite_tmp2_streamer_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(30, 30);
    fill_interior(TILE_GRANITE_WALL);
    reset_door_queue();

    dungeon_build_tunnel(Coord_t { y: 15, x: 5 }, Coord_t { y: 15, x: 20 });

    assert_eq!(tile_at(Coord_t { y: 15, x: 6 }).feature_id, TILE_CORR_FLOOR);
    assert_eq!(tile_at(Coord_t { y: 15, x: 7 }).feature_id, TMP2_WALL);
    assert_eq!(door_index(), 0);
    assert_eq!(next_random_pair(12), (12, 1));
}

#[test]
fn dungeon_build_tunnel_wandering_and_abort_seed1() {
    reset_for_new_game(Some(1));
    setup_dungeon(40, 40);
    fill_interior(TILE_GRANITE_WALL);
    for y in 10..=30 {
        set_tile(Coord_t { y, x: 20 }, TILE_CORR_FLOOR);
    }
    reset_door_queue();

    dungeon_build_tunnel(Coord_t { y: 5, x: 20 }, Coord_t { y: 35, x: 20 });

    let mut corr_on_column = 0;
    for y in 5..=35 {
        if tile_at(Coord_t { y, x: 20 }).feature_id == TILE_CORR_FLOOR {
            corr_on_column += 1;
        }
    }
    assert_eq!(corr_on_column, 21);
    assert_eq!(door_index(), 0);
    assert_eq!(next_random_pair(12), (12, 2));
}

#[test]
fn dungeon_build_tunnel_null_wall_pocket_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(30, 30);
    fill_interior(TILE_GRANITE_WALL);
    for y in 14..=16 {
        for x in 8..=12 {
            set_tile(Coord_t { y, x }, TILE_NULL_WALL);
        }
    }
    reset_door_queue();

    dungeon_build_tunnel(Coord_t { y: 15, x: 5 }, Coord_t { y: 15, x: 18 });

    assert_eq!(tile_at(Coord_t { y: 15, x: 10 }).feature_id, TILE_NULL_WALL);
    assert_eq!(next_random_pair(12), (12, 1));
}

// ---------------------------------------------------------------------------
// 2. dungeonPlaceDoorIfNextToTwoWalls guard
// ---------------------------------------------------------------------------
#[test]
fn dungeon_place_door_if_next_to_two_walls_no_rng_when_not_corr() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    fill_interior(TILE_GRANITE_WALL);
    let coord = Coord_t { y: 10, x: 10 };
    set_tile(coord, TILE_GRANITE_WALL);

    let seed_before = get_seed();
    dungeon_place_door_if_next_to_two_walls(coord);

    assert_eq!(tile_at(coord).feature_id, TILE_GRANITE_WALL);
    assert_eq!(get_seed(), seed_before);
}

#[test]
fn dungeon_place_door_if_next_to_two_walls_no_door_when_not_next_to_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    fill_interior(TILE_GRANITE_WALL);
    let coord = Coord_t { y: 10, x: 10 };
    set_tile(coord, TILE_CORR_FLOOR);
    for dx in -1..=1 {
        set_tile(Coord_t { y: 10, x: 10 + dx }, TILE_CORR_FLOOR);
    }

    dungeon_place_door_if_next_to_two_walls(coord);

    assert_eq!(tile_at(coord).feature_id, TILE_CORR_FLOOR);
    assert_eq!(next_random_pair(12), (12, 9));
}

#[test]
fn dungeon_place_door_if_next_to_two_walls_places_door_seed3() {
    reset_for_new_game(Some(3));
    setup_dungeon(20, 20);
    fill_interior(TILE_GRANITE_WALL);
    with_state_mut(|s| s.game.treasure.current_id = 1);

    let coord = Coord_t { y: 10, x: 10 };
    set_tile(coord, TILE_CORR_FLOOR);
    set_tile(Coord_t { y: 9, x: 10 }, TILE_GRANITE_WALL);
    set_tile(Coord_t { y: 11, x: 10 }, TILE_GRANITE_WALL);
    set_tile(Coord_t { y: 10, x: 9 }, TILE_CORR_FLOOR);
    set_tile(Coord_t { y: 10, x: 11 }, TILE_CORR_FLOOR);

    assert!(dungeon_is_next_to(coord));
    dungeon_place_door_if_next_to_two_walls(coord);

    assert_eq!(tile_at(coord).feature_id, TILE_BLOCKED_FLOOR);
    assert_eq!(next_random_pair(7), (7, 3));
}

// ---------------------------------------------------------------------------
// 3. dungeonIsNextTo truth table
// ---------------------------------------------------------------------------
#[test]
fn dungeon_is_next_to_truth_table() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    fill_interior(TILE_GRANITE_WALL);

    let junction = Coord_t { y: 10, x: 10 };
    set_tile(junction, TILE_CORR_FLOOR);
    set_tile(Coord_t { y: 9, x: 10 }, TILE_GRANITE_WALL);
    set_tile(Coord_t { y: 11, x: 10 }, TILE_GRANITE_WALL);
    set_tile(Coord_t { y: 10, x: 9 }, TILE_CORR_FLOOR);
    set_tile(Coord_t { y: 10, x: 11 }, TILE_CORR_FLOOR);
    assert!(coord_corridor_walls_next_to(junction) > 2);
    assert!(dungeon_is_next_to(junction));

    let tee = Coord_t { y: 12, x: 10 };
    set_tile(tee, TILE_CORR_FLOOR);
    set_tile(Coord_t { y: 11, x: 10 }, TILE_CORR_FLOOR);
    set_tile(Coord_t { y: 13, x: 10 }, TILE_GRANITE_WALL);
    set_tile(Coord_t { y: 12, x: 9 }, TILE_CORR_FLOOR);
    set_tile(Coord_t { y: 12, x: 11 }, TILE_CORR_FLOOR);
    assert!(coord_corridor_walls_next_to(tee) > 2);
    assert!(!dungeon_is_next_to(tee));

    let isolated = Coord_t { y: 14, x: 10 };
    set_tile(isolated, TILE_CORR_FLOOR);
    set_tile(Coord_t { y: 13, x: 10 }, TILE_CORR_FLOOR);
    set_tile(Coord_t { y: 15, x: 10 }, TILE_GRANITE_WALL);
    set_tile(Coord_t { y: 14, x: 9 }, TILE_GRANITE_WALL);
    set_tile(Coord_t { y: 14, x: 11 }, TILE_GRANITE_WALL);
    assert!(coord_corridor_walls_next_to(isolated) <= 2);
    assert!(!dungeon_is_next_to(isolated));

    let horizontal_wall_pair = Coord_t { y: 16, x: 10 };
    set_tile(horizontal_wall_pair, TILE_CORR_FLOOR);
    set_tile(Coord_t { y: 16, x: 9 }, TILE_GRANITE_WALL);
    set_tile(Coord_t { y: 16, x: 11 }, TILE_GRANITE_WALL);
    set_tile(Coord_t { y: 15, x: 10 }, TILE_CORR_FLOOR);
    set_tile(Coord_t { y: 17, x: 10 }, TILE_CORR_FLOOR);
    assert!(coord_corridor_walls_next_to(horizontal_wall_pair) > 2);
    assert!(dungeon_is_next_to(horizontal_wall_pair));
}

// ---------------------------------------------------------------------------
// 4. Door-queue integration
// ---------------------------------------------------------------------------
#[test]
fn door_queue_integration_two_rooms_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    fill_interior(TILE_GRANITE_WALL);
    for y in 18..=22 {
        for x in 10..=14 {
            set_tile(Coord_t { y, x }, TILE_CORR_FLOOR);
        }
    }
    for y in 18..=22 {
        for x in 26..=30 {
            set_tile(Coord_t { y, x }, TILE_CORR_FLOOR);
        }
    }
    reset_door_queue();

    dungeon_build_tunnel(Coord_t { y: 20, x: 14 }, Coord_t { y: 20, x: 26 });

    assert_eq!(door_index(), 1);
    assert_eq!(doors_tk_at(0).y, 20);
    assert_eq!(doors_tk_at(0).x, 14);
    assert_eq!(next_random_pair(12), (12, 4));
}
