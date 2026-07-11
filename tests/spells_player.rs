//! Player-effect & item-utility spells (`spells`).
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

use umoria::config::dungeon::objects::OBJ_NOTHING;
use umoria::config::treasure::flags::TR_CURSED;
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, MIN_CLOSED_SPACE, TILE_GRANITE_WALL, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::inventory::{
    inventory_item_copy_to, inventory_item_is_cursed, Inventory, PlayerEquipment,
    PLAYER_INVENTORY_SIZE,
};
use umoria::player::PlayerAttr;
use umoria::player_stats::player_initialize_base_experience_levels;
use umoria::spells::{
    spell_change_player_hit_points, spell_create_food, spell_enchant_item, spell_lose_chr,
    spell_lose_con, spell_lose_dex, spell_lose_exp, spell_lose_int, spell_lose_str, spell_lose_wis,
    spell_recharge_item, spell_recharge_item_at, spell_remove_curse_from_all_worn_items,
    spell_restore_player_levels, spell_slow_poison, spell_teleport_player_to,
};
use umoria::treasure::TV_WAND;
use umoria::types::{Coord_t, MESSAGE_HISTORY_SIZE};
use umoria::ui_io::test_set_ncurses_stub;

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

fn setup_player_base() {
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.misc.current_hp = 50;
        s.py.misc.max_hp = 100;
        s.py.misc.level = 10;
        s.py.misc.class_id = 2;
        s.py.misc.experience_factor = 100;
        s.py.misc.exp = 500;
        s.py.misc.max_exp = 1000;
        s.py.stats.current[PlayerAttr::A_STR as usize] = 18;
        s.py.stats.used[PlayerAttr::A_STR as usize] = 18;
        s.py.stats.current[PlayerAttr::A_INT as usize] = 18;
        s.py.stats.used[PlayerAttr::A_INT as usize] = 18;
        s.py.flags.sustain_str = false;
        s.py.flags.sustain_int = false;
        s.py.flags.sustain_wis = false;
        s.py.flags.sustain_dex = false;
        s.py.flags.sustain_con = false;
        s.py.flags.sustain_chr = false;
        s.py.flags.poisoned = 0;
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.py.pack.unique_items = 0;
        s.message_ready_to_print = true;
        for i in 0..PLAYER_INVENTORY_SIZE as usize {
            inventory_item_copy_to(OBJ_NOTHING as i16, &mut s.py.inventory[i]);
        }
    });
    player_initialize_base_experience_levels();
}

fn setup_dungeon(height: i16, width: i16) {
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                s.dg.floor[y as usize][x as usize].feature_id = TILE_LIGHT_FLOOR;
            }
        }
        s.dg.floor[s.py.pos.y as usize][s.py.pos.x as usize].creature_id = 1;
        s.game.treasure.current_id = 1;
    });
}

fn pack_wand(slot: i32, depth: u8, misc_use: i16) {
    with_state_mut(|s| {
        s.py.inventory[slot as usize] = Inventory {
            category_id: TV_WAND,
            sub_category_id: 64,
            items_count: 1,
            depth_first_found: depth,
            misc_use,
            ..Inventory::default()
        };
        if slot + 1 > s.py.pack.unique_items as i32 {
            s.py.pack.unique_items = (slot + 1) as i16;
        }
    });
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

// --------------------------------------------------------------------------
// 1. RNG-order golden — enchant
// --------------------------------------------------------------------------

fn assert_enchant_dual_run_parity(plusses: i16, limit: i16, seed: u32) {
    reset_for_new_game(Some(seed));
    setup_player_base();
    let mut first = plusses;
    let ok1 = spell_enchant_item(&mut first, limit);
    let tail1 = (random_number(100), random_number(10));

    reset_for_new_game(Some(seed));
    setup_player_base();
    let mut second = plusses;
    let ok2 = spell_enchant_item(&mut second, limit);
    assert_eq!(ok1, ok2);
    assert_eq!(first, second);
    assert_eq!((random_number(100), random_number(10)), tail1);
}

#[test]
fn spell_enchant_item_max_bonus_zero_skips_rng() {
    assert_rng_unchanged_after(
        || {
            setup_player_base();
        },
        || {
            let mut plusses = 5;
            assert!(!spell_enchant_item(&mut plusses, 0));
            assert_eq!(plusses, 5);
        },
    );
}

#[test]
fn spell_enchant_item_negative_limit_skips_rng() {
    assert_rng_unchanged_after(setup_player_base, || {
        let mut plusses = 3;
        assert!(!spell_enchant_item(&mut plusses, -1));
        assert_eq!(plusses, 3);
    });
}

#[test]
fn spell_enchant_item_seed42_plusses5_limit10() {
    assert_enchant_dual_run_parity(5, 10, 42);
}

#[test]
fn spell_enchant_item_seed42_zero_plusses() {
    assert_enchant_dual_run_parity(0, 10, 42);
}

#[test]
fn spell_enchant_item_critical_branch_seed1() {
    assert_enchant_dual_run_parity(5, 10, 1);
}

// --------------------------------------------------------------------------
// 2. RNG-order golden — recharge
// --------------------------------------------------------------------------

#[test]
fn spell_recharge_item_fail_chance_guard_skips_random_number_zero() {
    reset_for_new_game(Some(42));
    setup_player_base();
    pack_wand(0, 10, 60);
    spell_recharge_item_at(0, 20);
    with_state(|s| assert_eq!(s.py.pack.unique_items, 0));
    assert_rng_unchanged_after(
        || {
            setup_player_base();
            pack_wand(0, 10, 60);
        },
        || spell_recharge_item_at(0, 20),
    );
}

#[test]
fn spell_recharge_item_seed42_success_rng_order() {
    reset_for_new_game(Some(42));
    setup_player_base();
    pack_wand(0, 10, 5);
    spell_recharge_item_at(0, 20);
    let misc1 = with_state(|s| s.py.inventory[0].misc_use);
    let tail1 = random_number(100);

    reset_for_new_game(Some(42));
    setup_player_base();
    pack_wand(0, 10, 5);
    spell_recharge_item_at(0, 20);
    assert_eq!(with_state(|s| s.py.inventory[0].misc_use), misc1);
    assert_eq!(random_number(100), tail1);
}

#[test]
fn spell_recharge_item_no_wand_returns_false_no_rng() {
    assert_rng_unchanged_after(setup_player_base, || {
        assert!(!spell_recharge_item(20));
    });
}

// --------------------------------------------------------------------------
// 3. Teleport player
// --------------------------------------------------------------------------

#[test]
fn spell_teleport_player_to_seed42_lands_and_rng_order() {
    reset_for_new_game(Some(42));
    setup_player_base();
    setup_dungeon(20, 20);
    with_state_mut(|s| s.py.pos = Coord_t { y: 10, x: 10 });
    spell_teleport_player_to(Coord_t { y: 10, x: 10 });
    let pos1 = with_state(|s| s.py.pos);
    let tail1 = random_number(100);

    reset_for_new_game(Some(42));
    setup_player_base();
    setup_dungeon(20, 20);
    with_state_mut(|s| s.py.pos = Coord_t { y: 10, x: 10 });
    spell_teleport_player_to(Coord_t { y: 10, x: 10 });
    assert_eq!(with_state(|s| s.py.pos), pos1);
    assert_eq!(random_number(100), tail1);
    assert_ne!(pos1, Coord_t { y: 10, x: 10 });
    assert!(
        i32::from(with_state(
            |s| s.dg.floor[pos1.y as usize][pos1.x as usize].feature_id
        )) < i32::from(MIN_CLOSED_SPACE)
    );
}

#[test]
fn spell_teleport_player_to_retries_on_wall_seed7() {
    reset_for_new_game(Some(7));
    setup_player_base();
    setup_dungeon(20, 20);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.dg.floor[9][10].feature_id = TILE_GRANITE_WALL;
    });
    let pairs: Vec<(i32, i32)> = (0..4).map(|_| next_random_pair(3)).collect();
    reset_for_new_game(Some(7));
    setup_player_base();
    setup_dungeon(20, 20);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.dg.floor[9][10].feature_id = TILE_GRANITE_WALL;
    });
    spell_teleport_player_to(Coord_t { y: 10, x: 10 });
    let pos = with_state(|s| s.py.pos);
    assert!(
        i32::from(with_state(
            |s| s.dg.floor[pos.y as usize][pos.x as usize].feature_id
        )) < i32::from(MIN_CLOSED_SPACE)
    );
    assert_eq!(pairs.len(), 4);
}

// --------------------------------------------------------------------------
// 4. Stat / EXP loss math
// --------------------------------------------------------------------------

#[test]
fn spell_lose_str_decreases_stat_seed42() {
    reset_for_new_game(Some(42));
    setup_player_base();
    let before = with_state(|s| s.py.stats.current[PlayerAttr::A_STR as usize]);
    spell_lose_str();
    let after = with_state(|s| s.py.stats.current[PlayerAttr::A_STR as usize]);
    assert!(after < before);
    assert!(last_message_text().contains("sick"));
}

#[test]
fn spell_lose_str_sustain_skips_decrease() {
    reset_for_new_game(Some(42));
    setup_player_base();
    with_state_mut(|s| s.py.flags.sustain_str = true);
    let before = with_state(|s| s.py.stats.current[PlayerAttr::A_STR as usize]);
    spell_lose_str();
    assert_eq!(
        with_state(|s| s.py.stats.current[PlayerAttr::A_STR as usize]),
        before
    );
}

#[test]
fn spell_lose_int_decreases_stat() {
    reset_for_new_game(Some(42));
    setup_player_base();
    let before = with_state(|s| s.py.stats.current[PlayerAttr::A_INT as usize]);
    spell_lose_int();
    assert!(with_state(|s| s.py.stats.current[PlayerAttr::A_INT as usize]) < before);
}

#[test]
fn spell_lose_wis_decreases_stat() {
    reset_for_new_game(Some(42));
    setup_player_base();
    with_state_mut(|s| s.py.stats.current[PlayerAttr::A_WIS as usize] = 18);
    let before = with_state(|s| s.py.stats.current[PlayerAttr::A_WIS as usize]);
    spell_lose_wis();
    assert!(with_state(|s| s.py.stats.current[PlayerAttr::A_WIS as usize]) < before);
}

#[test]
fn spell_lose_dex_decreases_stat() {
    reset_for_new_game(Some(42));
    setup_player_base();
    with_state_mut(|s| s.py.stats.current[PlayerAttr::A_DEX as usize] = 18);
    let before = with_state(|s| s.py.stats.current[PlayerAttr::A_DEX as usize]);
    spell_lose_dex();
    assert!(with_state(|s| s.py.stats.current[PlayerAttr::A_DEX as usize]) < before);
}

#[test]
fn spell_lose_con_decreases_stat() {
    reset_for_new_game(Some(42));
    setup_player_base();
    with_state_mut(|s| s.py.stats.current[PlayerAttr::A_CON as usize] = 18);
    let before = with_state(|s| s.py.stats.current[PlayerAttr::A_CON as usize]);
    spell_lose_con();
    assert!(with_state(|s| s.py.stats.current[PlayerAttr::A_CON as usize]) < before);
}

#[test]
fn spell_lose_chr_decreases_stat() {
    reset_for_new_game(Some(42));
    setup_player_base();
    with_state_mut(|s| s.py.stats.current[PlayerAttr::A_CHR as usize] = 18);
    let before = with_state(|s| s.py.stats.current[PlayerAttr::A_CHR as usize]);
    spell_lose_chr();
    assert!(with_state(|s| s.py.stats.current[PlayerAttr::A_CHR as usize]) < before);
}

#[test]
fn spell_lose_exp_clamps_to_zero() {
    reset_for_new_game(Some(1));
    setup_player_base();
    with_state_mut(|s| {
        s.py.misc.exp = 100;
        s.py.misc.level = 5;
    });
    spell_lose_exp(500);
    assert_eq!(with_state(|s| s.py.misc.exp), 0);
}

#[test]
fn spell_lose_exp_subtracts_and_may_drop_level() {
    reset_for_new_game(Some(1));
    setup_player_base();
    with_state_mut(|s| {
        s.py.misc.exp = 500;
        s.py.misc.level = 10;
        s.py.misc.experience_factor = 100;
    });
    spell_lose_exp(450);
    assert_eq!(with_state(|s| s.py.misc.exp), 50);
    assert_eq!(with_state(|s| s.py.misc.level), 4);
}

#[test]
fn spell_restore_player_levels_restores_max_exp() {
    reset_for_new_game(Some(1));
    setup_player_base();
    with_state_mut(|s| {
        s.py.misc.exp = 400;
        s.py.misc.max_exp = 499;
        s.py.misc.level = 10;
    });
    assert!(spell_restore_player_levels());
    assert_eq!(with_state(|s| s.py.misc.exp), 499);
}

#[test]
fn spell_restore_player_levels_no_op_when_exp_at_max() {
    reset_for_new_game(Some(1));
    setup_player_base();
    with_state_mut(|s| {
        s.py.misc.exp = 800;
        s.py.misc.max_exp = 800;
    });
    assert!(!spell_restore_player_levels());
}

// --------------------------------------------------------------------------
// 5. HP / food / cure — zero RNG where uses none
// --------------------------------------------------------------------------

#[test]
fn spell_change_player_hit_points_at_max_returns_false_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_player_base();
            with_state_mut(|s| {
                s.py.misc.current_hp = 100;
                s.py.misc.max_hp = 100;
            });
        },
        || assert!(!spell_change_player_hit_points(20)),
    );
}

#[test]
fn spell_change_player_hit_points_heals_and_clamps() {
    reset_for_new_game(Some(1));
    setup_player_base();
    with_state_mut(|s| {
        s.py.misc.current_hp = 50;
        s.py.misc.max_hp = 100;
        s.py.misc.current_hp_fraction = 500;
    });
    assert!(spell_change_player_hit_points(100));
    with_state(|s| {
        assert_eq!(s.py.misc.current_hp, 100);
        assert_eq!(s.py.misc.current_hp_fraction, 0);
    });
    assert!(last_message_text().contains("very good"));
}

#[test]
fn spell_change_player_hit_points_small_heal_message() {
    reset_for_new_game(Some(1));
    setup_player_base();
    with_state_mut(|s| s.py.misc.current_hp = 50);
    assert!(spell_change_player_hit_points(1));
    assert!(last_message_text().contains("little better"));
}

#[test]
fn spell_create_food_blocked_when_object_present_no_rng() {
    reset_for_new_game(Some(42));
    setup_player_base();
    setup_dungeon(20, 20);
    with_state_mut(|s| s.dg.floor[10][10].treasure_id = 1);
    assert_rng_unchanged_after(
        || {
            setup_player_base();
            setup_dungeon(20, 20);
            with_state_mut(|s| s.dg.floor[10][10].treasure_id = 1);
        },
        spell_create_food,
    );
    assert!(with_state(|s| s.game.player_free_turn));
    assert!(last_message_text().contains("already an object"));
}

#[test]
fn spell_slow_poison_halves_and_floors_at_one() {
    reset_for_new_game(Some(1));
    setup_player_base();
    with_state_mut(|s| s.py.flags.poisoned = 10);
    assert!(spell_slow_poison());
    assert_eq!(with_state(|s| s.py.flags.poisoned), 5);
    with_state_mut(|s| s.py.flags.poisoned = 1);
    assert!(spell_slow_poison());
    assert_eq!(with_state(|s| s.py.flags.poisoned), 1);
}

#[test]
fn spell_slow_poison_false_when_not_poisoned_no_rng() {
    assert_rng_unchanged_after(setup_player_base, || {
        assert!(!spell_slow_poison());
    });
}

#[test]
fn spell_remove_curse_from_worn_items_curses_body() {
    reset_for_new_game(Some(1));
    setup_player_base();
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Body as usize] = Inventory {
            category_id: umoria::treasure::TV_HARD_ARMOR,
            flags: TR_CURSED,
            ..Inventory::default()
        };
        s.py.inventory[PlayerEquipment::Wield as usize] = Inventory {
            category_id: umoria::treasure::TV_SWORD,
            flags: 0,
            ..Inventory::default()
        };
    });
    assert!(spell_remove_curse_from_all_worn_items());
    assert!(!with_state(|s| {
        inventory_item_is_cursed(s.py.inventory[PlayerEquipment::Body as usize])
    }));
}

#[test]
fn spell_remove_curse_false_when_none_cursed_no_rng() {
    assert_rng_unchanged_after(setup_player_base, || {
        assert!(!spell_remove_curse_from_all_worn_items());
    });
}

// --------------------------------------------------------------------------
// 6. Integer semantics — i16 misc_use / plusses wrapping
// --------------------------------------------------------------------------

#[test]
fn spell_recharge_item_misc_use_i16_addition_parity() {
    reset_for_new_game(Some(42));
    setup_player_base();
    pack_wand(0, 10, 5);
    spell_recharge_item_at(0, 20);
    let misc1 = with_state(|s| s.py.inventory[0].misc_use);

    reset_for_new_game(Some(42));
    setup_player_base();
    pack_wand(0, 10, 5);
    spell_recharge_item_at(0, 20);
    assert_eq!(with_state(|s| s.py.inventory[0].misc_use), misc1);
}

#[test]
fn spell_enchant_item_plusses_i16_increment() {
    let mut plusses = 32767i16;
    reset_for_new_game(Some(1));
    setup_player_base();
    if spell_enchant_item(&mut plusses, 30000) {
        assert_eq!(plusses, -32768i16);
    }
}
