//! `player_magic` temporary-effect helpers.
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

use umoria::config::monsters::defense::{
    CD_ANIMAL, CD_DRAGON, CD_EVIL, CD_FIRE, CD_FROST, CD_UNDEAD,
};
use umoria::config::treasure::flags::{
    TR_CURSED, TR_FLAME_TONGUE, TR_FROST_BRAND, TR_SLAY_ANIMAL, TR_SLAY_DRAGON, TR_SLAY_EVIL,
    TR_SLAY_UNDEAD,
};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::inventory::Inventory;
use umoria::player_magic::{
    item_magic_ability_damage, player_bless, player_cure_blindness, player_cure_confusion,
    player_cure_poison, player_detect_invisible, player_protect_evil, player_remove_fear,
};
use umoria::treasure::{TV_ARROW, TV_FLASK, TV_HAFTED, TV_MISC, TV_SWORD};

const YOUNG_WHITE_DRAGON_ID: i32 = 217;
const LOST_SOUL_ID: i32 = 87;
const GIANT_CLEAR_ANT_ID: i32 = 122;
const STREET_URCHIN_ID: i32 = 0;
const YELLOW_JELLY_ID: i32 = 39;

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

fn ego_weapon(category_id: u8, slay_or_brand_flag: u32) -> Inventory {
    Inventory {
        category_id,
        flags: slay_or_brand_flag,
        ..Default::default()
    }
}

fn clear_recall_defenses(monster_id: i32) {
    with_state_mut(|s| s.creature_recall[monster_id as usize].defenses = 0);
}

fn recall_defenses(monster_id: i32) -> u16 {
    with_state(|s| s.creature_recall[monster_id as usize].defenses)
}

// ---------------------------------------------------------------------------
// 1. playerProtectEvil — RNG-order golden
// ---------------------------------------------------------------------------
#[test]
fn player_protect_evil_rng_order_seed42_level10() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.misc.level = 10;
        s.py.flags.protect_evil = 0;
    });

    assert!(player_protect_evil());

    // seed 42: randomNumber(25) == 2; protect_evil += 2 + 3*10
    with_state(|s| assert_eq!(s.py.flags.protect_evil, 32));
    assert_eq!(next_random_pair(25), (25, 23));
}

#[test]
fn player_protect_evil_returns_false_when_already_protected() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.misc.level = 5;
        s.py.flags.protect_evil = 7;
    });

    assert!(!player_protect_evil());

    // 7 + randomNumber(25)=2 + 3*5
    with_state(|s| assert_eq!(s.py.flags.protect_evil, 24));
}

// ---------------------------------------------------------------------------
// 2. Cure / remove-fear helpers — table-driven counter semantics
// ---------------------------------------------------------------------------
#[test]
fn player_cure_confusion_counter_semantics() {
    for counter in [0i16, 1, 2, 500] {
        reset_for_new_game(None);
        with_state_mut(|s| s.py.flags.confused = counter);
        let cured = player_cure_confusion();
        let expected = counter > 1;
        assert_eq!(cured, expected, "counter={counter}");
        with_state(|s| {
            assert_eq!(s.py.flags.confused, if expected { 1 } else { counter });
        });
    }
}

#[test]
fn player_cure_blindness_counter_semantics() {
    for counter in [0i16, 1, 2, 500] {
        reset_for_new_game(None);
        with_state_mut(|s| s.py.flags.blind = counter);
        let cured = player_cure_blindness();
        let expected = counter > 1;
        assert_eq!(cured, expected, "counter={counter}");
        with_state(|s| {
            assert_eq!(s.py.flags.blind, if expected { 1 } else { counter });
        });
    }
}

#[test]
fn player_cure_poison_counter_semantics() {
    for counter in [0i16, 1, 2, 500] {
        reset_for_new_game(None);
        with_state_mut(|s| s.py.flags.poisoned = counter);
        let cured = player_cure_poison();
        let expected = counter > 1;
        assert_eq!(cured, expected, "counter={counter}");
        with_state(|s| {
            assert_eq!(s.py.flags.poisoned, if expected { 1 } else { counter });
        });
    }
}

#[test]
fn player_remove_fear_counter_semantics() {
    for counter in [0i16, 1, 2, 500] {
        reset_for_new_game(None);
        with_state_mut(|s| s.py.flags.afraid = counter);
        let cured = player_remove_fear();
        let expected = counter > 1;
        assert_eq!(cured, expected, "counter={counter}");
        with_state(|s| {
            assert_eq!(s.py.flags.afraid, if expected { 1 } else { counter });
        });
    }
}

#[test]
fn cure_helpers_consume_no_rng() {
    assert_rng_unchanged_after(
        || {
            with_state_mut(|s| {
                s.py.flags.confused = 5;
                s.py.flags.blind = 5;
                s.py.flags.poisoned = 5;
                s.py.flags.afraid = 5;
            });
        },
        || {
            assert!(player_cure_confusion());
            assert!(player_cure_blindness());
            assert!(player_cure_poison());
            assert!(player_remove_fear());
        },
    );
}

// ---------------------------------------------------------------------------
// 3. playerBless / playerDetectInvisible — pure +=
// ---------------------------------------------------------------------------
#[test]
fn player_bless_adds_adjustment_without_clamp() {
    reset_for_new_game(None);
    with_state_mut(|s| s.py.flags.blessed = 4);
    player_bless(9);
    with_state(|s| assert_eq!(s.py.flags.blessed, 13));
}

#[test]
fn player_detect_invisible_adds_adjustment_without_clamp() {
    reset_for_new_game(None);
    with_state_mut(|s| s.py.flags.detect_invisible = 6);
    player_detect_invisible(11);
    with_state(|s| assert_eq!(s.py.flags.detect_invisible, 17));
}

#[test]
fn bless_and_detect_invisible_consume_no_rng() {
    assert_rng_unchanged_after(
        || {
            with_state_mut(|s| {
                s.py.flags.blessed = 0;
                s.py.flags.detect_invisible = 0;
            });
        },
        || {
            player_bless(5);
            player_detect_invisible(7);
        },
    );
}

// ---------------------------------------------------------------------------
// 4. itemMagicAbilityDamage — slay/brand branches
// ---------------------------------------------------------------------------
#[test]
fn item_magic_ability_damage_slay_dragon() {
    reset_for_new_game(None);
    clear_recall_defenses(YOUNG_WHITE_DRAGON_ID);
    let item = ego_weapon(TV_SWORD, TR_SLAY_DRAGON);
    assert_eq!(
        item_magic_ability_damage(item, 10, YOUNG_WHITE_DRAGON_ID),
        40
    );
    assert_eq!(recall_defenses(YOUNG_WHITE_DRAGON_ID), CD_DRAGON);
}

#[test]
fn item_magic_ability_damage_slay_undead() {
    reset_for_new_game(None);
    clear_recall_defenses(LOST_SOUL_ID);
    let item = ego_weapon(TV_HAFTED, TR_SLAY_UNDEAD);
    assert_eq!(item_magic_ability_damage(item, 10, LOST_SOUL_ID), 30);
    assert_eq!(recall_defenses(LOST_SOUL_ID), CD_UNDEAD);
}

#[test]
fn item_magic_ability_damage_slay_animal() {
    reset_for_new_game(None);
    clear_recall_defenses(GIANT_CLEAR_ANT_ID);
    let item = ego_weapon(TV_ARROW, TR_SLAY_ANIMAL);
    assert_eq!(item_magic_ability_damage(item, 10, GIANT_CLEAR_ANT_ID), 20);
    assert_eq!(recall_defenses(GIANT_CLEAR_ANT_ID), CD_ANIMAL);
}

#[test]
fn item_magic_ability_damage_slay_evil() {
    reset_for_new_game(None);
    clear_recall_defenses(STREET_URCHIN_ID);
    let item = ego_weapon(TV_SWORD, TR_SLAY_EVIL);
    assert_eq!(item_magic_ability_damage(item, 10, STREET_URCHIN_ID), 20);
    assert_eq!(recall_defenses(STREET_URCHIN_ID), CD_EVIL);
}

#[test]
fn item_magic_ability_damage_frost_brand() {
    reset_for_new_game(None);
    clear_recall_defenses(STREET_URCHIN_ID);
    let item = ego_weapon(TV_FLASK, TR_FROST_BRAND);
    assert_eq!(item_magic_ability_damage(item, 10, STREET_URCHIN_ID), 15);
    assert_eq!(recall_defenses(STREET_URCHIN_ID), CD_FROST);
}

#[test]
fn item_magic_ability_damage_flame_tongue() {
    reset_for_new_game(None);
    clear_recall_defenses(YELLOW_JELLY_ID);
    let item = ego_weapon(TV_SWORD, TR_FLAME_TONGUE);
    assert_eq!(item_magic_ability_damage(item, 10, YELLOW_JELLY_ID), 15);
    assert_eq!(recall_defenses(YELLOW_JELLY_ID), CD_FIRE);
}

#[test]
fn item_magic_ability_damage_integer_truncation_on_three_halves() {
    reset_for_new_game(None);
    clear_recall_defenses(YELLOW_JELLY_ID);
    let item = ego_weapon(TV_SWORD, TR_FLAME_TONGUE);
    assert_eq!(item_magic_ability_damage(item, 7, YELLOW_JELLY_ID), 10);
}

#[test]
fn item_magic_ability_damage_branch_order_dragon_before_undead() {
    reset_for_new_game(None);
    clear_recall_defenses(YOUNG_WHITE_DRAGON_ID);
    let item = ego_weapon(TV_SWORD, TR_SLAY_DRAGON | TR_SLAY_UNDEAD | TR_FLAME_TONGUE);
    assert_eq!(
        item_magic_ability_damage(item, 11, YOUNG_WHITE_DRAGON_ID),
        44
    );
    assert_eq!(recall_defenses(YOUNG_WHITE_DRAGON_ID), CD_DRAGON);
}

#[test]
fn item_magic_ability_damage_non_ego_weapon_unchanged() {
    reset_for_new_game(None);
    clear_recall_defenses(YOUNG_WHITE_DRAGON_ID);
    let item = Inventory {
        category_id: TV_SWORD,
        flags: TR_CURSED,
        ..Default::default()
    };
    assert_eq!(
        item_magic_ability_damage(item, 10, YOUNG_WHITE_DRAGON_ID),
        10
    );
    assert_eq!(recall_defenses(YOUNG_WHITE_DRAGON_ID), 0);
}

#[test]
fn item_magic_ability_damage_wrong_category_unchanged() {
    reset_for_new_game(None);
    clear_recall_defenses(YOUNG_WHITE_DRAGON_ID);
    let item = ego_weapon(TV_MISC, TR_SLAY_DRAGON);
    assert_eq!(
        item_magic_ability_damage(item, 10, YOUNG_WHITE_DRAGON_ID),
        10
    );
    assert_eq!(recall_defenses(YOUNG_WHITE_DRAGON_ID), 0);
}

#[test]
fn item_magic_ability_damage_no_matching_branch_unchanged() {
    reset_for_new_game(None);
    clear_recall_defenses(GIANT_CLEAR_ANT_ID);
    let item = ego_weapon(TV_SWORD, TR_SLAY_DRAGON);
    assert_eq!(item_magic_ability_damage(item, 10, GIANT_CLEAR_ANT_ID), 10);
    assert_eq!(recall_defenses(GIANT_CLEAR_ANT_ID), 0);
}

#[test]
fn item_magic_ability_damage_consume_no_rng() {
    assert_rng_unchanged_after(
        || clear_recall_defenses(YOUNG_WHITE_DRAGON_ID),
        || {
            let item = ego_weapon(TV_SWORD, TR_SLAY_DRAGON);
            assert_eq!(
                item_magic_ability_damage(item, 10, YOUNG_WHITE_DRAGON_ID),
                40
            );
        },
    );
}
