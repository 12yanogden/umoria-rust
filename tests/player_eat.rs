//! `player_eat` parity.
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
use umoria::config::identification::{OD_KNOWN1, OD_TRIED};
use umoria::config::player::status::{PY_HUNGRY, PY_WEAK};
use umoria::config::player::{PLAYER_FOOD_FULL, PLAYER_FOOD_MAX};
use umoria::data_treasure::GAME_OBJECTS;
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::identification::{
    item_set_as_identified, magic_initialize_item_names, object_position_offset, FlavorTables,
};
use umoria::inventory::{inventory_item_copy_to, Inventory, PLAYER_INVENTORY_SIZE};
use umoria::player::PLAYER_MAX_LEVEL;
use umoria::player_eat::{player_eat, player_ingest_food};
use umoria::treasure::TV_FOOD;
use umoria::types::MESSAGE_HISTORY_SIZE;
use umoria::ui_io::{test_push_getch_keys, test_set_ncurses_stub, ESCAPE};

const POISON_MUSHROOM: u16 = 0;
const BLINDNESS_MUSHROOM: u16 = 1;
const PARANOIA_MUSHROOM: u16 = 2;
const CONFUSION_MUSHROOM: u16 = 3;
const HALLUCINATION_MUSHROOM: u16 = 4;
const FIRST_AID_MUSHROOM: u16 = 12;
const MINOR_CURES_MUSHROOM: u16 = 13;
const LIGHT_CURES_MUSHROOM: u16 = 14;
const MAJOR_CURES_MUSHROOM: u16 = 20;
const RATION: u16 = 21;

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

fn setup_base() {
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.misc.level = 10;
        s.py.misc.exp = 0;
        s.py.misc.experience_factor = 100;
        s.py.base_exp_levels = [999_999; PLAYER_MAX_LEVEL as usize];
        s.py.misc.max_hp = 500;
        s.py.misc.current_hp = 200;
        s.py.flags.status = PY_WEAK | PY_HUNGRY;
        s.py.flags.poisoned = 0;
        s.py.flags.blind = 0;
        s.py.flags.confused = 0;
        s.py.flags.afraid = 0;
        s.py.flags.image = 0;
        s.py.pack.unique_items = 0;
        for i in 0..PLAYER_INVENTORY_SIZE as usize {
            inventory_item_copy_to(OBJ_NOTHING as i16, &mut s.py.inventory[i]);
        }
    });
}

fn setup_for_rng() {
    setup_base();
}

fn setup_for_ident() {
    setup_base();
    with_state_mut(|s| {
        s.flavor = FlavorTables::from_static_defaults();
        s.objects_identified.fill(0);
    });
    magic_initialize_item_names();
}

fn pack_food(slot: i32, object_id: u16) {
    with_state_mut(|s| {
        inventory_item_copy_to(object_id as i16, &mut s.py.inventory[slot as usize]);
        s.py.inventory[slot as usize].items_count = 1;
        if slot >= s.py.pack.unique_items as i32 {
            s.py.pack.unique_items = (slot + 1) as i16;
        }
    });
}

fn pack_custom_food(slot: i32, flags: u32, depth: u8, misc_use: i16, sub: u8) {
    with_state_mut(|s| {
        s.py.inventory[slot as usize] = Inventory {
            category_id: TV_FOOD,
            sub_category_id: sub,
            flags,
            depth_first_found: depth,
            misc_use,
            items_count: 1,
            weight: 1,
            ..Inventory::default()
        };
        if slot >= s.py.pack.unique_items as i32 {
            s.py.pack.unique_items = (slot + 1) as i16;
        }
    });
}

fn mushroom_ident_index(sub_category_id: u8) -> usize {
    let offset = object_position_offset(TV_FOOD, sub_category_id);
    assert!(offset >= 0);
    let id = (offset as usize) << 6;
    id + usize::from(sub_category_id & 63)
}

// ---------------------------------------------------------------------------
// 1. playerIngestFood — nourishment math (no RNG)
// ---------------------------------------------------------------------------

#[test]
fn player_ingest_food_clamps_negative_food_to_zero_before_add() {
    reset_for_new_game(None);
    setup_for_ident();
    with_state_mut(|s| s.py.flags.food = -50);
    player_ingest_food(100);
    with_state(|s| assert_eq!(s.py.flags.food, 100));
}

#[test]
fn player_ingest_food_full_message_without_bloating() {
    reset_for_new_game(None);
    setup_for_ident();
    with_state_mut(|s| {
        s.py.flags.food = PLAYER_FOOD_FULL as i16 - 100;
    });
    player_ingest_food(200);
    assert_eq!(last_message_text(), "You are full.");
    with_state(|s| assert_eq!(s.py.flags.food, PLAYER_FOOD_FULL as i16 + 100));
}

#[test]
fn player_ingest_food_bloat_penalty_when_entire_amount_causes_overflow() {
    reset_for_new_game(None);
    setup_for_ident();
    with_state_mut(|s| {
        s.py.flags.food = PLAYER_FOOD_MAX as i16;
        s.py.flags.slow = 0;
    });
    player_ingest_food(500);
    assert_eq!(last_message_text(), "You are bloated from overeating.");
    with_state(|s| {
        assert_eq!(s.py.flags.slow, 10);
        assert_eq!(s.py.flags.food, 15_010);
    });
}

#[test]
fn player_ingest_food_bloat_penalty_when_partial_amount_causes_overflow() {
    reset_for_new_game(None);
    setup_for_ident();
    with_state_mut(|s| {
        s.py.flags.food = PLAYER_FOOD_MAX as i16 - 100;
        s.py.flags.slow = 0;
    });
    player_ingest_food(500);
    with_state(|s| {
        assert_eq!(s.py.flags.slow, 8);
        assert_eq!(s.py.flags.food, PLAYER_FOOD_MAX as i16 + 8);
    });
}

// ---------------------------------------------------------------------------
// 2. Selection / cancel — zero effect RNG
// ---------------------------------------------------------------------------

#[test]
fn player_eat_empty_pack_message_and_no_rng() {
    assert_rng_unchanged_after(
        || {
            reset_for_new_game(Some(7));
            setup_for_rng();
        },
        player_eat,
    );
    assert_eq!(last_message_text(), "But you are not carrying anything.");
}

#[test]
fn player_eat_no_food_in_pack_message_and_no_rng() {
    assert_rng_unchanged_after(
        || {
            reset_for_new_game(Some(7));
            setup_for_rng();
            with_state_mut(|s| {
                s.py.pack.unique_items = 1;
                s.py.inventory[0].category_id = 1;
            });
        },
        player_eat,
    );
    assert_eq!(last_message_text(), "You are not carrying any food.");
}

#[test]
fn player_eat_escape_before_selection_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            reset_for_new_game(Some(7));
            setup_for_rng();
            pack_food(0, RATION);
            test_push_getch_keys(&[i32::from(ESCAPE)]);
        },
        player_eat,
    );
    assert!(with_state(|s| s.game.player_free_turn));
    assert_eq!(with_state(|s| s.py.pack.unique_items), 1);
}

// ---------------------------------------------------------------------------
// 3. RNG-order golden — one roll per effect branch
// ---------------------------------------------------------------------------

#[test]
fn player_eat_poison_mushroom_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_for_rng();
    pack_food(0, POISON_MUSHROOM);
    test_push_getch_keys(&[b'a' as i32]);

    player_eat();

    let depth = GAME_OBJECTS[POISON_MUSHROOM as usize].depth_first_found;
    with_state(|s| assert_eq!(s.py.flags.poisoned, (2 + i32::from(depth)) as i16));
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn player_eat_blindness_mushroom_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_for_rng();
    pack_food(0, BLINDNESS_MUSHROOM);
    test_push_getch_keys(&[b'a' as i32]);

    player_eat();

    let depth = i32::from(GAME_OBJECTS[BLINDNESS_MUSHROOM as usize].depth_first_found);
    with_state(|s| assert_eq!(s.py.flags.blind, (202 + 10 * depth + 100) as i16));
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn player_eat_paranoia_mushroom_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_for_rng();
    pack_food(0, PARANOIA_MUSHROOM);
    test_push_getch_keys(&[b'a' as i32]);

    player_eat();

    let depth = GAME_OBJECTS[PARANOIA_MUSHROOM as usize].depth_first_found;
    with_state(|s| assert_eq!(s.py.flags.afraid, (2 + i32::from(depth)) as i16));
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn player_eat_confusion_mushroom_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_for_rng();
    pack_food(0, CONFUSION_MUSHROOM);
    test_push_getch_keys(&[b'a' as i32]);

    player_eat();

    let depth = GAME_OBJECTS[CONFUSION_MUSHROOM as usize].depth_first_found;
    with_state(|s| assert_eq!(s.py.flags.confused, (2 + i32::from(depth)) as i16));
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn player_eat_hallucination_mushroom_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_for_rng();
    pack_food(0, HALLUCINATION_MUSHROOM);
    test_push_getch_keys(&[b'a' as i32]);

    player_eat();

    let depth = i32::from(GAME_OBJECTS[HALLUCINATION_MUSHROOM as usize].depth_first_found);
    with_state(|s| assert_eq!(s.py.flags.image, (102 + 25 * depth + 200) as i16));
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn player_eat_first_aid_mushroom_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_for_rng();
    pack_food(0, FIRST_AID_MUSHROOM);
    test_push_getch_keys(&[b'a' as i32]);

    player_eat();

    with_state(|s| assert_eq!(s.py.misc.current_hp, 202));
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn player_eat_minor_cures_mushroom_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_for_rng();
    pack_food(0, MINOR_CURES_MUSHROOM);
    test_push_getch_keys(&[b'a' as i32]);

    player_eat();

    with_state(|s| assert_eq!(s.py.misc.current_hp, 202));
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn player_eat_light_cures_mushroom_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_for_rng();
    pack_food(0, LIGHT_CURES_MUSHROOM);
    test_push_getch_keys(&[b'a' as i32]);

    player_eat();

    with_state(|s| assert_eq!(s.py.misc.current_hp, 202));
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn player_eat_major_cures_mushroom_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_for_rng();
    pack_food(0, MAJOR_CURES_MUSHROOM);
    test_push_getch_keys(&[b'a' as i32]);

    player_eat();

    with_state(|s| assert_eq!(s.py.misc.current_hp, 215));
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn player_eat_poisonous_food_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_for_rng();
    pack_custom_food(0, 0x0400_0000, 5, 500, 64);
    test_push_getch_keys(&[b'a' as i32]);

    player_eat();

    with_state(|s| assert_eq!(s.py.misc.current_hp, 198));
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn player_eat_plain_ration_consumes_no_effect_rng() {
    reset_for_new_game(Some(42));
    setup_for_rng();
    pack_food(0, RATION);
    test_push_getch_keys(&[b'a' as i32]);

    player_eat();

    assert_eq!(next_random_pair(100), (100, 2));
}

// ---------------------------------------------------------------------------
// 4. int16 status counter += semantics
// ---------------------------------------------------------------------------

#[test]
fn player_eat_poison_i16_wrap_on_large_counter() {
    reset_for_new_game(Some(42));
    setup_for_rng();
    with_state_mut(|s| s.py.flags.poisoned = 32_000);
    pack_food(0, POISON_MUSHROOM);
    test_push_getch_keys(&[b'a' as i32]);

    player_eat();

    let depth = GAME_OBJECTS[POISON_MUSHROOM as usize].depth_first_found;
    with_state(|s| assert_eq!(s.py.flags.poisoned, 32_000i16 + 2 + depth as i16));
}

// ---------------------------------------------------------------------------
// 5. Identification flow
// ---------------------------------------------------------------------------

#[test]
fn player_eat_identifying_mushroom_sets_known_flag() {
    reset_for_new_game(Some(42));
    setup_for_ident();
    pack_food(0, POISON_MUSHROOM);
    test_push_getch_keys(&[b'a' as i32]);

    let ident_idx = mushroom_ident_index(64);
    player_eat();

    with_state(|s| assert_ne!(s.objects_identified[ident_idx] & OD_KNOWN1, 0));
    assert_eq!(with_state(|s| s.py.pack.unique_items), 0);
}

#[test]
fn player_eat_inert_mushroom_sets_tried_not_known() {
    reset_for_new_game(Some(42));
    setup_for_ident();
    pack_custom_food(0, 0, 6, 500, 64);
    test_push_getch_keys(&[b'a' as i32]);

    let ident_idx = mushroom_ident_index(64);
    player_eat();

    with_state(|s| {
        assert_eq!(s.objects_identified[ident_idx] & OD_KNOWN1, 0);
        assert_ne!(s.objects_identified[ident_idx] & OD_TRIED, 0);
    });
}

#[test]
fn player_eat_already_known_mushroom_skips_identify_exp() {
    reset_for_new_game(Some(42));
    setup_for_ident();
    item_set_as_identified(TV_FOOD, 64);
    let exp_before = with_state(|s| s.py.misc.exp);
    pack_food(0, POISON_MUSHROOM);
    test_push_getch_keys(&[b'a' as i32]);

    player_eat();

    with_state(|s| assert_eq!(s.py.misc.exp, exp_before));
}

// ---------------------------------------------------------------------------
// 6. playerEat post-ingest status clearing
// ---------------------------------------------------------------------------

#[test]
fn player_eat_clears_hungry_and_weak_status_bits() {
    reset_for_new_game(Some(42));
    setup_for_rng();
    pack_food(0, RATION);
    test_push_getch_keys(&[b'a' as i32]);

    player_eat();

    with_state(|s| assert_eq!(s.py.flags.status & (PY_HUNGRY | PY_WEAK), 0));
}
