//! Phase 4.7.2 — area/line light & darken spells parity.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::config::dungeon::objects::{OBJ_CLOSED_DOOR, OBJ_SECRET_DOOR, OBJ_TRAP_LIST};
use umoria::config::monsters::defense::CD_LIGHT;
use umoria::config::treasure::chests::{CH_LOCKED, CH_TRAPPED};
use umoria::config::treasure::OBJECT_BOLTS_MAX_RANGE;
use umoria::data_creatures::CREATURES_LIST;
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH, SCREEN_HEIGHT, SCREEN_WIDTH};
use umoria::dungeon_tile::{
    Tile, MAX_CAVE_FLOOR, MAX_OPEN_SPACE, MIN_CLOSED_SPACE, TILE_CORR_FLOOR, TILE_DARK_FLOOR,
    TILE_GRANITE_WALL, TILE_LIGHT_FLOOR,
};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::game_objects::popt;
use umoria::identification::SpecialNameIds;
use umoria::inventory::{inventory_item_copy_to, Inventory};
use umoria::monster::Monster;
use umoria::player_move::player_move_position;
use umoria::spells::{
    spell_darken_area, spell_disarm_all_in_direction, spell_light_area, spell_light_line,
    spell_starlite,
};
use umoria::treasure::{TV_CHEST, TV_CLOSED_DOOR};
use umoria::types::Coord_t;
use umoria::ui::panel_bounds_fields;
use umoria::ui_io::test_set_ncurses_stub;

const FLOATING_EYE_ID: u16 = 18;

fn setup_dungeon(height: i16, width: i16) {
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
    });
}

fn setup_player_panel(row: i32, col: i32, pos: Coord_t) {
    test_set_ncurses_stub(true);
    let bounds = panel_bounds_fields(row, col);
    with_state_mut(|s| {
        s.py.pos = pos;
        s.dg.panel.row = row;
        s.dg.panel.col = col;
        s.dg.panel.top = bounds.top;
        s.dg.panel.bottom = bounds.bottom;
        s.dg.panel.left = bounds.left;
        s.dg.panel.right = bounds.right;
        s.dg.panel.row_prt = bounds.row_prt;
        s.dg.panel.col_prt = bounds.col_prt;
        s.py.flags.blind = 0;
        s.dg.current_level = 10;
        s.game.treasure.current_id = 1;
    });
}

fn set_tile(coord: Coord_t, tile: Tile) {
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize] = tile;
    });
}

fn tile_at(coord: Coord_t) -> Tile {
    with_state(|s| s.dg.floor[coord.y as usize][coord.x as usize])
}

fn fill_corridor(start: Coord_t, direction: i32, length: i32, feature_id: u8) {
    let mut coord = start;
    for _ in 0..length {
        set_tile(
            coord,
            Tile {
                feature_id,
                ..Tile::default()
            },
        );
        let _ = player_move_position(direction, &mut coord);
    }
}

fn place_monster(id: i32, creature_id: u16, hp: i16, coord: Coord_t) {
    with_state_mut(|s| {
        s.monsters[id as usize] = Monster {
            hp,
            creature_id,
            pos: coord,
            ..Default::default()
        };
        s.dg.floor[coord.y as usize][coord.x as usize].creature_id = id as u8;
        if id + 1 >= i32::from(s.next_free_monster_id) {
            s.next_free_monster_id = (id + 1) as i16;
        }
    });
}

fn place_treasure_at(coord: Coord_t, obj_id: u16) {
    let tid = popt() as u8;
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize].treasure_id = tid;
        inventory_item_copy_to(obj_id as i16, &mut s.game.treasure.list[tid as usize]);
    });
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

/// Independent C++ reference for spellLightLine tile count (spells.cpp lines 611–654).
fn reference_light_line_tile_count(start: Coord_t, direction: i32) -> usize {
    let mut coord = start;
    let mut distance = 0;
    let mut count = 0;
    loop {
        let tile = tile_at(coord);
        if distance > i32::from(OBJECT_BOLTS_MAX_RANGE) || tile.feature_id >= MIN_CLOSED_SPACE {
            break;
        }
        count += 1;
        let _ = player_move_position(direction, &mut coord);
        distance += 1;
    }
    count
}

/// Independent C++ reference for spellLightLine lit-tile sequence before casting.
fn reference_light_line_tiles(start: Coord_t, direction: i32) -> Vec<Coord_t> {
    let mut coord = start;
    let mut distance = 0;
    let mut lit = Vec::new();
    loop {
        let tile = tile_at(coord);
        if distance > i32::from(OBJECT_BOLTS_MAX_RANGE) || tile.feature_id >= MIN_CLOSED_SPACE {
            break;
        }
        if !tile.permanent_light && !tile.temporary_light {
            lit.push(coord);
        }
        let _ = player_move_position(direction, &mut coord);
        distance += 1;
    }
    lit
}

// ---------------------------------------------------------------------------
// 1. RNG-order golden — light line / starlite monster reaction
// ---------------------------------------------------------------------------

#[test]
fn spell_light_line_no_monster_consumes_zero_rng() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    fill_corridor(Coord_t { y: 10, x: 10 }, 2, 6, TILE_CORR_FLOOR);

    let baseline = {
        reset_for_new_game(Some(42));
        setup_dungeon(40, 40);
        setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
        fill_corridor(Coord_t { y: 10, x: 10 }, 2, 6, TILE_CORR_FLOOR);
        next_random_pair(8)
    };

    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    fill_corridor(Coord_t { y: 10, x: 10 }, 2, 6, TILE_CORR_FLOOR);
    spell_light_line(Coord_t { y: 10, x: 10 }, 2);

    assert_eq!(next_random_pair(8), baseline);
}

#[test]
fn spell_light_line_light_sensitive_monster_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    fill_corridor(Coord_t { y: 10, x: 10 }, 2, 6, TILE_CORR_FLOOR);
    place_monster(2, FLOATING_EYE_ID, 500, Coord_t { y: 12, x: 10 });

    spell_light_line(Coord_t { y: 10, x: 10 }, 2);

    assert_eq!(next_random_pair(8), (8, 4));
    assert_eq!(next_random_pair(8), (8, 2));
}

#[test]
fn spell_starlite_touches_one_monster_one_dice_roll_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player_panel(0, 0, Coord_t { y: 20, x: 20 });
    for y in 18..=22 {
        for x in 18..=22 {
            set_tile(
                Coord_t { y, x },
                Tile {
                    feature_id: TILE_CORR_FLOOR,
                    ..Tile::default()
                },
            );
        }
    }
    place_monster(2, FLOATING_EYE_ID, 500, Coord_t { y: 21, x: 20 });

    spell_starlite(Coord_t { y: 20, x: 20 });

    // South line (dir 2) reaches the eye; other directions do not.
    assert_eq!(next_random_pair(8), (8, 4));
    assert_eq!(next_random_pair(8), (8, 2));
}

// ---------------------------------------------------------------------------
// 2. Lighting extent parity — spellLightArea / spellDarkenArea
// ---------------------------------------------------------------------------

#[test]
fn spell_light_area_corridor_sets_three_by_three_permanent_light() {
    reset_for_new_game(Some(1));
    setup_dungeon(30, 30);
    setup_player_panel(0, 0, Coord_t { y: 15, x: 15 });
    let center = Coord_t { y: 15, x: 15 };
    set_tile(
        center,
        Tile {
            feature_id: TILE_CORR_FLOOR,
            ..Tile::default()
        },
    );

    assert!(spell_light_area(center));

    for dy in -1..=1 {
        for dx in -1..=1 {
            let spot = Coord_t {
                y: center.y + dy,
                x: center.x + dx,
            };
            assert!(
                tile_at(spot).permanent_light,
                "expected permanent light at ({},{})",
                spot.y,
                spot.x
            );
        }
    }
}

#[test]
fn spell_light_area_room_calls_dungeon_light_room() {
    reset_for_new_game(Some(1));
    setup_dungeon(66, 66);
    setup_player_panel(0, 0, Coord_t { y: 15, x: 15 });
    with_state_mut(|s| s.dg.current_level = 5);

    let room_top = 12;
    let room_left = 34;
    for y in room_top..=room_top + 10 {
        for x in room_left..=room_left + 32 {
            set_tile(
                Coord_t { y, x },
                Tile {
                    feature_id: TILE_DARK_FLOOR,
                    perma_lit_room: true,
                    ..Tile::default()
                },
            );
        }
    }

    let center = Coord_t {
        y: room_top + 5,
        x: room_left + 16,
    };
    assert!(spell_light_area(center));

    let sample = Coord_t {
        y: room_top + 2,
        x: room_left + 2,
    };
    let tile = tile_at(sample);
    assert!(tile.permanent_light);
    assert_eq!(tile.feature_id, TILE_LIGHT_FLOOR);
}

#[test]
fn spell_darken_area_corridor_clears_permanent_light() {
    reset_for_new_game(Some(1));
    setup_dungeon(30, 30);
    setup_player_panel(0, 0, Coord_t { y: 15, x: 15 });
    let center = Coord_t { y: 15, x: 15 };
    for dy in -1..=1 {
        for dx in -1..=1 {
            set_tile(
                Coord_t {
                    y: center.y + dy,
                    x: center.x + dx,
                },
                Tile {
                    feature_id: TILE_CORR_FLOOR,
                    permanent_light: true,
                    ..Tile::default()
                },
            );
        }
    }

    assert!(spell_darken_area(center));

    for dy in -1..=1 {
        for dx in -1..=1 {
            let spot = Coord_t {
                y: center.y + dy,
                x: center.x + dx,
            };
            assert!(!tile_at(spot).permanent_light);
        }
    }
}

#[test]
fn spell_darken_area_room_integer_bounds_and_darkened_flag() {
    reset_for_new_game(Some(1));
    setup_dungeon(66, 66);
    setup_player_panel(0, 0, Coord_t { y: 15, x: 40 });
    with_state_mut(|s| s.dg.current_level = 5);

    let half_height = i32::from(SCREEN_HEIGHT / 2);
    let half_width = i32::from(SCREEN_WIDTH / 2);
    let start_row = (15 / half_height) * half_height + 1;
    let start_col = (40 / half_width) * half_width + 1;
    let end_row = start_row + half_height - 1;
    let end_col = start_col + half_width - 1;

    for y in start_row..=end_row {
        for x in start_col..=end_col {
            set_tile(
                Coord_t { y, x },
                Tile {
                    feature_id: TILE_LIGHT_FLOOR,
                    perma_lit_room: true,
                    permanent_light: true,
                    ..Tile::default()
                },
            );
        }
    }
    set_tile(
        Coord_t { y: 15, x: 40 },
        Tile {
            feature_id: TILE_LIGHT_FLOOR,
            perma_lit_room: true,
            permanent_light: true,
            ..Tile::default()
        },
    );

    let darkened = spell_darken_area(Coord_t { y: 15, x: 40 });
    assert!(darkened);

    for y in start_row..=end_row {
        for x in start_col..=end_col {
            let tile = tile_at(Coord_t { y, x });
            if tile.perma_lit_room && tile.feature_id <= MAX_CAVE_FLOOR {
                assert!(!tile.permanent_light);
                assert_eq!(tile.feature_id, TILE_DARK_FLOOR);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Line traversal — spellLightLine stops at wall
// ---------------------------------------------------------------------------

#[test]
fn spell_light_line_stops_at_wall_same_as_cpp_reference() {
    reset_for_new_game(Some(1));
    setup_dungeon(40, 40);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    let start = Coord_t { y: 10, x: 10 };
    fill_corridor(start, 2, 4, TILE_CORR_FLOOR);
    set_tile(
        Coord_t { y: 14, x: 10 },
        Tile {
            feature_id: TILE_GRANITE_WALL,
            ..Tile::default()
        },
    );

    spell_light_line(start, 2);

    let expected = reference_light_line_tiles(start, 2);
    for coord in expected {
        assert!(
            tile_at(coord).permanent_light,
            "tile ({},{}) should be lit",
            coord.y,
            coord.x
        );
    }
    assert!(!tile_at(Coord_t { y: 14, x: 10 }).permanent_light);
}

#[test]
fn spell_light_line_respects_bolt_max_range() {
    reset_for_new_game(Some(1));
    setup_dungeon(60, 60);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    let start = Coord_t { y: 10, x: 10 };
    fill_corridor(start, 2, 25, TILE_CORR_FLOOR);

    spell_light_line(start, 2);

    let expected = reference_light_line_tile_count(start, 2);
    assert_eq!(expected, i32::from(OBJECT_BOLTS_MAX_RANGE) as usize + 1);
}

// ---------------------------------------------------------------------------
// 4. spellDisarmAllInDirection — traps/doors, zero RNG
// ---------------------------------------------------------------------------

#[test]
fn spell_disarm_all_in_direction_trap_and_zero_rng() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    fill_corridor(Coord_t { y: 10, x: 10 }, 2, 4, TILE_CORR_FLOOR);
    let trap_coord = Coord_t { y: 12, x: 10 };
    set_tile(
        trap_coord,
        Tile {
            feature_id: TILE_CORR_FLOOR,
            ..Tile::default()
        },
    );
    place_treasure_at(trap_coord, OBJ_TRAP_LIST);

    let baseline = {
        reset_for_new_game(Some(42));
        setup_dungeon(40, 40);
        setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
        fill_corridor(Coord_t { y: 10, x: 10 }, 2, 4, TILE_CORR_FLOOR);
        next_random_pair(100)
    };

    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    fill_corridor(Coord_t { y: 10, x: 10 }, 2, 4, TILE_CORR_FLOOR);
    let trap_coord = Coord_t { y: 12, x: 10 };
    set_tile(
        trap_coord,
        Tile {
            feature_id: TILE_CORR_FLOOR,
            permanent_light: true,
            ..Tile::default()
        },
    );
    place_treasure_at(trap_coord, OBJ_TRAP_LIST);

    assert!(spell_disarm_all_in_direction(Coord_t { y: 10, x: 10 }, 2));
    assert_eq!(tile_at(trap_coord).treasure_id, 0);
    assert_eq!(next_random_pair(100), baseline);
}

#[test]
fn spell_disarm_all_in_direction_closed_door_clears_misc_use() {
    reset_for_new_game(Some(1));
    setup_dungeon(40, 40);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    fill_corridor(Coord_t { y: 10, x: 10 }, 2, 3, TILE_CORR_FLOOR);
    let door = Coord_t { y: 12, x: 10 };
    set_tile(
        door,
        Tile {
            feature_id: TILE_CORR_FLOOR,
            ..Tile::default()
        },
    );
    place_treasure_at(door, OBJ_CLOSED_DOOR);
    with_state_mut(|s| {
        let tid = s.dg.floor[door.y as usize][door.x as usize].treasure_id;
        s.game.treasure.list[tid as usize].misc_use = 7;
    });

    let _ = spell_disarm_all_in_direction(Coord_t { y: 10, x: 10 }, 2);

    with_state(|s| {
        let tid = s.dg.floor[door.y as usize][door.x as usize].treasure_id;
        assert_eq!(s.game.treasure.list[tid as usize].misc_use, 0);
        assert_eq!(
            s.game.treasure.list[tid as usize].category_id,
            TV_CLOSED_DOOR
        );
    });
}

#[test]
fn spell_disarm_all_in_direction_secret_door_marks_field() {
    reset_for_new_game(Some(1));
    setup_dungeon(40, 40);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    fill_corridor(Coord_t { y: 10, x: 10 }, 2, 3, TILE_CORR_FLOOR);
    let door = Coord_t { y: 12, x: 10 };
    set_tile(
        door,
        Tile {
            feature_id: MAX_OPEN_SPACE,
            ..Tile::default()
        },
    );
    place_treasure_at(door, OBJ_SECRET_DOOR);

    assert!(spell_disarm_all_in_direction(Coord_t { y: 10, x: 10 }, 2));
    assert!(tile_at(door).field_mark);
    with_state(|s| {
        let tid = s.dg.floor[door.y as usize][door.x as usize].treasure_id;
        assert_eq!(
            s.game.treasure.list[tid as usize].category_id,
            TV_CLOSED_DOOR
        );
    });
}

#[test]
fn spell_disarm_all_in_direction_trapped_chest() {
    reset_for_new_game(Some(1));
    setup_dungeon(40, 40);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    fill_corridor(Coord_t { y: 10, x: 10 }, 2, 3, TILE_CORR_FLOOR);
    let chest = Coord_t { y: 12, x: 10 };
    set_tile(
        chest,
        Tile {
            feature_id: TILE_CORR_FLOOR,
            ..Tile::default()
        },
    );
    let tid = popt() as u8;
    with_state_mut(|s| {
        s.dg.floor[chest.y as usize][chest.x as usize].treasure_id = tid;
        s.game.treasure.list[tid as usize] = Inventory {
            category_id: TV_CHEST,
            flags: CH_TRAPPED | CH_LOCKED,
            ..Inventory::default()
        };
    });

    assert!(spell_disarm_all_in_direction(Coord_t { y: 10, x: 10 }, 2));
    with_state(|s| {
        let tid = s.dg.floor[chest.y as usize][chest.x as usize].treasure_id;
        let item = &s.game.treasure.list[tid as usize];
        assert_eq!(item.flags & (CH_TRAPPED | CH_LOCKED), 0);
        assert_eq!(item.special_name_id, SpecialNameIds::SN_UNLOCKED as u8);
    });
}

// ---------------------------------------------------------------------------
// 5. Light-sensitive defense bit on creature table
// ---------------------------------------------------------------------------

#[test]
fn floating_eye_has_cd_light_defense() {
    let creature = &CREATURES_LIST[FLOATING_EYE_ID as usize];
    assert_ne!(creature.defenses & CD_LIGHT, 0);
}
