//! `inventory` parity.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::config::dungeon::objects::OBJ_NOTHING;
use umoria::config::player::status::PY_STR_WGT;
use umoria::config::treasure::flags::{TR_CURSED, TR_RES_ACID};
use umoria::data_treasure::GAME_OBJECTS;
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::inventory::{
    damage_acid, damage_cold, damage_corroding_gas, damage_fire, damage_minus_ac,
    damage_poisoned_gas, execute_disenchant_attack, inventory_can_carry_item,
    inventory_can_carry_item_count, inventory_carry_item, inventory_collect_all_item_flags,
    inventory_damage_item, inventory_destroy_item, inventory_diminish_charges_attack,
    inventory_diminish_light_attack, inventory_drop_item, inventory_find_range,
    inventory_item_copy_to, inventory_item_is_cursed, inventory_item_remove_curse,
    inventory_item_single_stackable, inventory_item_stackable, inventory_take_one_item,
    set_fire_destroyable_items, set_frost_destroyable_items, set_null, Inventory, PlayerEquipment,
    ITEM_GROUP_MIN, ITEM_NEVER_STACK_MAX, ITEM_SINGLE_STACK_MAX, ITEM_SINGLE_STACK_MIN,
    PLAYER_INVENTORY_SIZE,
};
use umoria::player::PlayerAttr;
use umoria::treasure::{
    TV_ARROW, TV_FOOD, TV_HARD_ARMOR, TV_POTION1, TV_SCROLL1, TV_SWORD, TV_WAND,
};
use umoria::types::{Coord_t, MESSAGE_HISTORY_SIZE};
use umoria::ui_io::test_set_ncurses_stub;

fn message_text(id: i16) -> String {
    with_state(|s| {
        let idx = id.rem_euclid(MESSAGE_HISTORY_SIZE as i16) as usize;
        let msg = &s.messages[idx];
        let end = msg.iter().position(|&b| b == 0).unwrap_or(msg.len());
        String::from_utf8_lossy(&msg[..end]).into_owned()
    })
}

fn setup_player_base() {
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.misc.current_hp = 500;
        s.py.misc.max_hp = 500;
        s.py.misc.level = 10;
        s.py.misc.class_id = 1;
        s.py.stats.used[PlayerAttr::A_STR as usize] = 18;
        s.py.misc.weight = 0;
        s.py.pack.heaviness = 0;
        s.py.pack.weight = 0;
        s.py.pack.unique_items = 0;
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.py.flags.blind = 0;
        s.py.flags.resistant_to_fire = false;
        s.py.flags.resistant_to_cold = false;
        s.py.flags.resistant_to_acid = false;
        s.py.flags.resistant_to_light = false;
        s.py.flags.heat_resistance = 0;
        s.py.flags.cold_resistance = 0;
        s.py.flags.poisoned = 0;
        s.py.flags.status = 0;
        s.message_ready_to_print = false;
        s.game.character_is_dead = false;
        for i in 0..PLAYER_INVENTORY_SIZE as usize {
            inventory_item_copy_to(OBJ_NOTHING as i16, &mut s.py.inventory[i]);
        }
    });
}

fn setup_dungeon() {
    with_state_mut(|s| {
        s.dg.height = 20;
        s.dg.width = 20;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
        for y in 1..19 {
            for x in 1..19 {
                s.dg.floor[y][x].feature_id = TILE_LIGHT_FLOOR;
            }
        }
        s.game.treasure.current_id = 1;
    });
}

fn make_item(category_id: u8, sub_category_id: u8, items_count: u8, weight: u16) -> Inventory {
    Inventory {
        category_id,
        sub_category_id,
        items_count,
        weight,
        ..Default::default()
    }
}

fn pack_item(slot: i32, item: Inventory) {
    with_state_mut(|s| {
        s.py.inventory[slot as usize] = item;
        if slot >= s.py.pack.unique_items as i32 {
            s.py.pack.unique_items = (slot + 1) as i16;
        }
    });
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

// ---------------------------------------------------------------------------
// 5. Stacking & slot logic
// ---------------------------------------------------------------------------

#[test]
fn inventory_item_stackable_boundaries() {
    assert!(!inventory_item_stackable(make_item(
        TV_SWORD,
        ITEM_NEVER_STACK_MAX,
        1,
        10
    )));
    assert!(inventory_item_stackable(make_item(
        TV_SWORD,
        ITEM_SINGLE_STACK_MIN,
        1,
        10
    )));
    assert!(inventory_item_stackable(make_item(
        TV_FOOD,
        ITEM_GROUP_MIN,
        1,
        10
    )));

    assert!(!inventory_item_single_stackable(make_item(
        TV_SWORD,
        ITEM_NEVER_STACK_MAX,
        1,
        10
    )));
    assert!(inventory_item_single_stackable(make_item(
        TV_SWORD,
        ITEM_SINGLE_STACK_MIN,
        1,
        10
    )));
    assert!(inventory_item_single_stackable(make_item(
        TV_FOOD,
        ITEM_SINGLE_STACK_MAX,
        1,
        10
    )));
    // sub_category_id 192 is both ITEM_SINGLE_STACK_MAX and ITEM_GROUP_MIN (torch case).
    assert!(inventory_item_single_stackable(make_item(
        TV_FOOD,
        ITEM_GROUP_MIN,
        1,
        10
    )));
}

#[test]
fn inventory_find_range_finds_contiguous_tval_block() {
    reset_for_new_game(Some(1));
    setup_player_base();
    with_state_mut(|s| {
        s.py.pack.unique_items = 5;
        s.py.inventory[0] = make_item(TV_SCROLL1, 64, 1, 5);
        s.py.inventory[1] = make_item(TV_SCROLL1, 65, 1, 5);
        s.py.inventory[2] = make_item(TV_POTION1, 64, 1, 5);
        s.py.inventory[3] = make_item(TV_SCROLL1, 66, 1, 5);
        s.py.inventory[4] = make_item(TV_FOOD, 64, 1, 5);
    });

    let mut start = 0;
    let mut end = 0;
    assert!(inventory_find_range(
        TV_SCROLL1 as i32,
        TV_SCROLL1 as i32,
        &mut start,
        &mut end
    ));
    assert_eq!(start, 0);
    assert_eq!(end, 1);
}

// ---------------------------------------------------------------------------
// 8. Flags & curses
// ---------------------------------------------------------------------------

#[test]
fn inventory_collect_flags_ors_worn_equipment() {
    reset_for_new_game(Some(1));
    setup_player_base();
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Wield as usize].flags = 0x01;
        s.py.inventory[PlayerEquipment::Body as usize].flags = 0x02;
        s.py.inventory[PlayerEquipment::Light as usize].flags = 0x04;
    });
    assert_eq!(inventory_collect_all_item_flags(), 0x03);
}

#[test]
fn inventory_curse_helpers_match_cpp_bits() {
    let mut item = Inventory::default();
    assert!(!inventory_item_is_cursed(item));
    item.flags = TR_CURSED;
    assert!(inventory_item_is_cursed(item));
    inventory_item_remove_curse(&mut item);
    assert!(!inventory_item_is_cursed(item));
    assert_eq!(item.flags, 0);
}

// ---------------------------------------------------------------------------
// 6. Carry / capacity
// ---------------------------------------------------------------------------

#[test]
fn inventory_can_carry_item_count_empty_pack() {
    reset_for_new_game(Some(1));
    setup_player_base();
    let item = make_item(TV_FOOD, ITEM_SINGLE_STACK_MIN, 1, 10);
    assert!(inventory_can_carry_item_count(item));
}

#[test]
fn inventory_carry_item_stacks_and_updates_weight() {
    reset_for_new_game(Some(1));
    setup_player_base();
    let existing = make_item(TV_FOOD, ITEM_SINGLE_STACK_MIN, 2, 5);
    pack_item(0, existing);
    with_state_mut(|s| s.py.pack.weight = 10);

    let incoming = make_item(TV_FOOD, ITEM_SINGLE_STACK_MIN, 3, 5);
    let slot = inventory_carry_item(incoming);
    assert_eq!(slot, 0);
    with_state(|s| {
        assert_eq!(s.py.inventory[0].items_count, 5);
        assert_eq!(s.py.pack.weight, 25);
        assert_eq!(s.py.pack.unique_items, 1);
        assert_ne!(s.py.flags.status & PY_STR_WGT, 0);
    });
}

#[test]
fn inventory_can_carry_item_heaviness_matches_cpp_formula() {
    reset_for_new_game(Some(1));
    setup_player_base();
    let limit = with_state(|s| {
        let mut weight_cap = i32::from(s.py.stats.used[PlayerAttr::A_STR as usize]) * 150
            + i32::from(s.py.misc.weight);
        if weight_cap > 3000 {
            weight_cap = 3000;
        }
        weight_cap
    });
    with_state_mut(|s| {
        s.py.stats.used[PlayerAttr::A_STR as usize] = 10;
        s.py.pack.weight = 500;
        s.py.pack.heaviness = (500 / (limit + 1)) as i16;
    });
    let item = make_item(TV_FOOD, ITEM_SINGLE_STACK_MIN, 1, 100);
    assert!(inventory_can_carry_item(item));
}

// ---------------------------------------------------------------------------
// 7. Destroy / drop / copy / take-one
// ---------------------------------------------------------------------------

#[test]
fn inventory_destroy_item_partial_stack_single_stackable() {
    reset_for_new_game(Some(1));
    setup_player_base();
    let item = make_item(TV_FOOD, ITEM_SINGLE_STACK_MIN, 3, 10);
    pack_item(0, item);
    with_state_mut(|s| s.py.pack.weight = 30);

    inventory_destroy_item(0);
    with_state(|s| {
        assert_eq!(s.py.inventory[0].items_count, 2);
        assert_eq!(s.py.pack.unique_items, 1);
        assert_eq!(s.py.pack.weight, 20);
    });
}

#[test]
fn inventory_destroy_item_compacts_pack() {
    reset_for_new_game(Some(1));
    setup_player_base();
    pack_item(0, make_item(TV_FOOD, ITEM_SINGLE_STACK_MIN, 1, 5));
    pack_item(1, make_item(TV_SCROLL1, ITEM_SINGLE_STACK_MIN, 1, 3));
    pack_item(2, make_item(TV_POTION1, ITEM_SINGLE_STACK_MIN, 1, 4));
    with_state_mut(|s| s.py.pack.weight = 12);

    inventory_destroy_item(1);
    with_state(|s| {
        assert_eq!(s.py.pack.unique_items, 2);
        assert_eq!(s.py.inventory[0].category_id, TV_FOOD);
        assert_eq!(s.py.inventory[1].category_id, TV_POTION1);
        assert_eq!(s.py.pack.weight, 9);
    });
}

#[test]
fn inventory_take_one_item_splits_single_stackable_stack() {
    let from = make_item(TV_FOOD, ITEM_SINGLE_STACK_MIN, 5, 1);
    let mut to = Inventory::default();
    inventory_take_one_item(&mut to, &from);
    assert_eq!(to.items_count, 1);
    assert_eq!(from.items_count, 5);
}

#[test]
fn inventory_item_copy_to_clears_identification() {
    let sword_id = GAME_OBJECTS
        .iter()
        .position(|obj| obj.category_id == TV_SWORD)
        .unwrap() as i16;
    let mut item = Inventory::default();
    inventory_item_copy_to(sword_id, &mut item);
    assert_eq!(item.category_id, TV_SWORD);
    assert_eq!(item.identification, 0);
}

#[test]
fn inventory_drop_item_one_of_stack() {
    reset_for_new_game(Some(42));
    setup_player_base();
    setup_dungeon();
    pack_item(0, make_item(TV_FOOD, ITEM_SINGLE_STACK_MIN, 3, 5));
    with_state_mut(|s| s.py.pack.weight = 15);

    inventory_drop_item(0, false);
    with_state(|s| {
        assert_eq!(s.py.inventory[0].items_count, 2);
        assert_eq!(s.py.pack.weight, 10);
        assert_eq!(s.py.pack.unique_items, 1);
        assert_eq!(s.game.treasure.list[1].items_count, 1);
        assert_eq!(s.dg.floor[10][10].treasure_id, 1);
    });
    assert!(message_text(with_state(|s| s.last_message_id)).starts_with("Dropped "));
}

// ---------------------------------------------------------------------------
// 10. C++ integer semantics
// ---------------------------------------------------------------------------

#[test]
fn inventory_carry_item_items_count_wraps_at_255() {
    reset_for_new_game(Some(1));
    setup_player_base();
    let existing = make_item(TV_FOOD, ITEM_SINGLE_STACK_MIN, 250, 1);
    pack_item(0, existing);
    with_state_mut(|s| s.py.pack.unique_items = PlayerEquipment::Wield as i16);
    let incoming = make_item(TV_FOOD, ITEM_SINGLE_STACK_MIN, 10, 1);
    assert!(!inventory_can_carry_item_count(incoming));
}

#[test]
fn inventory_destroy_item_weight_uses_u16_times_u8() {
    reset_for_new_game(Some(1));
    setup_player_base();
    let mut item = make_item(TV_FOOD, ITEM_GROUP_MIN, 1, 1);
    item.weight = 60_000;
    pack_item(0, item);
    with_state_mut(|s| s.py.pack.weight = 0);

    inventory_destroy_item(0);
    with_state(|s| {
        assert_eq!(s.py.pack.weight, 5536);
    });
}

// ---------------------------------------------------------------------------
// 1. RNG-order golden — inventory_damage_item
// ---------------------------------------------------------------------------

#[test]
fn inventory_damage_item_rng_order_seed_42_frost() {
    reset_for_new_game(Some(42));
    setup_player_base();
    pack_item(0, make_item(TV_POTION1, ITEM_SINGLE_STACK_MIN, 1, 5));
    pack_item(1, make_item(TV_POTION1, ITEM_SINGLE_STACK_MIN, 1, 5));
    pack_item(2, make_item(TV_SWORD, ITEM_NEVER_STACK_MAX, 1, 50));

    let destroyed = inventory_damage_item(set_frost_destroyable_items, 5);
    assert_eq!(destroyed, 1);
    with_state(|s| {
        assert_eq!(s.py.pack.unique_items, 2);
        assert_eq!(s.py.inventory[0].category_id, TV_POTION1);
        assert_eq!(s.py.inventory[1].category_id, TV_SWORD);
    });
    let (max, val) = next_random_pair(100);
    assert_eq!((max, val), (100, 73));
}

#[test]
fn inventory_damage_item_set_null_never_destroys() {
    reset_for_new_game(Some(42));
    setup_player_base();
    pack_item(0, make_item(TV_POTION1, ITEM_SINGLE_STACK_MIN, 1, 5));
    assert_eq!(inventory_damage_item(set_null, 100), 0);
    with_state(|s| assert_eq!(s.py.pack.unique_items, 1));
}

#[test]
fn inventory_damage_item_fire_three_rolls_seed_777() {
    reset_for_new_game(Some(777));
    setup_player_base();
    pack_item(0, make_item(TV_SCROLL1, ITEM_SINGLE_STACK_MIN, 1, 1));
    pack_item(1, make_item(TV_ARROW, ITEM_SINGLE_STACK_MIN, 1, 1));
    pack_item(2, make_item(TV_ARROW, ITEM_SINGLE_STACK_MIN, 1, 1));

    let destroyed = inventory_damage_item(set_fire_destroyable_items, 3);
    assert_eq!(destroyed, 0);
    with_state(|s| assert_eq!(s.py.pack.unique_items, 3));
}

// ---------------------------------------------------------------------------
// 2. damage* handlers
// ---------------------------------------------------------------------------

#[test]
fn damage_fire_applies_resistance_and_pack_message() {
    reset_for_new_game(Some(42));
    setup_player_base();
    pack_item(0, make_item(TV_SCROLL1, ITEM_SINGLE_STACK_MIN, 1, 1));
    let mut label = [0u8; 80];
    label[..4].copy_from_slice(b"fire");
    let hp_before = with_state(|s| s.py.misc.current_hp);

    damage_fire(30, &label);
    with_state(|s| assert_eq!(s.py.misc.current_hp, hp_before - 30));
    assert_eq!(
        message_text(with_state(|s| s.last_message_id)),
        "There is smoke coming from your pack."
    );
}

#[test]
fn damage_cold_with_resistance_reduces_damage() {
    reset_for_new_game(Some(50));
    setup_player_base();
    with_state_mut(|s| {
        s.py.flags.resistant_to_cold = true;
        s.py.flags.cold_resistance = 3;
    });
    pack_item(0, make_item(TV_POTION1, ITEM_SINGLE_STACK_MIN, 1, 1));
    let mut label = [0u8; 80];
    label[..4].copy_from_slice(b"cold");
    let hp_before = with_state(|s| s.py.misc.current_hp);

    damage_cold(27, &label);
    with_state(|s| assert_eq!(s.py.misc.current_hp, hp_before - 3));
}

#[test]
fn damage_poisoned_gas_adds_poison_and_hp_loss() {
    reset_for_new_game(Some(99));
    setup_player_base();
    let mut label = [0u8; 80];
    label[..3].copy_from_slice(b"gas");
    damage_poisoned_gas(10, &label);
    with_state(|s| {
        assert_eq!(s.py.misc.current_hp, 490);
        assert_eq!(s.py.flags.poisoned, 13);
    });
}

#[test]
fn damage_acid_flag_divisor_and_pack_destruction() {
    reset_for_new_game(Some(200));
    setup_player_base();
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Body as usize] = make_item(TV_HARD_ARMOR, 30, 1, 100);
        s.py.inventory[PlayerEquipment::Body as usize].to_ac = 5;
    });
    pack_item(0, make_item(TV_FOOD, ITEM_GROUP_MIN, 1, 1));
    let mut label = [0u8; 80];
    label[..4].copy_from_slice(b"acid");
    damage_acid(20, &label);
    with_state(|s| assert_eq!(s.py.misc.current_hp, 490));
}

// ---------------------------------------------------------------------------
// 3. executeDisenchantAttack
// ---------------------------------------------------------------------------

#[test]
fn execute_disenchant_attack_seed_42_reduces_enchantment() {
    reset_for_new_game(Some(42));
    setup_player_base();
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
            s.py.inventory[slot as usize] = make_item(TV_SWORD, 30, 1, 100);
            s.py.inventory[slot as usize].to_hit = 3;
            s.py.inventory[slot as usize].to_damage = 2;
            s.py.inventory[slot as usize].to_ac = 4;
        }
    });

    assert!(execute_disenchant_attack());
    with_state(|s| {
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
            item.to_hit == 2 && item.to_damage == 0 && item.to_ac == 2
        });
        assert!(reduced);
    });
}

// ---------------------------------------------------------------------------
// 4. Diminish light / charges
// ---------------------------------------------------------------------------

#[test]
fn inventory_diminish_light_attack_seed_42() {
    reset_for_new_game(Some(42));
    setup_player_base();
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Light as usize].misc_use = 1000;
    });

    assert!(inventory_diminish_light_attack(true));
    with_state(|s| {
        assert_eq!(
            s.py.inventory[PlayerEquipment::Light as usize].misc_use,
            548
        );
    });
    assert_eq!(
        message_text(with_state(|s| s.last_message_id)),
        "Your light dims."
    );
}

#[test]
fn inventory_diminish_charges_attack_drains_wand() {
    reset_for_new_game(Some(42));
    setup_player_base();
    pack_item(0, make_item(TV_WAND, ITEM_SINGLE_STACK_MIN, 1, 1));
    with_state_mut(|s| {
        s.py.inventory[0].misc_use = 5;
    });
    let mut monster_hp = 10i16;
    assert!(inventory_diminish_charges_attack(3, &mut monster_hp, true));
    assert_eq!(monster_hp, 25);
    with_state(|s| assert_eq!(s.py.inventory[0].misc_use, 0));
}

// ---------------------------------------------------------------------------
// 9. damageMinusAC
// ---------------------------------------------------------------------------

#[test]
fn damage_minus_ac_damages_armor_seed_42() {
    reset_for_new_game(Some(42));
    setup_player_base();
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Body as usize] = make_item(TV_HARD_ARMOR, 30, 1, 100);
        s.py.inventory[PlayerEquipment::Body as usize].ac = 5;
        s.py.inventory[PlayerEquipment::Body as usize].to_ac = 2;
    });

    assert!(damage_minus_ac(TR_RES_ACID));
    with_state(|s| {
        assert_eq!(s.py.inventory[PlayerEquipment::Body as usize].to_ac, 1);
    });
}

#[test]
fn damage_corroding_gas_calls_minus_ac_and_damage_roll() {
    reset_for_new_game(Some(42));
    setup_player_base();
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Body as usize] = make_item(TV_HARD_ARMOR, 30, 1, 100);
        s.py.inventory[PlayerEquipment::Body as usize].to_ac = 1;
    });
    pack_item(0, make_item(TV_SWORD, ITEM_NEVER_STACK_MAX, 1, 50));
    let mut label = [0u8; 80];
    label[..4].copy_from_slice(b"gas!");
    damage_corroding_gas(&label);
    with_state(|s| assert_eq!(s.py.pack.unique_items, 1));
}
