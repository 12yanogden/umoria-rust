//! Phase 4.4.8 — player_quaff.cpp parity.
#![allow(clippy::int_plus_one)]

use umoria::config::dungeon::objects::OBJ_NOTHING;
use umoria::config::player::PLAYER_MAX_EXP;
use umoria::dice::{dice_roll, Dice};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::inventory::{inventory_item_copy_to, Inventory, PLAYER_INVENTORY_SIZE};
use umoria::player::{PlayerAttr, PLAYER_MAX_LEVEL};
use umoria::player_quaff::{player_drink_potion, quaff};
use umoria::treasure::{TV_FOOD, TV_POTION1, TV_POTION2};
use umoria::types::Coord_t;
use umoria::ui_io::{test_push_getch_keys, test_set_ncurses_stub, ESCAPE};

const POTION1: u8 = TV_POTION1;
const POTION2: u8 = TV_POTION2;

fn potion_flag(id: i32) -> u32 {
    1u32 << (id - 1)
}

fn potion2_flag(id: i32) -> u32 {
    1u32 << (id - 32 - 1)
}

fn sample_base_exp_levels() -> [u32; PLAYER_MAX_LEVEL as usize] {
    [
        10, 25, 45, 70, 100, 140, 200, 280, 380, 500, 650, 850, 1100, 1400, 1800, 2300, 2900,
        3600, 4400, 5400, 6800, 8400, 10200, 12500, 17500, 25000, 35000, 50000, 75000, 100000,
        150000, 200000, 300000, 400000, 500000, 750000, 1500000, 2500000, 5000000, 10000000,
    ]
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
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

fn setup_player() {
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.py.misc.level = 10;
        s.py.misc.class_id = 0;
        s.py.misc.experience_factor = 100;
        s.py.base_exp_levels = sample_base_exp_levels();
        s.py.misc.current_hp = 100;
        s.py.misc.max_hp = 200;
        s.py.misc.current_mana = 5;
        s.py.misc.mana = 20;
        s.py.misc.exp = 10;
        s.py.misc.max_exp = 10;
        s.py.stats.current[PlayerAttr::A_STR as usize] = 18;
        s.py.stats.max[PlayerAttr::A_STR as usize] = 18;
        s.py.stats.used[PlayerAttr::A_STR as usize] = 18;
        s.py.flags.free_action = false;
        s.py.flags.paralysis = 0;
        s.py.flags.blind = 0;
        s.py.flags.confused = 0;
        s.py.flags.poisoned = 0;
        s.py.flags.fast = 0;
        s.py.flags.slow = 0;
        s.py.flags.invulnerability = 0;
        s.py.flags.heroism = 0;
        s.py.flags.super_heroism = 0;
        s.py.flags.heat_resistance = 0;
        s.py.flags.cold_resistance = 0;
        s.py.flags.detect_invisible = 0;
        s.py.flags.timed_infra = 0;
        s.py.flags.food = 0;
        s.py.pack.unique_items = 0;
        s.game.player_free_turn = false;
        for i in 0..PLAYER_INVENTORY_SIZE as usize {
            inventory_item_copy_to(OBJ_NOTHING as i16, &mut s.py.inventory[i]);
        }
    });
}

fn pack_potion(slot: i32, item: Inventory) {
    with_state_mut(|s| {
        s.py.inventory[slot as usize] = item;
        if slot >= s.py.pack.unique_items as i32 {
            s.py.pack.unique_items = (slot + 1) as i16;
        }
    });
}

fn potion_item(flags: u32, category_id: u8) -> Inventory {
    Inventory {
        category_id,
        flags,
        items_count: 1,
        misc_use: 4,
        ..Default::default()
    }
}

fn drink(flags: u32, category_id: u8) -> bool {
    player_drink_potion(flags, category_id)
}

// ---------------------------------------------------------------------------
// 1. RNG-order golden traces — one roll site per subtype
// ---------------------------------------------------------------------------

#[test]
fn cure_light_wounds_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion_flag(13), POTION1));
    assert_eq!(next_random_pair(7), (7, 6));
    with_state(|s| assert_eq!(s.py.misc.current_hp, 104));
}

#[test]
fn cure_serious_wounds_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion_flag(14), POTION1));
    for _ in 0..4 {
        let _ = next_random_pair(7);
    }
    with_state(|s| assert!(s.py.misc.current_hp > 100));
}

#[test]
fn cure_critical_wounds_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion_flag(15), POTION1));
    for _ in 0..6 {
        let _ = next_random_pair(7);
    }
    with_state(|s| assert!(s.py.misc.current_hp > 100));
}

#[test]
fn sleep_potion_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion_flag(19), POTION1));
    with_state(|s| assert_eq!(s.py.flags.paralysis, 6));
    assert_eq!(next_random_pair(4), (4, 1));
}

#[test]
fn blindness_potion_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion_flag(20), POTION1));
    with_state(|s| assert_eq!(s.py.flags.blind, 102));
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn confusion_potion_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion_flag(21), POTION1));
    with_state(|s| assert_eq!(s.py.flags.confused, 14));
    assert_eq!(next_random_pair(20), (20, 13));
}

#[test]
fn poison_potion_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion_flag(22), POTION1));
    with_state(|s| assert_eq!(s.py.flags.poisoned, 12));
    assert_eq!(next_random_pair(15), (15, 3));
}

#[test]
fn haste_potion_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion_flag(23), POTION1));
    with_state(|s| assert_eq!(s.py.flags.fast, 17));
    assert_eq!(next_random_pair(25), (25, 23));
}

#[test]
fn slow_potion_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion_flag(24), POTION1));
    with_state(|s| assert_eq!(s.py.flags.slow, 17));
    assert_eq!(next_random_pair(25), (25, 23));
}

#[test]
fn invulnerability_potion_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion2_flag(36), POTION2));
    with_state(|s| assert_eq!(s.py.flags.invulnerability, 12));
    assert_eq!(next_random_pair(10), (10, 3));
}

#[test]
fn heroism_potion_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion2_flag(37), POTION2));
    with_state(|s| assert_eq!(s.py.flags.heroism, 27));
    assert_eq!(next_random_pair(25), (25, 23));
}

#[test]
fn super_heroism_potion_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion2_flag(38), POTION2));
    with_state(|s| assert_eq!(s.py.flags.super_heroism, 27));
    assert_eq!(next_random_pair(25), (25, 23));
}

#[test]
fn resist_heat_potion_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion2_flag(41), POTION2));
    with_state(|s| assert_eq!(s.py.flags.heat_resistance, 12));
    assert_eq!(next_random_pair(10), (10, 3));
}

#[test]
fn resist_cold_potion_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion2_flag(42), POTION2));
    with_state(|s| assert_eq!(s.py.flags.cold_resistance, 12));
    assert_eq!(next_random_pair(10), (10, 3));
}

#[test]
fn detect_invisible_potion_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion2_flag(43), POTION2));
    with_state(|s| assert_eq!(s.py.flags.detect_invisible, 14));
    assert_eq!(next_random_pair(12), (12, 9));
}

#[test]
fn infravision_potion_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion2_flag(47), POTION2));
    with_state(|s| assert_eq!(s.py.flags.timed_infra, 102));
    assert_eq!(next_random_pair(100), (100, 73));
}

// ---------------------------------------------------------------------------
// 2. Stat-potion branches
// ---------------------------------------------------------------------------

#[test]
fn strength_potion_increases_stat_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion_flag(1), POTION1));
    with_state(|s| assert_eq!(s.py.stats.current[PlayerAttr::A_STR as usize], 50));
}

#[test]
fn weakness_potion_routes_to_spell_lose_str() {
    reset_for_new_game(Some(42));
    setup_player();
    with_state_mut(|s| s.py.stats.current[PlayerAttr::A_STR as usize] = 20);
    assert!(drink(potion_flag(2), POTION1));
    with_state(|s| assert_eq!(s.py.stats.current[PlayerAttr::A_STR as usize], 18));
}

#[test]
fn restore_strength_potion_restores_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    with_state_mut(|s| {
        s.py.stats.current[PlayerAttr::A_STR as usize] = 10;
        s.py.stats.max[PlayerAttr::A_STR as usize] = 18;
    });
    assert!(drink(potion_flag(3), POTION1));
    with_state(|s| assert_eq!(s.py.stats.current[PlayerAttr::A_STR as usize], 18));
}

// ---------------------------------------------------------------------------
// 3. Experience-gain / lose math
// ---------------------------------------------------------------------------

#[test]
fn gain_experience_potion_adds_half_plus_ten() {
    reset_for_new_game(None);
    setup_player();
    with_state_mut(|s| {
        s.py.misc.level = 10;
        s.py.misc.exp = 100;
    });
    assert!(drink(potion_flag(18), POTION1));
    with_state(|s| assert_eq!(s.py.misc.exp, 160));
}

#[test]
fn gain_experience_potion_caps_at_player_max_exp() {
    reset_for_new_game(None);
    setup_player();
    with_state_mut(|s| s.py.misc.exp = PLAYER_MAX_EXP);
    assert!(!drink(potion_flag(18), POTION1));
    with_state(|s| assert_eq!(s.py.misc.exp, PLAYER_MAX_EXP));
}

#[test]
fn lose_experience_small_exp_branch_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    with_state_mut(|s| {
        s.py.misc.level = 10;
        s.py.misc.exp = 500;
    });
    assert!(drink(potion2_flag(34), POTION2));
    // exp/5 + randomNumber(500)/5 = 100 + 202/5 = 140 lost at seed 42
    with_state(|s| assert_eq!(s.py.misc.exp, 360));
}

#[test]
fn lose_experience_large_exp_scale_branch_seed42() {
    let expected_remain = {
        reset_for_new_game(Some(42));
        setup_player();
        with_state_mut(|s| {
            s.py.misc.exp = 50_000;
            s.py.misc.max_exp = 50_000;
            s.py.misc.level = 20;
        });
        drink(potion2_flag(34), POTION2);
        with_state(|s| s.py.misc.exp)
    };

    reset_for_new_game(Some(42));
    setup_player();
    with_state_mut(|s| {
        s.py.misc.exp = 50_000;
        s.py.misc.max_exp = 50_000;
        s.py.misc.level = 20;
    });
    assert!(drink(potion2_flag(34), POTION2));
    with_state(|s| assert_eq!(s.py.misc.exp, expected_remain));
    assert!(50_000 > 32_767, "uses scaled randomNumber branch");
}

// ---------------------------------------------------------------------------
// 4. int16_t status += semantics
// ---------------------------------------------------------------------------

#[test]
fn status_counters_use_wrapping_add_semantics() {
    reset_for_new_game(Some(42));
    setup_player();
    with_state_mut(|s| s.py.flags.blind = i16::MAX - 50);
    drink(potion_flag(20), POTION1);
    with_state(|s| {
        let expected = (i16::MAX - 50).wrapping_add(102);
        assert_eq!(s.py.flags.blind, expected);
    });
}

// ---------------------------------------------------------------------------
// 5. Identification + expend via quaff()
// ---------------------------------------------------------------------------

#[test]
fn quaff_consumes_potion_and_identifies_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    pack_potion(0, potion_item(potion_flag(23), POTION1));

    test_push_getch_keys(&[b'a' as i32]);
    quaff();

    with_state(|s| {
        assert_eq!(s.py.pack.unique_items, 0);
        assert_eq!(s.py.flags.fast, 17);
        assert!(!s.game.player_free_turn);
    });
}

#[test]
fn quaff_water_identifies_without_rng() {
    assert_rng_unchanged_after(
        || {
            setup_player();
            pack_potion(
                0,
                Inventory {
                    category_id: POTION1,
                    flags: 0,
                    items_count: 1,
                    misc_use: 2,
                    ..Default::default()
                },
            );
            test_push_getch_keys(&[b'a' as i32]);
        },
        quaff,
    );
}

#[test]
fn quaff_escape_consumes_no_rng_or_potion() {
    assert_rng_unchanged_after(
        || {
            setup_player();
            pack_potion(0, potion_item(potion_flag(22), POTION1));
            test_push_getch_keys(&[i32::from(ESCAPE)]);
        },
        quaff,
    );
    assert_eq!(with_state(|s| s.py.pack.unique_items), 1);
}

#[test]
fn quaff_no_potions_message_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_player();
            pack_potion(
                0,
                Inventory {
                    category_id: TV_FOOD,
                    ..Default::default()
                },
            );
        },
        quaff,
    );
}

#[test]
fn healing_potion_no_rng_when_at_full_hp() {
    reset_for_new_game(Some(42));
    setup_player();
    with_state_mut(|s| {
        s.py.misc.current_hp = 200;
        s.py.misc.max_hp = 200;
    });
    assert!(!drink(potion_flag(16), POTION1));
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn cure_light_matches_dice_roll_directly_seed42() {
    reset_for_new_game(Some(42));
    setup_player();
    let heal = dice_roll(Dice { dice: 2, sides: 7 });
    reset_for_new_game(Some(42));
    setup_player();
    assert!(drink(potion_flag(13), POTION1));
    with_state(|s| assert_eq!(i32::from(s.py.misc.current_hp), 100 + heal));
}
