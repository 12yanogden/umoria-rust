//! Dungeon core & tile helpers (`dungeon`).
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

use common::golden_root;
use umoria::config::dungeon::objects::OBJ_GOLD_LIST;
use umoria::config::player::status::PY_BLIND;
use umoria::data_creatures::CREATURES_LIST;
use umoria::dungeon::{
    cave_get_tile_symbol, cave_tile_visible, coord_corridor_walls_next_to, coord_distance_between,
    coord_in_bounds, coord_walls_next_to, dungeon_allocate_and_place_object,
    dungeon_delete_monster, dungeon_delete_monster_record, dungeon_delete_object,
    dungeon_place_gold, dungeon_place_random_object_near, dungeon_remove_monster_from_level,
    dungeon_summon_object, MAX_HEIGHT, MAX_WIDTH,
};
use umoria::dungeon_tile::{
    Tile, MAX_CAVE_FLOOR, MIN_CAVE_WALL, TILE_BLOCKED_FLOOR, TILE_CORR_FLOOR, TILE_GRANITE_WALL,
    TILE_LIGHT_FLOOR, TILE_MAGMA_WALL,
};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::treasure::{TV_INVIS_TRAP, TV_OPEN_DOOR, TV_RUBBLE};
use umoria::types::Coord_t;

fn setup_dungeon(height: i16, width: i16) {
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
    });
}

fn tile_at(coord: Coord_t) -> Tile {
    with_state(|s| s.dg.floor[coord.y as usize][coord.x as usize])
}

fn set_tile(coord: Coord_t, tile: Tile) {
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize] = tile;
    });
}

fn fill_floor(feature_id: u8) {
    with_state_mut(|s| {
        for y in 0..s.dg.height {
            for x in 0..s.dg.width {
                s.dg.floor[y as usize][x as usize].feature_id = feature_id;
            }
        }
    });
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

// --------------------------------------------------------------------------
// 1. Pure-helper parity (no RNG)
// --------------------------------------------------------------------------
#[test]
fn coord_in_bounds_hand_picked_table() {
    setup_dungeon(10, 20);
    let cases = [
        (Coord_t { y: 0, x: 0 }, false),
        (Coord_t { y: 1, x: 1 }, true),
        (Coord_t { y: 8, x: 18 }, true),
        (Coord_t { y: 9, x: 10 }, false),
        (Coord_t { y: 5, x: 19 }, false),
    ];
    for (coord, expected) in cases {
        assert_eq!(coord_in_bounds(coord), expected, "coord {coord:?}");
    }
}

#[test]
fn coord_distance_between_hand_picked_table() {
    let cases = [
        (Coord_t { y: 0, x: 0 }, Coord_t { y: 0, x: 0 }, 0),
        (Coord_t { y: 0, x: 0 }, Coord_t { y: 3, x: 4 }, 5),
        (Coord_t { y: 10, x: 20 }, Coord_t { y: 7, x: 15 }, 6),
        (Coord_t { y: 1, x: 1 }, Coord_t { y: 4, x: 1 }, 3),
        (Coord_t { y: 5, x: 8 }, Coord_t { y: 5, x: 12 }, 4),
    ];
    for (from, to, expected) in cases {
        assert_eq!(
            coord_distance_between(from, to),
            expected,
            "from {from:?} to {to:?}"
        );
    }
}

#[test]
fn coord_walls_next_to_counts_cardinal_walls() {
    setup_dungeon(10, 10);
    let center = Coord_t { y: 5, x: 5 };
    set_tile(
        Coord_t { y: 4, x: 5 },
        Tile {
            feature_id: MIN_CAVE_WALL,
            ..Default::default()
        },
    );
    set_tile(
        Coord_t { y: 6, x: 5 },
        Tile {
            feature_id: TILE_GRANITE_WALL,
            ..Default::default()
        },
    );
    set_tile(
        Coord_t { y: 5, x: 4 },
        Tile {
            feature_id: TILE_CORR_FLOOR,
            ..Default::default()
        },
    );
    assert_eq!(coord_walls_next_to(center), 2);
}

#[test]
fn coord_corridor_walls_next_to_counts_corr_without_doors() {
    setup_dungeon(10, 10);
    let center = Coord_t { y: 5, x: 5 };
    for (y, x) in [(4, 5), (6, 5), (5, 4), (5, 6)] {
        set_tile(
            Coord_t { y, x },
            Tile {
                feature_id: TILE_CORR_FLOOR,
                ..Default::default()
            },
        );
    }
    // Door treasure blocks counting (category_id >= TV_MIN_DOORS).
    with_state_mut(|s| {
        s.game.treasure.current_id = 1;
        s.dg.floor[4][5].treasure_id = 1;
        s.game.treasure.list[1].category_id = TV_OPEN_DOOR;
    });
    assert_eq!(coord_corridor_walls_next_to(center), 3);
}

#[test]
fn cave_tile_visible_truth_table() {
    setup_dungeon(10, 10);
    let coord = Coord_t { y: 3, x: 3 };
    for (permanent, temporary, field, expected) in [
        (true, false, false, true),
        (false, true, false, true),
        (false, false, true, true),
        (false, false, false, false),
    ] {
        set_tile(
            coord,
            Tile {
                permanent_light: permanent,
                temporary_light: temporary,
                field_mark: field,
                ..Default::default()
            },
        );
        assert_eq!(
            cave_tile_visible(coord),
            expected,
            "permanent={permanent} temporary={temporary} field={field}"
        );
    }
}

// --------------------------------------------------------------------------
// 2. caveGetTileSymbol RNG-order
// --------------------------------------------------------------------------
#[test]
fn cave_get_tile_symbol_no_rng_when_image_zero() {
    reset_for_new_game(Some(42));
    setup_dungeon(10, 10);
    with_state_mut(|s| {
        s.py.flags.image = 0;
        s.dg.floor[5][5].permanent_light = true;
        s.dg.floor[5][5].feature_id = TILE_LIGHT_FLOOR;
    });
    let sym = cave_get_tile_symbol(Coord_t { y: 5, x: 5 });
    assert_eq!(sym, b'.');
    assert_eq!(next_random_pair(12), (12, 2));
}

#[test]
fn cave_get_tile_symbol_rng_order_with_image() {
    reset_for_new_game(Some(42));
    setup_dungeon(10, 10);
    with_state_mut(|s| {
        s.py.flags.image = 1;
        s.dg.floor[5][5].permanent_light = true;
        s.dg.floor[5][5].feature_id = TILE_LIGHT_FLOOR;
    });
    let sym = cave_get_tile_symbol(Coord_t { y: 5, x: 5 });
    // seed 42: randomNumber(12) -> 9 (not 1), so floor tile '.'; next draws 9, 91.
    assert_eq!(sym, b'.');
    assert_eq!(next_random_pair(12), (12, 9));
    assert_eq!(next_random_pair(95), (95, 91));
}

#[test]
fn cave_get_tile_symbol_hallucination_symbol_from_golden_stream() {
    reset_for_new_game(Some(1));
    setup_dungeon(10, 10);
    with_state_mut(|s| {
        s.py.flags.image = 1;
        s.dg.floor[2][2].permanent_light = true;
        s.dg.floor[2][2].feature_id = TILE_LIGHT_FLOOR;
    });
    let sym = cave_get_tile_symbol(Coord_t { y: 2, x: 2 });
    // seed 1: randomNumber(12)==1, randomNumber(95)+31 == 46
    assert_eq!(sym, 46);
}

// --------------------------------------------------------------------------
// 3. dungeonPlaceGold RNG-order
// --------------------------------------------------------------------------
#[test]
fn dungeon_place_gold_rng_order_and_result_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    with_state_mut(|s| {
        s.dg.current_level = 5;
        s.game.treasure.current_id = 1;
    });
    let coord = Coord_t { y: 5, x: 5 };
    dungeon_place_gold(coord);

    let treasure_id = tile_at(coord).treasure_id;
    assert_eq!(treasure_id, 1);
    let item = with_state(|s| s.game.treasure.list[treasure_id as usize]);
    assert_eq!(item.category_id, umoria::treasure::TV_GOLD);
    // gold_type_id = ((randomNumber(7)+2)/2)-1 with first roll 5 -> ((5+2)/2)-1 = 2
    assert_eq!(item.id, OBJ_GOLD_LIST);
    // base cost 3; cost += 8*randomNumber(3)+randomNumber(8) -> 3+8*1+2=13 for seed 42
    assert_eq!(item.cost, 13);

    assert_eq!(next_random_pair(7), (7, 6));
    assert_eq!(next_random_pair(12), (12, 6));
    assert_eq!(next_random_pair(5), (5, 4));
    assert_eq!(next_random_pair(8), (8, 8));
}

// --------------------------------------------------------------------------
// 4. dungeonAllocateAndPlaceObject / trap allocation
// --------------------------------------------------------------------------
fn feature_is_floor(feature_id: i32) -> bool {
    feature_id <= i32::from(MAX_CAVE_FLOOR)
}

#[test]
fn dungeon_allocate_and_place_gold_fixed_seed() {
    reset_for_new_game(Some(12345));
    setup_dungeon(10, 10);
    fill_floor(TILE_CORR_FLOOR);
    with_state_mut(|s| {
        s.dg.current_level = 3;
        s.game.treasure.current_id = 1;
        s.py.pos = Coord_t { y: 0, x: 0 };
    });
    dungeon_allocate_and_place_object(feature_is_floor, 4, 1);
    let placed = with_state(|s| {
        for y in 1..9 {
            for x in 1..9 {
                if s.dg.floor[y][x].treasure_id != 0 {
                    return Some(Coord_t {
                        y: y as i32,
                        x: x as i32,
                    });
                }
            }
        }
        None
    });
    let placed = placed.expect("gold should be placed");
    assert_eq!(tile_at(placed).treasure_id, 1);
    assert_eq!(placed.y, 2);
    assert_eq!(placed.x, 3);
}

#[test]
fn dungeon_allocate_and_place_trap_fixed_seed() {
    reset_for_new_game(Some(99));
    setup_dungeon(12, 12);
    fill_floor(TILE_CORR_FLOOR);
    with_state_mut(|s| {
        s.game.treasure.current_id = 1;
        s.py.pos = Coord_t { y: 1, x: 1 };
    });
    dungeon_allocate_and_place_object(feature_is_floor, 1, 1);
    let coord = with_state(|s| {
        for y in 1..11 {
            for x in 1..11 {
                if s.dg.floor[y][x].treasure_id != 0 {
                    return Coord_t {
                        y: y as i32,
                        x: x as i32,
                    };
                }
            }
        }
        panic!("trap not placed");
    });
    assert_eq!(coord.y, 4);
    assert_eq!(coord.x, 9);
    let item = with_state(|s| {
        let tid = s.dg.floor[coord.y as usize][coord.x as usize].treasure_id;
        s.game.treasure.list[tid as usize]
    });
    assert_eq!(item.category_id, TV_INVIS_TRAP);
}

// --------------------------------------------------------------------------
// 5. dungeonPlaceRandomObjectNear retry loops (gold path)
// --------------------------------------------------------------------------
#[test]
fn dungeon_place_random_object_near_gold_path_seed1() {
    reset_for_new_game(Some(1));
    umoria::game_run::initialize_treasure_levels();
    setup_dungeon(20, 20);
    fill_floor(TILE_CORR_FLOOR);
    with_state_mut(|s| {
        s.dg.current_level = 1;
        s.game.treasure.current_id = 1;
    });
    let origin = Coord_t { y: 10, x: 10 };
    dungeon_place_random_object_near(origin, 1);
    // sets i=9 on success inside i<=10, so one post-success attempt may place a
    // second object. Seed 1 places gold at (12,7) then a second item.
    let at = Coord_t { y: 12, x: 7 };
    assert_eq!(tile_at(at).treasure_id, 1);
    assert_eq!(
        with_state(|s| s.game.treasure.list[1].category_id),
        umoria::treasure::TV_GOLD
    );
    assert_eq!(with_state(|s| s.game.treasure.list[1].cost), 13);
    assert!(with_state(|s| s.game.treasure.current_id) >= 2);
}

// --------------------------------------------------------------------------
// 6. Deletion helpers
// --------------------------------------------------------------------------
#[test]
fn dungeon_delete_object_restores_floor_and_frees_treasure() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    let coord = Coord_t { y: 5, x: 5 };
    with_state_mut(|s| {
        s.dg.floor[5][5] = Tile {
            feature_id: TILE_BLOCKED_FLOOR,
            treasure_id: 1,
            permanent_light: true,
            ..Default::default()
        };
        s.game.treasure.current_id = 2;
        s.game.treasure.list[1].category_id = TV_RUBBLE;
    });
    assert!(dungeon_delete_object(coord));
    assert_eq!(tile_at(coord).feature_id, TILE_CORR_FLOOR);
    assert_eq!(tile_at(coord).treasure_id, 0);
    assert!(!tile_at(coord).field_mark);
    assert_eq!(with_state(|s| s.game.treasure.current_id), 1);
}

#[test]
fn dungeon_delete_object_field_mark_only_returns_false() {
    // clears field_mark then returns caveTileVisible — so field_mark-only
    // visibility must yield false after delete.
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    let coord = Coord_t { y: 5, x: 5 };
    with_state_mut(|s| {
        s.dg.floor[5][5] = Tile {
            feature_id: TILE_CORR_FLOOR,
            treasure_id: 1,
            field_mark: true,
            permanent_light: false,
            temporary_light: false,
            ..Default::default()
        };
        s.game.treasure.current_id = 2;
        s.game.treasure.list[1].category_id = TV_RUBBLE;
    });
    assert!(!dungeon_delete_object(coord));
    assert!(!tile_at(coord).field_mark);
    assert_eq!(tile_at(coord).treasure_id, 0);
}

#[test]
fn dungeon_delete_monster_record_swap_and_compact() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    with_state_mut(|s| {
        s.next_free_monster_id = 4;
        s.monsters[2] = umoria::monster::Monster {
            hp: 10,
            creature_id: 5,
            pos: Coord_t { y: 3, x: 3 },
            lit: true,
            ..Default::default()
        };
        s.monsters[3] = umoria::monster::Monster {
            hp: 20,
            creature_id: 7,
            pos: Coord_t { y: 7, x: 7 },
            ..Default::default()
        };
        s.dg.floor[3][3].creature_id = 2;
        s.dg.floor[7][7].creature_id = 3;
    });
    dungeon_delete_monster_record(2);
    assert_eq!(with_state(|s| s.next_free_monster_id), 3);
    assert_eq!(with_state(|s| s.monsters[2].creature_id), 7);
    assert_eq!(with_state(|s| s.dg.floor[7][7].creature_id), 2);
    assert_eq!(with_state(|s| s.monsters[3].hp), 0);
    assert_eq!(with_state(|s| s.monsters[3].creature_id), 0);
}

#[test]
fn dungeon_remove_monster_from_level_clears_tile_and_hp() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    with_state_mut(|s| {
        s.monster_multiply_total = 2;
        s.monsters[2] = umoria::monster::Monster {
            hp: 50,
            pos: Coord_t { y: 4, x: 4 },
            lit: false,
            ..Default::default()
        };
        s.dg.floor[4][4].creature_id = 2;
    });
    dungeon_remove_monster_from_level(2);
    assert_eq!(with_state(|s| s.monsters[2].hp), -1);
    assert_eq!(with_state(|s| s.dg.floor[4][4].creature_id), 0);
    assert_eq!(with_state(|s| s.monster_multiply_total), 1);
}

#[test]
fn dungeon_delete_monster_composes_remove_and_record() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    with_state_mut(|s| {
        s.next_free_monster_id = 3;
        s.monsters[2] = umoria::monster::Monster {
            hp: 10,
            pos: Coord_t { y: 2, x: 2 },
            ..Default::default()
        };
        s.dg.floor[2][2].creature_id = 2;
    });
    dungeon_delete_monster(2);
    assert_eq!(with_state(|s| s.next_free_monster_id), 2);
    assert_eq!(with_state(|s| s.dg.floor[2][2].creature_id), 0);
}

// --------------------------------------------------------------------------
// 7. Integer-semantics: dungeonPlaceGold cost accumulation
// --------------------------------------------------------------------------
#[test]
fn dungeon_place_gold_cost_int32_truncation() {
    reset_for_new_game(Some(1));
    setup_dungeon(10, 10);
    with_state_mut(|s| {
        s.dg.current_level = 50;
        s.game.treasure.current_id = 1;
    });
    let coord = Coord_t { y: 5, x: 5 };
    dungeon_place_gold(coord);
    let (id, cost) = with_state(|s| (s.game.treasure.list[1].id, s.game.treasure.list[1].cost));
    assert_eq!(id, 410);
    assert_eq!(cost, 103);
}

#[test]
fn dungeon_summon_object_gold_on_open_floor_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    fill_floor(TILE_CORR_FLOOR);
    with_state_mut(|s| {
        s.dg.current_level = 2;
        s.game.treasure.current_id = 1;
    });
    let total = dungeon_summon_object(Coord_t { y: 10, x: 10 }, 1, 2);
    assert_eq!(total, 0);
    assert_eq!(tile_at(Coord_t { y: 9, x: 10 }).treasure_id, 1);
}

#[test]
fn cave_get_tile_symbol_blind_returns_space() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    with_state_mut(|s| {
        s.py.flags.status |= PY_BLIND;
        s.dg.floor[5][5].permanent_light = true;
        s.dg.floor[5][5].feature_id = TILE_LIGHT_FLOOR;
    });
    assert_eq!(cave_get_tile_symbol(Coord_t { y: 5, x: 5 }), b' ');
}

#[test]
fn cave_get_tile_symbol_player_at_tile() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    with_state_mut(|s| {
        s.py.running_tracker = 0;
        s.dg.floor[5][5].creature_id = 1;
        s.dg.floor[5][5].permanent_light = true;
    });
    assert_eq!(cave_get_tile_symbol(Coord_t { y: 5, x: 5 }), b'@');
}

#[test]
fn cave_get_tile_symbol_lit_monster_sprite() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    with_state_mut(|s| {
        s.dg.floor[5][5].creature_id = 2;
        s.dg.floor[5][5].permanent_light = true;
        s.monsters[2].lit = true;
        s.monsters[2].creature_id = 1;
    });
    assert_eq!(
        cave_get_tile_symbol(Coord_t { y: 5, x: 5 }),
        CREATURES_LIST[1].sprite
    );
}

#[test]
fn cave_get_tile_symbol_seam_highlight_percent() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    with_state_mut(|s| {
        s.options.highlight_seams = true;
        s.dg.floor[5][5].permanent_light = true;
        s.dg.floor[5][5].feature_id = TILE_MAGMA_WALL;
    });
    assert_eq!(cave_get_tile_symbol(Coord_t { y: 5, x: 5 }), b'%');
}

#[test]
fn cave_get_tile_symbol_granite_hash() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    with_state_mut(|s| {
        s.dg.floor[5][5].permanent_light = true;
        s.dg.floor[5][5].feature_id = TILE_GRANITE_WALL;
    });
    assert_eq!(cave_get_tile_symbol(Coord_t { y: 5, x: 5 }), b'#');
}

// Golden manifest sanity — harness available for RNG draws.
#[test]
fn golden_rng_manifest_loads_for_dungeon_tests() {
    let root = golden_root();
    assert!(root.join("manifest.json").is_file());
}
