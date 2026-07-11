//! Rings/amulets/wands/staffs/chests magical ability tests.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::config::identification::{ID_NO_SHOW_P1, ID_SHOW_HIT_DAM};
use umoria::config::treasure::chests::{CH_EXPLODE, CH_LOCKED, CH_PARALYSED, CH_SUMMON};
use umoria::config::treasure::flags::TR_CURSED;
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::identification::SpecialNameIds;
use umoria::inventory::Inventory;
use umoria::treasure::{
    magic_treasure_magical_ability, staff_magic_charges, wand_magic_charges, TV_AMULET, TV_CHEST,
    TV_RING, TV_STAFF, TV_WAND,
};

const ITEM_ID: i32 = 1;

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

fn setup_item(category_id: u8, sub_category_id: u8, cost: i32) {
    with_state_mut(|state| {
        state.game.treasure.list[ITEM_ID as usize] = Inventory {
            category_id,
            sub_category_id,
            cost,
            ..Inventory::default()
        };
    });
}

fn item_snapshot() -> Inventory {
    with_state(|state| state.game.treasure.list[ITEM_ID as usize])
}

fn run_magical(
    seed: u32,
    category_id: u8,
    sub_category_id: u8,
    level: i32,
    cost: i32,
) -> Inventory {
    reset_for_new_game(Some(seed));
    setup_item(category_id, sub_category_id, cost);
    magic_treasure_magical_ability(ITEM_ID, level);
    item_snapshot()
}

// --------------------------------------------------------------------------
// 1. processRings RNG-order golden capture
// --------------------------------------------------------------------------

#[test]
fn process_rings_stat_subcategories_seed42_level10_cursed() {
    for sub in [0u8, 1, 2, 3] {
        let item = run_magical(42, TV_RING, sub, 10, 100);
        assert_eq!(item.misc_use, -1);
        assert_eq!(item.flags, TR_CURSED);
        assert_eq!(item.cost, -100);
    }
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn process_rings_sub4_seed42_cursed_inverted_misc_use() {
    let item = run_magical(42, TV_RING, 4, 10, 100);
    assert_eq!(item.misc_use, -3);
    assert_eq!(item.flags, TR_CURSED);
    assert_eq!(item.cost, -100);
    assert_eq!(next_random_pair(100), (100, 36));
}

#[test]
fn process_rings_sub4_seed2_not_cursed() {
    let item = run_magical(2, TV_RING, 4, 10, 100);
    assert_eq!(item.misc_use, 1);
    assert_eq!(item.flags, 0);
    assert_eq!(item.cost, 100);
    assert_eq!(next_random_pair(100), (100, 48));
}

#[test]
fn process_rings_sub5_seed42_bonus_then_cursed_gate() {
    let item = run_magical(42, TV_RING, 5, 10, 100);
    assert_eq!(item.misc_use, 5);
    assert_eq!(item.flags, 0);
    assert_eq!(item.cost, 350);
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn process_rings_combat_subcategories_seed42() {
    let dmg = run_magical(42, TV_RING, 19, 10, 100);
    assert_eq!(dmg.to_damage, 1);
    assert_eq!(dmg.cost, 200);

    let hit = run_magical(42, TV_RING, 20, 10, 100);
    assert_eq!(hit.to_hit, 1);
    assert_eq!(hit.cost, 200);

    let ac = run_magical(42, TV_RING, 21, 10, 100);
    assert_eq!(ac.to_ac, 1);
    assert_eq!(ac.cost, 200);
}

#[test]
fn process_rings_id_only_subcategories_seed42() {
    for sub in [24u8, 25, 26, 27, 28, 29] {
        let item = run_magical(42, TV_RING, sub, 10, 100);
        assert_eq!(item.identification, ID_NO_SHOW_P1);
        assert_eq!(item.cost, 100);
    }
}

#[test]
fn process_rings_slaying_sub30_seed42() {
    let item = run_magical(42, TV_RING, 30, 10, 100);
    assert_eq!(item.identification, ID_SHOW_HIT_DAM);
    assert_eq!(item.to_hit, 1);
    assert_eq!(item.to_damage, 1);
    assert_eq!(item.cost, 300);
    assert_eq!(next_random_pair(100), (100, 74));
}

#[test]
fn process_rings_seed100_sub0_cursed() {
    let item = run_magical(100, TV_RING, 0, 10, 100);
    assert_eq!(item.misc_use, -1);
    assert_eq!(item.flags, TR_CURSED);
    assert_eq!(item.cost, -100);
    assert_eq!(next_random_pair(100), (100, 97));
}

#[test]
fn process_rings_seed42_level50_cursed_threshold() {
    let item = run_magical(42, TV_RING, 0, 50, 100);
    assert_eq!(item.misc_use, -1);
    assert_eq!(item.flags, TR_CURSED);
    assert_eq!(item.cost, -100);
    assert_eq!(next_random_pair(100), (100, 2));
}

// --------------------------------------------------------------------------
// 2. processAmulets golden capture
// --------------------------------------------------------------------------

#[test]
fn process_amulets_sub0_sub1_seed42_cursed() {
    for sub in [0u8, 1] {
        let item = run_magical(42, TV_AMULET, sub, 10, 200);
        assert_eq!(item.misc_use, -1);
        assert_eq!(item.flags, TR_CURSED);
        assert_eq!(item.cost, -200);
    }
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn process_amulets_sub2_seed42_not_cursed() {
    let item = run_magical(42, TV_AMULET, 2, 10, 200);
    assert_eq!(item.misc_use, 5);
    assert_eq!(item.flags, 0);
    assert_eq!(item.cost, 450);
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn process_amulets_sub2_seed2_higher_bonus() {
    let item = run_magical(2, TV_AMULET, 2, 10, 200);
    assert_eq!(item.misc_use, 10);
    assert_eq!(item.cost, 700);
    assert_eq!(next_random_pair(100), (100, 28));
}

#[test]
fn process_amulets_sub8_magi_never_cursed() {
    let item = run_magical(42, TV_AMULET, 8, 10, 200);
    assert_eq!(item.misc_use, 5);
    assert_eq!(item.flags, 0);
    assert_eq!(item.cost, 300);
    assert_eq!(next_random_pair(100), (100, 36));
}

// --------------------------------------------------------------------------
// 3. wandMagic / staffMagic charge rolls
// --------------------------------------------------------------------------

#[test]
fn wand_magic_all_ids_seed42() {
    reset_for_new_game(Some(42));
    let expected = [
        8, 12, 8, 12, 5, 12, 14, 14, 8, 8, 14, 5, 12, 8, 5, 5, 8, 6, 10, 4, 4, 12, 4, 14,
    ];
    for (id, &charges) in expected.iter().enumerate() {
        reset_for_new_game(Some(42));
        assert_eq!(wand_magic_charges(id as u8), charges, "wand id {id}");
    }
}

#[test]
fn wand_magic_out_of_range_no_rng() {
    reset_for_new_game(Some(42));
    assert_eq!(wand_magic_charges(24), -1);
    assert_eq!(next_random_pair(100), (100, 2));

    setup_item(TV_WAND, 24, 0);
    magic_treasure_magical_ability(ITEM_ID, 10);
    assert_eq!(item_snapshot().misc_use, 0);
}

#[test]
fn staff_magic_all_ids_seed42() {
    let expected = [
        14, 12, 8, 14, 8, 7, 5, 3, 3, 8, 14, 8, 8, 8, 14, 6, 8, 8, 6, 14, 6, 6, 8,
    ];
    for (id, &charges) in expected.iter().enumerate() {
        reset_for_new_game(Some(42));
        assert_eq!(staff_magic_charges(id as u8), charges, "staff id {id}");
    }
}

#[test]
fn staff_magic_out_of_range_no_rng() {
    reset_for_new_game(Some(42));
    assert_eq!(staff_magic_charges(23), -1);
    assert_eq!(next_random_pair(100), (100, 2));

    setup_item(TV_STAFF, 23, 0);
    magic_treasure_magical_ability(ITEM_ID, 10);
    assert_eq!(item_snapshot().misc_use, 0);
}

#[test]
fn wand_dispatch_sets_misc_use_seed42() {
    let item = run_magical(42, TV_WAND, 0, 10, 0);
    assert_eq!(item.misc_use, 8);
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn staff_dispatch_sets_misc_use_seed42() {
    let item = run_magical(42, TV_STAFF, 0, 10, 0);
    assert_eq!(item.misc_use, 14);
    assert_eq!(next_random_pair(100), (100, 73));
}

// --------------------------------------------------------------------------
// 4. magicalChests trap/lock assignment
// --------------------------------------------------------------------------

#[test]
fn magical_chests_level_sweep_seed42() {
    let cases: [(i32, u32, u8); 5] = [
        (1, CH_LOCKED, SpecialNameIds::SN_LOCKED as u8),
        (5, CH_LOCKED, SpecialNameIds::SN_LOCKED as u8),
        (
            10,
            CH_PARALYSED | CH_LOCKED,
            SpecialNameIds::SN_GAS_TRAP as u8,
        ),
        (
            20,
            CH_SUMMON | CH_LOCKED,
            SpecialNameIds::SN_SUMMONING_RUNES as u8,
        ),
        (
            50,
            CH_SUMMON | CH_EXPLODE | CH_LOCKED,
            SpecialNameIds::SN_MULTIPLE_TRAPS as u8,
        ),
    ];

    for (level, flags, sn) in cases {
        let item = run_magical(42, TV_CHEST, 0, level, 0);
        assert_eq!(item.flags, flags, "level {level}");
        assert_eq!(item.special_name_id, sn, "level {level}");
    }
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn magical_chests_seed777_empty() {
    let item = run_magical(777, TV_CHEST, 0, 10, 0);
    assert_eq!(item.flags, 0);
    assert_eq!(item.special_name_id, SpecialNameIds::SN_EMPTY as u8);
    assert_eq!(next_random_pair(100), (100, 29));
}

#[test]
fn magical_chests_seed2_gas_trap() {
    let item = run_magical(2, TV_CHEST, 0, 10, 0);
    assert_eq!(item.flags, CH_PARALYSED | CH_LOCKED);
    assert_eq!(item.special_name_id, SpecialNameIds::SN_GAS_TRAP as u8);
    assert_eq!(next_random_pair(100), (100, 48));
}

#[test]
fn magical_chests_locked_only_level1_seed42() {
    let item = run_magical(42, TV_CHEST, 0, 1, 0);
    assert_eq!(item.flags, CH_LOCKED);
    assert_eq!(item.special_name_id, SpecialNameIds::SN_LOCKED as u8);
}

// --------------------------------------------------------------------------
// 5. Integer semantics spot checks
// --------------------------------------------------------------------------

#[test]
fn ring_cursed_cost_negation_i32() {
    let item = run_magical(42, TV_RING, 0, 10, 100);
    assert_eq!(item.cost, -100);
}

#[test]
fn ring_sub5_misc_use_int16_scaling() {
    let item = run_magical(42, TV_RING, 5, 10, 100);
    assert_eq!(item.misc_use, 5i16);
    assert_eq!(item.cost, 100 + 5 * 50);
}
