//! Store pricing, stock maintenance, and inventory ops

use crate::config::dungeon::objects::OBJ_NOTHING;
use crate::config::identification::{ID_DAMD, ID_STORE_BOUGHT};
use crate::config::stores::{
    STORE_MAX_AUTO_BUY_ITEMS, STORE_MIN_AUTO_SELL_ITEMS, STORE_STOCK_TURN_AROUND,
};
use crate::config::treasure::LEVEL_TOWN_OBJECTS;
use crate::data_store_owners::STORE_OWNERS;
use crate::data_stores::{RACE_GOLD_ADJUSTMENTS, STORE_CHOICES};
use crate::data_treasure::GAME_OBJECTS;
use crate::game::{random_number_state, with_state, with_state_mut, State};
use crate::game_objects::{popt, pusht_state};
use crate::identification::{
    item_set_colorless_as_identified_for_state, spell_item_identified,
    spell_item_identify_and_remove_random_inscription_for_state, MAX_MUSHROOMS,
};
use crate::inventory::{
    inventory_item_copy_to, inventory_item_single_stackable, inventory_item_stackable, Inventory,
    ITEM_GROUP_MIN, ITEM_SINGLE_STACK_MIN,
};
use crate::store::{Store, MAX_STORES, STORE_MAX_DISCRETE_ITEMS, STORE_MAX_ITEM_TYPES};
use crate::treasure::{
    magic_treasure_magical_ability_state, TV_AMULET, TV_BOOTS, TV_BOW, TV_DIGGING, TV_FOOD,
    TV_POTION1, TV_POTION2, TV_RING, TV_SCROLL1, TV_SCROLL2, TV_SLING_AMMO, TV_SOFT_ARMOR,
    TV_SPIKE, TV_STAFF, TV_SWORD, TV_WAND,
};

fn get_weapon_armor_buy_price(item: &Inventory) -> i32 {
    if !spell_item_identified(*item) {
        return GAME_OBJECTS[item.id as usize].cost;
    }

    if item.category_id >= TV_BOW && item.category_id <= TV_SWORD {
        if item.to_hit < 0 || item.to_damage < 0 || item.to_ac < 0 {
            return 0;
        }

        return item.cost + i32::from(item.to_hit + item.to_damage + item.to_ac) * 100;
    }

    if item.to_ac < 0 {
        return 0;
    }

    item.cost + i32::from(item.to_ac) * 100
}

fn get_ammo_buy_price(item: &Inventory) -> i32 {
    if !spell_item_identified(*item) {
        return GAME_OBJECTS[item.id as usize].cost;
    }

    if item.to_hit < 0 || item.to_damage < 0 || item.to_ac < 0 {
        return 0;
    }

    item.cost + i32::from(item.to_hit + item.to_damage + item.to_ac) * 5
}

fn get_potion_scroll_buy_price(state: &State, item: &Inventory) -> i32 {
    if !item_set_colorless_as_identified_for_state(
        state,
        item.category_id,
        item.sub_category_id,
        item.identification,
    ) {
        return 20;
    }

    item.cost
}

fn get_food_buy_price(state: &State, item: &Inventory) -> i32 {
    if item.sub_category_id < ITEM_SINGLE_STACK_MIN + MAX_MUSHROOMS
        && !item_set_colorless_as_identified_for_state(
            state,
            item.category_id,
            item.sub_category_id,
            item.identification,
        )
    {
        return 1;
    }

    item.cost
}

fn get_ring_amulet_buy_price(state: &State, item: &Inventory) -> i32 {
    if !item_set_colorless_as_identified_for_state(
        state,
        item.category_id,
        item.sub_category_id,
        item.identification,
    ) {
        return 45;
    }

    if !spell_item_identified(*item) {
        return GAME_OBJECTS[item.id as usize].cost;
    }

    item.cost
}

fn get_wand_staff_buy_price(state: &State, item: &Inventory) -> i32 {
    if !item_set_colorless_as_identified_for_state(
        state,
        item.category_id,
        item.sub_category_id,
        item.identification,
    ) {
        if item.category_id == TV_WAND {
            return 50;
        }

        return 70;
    }

    if spell_item_identified(*item) {
        return item.cost + (item.cost / 20) * i32::from(item.misc_use);
    }

    item.cost
}

fn get_pick_shovel_buy_price(item: &Inventory) -> i32 {
    if !spell_item_identified(*item) {
        return GAME_OBJECTS[item.id as usize].cost;
    }

    if item.misc_use < 0 {
        return 0;
    }

    let mut value =
        item.cost + i32::from(item.misc_use - GAME_OBJECTS[item.id as usize].misc_use) * 100;

    if value < 0 {
        value = 0;
    }

    value
}

pub(crate) fn store_item_value_for_state(state: &State, item: &Inventory) -> i32 {
    let value = if (item.identification & ID_DAMD) != 0 {
        0
    } else if (item.category_id >= TV_BOW && item.category_id <= TV_SWORD)
        || (item.category_id >= TV_BOOTS && item.category_id <= TV_SOFT_ARMOR)
    {
        get_weapon_armor_buy_price(item)
    } else if item.category_id >= TV_SLING_AMMO && item.category_id <= TV_SPIKE {
        get_ammo_buy_price(item)
    } else if item.category_id == TV_SCROLL1
        || item.category_id == TV_SCROLL2
        || item.category_id == TV_POTION1
        || item.category_id == TV_POTION2
    {
        get_potion_scroll_buy_price(state, item)
    } else if item.category_id == TV_FOOD {
        get_food_buy_price(state, item)
    } else if item.category_id == TV_AMULET || item.category_id == TV_RING {
        get_ring_amulet_buy_price(state, item)
    } else if item.category_id == TV_STAFF || item.category_id == TV_WAND {
        get_wand_staff_buy_price(state, item)
    } else if item.category_id == TV_DIGGING {
        get_pick_shovel_buy_price(item)
    } else {
        item.cost
    };

    if item.sub_category_id > ITEM_GROUP_MIN {
        value * i32::from(item.items_count)
    } else {
        value
    }
}

/// 90
pub fn store_item_value(item: &Inventory) -> i32 {
    with_state(|state| store_item_value_for_state(state, item))
}

fn store_item_sell_price_for_state(
    state: &State,
    store: &Store,
    min_price: &mut i32,
    max_price: &mut i32,
    item: &Inventory,
) -> i32 {
    let mut price = store_item_value_for_state(state, item);

    if item.cost < 1 || price < 1 {
        return 0;
    }

    let owner = &STORE_OWNERS[store.owner_id as usize];

    price = price
        * i32::from(RACE_GOLD_ADJUSTMENTS[owner.race as usize][state.py.misc.race_id as usize])
        / 100;
    if price < 1 {
        price = 1;
    }

    *max_price = price * i32::from(owner.max_inflate) / 100;
    *min_price = price * i32::from(owner.min_inflate) / 100;

    if *min_price > *max_price {
        *min_price = *max_price;
    }

    price
}

/// 219
pub fn store_item_sell_price(
    store: &Store,
    min_price: &mut i32,
    max_price: &mut i32,
    item: &Inventory,
) -> i32 {
    with_state(|state| store_item_sell_price_for_state(state, store, min_price, max_price, item))
}

/// 245
pub fn store_check_player_items_count(store: &Store, item: &Inventory) -> bool {
    if store.unique_items_counter < STORE_MAX_DISCRETE_ITEMS {
        return true;
    }

    if !inventory_item_stackable(*item) {
        return false;
    }

    let mut store_check = false;

    for i in 0..store.unique_items_counter as usize {
        let store_item = &store.inventory[i].item;

        if store_item.category_id == item.category_id
            && store_item.sub_category_id == item.sub_category_id
            && i32::from(store_item.items_count) + i32::from(item.items_count) < 256
            && (item.sub_category_id < ITEM_GROUP_MIN || store_item.misc_use == item.misc_use)
        {
            store_check = true;
        }
    }

    store_check
}

fn store_item_insert(state: &mut State, store_id: i32, pos: i32, i_cost: i32, item: &Inventory) {
    let store = &mut state.stores[store_id as usize];

    for i in (pos..i32::from(store.unique_items_counter)).rev() {
        store.inventory[i as usize + 1] = store.inventory[i as usize];
    }

    store.inventory[pos as usize].item = *item;
    store.inventory[pos as usize].cost = -i_cost;
    store.unique_items_counter += 1;
}

fn store_carry_item_state(state: &mut State, store_id: i32, index_id: &mut i32, item: &Inventory) {
    *index_id = -1;

    let mut item_cost = 0;
    let mut dummy = 0;
    {
        let store = &state.stores[store_id as usize];
        if store_item_sell_price_for_state(state, store, &mut dummy, &mut item_cost, item) < 1 {
            return;
        }
    }

    let item_num = item.items_count;
    let item_category = item.category_id;
    let item_sub_category = item.sub_category_id;
    let item_misc = item.misc_use;

    let mut flag = false;
    let mut item_id = 0;

    while item_id < i32::from(state.stores[store_id as usize].unique_items_counter) && !flag {
        let (store_item_category, store_item_sub_category, store_item_misc) = {
            let store_item = &state.stores[store_id as usize].inventory[item_id as usize].item;
            (
                store_item.category_id,
                store_item.sub_category_id,
                store_item.misc_use,
            )
        };

        if item_category == store_item_category {
            if item_sub_category == store_item_sub_category
                && item_sub_category >= ITEM_SINGLE_STACK_MIN
                && (item_sub_category < ITEM_GROUP_MIN || store_item_misc == item_misc)
            {
                *index_id = item_id;
                state.stores[store_id as usize].inventory[item_id as usize]
                    .item
                    .items_count += item_num;

                if item_sub_category > ITEM_GROUP_MIN {
                    let store = &state.stores[store_id as usize];
                    let store_item =
                        &state.stores[store_id as usize].inventory[item_id as usize].item;
                    let _ = store_item_sell_price_for_state(
                        state,
                        store,
                        &mut dummy,
                        &mut item_cost,
                        store_item,
                    );
                    state.stores[store_id as usize].inventory[item_id as usize].cost = -item_cost;
                } else if state.stores[store_id as usize].inventory[item_id as usize]
                    .item
                    .items_count
                    > 24
                {
                    state.stores[store_id as usize].inventory[item_id as usize]
                        .item
                        .items_count = 24;
                }
                flag = true;
            }
        } else if item_category > store_item_category {
            store_item_insert(state, store_id, item_id, item_cost, item);
            flag = true;
            *index_id = item_id;
        }

        item_id += 1;
    }

    // Becomes last item in list
    // after insert, unique_items_counter has been incremented, so
    // index_id = unique_items_counter - 1 == insert position `pos`.
    if !flag {
        let pos = i32::from(state.stores[store_id as usize].unique_items_counter);
        store_item_insert(state, store_id, pos, item_cost, item);
        *index_id = i32::from(state.stores[store_id as usize].unique_items_counter) - 1;
    }
}

/// 312
pub fn store_carry_item(store_id: i32, index_id: &mut i32, item: &mut Inventory) {
    with_state_mut(|state| store_carry_item_state(state, store_id, index_id, item));
}

fn store_destroy_item_state(state: &mut State, store_id: i32, item_id: i32, only_one_of: bool) {
    let items_count = state.stores[store_id as usize].inventory[item_id as usize]
        .item
        .items_count;
    let single_stackable = inventory_item_single_stackable(
        state.stores[store_id as usize].inventory[item_id as usize].item,
    );

    let number = if single_stackable {
        if only_one_of {
            1u8
        } else {
            random_number_state(state, i32::from(items_count)) as u8
        }
    } else {
        items_count
    };

    let store_item = &mut state.stores[store_id as usize].inventory[item_id as usize].item;

    if number == store_item.items_count {
        let unique = state.stores[store_id as usize].unique_items_counter;
        for i in item_id..i32::from(unique) - 1 {
            state.stores[store_id as usize].inventory[i as usize] =
                state.stores[store_id as usize].inventory[i as usize + 1];
        }
        inventory_item_copy_to(
            OBJ_NOTHING as i16,
            &mut state.stores[store_id as usize].inventory[(unique - 1) as usize].item,
        );
        state.stores[store_id as usize].inventory[(unique - 1) as usize].cost = 0;
        state.stores[store_id as usize].unique_items_counter -= 1;
    } else {
        store_item.items_count -= number;
    }
}

/// 345
pub fn store_destroy_item(store_id: i32, item_id: i32, only_one_of: bool) {
    with_state_mut(|state| store_destroy_item_state(state, store_id, item_id, only_one_of));
}

fn store_item_create(store_id: i32, max_cost: i16) {
    // popt() may compactObjects at capacity
    // Call popt() outside with_state_mut so compact can re-enter global state.
    let free_id = popt();

    with_state_mut(|state| {
        let mut tries = 0;
        while tries <= 3 {
            let choice_index = random_number_state(state, i32::from(STORE_MAX_ITEM_TYPES)) - 1;
            let id = STORE_CHOICES[store_id as usize][choice_index as usize];
            inventory_item_copy_to(id as i16, &mut state.game.treasure.list[free_id as usize]);
            magic_treasure_magical_ability_state(state, free_id, i32::from(LEVEL_TOWN_OBJECTS));

            let item = state.game.treasure.list[free_id as usize];
            let store = state.stores[store_id as usize];

            if store_check_player_items_count(&store, &item)
                && item.cost > 0
                && item.cost < i32::from(max_cost)
            {
                state.game.treasure.list[free_id as usize].identification |= ID_STORE_BOUGHT;
                spell_item_identify_and_remove_random_inscription_for_state(
                    state,
                    free_id as usize,
                );
                let item = state.game.treasure.list[free_id as usize];
                let mut dummy = 0;
                store_carry_item_state(state, store_id, &mut dummy, &item);
                tries = 10;
            }

            tries += 1;
        }

        pusht_state(state, free_id as u8);
    });
}

/// 56
pub fn store_maintenance() {
    for store_id in 0..MAX_STORES as i32 {
        with_state_mut(|state| {
            state.stores[store_id as usize].insults_counter = 0;
        });

        let should_sell = with_state(|state| {
            state.stores[store_id as usize].unique_items_counter >= STORE_MIN_AUTO_SELL_ITEMS
        });
        if should_sell {
            let mut turnaround = with_state_mut(|state| {
                let unique = state.stores[store_id as usize].unique_items_counter;
                let mut turnaround = random_number_state(state, i32::from(STORE_STOCK_TURN_AROUND));
                if unique >= STORE_MAX_AUTO_BUY_ITEMS {
                    turnaround += 1 + i32::from(unique) - i32::from(STORE_MAX_AUTO_BUY_ITEMS);
                }
                turnaround - 1
            });
            while turnaround >= 0 {
                let item_id = with_state_mut(|state| {
                    let counter = i32::from(state.stores[store_id as usize].unique_items_counter);
                    random_number_state(state, counter) - 1
                });
                store_destroy_item(store_id, item_id, false);
                turnaround -= 1;
            }
        }

        let should_buy = with_state(|state| {
            state.stores[store_id as usize].unique_items_counter <= STORE_MAX_AUTO_BUY_ITEMS
        });
        if should_buy {
            let max_cost = with_state(|state| {
                STORE_OWNERS[state.stores[store_id as usize].owner_id as usize].max_cost
            });
            let mut turnaround = with_state_mut(|state| {
                let unique = state.stores[store_id as usize].unique_items_counter;
                let mut turnaround = random_number_state(state, i32::from(STORE_STOCK_TURN_AROUND));
                if unique < STORE_MIN_AUTO_SELL_ITEMS {
                    turnaround += i32::from(STORE_MIN_AUTO_SELL_ITEMS) - i32::from(unique);
                }
                turnaround - 1
            });
            while turnaround >= 0 {
                store_item_create(store_id, max_cost);
                turnaround -= 1;
            }
        }
    }
}
