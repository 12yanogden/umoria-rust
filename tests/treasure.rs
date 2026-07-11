//! `treasure` core, dispatch & weapon/armor enchantment tests.
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

use umoria::config::identification::{ID_SHOW_HIT_DAM, ID_SHOW_P1};
use umoria::config::treasure::flags::{
    TR_CON, TR_CURSED, TR_DEX, TR_FREE_ACT, TR_INT, TR_RES_LIGHT, TR_SPEED, TR_STR, TR_TUNNEL,
    TR_WIS,
};
use umoria::data_treasure::GAME_OBJECTS;
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::identification::SpecialNameIds;
use umoria::inventory::inventory_item_copy_to;
use umoria::treasure::{
    magic_enchantment_bonus, magic_should_be_enchanted, magic_treasure_magical_ability,
    staff_magic_charges, wand_magic_charges, TV_AMULET, TV_ARROW, TV_BOOTS, TV_BOW, TV_CHEST,
    TV_CLOAK, TV_DIGGING, TV_FOOD, TV_GLOVES, TV_HARD_ARMOR, TV_HELM, TV_LIGHT, TV_POTION1,
    TV_RING, TV_SCROLL1, TV_SHIELD, TV_SOFT_ARMOR, TV_SPIKE, TV_STAFF, TV_SWORD, TV_WAND,
};

fn find_object(category_id: u8, sub_category_id: Option<u8>) -> i16 {
    GAME_OBJECTS
        .iter()
        .position(|obj| {
            obj.category_id == category_id
                && sub_category_id.map_or(true, |sub| obj.sub_category_id == sub)
        })
        .unwrap() as i16
}

fn setup_treasure_item(object_id: i16) {
    with_state_mut(|s| {
        s.missiles_counter = 0;
        inventory_item_copy_to(object_id, &mut s.game.treasure.list[1]);
    });
}

fn enchant(object_id: i16, level: i32) {
    setup_treasure_item(object_id);
    magic_treasure_magical_ability(1, level);
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

fn item_snapshot() -> (i16, i16, i16, u32, u8, i16, i32, u8, u8, u8, u8) {
    with_state(|s| {
        let item = &s.game.treasure.list[1];
        (
            item.to_hit,
            item.to_damage,
            item.to_ac,
            item.flags,
            item.special_name_id,
            item.misc_use,
            item.cost,
            item.identification,
            item.sub_category_id,
            item.depth_first_found,
            item.items_count,
        )
    })
}

// --------------------------------------------------------------------------
// 2. magicEnchantmentBonus unit parity
// --------------------------------------------------------------------------

#[test]
fn magic_enchantment_bonus_level10_seed42() {
    reset_for_new_game(Some(42));
    let bonus = with_state_mut(|s| magic_enchantment_bonus(s, 0, 40, 10));
    assert_eq!(bonus, 0);
    assert_eq!(next_random_pair(100), (100, 36));
}

#[test]
fn magic_enchantment_bonus_overflow_guard_level100() {
    reset_for_new_game(Some(42));
    let bonus = with_state_mut(|s| magic_enchantment_bonus(s, 1, 30, 100));
    assert_eq!(bonus, 1);
    reset_for_new_game(Some(42));
    let bonus2 = with_state_mut(|s| magic_enchantment_bonus(s, 0, 5, 100));
    assert_eq!(bonus2, 0);
}

#[test]
fn magic_enchantment_bonus_clamps_below_base() {
    reset_for_new_game(Some(1));
    let bonus = with_state_mut(|s| magic_enchantment_bonus(s, 5, 30, 1));
    assert!(bonus >= 5);
}

// --------------------------------------------------------------------------
// 3. magicShouldBeEnchanted boundary
// --------------------------------------------------------------------------

#[test]
fn magic_should_be_enchanted_uses_lte_semantics_seed42() {
    reset_for_new_game(Some(42));
    let result = with_state_mut(|s| magic_should_be_enchanted(s, 15));
    assert!(result);
    assert_eq!(random_number(100), 73);
}

#[test]
fn magic_should_be_enchanted_rejects_when_roll_above_chance() {
    reset_for_new_game(Some(42));
    let fail = with_state_mut(|s| magic_should_be_enchanted(s, 0));
    assert!(!fail);
}

// --------------------------------------------------------------------------
// 4. Ego-item secondary rolls
// --------------------------------------------------------------------------

#[test]
fn magical_sword_holy_avenger_seed699_level30() {
    reset_for_new_game(Some(699));
    enchant(find_object(TV_SWORD, None), 30);
    let (to_hit, to_damage, to_ac, _, sn, misc, cost, id, ..) = item_snapshot();
    assert_eq!(to_hit, 7);
    assert_eq!(to_damage, 6);
    assert_eq!(to_ac, 4);
    assert_eq!(sn, SpecialNameIds::SN_HA as u8);
    assert_eq!(misc, 4);
    assert_eq!(cost, 12025);
    assert_eq!(id, ID_SHOW_HIT_DAM);
    assert_eq!(next_random_pair(100), (100, 63));
}

#[test]
fn magical_sword_defender_seed200_level30() {
    reset_for_new_game(Some(200));
    enchant(find_object(TV_SWORD, None), 30);
    let (to_hit, to_damage, to_ac, _, sn, misc, cost, ..) = item_snapshot();
    assert_eq!(to_hit, 5);
    assert_eq!(to_damage, 4);
    assert_eq!(to_ac, 7);
    assert_eq!(sn, SpecialNameIds::SN_DF as u8);
    assert_eq!(misc, 3);
    assert_eq!(cost, 9025);
    assert_eq!(next_random_pair(100), (100, 21));
}

// --------------------------------------------------------------------------
// 1. Per-category RNG-order golden captures (seed 42, level 10)
// --------------------------------------------------------------------------

#[test]
fn magical_armor_shield_seed42_level10() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_SHIELD, None), 10);
    let (_, _, to_ac, flags, sn, _, cost, id, ..) = item_snapshot();
    assert_eq!(to_ac, 1);
    assert_eq!(flags, TR_RES_LIGHT);
    assert_eq!(sn, SpecialNameIds::SN_RL as u8);
    assert_eq!(cost, 530);
    assert_eq!(id, 0);
    assert_eq!(next_random_pair(100), (100, 74));
}

#[test]
fn magical_armor_hard_armor_seed42_level10() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_HARD_ARMOR, None), 10);
    let (_, _, to_ac, flags, sn, _, cost, ..) = item_snapshot();
    assert_eq!(to_ac, 1);
    assert_eq!(flags, TR_RES_LIGHT);
    assert_eq!(sn, SpecialNameIds::SN_RL as u8);
    assert_eq!(cost, 930);
    assert_eq!(next_random_pair(100), (100, 74));
}

#[test]
fn magical_armor_soft_armor_seed42_level10() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_SOFT_ARMOR, None), 10);
    let (_, _, to_ac, flags, sn, _, cost, ..) = item_snapshot();
    assert_eq!(to_ac, 1);
    assert_eq!(flags, TR_RES_LIGHT);
    assert_eq!(sn, SpecialNameIds::SN_RL as u8);
    assert_eq!(cost, 504);
    assert_eq!(next_random_pair(100), (100, 74));
}

#[test]
fn magical_sword_unenchanted_seed42_level10() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_SWORD, None), 10);
    let (to_hit, to_damage, to_ac, flags, sn, _, cost, id, ..) = item_snapshot();
    assert_eq!((to_hit, to_damage, to_ac), (0, 0, 0));
    assert_eq!(flags, 0);
    assert_eq!(sn, 0);
    assert_eq!(cost, 25);
    assert_eq!(id, ID_SHOW_HIT_DAM);
    assert_eq!(next_random_pair(100), (100, 99));
}

#[test]
fn magical_bow_seed42_level10() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_BOW, None), 10);
    let (to_hit, to_damage, _, flags, _, misc, cost, id, ..) = item_snapshot();
    assert_eq!(to_hit, 1);
    assert_eq!(to_damage, 2);
    assert_eq!(flags, 0);
    assert_eq!(misc, 2);
    assert_eq!(cost, 50);
    assert_eq!(id, ID_SHOW_HIT_DAM);
    assert_eq!(next_random_pair(100), (100, 74));
}

#[test]
fn magical_digging_cursed_branch_seed42_level10() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_DIGGING, None), 10);
    let (_, _, _, flags, _, misc, cost, id, ..) = item_snapshot();
    assert_eq!(flags, TR_CURSED | TR_TUNNEL);
    assert_eq!(misc, -1);
    assert_eq!(cost, 0);
    assert_eq!(id, ID_SHOW_HIT_DAM);
    assert_eq!(next_random_pair(100), (100, 57));
}

#[test]
fn magical_gloves_seed42_level10() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_GLOVES, None), 10);
    let (_, _, to_ac, flags, sn, _, cost, ..) = item_snapshot();
    assert_eq!(to_ac, 1);
    assert_eq!(flags, TR_FREE_ACT);
    assert_eq!(sn, SpecialNameIds::SN_FREE_ACTION as u8);
    assert_eq!(cost, 1003);
    assert_eq!(next_random_pair(100), (100, 74));
}

#[test]
fn magical_boots_speed_seed42_level10() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_BOOTS, None), 10);
    let (_, _, to_ac, flags, sn, misc, cost, id, ..) = item_snapshot();
    assert_eq!(to_ac, 1);
    assert_eq!(flags, TR_SPEED);
    assert_eq!(sn, SpecialNameIds::SN_SPEED as u8);
    assert_eq!(misc, 1);
    assert_eq!(cost, 5004);
    assert_eq!(id, ID_SHOW_P1);
    assert_eq!(next_random_pair(100), (100, 74));
}

#[test]
fn magical_helm_regular_seed42_level10() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_HELM, Some(1)), 10);
    let (_, _, to_ac, flags, sn, misc, cost, id, ..) = item_snapshot();
    assert_eq!(to_ac, 1);
    assert_eq!(flags, TR_INT);
    assert_eq!(sn, SpecialNameIds::SN_INTELLIGENCE as u8);
    assert_eq!(misc, 2);
    assert_eq!(cost, 1004);
    assert_eq!(id, ID_SHOW_P1);
    assert_eq!(next_random_pair(100), (100, 99));
}

#[test]
fn magical_helm_crown_seed42_level10() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_HELM, Some(6)), 10);
    let (_, _, to_ac, flags, sn, misc, cost, id, ..) = item_snapshot();
    assert_eq!(to_ac, 1);
    assert_eq!(flags, TR_FREE_ACT | TR_CON | TR_DEX | TR_STR);
    assert_eq!(sn, SpecialNameIds::SN_MIGHT as u8);
    assert_eq!(misc, 3);
    assert_eq!(cost, 3000);
    assert_eq!(id, ID_SHOW_P1);
    assert_eq!(next_random_pair(100), (100, 99));
}

#[test]
fn magical_cloak_seed42_level10() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_CLOAK, None), 10);
    let (_, _, to_ac, flags, _, _, cost, ..) = item_snapshot();
    assert_eq!(to_ac, 1);
    assert_eq!(flags, 0);
    assert_eq!(cost, 3);
    assert_eq!(next_random_pair(100), (100, 57));
}

#[test]
fn magical_projectile_arrow_seed42_level10() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_ARROW, None), 10);
    let (to_hit, to_damage, _, flags, _, misc, cost, id, _, _, count) = item_snapshot();
    assert_eq!(to_hit, 1);
    assert_eq!(to_damage, 2);
    assert_eq!(flags, 0);
    assert_eq!(misc, 1);
    assert_eq!(cost, 1);
    assert_eq!(id, ID_SHOW_HIT_DAM);
    assert_eq!(count, 21);
    assert_eq!(next_random_pair(100), (100, 60));
}

#[test]
fn magical_projectile_spike_seed42_level10() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_SPIKE, None), 10);
    let (_, _, _, _, _, misc, cost, id, _, _, count) = item_snapshot();
    assert_eq!(misc, 1);
    assert_eq!(cost, 1);
    assert_eq!(id, 0);
    assert_eq!(count, 23);
    assert_eq!(next_random_pair(100), (100, 92));
}

// --------------------------------------------------------------------------
// 5. Inline categories + 4.5.3.2 delegate helpers (dispatch integration)
// --------------------------------------------------------------------------

#[test]
fn tv_light_odd_sub_category_seed42() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_LIGHT, Some(1)), 10);
    let (_, _, _, _, _, misc, cost, _, sub, ..) = item_snapshot();
    assert_eq!(misc, 2702);
    assert_eq!(cost, 35);
    assert_eq!(sub, 0);
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn tv_wand_applies_wand_magic_seed42() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_WAND, Some(0)), 10);
    let (_, _, _, flags, _, misc, cost, ..) = item_snapshot();
    assert_eq!(misc, 8);
    assert_eq!(flags, 1);
    assert_eq!(cost, 200);
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn tv_staff_applies_staff_magic_and_depth_fixup_seed42() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_STAFF, Some(7)), 10);
    let (_, _, _, flags, _, misc, cost, _, sub, depth, ..) = item_snapshot();
    assert_eq!(misc, 3);
    assert_eq!(flags, 128);
    assert_eq!(cost, 0);
    assert_eq!(sub, 7);
    assert_eq!(depth, 10);
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn tv_food_depth_fixup_seed42() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_FOOD, Some(90)), 10);
    let (_, _, _, _, _, misc, cost, _, sub, depth, ..) = item_snapshot();
    assert_eq!(misc, 5000);
    assert_eq!(cost, 3);
    assert_eq!(sub, 90);
    assert_eq!(depth, 0);
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn tv_scroll1_depth_fixup_seed42() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_SCROLL1, Some(67)), 10);
    let (_, _, _, flags, _, _, cost, _, sub, depth, ..) = item_snapshot();
    assert_eq!(flags, TR_DEX);
    assert_eq!(cost, 50);
    assert_eq!(sub, 67);
    assert_eq!(depth, 1);
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn tv_potion1_depth_fixup_seed42() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_POTION1, Some(76)), 10);
    let (_, _, _, flags, _, _, cost, _, sub, depth, ..) = item_snapshot();
    assert_eq!(flags, 512);
    assert_eq!(cost, 300);
    assert_eq!(sub, 76);
    assert_eq!(depth, 0);
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn tv_ring_delegate_seed42() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_RING, Some(0)), 10);
    let (_, _, _, flags, _, misc, cost, ..) = item_snapshot();
    assert_eq!(flags, TR_CURSED | TR_STR);
    assert_eq!(misc, -1);
    assert_eq!(cost, -400);
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn tv_amulet_delegate_seed42() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_AMULET, Some(0)), 10);
    let (_, _, _, flags, _, misc, cost, ..) = item_snapshot();
    assert_eq!(flags, TR_CURSED | TR_WIS);
    assert_eq!(misc, -1);
    assert_eq!(cost, -300);
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn tv_chest_delegate_seed42() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_CHEST, None), 10);
    let (_, _, _, flags, sn, _, cost, ..) = item_snapshot();
    assert_eq!(flags, 0x1380_0041);
    assert_eq!(sn, 49);
    assert_eq!(cost, 20);
    assert_eq!(next_random_pair(100), (100, 73));
}

// --------------------------------------------------------------------------
// wand/staff charge helpers (4.5.3.2 surface, tested via dispatch integration)
// --------------------------------------------------------------------------

#[test]
fn wand_magic_charges_id0_seed42() {
    reset_for_new_game(Some(42));
    assert_eq!(wand_magic_charges(0), 8);
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn staff_magic_charges_id7_seed42() {
    reset_for_new_game(Some(42));
    assert_eq!(staff_magic_charges(7), 3);
    assert_eq!(next_random_pair(100), (100, 73));
}

// --------------------------------------------------------------------------
// 6. Full cross-category dispatch golden (RNG consumption chain)
// --------------------------------------------------------------------------

#[test]
fn cross_category_dispatch_rng_chain_seed42() {
    reset_for_new_game(Some(42));
    let objects = [
        find_object(TV_SHIELD, None),
        find_object(TV_SWORD, None),
        find_object(TV_BOW, None),
        find_object(TV_ARROW, None),
        find_object(TV_WAND, Some(0)),
        find_object(TV_RING, Some(0)),
    ];
    for (slot, &obj) in objects.iter().enumerate() {
        let tid = slot as i32 + 1;
        with_state_mut(|s| inventory_item_copy_to(obj, &mut s.game.treasure.list[tid as usize]));
        magic_treasure_magical_ability(tid, 10);
    }
    assert_eq!(next_random_pair(100), (100, 17));
}

// --------------------------------------------------------------------------
// 7. Integer semantics
// --------------------------------------------------------------------------

#[test]
fn cursed_digging_misc_use_i16_cast() {
    reset_for_new_game(Some(42));
    enchant(find_object(TV_DIGGING, None), 10);
    let (_, _, _, _, _, misc, ..) = item_snapshot();
    assert_eq!(misc, -1i16);
}

#[test]
fn missiles_counter_wraps_at_shrt_max() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.missiles_counter = i16::MAX;
        inventory_item_copy_to(find_object(TV_ARROW, None), &mut s.game.treasure.list[1]);
    });
    magic_treasure_magical_ability(1, 10);
    with_state(|s| assert_eq!(s.missiles_counter, i16::MIN));
    let (_, _, _, _, _, misc, ..) = item_snapshot();
    assert_eq!(misc, i16::MIN);
}
