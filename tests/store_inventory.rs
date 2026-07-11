//! `store_inventory` parity (pricing, stock maintenance, inventory ops).
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

use umoria::config::identification::{ID_DAMD, ID_KNOWN2, ID_STORE_BOUGHT};
use umoria::config::stores::{
    STORE_MAX_AUTO_BUY_ITEMS, STORE_MIN_AUTO_SELL_ITEMS, STORE_STOCK_TURN_AROUND,
};
use umoria::data_treasure::GAME_OBJECTS;
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::identification::MAX_MUSHROOMS;
use umoria::inventory::{
    inventory_item_copy_to, inventory_item_single_stackable, Inventory, ITEM_GROUP_MIN,
    ITEM_SINGLE_STACK_MIN,
};
use umoria::store::{Store, MAX_STORES, STORE_MAX_DISCRETE_ITEMS};
use umoria::store_inventory::{
    store_carry_item, store_check_player_items_count, store_destroy_item, store_item_sell_price,
    store_item_value, store_maintenance,
};
use umoria::treasure::{
    TV_ARROW, TV_BOOTS, TV_DIGGING, TV_FOOD, TV_HARD_ARMOR, TV_RING, TV_SCROLL1, TV_STAFF,
    TV_SWORD, TV_WAND,
};

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

fn find_object(category_id: u8, sub_category_id: Option<u8>) -> i16 {
    GAME_OBJECTS
        .iter()
        .position(|obj| {
            obj.category_id == category_id
                && sub_category_id.map_or(true, |sub| obj.sub_category_id == sub)
        })
        .unwrap() as i16
}

fn make_item_from_object(object_id: i16) -> Inventory {
    let mut item = Inventory::default();
    inventory_item_copy_to(object_id, &mut item);
    item
}

fn store_snapshot(store_id: usize) -> (u8, Vec<(u8, u8, u8, i32)>) {
    with_state(|s| {
        let store = &s.stores[store_id];
        let mut items = Vec::new();
        for i in 0..store.unique_items_counter as usize {
            let rec = &store.inventory[i];
            items.push((
                rec.item.category_id,
                rec.item.sub_category_id,
                rec.item.items_count,
                rec.cost,
            ));
        }
        (store.unique_items_counter, items)
    })
}

fn set_all_stores_same(owner_id: u8, unique: u8, item: Inventory, cost: i32) {
    with_state_mut(|s| {
        s.game.treasure.current_id = 1;
        for store in &mut s.stores {
            store.owner_id = owner_id;
            store.unique_items_counter = unique;
            for i in 0..unique as usize {
                store.inventory[i].item = item;
                store.inventory[i].cost = cost;
            }
        }
    });
}

// --------------------------------------------------------------------------
// 4. storeItemValue per-category parity
// --------------------------------------------------------------------------

#[test]
fn store_item_value_cursed_is_zero() {
    reset_for_new_game(Some(1));
    let mut item = make_item_from_object(find_object(TV_SWORD, None));
    item.identification = ID_DAMD;
    assert_eq!(store_item_value(&item), 0);
}

#[test]
fn store_item_value_weapon_identified_plus_math() {
    reset_for_new_game(Some(1));
    let mut item = make_item_from_object(find_object(TV_SWORD, None));
    item.cost = 400;
    item.to_hit = 2;
    item.to_damage = 3;
    item.to_ac = 1;
    item.identification = ID_KNOWN2;
    assert_eq!(store_item_value(&item), 1000);
}

#[test]
fn store_item_value_weapon_negative_plus_is_zero() {
    reset_for_new_game(Some(1));
    let mut item = make_item_from_object(find_object(TV_SWORD, None));
    item.cost = 400;
    item.to_hit = -1;
    item.identification = ID_KNOWN2;
    assert_eq!(store_item_value(&item), 0);
}

#[test]
fn store_item_value_armor_unidentified_uses_template_cost() {
    reset_for_new_game(Some(1));
    let object_id = find_object(TV_HARD_ARMOR, None);
    let item = make_item_from_object(object_id);
    assert_eq!(
        store_item_value(&item),
        GAME_OBJECTS[object_id as usize].cost
    );
}

#[test]
fn store_item_value_armor_identified_to_ac_times_100() {
    reset_for_new_game(Some(1));
    let mut item = make_item_from_object(find_object(TV_BOOTS, None));
    item.cost = 200;
    item.to_ac = 3;
    item.identification = ID_KNOWN2;
    assert_eq!(store_item_value(&item), 500);
}

#[test]
fn store_item_value_ammo_plus_times_five() {
    reset_for_new_game(Some(1));
    let mut item = make_item_from_object(find_object(TV_ARROW, None));
    item.cost = 10;
    item.to_hit = 1;
    item.to_damage = 1;
    item.to_ac = 1;
    item.identification = ID_KNOWN2;
    assert_eq!(store_item_value(&item), 25);
}

#[test]
fn store_item_value_scroll_colorless_is_twenty() {
    reset_for_new_game(Some(1));
    let item = make_item_from_object(find_object(TV_SCROLL1, Some(64)));
    assert_eq!(store_item_value(&item), 20);
}

#[test]
fn store_item_value_food_unidentified_mushroom_is_one() {
    reset_for_new_game(Some(1));
    let item = make_item_from_object(find_object(TV_FOOD, Some(64)));
    assert_eq!(store_item_value(&item), 1);
}

#[test]
fn store_item_value_ring_unknown_is_forty_five() {
    reset_for_new_game(Some(1));
    let item = make_item_from_object(find_object(TV_RING, Some(0)));
    assert_eq!(store_item_value(&item), 45);
}

#[test]
fn store_item_value_ring_known_unidentified_uses_template_cost() {
    reset_for_new_game(Some(1));
    let object_id = find_object(TV_RING, Some(0));
    let mut item = make_item_from_object(object_id);
    item.identification = ID_STORE_BOUGHT;
    assert_eq!(
        store_item_value(&item),
        GAME_OBJECTS[object_id as usize].cost
    );
}

#[test]
fn store_item_value_wand_unknown_is_fifty() {
    reset_for_new_game(Some(1));
    let item = make_item_from_object(find_object(TV_WAND, Some(0)));
    assert_eq!(store_item_value(&item), 50);
}

#[test]
fn store_item_value_staff_unknown_is_seventy() {
    reset_for_new_game(Some(1));
    let item = make_item_from_object(find_object(TV_STAFF, Some(0)));
    assert_eq!(store_item_value(&item), 70);
}

#[test]
fn store_item_value_wand_identified_charge_bonus_truncates() {
    reset_for_new_game(Some(1));
    let mut item = make_item_from_object(find_object(TV_WAND, Some(0)));
    item.cost = 100;
    item.misc_use = 7;
    item.identification = ID_STORE_BOUGHT | ID_KNOWN2;
    assert_eq!(store_item_value(&item), 135);
}

#[test]
fn store_item_value_digging_negative_misc_use_is_zero() {
    reset_for_new_game(Some(1));
    let mut item = make_item_from_object(find_object(TV_DIGGING, None));
    item.misc_use = -1;
    item.identification = ID_KNOWN2;
    assert_eq!(store_item_value(&item), 0);
}

#[test]
fn store_item_value_digging_plus_from_misc_delta() {
    reset_for_new_game(Some(1));
    let object_id = find_object(TV_DIGGING, None);
    let mut item = make_item_from_object(object_id);
    item.cost = 300;
    item.misc_use = GAME_OBJECTS[object_id as usize].misc_use + 2;
    item.identification = ID_KNOWN2;
    assert_eq!(store_item_value(&item), 500);
}

#[test]
fn store_item_value_default_uses_item_cost() {
    reset_for_new_game(Some(1));
    let item = Inventory {
        category_id: 99,
        cost: 123,
        ..Inventory::default()
    };
    assert_eq!(store_item_value(&item), 123);
}

#[test]
fn store_item_value_group_stack_multiplies_by_count() {
    reset_for_new_game(Some(1));
    let mut item = make_item_from_object(find_object(TV_ARROW, None));
    item.sub_category_id = ITEM_GROUP_MIN + 1;
    item.items_count = 5;
    item.cost = 10;
    item.identification = ID_KNOWN2;
    assert_eq!(store_item_value(&item), 50);
}

// --------------------------------------------------------------------------
// 5. storeItemSellPrice parity
// --------------------------------------------------------------------------

#[test]
fn store_item_sell_price_zero_when_cursed_or_valueless() {
    reset_for_new_game(Some(1));
    let store = Store::default();
    let mut item = make_item_from_object(find_object(TV_SWORD, None));
    item.cost = 0;
    let mut min = 0;
    let mut max = 0;
    assert_eq!(store_item_sell_price(&store, &mut min, &mut max, &item), 0);

    item.cost = 100;
    item.identification = ID_DAMD;
    assert_eq!(store_item_sell_price(&store, &mut min, &mut max, &item), 0);
}

#[test]
fn store_item_sell_price_race_gold_and_inflation_seed42() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| s.py.misc.race_id = 0);
    let store = Store {
        owner_id: 0,
        ..Store::default()
    };
    let mut item = make_item_from_object(find_object(TV_SWORD, None));
    item.cost = 100;
    item.identification = ID_KNOWN2;
    let mut min = 0;
    let mut max = 0;
    let price = store_item_sell_price(&store, &mut min, &mut max, &item);
    assert_eq!(price, 100);
    assert_eq!(max, 175);
    assert_eq!(min, 108);
}

#[test]
fn store_item_sell_price_clamps_race_adjust_below_one() {
    reset_for_new_game(Some(1));
    with_state_mut(|s| s.py.misc.race_id = 0);
    let store = Store {
        owner_id: 0,
        ..Store::default()
    };
    let item = Inventory {
        category_id: 99,
        cost: 1,
        ..Inventory::default()
    };
    let mut min = 0;
    let mut max = 0;
    let price = store_item_sell_price(&store, &mut min, &mut max, &item);
    assert_eq!(price, 1);
}

#[test]
fn store_item_sell_price_min_inflate_clamped_to_max() {
    reset_for_new_game(Some(1));
    with_state_mut(|s| s.py.misc.race_id = 0);
    let store = Store {
        owner_id: 0,
        ..Store::default()
    };
    let item = Inventory {
        category_id: 99,
        cost: 1,
        ..Inventory::default()
    };
    let mut min = 999;
    let mut max = 1;
    let _ = store_item_sell_price(&store, &mut min, &mut max, &item);
    assert_eq!(min, max);
}

// --------------------------------------------------------------------------
// 6. storeCheckPlayerItemsCount parity
// --------------------------------------------------------------------------

#[test]
fn store_check_player_items_count_room_available() {
    let store = Store::default();
    let item = Inventory::default();
    assert!(store_check_player_items_count(&store, &item));
}

#[test]
fn store_check_player_items_count_full_non_stackable_false() {
    let mut store = Store {
        unique_items_counter: STORE_MAX_DISCRETE_ITEMS,
        ..Store::default()
    };
    store.inventory[0].item.category_id = TV_SWORD;
    store.inventory[0].item.sub_category_id = 1;
    let item = store.inventory[0].item;
    assert!(!store_check_player_items_count(&store, &item));
}

#[test]
fn store_check_player_items_count_full_stackable_match() {
    let mut store = Store {
        unique_items_counter: STORE_MAX_DISCRETE_ITEMS,
        ..Store::default()
    };
    store.inventory[0].item.category_id = TV_FOOD;
    store.inventory[0].item.sub_category_id = ITEM_SINGLE_STACK_MIN;
    store.inventory[0].item.items_count = 10;
    let mut item = store.inventory[0].item;
    item.items_count = 20;
    assert!(store_check_player_items_count(&store, &item));
}

#[test]
fn store_check_player_items_count_group_requires_misc_match() {
    let mut store = Store {
        unique_items_counter: STORE_MAX_DISCRETE_ITEMS,
        ..Store::default()
    };
    store.inventory[0].item.category_id = TV_ARROW;
    store.inventory[0].item.sub_category_id = ITEM_GROUP_MIN + 1;
    store.inventory[0].item.misc_use = 3;
    store.inventory[0].item.items_count = 10;
    let mut item = store.inventory[0].item;
    item.misc_use = 4;
    assert!(!store_check_player_items_count(&store, &item));
}

// --------------------------------------------------------------------------
// 7. storeCarryItem parity
// --------------------------------------------------------------------------

#[test]
fn store_carry_item_rejects_zero_sell_price() {
    reset_for_new_game(Some(1));
    let mut item = make_item_from_object(find_object(TV_SWORD, None));
    item.cost = 0;
    let mut index = 0;
    store_carry_item(0, &mut index, &mut item);
    assert_eq!(index, -1);
    assert_eq!(store_snapshot(0).0, 0);
}

#[test]
fn store_carry_item_merges_stack_and_caps_at_twenty_four() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.misc.race_id = 0;
        s.stores[0].owner_id = 0;
        s.stores[0].unique_items_counter = 1;
        s.stores[0].inventory[0].item.category_id = TV_FOOD;
        s.stores[0].inventory[0].item.sub_category_id = ITEM_SINGLE_STACK_MIN;
        s.stores[0].inventory[0].item.items_count = 20;
        s.stores[0].inventory[0].item.cost = 50;
        s.stores[0].inventory[0].cost = -100;
    });
    let mut item = Inventory {
        category_id: TV_FOOD,
        sub_category_id: ITEM_SINGLE_STACK_MIN,
        items_count: 10,
        cost: 50,
        ..Inventory::default()
    };
    let mut index = -1;
    store_carry_item(0, &mut index, &mut item);
    assert_eq!(index, 0);
    let count = with_state(|s| s.stores[0].inventory[0].item.items_count);
    assert_eq!(count, 24);
}

#[test]
fn store_carry_item_insert_before_higher_category_and_negative_cost() {
 // Store sorted by descending category: inserting FOOD (80) into a store
 // that already has SWORD (23) hits `item_category > store_item.category_id`
 // and inserts at slot 0.
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.misc.race_id = 0;
        s.stores[0].owner_id = 0;
        s.stores[0].unique_items_counter = 1;
        s.stores[0].inventory[0].item.category_id = TV_SWORD;
        s.stores[0].inventory[0].item.sub_category_id = 1;
        s.stores[0].inventory[0].item.cost = 100;
        s.stores[0].inventory[0].cost = -100;
    });
    let mut item = make_item_from_object(find_object(TV_FOOD, None));
    item.cost = 50;
    item.identification = ID_KNOWN2;
    let mut index = -1;
    store_carry_item(0, &mut index, &mut item);
    assert_eq!(index, 0);
    let (unique, items) = store_snapshot(0);
    assert_eq!(unique, 2);
    assert_eq!(items[0].0, TV_FOOD);
    assert_eq!(items[1].0, TV_SWORD);
    assert!(items[0].3 < 0);
}

#[test]
fn store_carry_item_append_returns_post_insert_index() {
 // after append insert,
 // index_id = unique_items_counter - 1 (the new last slot), not pos - 1.
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.misc.race_id = 0;
        s.stores[0].owner_id = 0;
        s.stores[0].unique_items_counter = 0;
    });
    let mut item = make_item_from_object(find_object(TV_FOOD, None));
    item.cost = 50;
    item.identification = ID_KNOWN2;
    let mut index = -1;
    store_carry_item(0, &mut index, &mut item);
    assert_eq!(index, 0);
    assert_eq!(store_snapshot(0).0, 1);

    let mut item2 = make_item_from_object(find_object(TV_SWORD, None));
    item2.cost = 100;
    item2.identification = ID_KNOWN2;
    let mut index2 = -1;
    store_carry_item(0, &mut index2, &mut item2);
    assert_eq!(index2, 1);
    assert_eq!(store_snapshot(0).0, 2);
}

// --------------------------------------------------------------------------
// 2. storeDestroyItem RNG
// --------------------------------------------------------------------------

#[test]
fn store_destroy_item_single_stackable_partial_seed42() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.stores[0].unique_items_counter = 1;
        s.stores[0].inventory[0].item.category_id = TV_FOOD;
        s.stores[0].inventory[0].item.sub_category_id = ITEM_SINGLE_STACK_MIN;
        s.stores[0].inventory[0].item.items_count = 10;
    });
    assert_eq!(random_number(100), 2);
    store_destroy_item(0, 0, false);
    let count = with_state(|s| s.stores[0].inventory[0].item.items_count);
    assert_eq!(count, 7);
    assert_eq!(next_random_pair(10), (10, 6));
}

#[test]
fn store_destroy_item_only_one_of_removes_one_no_rng() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.stores[0].unique_items_counter = 1;
        s.stores[0].inventory[0].item.category_id = TV_FOOD;
        s.stores[0].inventory[0].item.sub_category_id = ITEM_SINGLE_STACK_MIN;
        s.stores[0].inventory[0].item.items_count = 10;
    });
    assert_eq!(random_number(100), 2);
    store_destroy_item(0, 0, true);
    let count = with_state(|s| s.stores[0].inventory[0].item.items_count);
    assert_eq!(count, 9);
    assert_eq!(random_number(100), 73);
}

#[test]
fn store_destroy_item_non_single_stackable_compacts_slot() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.stores[0].unique_items_counter = 2;
        s.stores[0].inventory[0].item.category_id = TV_SWORD;
        s.stores[0].inventory[0].item.sub_category_id = 1;
        s.stores[0].inventory[0].item.items_count = 1;
        s.stores[0].inventory[1].item.category_id = TV_FOOD;
        s.stores[0].inventory[1].item.sub_category_id = ITEM_SINGLE_STACK_MIN;
        s.stores[0].inventory[1].item.items_count = 3;
    });
    assert_eq!(random_number(100), 2);
    store_destroy_item(0, 0, false);
    let (unique, items) = store_snapshot(0);
    assert_eq!(unique, 1);
    assert_eq!(items[0].0, TV_FOOD);
    assert_eq!(random_number(100), 73);
}

// --------------------------------------------------------------------------
// 1. storeMaintenance RNG-order golden
// --------------------------------------------------------------------------

#[test]
fn store_maintenance_seed42_counter15_all_stores() {
    reset_for_new_game(Some(42));
    let item = Inventory {
        category_id: TV_FOOD,
        sub_category_id: ITEM_SINGLE_STACK_MIN,
        items_count: 5,
        cost: 50,
        ..Inventory::default()
    };
    set_all_stores_same(0, 15, item, -100);

    store_maintenance();
    let counter = with_state(|s| s.stores[0].unique_items_counter);
    assert_eq!(counter, 18);
    assert_eq!(next_random_pair(100), (100, 53));
}

#[test]
fn store_maintenance_buy_only_low_stock_seed42() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.game.treasure.current_id = 1;
        s.stores[0].owner_id = 0;
        s.stores[0].unique_items_counter = 5;
        for id in 1..MAX_STORES as usize {
            s.stores[id].unique_items_counter = 20;
            s.stores[id].owner_id = 0;
            for i in 0..20 {
                s.stores[id].inventory[i].item.category_id = TV_SWORD;
                s.stores[id].inventory[i].item.sub_category_id = 1;
                s.stores[id].inventory[i].item.items_count = 1;
            }
        }
    });
    store_maintenance();
    let counter = with_state(|s| s.stores[0].unique_items_counter);
    assert!(counter >= 5);
}

#[test]
fn store_maintenance_sell_only_high_stock_shrinking_destroy_arg() {
    reset_for_new_game(Some(99));
    with_state_mut(|s| {
        for id in 0..MAX_STORES as usize {
            s.stores[id].owner_id = 0;
            s.stores[id].unique_items_counter = 12;
            for i in 0..12 {
                s.stores[id].inventory[i].item.category_id = TV_FOOD;
                s.stores[id].inventory[i].item.sub_category_id = ITEM_SINGLE_STACK_MIN;
                s.stores[id].inventory[i].item.items_count = 4;
            }
        }
    });
    let before = with_state(|s| s.stores[0].unique_items_counter);
    store_maintenance();
    let after = with_state(|s| s.stores[0].unique_items_counter);
    assert!(after <= before);
}

// --------------------------------------------------------------------------
// 3. storeItemCreate via maintenance buy path
// --------------------------------------------------------------------------

#[test]
fn store_item_create_acceptance_via_empty_store_seed42() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.game.treasure.current_id = 1;
        for id in 0..MAX_STORES as usize {
            s.stores[id].owner_id = 0;
            s.stores[id].unique_items_counter = 0;
            if id != 0 {
                s.stores[id].unique_items_counter = 24;
                for i in 0..24 {
                    s.stores[id].inventory[i].item.category_id = TV_SWORD;
                    s.stores[id].inventory[i].item.sub_category_id = 1;
                    s.stores[id].inventory[i].item.items_count = 1;
                }
            }
        }
    });
    store_maintenance();
    let (unique, _) = store_snapshot(0);
    assert!(unique > 0);
}

// --------------------------------------------------------------------------
// 8. Integer-semantics tests
// --------------------------------------------------------------------------

#[test]
fn store_item_value_wand_charge_bonus_integer_division() {
    reset_for_new_game(Some(1));
    let mut item = make_item_from_object(find_object(TV_WAND, Some(0)));
    item.cost = 99;
    item.misc_use = 1;
    item.identification = ID_STORE_BOUGHT | ID_KNOWN2;
    assert_eq!(store_item_value(&item), 103);
}

#[test]
fn store_check_player_items_count_sum255_boundary() {
    let mut store = Store {
        unique_items_counter: STORE_MAX_DISCRETE_ITEMS,
        ..Store::default()
    };
    store.inventory[0].item.category_id = TV_FOOD;
    store.inventory[0].item.sub_category_id = ITEM_SINGLE_STACK_MIN;
    store.inventory[0].item.items_count = 200;
    let mut item = store.inventory[0].item;
    item.items_count = 56;
    assert!(!store_check_player_items_count(&store, &item));
    item.items_count = 55;
    store.inventory[0].item.items_count = 200;
    item.items_count = 54;
    assert!(store_check_player_items_count(&store, &item));
}

#[test]
fn store_destroy_item_items_count_uint8_wrap_not_used() {
    reset_for_new_game(Some(1));
    with_state_mut(|s| {
        s.stores[0].unique_items_counter = 1;
        s.stores[0].inventory[0].item.category_id = TV_FOOD;
        s.stores[0].inventory[0].item.sub_category_id = ITEM_SINGLE_STACK_MIN;
        s.stores[0].inventory[0].item.items_count = 1;
    });
    store_destroy_item(0, 0, true);
    let unique = with_state(|s| s.stores[0].unique_items_counter);
    assert_eq!(unique, 0);
}

#[test]
fn inventory_item_single_stackable_boundaries_for_destroy() {
    let torch = Inventory {
        sub_category_id: ITEM_GROUP_MIN,
        ..Inventory::default()
    };
    assert!(inventory_item_single_stackable(torch));
    let never = Inventory {
        sub_category_id: ITEM_SINGLE_STACK_MIN - 1,
        ..Inventory::default()
    };
    assert!(!inventory_item_single_stackable(never));
}

#[test]
fn store_maintenance_branch_constants_match_expected() {
    assert_eq!(STORE_MIN_AUTO_SELL_ITEMS, 10);
    assert_eq!(STORE_MAX_AUTO_BUY_ITEMS, 18);
    assert_eq!(STORE_STOCK_TURN_AROUND, 9);
    assert_eq!(MAX_MUSHROOMS, 22);
}
