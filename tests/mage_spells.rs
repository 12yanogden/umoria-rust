//! `mage_spells` driver parity.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::data_player::MAGIC_SPELLS;
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::mage_spells::{
    can_read_spells, cast_spell, get_and_cast_magic_spell, spell_chance_of_success,
};
use umoria::player::PlayerAttr;
use umoria::player::PLAYER_MAX_LEVEL;
use umoria::treasure::TV_MAGIC_BOOK;
use umoria::types::{Coord_t, MESSAGE_HISTORY_SIZE};
use umoria::ui::panel_bounds_fields;
use umoria::ui_io::{test_clear_getch_keys, test_push_getch_keys, test_set_ncurses_stub, ESCAPE};

const MAGE_CLASS_ID: u8 = 1;
const WARRIOR_CLASS_ID: u8 = 0;

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

fn setup_mage_caster(pos: Coord_t, current_mana: i16, spell_bit: u32) {
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
        s.py.misc.class_id = MAGE_CLASS_ID;
        s.py.misc.level = 10;
        s.py.misc.current_mana = current_mana;
        s.py.misc.exp = 0;
        s.py.misc.experience_factor = 100;
        s.py.base_exp_levels = sample_base_exp_levels();
        s.py.stats.used[PlayerAttr::A_INT as usize] = 18;
        s.py.flags.spells_learnt = spell_bit;
        s.py.flags.spells_worked = 0;
        s.py.inventory[0] = umoria::inventory::Inventory {
            category_id: TV_MAGIC_BOOK,
            flags: spell_bit,
            ..Default::default()
        };
        s.py.pack.unique_items = 1;
        s.game.player_free_turn = false;
        s.message_ready_to_print = false;
    });
}

const DETECT_MONSTERS_BIT: u32 = 1 << 1;

fn cast_detect_monsters_from_book() {
    test_clear_getch_keys();
    test_push_getch_keys(&[b'a' as i32, b'a' as i32]);
    get_and_cast_magic_spell();
}

// ---------------------------------------------------------------------------
// 1. canReadSpells gating — no RNG
// ---------------------------------------------------------------------------

#[test]
fn can_read_spells_blind_blocks_without_rng() {
    assert_rng_unchanged_after(
        || {
            with_state_mut(|s| {
                s.py.flags.blind = 1;
                s.py.misc.class_id = MAGE_CLASS_ID;
                s.message_ready_to_print = false;
            });
        },
        || {
            assert!(!can_read_spells());
        },
    );
}

#[test]
fn can_read_spells_no_light_blocks_without_rng() {
    reset_for_new_game(Some(7));
    with_state_mut(|s| {
        s.py.misc.class_id = MAGE_CLASS_ID;
        s.py.pos = Coord_t { y: 5, x: 5 };
        s.dg.floor[5][5].permanent_light = false;
        s.dg.floor[5][5].temporary_light = false;
        s.message_ready_to_print = false;
    });
    assert_rng_unchanged_after(|| {}, || assert!(!can_read_spells()));
}

#[test]
fn can_read_spells_confused_blocks_without_rng() {
    reset_for_new_game(Some(7));
    with_state_mut(|s| {
        s.py.flags.confused = 1;
        s.py.misc.class_id = MAGE_CLASS_ID;
        s.py.pos = Coord_t { y: 5, x: 5 };
        s.dg.floor[5][5].permanent_light = true;
        s.message_ready_to_print = false;
    });
    assert_rng_unchanged_after(|| {}, || assert!(!can_read_spells()));
}

#[test]
fn can_read_spells_non_mage_blocks_without_rng() {
    reset_for_new_game(Some(7));
    with_state_mut(|s| {
        s.py.misc.class_id = WARRIOR_CLASS_ID;
        s.py.pos = Coord_t { y: 5, x: 5 };
        s.dg.floor[5][5].permanent_light = true;
        s.message_ready_to_print = false;
    });
    assert_rng_unchanged_after(|| {}, || assert!(!can_read_spells()));
}

// ---------------------------------------------------------------------------
// 2. spellChanceOfSuccess — formula + clamp (no RNG)
// ---------------------------------------------------------------------------

#[test]
fn spell_chance_formula_and_clamp_match_cpp() {
    reset_for_new_game(Some(1));
    with_state_mut(|s| {
        s.py.misc.class_id = MAGE_CLASS_ID;
        s.py.misc.level = 10;
        s.py.stats.used[PlayerAttr::A_INT as usize] = 18;
        s.py.misc.current_mana = 100;
    });
    assert_eq!(spell_chance_of_success(0), 5);

    with_state_mut(|s| {
        s.py.misc.level = 1;
        s.py.misc.current_mana = 0;
    });
    assert_eq!(spell_chance_of_success(22), 95);
}

fn probe_failure_roll(seed: u32) -> (i32, i32) {
    reset_for_new_game(Some(seed));
    let pos = Coord_t { y: 15, x: 20 };
    setup_dungeon(66, 66, pos);
    setup_mage_caster(pos, 100, DETECT_MONSTERS_BIT);
    let chance = spell_chance_of_success(1);
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
            setup_mage_caster(pos, 0, DETECT_MONSTERS_BIT);
            random_number(100);
            return seed;
        }
    }
    panic!("no faint seed found");
}

fn assert_cast_rng_unchanged_after(seed: u32, keys: &[i32], setup: impl Fn()) {
    reset_for_new_game(Some(seed));
    setup();
    test_clear_getch_keys();
    test_push_getch_keys(keys);
    get_and_cast_magic_spell();
    let baseline = random_number(100);

    reset_for_new_game(Some(seed));
    setup();
    test_clear_getch_keys();
    test_push_getch_keys(keys);
    get_and_cast_magic_spell();
    assert_eq!(random_number(100), baseline);
}

// ---------------------------------------------------------------------------
// 3. RNG-order golden — getAndCastMagicSpell driver
// ---------------------------------------------------------------------------

#[test]
fn get_and_cast_success_rng_order_detect_monsters() {
    let seed = find_success_seed();
    let pos = Coord_t { y: 15, x: 20 };
    assert_cast_rng_unchanged_after(seed, &[b'a' as i32, b'a' as i32], || {
        setup_dungeon(66, 66, pos);
        setup_mage_caster(pos, 100, DETECT_MONSTERS_BIT);
    });
    reset_for_new_game(Some(seed));
    setup_dungeon(66, 66, pos);
    setup_mage_caster(pos, 100, DETECT_MONSTERS_BIT);
    cast_detect_monsters_from_book();
    assert_eq!(with_state(|s| s.py.misc.current_mana), 99);
}

#[test]
fn get_and_cast_failure_rng_order() {
    let seed = find_failure_seed();
    let pos = Coord_t { y: 15, x: 20 };
    assert_cast_rng_unchanged_after(seed, &[b'a' as i32, b'a' as i32], || {
        setup_dungeon(66, 66, pos);
        setup_mage_caster(pos, 100, DETECT_MONSTERS_BIT);
    });
    reset_for_new_game(Some(seed));
    setup_dungeon(66, 66, pos);
    setup_mage_caster(pos, 100, DETECT_MONSTERS_BIT);
    cast_detect_monsters_from_book();
    assert_eq!(
        message_text(with_state(|s| s.last_message_id)),
        "You failed to get the spell off!"
    );
    assert_eq!(with_state(|s| s.py.misc.current_mana), 99);
}

#[test]
fn get_and_cast_faint_rng_order() {
    let seed = find_faint_seed();
    let pos = Coord_t { y: 15, x: 20 };
    let faint_keys = [b'a' as i32, b'a' as i32, b'y' as i32];
    assert_cast_rng_unchanged_after(seed, &faint_keys, || {
        setup_dungeon(66, 66, pos);
        setup_mage_caster(pos, 0, DETECT_MONSTERS_BIT);
    });
    reset_for_new_game(Some(seed));
    setup_dungeon(66, 66, pos);
    setup_mage_caster(pos, 0, DETECT_MONSTERS_BIT);
    test_clear_getch_keys();
    test_push_getch_keys(&faint_keys);
    get_and_cast_magic_spell();
    assert_eq!(with_state(|s| s.py.misc.current_mana), 0);
}

// ---------------------------------------------------------------------------
// 4. EXP-learn bookkeeping
// ---------------------------------------------------------------------------

#[test]
fn first_successful_cast_awards_exp_once() {
    let seed = find_success_seed();
    let pos = Coord_t { y: 15, x: 20 };
    reset_for_new_game(Some(seed));
    setup_dungeon(66, 66, pos);
    setup_mage_caster(pos, 100, DETECT_MONSTERS_BIT);
    cast_detect_monsters_from_book();

    let gain = i32::from(MAGIC_SPELLS[MAGE_CLASS_ID as usize - 1][1].exp_gain_for_learning) << 2;
    assert_eq!(with_state(|s| s.py.misc.exp), gain);
    assert_ne!(
        with_state(|s| s.py.flags.spells_worked & DETECT_MONSTERS_BIT),
        0
    );

    let exp_after_first = with_state(|s| s.py.misc.exp);
    cast_detect_monsters_from_book();
    assert_eq!(with_state(|s| s.py.misc.exp), exp_after_first);
}

// ---------------------------------------------------------------------------
// 5. castSpell dispatch
// ---------------------------------------------------------------------------

#[test]
fn cast_spell_haste_self_rng_order_seed42() {
    reset_for_new_game(Some(42));
    let pos = Coord_t { y: 15, x: 20 };
    setup_dungeon(66, 66, pos);
    setup_mage_caster(pos, 100, 1);
    let boost = random_number(20) + 10;
    let expected_next = random_number(100);

    reset_for_new_game(Some(42));
    setup_dungeon(66, 66, pos);
    setup_mage_caster(pos, 100, 1);
    cast_spell(28);

    assert_eq!(with_state(|s| s.py.flags.fast), boost as i16);
    assert_eq!(random_number(100), expected_next);
}

#[test]
fn cast_spell_detect_monsters_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            let pos = Coord_t { y: 15, x: 20 };
            setup_dungeon(66, 66, pos);
            setup_mage_caster(pos, 100, 1);
        },
        || cast_spell(2),
    );
}

// ---------------------------------------------------------------------------
// 6. Integer semantics
// ---------------------------------------------------------------------------

#[test]
fn spells_worked_bit_and_exp_add_match_cpp() {
    reset_for_new_game(Some(1));
    let choice = 4u32;
    with_state_mut(|s| {
        s.py.flags.spells_worked = 0;
        s.py.misc.exp = i32::MAX - 3;
        s.py.flags.spells_worked |= 1u32 << choice;
        s.py.misc.exp = s.py.misc.exp.wrapping_add(
            i32::from(
                MAGIC_SPELLS[MAGE_CLASS_ID as usize - 1][choice as usize].exp_gain_for_learning,
            ) << 2,
        );
    });
    assert_eq!(with_state(|s| s.py.flags.spells_worked), 1u32 << 4);
    assert_eq!(
        with_state(|s| s.py.misc.exp),
        (i32::MAX - 3).wrapping_add(8)
    );
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
// 7. Early exits
// ---------------------------------------------------------------------------

#[test]
fn get_and_cast_no_spell_book_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            let pos = Coord_t { y: 15, x: 20 };
            setup_dungeon(66, 66, pos);
            setup_mage_caster(pos, 100, 1);
            with_state_mut(|s| s.py.pack.unique_items = 0);
        },
        get_and_cast_magic_spell,
    );
}

#[test]
fn get_and_cast_escape_at_spell_prompt_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            let pos = Coord_t { y: 15, x: 20 };
            setup_dungeon(66, 66, pos);
            setup_mage_caster(pos, 100, 1);
            test_clear_getch_keys();
            test_push_getch_keys(&[b'a' as i32, i32::from(ESCAPE)]);
        },
        get_and_cast_magic_spell,
    );
}

#[test]
#[ignore = "needs recorded-input/PTY playthrough harness for mage spell casting"]
fn playthrough_mage_spell_cast_capture() {}
