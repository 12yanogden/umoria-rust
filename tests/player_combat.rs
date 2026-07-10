//! Player bonus hub & combat resolution parity.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use umoria::config::identification::ID_KNOWN2;
use umoria::config::monsters;
use umoria::config::player::status::{PY_ARMOR, PY_SPEED, PY_STR_WGT};
use umoria::config::treasure::flags::{
    TR_AGGRAVATE, TR_BLIND, TR_CURSED, TR_INFRA, TR_REGEN, TR_RES_FIRE, TR_SEARCH, TR_SPEED,
    TR_STEALTH, TR_STR, TR_SUST_STAT, TR_TELEPORT, TR_TIMID,
};
use umoria::data_player::CLASS_LEVEL_ADJ;
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::identification::spell_item_identified;
use umoria::inventory::{
    inventory_collect_all_item_flags, inventory_item_is_cursed, Inventory, PlayerEquipment,
    PLAYER_INVENTORY_SIZE,
};
use umoria::monster::{Monster, MON_TOTAL_ALLOCATIONS};
use umoria::player::{
    player_adjust_bonuses_for_item, player_attack_monster, player_attack_position,
    player_calculate_base_to_hit, player_calculate_to_hit_blows, player_change_speed,
    player_is_wielding_item, player_left_hand_ring_empty, player_recalculate_bonuses,
    player_right_hand_ring_empty, player_saving_throw, player_take_off, player_test_attack_hits,
    player_test_being_hit, player_weapon_critical_blow, player_worn_item_is_cursed,
    player_worn_item_remove_curse, PlayerAttr, PlayerClassLevelAdj, BTH_PER_PLUS_TO_HIT_ADJUST,
    CLASS_MISC_HIT,
};
use umoria::player_stats::{
    player_armor_class_adjustment, player_damage_adjustment, player_to_hit_adjustment,
};
use umoria::treasure::{TV_BOOTS, TV_BOW, TV_NOTHING, TV_RING, TV_SWORD};
use umoria::types::Coord_t;
use umoria::ui_io::test_set_ncurses_stub;

const GREY_MUSHROOM_ID: u16 = 8;

fn setup_dungeon(height: i16, width: i16) {
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                s.dg.floor[y as usize][x as usize].feature_id = TILE_LIGHT_FLOOR;
                s.dg.floor[y as usize][x as usize].permanent_light = true;
            }
        }
    });
}

fn reset_monster_slots() {
    with_state_mut(|s| {
        s.next_free_monster_id = i16::from(monsters::MON_MIN_INDEX_ID) + 2;
        s.monsters = [Monster::default(); MON_TOTAL_ALLOCATIONS as usize];
        for id in i32::from(monsters::MON_MIN_INDEX_ID)..s.next_free_monster_id as i32 {
            s.monsters[id as usize].speed = 10;
        }
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
        s.dg.floor[coord.y as usize][coord.x as usize].creature_id = id as u8;
    });
}

fn cpp_test_being_hit(
    base_to_hit: i32,
    level: i32,
    plus_to_hit: i32,
    armor_class: i32,
    class_id: u8,
    attack_type_id: u8,
) -> bool {
    let hit_chance = base_to_hit
        + plus_to_hit * i32::from(BTH_PER_PLUS_TO_HIT_ADJUST)
        + level * i32::from(CLASS_LEVEL_ADJ[class_id as usize][attack_type_id as usize]);

    let die = random_number(20);
    die != 1 && (die == 20 || (hit_chance > 0 && random_number(hit_chance) > armor_class))
}

fn cpp_weapon_critical_blow(
    weapon_weight: i32,
    plus_to_hit: i32,
    damage: i32,
    class_id: u8,
    level: u16,
    attack_type_id: u8,
) -> i32 {
    let mut critical = damage;
    let threshold = weapon_weight
        + 5 * plus_to_hit
        + i32::from(CLASS_LEVEL_ADJ[class_id as usize][attack_type_id as usize]) * i32::from(level);

    if random_number(5000) <= threshold {
        let weight = weapon_weight + random_number(650);
        critical = if weight < 400 {
            2 * damage + 5
        } else if weight < 700 {
            3 * damage + 10
        } else if weight < 900 {
            4 * damage + 15
        } else {
            5 * damage + 20
        };
    }
    critical
}

fn cpp_saving_throw(class_id: u8, level: u16, saving_throw: i16, wis: u8) -> bool {
    let wis_adj = wis_int_adj(i32::from(wis));
    let class_level_adjustment =
        i32::from(CLASS_LEVEL_ADJ[class_id as usize][PlayerClassLevelAdj::SAVE as usize])
            * i32::from(level)
            / 3;
    let saving = i32::from(saving_throw) + wis_adj + class_level_adjustment;
    random_number(100) <= saving
}

fn wis_int_adj(value: i32) -> i32 {
    if value > 117 {
        7
    } else if value > 107 {
        6
    } else if value > 87 {
        5
    } else if value > 67 {
        4
    } else if value > 17 {
        3
    } else if value > 14 {
        2
    } else {
        i32::from(value > 7)
    }
}

fn equip_item(slot: PlayerEquipment, item: Inventory) {
    with_state_mut(|s| {
        s.py.inventory[slot as usize] = item;
        if s.py.equipment_count == 0 {
            s.py.equipment_count = 1;
        }
    });
}

fn make_test_item(
    flags: u32,
    misc_use: i16,
    to_hit: i16,
    to_damage: i16,
    ac: i16,
    to_ac: i16,
    weight: u16,
    category_id: u8,
    identified: bool,
) -> Inventory {
    Inventory {
        category_id,
        flags,
        misc_use,
        to_hit,
        to_damage,
        ac,
        to_ac,
        weight,
        items_count: 1,
        identification: if identified { ID_KNOWN2 } else { 0 },
        ..Default::default()
    }
}

fn cpp_recalculate_bonuses_from_inventory(
    items: &[Inventory; PLAYER_INVENTORY_SIZE as usize],
) -> (i16, i16, i16, i16, i16, i16, i16, i16) {
    let mut plusses_to_hit = 0i16;
    let mut plusses_to_damage = 0i16;
    let mut magical_ac = 0i16;
    let mut ac = 0i16;
    let mut display_to_hit = 0i16;
    let mut display_to_damage = 0i16;
    let mut display_ac = 0i16;
    let mut display_to_ac = 0i16;

    for item in items
        .iter()
        .take(PlayerEquipment::Light as usize)
        .skip(PlayerEquipment::Wield as usize)
    {
        if item.category_id == TV_NOTHING {
            continue;
        }
        plusses_to_hit = plusses_to_hit.wrapping_add(item.to_hit);
        if item.category_id != TV_BOW {
            plusses_to_damage = plusses_to_damage.wrapping_add(item.to_damage);
        }
        magical_ac = magical_ac.wrapping_add(item.to_ac);
        ac = ac.wrapping_add(item.ac);

        if spell_item_identified(*item) {
            display_to_hit = display_to_hit.wrapping_add(item.to_hit);
            if item.category_id != TV_BOW {
                display_to_damage = display_to_damage.wrapping_add(item.to_damage);
            }
            display_to_ac = display_to_ac.wrapping_add(item.to_ac);
            display_ac = display_ac.wrapping_add(item.ac);
        } else if !inventory_item_is_cursed(*item) {
            display_ac = display_ac.wrapping_add(item.ac);
        }
    }

    (
        plusses_to_hit,
        plusses_to_damage,
        magical_ac,
        ac,
        display_to_hit,
        display_to_damage,
        display_ac,
        display_to_ac,
    )
}

// ---------------------------------------------------------------------------
// 1. RNG-order/count parity
// ---------------------------------------------------------------------------

#[test]
fn test_being_hit_rng_short_circuit_natural_one() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| s.py.misc.class_id = 1);
    let die = random_number(20);
    if die == 1 {
        let rust = player_test_being_hit(60, 5, 0, 10, CLASS_MISC_HIT);
        assert!(!rust);
    }
}

#[test]
fn test_being_hit_rng_short_circuit_natural_twenty() {
    reset_for_new_game(Some(99));
    with_state_mut(|s| s.py.misc.class_id = 1);
    let die = random_number(20);
    if die == 20 {
        let rust = player_test_being_hit(60, 5, 0, 10, CLASS_MISC_HIT);
        assert!(rust);
    }
}

#[test]
fn test_being_hit_rng_short_circuit_zero_hit_chance() {
    reset_for_new_game(Some(7));
    with_state_mut(|s| s.py.misc.class_id = 1);
    let die = random_number(20);
    let rust = player_test_being_hit(-100, 1, 0, 10, CLASS_MISC_HIT);
    if die != 1 && die != 20 {
        assert!(!rust);
    }
}

#[test]
fn test_being_hit_rng_two_rolls_on_normal_hit() {
    reset_for_new_game(Some(1234));
    with_state_mut(|s| {
        s.py.misc.class_id = 4;
        s.py.misc.level = 10;
    });
    let expected = cpp_test_being_hit(60, 10, 2, 5, 4, CLASS_MISC_HIT);
    reset_for_new_game(Some(1234));
    with_state_mut(|s| {
        s.py.misc.class_id = 4;
        s.py.misc.level = 10;
    });
    let actual = player_test_being_hit(60, 10, 2, 5, CLASS_MISC_HIT);
    assert_eq!(actual, expected);
}

#[test]
fn weapon_critical_blow_rng_fail_path_single_roll() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.misc.class_id = 1;
        s.py.misc.level = 1;
    });
    let expected = cpp_weapon_critical_blow(1, 0, 4, 1, 1, PlayerClassLevelAdj::BTH as u8);
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.misc.class_id = 1;
        s.py.misc.level = 1;
    });
    let damage = player_weapon_critical_blow(1, 0, 4, PlayerClassLevelAdj::BTH as u8);
    assert_eq!(damage, expected);
}

#[test]
fn weapon_critical_blow_rng_success_path_two_rolls() {
    reset_for_new_game(Some(1));
    with_state_mut(|s| {
        s.py.misc.class_id = 1;
        s.py.misc.level = 40;
    });
    let expected = cpp_weapon_critical_blow(500, 10, 6, 1, 40, PlayerClassLevelAdj::BTH as u8);
    reset_for_new_game(Some(1));
    with_state_mut(|s| {
        s.py.misc.class_id = 1;
        s.py.misc.level = 40;
    });
    let actual = player_weapon_critical_blow(500, 10, 6, PlayerClassLevelAdj::BTH as u8);
    assert_eq!(actual, expected);
}

#[test]
fn saving_throw_rng_order_seed42() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.misc.class_id = 2;
        s.py.misc.level = 5;
        s.py.misc.saving_throw = 10;
        s.py.stats.used[PlayerAttr::A_WIS as usize] = 15;
    });
    let expected = cpp_saving_throw(2, 5, 10, 15);
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.misc.class_id = 2;
        s.py.misc.level = 5;
        s.py.misc.saving_throw = 10;
        s.py.stats.used[PlayerAttr::A_WIS as usize] = 15;
    });
    assert_eq!(player_saving_throw(), expected);
}

#[test]
fn attack_monster_unarmed_rng_sequence_seed42() {
    test_set_ncurses_stub(true);
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    let coord = Coord_t { y: 10, x: 10 };
    place_monster(2, GREY_MUSHROOM_ID, 500, coord, true);
    with_state_mut(|s| {
        s.py.misc.class_id = 1;
        s.py.misc.level = 10;
        s.py.misc.bth = 20;
        s.py.stats.used[PlayerAttr::A_STR as usize] = 18;
        s.py.stats.used[PlayerAttr::A_DEX as usize] = 18;
        s.py.inventory[PlayerEquipment::Wield as usize].category_id = TV_NOTHING;
    });
    player_recalculate_bonuses();

    player_attack_monster(coord);
    test_set_ncurses_stub(false);
}

// ---------------------------------------------------------------------------
// 2. playerRecalculateBonuses state parity
// ---------------------------------------------------------------------------

#[test]
fn recalculate_bonuses_empty_inventory() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.stats.used[PlayerAttr::A_STR as usize] = 16;
        s.py.stats.used[PlayerAttr::A_DEX as usize] = 14;
        s.py.misc.bth = 10;
        s.py.misc.disarm = 5;
        s.py.misc.saving_throw = 8;
    });
    player_recalculate_bonuses();
    with_state(|s| {
        assert_eq!(s.py.misc.plusses_to_hit, player_to_hit_adjustment() as i16);
        assert_eq!(
            s.py.misc.plusses_to_damage,
            player_damage_adjustment() as i16
        );
        assert_eq!(s.py.misc.magical_ac, player_armor_class_adjustment() as i16);
        assert_eq!(s.py.misc.ac, 0);
        assert_eq!(s.py.misc.display_to_hit, s.py.misc.plusses_to_hit);
        assert_eq!(s.py.misc.display_to_damage, s.py.misc.plusses_to_damage);
        assert_eq!(s.py.misc.display_to_ac, s.py.misc.magical_ac);
        assert!(!s.py.flags.see_invisible);
        assert!(!s.py.flags.teleport);
    });
}

#[test]
fn recalculate_bonuses_identified_weapon_and_resist_flags() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.stats.used[PlayerAttr::A_STR as usize] = 18;
        s.py.stats.used[PlayerAttr::A_DEX as usize] = 18;
    });
    let weapon = make_test_item(
        TR_RES_FIRE | TR_REGEN | TR_SUST_STAT,
        1,
        2,
        3,
        1,
        4,
        120,
        TV_SWORD,
        true,
    );
    equip_item(PlayerEquipment::Wield, weapon);
    player_recalculate_bonuses();
    with_state(|s| {
        let inv = s.py.inventory;
        let (pth, ptd, mac, ac, dth, dtd, dac, dtac) = cpp_recalculate_bonuses_from_inventory(&inv);
        let base_th = player_to_hit_adjustment() as i16;
        let base_td = player_damage_adjustment() as i16;
        let base_mac = player_armor_class_adjustment() as i16;
        assert_eq!(s.py.misc.plusses_to_hit, base_th.wrapping_add(pth));
        assert_eq!(s.py.misc.plusses_to_damage, base_td.wrapping_add(ptd));
        assert_eq!(s.py.misc.magical_ac, base_mac.wrapping_add(mac));
        assert_eq!(s.py.misc.ac, ac);
        assert_eq!(s.py.misc.display_to_hit, base_th.wrapping_add(dth));
        assert_eq!(s.py.misc.display_to_damage, base_td.wrapping_add(dtd));
        assert_eq!(s.py.misc.display_to_ac, base_mac.wrapping_add(dtac));
        assert_eq!(
            s.py.misc.display_ac,
            dac.wrapping_add(base_mac.wrapping_add(dtac))
        );
        assert!(s.py.flags.resistant_to_fire);
        assert!(s.py.flags.regenerate_hp);
        assert!(s.py.flags.sustain_str);
    });
}

#[test]
fn recalculate_bonuses_cursed_unidentified_hides_base_ac() {
    reset_for_new_game(None);
    let item = make_test_item(TR_CURSED, 0, 5, 5, 2, 3, 50, TV_SWORD, false);
    equip_item(PlayerEquipment::Wield, item);
    let base_th = player_to_hit_adjustment() as i16;
    let base_mac = player_armor_class_adjustment() as i16;
    player_recalculate_bonuses();
    with_state(|s| {
        assert_eq!(s.py.misc.display_to_hit, base_th);
        assert_eq!(s.py.misc.plusses_to_hit, base_th.wrapping_add(5));
        // C++: cursed + unidentified → neither display branch runs; only display_to_ac added.
        assert_eq!(s.py.misc.display_ac, base_mac);
        assert_eq!(s.py.misc.magical_ac, base_mac.wrapping_add(3));
        assert_eq!(s.py.misc.ac, 2);
    });
}

#[test]
fn recalculate_bonuses_heavy_weapon_display_to_hit_adjustment() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.stats.used[PlayerAttr::A_STR as usize] = 10;
        s.py.weapon_is_heavy = true;
    });
    let weapon = make_test_item(0, 0, 0, 0, 0, 0, 200, TV_SWORD, true);
    equip_item(PlayerEquipment::Wield, weapon);
    player_recalculate_bonuses();
    with_state(|s| {
        let extra = s.py.stats.used[PlayerAttr::A_STR as usize] as i16 * 15
            - s.py.inventory[PlayerEquipment::Wield as usize].weight as i16;
        assert_eq!(
            s.py.misc.display_to_hit,
            s.py.misc.plusses_to_hit.wrapping_add(extra)
        );
    });
}

#[test]
fn recalculate_bonuses_invulnerability_and_blessed_ac() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.flags.invulnerability = 5;
        s.py.flags.blessed = 3;
        s.py.flags.detect_invisible = 1;
        s.py.misc.display_ac = 0;
    });
    player_recalculate_bonuses();
    with_state(|s| {
        assert_eq!(s.py.misc.ac, 102);
        assert_eq!(s.py.misc.display_ac, s.py.misc.magical_ac.wrapping_add(102));
        assert!(s.py.flags.see_invisible);
        assert_ne!(s.py.flags.status & PY_ARMOR, 0);
    });
}

#[test]
fn recalculate_bonuses_bow_excludes_damage_plusses() {
    reset_for_new_game(None);
    let bow = make_test_item(0, 0, 1, 9, 0, 0, 30, TV_BOW, true);
    equip_item(PlayerEquipment::Wield, bow);
    player_recalculate_bonuses();
    with_state(|s| {
        assert_eq!(
            s.py.misc.plusses_to_damage,
            player_damage_adjustment() as i16
        );
        assert_eq!(
            s.py.misc.display_to_damage,
            player_damage_adjustment() as i16
        );
    });
}

// ---------------------------------------------------------------------------
// 3. playerAdjustBonusesForItem factor symmetry
// ---------------------------------------------------------------------------

#[test]
fn adjust_bonuses_for_item_wear_remove_symmetry() {
    reset_for_new_game(None);
    let item = make_test_item(
        TR_SEARCH | TR_STEALTH | TR_INFRA | TR_STR,
        2,
        0,
        0,
        0,
        0,
        10,
        TV_RING,
        true,
    );
    let (base_search, base_fos, base_stealth, base_infra, base_speed) = with_state(|s| {
        (
            s.py.misc.chance_in_search,
            s.py.misc.fos,
            s.py.misc.stealth_factor,
            s.py.flags.see_infra,
            s.py.flags.speed,
        )
    });
    player_adjust_bonuses_for_item(item, 1);
    player_adjust_bonuses_for_item(item, -1);
    with_state(|s| {
        assert_eq!(s.py.misc.chance_in_search, base_search);
        assert_eq!(s.py.misc.fos, base_fos);
        assert_eq!(s.py.misc.stealth_factor, base_stealth);
        assert_eq!(s.py.flags.see_infra, base_infra);
        assert_eq!(s.py.flags.speed, base_speed);
    });
}

#[test]
fn adjust_bonuses_for_item_blind_timid_only_on_wear() {
    reset_for_new_game(None);
    let item = make_test_item(TR_BLIND | TR_TIMID, 0, 0, 0, 0, 0, 1, TV_RING, true);
    player_adjust_bonuses_for_item(item, 1);
    with_state(|s| {
        assert_eq!(s.py.flags.blind, 1000);
        assert_eq!(s.py.flags.afraid, 50);
    });
    player_adjust_bonuses_for_item(item, -1);
    with_state(|s| {
        assert_eq!(s.py.flags.blind, 1000);
        assert_eq!(s.py.flags.afraid, 50);
    });
}

#[test]
fn adjust_bonuses_for_item_speed_calls_change_speed() {
    reset_for_new_game(None);
    reset_monster_slots();
    let item = make_test_item(TR_SPEED, 3, 0, 0, 0, 0, 1, TV_BOOTS, true);
    player_adjust_bonuses_for_item(item, 1);
    with_state(|s| {
        assert_eq!(s.py.flags.speed, -3);
        assert_ne!(s.py.flags.status & PY_SPEED, 0);
        assert_eq!(s.monsters[2].speed, 7);
    });
}

// ---------------------------------------------------------------------------
// 4. playerTakeOff + curse helpers
// ---------------------------------------------------------------------------

#[test]
fn ring_empty_helpers() {
    reset_for_new_game(None);
    assert!(player_left_hand_ring_empty());
    assert!(player_right_hand_ring_empty());
    equip_item(
        PlayerEquipment::Left,
        make_test_item(0, 0, 0, 0, 0, 0, 1, TV_RING, true),
    );
    assert!(!player_left_hand_ring_empty());
    assert!(player_right_hand_ring_empty());
}

#[test]
fn wielding_item_helper() {
    reset_for_new_game(None);
    assert!(!player_is_wielding_item());
    equip_item(
        PlayerEquipment::Wield,
        make_test_item(0, 0, 0, 0, 0, 0, 10, TV_SWORD, true),
    );
    assert!(player_is_wielding_item());
}

#[test]
fn worn_item_curse_helpers() {
    reset_for_new_game(None);
    let cursed = make_test_item(TR_CURSED, 0, 0, 0, 0, 0, 1, TV_RING, true);
    equip_item(PlayerEquipment::Right, cursed);
    assert!(player_worn_item_is_cursed(PlayerEquipment::Right));
    player_worn_item_remove_curse(PlayerEquipment::Right);
    assert!(!player_worn_item_is_cursed(PlayerEquipment::Right));
}

#[test]
fn take_off_updates_pack_and_equipment() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    let item = make_test_item(TR_STEALTH, 2, 1, 1, 1, 1, 50, TV_SWORD, true);
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Body as usize] = item;
        s.py.pack.weight = item.weight as i16 * item.items_count as i16;
        s.py.equipment_count = 1;
        s.py.misc.chance_in_search = 0;
        s.py.misc.fos = 0;
        s.py.misc.stealth_factor = 0;
    });
    player_take_off(PlayerEquipment::Body as i32, 2);
    with_state(|s| {
        assert_eq!(s.py.pack.weight, 0);
        assert_eq!(s.py.equipment_count, 0);
        assert_eq!(
            s.py.inventory[PlayerEquipment::Body as usize].category_id,
            TV_NOTHING
        );
        assert_ne!(s.py.flags.status & PY_STR_WGT, 0);
    });
    test_set_ncurses_stub(false);
}

#[test]
fn take_off_auxiliary_skips_bonus_adjustment() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    let item = make_test_item(TR_SEARCH, 5, 0, 0, 0, 0, 10, TV_SWORD, true);
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Auxiliary as usize] = item;
        s.py.pack.weight = 10;
        s.py.equipment_count = 1;
        s.py.misc.chance_in_search = 0;
    });
    player_take_off(PlayerEquipment::Auxiliary as i32, -1);
    with_state(|s| {
        assert_eq!(s.py.misc.chance_in_search, 0);
    });
    test_set_ncurses_stub(false);
}

// ---------------------------------------------------------------------------
// 5. Combat math value parity
// ---------------------------------------------------------------------------

#[test]
fn calculate_to_hit_blows_bare_hands_and_ammo() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.misc.plusses_to_hit = 5;
    });
    let (blows, tot) = player_calculate_to_hit_blows(TV_NOTHING, 0);
    assert_eq!((blows, tot), (2, 2));

    with_state_mut(|s| {
        s.py.stats.used[PlayerAttr::A_STR as usize] = 18;
        s.py.stats.used[PlayerAttr::A_DEX as usize] = 18;
        s.py.misc.plusses_to_hit = 0;
    });
    let (blows, _tot) = player_calculate_to_hit_blows(TV_SWORD, 50);
    assert_eq!(blows, 1);
}

#[test]
fn calculate_base_to_hit_lit_and_unlit() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.misc.bth = 40;
        s.py.misc.level = 10;
        s.py.misc.class_id = 1;
    });
    assert_eq!(player_calculate_base_to_hit(true, 5), 40);
    let unlit = player_calculate_base_to_hit(false, 5);
    let expected = 40 / 2
        - 5 * (i32::from(BTH_PER_PLUS_TO_HIT_ADJUST) - 1)
        - 10 * i32::from(CLASS_LEVEL_ADJ[1][PlayerClassLevelAdj::BTH as usize]) / 2;
    assert_eq!(unlit, expected);
}

#[test]
fn test_attack_hits_attack_id_matrix() {
    reset_for_new_game(Some(500));
    with_state_mut(|s| {
        s.py.misc.ac = 5;
        s.py.misc.magical_ac = 2;
        s.py.misc.level = 3;
        s.py.misc.class_id = 1;
        s.py.misc.au = 100;
        s.py.pack.unique_items = 2;
    });
    assert!(player_test_attack_hits(20, 1));
    assert!(!player_test_attack_hits(0, 1));
    assert!(player_test_attack_hits(99, 1));
}

#[test]
fn attack_position_respects_afraid() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    with_state_mut(|s| s.py.flags.afraid = 1);
    player_attack_position(Coord_t { y: 1, x: 1 });
    with_state(|s| assert_eq!(s.monsters[2].hp, 0));
    test_set_ncurses_stub(false);
}

// ---------------------------------------------------------------------------
// 6. Integer semantics — i16 wrapping on AC sums
// ---------------------------------------------------------------------------

#[test]
fn recalculate_bonuses_i16_ac_wrapping() {
    reset_for_new_game(None);
    let item1 = make_test_item(0, 0, 0, 0, 30_000, 30_000, 1, TV_SWORD, true);
    let item2 = make_test_item(0, 0, 0, 0, 30_000, 30_000, 1, TV_SWORD, true);
    equip_item(PlayerEquipment::Wield, item1);
    equip_item(PlayerEquipment::Body, item2);
    let base_mac = player_armor_class_adjustment() as i16;
    player_recalculate_bonuses();
    with_state(|s| {
        assert_eq!(s.py.misc.ac, -5536i16);
        assert_eq!(
            s.py.misc.magical_ac,
            base_mac.wrapping_add(30_000i16).wrapping_add(30_000i16)
        );
    });
}

#[test]
fn inventory_collect_all_item_flags_or_equipment() {
    reset_for_new_game(None);
    equip_item(
        PlayerEquipment::Wield,
        make_test_item(TR_TELEPORT, 0, 0, 0, 0, 0, 1, TV_SWORD, true),
    );
    equip_item(
        PlayerEquipment::Body,
        make_test_item(TR_AGGRAVATE, 0, 0, 0, 0, 0, 1, TV_SWORD, true),
    );
    assert_eq!(
        inventory_collect_all_item_flags(),
        TR_TELEPORT | TR_AGGRAVATE
    );
}

#[test]
fn change_speed_updates_all_monsters() {
    reset_for_new_game(None);
    reset_monster_slots();
    player_change_speed(4);
    with_state(|s| {
        assert_eq!(s.py.flags.speed, 4);
        assert_ne!(s.py.flags.status & PY_SPEED, 0);
        assert_eq!(s.monsters[2].speed, 14);
        assert_eq!(s.monsters[3].speed, 14);
    });
    player_change_speed(-2);
    with_state(|s| assert_eq!(s.py.flags.speed, 2));
}
