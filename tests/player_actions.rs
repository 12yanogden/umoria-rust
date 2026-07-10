//! Player movement/state/search/experience/doors/tunnel parity.
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

use umoria::config::player::status::{PY_REST, PY_SEARCH, PY_STR_WGT};
use umoria::config::player::PLAYER_WEIGHT_CAP;
use umoria::config::treasure::chests::CH_LOCKED;
use umoria::data_player::CLASS_RANK_TITLES;
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, MIN_CLOSED_SPACE, TILE_CORR_FLOOR, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::inventory::PlayerEquipment;
use umoria::monster::Creature;
use umoria::player::{
    open_closed_chest, open_closed_door, player_carrying_load_limit, player_disturb,
    player_gain_kill_experience, player_get_gender_label, player_is_male,
    player_lock_picking_skill, player_move_position, player_no_light, player_rank_title,
    player_rest_off, player_rest_on, player_search, player_search_off, player_search_on,
    player_set_gender, player_strength, player_teleport, PlayerAttr, PLAYER_MAX_CLASSES,
    PLAYER_MAX_LEVEL,
};
use umoria::player_stats::player_initialize_base_experience_levels;
use umoria::player_tunnel::player_tunnel_wall;
use umoria::treasure::{TV_CHEST, TV_CLOSED_DOOR, TV_NOTHING};
use umoria::types::Coord_t;
use umoria::ui_io::test_set_ncurses_stub;

fn setup_open_dungeon(height: i16, width: i16) {
    player_initialize_base_experience_levels();
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
        for y in 0..height {
            for x in 0..width {
                s.dg.floor[y as usize][x as usize].feature_id = TILE_LIGHT_FLOOR;
            }
        }
    });
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

fn cpp_gain_kill_exp(
    kill_exp: u16,
    creature_level: u8,
    player_level: u16,
    exp_fraction: u16,
) -> (i32, u16) {
    let exp = i32::from(kill_exp) * i32::from(creature_level);
    let level = i32::from(player_level);
    let mut quotient = exp / level;
    let mut remainder = exp % level;
    remainder *= 0x1_0000;
    remainder /= level;
    remainder += i32::from(exp_fraction);
    let new_fraction = if remainder >= 0x1_0000 {
        quotient += 1;
        (remainder - 0x1_0000) as u16
    } else {
        remainder as u16
    };
    (quotient, new_fraction)
}

fn cpp_carrying_load_limit(used_str: u8, body_weight: u16) -> i32 {
    let mut weight_cap =
        i32::from(used_str) * i32::from(PLAYER_WEIGHT_CAP) + i32::from(body_weight);
    if weight_cap > 3000 {
        weight_cap = 3000;
    }
    weight_cap
}

// ---------------------------------------------------------------------------
// Gender / rank title
// ---------------------------------------------------------------------------

#[test]
fn player_gender_label_parity() {
    reset_for_new_game(None);
    player_set_gender(true);
    assert!(player_is_male());
    assert_eq!(player_get_gender_label(), "Male");
    player_set_gender(false);
    assert!(!player_is_male());
    assert_eq!(player_get_gender_label(), "Female");
}

#[test]
fn player_rank_title_matrix() {
    reset_for_new_game(None);
    for class_id in 0..PLAYER_MAX_CLASSES {
        for level in 1..=PLAYER_MAX_LEVEL {
            with_state_mut(|s| {
                s.py.misc.class_id = class_id;
                s.py.misc.level = u16::from(level);
                s.py.misc.gender = true;
            });
            assert_eq!(
                player_rank_title(),
                CLASS_RANK_TITLES[class_id as usize][level as usize - 1]
            );
        }
    }
    with_state_mut(|s| {
        s.py.misc.level = 0;
    });
    assert_eq!(player_rank_title(), "Babe in arms");
    with_state_mut(|s| {
        s.py.misc.level = PLAYER_MAX_LEVEL as u16 + 1;
        s.py.misc.gender = true;
    });
    assert_eq!(player_rank_title(), "**KING**");
    with_state_mut(|s| {
        s.py.misc.gender = false;
    });
    assert_eq!(player_rank_title(), "**QUEEN**");
}

// ---------------------------------------------------------------------------
// Load / strength
// ---------------------------------------------------------------------------

#[test]
fn player_carrying_load_limit_parity() {
    reset_for_new_game(None);
    for used_str in [3u8, 10, 18, 25] {
        for body_weight in [0u16, 100, 500] {
            with_state_mut(|s| {
                s.py.stats.used[0] = used_str;
                s.py.misc.weight = body_weight;
            });
            assert_eq!(
                player_carrying_load_limit(),
                cpp_carrying_load_limit(used_str, body_weight)
            );
        }
    }
}

#[test]
fn player_strength_heavy_weapon_and_pack_speed() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.stats.used[0] = 10;
        s.py.inventory[PlayerEquipment::Wield as usize].category_id = TV_NOTHING + 1;
        s.py.inventory[PlayerEquipment::Wield as usize].weight = 200;
        s.py.weapon_is_heavy = false;
        s.py.pack.weight = 5000;
        s.py.pack.heaviness = 0;
        s.py.flags.speed = 0;
    });
    player_strength();
    with_state(|s| {
        assert!(s.py.weapon_is_heavy);
        assert!(s.py.pack.heaviness > 0);
        assert_ne!(s.py.flags.speed, 0);
        assert_eq!(s.py.flags.status & PY_STR_WGT, 0);
    });
}

// ---------------------------------------------------------------------------
// Experience
// ---------------------------------------------------------------------------

#[test]
fn player_gain_kill_experience_parity_matrix() {
    reset_for_new_game(None);
    let cases: [(u16, u8, u16); 6] = [
        (10, 1, 1),
        (25, 5, 10),
        (100, 12, 20),
        (65535, 10, 15),
        (50, 3, 7),
        (1, 40, 40),
    ];
    for (kill_exp, creature_level, player_level) in cases {
        reset_for_new_game(Some(1));
        with_state_mut(|s| {
            s.py.misc.level = player_level;
            s.py.misc.exp = 100;
            s.py.misc.exp_fraction = 12345;
        });
        let creature = Creature {
            kill_exp_value: kill_exp,
            level: creature_level,
            ..Default::default()
        };
        let (expected_quotient, expected_fraction) =
            cpp_gain_kill_exp(kill_exp, creature_level, player_level, 12345);
        player_gain_kill_experience(&creature);
        with_state(|s| {
            assert_eq!(
                s.py.misc.exp,
                100 + expected_quotient,
                "level={player_level}"
            );
            assert_eq!(s.py.misc.exp_fraction, expected_fraction);
        });
    }
}

// ---------------------------------------------------------------------------
// Search / disturb / rest state transitions
// ---------------------------------------------------------------------------

#[test]
fn player_search_on_off_state_parity() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.flags.speed = 5;
        s.py.flags.food_digested = 10;
    });
    player_search_on();
    with_state(|s| {
        assert_ne!(s.py.flags.status & PY_SEARCH, 0);
        assert_eq!(s.py.flags.speed, 6);
        assert_eq!(s.py.flags.food_digested, 11);
    });
    player_search_off();
    with_state(|s| {
        assert_eq!(s.py.flags.status & PY_SEARCH, 0);
        assert_eq!(s.py.flags.speed, 5);
        assert_eq!(s.py.flags.food_digested, 10);
    });
}

#[test]
fn player_disturb_clears_search_rest_and_running() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.flags.status |= PY_SEARCH;
        s.py.flags.rest = 50;
        s.py.running_tracker = 3;
        s.game.command_count = 9;
    });
    player_disturb(1, 1);
    with_state(|s| {
        assert_eq!(s.game.command_count, 0);
        assert_eq!(s.py.flags.status & PY_SEARCH, 0);
        assert_eq!(s.py.flags.rest, 0);
        assert_eq!(s.py.running_tracker, 0);
    });
}

#[test]
fn player_rest_on_via_command_count() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.game.command_count = 25;
        s.py.flags.food_digested = 5;
    });
    player_rest_on();
    with_state(|s| {
        assert_eq!(s.game.command_count, 0);
        assert_eq!(s.py.flags.rest, 25);
        assert_ne!(s.py.flags.status & PY_REST, 0);
        assert_eq!(s.py.flags.food_digested, 4);
    });
    player_rest_off();
    with_state(|s| {
        assert_eq!(s.py.flags.rest, 0);
        assert_eq!(s.py.flags.status & PY_REST, 0);
        assert_eq!(s.py.flags.food_digested, 5);
    });
}

// ---------------------------------------------------------------------------
// Movement helpers
// ---------------------------------------------------------------------------

#[test]
fn player_move_position_bounds_parity() {
    reset_for_new_game(None);
    setup_open_dungeon(10, 10);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 5, x: 5 };
    });
    let mut coord = Coord_t { y: 8, x: 5 };
    assert!(player_move_position(2, &mut coord));
    assert_eq!(coord, Coord_t { y: 9, x: 5 });
    assert!(!player_move_position(2, &mut coord));
    assert_eq!(coord, Coord_t { y: 9, x: 5 });
}

#[test]
fn player_no_light_parity() {
    reset_for_new_game(None);
    setup_open_dungeon(5, 5);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 2, x: 2 };
        s.dg.floor[2][2].temporary_light = false;
        s.dg.floor[2][2].permanent_light = false;
    });
    assert!(player_no_light());
    with_state_mut(|s| {
        s.dg.floor[2][2].permanent_light = true;
    });
    assert!(!player_no_light());
}

// ---------------------------------------------------------------------------
// RNG-order paths
// ---------------------------------------------------------------------------

#[test]
fn player_teleport_rng_and_destination_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_open_dungeon(20, 20);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.dg.floor[10][10].creature_id = 1;
    });
    player_teleport(5);
    let (final_y, final_x) = with_state(|s| (s.py.pos.y, s.py.pos.x));
    assert!((0..20).contains(&final_y));
    assert!((0..20).contains(&final_x));
    with_state(|s| {
        let tile = &s.dg.floor[final_y as usize][final_x as usize];
        assert!(tile.feature_id < MIN_CLOSED_SPACE);
        assert!(tile.creature_id < 2);
        assert!(!s.game.teleport_player);
    });
    // Post-teleport RNG stream (rolls consumed during placement loop + update_monsters).
    assert_eq!(next_random_pair(20), (20, 16));
    assert_eq!(next_random_pair(20), (20, 2));
}

#[test]
fn player_search_rng_roll_count_seed99() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(99));
    setup_open_dungeon(10, 10);
    let coord = Coord_t { y: 5, x: 5 };
    player_search(coord, 50);
    // Nine adjacent tiles × one randomNumber(100) each.
    assert_eq!(next_random_pair(100), (100, 30));
}

#[test]
fn open_closed_door_lock_pick_rng_seed777() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(777));
    setup_open_dungeon(10, 10);
    with_state_mut(|s| {
        s.py.misc.disarm = 80;
        s.py.misc.class_id = 0;
        s.py.misc.level = 20;
        s.py.misc.experience_factor = 100;
        s.game.treasure.list[1].category_id = TV_CLOSED_DOOR;
        s.game.treasure.list[1].misc_use = 10;
        s.dg.floor[5][6].treasure_id = 1;
        s.dg.floor[5][6].feature_id = TILE_CORR_FLOOR;
    });
    open_closed_door(Coord_t { y: 5, x: 6 });
    with_state(|s| {
        assert_eq!(s.game.treasure.list[1].misc_use, 0);
        assert_eq!(
            s.game.treasure.list[1].category_id,
            umoria::treasure::TV_OPEN_DOOR
        );
    });
    assert_eq!(next_random_pair(100), (100, 29));
}

#[test]
fn open_closed_chest_lock_pick_rng_seed888() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(888));
    setup_open_dungeon(10, 10);
    with_state_mut(|s| {
        s.py.misc.disarm = 90;
        s.py.misc.class_id = 0;
        s.py.misc.level = 25;
        s.py.misc.experience_factor = 100;
        s.game.treasure.list[2].category_id = TV_CHEST;
        s.game.treasure.list[2].depth_first_found = 5;
        s.game.treasure.list[2].flags = CH_LOCKED;
        s.dg.floor[4][4].treasure_id = 2;
    });
    open_closed_chest(Coord_t { y: 4, x: 4 });
    with_state(|s| {
        let item = &s.game.treasure.list[2];
        assert_eq!(item.flags & CH_LOCKED, 0);
    });
    assert_eq!(next_random_pair(100), (100, 10));
}

// ---------------------------------------------------------------------------
// Tunnel wall
// ---------------------------------------------------------------------------

#[test]
fn player_tunnel_wall_ability_gate() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_open_dungeon(10, 10);
    let coord = Coord_t { y: 5, x: 5 };
    with_state_mut(|s| {
        s.dg.floor[5][5].feature_id = MIN_CLOSED_SPACE + 1;
    });
    assert!(!player_tunnel_wall(coord, 5, 10));
    assert!(player_tunnel_wall(coord, 15, 10));
    with_state(|s| {
        assert_eq!(s.dg.floor[5][5].feature_id, TILE_CORR_FLOOR);
    });
}

// ---------------------------------------------------------------------------
// Lock picking skill (deterministic components)
// ---------------------------------------------------------------------------

#[test]
fn player_lock_picking_skill_formula() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.misc.disarm = 40;
        s.py.misc.class_id = 0;
        s.py.misc.level = 15;
        s.py.stats.used[PlayerAttr::A_DEX as usize] = 16;
        s.py.stats.used[PlayerAttr::A_INT as usize] = 16;
    });
    let skill = player_lock_picking_skill();
    assert!(skill > 40);
}
