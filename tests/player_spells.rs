//! Player spell & mana learning parity.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use umoria::config::player::status::PY_STUDY;
use umoria::config::spells::{NAME_OFFSET_PRAYERS, SPELL_TYPE_MAGE};
use umoria::data_player::{CLASSES, MAGIC_SPELLS, SPELL_NAMES};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::player::{
    last_known_spell, new_mana, number_of_spells_allowed, number_of_spells_known,
    player_calculate_allowed_spells_count, player_can_read, player_determine_learnable_spells,
    player_gain_mana, player_gain_spells, player_no_light, PlayerAttr,
};
use umoria::player_stats::player_stat_adjustment_wisdom_intelligence;
use umoria::spells::Spell;
use umoria::treasure::TV_MAGIC_BOOK;
use umoria::types::Coord_t;
use umoria::ui_io::{test_clear_getch_keys, test_push_getch_keys, test_set_ncurses_stub};

const PRIEST_CLASS_ID: u8 = 2;
const MAGE_CLASS_ID: u8 = 1;

fn init_spell_order() {
    with_state_mut(|s| {
        s.py.flags.spells_learned_order = [99; 32];
    });
}

fn set_floor_light(coord: Coord_t, lit: bool) {
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize].permanent_light = lit;
        s.dg.floor[coord.y as usize][coord.x as usize].temporary_light = false;
    });
}

fn set_used_stat(stat: PlayerAttr, value: u8) {
    with_state_mut(|s| {
        s.py.stats.used[stat as usize] = value;
    });
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

fn cpp_wis_int_adj(value: i32) -> i32 {
    if value > 117 {
        7
    } else if value > 107 {
        6
    } else if value > 87 {
        5
    } else if value > 67 {
        4
    } else if value > 17 {
        3
    } else if value > 14 {
        2
    } else {
        i32::from(value > 7)
    }
}

fn cpp_new_mana(_stat: PlayerAttr, class_id: u8, level: u16, used_stat: u8) -> i32 {
    let adj = cpp_wis_int_adj(i32::from(used_stat));
    let levels =
        i32::from(level) - i32::from(CLASSES[class_id as usize].min_level_for_spell_casting) + 1;
    match adj {
        1 | 2 => levels,
        3 => 3 * levels / 2,
        4 => 2 * levels,
        5 => 5 * levels / 2,
        6 => 3 * levels,
        7 => 4 * levels,
        _ => 0,
    }
}

fn cpp_number_of_spells_allowed(_stat: PlayerAttr, class_id: u8, level: u16, used_stat: u8) -> i32 {
    let adj = cpp_wis_int_adj(i32::from(used_stat));
    let levels =
        i32::from(level) - i32::from(CLASSES[class_id as usize].min_level_for_spell_casting) + 1;
    match adj {
        1..=3 => levels,
        4 | 5 => 3 * levels / 2,
        6 => 2 * levels,
        7 => 5 * levels / 2,
        _ => 0,
    }
}

fn cpp_number_of_spells_known(spells_learnt: u32) -> i32 {
    let mut known = 0;
    let mut mask = 1u32;
    while mask != 0 {
        if spells_learnt & mask != 0 {
            known += 1;
        }
        mask <<= 1;
    }
    known
}

fn cpp_learnable_spells(
    spells: &[Spell; 31],
    level: u16,
    spells_learnt: u32,
    new_spells: i32,
) -> i32 {
    let mut spell_flag = 0x7FFF_FFFFu32 & !spells_learnt;
    let mut id = 0;
    let mut mask = 1u32;
    let mut i = 0;
    while spell_flag != 0 {
        if spell_flag & mask != 0 {
            spell_flag &= !mask;
            if spells[i].level_required <= level as u8 {
                id += 1;
            }
        }
        mask <<= 1;
        i += 1;
    }
    if new_spells > id {
        id
    } else {
        new_spells
    }
}

fn setup_base_player(class_id: u8, level: u16) {
    with_state_mut(|s| {
        s.py.misc.class_id = class_id;
        s.py.misc.level = level;
        s.py.pos = Coord_t { y: 5, x: 5 };
        s.py.flags.confused = 0;
        s.py.flags.blind = 0;
        s.py.flags.new_spells_to_learn = 0;
        s.py.flags.spells_learnt = 0;
        s.py.flags.spells_forgotten = 0;
        s.py.flags.status = 0;
        s.py.misc.mana = 0;
        s.py.misc.current_mana = 0;
        s.py.misc.current_mana_fraction = 0;
        s.py.pack.unique_items = 0;
    });
    init_spell_order();
    set_floor_light(Coord_t { y: 5, x: 5 }, true);
}

// ---------------------------------------------------------------------------
// 1. playerCanRead / lastKnownSpell / playerDetermineLearnableSpells
// ---------------------------------------------------------------------------

#[test]
fn player_can_read_false_when_blind() {
    reset_for_new_game(Some(42));
    test_set_ncurses_stub(true);
    setup_base_player(MAGE_CLASS_ID, 5);
    with_state_mut(|s| s.py.flags.blind = 1);
    assert!(!player_can_read());
}

#[test]
fn player_can_read_false_when_no_light() {
    reset_for_new_game(Some(42));
    test_set_ncurses_stub(true);
    setup_base_player(MAGE_CLASS_ID, 5);
    set_floor_light(Coord_t { y: 5, x: 5 }, false);
    assert!(player_no_light());
    assert!(!player_can_read());
}

#[test]
fn player_can_read_true_with_light() {
    reset_for_new_game(Some(42));
    setup_base_player(MAGE_CLASS_ID, 5);
    assert!(player_can_read());
}

#[test]
fn last_known_spell_finds_first_99_slot() {
    reset_for_new_game(None);
    init_spell_order();
    with_state_mut(|s| {
        s.py.flags.spells_learned_order[0] = 3;
        s.py.flags.spells_learned_order[1] = 7;
    });
    assert_eq!(last_known_spell(), 2);
}

#[test]
fn determine_learnable_spells_or_from_magic_books() {
    reset_for_new_game(None);
    setup_base_player(MAGE_CLASS_ID, 10);
    with_state_mut(|s| {
        s.py.pack.unique_items = 2;
        s.py.inventory[0].category_id = TV_MAGIC_BOOK;
        s.py.inventory[0].flags = 0b101;
        s.py.inventory[1].category_id = TV_MAGIC_BOOK;
        s.py.inventory[1].flags = 0b1000;
        s.py.inventory[2].category_id = 0; // not a book
        s.py.inventory[2].flags = 0xFFFF;
    });
    assert_eq!(player_determine_learnable_spells(), 0b1101);
}

// ---------------------------------------------------------------------------
// 2. newMana / playerGainMana
// ---------------------------------------------------------------------------

#[test]
fn new_mana_matches_cpp_formula() {
    reset_for_new_game(None);
    setup_base_player(MAGE_CLASS_ID, 10);
    set_used_stat(PlayerAttr::A_INT, 18);
    assert_eq!(
        new_mana(PlayerAttr::A_INT),
        cpp_new_mana(PlayerAttr::A_INT, MAGE_CLASS_ID, 10, 18)
    );
    assert_eq!(new_mana(PlayerAttr::A_INT), 15);
}

#[test]
fn player_gain_mana_sets_first_level_mana_with_plus_one() {
    reset_for_new_game(None);
    setup_base_player(MAGE_CLASS_ID, 5);
    set_used_stat(PlayerAttr::A_INT, 18);
    with_state_mut(|s| {
        s.py.flags.spells_learnt = 1;
        s.py.misc.mana = 0;
    });
    player_gain_mana(PlayerAttr::A_INT);
    with_state(|s| {
        assert_eq!(s.py.misc.mana, 8); // newMana=7 at level 5, +1 => 8
        assert_eq!(s.py.misc.current_mana, 8);
        assert_eq!(s.py.misc.current_mana_fraction, 0);
    });
}

#[test]
fn player_gain_mana_scales_current_proportionally() {
    reset_for_new_game(None);
    setup_base_player(MAGE_CLASS_ID, 10);
    set_used_stat(PlayerAttr::A_INT, 18);
    with_state_mut(|s| {
        s.py.flags.spells_learnt = 1;
        s.py.misc.mana = 10;
        s.py.misc.current_mana = 5;
        s.py.misc.current_mana_fraction = 0x8000;
    });
    player_gain_mana(PlayerAttr::A_INT);
    with_state(|s| {
        let expected_mana = i64::from(cpp_new_mana(PlayerAttr::A_INT, MAGE_CLASS_ID, 10, 18) + 1);
        let value = ((5i64 << 16) + 0x8000) / 10 * expected_mana;
        assert_eq!(s.py.misc.mana, expected_mana as i16);
        assert_eq!(s.py.misc.current_mana, (value >> 16) as i16);
        assert_eq!(s.py.misc.current_mana_fraction, (value & 0xFFFF) as u16);
    });
}

#[test]
fn player_gain_mana_clears_mana_when_no_spells_known() {
    reset_for_new_game(None);
    setup_base_player(MAGE_CLASS_ID, 5);
    with_state_mut(|s| {
        s.py.flags.spells_learnt = 0;
        s.py.misc.mana = 8;
        s.py.misc.current_mana = 4;
    });
    player_gain_mana(PlayerAttr::A_INT);
    with_state(|s| {
        assert_eq!(s.py.misc.mana, 0);
        assert_eq!(s.py.misc.current_mana, 0);
    });
}

// ---------------------------------------------------------------------------
// 3. playerCalculateAllowedSpellsCount helpers
// ---------------------------------------------------------------------------

#[test]
fn number_of_spells_allowed_and_known() {
    reset_for_new_game(None);
    setup_base_player(PRIEST_CLASS_ID, 10);
    set_used_stat(PlayerAttr::A_WIS, 18);
    assert_eq!(
        number_of_spells_allowed(PlayerAttr::A_WIS),
        cpp_number_of_spells_allowed(PlayerAttr::A_WIS, PRIEST_CLASS_ID, 10, 18)
    );
    with_state_mut(|s| s.py.flags.spells_learnt = 0b1010);
    assert_eq!(number_of_spells_known(), cpp_number_of_spells_known(0b1010));
    assert_eq!(number_of_spells_known(), 2);
}

#[test]
fn calculate_allowed_spells_remembers_forgotten_first() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    setup_base_player(PRIEST_CLASS_ID, 10);
    set_used_stat(PlayerAttr::A_WIS, 18);
    with_state_mut(|s| {
        s.py.flags.spells_learnt = 0;
        s.py.flags.spells_forgotten = 1 << 2;
        s.py.flags.spells_learned_order[0] = 2;
        s.py.flags.new_spells_to_learn = 0;
    });
    player_calculate_allowed_spells_count(PlayerAttr::A_WIS);
    with_state(|s| {
        assert_eq!(s.py.flags.spells_learnt, 1 << 2);
        assert_eq!(s.py.flags.spells_forgotten, 0);
        assert!(s.py.flags.status & PY_STUDY != 0);
    });
}

#[test]
fn calculate_allowed_spells_forgets_excess_in_reverse_order() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    setup_base_player(PRIEST_CLASS_ID, 3);
    set_used_stat(PlayerAttr::A_WIS, 18); // adj=3 => 3 allowed at level 3
    with_state_mut(|s| {
        s.py.flags.spells_learnt = 0b1111;
        s.py.flags.spells_learned_order = [99; 32];
        s.py.flags.spells_learned_order[0] = 0;
        s.py.flags.spells_learned_order[1] = 1;
        s.py.flags.spells_learned_order[2] = 2;
        s.py.flags.spells_learned_order[3] = 3;
    });
    player_calculate_allowed_spells_count(PlayerAttr::A_WIS);
    with_state(|s| {
        assert_eq!(s.py.flags.spells_learnt, 0b111);
        assert_eq!(s.py.flags.spells_forgotten, 1 << 3);
        assert_eq!(s.py.flags.new_spells_to_learn, 0);
    });
}

#[test]
fn calculate_allowed_spells_caps_by_learnable_count() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    setup_base_player(PRIEST_CLASS_ID, 5);
    set_used_stat(PlayerAttr::A_WIS, 18); // adj=3 => 5 allowed
    with_state_mut(|s| {
        s.py.flags.spells_learnt = 0;
        s.py.flags.new_spells_to_learn = 0;
    });
    player_calculate_allowed_spells_count(PlayerAttr::A_WIS);
    let spells = &MAGIC_SPELLS[(PRIEST_CLASS_ID - 1) as usize];
    with_state(|s| {
        let expected = cpp_learnable_spells(spells, 5, 0, 5);
        assert_eq!(s.py.flags.new_spells_to_learn, expected as u8);
    });
}

// ---------------------------------------------------------------------------
// 4. playerGainSpells — zero-roll early returns
// ---------------------------------------------------------------------------

#[test]
fn gain_spells_confused_consumes_no_rng() {
    reset_for_new_game(Some(42));
    test_set_ncurses_stub(true);
    setup_base_player(PRIEST_CLASS_ID, 10);
    with_state_mut(|s| {
        s.py.flags.confused = 1;
        s.py.flags.new_spells_to_learn = 3;
    });
    player_gain_spells();
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn gain_spells_mage_blind_consumes_no_rng() {
    reset_for_new_game(Some(42));
    test_set_ncurses_stub(true);
    setup_base_player(MAGE_CLASS_ID, 10);
    with_state_mut(|s| {
        s.py.flags.blind = 1;
        s.py.flags.new_spells_to_learn = 3;
    });
    player_gain_spells();
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn gain_spells_zero_to_learn_sets_free_turn_no_rng() {
    reset_for_new_game(Some(42));
    test_set_ncurses_stub(true);
    setup_base_player(PRIEST_CLASS_ID, 10);
    with_state_mut(|s| s.py.flags.new_spells_to_learn = 0);
    player_gain_spells();
    with_state(|s| assert!(s.game.player_free_turn));
    assert_eq!(next_random_pair(100), (100, 2));
}

// ---------------------------------------------------------------------------
// 5. playerGainSpells — priest RNG golden (seed 42)
// ---------------------------------------------------------------------------

#[test]
fn priest_spell_bank_level10_matches_cpp() {
    let class_id = PRIEST_CLASS_ID as usize;
    let level = 10u8;
    let mut spell_flag = 0x7FFF_FFFFu32;
    let mut spell_bank = Vec::new();
    let mut mask = 1u32;
    let mut i = 0;
    while spell_flag != 0 {
        if spell_flag & mask != 0 {
            spell_flag &= !mask;
            if MAGIC_SPELLS[class_id - 1][i].level_required <= level {
                spell_bank.push(i);
            }
        }
        mask <<= 1;
        i += 1;
    }
    assert_eq!(spell_bank, (0..=18).collect::<Vec<_>>());
}

#[test]
fn gain_spells_priest_random_learn_seed42() {
    reset_for_new_game(Some(42));
    test_set_ncurses_stub(true);
    setup_base_player(PRIEST_CLASS_ID, 10);
    set_used_stat(PlayerAttr::A_WIS, 18);
    with_state_mut(|s| {
        s.py.flags.new_spells_to_learn = 2;
        s.py.flags.spells_learnt = 0;
    });

    player_gain_spells();

    with_state(|s| {
        assert_eq!(s.py.flags.new_spells_to_learn, 0);
        assert_eq!(s.py.flags.spells_learnt.count_ones(), 2);
        assert_eq!(s.py.flags.spells_learned_order[0], 17);
        assert_eq!(s.py.flags.spells_learned_order[1], 8);
        assert_eq!(s.py.flags.status & PY_STUDY, PY_STUDY);
    });
    assert_eq!(next_random_pair(100), (100, 36));
}

#[test]
fn gain_spells_priest_learn_sets_initial_mana() {
    reset_for_new_game(Some(42));
    test_set_ncurses_stub(true);
    setup_base_player(PRIEST_CLASS_ID, 10);
    set_used_stat(PlayerAttr::A_WIS, 18);
    with_state_mut(|s| {
        s.py.flags.new_spells_to_learn = 1;
        s.py.misc.mana = 0;
    });
    player_gain_spells();
    with_state(|s| assert!(s.py.misc.mana > 0));
}

// ---------------------------------------------------------------------------
// 6. playerGainSpells — mage interactive path
// ---------------------------------------------------------------------------

#[test]
fn gain_spells_mage_picks_from_menu_seed42() {
    reset_for_new_game(Some(42));
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    // pop order: first key consumed first
    test_push_getch_keys(&[b'b' as i32]); // learn spell_bank[1]

    setup_base_player(MAGE_CLASS_ID, 10);
    set_used_stat(PlayerAttr::A_INT, 18);
    with_state_mut(|s| {
        s.py.flags.new_spells_to_learn = 1;
        s.py.pack.unique_items = 1;
        s.py.inventory[0].category_id = TV_MAGIC_BOOK;
        s.py.inventory[0].flags = 0b11; // spells 0 and 1
    });

    player_gain_spells();

    with_state(|s| {
        assert_eq!(s.py.flags.new_spells_to_learn, 0);
        assert_eq!(s.py.flags.spells_learnt, 1 << 1);
        assert_eq!(s.py.flags.spells_learned_order[0], 1);
    });
    assert_eq!(next_random_pair(100), (100, 2));
}

// ---------------------------------------------------------------------------
// 7. Integer semantics
// ---------------------------------------------------------------------------

#[test]
fn bitmask_shift_matches_cpp_u32() {
    let order_id = 5u8;
    let mask = 1u32 << order_id;
    assert_eq!(mask, 32);
    assert_eq!(
        player_stat_adjustment_wisdom_intelligence(PlayerAttr::A_INT),
        0
    );
}

#[test]
fn spell_name_offsets_match_data() {
    assert_eq!(SPELL_NAMES[0], "Magic Missile");
    assert_eq!(SPELL_NAMES[NAME_OFFSET_PRAYERS as usize], "Detect Evil");
    assert_eq!(
        CLASSES[MAGE_CLASS_ID as usize].class_to_use_mage_spells,
        SPELL_TYPE_MAGE
    );
}
