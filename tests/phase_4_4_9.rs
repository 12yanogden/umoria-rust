//! Phase 4.4.9 — player_pray.cpp parity.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::data_player::MAGIC_SPELLS;
use umoria::dice::{dice_roll, Dice};
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::player::PlayerAttr;
use umoria::player::PLAYER_MAX_LEVEL;
use umoria::player_pray::{player_can_pray, player_recite_prayer, pray};
use umoria::spells::spell_chance_of_success;
use umoria::treasure::TV_PRAYER_BOOK;
use umoria::types::{Coord_t, MESSAGE_HISTORY_SIZE};
use umoria::ui::panel_bounds_fields;
use umoria::ui_io::{
    test_clear_getch_keys, test_push_getch_keys, test_set_direction, test_set_ncurses_stub, ESCAPE,
};

const PRIEST_CLASS_ID: u8 = 2;
const WARRIOR_CLASS_ID: u8 = 0;

const DETECT_EVIL_BIT: u32 = 1;

fn message_text(id: i16) -> String {
    with_state(|s| {
        let idx = id.rem_euclid(MESSAGE_HISTORY_SIZE as i16) as usize;
        let msg = &s.messages[idx];
        let end = msg.iter().position(|&b| b == 0).unwrap_or(msg.len());
        String::from_utf8_lossy(&msg[..end]).into_owned()
    })
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

fn setup_dungeon(height: i16, width: i16, pos: Coord_t) {
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                s.dg.floor[y as usize][x as usize].feature_id = TILE_LIGHT_FLOOR;
            }
        }
        s.dg.floor[pos.y as usize][pos.x as usize].permanent_light = true;
    });
}

fn sample_base_exp_levels() -> [u32; PLAYER_MAX_LEVEL as usize] {
    [
        10, 25, 45, 70, 100, 140, 200, 280, 380, 500, 650, 850, 1100, 1400, 1800, 2300, 2900, 3600,
        4400, 5400, 6800, 8400, 10200, 12500, 17500, 25000, 35000, 50000, 75000, 100000, 150000,
        200000, 300000, 400000, 500000, 750000, 1500000, 2500000, 5000000, 10000000,
    ]
}

fn setup_priest_caster(pos: Coord_t, current_mana: i16, spell_bit: u32) {
    test_set_ncurses_stub(true);
    let bounds = panel_bounds_fields(0, 0);
    with_state_mut(|s| {
        s.py.pos = pos;
        s.dg.panel.row = 0;
        s.dg.panel.col = 0;
        s.dg.panel.top = bounds.top;
        s.dg.panel.bottom = bounds.bottom;
        s.dg.panel.left = bounds.left;
        s.dg.panel.right = bounds.right;
        s.dg.panel.row_prt = bounds.row_prt;
        s.dg.panel.col_prt = bounds.col_prt;
        s.py.misc.class_id = PRIEST_CLASS_ID;
        s.py.misc.level = 10;
        s.py.misc.current_mana = current_mana;
        s.py.misc.current_hp = 50;
        s.py.misc.max_hp = 100;
        s.py.misc.exp = 0;
        s.py.misc.experience_factor = 100;
        s.py.base_exp_levels = sample_base_exp_levels();
        s.py.stats.used[PlayerAttr::A_WIS as usize] = 18;
        s.py.flags.spells_learnt = spell_bit;
        s.py.flags.spells_worked = 0;
        s.py.flags.blessed = 0;
        s.py.flags.detect_invisible = 0;
        s.py.flags.heat_resistance = 0;
        s.py.flags.cold_resistance = 0;
        s.py.inventory[0] = umoria::inventory::Inventory {
            category_id: TV_PRAYER_BOOK,
            flags: spell_bit,
            ..Default::default()
        };
        s.py.pack.unique_items = 1;
        s.game.player_free_turn = false;
        s.message_ready_to_print = false;
    });
}

fn setup_recite_base(pos: Coord_t) {
    test_set_ncurses_stub(true);
    let bounds = panel_bounds_fields(0, 0);
    with_state_mut(|s| {
        s.py.pos = pos;
        s.dg.panel.top = bounds.top;
        s.dg.panel.bottom = bounds.bottom;
        s.dg.panel.left = bounds.left;
        s.dg.panel.right = bounds.right;
        s.dg.panel.row_prt = bounds.row_prt;
        s.dg.panel.col_prt = bounds.col_prt;
        s.py.misc.class_id = PRIEST_CLASS_ID;
        s.py.misc.level = 10;
        s.py.misc.current_hp = 50;
        s.py.misc.max_hp = 100;
        s.py.flags.blessed = 0;
        s.py.flags.detect_invisible = 0;
        s.py.flags.heat_resistance = 0;
        s.py.flags.cold_resistance = 0;
    });
}

fn cast_detect_evil_from_book() {
    test_clear_getch_keys();
    test_push_getch_keys(&[b'a' as i32, b'a' as i32]);
    pray();
}

fn probe_failure_roll(seed: u32) -> (i32, i32) {
    reset_for_new_game(Some(seed));
    let pos = Coord_t { y: 15, x: 20 };
    setup_dungeon(66, 66, pos);
    setup_priest_caster(pos, 100, DETECT_EVIL_BIT);
    let chance = spell_chance_of_success(0);
    (random_number(100), chance)
}

fn find_success_seed() -> u32 {
    for seed in 1..256u32 {
        let (roll, chance) = probe_failure_roll(seed);
        if roll >= chance {
            return seed;
        }
    }
    panic!("no success seed found");
}

fn find_failure_seed() -> u32 {
    for seed in 1..256u32 {
        let (roll, chance) = probe_failure_roll(seed);
        if roll < chance {
            return seed;
        }
    }
    panic!("no failure seed found");
}

fn find_faint_seed() -> u32 {
    for seed in 1..256u32 {
        let (roll, chance) = probe_failure_roll(seed);
        if roll >= chance {
            reset_for_new_game(Some(seed));
            let pos = Coord_t { y: 15, x: 20 };
            setup_dungeon(66, 66, pos);
            setup_priest_caster(pos, 0, DETECT_EVIL_BIT);
            random_number(100);
            return seed;
        }
    }
    panic!("no faint seed found");
}

fn assert_pray_rng_unchanged_after(seed: u32, keys: &[i32], setup: impl Fn()) {
    reset_for_new_game(Some(seed));
    setup();
    test_clear_getch_keys();
    test_push_getch_keys(keys);
    pray();
    let baseline = random_number(100);

    reset_for_new_game(Some(seed));
    setup();
    test_clear_getch_keys();
    test_push_getch_keys(keys);
    pray();
    assert_eq!(random_number(100), baseline);
}

fn assert_recite_rng_unchanged_after(seed: u32, choice: i32, setup: impl Fn()) {
    reset_for_new_game(Some(seed));
    setup();
    player_recite_prayer(choice);
    let baseline = random_number(100);

    reset_for_new_game(Some(seed));
    setup();
    player_recite_prayer(choice);
    assert_eq!(random_number(100), baseline);
}

// ---------------------------------------------------------------------------
// 1. playerCanPray gating — no RNG
// ---------------------------------------------------------------------------

#[test]
fn player_can_pray_blind_blocks_without_rng() {
    assert_rng_unchanged_after(
        || {
            with_state_mut(|s| {
                s.py.flags.blind = 1;
                s.py.misc.class_id = PRIEST_CLASS_ID;
                s.message_ready_to_print = false;
            });
        },
        || {
            let mut begin = 0;
            let mut end = 0;
            assert!(!player_can_pray(&mut begin, &mut end));
        },
    );
}

#[test]
fn player_can_pray_no_light_blocks_without_rng() {
    reset_for_new_game(Some(7));
    with_state_mut(|s| {
        s.py.misc.class_id = PRIEST_CLASS_ID;
        s.py.pos = Coord_t { y: 5, x: 5 };
        s.dg.floor[5][5].permanent_light = false;
        s.dg.floor[5][5].temporary_light = false;
        s.message_ready_to_print = false;
    });
    assert_rng_unchanged_after(
        || {},
        || {
            let mut begin = 0;
            let mut end = 0;
            assert!(!player_can_pray(&mut begin, &mut end));
        },
    );
}

#[test]
fn player_can_pray_non_priest_message_without_rng() {
    assert_rng_unchanged_after(
        || {
            with_state_mut(|s| {
                s.py.misc.class_id = WARRIOR_CLASS_ID;
                s.py.pos = Coord_t { y: 5, x: 5 };
                s.dg.floor[5][5].permanent_light = true;
                s.py.pack.unique_items = 1;
                s.message_ready_to_print = false;
            });
        },
        || {
            let mut begin = 0;
            let mut end = 0;
            assert!(!player_can_pray(&mut begin, &mut end));
            assert_eq!(
                message_text(with_state(|s| s.last_message_id)),
                "Pray hard enough and your prayers may be answered."
            );
        },
    );
}

#[test]
fn player_can_pray_no_items_blocks_without_rng() {
    reset_for_new_game(Some(7));
    with_state_mut(|s| {
        s.py.misc.class_id = PRIEST_CLASS_ID;
        s.py.pos = Coord_t { y: 5, x: 5 };
        s.dg.floor[5][5].permanent_light = true;
        s.py.pack.unique_items = 0;
        s.message_ready_to_print = false;
    });
    assert_rng_unchanged_after(
        || {},
        || {
            let mut begin = 0;
            let mut end = 0;
            assert!(!player_can_pray(&mut begin, &mut end));
        },
    );
}

// ---------------------------------------------------------------------------
// 2. RNG-order golden — pray() cast gate
// ---------------------------------------------------------------------------

#[test]
fn pray_success_rng_order_detect_evil() {
    let seed = find_success_seed();
    let pos = Coord_t { y: 15, x: 20 };
    assert_pray_rng_unchanged_after(seed, &[b'a' as i32, b'a' as i32], || {
        setup_dungeon(66, 66, pos);
        setup_priest_caster(pos, 100, DETECT_EVIL_BIT);
    });
    reset_for_new_game(Some(seed));
    setup_dungeon(66, 66, pos);
    setup_priest_caster(pos, 100, DETECT_EVIL_BIT);
    cast_detect_evil_from_book();
    assert_eq!(with_state(|s| s.py.misc.current_mana), 99);
}

#[test]
fn pray_failure_skips_effect_rng() {
    let seed = find_failure_seed();
    let pos = Coord_t { y: 15, x: 20 };
    assert_pray_rng_unchanged_after(seed, &[b'a' as i32, b'a' as i32], || {
        setup_dungeon(66, 66, pos);
        setup_priest_caster(pos, 100, DETECT_EVIL_BIT);
    });
    reset_for_new_game(Some(seed));
    setup_dungeon(66, 66, pos);
    setup_priest_caster(pos, 100, DETECT_EVIL_BIT);
    cast_detect_evil_from_book();
    assert_eq!(
        message_text(with_state(|s| s.last_message_id)),
        "You lost your concentration!"
    );
    assert_eq!(with_state(|s| s.py.misc.current_mana), 99);
}

#[test]
fn pray_faint_rng_order_overdraw_only() {
    let seed = find_faint_seed();
    let pos = Coord_t { y: 15, x: 20 };
    let faint_keys = [b'a' as i32, b'a' as i32, b'y' as i32];
    assert_pray_rng_unchanged_after(seed, &faint_keys, || {
        setup_dungeon(66, 66, pos);
        setup_priest_caster(pos, 0, DETECT_EVIL_BIT);
    });
    reset_for_new_game(Some(seed));
    setup_dungeon(66, 66, pos);
    setup_priest_caster(pos, 0, DETECT_EVIL_BIT);
    test_clear_getch_keys();
    test_push_getch_keys(&faint_keys);
    pray();
    assert_eq!(with_state(|s| s.py.misc.current_mana), 0);
    assert!(with_state(|s| s.py.flags.paralysis) >= 0);
}

#[test]
fn faint_paralysis_truncates_to_i16() {
    let raw = 40000i32;
    assert_eq!(
        raw as i16,
        with_state_mut(|s| {
            s.py.flags.paralysis = raw as i16;
            s.py.flags.paralysis
        })
    );
}

// ---------------------------------------------------------------------------
// 3. RNG-order golden — playerRecitePrayer per effect
// ---------------------------------------------------------------------------

#[test]
fn player_recite_cure_light_rng_order_seed42() {
    let pos = Coord_t { y: 15, x: 20 };
    assert_recite_rng_unchanged_after(42, 1, || {
        setup_dungeon(66, 66, pos);
        setup_recite_base(pos);
    });
    reset_for_new_game(Some(42));
    setup_dungeon(66, 66, pos);
    setup_recite_base(pos);
    let heal = dice_roll(Dice { dice: 3, sides: 3 });
    let expected_next = random_number(100);
    reset_for_new_game(Some(42));
    setup_dungeon(66, 66, pos);
    setup_recite_base(pos);
    let hp_before = with_state(|s| s.py.misc.current_hp);
    player_recite_prayer(1);
    assert_eq!(
        with_state(|s| s.py.misc.current_hp),
        hp_before + heal as i16
    );
    assert_eq!(random_number(100), expected_next);
}

#[test]
fn player_recite_bless_rng_order_seed42() {
    let pos = Coord_t { y: 15, x: 20 };
    reset_for_new_game(Some(42));
    setup_recite_base(pos);
    let adj = random_number(12) + 12;
    let expected_next = random_number(100);
    reset_for_new_game(Some(42));
    setup_recite_base(pos);
    player_recite_prayer(2);
    assert_eq!(with_state(|s| s.py.flags.blessed), adj as i16);
    assert_eq!(random_number(100), expected_next);
}

#[test]
fn player_recite_cure_medium_rng_order_seed42() {
    let pos = Coord_t { y: 15, x: 20 };
    assert_recite_rng_unchanged_after(42, 10, || {
        setup_dungeon(66, 66, pos);
        setup_recite_base(pos);
    });
}

#[test]
fn player_recite_chant_rng_order_seed42() {
    let pos = Coord_t { y: 15, x: 20 };
    reset_for_new_game(Some(42));
    setup_recite_base(pos);
    let adj = random_number(24) + 24;
    let expected_next = random_number(100);
    reset_for_new_game(Some(42));
    setup_recite_base(pos);
    player_recite_prayer(11);
    assert_eq!(with_state(|s| s.py.flags.blessed), adj as i16);
    assert_eq!(random_number(100), expected_next);
}

#[test]
fn player_recite_resist_heat_cold_rng_order_seed42() {
    let pos = Coord_t { y: 15, x: 20 };
    reset_for_new_game(Some(42));
    setup_recite_base(pos);
    let heat = random_number(10) + 10;
    let cold = random_number(10) + 10;
    let expected_next = random_number(100);
    reset_for_new_game(Some(42));
    setup_recite_base(pos);
    player_recite_prayer(15);
    assert_eq!(with_state(|s| s.py.flags.heat_resistance), heat as i16);
    assert_eq!(with_state(|s| s.py.flags.cold_resistance), cold as i16);
    assert_eq!(random_number(100), expected_next);
}

#[test]
fn player_recite_orb_rng_order_seed42() {
    let pos = Coord_t { y: 15, x: 20 };
    test_set_direction(Some(6));
    assert_recite_rng_unchanged_after(42, 17, || {
        setup_dungeon(66, 66, pos);
        setup_recite_base(pos);
        test_set_direction(Some(6));
    });
}

#[test]
fn player_recite_cure_serious_rng_order_seed42() {
    let pos = Coord_t { y: 15, x: 20 };
    assert_recite_rng_unchanged_after(42, 18, || {
        setup_dungeon(66, 66, pos);
        setup_recite_base(pos);
    });
}

#[test]
fn player_recite_sense_invisible_rng_order_seed42() {
    let pos = Coord_t { y: 15, x: 20 };
    reset_for_new_game(Some(42));
    setup_recite_base(pos);
    let adj = random_number(24) + 24;
    let expected_next = random_number(100);
    reset_for_new_game(Some(42));
    setup_recite_base(pos);
    player_recite_prayer(19);
    assert_eq!(with_state(|s| s.py.flags.detect_invisible), adj as i16);
    assert_eq!(random_number(100), expected_next);
}

#[test]
fn player_recite_cure_critical_rng_order_seed42() {
    let pos = Coord_t { y: 15, x: 20 };
    assert_recite_rng_unchanged_after(42, 23, || {
        setup_dungeon(66, 66, pos);
        setup_recite_base(pos);
    });
}

#[test]
fn player_recite_prayer_bless_rng_order_seed42() {
    let pos = Coord_t { y: 15, x: 20 };
    reset_for_new_game(Some(42));
    setup_recite_base(pos);
    let adj = random_number(48) + 48;
    let expected_next = random_number(100);
    reset_for_new_game(Some(42));
    setup_recite_base(pos);
    player_recite_prayer(25);
    assert_eq!(with_state(|s| s.py.flags.blessed), adj as i16);
    assert_eq!(random_number(100), expected_next);
}

#[test]
fn player_recite_heal_fixed_amount_no_extra_rng() {
    let pos = Coord_t { y: 15, x: 20 };
    assert_rng_unchanged_after(
        || {
            setup_dungeon(66, 66, pos);
            setup_recite_base(pos);
        },
        || player_recite_prayer(27),
    );
    with_state(|s| assert_eq!(s.py.misc.current_hp, 100));
}

#[test]
fn player_recite_detect_evil_consumes_no_rng() {
    let pos = Coord_t { y: 15, x: 20 };
    assert_rng_unchanged_after(
        || {
            setup_dungeon(66, 66, pos);
            setup_recite_base(pos);
        },
        || player_recite_prayer(0),
    );
}

// ---------------------------------------------------------------------------
// 4. EXP-learn bookkeeping + early exits
// ---------------------------------------------------------------------------

#[test]
fn first_successful_pray_awards_exp_once() {
    let seed = find_success_seed();
    let pos = Coord_t { y: 15, x: 20 };
    reset_for_new_game(Some(seed));
    setup_dungeon(66, 66, pos);
    setup_priest_caster(pos, 100, DETECT_EVIL_BIT);
    cast_detect_evil_from_book();

    let gain = i32::from(MAGIC_SPELLS[PRIEST_CLASS_ID as usize - 1][0].exp_gain_for_learning) << 2;
    assert_eq!(with_state(|s| s.py.misc.exp), gain);
    assert_ne!(
        with_state(|s| s.py.flags.spells_worked & DETECT_EVIL_BIT),
        0
    );

    let exp_after_first = with_state(|s| s.py.misc.exp);
    cast_detect_evil_from_book();
    assert_eq!(with_state(|s| s.py.misc.exp), exp_after_first);
}

#[test]
fn pray_escape_at_prayer_prompt_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            let pos = Coord_t { y: 15, x: 20 };
            setup_dungeon(66, 66, pos);
            setup_priest_caster(pos, 100, DETECT_EVIL_BIT);
            test_clear_getch_keys();
            test_push_getch_keys(&[b'a' as i32, i32::from(ESCAPE)]);
        },
        pray,
    );
}

#[test]
fn pray_no_known_prayers_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            let pos = Coord_t { y: 15, x: 20 };
            setup_dungeon(66, 66, pos);
            setup_priest_caster(pos, 100, 0);
            test_clear_getch_keys();
            test_push_getch_keys(&[b'a' as i32, b'a' as i32]);
        },
        pray,
    );
}
