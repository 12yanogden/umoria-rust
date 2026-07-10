//! Phase 5.6.2 — per-turn player status/upkeep (strict TDD).

use umoria::config::identification::ID_MAGIK;
use umoria::config::player::{
    PLAYER_FOOD_ALERT, PLAYER_FOOD_FAINT, PLAYER_FOOD_WEAK, PLAYER_REGEN_FAINT,
    PLAYER_REGEN_NORMAL, PLAYER_REGEN_WEAK,
};
use umoria::config::player::status::{
    PY_ARMOR, PY_BLESSED, PY_BLIND, PY_CONFUSED, PY_DET_INV, PY_FEAR, PY_FAST, PY_HERO, PY_HP,
    PY_HUNGRY, PY_INVULN, PY_MANA, PY_PARALYSED, PY_POISONED, PY_REST, PY_SEARCH, PY_SHERO,
    PY_SLOW, PY_SPEED, PY_STATS, PY_STR, PY_TIM_INFRA, PY_WEAK,
};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::game_run::{
    item_enchanted, player_detect_enchantment, player_food_consumption,
    player_update_blindness, player_update_blessedness, player_update_confusion,
    player_update_detect_invisible, player_update_evil_protection, player_update_fear_state,
    player_update_fastness, player_update_heat_resistance, player_update_hero_status,
    player_update_hallucination, player_update_infra_vision, player_update_invulnerability,
    player_update_light_status, player_update_max_dungeon_depth, player_update_paralysis,
    player_update_poisoned_state, player_update_regeneration, player_update_resting_state,
    player_update_slowness, player_update_speed, player_update_status_flags,
    player_update_word_of_recall, test_end_running_count, test_regenerate_hp_amounts,
    test_regenerate_mana_amounts, test_reset_game_run_hooks,
};
use umoria::inventory::{Inventory, PlayerEquipment, PLAYER_INVENTORY_SIZE};
use umoria::monster::{test_reset_update_monsters_hooks, test_update_monsters_calls};
use umoria::player::PlayerAttr;
use umoria::treasure::{TV_MIN_ENCHANT, TV_NOTHING, TV_SWORD};
use umoria::ui_io::{
    register_game_ui_hooks, test_set_ncurses_stub, test_set_ui_capture, test_ui_messages,
};

fn setup() {
    test_set_ncurses_stub(true);
    register_game_ui_hooks();
    test_set_ui_capture(true);
    test_reset_game_run_hooks();
    test_reset_update_monsters_hooks();
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.pos.y = 10;
        s.py.pos.x = 10;
    });
}

fn light_misc_use() -> i16 {
    with_state(|s| s.py.inventory[PlayerEquipment::Light as usize].misc_use)
}

fn set_light(misc_use: i16, carrying: bool) {
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Light as usize].misc_use = misc_use;
        s.py.carrying_light = carrying;
        s.py.flags.blind = 0;
    });
}

fn messages_contain(needle: &str) -> bool {
    test_ui_messages().iter().any(|m| m.contains(needle))
}

// ---------------------------------------------------------------------------
// 1. playerUpdateLightStatus
// ---------------------------------------------------------------------------

#[test]
fn light_burning_decrements_and_goes_out() {
    setup();
    set_light(1, true);
    player_update_light_status();
    assert_eq!(light_misc_use(), 0);
    assert!(!with_state(|s| s.py.carrying_light));
    assert!(messages_contain("Your light has gone out!"));
    assert_eq!(test_update_monsters_calls(), vec![false]);
}

#[test]
fn light_growing_faint_only_on_guarded_rng_path() {
    setup();
    set_light(41, true);
    player_update_light_status();
    assert!(!messages_contain("Your light is growing faint."));
    assert_eq!(light_misc_use(), 40);

    let mut faint_seed = None;
    for seed in 1..500u32 {
        reset_for_new_game(Some(seed));
        with_state_mut(|s| {
            s.py.pos.y = 10;
            s.py.pos.x = 10;
        });
        set_light(40, true);
        player_update_light_status();
        if messages_contain("Your light is growing faint.") {
            faint_seed = Some(seed);
            break;
        }
    }
    assert!(faint_seed.is_some(), "expected some seed to trigger faint RNG path");
}

#[test]
fn light_no_rng_when_misc_use_at_least_40() {
    setup();
    set_light(41, true);
    let before = umoria::rng::get_seed();
    player_update_light_status();
    assert_eq!(umoria::rng::get_seed(), before);
}

#[test]
fn light_already_empty_unlights() {
    setup();
    set_light(0, true);
    player_update_light_status();
    assert!(!with_state(|s| s.py.carrying_light));
    assert_eq!(test_update_monsters_calls(), vec![false]);
}

#[test]
fn light_unlit_refuel_turns_on() {
    setup();
    set_light(5, false);
    player_update_light_status();
    assert!(with_state(|s| s.py.carrying_light));
    assert_eq!(light_misc_use(), 4);
    assert_eq!(test_update_monsters_calls(), vec![false]);
}

// ---------------------------------------------------------------------------
// 2. Hero / Super-hero
// ---------------------------------------------------------------------------

#[test]
fn hero_single_turn_activate_and_disable() {
    setup();
    with_state_mut(|s| {
        s.py.misc.max_hp = 50;
        s.py.misc.current_hp = 50;
        s.py.misc.bth = 10;
        s.py.misc.bth_with_bows = 10;
        s.py.flags.heroism = 1;
    });
    player_update_hero_status();
    assert_eq!(with_state(|s| s.py.misc.max_hp), 50);
    assert_eq!(with_state(|s| s.py.flags.status & PY_HERO), 0);
    assert!(messages_contain("The heroism wears off."));
}

#[test]
fn hero_two_turns_stays_active_one_turn() {
    setup();
    with_state_mut(|s| {
        s.py.misc.max_hp = 50;
        s.py.misc.current_hp = 50;
        s.py.flags.heroism = 2;
    });
    player_update_hero_status();
    assert_eq!(with_state(|s| s.py.flags.status & PY_HERO), PY_HERO);
    assert_eq!(with_state(|s| s.py.misc.max_hp), 60);
    player_update_hero_status();
    assert_eq!(with_state(|s| s.py.flags.status & PY_HERO), 0);
}

#[test]
fn super_hero_deltas_and_clamp_on_disable() {
    setup();
    with_state_mut(|s| {
        s.py.misc.max_hp = 30;
        s.py.misc.current_hp = 50;
        s.py.misc.bth = 5;
        s.py.misc.bth_with_bows = 5;
        s.py.flags.super_heroism = 1;
    });
    player_update_hero_status();
    assert_eq!(with_state(|s| s.py.misc.max_hp), 30);
    assert_eq!(with_state(|s| s.py.misc.bth), 5);
    assert_eq!(with_state(|s| s.py.misc.bth_with_bows), 5);
}

// ---------------------------------------------------------------------------
// 3. playerFoodConsumption
// ---------------------------------------------------------------------------

#[test]
fn food_regen_tiers_and_hungry_message() {
    setup();
    with_state_mut(|s| s.py.flags.food = (PLAYER_FOOD_ALERT - 1) as i16);
    let regen = player_food_consumption();
    assert_eq!(regen, i32::from(PLAYER_REGEN_NORMAL));
    assert_eq!(with_state(|s| s.py.flags.status & PY_HUNGRY), PY_HUNGRY);
    assert!(messages_contain("You are getting hungry."));
}

#[test]
fn food_weak_tier_and_status() {
    setup();
    with_state_mut(|s| s.py.flags.food = 500);
    let regen = player_food_consumption();
    assert_eq!(regen, i32::from(PLAYER_REGEN_WEAK));
    assert_eq!(with_state(|s| s.py.flags.status & PY_WEAK), PY_WEAK);
}

#[test]
fn food_faint_tier() {
    setup();
    with_state_mut(|s| s.py.flags.food = 100);
    let regen = player_food_consumption();
    assert_eq!(regen, i32::from(PLAYER_REGEN_FAINT));
}

#[test]
fn food_starvation_zero_regen() {
    setup();
    with_state_mut(|s| s.py.flags.food = -32);
    let regen = player_food_consumption();
    assert_eq!(regen, 0);
    assert!(with_state(|s| s.py.misc.current_hp < 0 || s.py.flags.food < 0));
}

#[test]
fn food_speed_burn_and_faint_rng() {
    setup();
    with_state_mut(|s| {
        s.py.flags.food = 100;
        s.py.flags.speed = -3;
    });
    reset_for_new_game(Some(99));
    with_state_mut(|s| {
        s.py.flags.food = 100;
        s.py.flags.speed = -3;
        s.py.flags.food_digested = 2;
    });
    let _ = player_food_consumption();
    let food_after = with_state(|s| s.py.flags.food);
    assert_eq!(food_after, 100 - 9 - 2);
}

// ---------------------------------------------------------------------------
// 4. playerUpdateRegeneration
// ---------------------------------------------------------------------------

#[test]
fn regeneration_scaling_and_invocation() {
    setup();
    with_state_mut(|s| {
        s.py.flags.regenerate_hp = true;
        s.py.flags.status |= PY_SEARCH;
        s.py.misc.max_hp = 100;
        s.py.misc.current_hp = 50;
        s.py.misc.mana = 20;
        s.py.misc.current_mana = 5;
        s.py.flags.poisoned = 0;
    });
    player_update_regeneration(i32::from(PLAYER_REGEN_NORMAL));
    assert_eq!(test_regenerate_hp_amounts(), vec![197 * 3 / 2 * 2]);
    assert_eq!(test_regenerate_mana_amounts(), vec![197 * 3 / 2 * 2]);
}

#[test]
fn regeneration_skips_hp_when_poisoned() {
    setup();
    with_state_mut(|s| {
        s.py.flags.poisoned = 1;
        s.py.misc.max_hp = 100;
        s.py.misc.current_hp = 50;
        s.py.misc.mana = 10;
        s.py.misc.current_mana = 5;
    });
    player_update_regeneration(100);
    assert!(test_regenerate_hp_amounts().is_empty());
    assert_eq!(test_regenerate_mana_amounts(), vec![100]);
}

// ---------------------------------------------------------------------------
// 5. Timed-counter updaters (sample branches)
// ---------------------------------------------------------------------------

#[test]
fn blindness_activate_decrement_expire() {
    setup();
    with_state_mut(|s| s.py.flags.blind = 1);
    player_update_blindness();
    assert_eq!(with_state(|s| s.py.flags.blind), 0);
    assert_eq!(with_state(|s| s.py.flags.status & PY_BLIND), 0);
    assert!(messages_contain("The veil of darkness lifts."));
}

#[test]
fn confusion_expiry_calls_rest_off_when_resting() {
    setup();
    with_state_mut(|s| {
        s.py.flags.confused = 1;
        s.py.flags.rest = 5;
        s.py.flags.status |= PY_REST;
    });
    player_update_confusion();
    assert_eq!(with_state(|s| s.py.flags.rest), 0);
}

#[test]
fn fear_hero_suppression_on_activate() {
    setup();
    with_state_mut(|s| {
        s.py.flags.afraid = 2;
        s.py.flags.heroism = 3;
    });
    player_update_fear_state();
    assert_eq!(with_state(|s| s.py.flags.afraid), -1);
    assert_eq!(with_state(|s| s.py.flags.status & PY_FEAR), 0);
}

#[test]
fn poison_expiry_message() {
    setup();
    with_state_mut(|s| s.py.flags.poisoned = 1);
    player_update_poisoned_state();
    assert!(messages_contain("You feel better."));
}

#[test]
fn poison_damage_cadence_by_constitution() {
    setup();
    with_state_mut(|s| {
        s.py.flags.poisoned = 5;
        s.py.stats.used[PlayerAttr::A_CON as usize] = 18;
        s.dg.game_turn = 4;
        s.py.misc.current_hp = 100;
        s.py.misc.max_hp = 100;
    });
    player_update_poisoned_state();
    assert_eq!(with_state(|s| s.py.misc.current_hp), 99);
}

#[test]
fn speed_fastness_and_slowness_toggle() {
    setup();
    with_state_mut(|s| {
        s.py.flags.fast = 1;
        s.py.flags.slow = 1;
    });
    player_update_speed();
    assert_eq!(with_state(|s| s.py.flags.status & (PY_FAST | PY_SLOW)), 0);
}

#[test]
fn resting_positive_countdown_rest_off() {
    setup();
    with_state_mut(|s| {
        s.py.flags.rest = 1;
        s.py.flags.status |= PY_REST;
    });
    player_update_resting_state();
    assert_eq!(with_state(|s| s.py.flags.rest), 0);
}

#[test]
fn hallucination_end_running_and_panel() {
    setup();
    with_state_mut(|s| s.py.flags.image = 1);
    player_update_hallucination();
    assert_eq!(test_end_running_count(), 1);
    assert_eq!(with_state(|s| s.py.flags.image), 0);
}

#[test]
fn invulnerability_and_blessed_bonuses() {
    setup();
    with_state_mut(|s| {
        s.py.misc.ac = 10;
        s.py.misc.display_ac = 10;
        s.py.misc.bth = 5;
        s.py.flags.invulnerability = 1;
    });
    player_update_invulnerability();
    assert_eq!(with_state(|s| s.py.misc.ac), 10);
    assert_eq!(with_state(|s| s.py.flags.status & PY_INVULN), 0);

    setup();
    with_state_mut(|s| {
        s.py.misc.ac = 10;
        s.py.misc.bth = 5;
        s.py.flags.blessed = 1;
    });
    player_update_blessedness();
    assert_eq!(with_state(|s| s.py.misc.ac), 10);
    assert!(messages_contain("The prayer has expired."));
}

#[test]
fn resist_and_evil_protection_messages() {
    setup();
    with_state_mut(|s| s.py.flags.heat_resistance = 1);
    player_update_heat_resistance();
    assert!(messages_contain("You no longer feel safe from flame."));

    setup();
    with_state_mut(|s| s.py.flags.protect_evil = 1);
    player_update_evil_protection();
    assert!(messages_contain("You no longer feel safe from evil."));
}

#[test]
fn detect_invisible_and_infra_monster_updates() {
    setup();
    with_state_mut(|s| s.py.flags.detect_invisible = 1);
    player_update_detect_invisible();
    assert_eq!(with_state(|s| s.py.flags.status & PY_DET_INV), 0);
    assert_eq!(test_update_monsters_calls(), vec![false, false]);

    setup();
    with_state_mut(|s| s.py.flags.timed_infra = 1);
    player_update_infra_vision();
    assert_eq!(with_state(|s| s.py.flags.status & PY_TIM_INFRA), 0);
    assert_eq!(test_update_monsters_calls(), vec![false, false]);
}

#[test]
fn word_of_recall_yank_upwards() {
    setup();
    with_state_mut(|s| {
        s.dg.current_level = 5;
        s.py.flags.word_of_recall = 1;
    });
    player_update_word_of_recall();
    assert!(with_state(|s| s.dg.generate_new_level));
    assert_eq!(with_state(|s| s.dg.current_level), 0);
    assert!(messages_contain("yanked upwards"));
}

#[test]
fn word_of_recall_decrements_when_above_one() {
    setup();
    with_state_mut(|s| s.py.flags.word_of_recall = 3);
    player_update_word_of_recall();
    assert_eq!(with_state(|s| s.py.flags.word_of_recall), 2);
}

#[test]
fn max_dungeon_depth_updates() {
    setup();
    with_state_mut(|s| {
        s.dg.current_level = 10;
        s.py.misc.max_dungeon_depth = 5;
    });
    player_update_max_dungeon_depth();
    assert_eq!(with_state(|s| s.py.misc.max_dungeon_depth), 10);
}

// ---------------------------------------------------------------------------
// 6. playerUpdateStatusFlags
// ---------------------------------------------------------------------------

#[test]
fn status_flags_dispatch_clears_bits() {
    setup();
    with_state_mut(|s| {
        s.py.flags.status = PY_SPEED | PY_HP | PY_MANA | PY_ARMOR;
        s.py.flags.paralysis = 0;
    });
    player_update_status_flags();
    let status = with_state(|s| s.py.flags.status);
    assert_eq!(status & (PY_SPEED | PY_HP | PY_MANA | PY_ARMOR), 0);
}

#[test]
fn status_flags_stats_loop() {
    setup();
    with_state_mut(|s| {
        s.py.flags.status = PY_STATS | PY_STR;
    });
    player_update_status_flags();
    assert_eq!(with_state(|s| s.py.flags.status & PY_STATS), 0);
}

// ---------------------------------------------------------------------------
// 7. playerDetectEnchantment
// ---------------------------------------------------------------------------

#[test]
fn item_enchanted_truth_table() {
    let mut item = Inventory::default();
    item.category_id = TV_NOTHING;
    assert!(!item_enchanted(item));

    item.category_id = TV_SWORD;
    item.to_hit = 1;
    assert!(item_enchanted(item));

    item.identification |= ID_MAGIK;
    assert!(!item_enchanted(item));
}

#[test]
fn detect_enchantment_skips_unique_items_slot() {
    setup();
    with_state_mut(|s| {
        s.py.pack.unique_items = 5;
        s.py.inventory[PlayerEquipment::Wield as usize].category_id = TV_SWORD;
        s.py.inventory[PlayerEquipment::Wield as usize].to_hit = 5;
    });
    player_detect_enchantment();
    assert!(!with_state(|s| {
        (s.py.inventory[PlayerEquipment::Wield as usize].identification & ID_MAGIK) != 0
    }) || messages_contain("There's something about"));
}

// ---------------------------------------------------------------------------
// 8. Playthrough parity — multi-upkeep turn sequence
// ---------------------------------------------------------------------------

#[test]
fn playthrough_upkeep_turn_sequence_fixed_seed() {
    setup();
    with_state_mut(|s| {
        s.py.flags.food = (PLAYER_FOOD_WEAK - 1) as i16;
        s.py.flags.food_digested = 2;
        s.py.flags.heroism = 1;
        s.py.flags.poisoned = 2;
        s.py.flags.fast = 1;
        s.dg.game_turn = 1;
        s.py.misc.max_hp = 80;
        s.py.misc.current_hp = 40;
        s.py.misc.mana = 10;
        s.py.misc.current_mana = 5;
        s.py.inventory[PlayerEquipment::Light as usize].misc_use = 100;
        s.py.carrying_light = true;
    });

    player_update_light_status();
    player_update_hero_status();
    let regen = player_food_consumption();
    player_update_regeneration(regen);
    player_update_poisoned_state();
    player_update_speed();

    assert!(with_state(|s| s.py.flags.food < PLAYER_FOOD_WEAK as i16));
    assert_eq!(with_state(|s| s.py.flags.heroism), 0);
    assert_eq!(with_state(|s| s.py.flags.fast), 0);
    let hp = with_state(|s| s.py.misc.current_hp);
    assert!(hp >= 40);
}
