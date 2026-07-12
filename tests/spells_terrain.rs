//! Terrain & wall spells (`spells`) tests.
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

use umoria::config::dungeon::objects::OBJ_RUBBLE;
use umoria::config::monsters::defense::CD_STONE;
use umoria::config::treasure::chests::{CH_LOCKED, CH_TRAPPED};
use umoria::config::treasure::OBJECT_BOLTS_MAX_RANGE;
use umoria::dungeon::{coord_distance_between, coord_in_bounds, MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{
    Tile, MAX_CAVE_FLOOR, MIN_CAVE_WALL, MIN_CLOSED_SPACE, TILE_CORR_FLOOR, TILE_GRANITE_WALL,
    TILE_MAGMA_WALL, TILE_QUARTZ_WALL,
};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::game_objects::popt;
use umoria::identification::SpecialNameIds;
use umoria::inventory::{inventory_item_copy_to, Inventory};
use umoria::monster::Monster;
use umoria::player_move::player_move_position;
use umoria::spells::{
    spell_build_wall, spell_destroy_area, spell_destroy_doors_traps_in_direction, spell_earthquake,
    spell_wall_to_mud, spell_warding_glyph,
};
use umoria::treasure::{TV_CHEST, TV_VIS_TRAP};
use umoria::types::Coord_t;
use umoria::ui::panel_bounds_fields;
use umoria::ui_io::test_set_ncurses_stub;

const URCHIN_ID: u16 = 0;
const STONE_GOLEM_ID: u16 = 167;
const EARTH_ELEMENTAL_ID: u16 = 243;

fn setup_dungeon(height: i16, width: i16) {
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                s.dg.floor[y as usize][x as usize].feature_id = TILE_CORR_FLOOR;
            }
        }
    });
}

fn setup_player(pos: Coord_t, level: u16) {
    test_set_ncurses_stub(true);
    let bounds = panel_bounds_fields(0, 0);
    with_state_mut(|s| {
        s.py.pos = pos;
        s.py.misc.level = level;
        s.dg.panel.row = 0;
        s.dg.panel.col = 0;
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

fn seal_directional_ray(start: Coord_t, direction: i32) {
    let mut coord = start;
    for _ in 0..=i32::from(OBJECT_BOLTS_MAX_RANGE) {
        let _ = player_move_position(direction, &mut coord);
    }
    set_tile(
        coord,
        Tile {
            feature_id: TILE_GRANITE_WALL,
            ..Tile::default()
        },
    );
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

/// Mirror earthquake RNG consumption for the current dungeon fixture.
fn reference_earthquake_rng_after(player_pos: Coord_t) -> (i32, i32) {
    for coord_y in player_pos.y - 8..=player_pos.y + 8 {
        for coord_x in player_pos.x - 8..=player_pos.x + 8 {
            let coord = Coord_t {
                y: coord_y,
                x: coord_x,
            };
            // Keep nested `if`s (not `&&`-collapsed): floor RNG must run whenever the
            // 1-in-8 roll hits, even when `creature_id <= 1`. Collapsing would only be
            // safe if the floor check stayed outside the creature branch.
            #[allow(
                clippy::collapsible_if,
                reason = "nested structure mirrors earthquake RNG; floor roll is sibling of creature branch"
            )]
            if (coord.y != player_pos.y || coord.x != player_pos.x) && coord_in_bounds(coord) {
                if random_number(8) == 1 {
                    let tile = tile_at(coord);
                    if tile.creature_id > 1 {
                        let creature_id =
                            with_state(|s| s.monsters[tile.creature_id as usize].creature_id);
                        let creature =
                            &umoria::data_creatures::CREATURES_LIST[creature_id as usize];
                        if (creature.movement & umoria::config::monsters::move_flags::CM_PHASE) == 0
                            || creature.sprite == b'E'
                            || creature.sprite == b'X'
                        {
                            for _ in 0..4 {
                                random_number(8);
                            }
                        }
                    }
                    if i32::from(tile.feature_id) <= i32::from(MAX_CAVE_FLOOR) {
                        random_number(10);
                    }
                }
            }
        }
    }
    next_random_pair(8)
}

/// Mirror destroy-area RNG consumption for the current dungeon fixture.
fn reference_destroy_area_rng_after(center: Coord_t) -> ((i32, i32), i16) {
    if with_state(|s| s.dg.current_level) > 0 {
        for spot_y in center.y - 15..=center.y + 15 {
            for spot_x in center.x - 15..=center.x + 15 {
                let spot = Coord_t {
                    y: spot_y,
                    x: spot_x,
                };
                if coord_in_bounds(spot)
                    && tile_at(spot).feature_id != umoria::dungeon_tile::TILE_BOUNDARY_WALL
                {
                    let distance = coord_distance_between(spot, center);
                    if distance == 0 {
                    } else if distance < 13 {
                        random_number(6);
                    } else if distance < 16 {
                        random_number(9);
                    }
                }
            }
        }
    }
    let blind = (10 + random_number(10)) as i16;
    (next_random_pair(8), blind)
}

// --------------------------------------------------------------------------
// 1. RNG-order golden — earthquake
// --------------------------------------------------------------------------

#[test]
fn spell_earthquake_rng_order_matches_scan_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(66, 66);
    setup_player(Coord_t { y: 33, x: 33 }, 10);

    let expected = {
        reset_for_new_game(Some(42));
        setup_dungeon(66, 66);
        setup_player(Coord_t { y: 33, x: 33 }, 10);
        reference_earthquake_rng_after(Coord_t { y: 33, x: 33 })
    };

    reset_for_new_game(Some(42));
    setup_dungeon(66, 66);
    setup_player(Coord_t { y: 33, x: 33 }, 10);
    spell_earthquake();

    assert_eq!(next_random_pair(8), expected);
}

#[test]
fn spell_earthquake_affected_floor_becomes_wall_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(66, 66);
    setup_player(Coord_t { y: 33, x: 33 }, 10);

    spell_earthquake();

    let mut affected = 0;
    for dy in -8..=8 {
        for dx in -8..=8 {
            if dy == 0 && dx == 0 {
                continue;
            }
            let coord = Coord_t {
                y: 33 + dy,
                x: 33 + dx,
            };
            let tile = tile_at(coord);
            if tile.feature_id >= MIN_CAVE_WALL {
                affected += 1;
                assert!(
                    tile.feature_id == TILE_QUARTZ_WALL
                        || tile.feature_id == TILE_MAGMA_WALL
                        || tile.feature_id == TILE_GRANITE_WALL
                );
            }
        }
    }
    assert!(affected > 0);
}

// --------------------------------------------------------------------------
// 2. RNG-order golden — destroy area
// --------------------------------------------------------------------------

#[test]
fn spell_destroy_area_rng_order_and_blind_seed42() {
    let center = Coord_t { y: 33, x: 33 };

    let (expected_next, expected_blind) = {
        reset_for_new_game(Some(42));
        setup_dungeon(66, 66);
        setup_player(center, 10);
        reference_destroy_area_rng_after(center)
    };

    reset_for_new_game(Some(42));
    setup_dungeon(66, 66);
    setup_player(center, 10);
    spell_destroy_area(center);

    with_state(|s| assert_eq!(s.py.flags.blind, expected_blind));
    assert_eq!(next_random_pair(8), expected_next);
    assert!((11..=20).contains(&expected_blind));
}

#[test]
fn spell_destroy_area_skips_grid_when_current_level_zero() {
    reset_for_new_game(Some(42));
    setup_dungeon(66, 66);
    setup_player(Coord_t { y: 33, x: 33 }, 10);
    with_state_mut(|s| s.dg.current_level = 0);
    set_tile(
        Coord_t { y: 33, x: 34 },
        Tile {
            feature_id: TILE_GRANITE_WALL,
            ..Tile::default()
        },
    );

    spell_destroy_area(Coord_t { y: 33, x: 33 });

    assert_eq!(
        tile_at(Coord_t { y: 33, x: 34 }).feature_id,
        TILE_GRANITE_WALL
    );
    with_state(|s| assert!(s.py.flags.blind >= 11 && s.py.flags.blind <= 20));
}

// --------------------------------------------------------------------------
// 3. Wall-to-mud / build-wall crush
// --------------------------------------------------------------------------

#[test]
fn spell_wall_to_mud_granite_turns_to_floor_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    fill_corridor(Coord_t { y: 10, x: 10 }, 2, 3, TILE_CORR_FLOOR);
    set_tile(
        Coord_t { y: 13, x: 10 },
        Tile {
            feature_id: TILE_GRANITE_WALL,
            permanent_light: true,
            ..Tile::default()
        },
    );

    let _ = spell_wall_to_mud(Coord_t { y: 10, x: 10 }, 2);
    assert_eq!(
        tile_at(Coord_t { y: 13, x: 10 }).feature_id,
        TILE_CORR_FLOOR
    );
}

fn sum_four_d8() -> i32 {
    (0..4).map(|_| random_number(8)).sum()
}

#[test]
fn spell_wall_to_mud_rubble_deletes_object_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    fill_corridor(Coord_t { y: 10, x: 10 }, 2, 2, TILE_CORR_FLOOR);
    let rubble = Coord_t { y: 12, x: 10 };
    set_tile(
        rubble,
        Tile {
            feature_id: MIN_CLOSED_SPACE,
            ..Tile::default()
        },
    );
    place_treasure_at(rubble, OBJ_RUBBLE);

    let _ = spell_wall_to_mud(Coord_t { y: 10, x: 10 }, 2);

    assert_eq!(tile_at(rubble).treasure_id, 0);
}

#[test]
fn spell_build_wall_crushes_monster_with_four_d8_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    fill_corridor(Coord_t { y: 10, x: 10 }, 2, 3, TILE_CORR_FLOOR);
    place_monster(2, URCHIN_ID, 50, Coord_t { y: 13, x: 10 });

    assert!(spell_build_wall(Coord_t { y: 10, x: 10 }, 2));

    assert_eq!(
        tile_at(Coord_t { y: 13, x: 10 }).feature_id,
        TILE_MAGMA_WALL
    );
    with_state(|s| {
        let damage = 50 - i32::from(s.monsters[2].hp);
        assert!((4..=32).contains(&damage));
    });
}

#[test]
fn spell_build_wall_earth_elemental_gains_hp_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    fill_corridor(Coord_t { y: 10, x: 10 }, 2, 3, TILE_CORR_FLOOR);
    place_monster(2, EARTH_ELEMENTAL_ID, 200, Coord_t { y: 13, x: 10 });

    assert!(spell_build_wall(Coord_t { y: 10, x: 10 }, 2));
    let hp_after = with_state(|s| s.monsters[2].hp);

    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    fill_corridor(Coord_t { y: 10, x: 10 }, 2, 3, TILE_CORR_FLOOR);
    place_monster(2, EARTH_ELEMENTAL_ID, 200, Coord_t { y: 13, x: 10 });
    let hp_gain = sum_four_d8();

    assert_eq!(i32::from(hp_after), 200 + hp_gain);
}

#[test]
fn spell_wall_to_mud_stone_creature_takes_fixed_damage_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    fill_corridor(Coord_t { y: 10, x: 10 }, 2, 3, TILE_CORR_FLOOR);
    place_monster(2, STONE_GOLEM_ID, 500, Coord_t { y: 13, x: 10 });

    let _ = spell_wall_to_mud(Coord_t { y: 10, x: 10 }, 2);

    with_state(|s| assert!(s.monsters[2].hp < 500));
    with_state(|s| {
        assert_ne!(
            s.creature_recall[STONE_GOLEM_ID as usize].defenses & CD_STONE,
            0
        );
    });
}

// --------------------------------------------------------------------------
// 4. Tile-feature parity — warding glyph, destroy doors
// --------------------------------------------------------------------------

#[test]
fn spell_warding_glyph_places_scare_monster_no_rng() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 }, 10);

    let before = with_state(|s| s.rng.old_seed);
    spell_warding_glyph();
    let after = with_state(|s| s.rng.old_seed);

    assert_eq!(before, after);
    with_state(|s| {
        let tid = s.dg.floor[10][10].treasure_id;
        assert_ne!(tid, 0);
        assert_eq!(s.game.treasure.list[tid as usize].category_id, TV_VIS_TRAP);
    });
}

#[test]
fn spell_destroy_doors_traps_in_direction_deletes_trap_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    fill_corridor(Coord_t { y: 10, x: 10 }, 2, 25, TILE_CORR_FLOOR);
    let trap = Coord_t { y: 12, x: 10 };
    let tid = popt() as u8;
    with_state_mut(|s| {
        s.dg.floor[trap.y as usize][trap.x as usize].treasure_id = tid;
        s.game.treasure.list[tid as usize] = Inventory {
            category_id: TV_VIS_TRAP,
            ..Inventory::default()
        };
    });
    seal_directional_ray(Coord_t { y: 10, x: 10 }, 2);

    let _ = spell_destroy_doors_traps_in_direction(Coord_t { y: 10, x: 10 }, 2);
    assert_eq!(tile_at(trap).treasure_id, 0);
}

#[test]
fn spell_destroy_doors_traps_disarms_chest_without_deleting() {
    reset_for_new_game(Some(1));
    setup_dungeon(40, 40);
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    fill_corridor(Coord_t { y: 10, x: 10 }, 2, 25, TILE_CORR_FLOOR);
    let chest = Coord_t { y: 12, x: 10 };
    let tid = popt() as u8;
    with_state_mut(|s| {
        s.dg.floor[chest.y as usize][chest.x as usize].treasure_id = tid;
        s.game.treasure.list[tid as usize] = Inventory {
            category_id: TV_CHEST,
            flags: CH_TRAPPED | CH_LOCKED,
            ..Inventory::default()
        };
    });
    seal_directional_ray(Coord_t { y: 10, x: 10 }, 2);

    assert!(spell_destroy_doors_traps_in_direction(
        Coord_t { y: 10, x: 10 },
        2
    ));
    with_state(|s| {
        let tid = s.dg.floor[chest.y as usize][chest.x as usize].treasure_id;
        let item = &s.game.treasure.list[tid as usize];
        assert_eq!(item.flags & (CH_TRAPPED | CH_LOCKED), 0);
        assert_eq!(item.special_name_id, SpecialNameIds::SN_UNLOCKED as u8);
    });
}

// --------------------------------------------------------------------------
// 5. Integer semantics — blind int16 accumulation
// --------------------------------------------------------------------------

#[test]
fn spell_destroy_area_blind_accumulates_int16_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(66, 66);
    setup_player(Coord_t { y: 33, x: 33 }, 10);

    spell_destroy_area(Coord_t { y: 33, x: 33 });

    with_state(|s| {
        assert!(s.py.flags.blind >= 11 && s.py.flags.blind <= 20);
    });
}

#[test]
fn bolt_max_range_is_eighteen_for_directional_spells() {
    assert_eq!(OBJECT_BOLTS_MAX_RANGE, 18);
}
