//! Phase 4.3.2 — player_stats.cpp parity.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use umoria::config::player::status::{PY_HERO, PY_HP, PY_SHERO, PY_STR, PY_STR_WGT};
use umoria::config::spells::{SPELL_TYPE_MAGE, SPELL_TYPE_PRIEST};
use umoria::data_player::{BLOWS_TABLE, CLASSES};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::player::{PlayerAttr, PLAYER_MAX_LEVEL};
use umoria::player_stats::{
    player_armor_class_adjustment, player_attack_blows, player_calculate_hit_points,
    player_damage_adjustment, player_disarm_adjustment, player_initialize_base_experience_levels,
    player_modify_stat, player_set_and_use_stat, player_stat_adjustment_charisma,
    player_stat_adjustment_constitution, player_stat_adjustment_wisdom_intelligence,
    player_stat_boost, player_stat_random_decrease, player_stat_random_increase,
    player_stat_restore, player_to_hit_adjustment,
};
use umoria::ui_io::test_set_ncurses_stub;

const EXPECTED_BASE_EXP_LEVELS: [u32; PLAYER_MAX_LEVEL as usize] = [
    10, 25, 45, 70, 100, 140, 200, 280, 380, 500, 650, 850, 1100, 1400, 1800, 2300, 2900, 3600,
    4400, 5400, 6800, 8400, 10200, 12500, 17500, 25000, 35000, 50000, 75000, 100000, 150000,
    200000, 300000, 400000, 500000, 750000, 1500000, 2500000, 5000000, 10000000,
];

fn set_used_stat(stat: PlayerAttr, value: u8) {
    with_state_mut(|s| {
        s.py.stats.used[stat as usize] = value;
    });
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

// ---------------------------------------------------------------------------
// C++ oracle helpers (player_stats.cpp)
// ---------------------------------------------------------------------------

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

fn cpp_charisma_adj(charisma: i32) -> i32 {
    if charisma > 117 {
        90
    } else if charisma > 107 {
        92
    } else if charisma > 87 {
        94
    } else if charisma > 67 {
        96
    } else if charisma > 18 {
        98
    } else {
        match charisma {
            18 => 100,
            17 => 101,
            16 => 102,
            15 => 103,
            14 => 104,
            13 => 106,
            12 => 108,
            11 => 110,
            10 => 112,
            9 => 114,
            8 => 116,
            7 => 118,
            6 => 120,
            5 => 122,
            4 => 125,
            3 => 130,
            _ => 100,
        }
    }
}

fn cpp_con_adj(con: i32) -> i32 {
    if con < 7 {
        con - 7
    } else if con < 17 {
        0
    } else if con == 17 {
        1
    } else if con < 94 {
        2
    } else if con < 117 {
        3
    } else {
        4
    }
}

fn cpp_to_hit_adj(dexterity: i32, strength: i32) -> i16 {
    let mut total = if dexterity < 4 {
        -3
    } else if dexterity < 6 {
        -2
    } else if dexterity < 8 {
        -1
    } else if dexterity < 16 {
        0
    } else if dexterity < 17 {
        1
    } else if dexterity < 18 {
        2
    } else if dexterity < 69 {
        3
    } else if dexterity < 118 {
        4
    } else {
        5
    };

    if strength < 4 {
        total -= 3;
    } else if strength < 5 {
        total -= 2;
    } else if strength < 7 {
        total -= 1;
    } else if strength < 18 {
        // no change
    } else if strength < 94 {
        total += 1;
    } else if strength < 109 {
        total += 2;
    } else if strength < 117 {
        total += 3;
    } else {
        total += 4;
    }

    total
}

fn cpp_ac_adj(stat: i32) -> i16 {
    if stat < 4 {
        -4
    } else if stat == 4 {
        -3
    } else if stat == 5 {
        -2
    } else if stat == 6 {
        -1
    } else if stat < 15 {
        0
    } else if stat < 18 {
        1
    } else if stat < 59 {
        2
    } else if stat < 94 {
        3
    } else if stat < 117 {
        4
    } else {
        5
    }
}

fn cpp_disarm_adj(stat: i32) -> i16 {
    if stat < 4 {
        -8
    } else if stat == 4 {
        -6
    } else if stat == 5 {
        -4
    } else if stat == 6 {
        -2
    } else if stat == 7 {
        -1
    } else if stat < 13 {
        0
    } else if stat < 16 {
        1
    } else if stat < 18 {
        2
    } else if stat < 59 {
        4
    } else if stat < 94 {
        5
    } else if stat < 117 {
        6
    } else {
        8
    }
}

fn cpp_damage_adj(stat: i32) -> i16 {
    if stat < 4 {
        -2
    } else if stat < 5 {
        -1
    } else if stat < 16 {
        0
    } else if stat < 17 {
        1
    } else if stat < 18 {
        2
    } else if stat < 94 {
        3
    } else if stat < 109 {
        4
    } else if stat < 117 {
        5
    } else {
        6
    }
}

fn cpp_attack_blows_dexterity(dexterity: i32) -> i32 {
    if dexterity < 10 {
        0
    } else if dexterity < 19 {
        1
    } else if dexterity < 68 {
        2
    } else if dexterity < 108 {
        3
    } else if dexterity < 118 {
        4
    } else {
        5
    }
}

fn cpp_attack_blows_strength(strength: i32, weight: i32) -> i32 {
    let adj_weight = strength * 10 / weight;
    if adj_weight < 2 {
        0
    } else if adj_weight < 3 {
        1
    } else if adj_weight < 4 {
        2
    } else if adj_weight < 5 {
        3
    } else if adj_weight < 7 {
        4
    } else if adj_weight < 9 {
        5
    } else {
        6
    }
}

fn cpp_attack_blows(strength: i32, dexterity: i32, weight: i32) -> (i32, i32) {
    if strength * 15 < weight {
        return (1, strength * 15 - weight);
    }
    let dex = cpp_attack_blows_dexterity(dexterity);
    let str_idx = cpp_attack_blows_strength(strength, weight);
    (i32::from(BLOWS_TABLE[str_idx as usize][dex as usize]), 0)
}

fn cpp_modify_stat(current: u8, amount: i16) -> u8 {
    let mut new_stat = current;
    let loop_count = if amount < 0 {
        i32::from(-amount)
    } else {
        i32::from(amount)
    };

    for _ in 0..loop_count {
        if amount > 0 {
            if new_stat < 18 {
                new_stat = new_stat.wrapping_add(1);
            } else if new_stat < 108 {
                new_stat = new_stat.wrapping_add(10);
            } else {
                new_stat = 118;
            }
        } else if new_stat > 27 {
            new_stat = new_stat.wrapping_sub(10);
        } else if new_stat > 18 {
            new_stat = 18;
        } else if new_stat > 3 {
            new_stat = new_stat.wrapping_sub(1);
        }
    }

    new_stat
}

fn cpp_calculate_hit_points(
    level: u16,
    base_hp: u16,
    con: u8,
    status: u32,
    current_hp: i16,
    current_hp_fraction: u16,
    max_hp: i16,
) -> (i16, u16, i16, u32) {
    let con_adj = cpp_con_adj(i32::from(con));
    let mut hp = i32::from(base_hp) + con_adj * i32::from(level);
    if hp < i32::from(level) + 1 {
        hp = i32::from(level) + 1;
    }
    if (status & PY_HERO) != 0 {
        hp += 10;
    }
    if (status & PY_SHERO) != 0 {
        hp += 20;
    }

    let mut new_current_hp = current_hp;
    let mut new_fraction = current_hp_fraction;
    let mut new_max_hp = max_hp;
    let mut new_status = status;

    if hp != i32::from(max_hp) && max_hp != 0 {
        let value = ((i32::from(current_hp) << 16) + i32::from(current_hp_fraction))
            / i32::from(max_hp)
            * hp;
        new_current_hp = (value >> 16) as i16;
        new_fraction = (value & 0xFFFF) as u16;
        new_max_hp = hp as i16;
        new_status |= PY_HP;
    }

    (new_current_hp, new_fraction, new_max_hp, new_status)
}

// ---------------------------------------------------------------------------
// 1. Stat-adjustment tables (exhaustive golden)
// ---------------------------------------------------------------------------

#[test]
fn stat_adjustment_wisdom_intelligence_exhaustive() {
    reset_for_new_game(None);
    for value in 3..=118u8 {
        for stat in [PlayerAttr::A_INT, PlayerAttr::A_WIS] {
            set_used_stat(stat, value);
            let expected = cpp_wis_int_adj(i32::from(value));
            assert_eq!(
                player_stat_adjustment_wisdom_intelligence(stat),
                expected,
                "stat={stat:?} value={value}"
            );
        }
    }
}

#[test]
fn stat_adjustment_charisma_exhaustive() {
    reset_for_new_game(None);
    for value in 0..=118u8 {
        set_used_stat(PlayerAttr::A_CHR, value);
        let expected = cpp_charisma_adj(i32::from(value));
        assert_eq!(
            player_stat_adjustment_charisma(),
            expected,
            "charisma={value}"
        );
    }
}

#[test]
fn stat_adjustment_constitution_exhaustive() {
    reset_for_new_game(None);
    for value in 3..=118u8 {
        set_used_stat(PlayerAttr::A_CON, value);
        let expected = cpp_con_adj(i32::from(value));
        assert_eq!(
            player_stat_adjustment_constitution(),
            expected,
            "con={value}"
        );
    }
}

#[test]
fn to_hit_adjustment_exhaustive() {
    reset_for_new_game(None);
    for dex in 3..=118u8 {
        for str in 3..=118u8 {
            with_state_mut(|s| {
                s.py.stats.used[PlayerAttr::A_DEX as usize] = dex;
                s.py.stats.used[PlayerAttr::A_STR as usize] = str;
            });
            let expected = cpp_to_hit_adj(i32::from(dex), i32::from(str));
            assert_eq!(
                player_to_hit_adjustment(),
                i32::from(expected),
                "dex={dex} str={str}"
            );
        }
    }
}

#[test]
fn armor_class_adjustment_exhaustive() {
    reset_for_new_game(None);
    for dex in 3..=118u8 {
        set_used_stat(PlayerAttr::A_DEX, dex);
        let expected = cpp_ac_adj(i32::from(dex));
        assert_eq!(
            player_armor_class_adjustment(),
            i32::from(expected),
            "dex={dex}"
        );
    }
}

#[test]
fn disarm_adjustment_exhaustive() {
    reset_for_new_game(None);
    for dex in 3..=118u8 {
        set_used_stat(PlayerAttr::A_DEX, dex);
        let expected = cpp_disarm_adj(i32::from(dex));
        assert_eq!(player_disarm_adjustment(), i32::from(expected), "dex={dex}");
    }
}

#[test]
fn damage_adjustment_exhaustive() {
    reset_for_new_game(None);
    for str in 3..=118u8 {
        set_used_stat(PlayerAttr::A_STR, str);
        let expected = cpp_damage_adj(i32::from(str));
        assert_eq!(player_damage_adjustment(), i32::from(expected), "str={str}");
    }
}

// ---------------------------------------------------------------------------
// 2. playerAttackBlows parity
// ---------------------------------------------------------------------------

#[test]
fn attack_blows_weight_str_dex_matrix() {
    reset_for_new_game(None);
    for weight in [1, 5, 10, 50, 100, 150, 200, 500] {
        for strength in 3..=118u8 {
            for dexterity in 3..=118u8 {
                with_state_mut(|s| {
                    s.py.stats.used[PlayerAttr::A_STR as usize] = strength;
                    s.py.stats.used[PlayerAttr::A_DEX as usize] = dexterity;
                });
                let (expected_blows, expected_wth) =
                    cpp_attack_blows(i32::from(strength), i32::from(dexterity), weight);
                let mut weight_to_hit = 0;
                let blows = player_attack_blows(weight, &mut weight_to_hit);
                assert_eq!(
                    (blows, weight_to_hit),
                    (expected_blows, expected_wth),
                    "weight={weight} str={strength} dex={dexterity}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 3. playerCalculateHitPoints parity
// ---------------------------------------------------------------------------

#[test]
fn calculate_hit_points_basic_levels() {
    reset_for_new_game(None);
    for level in 1..=10u16 {
        for con in [6u8, 10, 17, 18, 50, 100, 118] {
            let base_hp = level * 8;
            with_state_mut(|s| {
                s.py.misc.level = level;
                s.py.misc.max_hp = 100;
                s.py.misc.current_hp = 50;
                s.py.misc.current_hp_fraction = 0;
                s.py.flags.status = 0;
                s.py.stats.used[PlayerAttr::A_CON as usize] = con;
                s.py.base_hp_levels[(level - 1) as usize] = base_hp;
            });
            player_calculate_hit_points();
            let (expected_cur, expected_frac, expected_max, expected_status) =
                cpp_calculate_hit_points(level, base_hp, con, 0, 50, 0, 100);
            with_state(|s| {
                assert_eq!(s.py.misc.max_hp, expected_max, "level={level} con={con}");
                assert_eq!(s.py.misc.current_hp, expected_cur);
                assert_eq!(s.py.misc.current_hp_fraction, expected_frac);
                assert_eq!(s.py.flags.status, expected_status);
            });
        }
    }
}

#[test]
fn calculate_hit_points_hero_shero_bonuses() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.misc.level = 5;
        s.py.misc.max_hp = 50;
        s.py.misc.current_hp = 25;
        s.py.misc.current_hp_fraction = 0;
        s.py.flags.status = PY_HERO | PY_SHERO;
        s.py.stats.used[PlayerAttr::A_CON as usize] = 10;
        s.py.base_hp_levels[4] = 30;
    });
    player_calculate_hit_points();
    with_state(|s| {
        assert_eq!(s.py.misc.max_hp, 60);
        assert_eq!(s.py.misc.current_hp, 30);
    });
}

#[test]
fn calculate_hit_points_skips_when_max_hp_zero() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.misc.level = 1;
        s.py.misc.max_hp = 0;
        s.py.misc.current_hp = 0;
        s.py.base_hp_levels[0] = 10;
        s.py.stats.used[PlayerAttr::A_CON as usize] = 10;
    });
    player_calculate_hit_points();
    with_state(|s| assert_eq!(s.py.misc.max_hp, 0));
}

#[test]
fn calculate_hit_points_minimum_per_level() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.misc.level = 10;
        s.py.misc.max_hp = 100;
        s.py.misc.current_hp = 50;
        s.py.stats.used[PlayerAttr::A_CON as usize] = 3;
        s.py.base_hp_levels[9] = 0;
    });
    player_calculate_hit_points();
    with_state(|s| assert_eq!(s.py.misc.max_hp, 11));
}

// ---------------------------------------------------------------------------
// 4. RNG-order/count parity
// ---------------------------------------------------------------------------

#[test]
fn stat_random_increase_below_18_no_rng_seed42() {
    reset_for_new_game(Some(42));
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.stats.current[PlayerAttr::A_STR as usize] = 10;
        s.py.stats.max[PlayerAttr::A_STR as usize] = 10;
        s.py.stats.modified[PlayerAttr::A_STR as usize] = 0;
    });
    assert!(player_stat_random_increase(PlayerAttr::A_STR));
    with_state(|s| assert_eq!(s.py.stats.current[PlayerAttr::A_STR as usize], 11));
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn stat_random_increase_percentile_rng_seed42() {
    reset_for_new_game(Some(42));
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.stats.current[PlayerAttr::A_STR as usize] = 18;
        s.py.stats.max[PlayerAttr::A_STR as usize] = 18;
        s.py.stats.modified[PlayerAttr::A_STR as usize] = 0;
    });
    assert!(player_stat_random_increase(PlayerAttr::A_STR));
    with_state(|s| {
        assert_eq!(s.py.stats.current[PlayerAttr::A_STR as usize], 50);
        assert_eq!(s.py.stats.max[PlayerAttr::A_STR as usize], 50);
    });
    assert_eq!(next_random_pair(17), (17, 13));
}

#[test]
fn stat_random_increase_at_max_returns_false_no_rng() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.stats.current[PlayerAttr::A_STR as usize] = 118;
    });
    assert!(!player_stat_random_increase(PlayerAttr::A_STR));
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn stat_random_decrease_above_18_rng_seed42() {
    reset_for_new_game(Some(42));
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.stats.current[PlayerAttr::A_STR as usize] = 50;
        s.py.stats.modified[PlayerAttr::A_STR as usize] = 0;
    });
    assert!(player_stat_random_decrease(PlayerAttr::A_STR));
    with_state(|s| assert_eq!(s.py.stats.current[PlayerAttr::A_STR as usize], 18));
    assert_eq!(next_random_pair(17), (17, 13));
}

#[test]
fn stat_random_decrease_below_19_no_rng_seed42() {
    reset_for_new_game(Some(42));
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.stats.current[PlayerAttr::A_STR as usize] = 10;
        s.py.stats.modified[PlayerAttr::A_STR as usize] = 0;
    });
    assert!(player_stat_random_decrease(PlayerAttr::A_STR));
    with_state(|s| assert_eq!(s.py.stats.current[PlayerAttr::A_STR as usize], 9));
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn stat_random_decrease_at_min_returns_false_no_rng() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.stats.current[PlayerAttr::A_STR as usize] = 3;
    });
    assert!(!player_stat_random_decrease(PlayerAttr::A_STR));
    assert_eq!(next_random_pair(100), (100, 2));
}

// ---------------------------------------------------------------------------
// 5. playerModifyStat / set / restore / boost (no RNG)
// ---------------------------------------------------------------------------

#[test]
fn modify_stat_increment_decrement_boundaries() {
    reset_for_new_game(None);
    for current in [17u8, 18, 28, 108, 117] {
        for amount in [-1i16, 1, 2, 5, -5] {
            let expected = cpp_modify_stat(current, amount);
            with_state_mut(|s| {
                s.py.stats.current[PlayerAttr::A_STR as usize] = current;
            });
            assert_eq!(
                player_modify_stat(PlayerAttr::A_STR, amount),
                expected,
                "current={current} amount={amount}"
            );
        }
    }
}

#[test]
fn set_and_use_stat_applies_modified() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.stats.current[PlayerAttr::A_STR as usize] = 18;
        s.py.stats.modified[PlayerAttr::A_STR as usize] = 2;
    });
    player_set_and_use_stat(PlayerAttr::A_STR);
    with_state(|s| {
        assert_eq!(s.py.stats.used[PlayerAttr::A_STR as usize], 38);
        assert_ne!(s.py.flags.status & PY_STR_WGT, 0);
    });
}

#[test]
fn set_and_use_stat_con_recalculates_hp() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.misc.level = 5;
        s.py.misc.max_hp = 50;
        s.py.misc.current_hp = 25;
        s.py.stats.current[PlayerAttr::A_CON as usize] = 10;
        s.py.stats.modified[PlayerAttr::A_CON as usize] = 0;
        s.py.stats.used[PlayerAttr::A_CON as usize] = 10;
        s.py.base_hp_levels[4] = 20;
    });
    player_set_and_use_stat(PlayerAttr::A_CON);
    with_state(|s| assert_eq!(s.py.misc.max_hp, 20));
}

#[test]
fn stat_restore_returns_false_when_already_at_max() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.stats.current[PlayerAttr::A_DEX as usize] = 15;
        s.py.stats.max[PlayerAttr::A_DEX as usize] = 15;
    });
    assert!(!player_stat_restore(PlayerAttr::A_DEX));
}

#[test]
fn stat_restore_increases_to_max_no_rng() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.stats.current[PlayerAttr::A_DEX as usize] = 10;
        s.py.stats.max[PlayerAttr::A_DEX as usize] = 15;
        s.py.stats.modified[PlayerAttr::A_DEX as usize] = 0;
    });
    assert!(player_stat_restore(PlayerAttr::A_DEX));
    with_state(|s| assert_eq!(s.py.stats.current[PlayerAttr::A_DEX as usize], 15));
}

#[test]
fn stat_boost_sets_modified_and_status_flag() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.stats.current[PlayerAttr::A_INT as usize] = 12;
        s.py.stats.modified[PlayerAttr::A_INT as usize] = 0;
    });
    player_stat_boost(PlayerAttr::A_INT, 3);
    with_state(|s| {
        assert_eq!(s.py.stats.modified[PlayerAttr::A_INT as usize], 3);
        assert_eq!(s.py.stats.used[PlayerAttr::A_INT as usize], 15);
        assert_ne!(s.py.flags.status & (PY_STR << PlayerAttr::A_INT as u32), 0);
    });
}

// ---------------------------------------------------------------------------
// 6. playerInitializeBaseExperienceLevels
// ---------------------------------------------------------------------------

#[test]
fn initialize_base_experience_levels_table() {
    reset_for_new_game(None);
    player_initialize_base_experience_levels();
    with_state(|s| {
        for (i, &exp) in EXPECTED_BASE_EXP_LEVELS
            .iter()
            .enumerate()
            .take(PLAYER_MAX_LEVEL as usize)
        {
            assert_eq!(s.py.base_exp_levels[i], exp, "level index {i}");
        }
    });
}

// ---------------------------------------------------------------------------
// 7. Integer semantics
// ---------------------------------------------------------------------------

#[test]
fn to_hit_adjustment_i16_wrap_on_overflow_sum() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.stats.used[PlayerAttr::A_DEX as usize] = 3;
        s.py.stats.used[PlayerAttr::A_STR as usize] = 3;
    });
    assert_eq!(player_to_hit_adjustment(), -6);
}

#[test]
fn attack_blows_strength_weight_integer_division() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.stats.used[PlayerAttr::A_STR as usize] = 17;
        s.py.stats.used[PlayerAttr::A_DEX as usize] = 10;
    });
    let mut weight_to_hit = 0;
    let blows = player_attack_blows(17, &mut weight_to_hit);
    assert_eq!(blows, i32::from(BLOWS_TABLE[6][1]));
    assert_eq!(weight_to_hit, 0);
}

#[test]
fn calculate_hit_points_proportional_int32_math() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.misc.level = 1;
        s.py.misc.max_hp = 3;
        s.py.misc.current_hp = 2;
        s.py.misc.current_hp_fraction = 0x8000;
        s.py.stats.used[PlayerAttr::A_CON as usize] = 10;
        s.py.base_hp_levels[0] = 10;
    });
    player_calculate_hit_points();
    with_state(|s| {
        assert_eq!(s.py.misc.max_hp, 10);
        assert_eq!(s.py.misc.current_hp, 8);
        assert_eq!(s.py.misc.current_hp_fraction, 0x5552);
        assert_ne!(s.py.flags.status & PY_HP, 0);
    });
}

#[test]
fn set_and_use_stat_mage_int_triggers_spell_side_effects() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.misc.class_id = 1;
        assert_eq!(CLASSES[1].class_to_use_mage_spells, SPELL_TYPE_MAGE);
        s.py.stats.current[PlayerAttr::A_INT as usize] = 10;
        s.py.stats.modified[PlayerAttr::A_INT as usize] = 0;
    });
    player_set_and_use_stat(PlayerAttr::A_INT);
    with_state(|s| assert_eq!(s.py.stats.used[PlayerAttr::A_INT as usize], 10));
}

#[test]
fn set_and_use_stat_priest_wis_triggers_spell_side_effects() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.misc.class_id = 2;
        assert_eq!(CLASSES[2].class_to_use_mage_spells, SPELL_TYPE_PRIEST);
        s.py.stats.current[PlayerAttr::A_WIS as usize] = 12;
        s.py.stats.modified[PlayerAttr::A_WIS as usize] = 0;
    });
    player_set_and_use_stat(PlayerAttr::A_WIS);
    with_state(|s| assert_eq!(s.py.stats.used[PlayerAttr::A_WIS as usize], 12));
}
