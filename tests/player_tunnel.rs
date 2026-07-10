//! `player_tunnel` parity.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::config::treasure::flags::TR_TUNNEL;
use umoria::dice::{max_dice_roll, Dice};
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{
    Tile, MIN_CLOSED_SPACE, TILE_BOUNDARY_WALL, TILE_CORR_FLOOR, TILE_GRANITE_WALL,
    TILE_LIGHT_FLOOR, TILE_MAGMA_WALL, TILE_QUARTZ_WALL,
};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::inventory::{Inventory, PlayerEquipment};
use umoria::player::PlayerAttr;
use umoria::player_stats::player_initialize_base_experience_levels;
use umoria::player_tunnel::{player_digging_ability, player_tunnel};
use umoria::treasure::{TV_NOTHING, TV_RUBBLE, TV_SECRET_DOOR, TV_SWORD};
use umoria::types::{Coord_t, MESSAGE_HISTORY_SIZE};
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

fn message_text(id: i16) -> String {
    with_state(|s| {
        let idx = id.rem_euclid(MESSAGE_HISTORY_SIZE as i16) as usize;
        let msg = &s.messages[idx];
        let end = msg.iter().position(|&b| b == 0).unwrap_or(msg.len());
        String::from_utf8_lossy(&msg[..end]).into_owned()
    })
}

fn last_message_text() -> String {
    message_text(with_state(|s| s.last_message_id))
}

fn equip_wield(weapon: Inventory) {
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Wield as usize] = weapon;
    });
}

fn cpp_digging_ability(str: u8, weapon: Inventory, weapon_is_heavy: bool) -> i32 {
    let mut digging_ability = i32::from(str);
    if (weapon.flags & TR_TUNNEL) != 0 {
        digging_ability += 25 + i32::from(weapon.misc_use) * 50;
    } else {
        digging_ability +=
            max_dice_roll(weapon.damage) + i32::from(weapon.to_hit) + i32::from(weapon.to_damage);
        digging_ability >>= 1;
    }
    if weapon_is_heavy {
        digging_ability += i32::from(str) * 15 - i32::from(weapon.weight);
        if digging_ability < 0 {
            digging_ability = 0;
        }
    }
    digging_ability
}

fn setup_player_at(y: i32, x: i32) {
    with_state_mut(|s| {
        s.py.pos = Coord_t { y, x };
        s.py.stats.used[PlayerAttr::A_STR as usize] = 18;
        s.py.weapon_is_heavy = false;
    });
}

fn assert_rng_unchanged_after(setup: impl Fn(), action: impl FnOnce()) {
    reset_for_new_game(Some(42));
    setup();
    let baseline = random_number(100);
    reset_for_new_game(Some(42));
    setup();
    action();
    assert_eq!(random_number(100), baseline);
}

fn alloc_rubble_at(y: i32, x: i32) {
    with_state_mut(|s| {
        s.game.treasure.current_id = 2;
        s.game.treasure.list[1].category_id = TV_RUBBLE;
        s.dg.floor[y as usize][x as usize].treasure_id = 1;
    });
}

fn setup_target_tile(y: i32, x: i32, feature_id: u8, treasure_id: u8, category_id: u8) {
    with_state_mut(|s| {
        s.dg.floor[y as usize][x as usize].feature_id = feature_id;
        s.dg.floor[y as usize][x as usize].treasure_id = treasure_id;
        if treasure_id != 0 {
            s.game.treasure.list[treasure_id as usize].category_id = category_id;
        }
    });
}

// ---------------------------------------------------------------------------
// Digging-ability math (deterministic, no RNG)
// ---------------------------------------------------------------------------

#[test]
fn player_digging_ability_weapon_formula() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.stats.used[PlayerAttr::A_STR as usize] = 18;
        s.py.weapon_is_heavy = false;
    });
    let weapon = Inventory {
        category_id: TV_SWORD,
        damage: Dice { dice: 3, sides: 6 },
        to_hit: 5,
        to_damage: 7,
        ..Default::default()
    };
    let expected = cpp_digging_ability(18, weapon, false);
    assert_eq!(player_digging_ability(weapon), expected);
    assert_eq!(expected, (18 + max_dice_roll(weapon.damage) + 5 + 7) / 2);
}

#[test]
fn player_digging_ability_tunnel_tool_formula() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.stats.used[PlayerAttr::A_STR as usize] = 16;
        s.py.weapon_is_heavy = false;
    });
    let weapon = Inventory {
        flags: TR_TUNNEL,
        misc_use: 2,
        ..Default::default()
    };
    let expected = cpp_digging_ability(16, weapon, false);
    assert_eq!(player_digging_ability(weapon), expected);
    assert_eq!(expected, 16 + 25 + 2 * 50);
}

#[test]
fn player_digging_ability_heavy_weapon_penalty() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.stats.used[PlayerAttr::A_STR as usize] = 10;
        s.py.weapon_is_heavy = true;
    });
    let weapon = Inventory {
        category_id: TV_SWORD,
        weight: 500,
        damage: Dice { dice: 2, sides: 6 },
        to_hit: 0,
        to_damage: 0,
        ..Default::default()
    };
    let expected = cpp_digging_ability(10, weapon, true);
    assert_eq!(player_digging_ability(weapon), expected);
}

#[test]
fn player_digging_ability_consumes_no_rng() {
    test_set_ncurses_stub(true);
    let weapon = Inventory {
        category_id: TV_SWORD,
        damage: Dice { dice: 2, sides: 6 },
        to_hit: 3,
        to_damage: 4,
        ..Default::default()
    };
    assert_rng_unchanged_after(
        || with_state_mut(|s| s.py.stats.used[PlayerAttr::A_STR as usize] = 18),
        || {
            let _ = player_digging_ability(weapon);
        },
    );
}

// ---------------------------------------------------------------------------
// No-weapon / illegal-tile paths
// ---------------------------------------------------------------------------

#[test]
fn player_tunnel_hands_only_message() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_open_dungeon(10, 10);
    setup_player_at(5, 5);
    setup_target_tile(5, 6, TILE_GRANITE_WALL, 0, 0);
    equip_wield(Inventory {
        category_id: TV_NOTHING,
        ..Default::default()
    });
    player_tunnel(6);
    assert_eq!(
        last_message_text(),
        "You dig with your hands, making no progress."
    );
    with_state(|s| assert_eq!(s.dg.floor[5][6].feature_id, TILE_GRANITE_WALL));
}

#[test]
fn player_tunnel_illegal_empty_air_sets_free_turn() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_open_dungeon(10, 10);
    setup_player_at(5, 5);
    equip_wield(Inventory {
        category_id: TV_SWORD,
        ..Default::default()
    });
    player_tunnel(6);
    with_state(|s| assert!(s.game.player_free_turn));
    assert_eq!(last_message_text(), "Tunnel through what?  Empty air?!?");
}

#[test]
fn player_tunnel_boundary_wall_message_no_rng() {
    test_set_ncurses_stub(true);
    assert_rng_unchanged_after(
        || {
            setup_open_dungeon(10, 10);
            setup_player_at(5, 5);
            setup_target_tile(5, 6, TILE_BOUNDARY_WALL, 0, 0);
            equip_wield(Inventory {
                category_id: TV_SWORD,
                damage: Dice { dice: 3, sides: 6 },
                to_hit: 10,
                to_damage: 10,
                ..Default::default()
            });
        },
        || player_tunnel(6),
    );
    assert_eq!(last_message_text(), "This seems to be permanent rock.");
    with_state(|s| assert_eq!(s.dg.floor[5][6].feature_id, TILE_BOUNDARY_WALL));
}

// ---------------------------------------------------------------------------
// Confusion RNG gate
// ---------------------------------------------------------------------------

#[test]
fn player_tunnel_non_confused_skips_confusion_rolls() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_open_dungeon(10, 10);
    setup_player_at(5, 5);
    setup_target_tile(5, 6, TILE_GRANITE_WALL, 0, 0);
    with_state_mut(|s| s.py.flags.confused = 0);
    equip_wield(Inventory {
        category_id: TV_SWORD,
        damage: Dice { dice: 3, sides: 6 },
        to_hit: 10,
        to_damage: 10,
        ..Default::default()
    });
    player_tunnel(6);
    // Granite path: randomNumber(1200)+80 consumed; next probe is randomNumber(1200).
    assert_eq!(next_random_pair(1200), (1200, 273));
}

#[test]
fn player_tunnel_confused_consumes_two_rolls_before_material() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_open_dungeon(10, 10);
    setup_player_at(5, 5);
    setup_target_tile(5, 6, TILE_GRANITE_WALL, 0, 0);
    with_state_mut(|s| s.py.flags.confused = 5);
    equip_wield(Inventory {
        category_id: TV_SWORD,
        damage: Dice { dice: 3, sides: 6 },
        to_hit: 10,
        to_damage: 10,
        ..Default::default()
    });
    player_tunnel(6);
    // seed 42: randomNumber(4)=3 (>1), randomNumber(9)=8, then granite roll.
    assert_eq!(next_random_pair(1200), (1200, 436));
}

#[test]
fn player_tunnel_granite_rng_and_failure_message() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_open_dungeon(10, 10);
    setup_player_at(5, 5);
    setup_target_tile(5, 6, TILE_GRANITE_WALL, 0, 0);
    equip_wield(Inventory {
        category_id: TV_SWORD,
        damage: Dice { dice: 1, sides: 4 },
        to_hit: 0,
        to_damage: 0,
        ..Default::default()
    });
    player_tunnel(6);
    assert_eq!(last_message_text(), "You tunnel into the granite wall.");
    with_state(|s| assert_eq!(s.dg.floor[5][6].feature_id, TILE_GRANITE_WALL));
    assert_eq!(next_random_pair(1200), (1200, 273));
}

#[test]
fn player_tunnel_magma_rng_and_failure_message() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_open_dungeon(10, 10);
    setup_player_at(5, 5);
    setup_target_tile(5, 6, TILE_MAGMA_WALL, 0, 0);
    equip_wield(Inventory {
        category_id: TV_SWORD,
        damage: Dice { dice: 1, sides: 4 },
        to_hit: 0,
        to_damage: 0,
        ..Default::default()
    });
    player_tunnel(6);
    assert_eq!(last_message_text(), "You tunnel into the magma intrusion.");
    assert_eq!(next_random_pair(600), (600, 273));
}

#[test]
fn player_tunnel_quartz_rng_and_failure_message() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_open_dungeon(10, 10);
    setup_player_at(5, 5);
    setup_target_tile(5, 6, TILE_QUARTZ_WALL, 0, 0);
    equip_wield(Inventory {
        category_id: TV_SWORD,
        damage: Dice { dice: 1, sides: 4 },
        to_hit: 0,
        to_damage: 0,
        ..Default::default()
    });
    player_tunnel(6);
    assert_eq!(last_message_text(), "You tunnel into the quartz vein.");
    assert_eq!(next_random_pair(400), (400, 273));
}

#[test]
fn player_tunnel_granite_success_converts_tile() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(1));
    setup_open_dungeon(10, 10);
    setup_player_at(5, 5);
    setup_target_tile(5, 6, TILE_GRANITE_WALL, 0, 0);
    equip_wield(Inventory {
        category_id: TV_SWORD,
        flags: TR_TUNNEL,
        misc_use: 5,
        ..Default::default()
    });
    with_state_mut(|s| s.py.stats.used[PlayerAttr::A_STR as usize] = 18);
    player_tunnel(6);
    assert_eq!(last_message_text(), "You have finished the tunnel.");
    with_state(|s| assert_eq!(s.dg.floor[5][6].feature_id, TILE_CORR_FLOOR));
}

// ---------------------------------------------------------------------------
// Rubble path — randomNumber(180) gate + optional treasure reveal
// ---------------------------------------------------------------------------

#[test]
fn player_tunnel_rubble_failure_message() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_open_dungeon(10, 10);
    setup_player_at(5, 5);
    with_state_mut(|s| s.py.stats.used[PlayerAttr::A_STR as usize] = 0);
    setup_target_tile(5, 6, TILE_LIGHT_FLOOR, 0, 0);
    alloc_rubble_at(5, 6);
    equip_wield(Inventory {
        category_id: TV_SWORD,
        ..Default::default()
    });
    player_tunnel(6);
    assert_eq!(last_message_text(), "You dig in the rubble.");
    with_state(|s| assert_eq!(s.dg.floor[5][6].treasure_id, 1));
    assert_eq!(next_random_pair(180), (180, 153));
}

#[test]
fn player_tunnel_rubble_success_removes_object() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(1));
    setup_open_dungeon(10, 10);
    setup_player_at(5, 5);
    setup_target_tile(5, 6, TILE_LIGHT_FLOOR, 0, 0);
    alloc_rubble_at(5, 6);
    equip_wield(Inventory {
        category_id: TV_SWORD,
        flags: TR_TUNNEL,
        misc_use: 5,
        ..Default::default()
    });
    with_state_mut(|s| s.py.stats.used[PlayerAttr::A_STR as usize] = 18);
    player_tunnel(6);
    assert_eq!(last_message_text(), "You have removed the rubble.");
    with_state(|s| assert_eq!(s.dg.floor[5][6].treasure_id, 0));
    // randomNumber(10) consumed; seed 1 next roll for max=10 is 7.
    assert_eq!(next_random_pair(10), (10, 10));
}

#[test]
fn player_tunnel_rubble_treasure_reveal_rng_site() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(99));
    setup_open_dungeon(10, 10);
    setup_player_at(5, 5);
    setup_target_tile(5, 6, TILE_LIGHT_FLOOR, 0, 0);
    alloc_rubble_at(5, 6);
    with_state_mut(|s| {
        s.dg.floor[5][6].temporary_light = true;
        s.py.stats.used[PlayerAttr::A_STR as usize] = 18;
    });
    equip_wield(Inventory {
        category_id: TV_SWORD,
        flags: TR_TUNNEL,
        misc_use: 5,
        ..Default::default()
    });
    player_tunnel(6);
    with_state(|s| assert_eq!(s.dg.floor[5][6].treasure_id, 0));
    // seed 99: randomNumber(180) then randomNumber(10)==1 triggers placement RNG.
    assert_eq!(next_random_pair(10), (10, 6));
}

// ---------------------------------------------------------------------------
// Secret door path
// ---------------------------------------------------------------------------

#[test]
fn player_tunnel_secret_door_triggers_search() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_open_dungeon(10, 10);
    setup_player_at(5, 5);
    setup_target_tile(5, 6, MIN_CLOSED_SPACE + 1, 2, TV_SECRET_DOOR);
    with_state_mut(|s| {
        s.game.treasure.current_id = 3;
        s.py.misc.chance_in_search = 100;
    });
    equip_wield(Inventory {
        category_id: TV_SWORD,
        damage: Dice { dice: 3, sides: 6 },
        to_hit: 10,
        to_damage: 10,
        ..Default::default()
    });
    player_tunnel(6);
    assert_eq!(last_message_text(), "You tunnel into the granite wall.");
    // player_search at py.pos consumes nine randomNumber(100) rolls.
    assert_eq!(next_random_pair(100), (100, 8));
}
