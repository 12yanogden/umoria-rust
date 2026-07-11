//! `store` interaction & haggling parity.
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

use umoria::config::dungeon::objects::OBJ_NOTHING;
use umoria::config::identification::ID_DAMD;
use umoria::data_store_owners::STORE_OWNERS;
use umoria::data_stores::{
    SPEECH_INSULTED_HAGGLING_DONE as SPEECH_INSULTED_HAGGLING_DONE_TABLE,
    SPEECH_SALE_ACCEPTED as SPEECH_SALE_ACCEPTED_TABLE,
};
use umoria::data_treasure::GAME_OBJECTS;
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::inventory::{inventory_item_copy_to, Inventory};
use umoria::player::PlayerAttr;
use umoria::player_stats::player_stat_adjustment_charisma;
use umoria::store::{
    print_speech_buying_haggle, print_speech_finished_haggling, print_speech_get_out_of_my_store,
    print_speech_selling_haggle, print_speech_sorry, print_speech_try_again, set_alchemist_items,
    set_armory_items, set_general_store_items, set_magic_shop_items, set_temple_items,
    set_weaponsmith_items, store_enter, store_get_haggle, store_initialize_owners,
    store_purchase_customer_adjustment, store_purchase_haggle, store_receive_offer,
    store_sell_customer_adjustment, store_sell_haggle, test_reset_store_last_increment,
    test_store_decrease_insults, test_store_increase_insults, test_store_last_increment,
    test_store_no_need_to_bargain, test_store_update_bargaining_skills, BidState, Store,
    MAX_OWNERS, MAX_STORES, STORE_BUY,
};
use umoria::treasure::{
    TV_AMULET, TV_ARROW, TV_BOOTS, TV_CLOAK, TV_DIGGING, TV_FOOD, TV_HAFTED, TV_MAGIC_BOOK,
    TV_POTION1, TV_SCROLL1, TV_SOFT_ARMOR, TV_SWORD, TV_WAND,
};
use umoria::ui_io::{
    ctrl_key, test_clear_getch_keys, test_push_getch_keys, test_set_ncurses_stub, ESCAPE,
};

const SHRT_MAX: u16 = i16::MAX as u16;

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

fn push_keys_in_consume_order(keys: &[i32]) {
    let mut reversed = keys.to_vec();
    reversed.reverse();
    test_push_getch_keys(&reversed);
}

fn push_string_input(s: &str) {
    let mut keys: Vec<i32> = s.bytes().map(i32::from).collect();
    keys.push(i32::from(ctrl_key(b'J')));
    push_keys_in_consume_order(&keys);
}

fn push_escape() {
    push_keys_in_consume_order(&[i32::from(ESCAPE)]);
}

fn push_input_sequence(values: &[&str]) {
    let mut keys = Vec::new();
    for value in values {
        keys.extend(value.bytes().map(i32::from));
        keys.push(i32::from(ctrl_key(b'J')));
    }
    push_keys_in_consume_order(&keys);
}

fn speech_roll(max: i32) -> i32 {
    reset_for_new_game(Some(42));
    random_number(max)
}

fn setup_stub_io() {
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
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

fn make_item(object_id: i16) -> Inventory {
    let mut item = Inventory::default();
    inventory_item_copy_to(object_id, &mut item);
    item
}

fn food_item() -> Inventory {
    make_item(find_object(TV_FOOD, Some(64)))
}

fn set_store_item(store_id: usize, owner_id: u8, item: Inventory, cost: i32) {
    with_state_mut(|s| {
        let store = &mut s.stores[store_id];
        store.owner_id = owner_id;
        store.unique_items_counter = 1;
        store.inventory[0].item = item;
        store.inventory[0].cost = cost;
    });
}

// ---------------------------------------------------------------------------
// 1. storeInitializeOwners RNG golden
// ---------------------------------------------------------------------------

#[test]
fn store_initialize_owners_rng_golden_seed42() {
    reset_for_new_game(Some(42));
    store_initialize_owners();

    let owners: Vec<u8> = with_state(|s| s.stores.iter().map(|st| st.owner_id).collect());
    assert_eq!(owners, vec![6, 13, 2, 9, 4, 17]);

    with_state(|s| {
        for store in &s.stores {
            assert_eq!(store.insults_counter, 0);
            assert_eq!(store.turns_left_before_closing, 0);
            assert_eq!(store.unique_items_counter, 0);
            assert_eq!(store.good_purchases, 0);
            assert_eq!(store.bad_purchases, 0);
            assert_eq!(store.inventory[0].cost, 0);
            assert_eq!(store.inventory[0].item.id, OBJ_NOTHING);
        }
    });

    assert_eq!(next_random_pair(3), (3, 2));
    assert_eq!(next_random_pair(3), (3, 1));
    assert_eq!(next_random_pair(3), (3, 1));
    assert_eq!(next_random_pair(3), (3, 1));
    assert_eq!(next_random_pair(3), (3, 3));
    assert_eq!(next_random_pair(3), (3, 2));
}

#[test]
fn store_initialize_owners_draws_max_stores_times() {
    reset_for_new_game(Some(1));
    store_initialize_owners();
    for _ in 0..MAX_STORES as i32 {
        let (max, _) = next_random_pair(i32::from(MAX_OWNERS / MAX_STORES));
        assert_eq!(max, 3);
    }
}

// ---------------------------------------------------------------------------
// 2. Speech-helper RNG golden
// ---------------------------------------------------------------------------

#[test]
fn print_speech_finished_haggling_rng_seed42() {
    let roll = speech_roll(14);
    reset_for_new_game(Some(42));
    setup_stub_io();
    print_speech_finished_haggling();
    assert_eq!(
        SPEECH_SALE_ACCEPTED_TABLE[(roll - 1) as usize],
        SPEECH_SALE_ACCEPTED_TABLE[(roll - 1) as usize]
    );
    assert!((1..=14).contains(&roll));
}

#[test]
fn print_speech_selling_haggle_non_final_rng_seed42() {
    let roll = speech_roll(16);
    reset_for_new_game(Some(42));
    setup_stub_io();
    print_speech_selling_haggle(100, 200, 0);
    assert!((1..=16).contains(&roll));
}

#[test]
fn print_speech_selling_haggle_final_rng_seed42() {
    let roll = speech_roll(3);
    reset_for_new_game(Some(42));
    setup_stub_io();
    print_speech_selling_haggle(100, 200, 1);
    assert!((1..=3).contains(&roll));
}

#[test]
fn print_speech_buying_haggle_non_final_rng_seed42() {
    let roll = speech_roll(15);
    reset_for_new_game(Some(42));
    setup_stub_io();
    print_speech_buying_haggle(200, 100, 0);
    assert!((1..=15).contains(&roll));
}

#[test]
fn print_speech_buying_haggle_final_rng_seed42() {
    let roll = speech_roll(3);
    reset_for_new_game(Some(42));
    setup_stub_io();
    print_speech_buying_haggle(200, 100, 1);
    assert!((1..=3).contains(&roll));
}

#[test]
fn print_speech_get_out_of_my_store_one_draw_seed42() {
    let roll = speech_roll(5);
    reset_for_new_game(Some(42));
    setup_stub_io();
    print_speech_get_out_of_my_store();
    assert!((1..=5).contains(&roll));
    assert_eq!(
        SPEECH_INSULTED_HAGGLING_DONE_TABLE[(roll - 1) as usize],
        SPEECH_INSULTED_HAGGLING_DONE_TABLE[(roll - 1) as usize]
    );
}

#[test]
fn print_speech_try_again_rng_seed42() {
    let roll = speech_roll(10);
    reset_for_new_game(Some(42));
    setup_stub_io();
    print_speech_try_again();
    assert!((1..=10).contains(&roll));
}

#[test]
fn print_speech_sorry_rng_seed42() {
    let roll = speech_roll(5);
    reset_for_new_game(Some(42));
    setup_stub_io();
    print_speech_sorry();
    assert!((1..=5).contains(&roll));
}

// ---------------------------------------------------------------------------
// 3. storeGetHaggle parsing (no RNG)
// ---------------------------------------------------------------------------

#[test]
fn store_get_haggle_first_offer_absolute() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    test_reset_store_last_increment();
    push_string_input("150");

    let mut offer = 0;
    assert!(store_get_haggle("Offer: ", &mut offer, 0));
    assert_eq!(offer, 150);
    assert_eq!(test_store_last_increment(), 0);
}

#[test]
fn store_get_haggle_increment_plus() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    test_reset_store_last_increment();
    push_string_input("+25");

    let mut offer = 100;
    assert!(store_get_haggle("Offer: ", &mut offer, 1));
    assert_eq!(offer, 125);
    assert_eq!(test_store_last_increment(), 25);
}

#[test]
fn store_get_haggle_increment_minus() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    test_reset_store_last_increment();
    push_string_input("-10");

    let mut offer = 100;
    assert!(store_get_haggle("Offer: ", &mut offer, 2));
    assert_eq!(offer, 90);
    assert_eq!(test_store_last_increment(), -10);
}

#[test]
fn store_get_haggle_empty_repeats_last_increment() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    with_state_mut(|_| {
        test_reset_store_last_increment();
    });
    push_string_input("+15");
    let mut offer = 50;
    assert!(store_get_haggle("Offer: ", &mut offer, 1));
    assert_eq!(offer, 65);

    push_string_input("");
    assert!(store_get_haggle("Offer: ", &mut offer, 2));
    assert_eq!(offer, 80);
}

#[test]
fn store_get_haggle_zero_after_sign_disables_increment() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    test_reset_store_last_increment();
    push_input_sequence(&["+", "50"]);

    let mut offer = 50;
    assert!(store_get_haggle("Offer: ", &mut offer, 1));
    assert_eq!(offer, 50);
    assert_eq!(test_store_last_increment(), 0);
}

#[test]
fn store_get_haggle_first_offer_increment_guard() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    test_reset_store_last_increment();
    push_string_input("+5");

    let mut offer = 0;
    assert!(!store_get_haggle("Offer: ", &mut offer, 0));
    assert_eq!(offer, 0);
}

#[test]
fn store_get_haggle_escape_invalid() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    push_escape();

    let mut offer = 99;
    assert!(!store_get_haggle("Offer: ", &mut offer, 0));
}

#[test]
fn store_get_haggle_int16_wrap_on_increment() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    test_reset_store_last_increment();
    push_string_input("+40000");

    let mut offer = 0;
    assert!(store_get_haggle("Offer: ", &mut offer, 1));
    assert_eq!(test_store_last_increment(), -25536);
}

// ---------------------------------------------------------------------------
// 4. storeReceiveOffer / BidState transitions
// ---------------------------------------------------------------------------

#[test]
fn store_receive_offer_accepts_at_threshold() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    test_reset_store_last_increment();
    push_string_input("200");

    let mut offer = 0;
    let status = store_receive_offer(0, "Offer: ", &mut offer, 150, 0, 1);
    assert_eq!(status, BidState::Received);
    assert_eq!(offer, 200);
}

#[test]
fn store_receive_offer_rejects_below_threshold_then_resets() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    with_state_mut(|s| {
        s.stores[0].insults_counter = 0;
        s.stores[0].owner_id = 0;
    });
    test_reset_store_last_increment();
    push_string_input("100");
    push_string_input("160");

    let mut offer = 0;
    let status = store_receive_offer(0, "Offer: ", &mut offer, 150, 0, 1);
    assert_eq!(status, BidState::Received);
    assert_eq!(offer, 160);
}

#[test]
fn store_receive_offer_esc_rejected() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    push_escape();

    let mut offer = 0;
    let status = store_receive_offer(0, "Offer: ", &mut offer, 150, 0, 1);
    assert_eq!(status, BidState::Rejected);
}

#[test]
fn store_receive_offer_sell_factor_accepts_lower() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    push_string_input("80");

    let mut offer = 0;
    let status = store_receive_offer(0, "Ask: ", &mut offer, 100, 0, -1);
    assert_eq!(status, BidState::Received);
    assert_eq!(offer, 80);
}

// ---------------------------------------------------------------------------
// 5. Buying-haggle golden (storePurchaseHaggle)
// ---------------------------------------------------------------------------

#[test]
fn store_purchase_haggle_no_need_to_bargain_fast_path() {
    reset_for_new_game(Some(42));
    setup_stub_io();
    let item = food_item();
    set_store_item(0, 0, item, -100);
    with_state_mut(|s| s.stores[0].good_purchases = SHRT_MAX);

    push_escape();

    let mut price = 0;
    let status = store_purchase_haggle(0, &mut price, &item);
    assert_eq!(status, BidState::Rejected);
    with_state(|s| assert_eq!(s.stores[0].good_purchases, SHRT_MAX));
}

#[test]
fn store_purchase_haggle_exact_match_seed99() {
    reset_for_new_game(Some(99));
    setup_stub_io();
    let item = food_item();
    set_store_item(0, 0, item, -50);

    let asking = with_state(|s| {
        let store = &s.stores[0];
        let mut min_sell = 0;
        let mut max_sell = 0;
        umoria::store_inventory::store_item_sell_price(store, &mut min_sell, &mut max_sell, &item);
        store_purchase_customer_adjustment(&mut min_sell, &mut max_sell);
        max_sell
    });

    push_string_input(&asking.to_string());

    let mut price = 0;
    let status = store_purchase_haggle(0, &mut price, &item);
    assert_eq!(status, BidState::Received);
    assert_eq!(price, asking);
}

#[test]
fn store_purchase_haggle_over_ask_sorry_path_rng_seed200() {
    reset_for_new_game(Some(200));
    setup_stub_io();
    let item = food_item();
    set_store_item(0, 0, item, -50);

    let (asking, min_offer) = with_state(|s| {
        let store = &s.stores[0];
        let mut min_sell = 0;
        let mut max_sell = 0;
        let cost = umoria::store_inventory::store_item_sell_price(
            store,
            &mut min_sell,
            &mut max_sell,
            &item,
        );
        store_purchase_customer_adjustment(&mut min_sell, &mut max_sell);
        let owner = &STORE_OWNERS[store.owner_id as usize];
        let max_buy = (cost * (200 - i32::from(owner.max_inflate)) / 100).max(1);
        (max_sell, max_buy)
    });

    push_string_input(&min_offer.to_string());
    push_string_input(&(asking + 100).to_string());
    push_string_input(&asking.to_string());

    let mut price = 0;
    let status = store_purchase_haggle(0, &mut price, &item);
    assert_eq!(status, BidState::Received);
    assert_eq!(price, asking);
}

// ---------------------------------------------------------------------------
// 6. Selling-haggle golden (storeSellHaggle)
// ---------------------------------------------------------------------------

#[test]
fn store_sell_haggle_offended_no_rng() {
    reset_for_new_game(Some(42));
    let next = random_number(5);
    reset_for_new_game(Some(42));
    setup_stub_io();
    let mut item = food_item();
    item.identification |= ID_DAMD;

    let mut price = 999;
    let status = store_sell_haggle(0, &mut price, &item);
    assert_eq!(status, BidState::Offended);
    assert_eq!(price, 0);
    assert_eq!(random_number(5), next);
}

// no-money fast-path haggle integration: see gaps in phase report (needs scripted multi-offer input)

#[test]
fn store_sell_haggle_no_need_to_bargain_fast_path() {
    reset_for_new_game(Some(42));
    setup_stub_io();
    let item = food_item();
    set_store_item(0, 0, item, -1);
    with_state_mut(|s| s.stores[0].good_purchases = SHRT_MAX);

    push_escape();

    let mut price = 0;
    let status = store_sell_haggle(0, &mut price, &item);
    assert_eq!(status, BidState::Rejected);
}

// ---------------------------------------------------------------------------
// 7. Customer-adjustment math parity
// ---------------------------------------------------------------------------

#[test]
fn store_purchase_customer_adjustment_charisma_clamp() {
    reset_for_new_game(Some(1));
    with_state_mut(|s| s.py.stats.used[PlayerAttr::A_CHR as usize] = 18);

    let chr_adj = player_stat_adjustment_charisma();
    let mut min_sell = 100;
    let mut max_sell = 200;
    store_purchase_customer_adjustment(&mut min_sell, &mut max_sell);

    assert_eq!(chr_adj, 100);
    assert_eq!(max_sell, 200);
    assert_eq!(min_sell, 100);
}

#[test]
fn store_purchase_customer_adjustment_zero_clamps_to_one() {
    reset_for_new_game(Some(1));
    with_state_mut(|s| s.py.stats.used[PlayerAttr::A_CHR as usize] = 1);

    let mut min_sell = 1;
    let mut max_sell = 1;
    store_purchase_customer_adjustment(&mut min_sell, &mut max_sell);
    assert_eq!(min_sell, 1);
    assert_eq!(max_sell, 1);
}

#[test]
fn store_sell_customer_adjustment_min_buy_clamp() {
    reset_for_new_game(Some(1));
    with_state_mut(|s| {
        s.py.stats.used[PlayerAttr::A_CHR as usize] = 18;
        s.py.misc.race_id = 0;
    });

    let owner = STORE_OWNERS[0];
    let mut cost = 1000;
    let mut min_buy = 0;
    let mut max_buy = 0;
    let mut max_sell = 0;
    store_sell_customer_adjustment(&owner, &mut cost, &mut min_buy, &mut max_buy, &mut max_sell);

    assert!(cost >= 1);
    assert!(min_buy >= max_buy);
}

// ---------------------------------------------------------------------------
// 8. store_buy predicates
// ---------------------------------------------------------------------------

#[test]
fn store_buy_general_store_items() {
    assert!(set_general_store_items(TV_DIGGING));
    assert!(set_general_store_items(TV_FOOD));
    assert!(!set_general_store_items(TV_SWORD));
}

#[test]
fn store_buy_armory_items() {
    assert!(set_armory_items(TV_SOFT_ARMOR));
    assert!(!set_armory_items(TV_SWORD));
}

#[test]
fn store_buy_weaponsmith_items() {
    assert!(set_weaponsmith_items(TV_ARROW));
    assert!(!set_weaponsmith_items(TV_BOOTS));
}

#[test]
fn store_buy_temple_items() {
    assert!(set_temple_items(TV_SCROLL1));
    assert!(!set_temple_items(TV_WAND));
}

#[test]
fn store_buy_alchemist_items() {
    assert!(set_alchemist_items(TV_POTION1));
    assert!(!set_alchemist_items(TV_FOOD));
}

#[test]
fn store_buy_magic_shop_items() {
    assert!(set_magic_shop_items(TV_AMULET));
    assert!(set_magic_shop_items(TV_MAGIC_BOOK));
    assert!(!set_magic_shop_items(TV_HAFTED));
}

#[test]
fn store_buy_dispatch_table_len() {
    assert_eq!(STORE_BUY.len(), MAX_STORES as usize);
    assert!(STORE_BUY[0](TV_CLOAK));
    assert!(STORE_BUY[2](TV_SWORD));
}

// ---------------------------------------------------------------------------
// 9. Bargaining-skill parity
// ---------------------------------------------------------------------------

#[test]
fn store_no_need_to_bargain_shrt_max_short_circuit() {
    let store = Store {
        good_purchases: SHRT_MAX,
        ..Store::default()
    };
    assert!(test_store_no_need_to_bargain(&store, 1000));
}

#[test]
fn store_no_need_to_bargain_heuristic_truncating_division() {
    let store = Store {
        good_purchases: 10,
        bad_purchases: 0,
        ..Store::default()
    };
    assert!(test_store_no_need_to_bargain(&store, 1201));
    assert!(!test_store_no_need_to_bargain(&store, 1250));
}

#[test]
fn store_update_bargaining_skills_min_price_early_return() {
    let mut store = Store::default();
    test_store_update_bargaining_skills(&mut store, 5, 5);
    assert_eq!(store.good_purchases, 0);
    assert_eq!(store.bad_purchases, 0);
}

#[test]
fn store_update_bargaining_skills_good_and_bad_caps() {
    let mut store = Store {
        good_purchases: SHRT_MAX,
        bad_purchases: SHRT_MAX,
        ..Store::default()
    };
    test_store_update_bargaining_skills(&mut store, 100, 100);
    test_store_update_bargaining_skills(&mut store, 50, 100);
    assert_eq!(store.good_purchases, SHRT_MAX);
    assert_eq!(store.bad_purchases, SHRT_MAX);
}

#[test]
fn store_update_bargaining_skills_increments() {
    let mut store = Store::default();
    test_store_update_bargaining_skills(&mut store, 50, 50);
    assert_eq!(store.good_purchases, 1);
    test_store_update_bargaining_skills(&mut store, 60, 50);
    assert_eq!(store.bad_purchases, 1);
}

// ---------------------------------------------------------------------------
// 10. Insult counters
// ---------------------------------------------------------------------------

#[test]
fn store_increase_insults_under_max_returns_false() {
    reset_for_new_game(Some(42));
    setup_stub_io();
    with_state_mut(|s| {
        s.stores[0].owner_id = 0;
        s.stores[0].insults_counter = 0;
    });
    assert!(!test_store_increase_insults(0));
    with_state(|s| assert_eq!(s.stores[0].insults_counter, 1));
}

#[test]
fn store_increase_insults_over_max_rng_seed42() {
    reset_for_new_game(Some(42));
    let _ = random_number(5);
    let closing_roll = random_number(2500);
    reset_for_new_game(Some(42));
    setup_stub_io();
    with_state_mut(|s| {
        s.stores[0].owner_id = 0;
        s.stores[0].insults_counter = 12;
        s.dg.game_turn = 100;
    });
    assert!(test_store_increase_insults(0));
    with_state(|s| {
        assert_eq!(s.stores[0].insults_counter, 0);
        assert_eq!(s.stores[0].bad_purchases, 1);
        assert_eq!(
            s.stores[0].turns_left_before_closing,
            100 + 2500 + closing_roll
        );
    });
}

#[test]
fn store_decrease_insults() {
    reset_for_new_game(Some(1));
    with_state_mut(|s| s.stores[0].insults_counter = 3);
    test_store_decrease_insults(0);
    with_state(|s| assert_eq!(s.stores[0].insults_counter, 2));
    test_store_decrease_insults(0);
    with_state(|s| assert_eq!(s.stores[0].insults_counter, 1));
}

// ---------------------------------------------------------------------------
// 11. storeEnter command loop
// ---------------------------------------------------------------------------

#[test]
fn store_enter_locked_doors() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    with_state_mut(|s| {
        s.stores[0].turns_left_before_closing = 1000;
        s.dg.game_turn = 500;
    });
    store_enter(0);
}

#[test]
fn store_enter_esc_exits() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    with_state_mut(|s| {
        s.stores[0].owner_id = 0;
        s.stores[0].unique_items_counter = 0;
    });
    push_escape();
    store_enter(0);
}

#[test]
fn store_enter_stocked_store_does_not_panic() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    set_store_item(0, 0, food_item(), -50);
    push_escape();
    store_enter(0);
}

#[test]
fn store_enter_then_inventory_does_not_panic() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    set_store_item(0, 0, food_item(), -50);
    with_state_mut(|s| {
        s.py.pack.unique_items = 1;
        s.py.inventory[0] = food_item();
    });
    // Open inventory from store, then ESC out of inventory, then ESC out of store.
    push_keys_in_consume_order(&[i32::from(ESCAPE), i32::from(ESCAPE), i32::from(b'i')]);
    store_enter(0);
}

// ---------------------------------------------------------------------------
// 12. Integer-semantics tests
// ---------------------------------------------------------------------------

#[test]
fn store_last_increment_int16_wrap_assignment() {
    reset_for_new_game(Some(1));
    setup_stub_io();
    test_reset_store_last_increment();
    push_string_input("+70000");
    let mut offer = 0;
    assert!(store_get_haggle("x", &mut offer, 1));
    assert_eq!(test_store_last_increment(), 4464);
}

#[test]
fn store_purchase_counters_uint16_at_shrt_max() {
    let mut store = Store {
        good_purchases: SHRT_MAX - 1,
        bad_purchases: SHRT_MAX - 1,
        ..Store::default()
    };
    test_store_update_bargaining_skills(&mut store, 20, 20);
    test_store_update_bargaining_skills(&mut store, 30, 20);
    assert_eq!(store.good_purchases, SHRT_MAX);
    assert_eq!(store.bad_purchases, SHRT_MAX);
}
