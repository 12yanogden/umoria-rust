//! Port of src/store.cpp — store interaction, haggling, and command loop.

use std::cell::Cell;
use std::ffi::CStr;

use crate::config::dungeon::objects::OBJ_NOTHING;
use crate::data_store_owners::STORE_OWNERS;
use crate::data_stores::{
    SPEECH_BUYING_HAGGLE as SPEECH_BUYING_HAGGLE_TABLE,
    SPEECH_BUYING_HAGGLE_FINAL as SPEECH_BUYING_HAGGLE_FINAL_TABLE,
    SPEECH_GET_OUT_OF_MY_STORE as SPEECH_GET_OUT_OF_MY_STORE_TABLE,
    SPEECH_HAGGLING_TRY_AGAIN as SPEECH_HAGGLING_TRY_AGAIN_TABLE,
    SPEECH_INSULTED_HAGGLING_DONE as SPEECH_INSULTED_HAGGLING_DONE_TABLE,
    SPEECH_SALE_ACCEPTED as SPEECH_SALE_ACCEPTED_TABLE,
    SPEECH_SELLING_HAGGLE as SPEECH_SELLING_HAGGLE_TABLE,
    SPEECH_SELLING_HAGGLE_FINAL as SPEECH_SELLING_HAGGLE_FINAL_TABLE,
    SPEECH_SORRY as SPEECH_SORRY_TABLE,
};
use crate::game::{random_number, random_number_state, with_state, with_state_mut, State};
use crate::helpers::{insert_number_into_string, string_to_number};
use crate::identification::{
    item_description, item_identify, spell_item_identify_and_remove_random_inscription,
};
use crate::inventory::{
    inventory_can_carry_item_count, inventory_carry_item, inventory_destroy_item,
    inventory_item_single_stackable, inventory_take_one_item, PlayerEquipment,
};
use crate::player::{player_strength, PlayerAttr};
use crate::player_stats::player_stat_adjustment_charisma;
use crate::store_inventory::{
    store_carry_item, store_check_player_items_count, store_destroy_item, store_item_sell_price,
    store_item_value,
};
use crate::treasure::{
    TV_AMULET, TV_ARROW, TV_BOLT, TV_BOOTS, TV_BOW, TV_CLOAK, TV_DIGGING, TV_FLASK, TV_FOOD,
    TV_GLOVES, TV_HAFTED, TV_HARD_ARMOR, TV_HELM, TV_LIGHT, TV_MAGIC_BOOK, TV_POLEARM, TV_POTION1,
    TV_POTION2, TV_PRAYER_BOOK, TV_RING, TV_SCROLL1, TV_SCROLL2, TV_SHIELD, TV_SLING_AMMO,
    TV_SOFT_ARMOR, TV_SPIKE, TV_STAFF, TV_SWORD, TV_WAND,
};
use crate::types::{Vtype_t, MORIA_MESSAGE_SIZE, MORIA_OBJ_DESC_SIZE};
use crate::ui::draw_cave_panel;
use crate::ui_inventory::{inventory_execute_command, inventory_get_input_for_item_id};
use crate::ui_io::terminal::{self, Coord};

pub const MAX_OWNERS: u8 = 18;
pub const MAX_STORES: u8 = 6;
pub const STORE_MAX_DISCRETE_ITEMS: u8 = 24;
pub const STORE_MAX_ITEM_TYPES: u8 = 26;
pub const COST_ADJUSTMENT: u8 = 100;

pub const SPEECH_SALE_ACCEPTED: u8 = 14;
pub const SPEECH_SELLING_HAGGLE_FINAL: u8 = 3;
pub const SPEECH_SELLING_HAGGLE: u8 = 16;
pub const SPEECH_BUYING_HAGGLE_FINAL: u8 = 3;
pub const SPEECH_BUYING_HAGGLE: u8 = 15;
pub const SPEECH_INSULTED_HAGGLING_DONE: u8 = 5;
pub const SPEECH_GET_OUT_OF_MY_STORE: u8 = 5;
pub const SPEECH_HAGGLING_TRY_AGAIN: u8 = 10;
pub const SPEECH_SORRY: u8 = 5;

const SHRT_MAX: u16 = i16::MAX as u16;

thread_local! {
    static STORE_LAST_INCREMENT: Cell<i16> = const { Cell::new(0) };
}

/// Port of `BidState` in store.cpp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BidState {
    Received = 0,
    Rejected,
    Offended,
    Insulted,
}

/// Port of `Owner_t` in store.h.
#[derive(Clone, Copy, Debug, Default)]
pub struct Owner {
    pub name: &'static str,
    pub max_cost: i16,
    pub max_inflate: u8,
    pub min_inflate: u8,
    pub haggles_per: u8,
    pub race: u8,
    pub max_insults: u8,
}

/// Port of `InventoryRecord_t` in store.h.
#[derive(Clone, Copy, Debug, Default)]
pub struct InventoryRecord {
    pub cost: i32,
    pub item: crate::inventory::Inventory,
}

/// Port of `Store_t` in store.h.
#[derive(Clone, Copy, Debug)]
pub struct Store {
    pub turns_left_before_closing: i32,
    pub insults_counter: i16,
    pub owner_id: u8,
    pub unique_items_counter: u8,
    pub good_purchases: u16,
    pub bad_purchases: u16,
    pub inventory: [InventoryRecord; STORE_MAX_DISCRETE_ITEMS as usize],
}

impl Default for Store {
    fn default() -> Self {
        Self {
            turns_left_before_closing: 0,
            insults_counter: 0,
            owner_id: 0,
            unique_items_counter: 0,
            good_purchases: 0,
            bad_purchases: 0,
            inventory: [InventoryRecord::default(); STORE_MAX_DISCRETE_ITEMS as usize],
        }
    }
}

fn store_last_increment_get() -> i16 {
    STORE_LAST_INCREMENT.with(std::cell::Cell::get)
}

fn store_last_increment_set(value: i16) {
    STORE_LAST_INCREMENT.with(|c| c.set(value));
}

#[doc(hidden)]
pub fn test_reset_store_last_increment() {
    store_last_increment_set(0);
}

#[doc(hidden)]
pub fn test_store_last_increment() -> i16 {
    store_last_increment_get()
}

fn c_str_to_bytes(s: &str) -> [u8; MORIA_MESSAGE_SIZE] {
    let mut buf = [0u8; MORIA_MESSAGE_SIZE];
    let bytes = s.as_bytes();
    let len = bytes.len().min(MORIA_MESSAGE_SIZE - 1);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf
}

fn vtype_from_str(s: &str) -> Vtype_t {
    c_str_to_bytes(s)
}

fn print_speech_from_table(table: &[&str], bound: i32) {
    let index = random_number(bound) as usize - 1;
    terminal::print_message(Some(table[index]));
}

/// C++ store.cpp lines 17–35.
pub fn store_initialize_owners() {
    let count = i32::from(MAX_OWNERS / MAX_STORES);

    with_state_mut(|state| {
        for store_id in 0..MAX_STORES as i32 {
            let rn = random_number_state(state, count);
            let store = &mut state.stores[store_id as usize];
            store.owner_id = MAX_STORES * (rn as u8 - 1) + store_id as u8;
            store.insults_counter = 0;
            store.turns_left_before_closing = 0;
            store.unique_items_counter = 0;
            store.good_purchases = 0;
            store.bad_purchases = 0;

            for item in &mut store.inventory {
                crate::inventory::inventory_item_copy_to(OBJ_NOTHING as i16, &mut item.item);
                item.cost = 0;
            }
        }
    });
}

/// C++ store.cpp lines 39–41.
#[doc(hidden)]
pub fn print_speech_finished_haggling() {
    print_speech_from_table(&SPEECH_SALE_ACCEPTED_TABLE, 14);
}

/// C++ store.cpp lines 44–56.
#[doc(hidden)]
pub fn print_speech_selling_haggle(offer: i32, asking: i32, final_flag: i32) {
    let mut comment = if final_flag > 0 {
        vtype_from_str(SPEECH_SELLING_HAGGLE_FINAL_TABLE[random_number(3) as usize - 1])
    } else {
        vtype_from_str(SPEECH_SELLING_HAGGLE_TABLE[random_number(16) as usize - 1])
    };

    insert_number_into_string(&mut comment, b"%A1", offer, false);
    insert_number_into_string(&mut comment, b"%A2", asking, false);
    let msg = CStr::from_bytes_until_nul(&comment)
        .map(|c| c.to_string_lossy().into_owned())
        .unwrap_or_default();
    terminal::print_message(Some(&msg));
}

/// C++ store.cpp lines 58–70.
#[doc(hidden)]
pub fn print_speech_buying_haggle(offer: i32, asking: i32, final_flag: i32) {
    let mut comment = if final_flag > 0 {
        vtype_from_str(SPEECH_BUYING_HAGGLE_FINAL_TABLE[random_number(3) as usize - 1])
    } else {
        vtype_from_str(SPEECH_BUYING_HAGGLE_TABLE[random_number(15) as usize - 1])
    };

    insert_number_into_string(&mut comment, b"%A1", offer, false);
    insert_number_into_string(&mut comment, b"%A2", asking, false);
    let msg = CStr::from_bytes_until_nul(&comment)
        .map(|c| c.to_string_lossy().into_owned())
        .unwrap_or_default();
    terminal::print_message(Some(&msg));
}

/// C++ store.cpp lines 73–77.
#[doc(hidden)]
pub fn print_speech_get_out_of_my_store() {
    let comment = random_number(5) as usize - 1;
    terminal::print_message(Some(SPEECH_INSULTED_HAGGLING_DONE_TABLE[comment]));
    terminal::print_message(Some(SPEECH_GET_OUT_OF_MY_STORE_TABLE[comment]));
}

/// C++ store.cpp lines 79–81.
#[doc(hidden)]
pub fn print_speech_try_again() {
    print_speech_from_table(&SPEECH_HAGGLING_TRY_AGAIN_TABLE, 10);
}

/// C++ store.cpp lines 83–85.
#[doc(hidden)]
pub fn print_speech_sorry() {
    print_speech_from_table(&SPEECH_SORRY_TABLE, 5);
}

fn display_store_commands() {
    terminal::put_string_clear_to_eol("You may:", Coord { y: 20, x: 0 });
    terminal::put_string_clear_to_eol(
        " p) Purchase an item.           b) Browse store's inventory.",
        Coord { y: 21, x: 0 },
    );
    terminal::put_string_clear_to_eol(
        " s) Sell an item.               i/e/t/w/x) Inventory/Equipment Lists.",
        Coord { y: 22, x: 0 },
    );
    terminal::put_string_clear_to_eol(
        "ESC) Exit from Building.        ^R) Redraw the screen.",
        Coord { y: 23, x: 0 },
    );
}

fn display_store_haggle_commands(haggle_type: i32) {
    if haggle_type == -1 {
        terminal::put_string_clear_to_eol(
            "Specify an asking-price in gold pieces.",
            Coord { y: 21, x: 0 },
        );
    } else {
        terminal::put_string_clear_to_eol(
            "Specify an offer in gold pieces.",
            Coord { y: 21, x: 0 },
        );
    }

    terminal::put_string_clear_to_eol("ESC) Quit Haggling.", Coord { y: 22, x: 0 });
    terminal::erase_line(Coord { y: 23, x: 0 });
}

fn display_store_inventory_for_state(state: &mut State, store_id: usize, item_pos_start: i32) {
    let store = &mut state.stores[store_id];
    let mut item_pos_end = ((item_pos_start / 12) + 1) * 12;
    if item_pos_end > i32::from(store.unique_items_counter) {
        item_pos_end = i32::from(store.unique_items_counter);
    }

    let mut item_line_num = item_pos_start % 12;
    let mut item_pos_start = item_pos_start;

    while item_pos_start < item_pos_end {
        let item = &mut store.inventory[item_pos_start as usize].item;
        let current_item_count = item.items_count;

        if inventory_item_single_stackable(*item) {
            item.items_count = 1;
        }

        let mut description = [0u8; MORIA_OBJ_DESC_SIZE as usize];
        item_description(&mut description, *item, true);
        item.items_count = current_item_count;

        let desc = CStr::from_bytes_until_nul(&description)
            .map(|c| c.to_string_lossy().into_owned())
            .unwrap_or_default();
        let msg = format!("{}) {}", (b'a' + item_line_num as u8) as char, desc);
        terminal::put_string_clear_to_eol(
            &msg,
            Coord {
                y: item_line_num + 5,
                x: 0,
            },
        );

        let current_item_count = store.inventory[item_pos_start as usize].cost;
        let price_msg = if current_item_count <= 0 {
            let mut value = -current_item_count;
            value = value * player_stat_adjustment_charisma() / 100;
            if value <= 0 {
                value = 1;
            }
            format!("{value:>9}")
        } else {
            format!("{current_item_count:>9} [Fixed]")
        };

        terminal::put_string_clear_to_eol(
            &price_msg,
            Coord {
                y: item_line_num + 5,
                x: 59,
            },
        );

        item_pos_start += 1;
        item_line_num += 1;
    }

    if item_line_num < 12 {
        for i in 0..=(11 - item_line_num) {
            terminal::erase_line(Coord {
                y: i + item_line_num + 5,
                x: 0,
            });
        }
    }

    if store.unique_items_counter > 12 {
        terminal::put_string("- cont. -", Coord { y: 17, x: 60 });
    } else {
        terminal::erase_line(Coord { y: 17, x: 60 });
    }
}

fn display_single_cost(store_id: i32, item_id: i32) {
    let cost = with_state(|state| state.stores[store_id as usize].inventory[item_id as usize].cost);

    let msg = if cost < 0 {
        let mut c = -cost;
        c = c * player_stat_adjustment_charisma() / 100;
        format!("{c}")
    } else {
        format!("{cost:>9} [Fixed]")
    };
    terminal::put_string_clear_to_eol(
        &msg,
        Coord {
            y: (item_id % 12) + 5,
            x: 59,
        },
    );
}

fn display_player_remaining_gold() {
    let gold = with_state(|state| state.py.misc.au);
    let msg = format!("Gold Remaining : {gold}");
    terminal::put_string_clear_to_eol(&msg, Coord { y: 18, x: 17 });
}

fn display_store(store_id: i32, current_top_item_id: i32) {
    let owner_name =
        with_state(|state| STORE_OWNERS[state.stores[store_id as usize].owner_id as usize].name);

    terminal::clear_screen();
    terminal::put_string(owner_name, Coord { y: 3, x: 9 });
    terminal::put_string("Item", Coord { y: 4, x: 3 });
    terminal::put_string("Asking Price", Coord { y: 4, x: 60 });
    display_player_remaining_gold();
    display_store_commands();
    with_state_mut(|state| {
        display_store_inventory_for_state(state, store_id as usize, current_top_item_id);
    });
}

fn store_get_item_id(
    item_id: &mut i32,
    prompt: &str,
    item_pos_start: i32,
    item_pos_end: i32,
) -> bool {
    *item_id = -1;
    let mut item_found = false;

    let msg = format!(
        "(Items {}-{}, ESC to exit) {}",
        (item_pos_start + b'a' as i32) as u8 as char,
        (item_pos_end + b'a' as i32) as u8 as char,
        prompt
    );

    let mut key_char = 0u8;
    while terminal::get_menu_item_id(&msg, &mut key_char) {
        let adjusted = i32::from(key_char) - i32::from(b'a');
        if adjusted >= item_pos_start && adjusted <= item_pos_end {
            item_found = true;
            *item_id = adjusted;
            break;
        }
        let _ = terminal::terminal_bell_sound();
    }
    terminal::message_line_clear();

    item_found
}

fn store_increase_insults(store_id: i32) -> bool {
    let mut kick = false;
    with_state_mut(|state| {
        let owner_id = state.stores[store_id as usize].owner_id;
        let max_insults = STORE_OWNERS[owner_id as usize].max_insults;

        state.stores[store_id as usize].insults_counter += 1;

        if state.stores[store_id as usize].insults_counter <= i16::from(max_insults) {
            return;
        }

        kick = true;
        state.stores[store_id as usize].insults_counter = 0;
        state.stores[store_id as usize].bad_purchases += 1;
    });

    if kick {
        print_speech_get_out_of_my_store();
        with_state_mut(|state| {
            state.stores[store_id as usize].turns_left_before_closing =
                state.dg.game_turn + 2500 + random_number_state(state, 2500);
        });
    }

    kick
}

fn store_decrease_insults(store_id: i32) {
    with_state_mut(|state| {
        if state.stores[store_id as usize].insults_counter != 0 {
            state.stores[store_id as usize].insults_counter -= 1;
        }
    });
}

fn store_haggle_insults(store_id: i32) -> bool {
    if store_increase_insults(store_id) {
        return true;
    }

    print_speech_try_again();
    terminal::print_message(None);

    false
}

/// C++ store.cpp lines 266–345.
#[doc(hidden)]
pub fn store_get_haggle(prompt: &str, new_offer: &mut i32, offer_count: i32) -> bool {
    let mut valid_offer = true;

    if offer_count == 0 {
        store_last_increment_set(0);
    }

    let mut increment = false;
    let mut adjustment = 0i32;

    let prompt_len = prompt.len() as i32;
    let start_len = prompt_len;

    let mut msg = [0u8; MORIA_MESSAGE_SIZE];

    while valid_offer && adjustment == 0 {
        terminal::put_string_clear_to_eol(prompt, Coord { y: 0, x: 0 });

        let mut prompt_len = prompt_len;
        if offer_count != 0 && store_last_increment_get() != 0 {
            let abs_store_last_increment = i32::from(store_last_increment_get().unsigned_abs());
            let sign = if store_last_increment_get() < 0 {
                '-'
            } else {
                '+'
            };
            let last_offer_str = format!("[{sign}{abs_store_last_increment}] ");
            terminal::put_string_clear_to_eol(&last_offer_str, Coord { y: 0, x: start_len });
            prompt_len = start_len + last_offer_str.len() as i32;
        }

        if !terminal::get_string_input(
            &mut msg,
            Coord {
                y: 0,
                x: prompt_len,
            },
            40,
        ) {
            valid_offer = false;
        }

        let msg_str = CStr::from_bytes_until_nul(&msg)
            .map(|c| c.to_string_lossy().into_owned())
            .unwrap_or_default();

        // C++ store.cpp lines 301–306: skip only ASCII space before +/- detection.
        let bytes = msg_str.as_bytes();
        let mut p = 0usize;
        while p < bytes.len() && bytes[p] == b' ' {
            p += 1;
        }
        if p < bytes.len() && (bytes[p] == b'+' || bytes[p] == b'-') {
            increment = true;
        }

        if offer_count != 0 && increment {
            let _ = string_to_number(&msg_str, &mut adjustment);
            if adjustment == 0 {
                increment = false;
            } else {
                store_last_increment_set(adjustment as i16);
            }
        } else if offer_count != 0 && msg_str.is_empty() {
            adjustment = i32::from(store_last_increment_get());
            increment = true;
        } else {
            let _ = string_to_number(&msg_str, &mut adjustment);
        }

        if valid_offer && offer_count == 0 && increment {
            terminal::print_message(Some("You haven't even made your first offer yet!"));
            adjustment = 0;
            increment = false;
        }
    }

    if valid_offer {
        if increment {
            *new_offer += adjustment;
        } else {
            *new_offer = adjustment;
        }
    } else {
        terminal::message_line_clear();
    }

    valid_offer
}

/// C++ store.cpp lines 356–382.
#[doc(hidden)]
pub fn store_receive_offer(
    store_id: i32,
    prompt: &str,
    new_offer: &mut i32,
    last_offer: i32,
    offer_count: i32,
    factor: i32,
) -> BidState {
    let mut status = BidState::Received;
    let mut done = false;

    while !done {
        if store_get_haggle(prompt, new_offer, offer_count) {
            if *new_offer * factor >= last_offer * factor {
                done = true;
            } else if store_haggle_insults(store_id) {
                status = BidState::Insulted;
                done = true;
            } else {
                *new_offer = last_offer;
            }
        } else {
            status = BidState::Rejected;
            done = true;
        }
    }

    status
}

/// C++ store.cpp lines 384–396.
#[doc(hidden)]
pub fn store_purchase_customer_adjustment(min_sell: &mut i32, max_sell: &mut i32) {
    let charisma = player_stat_adjustment_charisma();

    *max_sell = *max_sell * charisma / 100;
    if *max_sell <= 0 {
        *max_sell = 1;
    }

    *min_sell = *min_sell * charisma / 100;
    if *min_sell <= 0 {
        *min_sell = 1;
    }
}

fn store_no_need_to_bargain(store: &Store, min_price: i32) -> bool {
    if store.good_purchases == SHRT_MAX {
        return true;
    }

    let record = i32::from(store.good_purchases) - 3 * i32::from(store.bad_purchases) - 5;

    record > 0 && record * record > min_price / 50
}

fn store_update_bargaining_skills(store: &mut Store, price: i32, min_price: i32) {
    if min_price < 10 {
        return;
    }

    if price == min_price {
        if store.good_purchases < SHRT_MAX {
            store.good_purchases += 1;
        }
    } else if store.bad_purchases < SHRT_MAX {
        store.bad_purchases += 1;
    }
}

/// C++ store.cpp lines 399–559.
#[doc(hidden)]
pub fn store_purchase_haggle(
    store_id: i32,
    price: &mut i32,
    item: &crate::inventory::Inventory,
) -> BidState {
    let mut status = BidState::Received;
    let mut new_price = 0;

    let (owner_id, store_snapshot) = with_state(|state| {
        (
            state.stores[store_id as usize].owner_id,
            state.stores[store_id as usize],
        )
    });
    let owner = &STORE_OWNERS[owner_id as usize];

    let mut min_sell = 0;
    let mut max_sell = 0;
    let cost = store_item_sell_price(&store_snapshot, &mut min_sell, &mut max_sell, item);

    store_purchase_customer_adjustment(&mut min_sell, &mut max_sell);

    let mut max_buy = cost * (200 - i32::from(owner.max_inflate)) / 100;
    if max_buy <= 0 {
        max_buy = 1;
    }

    display_store_haggle_commands(1);

    let final_asking_price = min_sell;
    let mut current_asking_price = max_sell;

    let mut comment = "Asking";
    let mut accepted_without_haggle = false;
    let mut offers_count = 0;

    if with_state(|state| {
        store_no_need_to_bargain(&state.stores[store_id as usize], final_asking_price)
    }) {
        terminal::print_message(Some(
            "After a long bargaining session, you agree upon the price.",
        ));
        current_asking_price = min_sell;
        comment = "Final offer";
        accepted_without_haggle = true;
        store_last_increment_set(min_sell as i16);
        offers_count = 1;
    }

    let min_offer = max_buy;
    let mut last_offer = min_offer;
    let mut new_offer = 0;

    let min_per = i32::from(owner.haggles_per);
    let max_per = min_per * 3;

    let mut final_flag = 0;
    let mut rejected = false;

    while !rejected {
        loop {
            let mut bidding_open = true;

            let msg = format!("{comment} :  {current_asking_price}");
            terminal::put_string(&msg, Coord { y: 1, x: 0 });

            status = store_receive_offer(
                store_id,
                "What do you offer? ",
                &mut new_offer,
                last_offer,
                offers_count,
                1,
            );

            if status != BidState::Received {
                rejected = true;
                break;
            }

            match new_offer.cmp(&current_asking_price) {
                std::cmp::Ordering::Greater => {
                    print_speech_sorry();
                    new_offer = last_offer;
                    if last_offer + i32::from(store_last_increment_get()) > current_asking_price {
                        store_last_increment_set(0);
                    }
                }
                std::cmp::Ordering::Equal => {
                    rejected = true;
                    new_price = new_offer;
                    break;
                }
                std::cmp::Ordering::Less => {
                    bidding_open = false;
                }
            }

            if rejected || !bidding_open {
                break;
            }
        }

        if rejected {
            break;
        }

        let mut adjustment = (new_offer - last_offer) * 100 / (current_asking_price - last_offer);

        if adjustment < min_per {
            rejected = store_haggle_insults(store_id);
            if rejected {
                status = BidState::Insulted;
            }
        } else if adjustment > max_per {
            adjustment = adjustment * 75 / 100;
            if adjustment < max_per {
                adjustment = max_per;
            }
        }

        adjustment =
            ((current_asking_price - new_offer) * (adjustment + random_number(5) - 3) / 100) + 1;

        if adjustment > 0 {
            current_asking_price -= adjustment;
        }

        if current_asking_price < final_asking_price {
            current_asking_price = final_asking_price;
            comment = "Final Offer";
            store_last_increment_set((final_asking_price - new_offer) as i16);
            final_flag += 1;

            if final_flag > 3 {
                if store_increase_insults(store_id) {
                    status = BidState::Insulted;
                } else {
                    status = BidState::Rejected;
                }
                rejected = true;
            }
        } else if new_offer >= current_asking_price {
            rejected = true;
            new_price = new_offer;
        }

        if !rejected {
            last_offer = new_offer;
            offers_count += 1;

            terminal::erase_line(Coord { y: 1, x: 0 });
            let msg = format!("Your last offer : {last_offer}");
            terminal::put_string(&msg, Coord { y: 1, x: 39 });

            print_speech_selling_haggle(last_offer, current_asking_price, final_flag);

            if current_asking_price - last_offer < i32::from(store_last_increment_get()) {
                store_last_increment_set((current_asking_price - last_offer) as i16);
            }
        }
    }

    if status == BidState::Received && !accepted_without_haggle {
        with_state_mut(|state| {
            store_update_bargaining_skills(
                &mut state.stores[store_id as usize],
                new_price,
                final_asking_price,
            );
        });
    }

    *price = new_price;
    status
}

/// C++ store.cpp lines 561–582.
#[doc(hidden)]
pub fn store_sell_customer_adjustment(
    owner: &Owner,
    cost: &mut i32,
    min_buy: &mut i32,
    max_buy: &mut i32,
    max_sell: &mut i32,
) {
    let race_id = with_state(|state| state.py.misc.race_id);

    *cost = *cost * (200 - player_stat_adjustment_charisma()) / 100;
    *cost = *cost
        * (200
            - i32::from(
                crate::data_stores::RACE_GOLD_ADJUSTMENTS[owner.race as usize][race_id as usize],
            ))
        / 100;
    if *cost < 1 {
        *cost = 1;
    }

    *max_sell = *cost * i32::from(owner.max_inflate) / 100;

    *max_buy = *cost * (200 - i32::from(owner.max_inflate)) / 100;
    *min_buy = *cost * (200 - i32::from(owner.min_inflate)) / 100;
    if *min_buy < 1 {
        *min_buy = 1;
    }
    if *max_buy < 1 {
        *max_buy = 1;
    }
    if *min_buy < *max_buy {
        *min_buy = *max_buy;
    }
}

/// C++ store.cpp lines 585–781.
#[doc(hidden)]
pub fn store_sell_haggle(
    store_id: i32,
    price: &mut i32,
    item: &crate::inventory::Inventory,
) -> BidState {
    let mut status = BidState::Received;
    let mut new_price = 0;

    let cost = store_item_value(item);
    let mut rejected = false;

    let mut max_gold = 0;
    let mut min_per = 0;
    let mut max_per = 0;
    let mut max_sell = 0;
    let mut min_buy = 0;
    let mut max_buy = 0;

    let owner = if cost < 1 {
        status = BidState::Offended;
        rejected = true;
        None
    } else {
        let owner_id = with_state(|state| state.stores[store_id as usize].owner_id);
        let owner = STORE_OWNERS[owner_id as usize];
        let mut cost_mut = cost;
        store_sell_customer_adjustment(
            &owner,
            &mut cost_mut,
            &mut min_buy,
            &mut max_buy,
            &mut max_sell,
        );
        min_per = i32::from(owner.haggles_per);
        max_per = min_per * 3;
        max_gold = i32::from(owner.max_cost);
        Some(owner)
    };

    let mut final_asking_price = 0;
    let mut final_flag = 0;
    let mut comment = "Offer";
    let mut accepted_without_haggle = false;

    if !rejected {
        let mut current_asking_price;
        display_store_haggle_commands(-1);

        let mut offer_count = 0;

        if max_buy > max_gold {
            final_flag = 1;
            comment = "Final Offer";
            store_last_increment_set(0);
            current_asking_price = max_gold;
            final_asking_price = max_gold;
            terminal::print_message(Some(
                "I am sorry, but I have not the money to afford such a fine item.",
            ));
            accepted_without_haggle = true;
        } else {
            current_asking_price = max_buy;
            final_asking_price = min_buy;

            if final_asking_price > max_gold {
                final_asking_price = max_gold;
            }

            if with_state(|state| {
                store_no_need_to_bargain(&state.stores[store_id as usize], final_asking_price)
            }) {
                terminal::print_message(Some(
                    "After a long bargaining session, you agree upon the price.",
                ));
                current_asking_price = final_asking_price;
                comment = "Final offer";
                accepted_without_haggle = true;
                store_last_increment_set(final_asking_price as i16);
                offer_count = 1;
            }
        }

        let min_offer = max_sell;
        let mut last_offer = min_offer;
        let mut new_offer = 0;

        if current_asking_price < 1 {
            current_asking_price = 1;
        }

        while !rejected {
            loop {
                let mut bidding_open = true;

                let msg = format!("{comment} :  {current_asking_price}");
                terminal::put_string(&msg, Coord { y: 1, x: 0 });

                status = store_receive_offer(
                    store_id,
                    "What price do you ask? ",
                    &mut new_offer,
                    last_offer,
                    offer_count,
                    -1,
                );

                if status != BidState::Received {
                    rejected = true;
                    break;
                }

                match new_offer.cmp(&current_asking_price) {
                    std::cmp::Ordering::Less => {
                        print_speech_sorry();
                        new_offer = last_offer;
                        if last_offer + i32::from(store_last_increment_get()) < current_asking_price
                        {
                            store_last_increment_set(0);
                        }
                    }
                    std::cmp::Ordering::Equal => {
                        rejected = true;
                        new_price = new_offer;
                        break;
                    }
                    std::cmp::Ordering::Greater => {
                        bidding_open = false;
                    }
                }

                if rejected || !bidding_open {
                    break;
                }
            }

            if rejected {
                break;
            }

            let mut adjustment =
                (last_offer - new_offer) * 100 / (last_offer - current_asking_price);

            if adjustment < min_per {
                rejected = store_haggle_insults(store_id);
                if rejected {
                    status = BidState::Insulted;
                }
            } else if adjustment > max_per {
                adjustment = adjustment * 75 / 100;
                if adjustment < max_per {
                    adjustment = max_per;
                }
            }

            adjustment = ((new_offer - current_asking_price) * (adjustment + random_number(5) - 3)
                / 100)
                + 1;

            if adjustment > 0 {
                current_asking_price += adjustment;
            }

            if current_asking_price > final_asking_price {
                current_asking_price = final_asking_price;
                comment = "Final Offer";
                store_last_increment_set((final_asking_price - new_offer) as i16);
                final_flag += 1;

                if final_flag > 3 {
                    if store_increase_insults(store_id) {
                        status = BidState::Insulted;
                    } else {
                        status = BidState::Rejected;
                    }
                    rejected = true;
                }
            } else if new_offer <= current_asking_price {
                rejected = true;
                new_price = new_offer;
            }

            if !rejected {
                last_offer = new_offer;
                offer_count += 1;

                terminal::erase_line(Coord { y: 1, x: 0 });
                let msg = format!("Your last bid {last_offer}");
                terminal::put_string(&msg, Coord { y: 1, x: 39 });

                print_speech_buying_haggle(current_asking_price, last_offer, final_flag);

                if current_asking_price - last_offer > i32::from(store_last_increment_get()) {
                    store_last_increment_set((current_asking_price - last_offer) as i16);
                }
            }
        }
    }

    let _ = owner;

    if status == BidState::Received && !accepted_without_haggle {
        with_state_mut(|state| {
            store_update_bargaining_skills(
                &mut state.stores[store_id as usize],
                new_price,
                final_asking_price,
            );
        });
    }

    *price = new_price;
    status
}

fn store_items_to_display(store_counter: i32, current_top_item_id: i32) -> i32 {
    if current_top_item_id == 12 {
        return store_counter - 1 - 12;
    }

    if store_counter > 11 {
        return 11;
    }

    store_counter - 1
}

fn store_purchase_an_item(store_id: i32, current_top_item_id: &mut i32) -> bool {
    let unique = with_state(|state| state.stores[store_id as usize].unique_items_counter);
    if unique < 1 {
        terminal::print_message(Some("I am currently out of stock."));
        return false;
    }

    let item_count = store_items_to_display(i32::from(unique), *current_top_item_id);
    let mut item_id = 0;
    if !store_get_item_id(
        &mut item_id,
        "Which item are you interested in? ",
        0,
        item_count,
    ) {
        return false;
    }

    item_id += *current_top_item_id;

    let mut sell_item = crate::inventory::Inventory::default();
    let store_item =
        with_state(|state| state.stores[store_id as usize].inventory[item_id as usize].item);
    inventory_take_one_item(&mut sell_item, &store_item);

    if !inventory_can_carry_item_count(sell_item) {
        terminal::put_string_clear_to_eol(
            "You cannot carry that many different items.",
            Coord { y: 0, x: 0 },
        );
        return false;
    }

    let fixed_cost =
        with_state(|state| state.stores[store_id as usize].inventory[item_id as usize].cost);
    let mut status = BidState::Received;
    let mut price = 0;

    if fixed_cost > 0 {
        price = fixed_cost;
    } else {
        status = store_purchase_haggle(store_id, &mut price, &sell_item);
    }

    let mut kick_customer = false;

    if status == BidState::Insulted {
        kick_customer = true;
    } else if status == BidState::Received {
        let gold = with_state(|state| state.py.misc.au);
        if gold >= price {
            print_speech_finished_haggling();
            store_decrease_insults(store_id);
            with_state_mut(|state| state.py.misc.au -= price);

            let new_item_id = inventory_carry_item(sell_item);
            let saved_store_counter =
                with_state(|state| state.stores[store_id as usize].unique_items_counter);

            store_destroy_item(store_id, item_id, true);

            let mut description = [0u8; MORIA_OBJ_DESC_SIZE as usize];
            with_state(|state| {
                item_description(
                    &mut description,
                    state.py.inventory[new_item_id as usize],
                    true,
                );
            });
            let desc = CStr::from_bytes_until_nul(&description)
                .map(|c| c.to_string_lossy().into_owned())
                .unwrap_or_default();
            let msg = format!("You have {} ({})", desc, (b'a' + new_item_id as u8) as char);
            terminal::put_string_clear_to_eol(&msg, Coord { y: 0, x: 0 });

            player_strength();

            let unique_after =
                with_state(|state| state.stores[store_id as usize].unique_items_counter);
            if *current_top_item_id >= unique_after as i32 {
                *current_top_item_id = 0;
                with_state_mut(|state| {
                    display_store_inventory_for_state(
                        state,
                        store_id as usize,
                        *current_top_item_id,
                    );
                });
            } else {
                let (same_count, store_item_cost) = with_state(|state| {
                    (
                        saved_store_counter == state.stores[store_id as usize].unique_items_counter,
                        state.stores[store_id as usize].inventory[item_id as usize].cost,
                    )
                });
                if same_count {
                    if store_item_cost < 0 {
                        with_state_mut(|state| {
                            state.stores[store_id as usize].inventory[item_id as usize].cost =
                                price;
                        });
                        display_single_cost(store_id, item_id);
                    }
                } else {
                    with_state_mut(|state| {
                        display_store_inventory_for_state(state, store_id as usize, item_id);
                    });
                }
            }
            display_player_remaining_gold();
        } else if store_increase_insults(store_id) {
            kick_customer = true;
        } else {
            print_speech_finished_haggling();
            terminal::print_message(Some("Liar!  You have not the gold!"));
        }
    }

    display_store_commands();
    terminal::erase_line(Coord { y: 1, x: 0 });

    kick_customer
}

pub fn set_general_store_items(item_type: u8) -> bool {
    matches!(
        item_type,
        TV_DIGGING | TV_BOOTS | TV_CLOAK | TV_FOOD | TV_FLASK | TV_LIGHT | TV_SPIKE
    )
}

pub fn set_armory_items(item_type: u8) -> bool {
    matches!(
        item_type,
        TV_BOOTS | TV_GLOVES | TV_HELM | TV_SHIELD | TV_HARD_ARMOR | TV_SOFT_ARMOR
    )
}

pub fn set_weaponsmith_items(item_type: u8) -> bool {
    matches!(
        item_type,
        TV_SLING_AMMO | TV_BOLT | TV_ARROW | TV_BOW | TV_HAFTED | TV_POLEARM | TV_SWORD
    )
}

pub fn set_temple_items(item_type: u8) -> bool {
    matches!(
        item_type,
        TV_HAFTED | TV_SCROLL1 | TV_SCROLL2 | TV_POTION1 | TV_POTION2 | TV_PRAYER_BOOK
    )
}

pub fn set_alchemist_items(item_type: u8) -> bool {
    matches!(item_type, TV_SCROLL1 | TV_SCROLL2 | TV_POTION1 | TV_POTION2)
}

pub fn set_magic_shop_items(item_type: u8) -> bool {
    matches!(
        item_type,
        TV_AMULET
            | TV_RING
            | TV_STAFF
            | TV_WAND
            | TV_SCROLL1
            | TV_SCROLL2
            | TV_POTION1
            | TV_POTION2
            | TV_MAGIC_BOOK
    )
}

pub const STORE_BUY: [fn(u8) -> bool; MAX_STORES as usize] = [
    set_general_store_items,
    set_armory_items,
    set_weaponsmith_items,
    set_temple_items,
    set_alchemist_items,
    set_magic_shop_items,
];

fn store_sell_an_item(store_id: i32, current_top_item_id: &mut i32) -> bool {
    let (first_item, last_item, mask) = with_state(|state| {
        let unique = state.py.pack.unique_items;
        let mut first = i32::from(unique);
        let mut last = -1;
        let mut mask = [0u8; PlayerEquipment::Wield as usize];

        for counter in 0..unique as i32 {
            let flag =
                STORE_BUY[store_id as usize](state.py.inventory[counter as usize].category_id);
            if flag {
                mask[counter as usize] = 1;
                if counter < first {
                    first = counter;
                }
                if counter > last {
                    last = counter;
                }
            } else {
                mask[counter as usize] = 0;
            }
        }

        (first, last, mask)
    });

    if last_item == -1 {
        terminal::print_message(Some("You have nothing to sell to this store!"));
        return false;
    }

    let mut item_id = 0;
    if !inventory_get_input_for_item_id(
        &mut item_id,
        "Which one? ",
        first_item,
        last_item,
        Some(&mask),
        Some("I do not buy such items."),
    ) {
        return false;
    }

    let mut sold_item = crate::inventory::Inventory::default();
    let inv_item = with_state(|state| state.py.inventory[item_id as usize]);
    inventory_take_one_item(&mut sold_item, &inv_item);

    let mut description = [0u8; MORIA_OBJ_DESC_SIZE as usize];
    item_description(&mut description, sold_item, true);
    let desc = CStr::from_bytes_until_nul(&description)
        .map(|c| c.to_string_lossy().into_owned())
        .unwrap_or_default();
    let msg = format!("Selling {} ({})", desc, (b'a' + item_id as u8) as char);
    terminal::print_message(Some(&msg));

    let store_snapshot = with_state(|state| state.stores[store_id as usize]);
    if !store_check_player_items_count(&store_snapshot, &sold_item) {
        terminal::print_message(Some("I have not the room in my store to keep it."));
        return false;
    }

    let mut price = 0;
    let status = store_sell_haggle(store_id, &mut price, &sold_item);

    let mut kick_customer = false;

    if status == BidState::Insulted {
        kick_customer = true;
    } else if status == BidState::Offended {
        terminal::print_message(Some("How dare you!"));
        terminal::print_message(Some("I will not buy that!"));
        kick_customer = store_increase_insults(store_id);
    } else if status == BidState::Received {
        print_speech_finished_haggling();
        store_decrease_insults(store_id);
        with_state_mut(|state| state.py.misc.au += price);

        item_identify(&mut item_id);

        inventory_take_one_item(
            &mut sold_item,
            &with_state(|state| state.py.inventory[item_id as usize]),
        );
        spell_item_identify_and_remove_random_inscription(&mut sold_item);
        inventory_destroy_item(item_id);

        item_description(&mut description, sold_item, true);
        let desc = CStr::from_bytes_until_nul(&description)
            .map(|c| c.to_string_lossy().into_owned())
            .unwrap_or_default();
        terminal::print_message(Some(&format!("You've sold {desc}")));

        let mut item_pos_id = -1;
        store_carry_item(store_id, &mut item_pos_id, &mut sold_item);

        player_strength();

        if item_pos_id >= 0 {
            if item_pos_id < 12 {
                if *current_top_item_id < 12 {
                    with_state_mut(|state| {
                        display_store_inventory_for_state(state, store_id as usize, item_pos_id);
                    });
                } else {
                    *current_top_item_id = 0;
                    with_state_mut(|state| {
                        display_store_inventory_for_state(
                            state,
                            store_id as usize,
                            *current_top_item_id,
                        );
                    });
                }
            } else if *current_top_item_id > 11 {
                with_state_mut(|state| {
                    display_store_inventory_for_state(state, store_id as usize, item_pos_id);
                });
            } else {
                *current_top_item_id = 12;
                with_state_mut(|state| {
                    display_store_inventory_for_state(
                        state,
                        store_id as usize,
                        *current_top_item_id,
                    );
                });
            }
        }
        display_player_remaining_gold();
    }

    terminal::erase_line(Coord { y: 1, x: 0 });
    display_store_commands();

    kick_customer
}

/// C++ store.cpp lines 1097–1174.
pub fn store_enter(store_id: i32) {
    let locked = with_state(|state| {
        state.stores[store_id as usize].turns_left_before_closing >= state.dg.game_turn
    });
    if locked {
        terminal::print_message(Some("The doors are locked."));
        return;
    }

    let mut current_top_item_id = 0;
    display_store(store_id, current_top_item_id);

    let mut exit_store = false;
    while !exit_store {
        terminal::move_cursor(Coord { y: 20, x: 9 });

        with_state_mut(|state| state.message_ready_to_print = false);

        let mut command = 0u8;
        if terminal::get_command("", &mut command) {
            let saved_chr = with_state(|state| state.py.stats.used[PlayerAttr::A_CHR as usize]);

            match command {
                b'b' => {
                    let unique =
                        with_state(|state| state.stores[store_id as usize].unique_items_counter);
                    if current_top_item_id == 0 {
                        if unique > 12 {
                            current_top_item_id = 12;
                            with_state_mut(|state| {
                                display_store_inventory_for_state(
                                    state,
                                    store_id as usize,
                                    current_top_item_id,
                                );
                            });
                        } else {
                            terminal::print_message(Some("Entire inventory is shown."));
                        }
                    } else {
                        current_top_item_id = 0;
                        with_state_mut(|state| {
                            display_store_inventory_for_state(
                                state,
                                store_id as usize,
                                current_top_item_id,
                            );
                        });
                    }
                }
                b'E' | b'e' | b'I' | b'i' | b'T' | b't' | b'W' | b'w' | b'X' | b'x' => {
                    let mut cmd = command;
                    loop {
                        inventory_execute_command(cmd);
                        cmd = with_state(|state| state.game.doing_inventory_command);
                        if cmd == 0 {
                            break;
                        }
                    }

                    let new_chr =
                        with_state(|state| state.py.stats.used[PlayerAttr::A_CHR as usize]);
                    if saved_chr != new_chr {
                        with_state_mut(|state| {
                            display_store_inventory_for_state(
                                state,
                                store_id as usize,
                                current_top_item_id,
                            );
                        });
                    }

                    with_state_mut(|state| state.game.player_free_turn = false);
                }
                b'p' => {
                    exit_store = store_purchase_an_item(store_id, &mut current_top_item_id);
                }
                b's' => {
                    exit_store = store_sell_an_item(store_id, &mut current_top_item_id);
                }
                _ => {
                    let _ = terminal::terminal_bell_sound();
                }
            }
        } else {
            exit_store = true;
        }
    }

    draw_cave_panel();
}

/// C++ `store_inventory.cpp` lines 24+.
pub fn store_maintenance() {
    crate::store_inventory::store_maintenance();
}

#[doc(hidden)]
pub fn test_store_no_need_to_bargain(store: &Store, min_price: i32) -> bool {
    store_no_need_to_bargain(store, min_price)
}

#[doc(hidden)]
pub fn test_store_update_bargaining_skills(store: &mut Store, price: i32, min_price: i32) {
    store_update_bargaining_skills(store, price, min_price);
}

#[doc(hidden)]
pub fn test_store_increase_insults(store_id: i32) -> bool {
    store_increase_insults(store_id)
}

#[doc(hidden)]
pub fn test_store_decrease_insults(store_id: i32) {
    store_decrease_insults(store_id);
}
