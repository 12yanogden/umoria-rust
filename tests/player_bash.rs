//! `player_bash` parity.
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

use umoria::config::dungeon::objects::{OBJ_CLOSED_DOOR, OBJ_OPEN_DOOR, OBJ_RUINED_CHEST};
use umoria::config::monsters::defense::CD_MAX_HP;
use umoria::config::treasure::chests::CH_LOCKED;
use umoria::data_creatures::CREATURES_LIST;
use umoria::data_player::CLASS_LEVEL_ADJ;
use umoria::dice::{max_dice_roll, Dice};
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, TILE_CORR_FLOOR, TILE_GRANITE_WALL, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::inventory::{
    inventory_item_copy_to, Inventory, PlayerEquipment, PLAYER_INVENTORY_SIZE,
};
use umoria::monster::{Monster, MON_TOTAL_ALLOCATIONS};
use umoria::player::{PlayerAttr, PlayerClassLevelAdj, BTH_PER_PLUS_TO_HIT_ADJUST};
use umoria::player_bash::{
    player_bash, player_bash_attack, player_bash_closed_chest, player_bash_closed_door,
};
use umoria::treasure::TV_CHEST;
use umoria::types::{Coord_t, MESSAGE_HISTORY_SIZE};
use umoria::ui_io::{test_set_direction, test_set_ncurses_stub};

const GREY_MUSHROOM_ID: u16 = 8;
const POS: Coord_t = Coord_t { y: 10, x: 10 };
const TARGET: Coord_t = Coord_t { y: 11, x: 10 };

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
    with_state(|s| message_text(s.last_message_id))
}

fn assert_rng_unchanged_after(setup: impl Fn(), action: impl FnOnce()) {
    reset_for_new_game(Some(7));
    setup();
    let baseline = random_number(100);
    reset_for_new_game(Some(7));
    setup();
    action();
    assert_eq!(random_number(100), baseline);
}

fn setup_dungeon(height: i16, width: i16, pos: Coord_t) {
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                s.dg.floor[y as usize][x as usize].feature_id = TILE_LIGHT_FLOOR;
            }
        }
        s.dg.floor[pos.y as usize][pos.x as usize].temporary_light = true;
        s.game.treasure.current_id = 1;
    });
}

fn setup_player(str_stat: u8, dex_stat: u8, weight: u16) {
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.pos = POS;
        s.py.misc.class_id = 0;
        s.py.misc.level = 10;
        s.py.misc.weight = weight;
        s.py.misc.bth = 50;
        s.py.flags.confused = 0;
        s.py.flags.afraid = 0;
        s.py.flags.paralysis = 0;
        s.py.stats.used[PlayerAttr::A_STR as usize] = str_stat;
        s.py.stats.used[PlayerAttr::A_DEX as usize] = dex_stat;
        for i in 0..PLAYER_INVENTORY_SIZE as usize {
            inventory_item_copy_to(0, &mut s.py.inventory[i]);
        }
        s.py.inventory[PlayerEquipment::Arm as usize] = Inventory {
            category_id: 34,
            weight: 50,
            damage: Dice { dice: 1, sides: 6 },
            ..Default::default()
        };
    });
}

fn reset_monster_slots() {
    with_state_mut(|s| {
        s.next_free_monster_id = 2;
        s.monsters = [Monster::default(); MON_TOTAL_ALLOCATIONS as usize];
    });
}

fn place_monster(id: i32, creature_id: u16, hp: i16, coord: Coord_t, lit: bool) {
    with_state_mut(|s| {
        s.monsters[id as usize] = Monster {
            hp,
            creature_id,
            pos: coord,
            lit,
            ..Default::default()
        };
        s.dg.floor[coord.y as usize][coord.x as usize].creature_id = id as u8;
        if id + 1 >= i32::from(s.next_free_monster_id) {
            s.next_free_monster_id = (id + 1) as i16;
        }
    });
}

fn place_closed_door(coord: Coord_t, misc_use: i16) -> u8 {
    with_state_mut(|s| {
        let treasure_id = s.game.treasure.current_id as u8;
        s.game.treasure.current_id += 1;
        inventory_item_copy_to(
            OBJ_CLOSED_DOOR as i16,
            &mut s.game.treasure.list[treasure_id as usize],
        );
        s.game.treasure.list[treasure_id as usize].misc_use = misc_use;
        s.dg.floor[coord.y as usize][coord.x as usize].treasure_id = treasure_id;
        s.dg.floor[coord.y as usize][coord.x as usize].feature_id = TILE_CORR_FLOOR;
        treasure_id
    })
}

fn place_chest(coord: Coord_t, flags: u32) -> u8 {
    with_state_mut(|s| {
        let treasure_id = s.game.treasure.current_id as u8;
        s.game.treasure.current_id += 1;
        s.game.treasure.list[treasure_id as usize] = Inventory {
            category_id: TV_CHEST,
            flags,
            ..Default::default()
        };
        s.dg.floor[coord.y as usize][coord.x as usize].treasure_id = treasure_id;
        treasure_id
    })
}

fn bash_hit_chance() -> i32 {
    with_state(|s| {
        i32::from(s.py.stats.used[PlayerAttr::A_STR as usize])
            + i32::from(s.py.inventory[PlayerEquipment::Arm as usize].weight) / 2
            + i32::from(s.py.misc.weight) / 10
            + i32::from(s.py.stats.used[PlayerAttr::A_DEX as usize])
                * i32::from(BTH_PER_PLUS_TO_HIT_ADJUST)
            + i32::from(s.py.misc.level)
                * i32::from(
                    CLASS_LEVEL_ADJ[s.py.misc.class_id as usize][PlayerClassLevelAdj::BTH as usize],
                )
    })
}

// ---------------------------------------------------------------------------
// 1. Dispatch — routing and zero-RNG invalid targets
// ---------------------------------------------------------------------------
#[test]
fn player_bash_empty_space_message_and_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(20, 20, POS);
            setup_player(18, 10, 150);
            test_set_direction(Some(2));
        },
        player_bash,
    );
    assert_eq!(last_message_text(), "You bash at empty space.");
}

#[test]
fn player_bash_wall_message_and_no_rng() {
    reset_for_new_game(Some(7));
    setup_dungeon(20, 20, POS);
    setup_player(18, 10, 150);
    with_state_mut(|s| {
        s.dg.floor[TARGET.y as usize][TARGET.x as usize].feature_id = TILE_GRANITE_WALL;
    });
    test_set_direction(Some(2));
    assert_rng_unchanged_after(
        || {
            setup_dungeon(20, 20, POS);
            setup_player(18, 10, 150);
            with_state_mut(|s| {
                s.dg.floor[TARGET.y as usize][TARGET.x as usize].feature_id = TILE_GRANITE_WALL;
            });
            test_set_direction(Some(2));
        },
        player_bash,
    );
    assert_eq!(
        last_message_text(),
        "You bash it, but nothing interesting happens."
    );
}

#[test]
fn player_bash_routes_to_monster() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20, POS);
    setup_player(18, 10, 150);
    reset_monster_slots();
    place_monster(2, GREY_MUSHROOM_ID, 50, TARGET, true);
    test_set_direction(Some(2));
    player_bash();
    with_state(|s| assert_eq!(s.monsters[2].sleep_count, 0));
}

#[test]
fn player_bash_routes_to_door() {
    reset_for_new_game(Some(99));
    setup_dungeon(20, 20, POS);
    setup_player(18, 10, 150);
    place_closed_door(TARGET, 3);
    test_set_direction(Some(2));
    player_bash();
    assert!(last_message_text().contains("door"));
}

#[test]
fn player_bash_routes_to_chest() {
    reset_for_new_game(Some(55));
    setup_dungeon(20, 20, POS);
    setup_player(18, 10, 150);
    place_chest(TARGET, CH_LOCKED);
    test_set_direction(Some(2));
    player_bash();
    assert!(last_message_text().contains("chest"));
}

// ---------------------------------------------------------------------------
// 2. Monster bash — RNG order (hit + stun + stumble)
// ---------------------------------------------------------------------------
#[test]
fn player_bash_monster_rng_order_hit_stun_seed1() {
    reset_for_new_game(Some(1));
    setup_dungeon(20, 20, POS);
    setup_player(18, 10, 150);
    reset_monster_slots();
    place_monster(2, GREY_MUSHROOM_ID, 200, TARGET, true);

    player_bash_attack(TARGET);

    let hit_chance = bash_hit_chance();
    assert_eq!(next_random_pair(20), (20, 20));
    assert_eq!(next_random_pair(hit_chance), (hit_chance, 28));
    assert_eq!(next_random_pair(6), (6, 1));
    assert_eq!(next_random_pair(5000), (5000, 2684));
    assert_eq!(next_random_pair(400), (400, 138));
    assert_eq!(next_random_pair(400), (400, 85));
    assert_eq!(next_random_pair(3), (3, 1));
    assert_eq!(next_random_pair(150), (150, 10));

    with_state(|s| {
        assert_eq!(s.monsters[2].stunned_amount, 2);
        assert_eq!(s.py.flags.paralysis, 0);
    });
}

#[test]
fn player_bash_monster_stun_amount_roll_skipped_when_gate_fails() {
    reset_for_new_game(Some(1));
    setup_dungeon(20, 20, POS);
    setup_player(18, 10, 150);
    reset_monster_slots();
    place_monster(2, GREY_MUSHROOM_ID, 5000, TARGET, true);

    player_bash_attack(TARGET);

    let hit_chance = bash_hit_chance();
    let _ = next_random_pair(20);
    let _ = next_random_pair(hit_chance);
    let _ = next_random_pair(6);
    let _ = next_random_pair(5000);
    let _ = next_random_pair(400);
    let _ = next_random_pair(400);

    assert_eq!(next_random_pair(150), (150, 25));
    with_state(|s| assert_eq!(s.monsters[2].stunned_amount, 0));
}

#[test]
fn player_bash_monster_avg_hp_uses_max_dice_roll_when_cd_max_hp() {
    let creature_id = CREATURES_LIST
        .iter()
        .position(|creature| (creature.defenses & CD_MAX_HP) != 0)
        .unwrap() as u16;

    reset_for_new_game(Some(1));
    setup_dungeon(20, 20, POS);
    setup_player(18, 10, 150);
    reset_monster_slots();
    place_monster(2, creature_id, 200, TARGET, true);

    player_bash_attack(TARGET);

    let expected_avg = max_dice_roll(CREATURES_LIST[creature_id as usize].hit_die);
    let hp = with_state(|s| i32::from(s.monsters[2].hp));
    let stun_passes = 100 + 138 + 85 > hp + expected_avg;

    assert!(stun_passes);
    with_state(|s| assert!(s.monsters[2].stunned_amount > 0));
}

// ---------------------------------------------------------------------------
// 3. Door bash — RNG order and int16 misc_use cast
// ---------------------------------------------------------------------------
#[test]
fn player_bash_door_success_rng_order_seed4() {
    reset_for_new_game(Some(4));
    setup_dungeon(20, 20, POS);
    setup_player(18, 10, 150);
    with_state_mut(|s| s.py.flags.confused = 1);
    place_closed_door(TARGET, 3);

    player_bash_closed_door(TARGET, 2);

    let chance = 18 + 150 / 2;
    assert_eq!(
        next_random_pair(chance * (20 + 3)),
        (chance * (20 + 3), 857)
    );
    assert_eq!(next_random_pair(2), (2, 1));

    with_state(|s| {
        let item = &s.game.treasure.list
            [s.dg.floor[TARGET.y as usize][TARGET.x as usize].treasure_id as usize];
        assert_eq!(item.misc_use, -1);
        assert_eq!(item.id, OBJ_OPEN_DOOR);
    });
}

#[test]
fn player_bash_door_failure_stumble_rng_order_seed7() {
    reset_for_new_game(Some(7));
    setup_dungeon(20, 20, POS);
    setup_player(10, 5, 0);
    place_closed_door(TARGET, 10);

    player_bash_closed_door(TARGET, 2);

    let chance = 10;
    assert_eq!(
        next_random_pair(chance * (20 + 10)),
        (chance * (20 + 10), 224)
    );
    assert_eq!(next_random_pair(150), (150, 53));
    assert_eq!(next_random_pair(2), (2, 2));

    with_state(|s| assert_eq!(s.py.flags.paralysis, 2));
}

#[test]
fn player_bash_door_misc_use_int16_cast_can_be_negative() {
    reset_for_new_game(Some(6));
    setup_dungeon(20, 20, POS);
    setup_player(18, 10, 150);
    with_state_mut(|s| s.py.flags.confused = 1);
    place_closed_door(TARGET, 3);
    player_bash_closed_door(TARGET, 2);

    let chance = 18 + 150 / 2;
    let _ = next_random_pair(chance * (20 + 3));
    assert_eq!(next_random_pair(2), (2, 2));
    with_state(|s| {
        let treasure_id = s.dg.floor[TARGET.y as usize][TARGET.x as usize].treasure_id;
        assert_eq!(s.game.treasure.list[treasure_id as usize].misc_use, -1);
    });
}

// ---------------------------------------------------------------------------
// 4. Chest bash — RNG order
// ---------------------------------------------------------------------------
#[test]
fn player_bash_chest_destroy_rng_order_seed9() {
    reset_for_new_game(Some(9));
    setup_dungeon(20, 20, POS);
    let treasure_id = place_chest(TARGET, CH_LOCKED);

    player_bash_closed_chest(treasure_id);

    assert_eq!(next_random_pair(10), (10, 4));
    with_state(|s| {
        assert_eq!(
            s.game.treasure.list[treasure_id as usize].id,
            OBJ_RUINED_CHEST
        );
        assert_eq!(s.game.treasure.list[treasure_id as usize].flags, 0);
    });
}

#[test]
fn player_bash_chest_lock_break_rng_order_seed12() {
    reset_for_new_game(Some(12));
    setup_dungeon(20, 20, POS);
    let treasure_id = place_chest(TARGET, CH_LOCKED);

    player_bash_closed_chest(treasure_id);

    assert_eq!(next_random_pair(10), (10, 7));
    with_state(|s| {
        assert!(s.game.treasure.list[treasure_id as usize].flags & CH_LOCKED == 0);
    });
}

#[test]
fn player_bash_chest_holds_firm_rng_order_seed7() {
    reset_for_new_game(Some(7));
    setup_dungeon(20, 20, POS);
    let treasure_id = place_chest(TARGET, CH_LOCKED);

    player_bash_closed_chest(treasure_id);

    assert_eq!(next_random_pair(10), (10, 3));
    assert_eq!(next_random_pair(10), (10, 4));
    let _ = treasure_id;
}

#[test]
fn player_bash_chest_destroy_skips_second_roll() {
    reset_for_new_game(Some(9));
    setup_dungeon(20, 20, POS);
    let treasure_id = place_chest(TARGET, CH_LOCKED);
    player_bash_closed_chest(treasure_id);
    let baseline = random_number(100);

    reset_for_new_game(Some(9));
    setup_dungeon(20, 20, POS);
    let treasure_id = place_chest(TARGET, CH_LOCKED);
    player_bash_closed_chest(treasure_id);
    assert_eq!(random_number(100), baseline);
    let _ = treasure_id;
}
