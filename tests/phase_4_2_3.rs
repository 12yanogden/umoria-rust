//! Phase 4.2.3 — monster melee attacks on player parity.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use umoria::config::dungeon::objects::OBJ_NOTHING;
use umoria::config::monsters::defense::{CD_EVIL, CD_NO_SLEEP};
use umoria::data_creatures::CREATURES_LIST;
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::inventory::{
    inventory_item_copy_to, Inventory, PlayerEquipment, ITEM_GROUP_MIN, ITEM_SINGLE_STACK_MIN,
};
use umoria::monster::{
    execute_attack_on_player, monster_attack_player, monster_confuse_on_attack,
    monster_print_attack_description, Monster, MON_TOTAL_ALLOCATIONS,
};
use umoria::player::PlayerAttr;
use umoria::treasure::{TV_FOOD, TV_WAND};
use umoria::types::{Coord_t, Vtype_t, MORIA_MESSAGE_SIZE};
use umoria::ui_io::test_set_ncurses_stub;

const URCHIN_ID: u16 = 0;
const GREY_MUSHROOM_ID: u16 = 8;
/// Kobold: normal hit attack with description_id 1 (prints "misses you." on miss).
const KOBOLD_ID: u16 = 16;

fn setup_dungeon(height: i16, width: i16) {
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                s.dg.floor[y as usize][x as usize].feature_id = TILE_LIGHT_FLOOR;
            }
        }
    });
}

fn reset_monster_slots() {
    with_state_mut(|s| {
        s.next_free_monster_id = i16::from(umoria::config::monsters::MON_MIN_INDEX_ID);
        s.monsters = [Monster::default(); MON_TOTAL_ALLOCATIONS as usize];
        s.hack_monptr = -1;
    });
}

fn place_monster(id: i32, creature_id: u16, hp: i16, coord: Coord_t, lit: bool) {
    with_state_mut(|s| {
        s.monsters[id as usize] = Monster {
            hp,
            sleep_count: 99,
            creature_id,
            pos: coord,
            lit,
            ..Default::default()
        };
        if id + 1 >= i32::from(s.next_free_monster_id) {
            s.next_free_monster_id = (id + 1) as i16;
        }
    });
}

fn setup_player_for_combat() {
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.misc.current_hp = 500;
        s.py.misc.max_hp = 500;
        s.py.misc.ac = 0;
        s.py.misc.magical_ac = 0;
        s.py.misc.level = 10;
        s.py.misc.au = 1000;
        s.py.misc.saving_throw = 0;
        s.py.misc.class_id = 0;
        s.py.stats.used[PlayerAttr::A_DEX as usize] = 10;
        s.py.stats.current[PlayerAttr::A_STR as usize] = 16;
        s.py.stats.current[PlayerAttr::A_DEX as usize] = 16;
        s.py.stats.current[PlayerAttr::A_CON as usize] = 16;
        s.py.stats.current[PlayerAttr::A_INT as usize] = 16;
        s.py.stats.current[PlayerAttr::A_WIS as usize] = 16;
        s.py.pack.unique_items = 5;
        s.game.character_is_dead = false;
        s.message_ready_to_print = false;
    });
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

fn message_text(id: i16) -> String {
    with_state(|s| {
        let idx = id.rem_euclid(umoria::types::MESSAGE_HISTORY_SIZE as i16) as usize;
        let msg = &s.messages[idx];
        let end = msg.iter().position(|&b| b == 0).unwrap_or(msg.len());
        String::from_utf8_lossy(&msg[..end]).into_owned()
    })
}

fn set_it_prefix(msg: &mut Vtype_t) {
    msg[0] = b'I';
    msg[1] = b't';
    msg[2] = b' ';
    msg[3] = 0;
}

fn death_description_for(creature_id: u16) -> Vtype_t {
    let creature = &CREATURES_LIST[creature_id as usize];
    let mut desc = [0u8; MORIA_MESSAGE_SIZE];
    umoria::player::player_died_from_string(&mut desc, creature.name, creature.movement);
    desc
}

// ---------------------------------------------------------------------------
// 1. monster_attack_player loop parity
// ---------------------------------------------------------------------------
#[test]
fn monster_attack_player_repel_protect_evil_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player_for_combat();
    with_state_mut(|s| {
        s.py.flags.protect_evil = 1;
    });
    place_monster(2, URCHIN_ID, 10, Coord_t { y: 10, x: 10 }, true);

    monster_attack_player(2);

    with_state(|s| {
        assert_eq!(
            message_text(s.last_message_id),
            "The Filthy Street Urchin is repelled."
        );
        assert!(s.creature_recall[URCHIN_ID as usize].defenses & CD_EVIL != 0);
        assert_eq!(s.py.misc.current_hp, 500);
    });
    assert_eq!(next_random_pair(2), (2, 2));
    assert_eq!(next_random_pair(4), (4, 1));
}

#[test]
fn monster_attack_player_miss_prints_message_seed777() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(777));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player_for_combat();
    with_state_mut(|s| {
        s.py.misc.ac = 200;
        s.py.misc.magical_ac = 200;
    });
    // Kobold attack desc is in 1..=3, so the miss path prints "misses you."
    place_monster(2, KOBOLD_ID, 10, Coord_t { y: 10, x: 10 }, true);

    monster_attack_player(2);

    with_state(|s| {
        let msg = message_text(s.last_message_id);
        assert!(
            msg.contains("misses you."),
            "expected miss message, got {msg:?}"
        );
        assert_eq!(s.py.misc.current_hp, 500);
    });
}

// ---------------------------------------------------------------------------
// 2. execute_attack_on_player per-type parity
// ---------------------------------------------------------------------------
#[test]
fn execute_attack_normal_ac_reduction_truncates() {
    reset_for_new_game(None);
    setup_player_for_combat();
    with_state_mut(|s| {
        s.py.misc.ac = 100;
        s.py.misc.magical_ac = 0;
    });
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    let noticed = execute_attack_on_player(5, &mut hp, 2, 1, 100, &death, true);
    assert!(noticed);
    with_state(|s| assert_eq!(s.py.misc.current_hp, 450));
}

#[test]
fn execute_attack_normal_ac_halfway_rounds_down() {
    reset_for_new_game(None);
    setup_player_for_combat();
    with_state_mut(|s| {
        s.py.misc.ac = 100;
        s.py.misc.magical_ac = 0;
    });
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(5, &mut hp, 2, 1, 99, &death, true);
    with_state(|s| assert_eq!(s.py.misc.current_hp, 450));
}

#[test]
fn execute_attack_lose_str_rng_and_stat_seed1() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(1));
    setup_player_for_combat();
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    let noticed = execute_attack_on_player(5, &mut hp, 2, 2, 10, &death, true);
    assert!(noticed);
    with_state(|s| {
        assert_eq!(message_text(s.last_message_id), "You feel weaker.");
        assert_eq!(s.py.stats.current[PlayerAttr::A_STR as usize], 15);
    });
    assert_eq!(next_random_pair(2), (2, 1));
}

#[test]
fn execute_attack_confuse_seed1() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(1));
    setup_player_for_combat();
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 3, 5, &death, true);
    with_state(|s| {
        assert_eq!(message_text(s.last_message_id), "You feel confused.");
        assert_eq!(s.py.flags.confused, 12);
    });
}

#[test]
fn execute_attack_fear_failed_save_seed1() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(1));
    setup_player_for_combat();
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 4, 5, &death, true);
    with_state(|s| {
        assert_eq!(message_text(s.last_message_id), "You are suddenly afraid!");
        assert_eq!(s.py.flags.afraid, 12);
    });
}

fn clear_pack_and_equipment() {
    with_state_mut(|s| {
        for i in 0..umoria::inventory::PLAYER_INVENTORY_SIZE as usize {
            inventory_item_copy_to(OBJ_NOTHING as i16, &mut s.py.inventory[i]);
        }
        s.py.pack.unique_items = 0;
    });
}

#[test]
fn execute_attack_fire_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    clear_pack_and_equipment();
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 5, 20, &death, true);
    with_state(|s| {
        assert_eq!(s.py.misc.current_hp, 480);
        assert_eq!(
            message_text(s.last_message_id),
            "You are enveloped in flames!"
        );
    });
}

#[test]
fn execute_attack_acid_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    clear_pack_and_equipment();
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 6, 20, &death, true);
    with_state(|s| {
        // No AC acid resist → full damage (flag+1 == 1).
        assert_eq!(s.py.misc.current_hp, 480);
        assert_eq!(message_text(s.last_message_id), "You are covered in acid!");
    });
}

#[test]
fn execute_attack_cold_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    clear_pack_and_equipment();
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 7, 20, &death, true);
    with_state(|s| {
        assert_eq!(s.py.misc.current_hp, 480);
        assert_eq!(
            message_text(s.last_message_id),
            "You are covered with frost!"
        );
    });
}

#[test]
fn execute_attack_lightning_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    clear_pack_and_equipment();
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 8, 20, &death, true);
    with_state(|s| {
        assert_eq!(s.py.misc.current_hp, 480);
        assert_eq!(message_text(s.last_message_id), "Lightning strikes you!");
    });
}

#[test]
fn execute_attack_corrosion_gas_and_hit_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 9, 12, &death, true);
    with_state(|s| {
        assert_eq!(s.py.misc.current_hp, 482);
        assert_eq!(
            message_text(s.last_message_id),
            "A stinging red gas swirls about you."
        );
    });
}

#[test]
fn execute_attack_blind_first_time_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 10, 5, &death, true);
    with_state(|s| {
        assert_eq!(message_text(s.last_message_id), "Your eyes begin to sting.");
        assert_eq!(s.py.flags.blind, 12);
    });
}

#[test]
fn execute_attack_paralyze_seed1() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(1));
    setup_player_for_combat();
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 11, 5, &death, true);
    with_state(|s| {
        assert_eq!(message_text(s.last_message_id), "You are paralyzed.");
        assert_eq!(s.py.flags.paralysis, 12);
    });
}

#[test]
fn execute_attack_steal_gold_dex_save_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player_for_combat();
    place_monster(2, GREY_MUSHROOM_ID, 10, Coord_t { y: 10, x: 11 }, true);
    with_state_mut(|s| s.py.stats.used[PlayerAttr::A_DEX as usize] = 100);
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 12, 0, &death, true);
    with_state(|s| {
        assert!(message_text(s.last_message_id).contains("You quickly protect your money pouch!"));
        assert_eq!(s.py.misc.au, 1000);
    });
}

#[test]
fn execute_attack_steal_gold_taken_seed777() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(777));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player_for_combat();
    place_monster(2, GREY_MUSHROOM_ID, 10, Coord_t { y: 10, x: 11 }, true);
    with_state_mut(|s| s.py.stats.used[PlayerAttr::A_DEX as usize] = 5);
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 12, 0, &death, true);
    with_state(|s| {
        assert_eq!(message_text(s.last_message_id), "Your purse feels lighter.");
        assert!(s.py.misc.au < 1000);
    });
}

#[test]
fn execute_attack_steal_item_dex_save_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player_for_combat();
    place_monster(2, GREY_MUSHROOM_ID, 10, Coord_t { y: 10, x: 11 }, true);
    with_state_mut(|s| s.py.stats.used[PlayerAttr::A_DEX as usize] = 100);
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 13, 0, &death, true);
    with_state(|s| {
        assert!(message_text(s.last_message_id).contains("You grab hold of your backpack!"));
    });
}

#[test]
fn execute_attack_poison_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 14, 8, &death, true);
    with_state(|s| {
        assert_eq!(message_text(s.last_message_id), "You feel very sick.");
        assert_eq!(s.py.flags.poisoned, 7);
    });
}

#[test]
fn execute_attack_drain_xp_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    with_state_mut(|s| {
        s.py.misc.exp = 1000;
        s.py.misc.max_exp = 1000;
        s.py.misc.level = 5;
        s.py.misc.experience_factor = 100;
        // Thresholds so level stays 5 after draining 30 exp (970 still above level-5 floor).
        for i in 0..s.py.base_exp_levels.len() {
            s.py.base_exp_levels[i] = if i < 4 { 0 } else { 10_000 };
        }
    });
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    // damage 10 → drain = 10 + (1000/100)*2 = 30
    execute_attack_on_player(10, &mut hp, 2, 19, 10, &death, true);
    with_state(|s| {
        assert_eq!(s.py.misc.exp, 970);
        assert_eq!(s.py.misc.level, 5);
        assert_eq!(
            message_text(s.last_message_id),
            "You feel your life draining away!"
        );
    });
}

#[test]
fn execute_attack_aggravate_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player_for_combat();
    place_monster(2, URCHIN_ID, 10, Coord_t { y: 10, x: 12 }, true);
    with_state_mut(|s| {
        s.monsters[2].sleep_count = 40;
        s.monsters[2].distance_from_player = 5;
        s.monsters[2].speed = 1;
    });
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 20, 0, &death, true);
    with_state(|s| {
        assert_eq!(s.monsters[2].sleep_count, 0);
        assert_eq!(s.monsters[2].speed, 2);
        assert_eq!(
            message_text(s.last_message_id),
            "You hear a sudden stirring in the distance!"
        );
    });
}

#[test]
fn execute_attack_disenchant_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    with_state_mut(|s| {
        for slot in [
            PlayerEquipment::Wield,
            PlayerEquipment::Body,
            PlayerEquipment::Arm,
            PlayerEquipment::Outer,
            PlayerEquipment::Hands,
            PlayerEquipment::Head,
            PlayerEquipment::Feet,
        ] {
            s.py.inventory[slot as usize] = Inventory {
                category_id: 30, // TV_SWORD-ish placeholder
                to_hit: 3,
                to_damage: 2,
                to_ac: 4,
                ..Default::default()
            };
        }
    });
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    let noticed = execute_attack_on_player(10, &mut hp, 2, 21, 0, &death, true);
    assert!(noticed);
    with_state(|s| {
        assert_eq!(
            message_text(s.last_message_id),
            "There is a static feeling in the air."
        );
        let reduced = [
            PlayerEquipment::Wield,
            PlayerEquipment::Body,
            PlayerEquipment::Arm,
            PlayerEquipment::Outer,
            PlayerEquipment::Hands,
            PlayerEquipment::Head,
            PlayerEquipment::Feet,
        ]
        .into_iter()
        .any(|slot| {
            let item = &s.py.inventory[slot as usize];
            item.to_hit < 3 || item.to_damage < 2 || item.to_ac < 4
        });
        assert!(reduced);
    });
}

#[test]
fn execute_attack_eat_food_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    with_state_mut(|s| {
        for i in 0..umoria::inventory::PLAYER_INVENTORY_SIZE as usize {
            inventory_item_copy_to(OBJ_NOTHING as i16, &mut s.py.inventory[i]);
        }
        s.py.inventory[0] = Inventory {
            category_id: TV_FOOD,
            sub_category_id: ITEM_GROUP_MIN,
            items_count: 1,
            weight: 1,
            ..Default::default()
        };
        s.py.pack.unique_items = 1;
    });
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    let noticed = execute_attack_on_player(10, &mut hp, 2, 22, 0, &death, true);
    assert!(noticed);
    with_state(|s| {
        assert_eq!(s.py.pack.unique_items, 0);
        assert_eq!(message_text(s.last_message_id), "It got at your rations!");
    });
}

#[test]
fn execute_attack_eat_light_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Light as usize].misc_use = 1000;
        s.py.flags.blind = 0;
    });
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    let noticed = execute_attack_on_player(10, &mut hp, 2, 23, 0, &death, true);
    assert!(noticed);
    with_state(|s| {
        // C++: misc_use -= 250 + randomNumber(250); seed42 → 548 (same as phase_4_5_1).
        assert_eq!(
            s.py.inventory[PlayerEquipment::Light as usize].misc_use,
            548
        );
        assert_eq!(message_text(s.last_message_id), "Your light dims.");
    });
}

#[test]
fn execute_attack_drain_charges_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    with_state_mut(|s| {
        for i in 0..umoria::inventory::PLAYER_INVENTORY_SIZE as usize {
            inventory_item_copy_to(OBJ_NOTHING as i16, &mut s.py.inventory[i]);
        }
        s.py.inventory[0] = Inventory {
            category_id: TV_WAND,
            sub_category_id: ITEM_SINGLE_STACK_MIN,
            items_count: 1,
            misc_use: 5,
            ..Default::default()
        };
        s.py.pack.unique_items = 1;
    });
    let mut hp = 10i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    // creature_level 3 → monster_hp += 3 * 5 = 15 → 25
    let noticed = execute_attack_on_player(3, &mut hp, 2, 24, 0, &death, true);
    assert!(noticed);
    assert_eq!(hp, 25);
    with_state(|s| {
        assert_eq!(s.py.inventory[0].misc_use, 0);
        assert_eq!(
            message_text(s.last_message_id),
            "Energy drains from your pack!"
        );
    });
}

#[test]
fn execute_attack_repel_type_noticed_false() {
    reset_for_new_game(None);
    setup_player_for_combat();
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    let noticed = execute_attack_on_player(5, &mut hp, 2, 99, 10, &death, true);
    assert!(!noticed);
    with_state(|s| assert_eq!(s.py.misc.current_hp, 500));
}

// ---------------------------------------------------------------------------
// 3. monster_confuse_on_attack parity
// ---------------------------------------------------------------------------
#[test]
fn monster_confuse_on_attack_confuses_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    with_state_mut(|s| s.py.flags.confuse_monster = true);
    let creature = &CREATURES_LIST[URCHIN_ID as usize];
    let mut name = [0u8; MORIA_MESSAGE_SIZE];
    set_it_prefix(&mut name);
    let mut confused = 0u8;
    monster_confuse_on_attack(creature, &mut confused, 1, &name, true, URCHIN_ID);
    assert_eq!(confused, 3);
    with_state(|s| {
        assert!(message_text(s.last_message_id).contains("appears confused."));
        assert!(!s.py.flags.confuse_monster);
    });
    assert_eq!(next_random_pair(40), (40, 2));
}

#[test]
fn monster_confuse_on_attack_no_sleep_unaffected_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    with_state_mut(|s| s.py.flags.confuse_monster = true);
    let mut creature = CREATURES_LIST[GREY_MUSHROOM_ID as usize];
    creature.defenses |= CD_NO_SLEEP;
    let mut name = [0u8; MORIA_MESSAGE_SIZE];
    set_it_prefix(&mut name);
    let mut confused = 0u8;
    monster_confuse_on_attack(&creature, &mut confused, 1, &name, false, GREY_MUSHROOM_ID);
    assert_eq!(confused, 0);
    with_state(|s| {
        assert!(message_text(s.last_message_id).contains("is unaffected."));
    });
}

#[test]
fn monster_confuse_on_attack_skipped_for_repel_desc() {
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    with_state_mut(|s| s.py.flags.confuse_monster = true);
    let creature = &CREATURES_LIST[GREY_MUSHROOM_ID as usize];
    let name = [0u8; MORIA_MESSAGE_SIZE];
    let mut confused = 0u8;
    monster_confuse_on_attack(creature, &mut confused, 99, &name, true, GREY_MUSHROOM_ID);
    with_state(|s| assert!(s.py.flags.confuse_monster));
}

// ---------------------------------------------------------------------------
// 4. monster_print_attack_description parity
// ---------------------------------------------------------------------------
#[test]
fn monster_print_attack_description_static_cases() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_player_for_combat();
    let mut msg = [0u8; MORIA_MESSAGE_SIZE];
    set_it_prefix(&mut msg);
    monster_print_attack_description(&mut msg, 1);
    with_state(|s| assert_eq!(message_text(s.last_message_id), "It hits you."));
    with_state_mut(|s| s.message_ready_to_print = false);
    set_it_prefix(&mut msg);
    monster_print_attack_description(&mut msg, 99);
    with_state(|s| assert_eq!(message_text(s.last_message_id), "It is repelled."));
}

#[test]
fn monster_print_attack_description_insult_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    let mut msg = [0u8; MORIA_MESSAGE_SIZE];
    set_it_prefix(&mut msg);
    monster_print_attack_description(&mut msg, 19);
    with_state(|s| assert_eq!(message_text(s.last_message_id), "It insults your mother!"));
    assert_eq!(next_random_pair(9), (9, 9));
}

#[test]
fn monster_print_attack_description_slimed() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    let mut msg = [0u8; MORIA_MESSAGE_SIZE];
    monster_print_attack_description(&mut msg, 15);
    with_state(|s| assert_eq!(message_text(s.last_message_id), "You've been slimed!"));
}

// ---------------------------------------------------------------------------
// 5. Sustain/immunity branches
// ---------------------------------------------------------------------------
#[test]
fn execute_attack_sustain_str_skips_decrease_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    with_state_mut(|s| s.py.flags.sustain_str = true);
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(5, &mut hp, 2, 2, 10, &death, true);
    with_state(|s| {
        assert_eq!(
            message_text(s.last_message_id),
            "You feel weaker for a moment, but it passes."
        );
        assert_eq!(s.py.stats.current[PlayerAttr::A_STR as usize], 16);
    });
    assert_eq!(next_random_pair(2), (2, 2));
}

#[test]
fn execute_attack_free_action_paralyze_seed1() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(1));
    setup_player_for_combat();
    with_state_mut(|s| s.py.flags.free_action = true);
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 11, 5, &death, true);
    with_state(|s| {
        assert_eq!(message_text(s.last_message_id), "You are unaffected.");
        assert_eq!(s.py.flags.paralysis, 0);
    });
}

#[test]
fn execute_attack_fear_resisted_save_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_player_for_combat();
    with_state_mut(|s| {
        s.py.misc.saving_throw = 100;
        s.py.stats.used[PlayerAttr::A_WIS as usize] = 18;
    });
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 4, 5, &death, true);
    with_state(|s| {
        assert_eq!(message_text(s.last_message_id), "You resist the effects!");
        assert_eq!(s.py.flags.afraid, 0);
    });
}

// ---------------------------------------------------------------------------
// 6. Integer-semantics tests
// ---------------------------------------------------------------------------
#[test]
fn execute_attack_monster_hp_reference_i16_mutated() {
    reset_for_new_game(None);
    setup_player_for_combat();
    let mut monster_hp = 50i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    let _ = execute_attack_on_player(10, &mut monster_hp, 2, 24, 0, &death, true);
    assert_eq!(monster_hp, 50);
}

#[test]
fn execute_attack_steal_gold_zeroes_au_when_theft_exceeds_au_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player_for_combat();
    place_monster(2, GREY_MUSHROOM_ID, 10, Coord_t { y: 10, x: 11 }, true);
    with_state_mut(|s| {
        s.py.misc.au = 5;
        s.py.stats.used[PlayerAttr::A_DEX as usize] = 5;
    });
    let mut hp = 0i16;
    let death = death_description_for(GREY_MUSHROOM_ID);
    execute_attack_on_player(10, &mut hp, 2, 12, 0, &death, true);
    with_state(|s| assert_eq!(s.py.misc.au, 0));
}

#[test]
fn monster_attack_player_recall_attacks_increment_when_noticed() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(1));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player_for_combat();
    with_state_mut(|s| {
        s.py.flags.protect_evil = 0;
        s.creature_recall[URCHIN_ID as usize].attacks[0] = 1;
    });
    place_monster(2, URCHIN_ID, 10, Coord_t { y: 10, x: 10 }, true);
    monster_attack_player(2);
    with_state(|s| {
        assert!(s.creature_recall[URCHIN_ID as usize].attacks[0] >= 2);
    });
}
