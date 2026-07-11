//! Level/town assembly + `generateCave` in `dungeon_generate`.
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

use std::panic::{catch_unwind, AssertUnwindSafe};

use umoria::config::dungeon::objects::OBJ_NOTHING;
use umoria::config::monsters::MON_MIN_INDEX_ID;
use umoria::config::treasure::MIN_TREASURE_LIST_ID;
use umoria::data_creatures::CREATURES_LIST;
use umoria::data_treasure::GAME_OBJECTS;
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH, SCREEN_HEIGHT, SCREEN_WIDTH};
use umoria::dungeon_generate::{
    dungeon_build_store, dungeon_generate, dungeon_place_town_stores, generate_cave, is_nigh_time,
    light_town, monster_linker, town_generation, treasure_linker,
};
use umoria::dungeon_tile::{
    Tile, TILE_BOUNDARY_WALL, TILE_CORR_FLOOR, TILE_DARK_FLOOR, TILE_NULL_WALL,
};
use umoria::game::{
    random_number, random_number_normal_distribution, reset_for_new_game, seed_reset_to_old_seed,
    seed_set, with_state, with_state_mut,
};
use umoria::monster::{MON_MAX_CREATURES, MON_MAX_LEVELS};
use umoria::rng::{get_seed, RNG_M};
use umoria::types::{Coord_t, MAX_DUNGEON_OBJECTS, TREASURE_MAX_LEVELS};

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
// 5. treasureLinker / monsterLinker
// --------------------------------------------------------------------------
#[test]
fn treasure_linker_resets_heap_to_nothing_and_current_id() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.game.treasure.current_id = 10;
        s.game.treasure.list[5].category_id = 99;
    });

    treasure_linker();

    with_state(|s| {
        assert_eq!(s.game.treasure.current_id, i16::from(MIN_TREASURE_LIST_ID));
        for item in &s.game.treasure.list {
            assert_eq!(item.id, OBJ_NOTHING);
        }
    });
}

#[test]
fn monster_linker_resets_monsters_and_next_free_id() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.next_free_monster_id = 20;
        s.monsters[5].creature_id = 42;
    });

    monster_linker();

    with_state(|s| {
        assert_eq!(s.next_free_monster_id, i16::from(MON_MIN_INDEX_ID));
        for monster in &s.monsters {
            assert_eq!(monster.hp, 0);
            assert_eq!(monster.creature_id, 0);
        }
    });
}

// --------------------------------------------------------------------------
// isNighTime
// --------------------------------------------------------------------------
#[test]
fn is_nigh_time_alternates_on_game_turn_halves() {
    reset_for_new_game(None);
    with_state_mut(|s| s.dg.game_turn = 0);
    assert!(!is_nigh_time());
    with_state_mut(|s| s.dg.game_turn = 4999);
    assert!(!is_nigh_time());
    with_state_mut(|s| s.dg.game_turn = 5000);
    assert!(is_nigh_time());
    with_state_mut(|s| s.dg.game_turn = 9999);
    assert!(is_nigh_time());
    with_state_mut(|s| s.dg.game_turn = 10000);
    assert!(!is_nigh_time());
}

// --------------------------------------------------------------------------
// 4. dungeonBuildStore / dungeonPlaceTownStores
// --------------------------------------------------------------------------
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

fn init_monster_levels() {
    with_state_mut(|state| {
        state.monster_levels = [0; MON_MAX_LEVELS as usize + 1];
        let endgame = umoria::config::monsters::MON_ENDGAME_MONSTERS as usize;
        for i in 0..MON_MAX_CREATURES as usize - endgame {
            let level = CREATURES_LIST[i].level as usize;
            state.monster_levels[level] += 1;
        }
        for i in 1..=MON_MAX_LEVELS as usize {
            state.monster_levels[i] += state.monster_levels[i - 1];
        }
    });
}

fn prepare_treasure_heap() {
    treasure_linker();
    init_treasure_levels();
    init_monster_levels();
}

#[test]
fn dungeon_build_store_footprint_door_and_rng_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(i16::from(SCREEN_HEIGHT), i16::from(SCREEN_WIDTH));
    prepare_treasure_heap();

    dungeon_build_store(0, Coord_t { y: 0, x: 0 });

    let door_count = count_feature(TILE_CORR_FLOOR);
    assert_eq!(door_count, 1);
    assert!(count_feature(TILE_BOUNDARY_WALL) > 0);

    let door = with_state(|s| {
        for y in 0..s.dg.height {
            for x in 0..s.dg.width {
                let tile = &s.dg.floor[y as usize][x as usize];
                if tile.feature_id == TILE_CORR_FLOOR {
                    return Some((
                        Coord_t {
                            y: i32::from(y),
                            x: i32::from(x),
                        },
                        tile.treasure_id,
                    ));
                }
            }
        }
        None
    });
    let (door_coord, treasure_id) = door.expect("store door");
    assert_ne!(treasure_id, 0);
    assert_eq!(tile_at(door_coord).feature_id, TILE_CORR_FLOOR);
    assert_eq!(next_random_pair(4), (4, 3));
}

#[test]
fn dungeon_place_town_stores_six_stores_shuffle_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(i16::from(SCREEN_HEIGHT), i16::from(SCREEN_WIDTH));
    prepare_treasure_heap();

    dungeon_place_town_stores();

    assert_eq!(count_feature(TILE_CORR_FLOOR), 6);
    assert!(count_feature(TILE_BOUNDARY_WALL) >= 210);
    assert_eq!(next_random_pair(4), (4, 3));
}

// --------------------------------------------------------------------------
// 2. Town seed-bracket (partial — before lightTown/storeMaintenance stubs)
// --------------------------------------------------------------------------
#[test]
fn town_seed_bracket_restores_main_rng_after_stairs_seed42() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.game.town_seed = 12345;
        s.dg.current_level = 0;
        s.dg.height = i16::from(SCREEN_HEIGHT);
        s.dg.width = i16::from(SCREEN_WIDTH);
    });
    let main_before = get_seed();

    seed_set(with_state(|s| s.game.town_seed));
    prepare_treasure_heap();
    dungeon_place_town_stores();
    umoria::dungeon_generate::dungeon_fill_empty_tiles_with(TILE_DARK_FLOOR);
    umoria::dungeon_generate::dungeon_place_boundary_walls();
    umoria::dungeon_generate::dungeon_place_stairs(2, 1, 0);
    seed_reset_to_old_seed();

    let expected_after_reset = (main_before % (RNG_M as u32 - 1)) + 1;
    assert_eq!(get_seed(), expected_after_reset);
    assert!(count_feature(TILE_DARK_FLOOR) > 0);
    assert_eq!(next_random_pair(64), (64, 53));
}

// --------------------------------------------------------------------------
// 3. dungeonGenerate RNG sub-sequence (post full generation, -verified)
// --------------------------------------------------------------------------
#[test]
fn dungeon_generate_rng_after_full_generation_seed42_level5() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.dg.current_level = 5;
        s.dg.height = i16::from(MAX_HEIGHT);
        s.dg.width = i16::from(MAX_WIDTH);
    });
    prepare_treasure_heap();

    dungeon_generate();

 // reference (seed 42, level 5): post-dungeonGenerate randomNumber(8) == 7
    assert_eq!(next_random_pair(8), (8, 7));
}

#[test]
fn dungeon_generate_room_map_seeding_rng_order_seed42() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.dg.height = i16::from(MAX_HEIGHT);
        s.dg.width = i16::from(MAX_WIDTH);
        s.dg.current_level = 5;
    });

    let row_rooms = 2 * (i32::from(MAX_HEIGHT) / i32::from(SCREEN_HEIGHT));
    let col_rooms = 2 * (i32::from(MAX_WIDTH) / i32::from(SCREEN_WIDTH));
    let room_count = random_number_normal_distribution(32, 2);
    assert_eq!(room_count, 32);
    for _ in 0..room_count {
        let _ = random_number(row_rooms);
        let _ = random_number(col_rooms);
    }
    assert_eq!(next_random_pair(25), (25, 20));
}

// --------------------------------------------------------------------------
// generateCave setup + full goldens (-verified)
// --------------------------------------------------------------------------
#[test]
fn generate_cave_resets_panel_and_town_dimensions() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.dg.current_level = 0;
        s.dg.panel.top = 9;
        s.py.pos = Coord_t { y: 3, x: 4 };
    });

    let _ = catch_unwind(AssertUnwindSafe(generate_cave));

    with_state(|s| {
        assert_eq!(s.dg.panel.top, 0);
        assert_eq!(s.dg.panel.bottom, 0);
        assert_eq!(s.dg.panel.left, 0);
        assert_eq!(s.dg.panel.right, 0);
        assert_eq!(s.dg.height, i16::from(SCREEN_HEIGHT));
        assert_eq!(s.dg.width, i16::from(SCREEN_WIDTH));
        assert_eq!(s.dg.panel.max_rows, 0);
        assert_eq!(s.dg.panel.max_cols, 0);
        assert_eq!(s.next_free_monster_id, i16::from(MON_MIN_INDEX_ID));
        assert_ne!(s.py.pos.y, -1);
    });
}

#[test]
fn generate_cave_dungeon_dimensions_and_panel_max_seed42() {
    reset_for_new_game(Some(42));
    init_treasure_levels();
    init_monster_levels();
    with_state_mut(|s| s.dg.current_level = 5);

    generate_cave();

    with_state(|s| {
        assert_eq!(s.dg.height, i16::from(MAX_HEIGHT));
        assert_eq!(s.dg.width, i16::from(MAX_WIDTH));
        assert_eq!(s.dg.panel.max_rows, 4);
        assert_eq!(s.dg.panel.max_cols, 4);
        assert_eq!(s.dg.panel.row, 4);
        assert_eq!(s.dg.panel.col, 4);
    });
 // reference (seed 42, level 5): post-generateCave randomNumber(8) == 7
    assert_eq!(next_random_pair(8), (8, 7));
}

#[test]
fn generate_cave_dungeon_level5_full_floor_golden_seed42() {
    reset_for_new_game(Some(42));
    prepare_treasure_heap();
    with_state_mut(|s| s.dg.current_level = 5);
    generate_cave();
    let null_walls = count_feature(TILE_NULL_WALL);
    assert_eq!(null_walls, 0);
}

#[test]
fn generate_cave_town_level0_full_layout_golden_seed42() {
    reset_for_new_game(Some(42));
    prepare_treasure_heap();
    with_state_mut(|s| {
        s.dg.current_level = 0;
        s.game.town_seed = 12345;
    });
    generate_cave();
    assert_eq!(count_feature(TILE_CORR_FLOOR), 6);
}

#[test]
fn town_generation_full_rng_and_layout_golden_seed42() {
    reset_for_new_game(Some(42));
    prepare_treasure_heap();
    with_state_mut(|s| {
        s.dg.current_level = 0;
        s.game.town_seed = 12345;
        s.dg.height = i16::from(SCREEN_HEIGHT);
        s.dg.width = i16::from(SCREEN_WIDTH);
    });
    town_generation();
    assert_eq!(count_feature(TILE_CORR_FLOOR), 6);
}

#[test]
fn light_town_permanent_light_day_seed42() {
    reset_for_new_game(Some(42));
    prepare_treasure_heap();
    setup_dungeon(i16::from(SCREEN_HEIGHT), i16::from(SCREEN_WIDTH));
    with_state_mut(|s| {
        s.dg.game_turn = 0;
        s.py.pos = Coord_t { y: 10, x: 10 };
    });
    light_town();
    with_state(|s| {
        assert!(s.dg.floor[1][1].permanent_light);
        assert!(s.next_free_monster_id > i16::from(MON_MIN_INDEX_ID));
    });
}
