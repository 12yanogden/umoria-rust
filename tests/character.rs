//! `character` character creation tests.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use umoria::character::{
    character_generate_stats_and_race, character_get_history, character_set_age_height_weight,
    create_modify_player_stat, decrement_stat, generate_character_class, increment_stat,
    monetary_value_calculated_from_stat, player_calculate_start_gold, player_clear_history,
};
use umoria::data_player::{CHARACTER_BACKGROUNDS, CHARACTER_RACES, CLASSES};
use umoria::game::{
    random_number, random_number_normal_distribution, reset_for_new_game, with_state,
    with_state_mut,
};
use umoria::player::{PlayerAttr, PLAYER_MAX_LEVEL};
use umoria::rng::get_seed;
use umoria::ui_io;

// --------------------------------------------------------------------------
// Oracle helpers
// --------------------------------------------------------------------------

fn expected_decrement_stat(adjustment: i16, current_stat: u8) -> u8 {
    let mut stat = current_stat;
    let mut i = 0i16;
    while i > adjustment {
        if stat > 108 {
            stat -= 1;
        } else if stat > 88 {
            stat = stat.wrapping_sub((random_number(6) + 2) as u8);
        } else if stat > 18 {
            stat = stat.wrapping_sub((random_number(15) + 5) as u8);
            if stat < 18 {
                stat = 18;
            }
        } else if stat > 3 {
            stat -= 1;
        }
        i -= 1;
    }
    stat
}

fn expected_increment_stat(adjustment: i16, current_stat: u8) -> u8 {
    let mut stat = current_stat;
    for _ in 0..adjustment {
        if stat < 18 {
            stat += 1;
        } else if stat < 88 {
            stat = stat.wrapping_add((random_number(15) + 5) as u8);
        } else if stat < 108 {
            stat = stat.wrapping_add((random_number(6) + 2) as u8);
        } else if stat < 118 {
            stat += 1;
        }
    }
    stat
}

fn expected_create_modify_player_stat(stat: u8, adjustment: i16) -> u8 {
    if adjustment < 0 {
        expected_decrement_stat(adjustment, stat)
    } else {
        expected_increment_stat(adjustment, stat)
    }
}

fn expected_character_generate_stats() -> [u8; 6] {
    let mut dice = [0i32; 18];
    loop {
        let mut total = 0;
        for (i, dice_entry) in dice.iter_mut().enumerate() {
            *dice_entry = random_number(3 + (i % 3) as i32);
            total += *dice_entry;
        }
        if total > 42 && total < 54 {
            break;
        }
    }
    let mut stats = [0u8; 6];
    for i in 0..6 {
        stats[i] = (5 + dice[3 * i] + dice[3 * i + 1] + dice[3 * i + 2]) as u8;
    }
    stats
}

fn expected_character_generate_stats_and_race(race_id: u8) {
    let race = &CHARACTER_RACES[race_id as usize];
    let mut stats = expected_character_generate_stats();
    stats[PlayerAttr::A_STR as usize] =
        expected_create_modify_player_stat(stats[PlayerAttr::A_STR as usize], race.str_adjustment);
    stats[PlayerAttr::A_INT as usize] =
        expected_create_modify_player_stat(stats[PlayerAttr::A_INT as usize], race.int_adjustment);
    stats[PlayerAttr::A_WIS as usize] =
        expected_create_modify_player_stat(stats[PlayerAttr::A_WIS as usize], race.wis_adjustment);
    stats[PlayerAttr::A_DEX as usize] =
        expected_create_modify_player_stat(stats[PlayerAttr::A_DEX as usize], race.dex_adjustment);
    stats[PlayerAttr::A_CON as usize] =
        expected_create_modify_player_stat(stats[PlayerAttr::A_CON as usize], race.con_adjustment);
    stats[PlayerAttr::A_CHR as usize] =
        expected_create_modify_player_stat(stats[PlayerAttr::A_CHR as usize], race.chr_adjustment);
    with_state_mut(|s| {
        s.py.stats.max = stats;
        s.py.misc.level = 1;
        for i in 0..6 {
            s.py.stats.current[i] = s.py.stats.max[i];
        }
    });
}

fn expected_character_get_history(race_id: u8) -> (i16, [[u8; 60]; 4]) {
    let mut history_id = i32::from(race_id) * 3 + 1;
    let mut social_class = random_number(4);
    let mut history_block = String::new();
    let mut background_id = 0usize;

    loop {
        let mut flag = false;
        while !flag {
            if CHARACTER_BACKGROUNDS[background_id].chart == history_id as u8 {
                let test_roll = random_number(100);
                while test_roll > i32::from(CHARACTER_BACKGROUNDS[background_id].roll) {
                    background_id += 1;
                }
                let background = &CHARACTER_BACKGROUNDS[background_id];
                history_block.push_str(background.info);
                social_class += i32::from(background.bonus) - 50;
                if history_id > i32::from(background.next) {
                    background_id = 0;
                }
                history_id = i32::from(background.next);
                flag = true;
            } else {
                background_id += 1;
            }
        }
        if history_id < 1 {
            break;
        }
    }

    social_class = social_class.clamp(1, 100);

    let mut history = [[0u8; 60]; 4];
    let bytes = history_block.as_bytes();
    let mut cursor_start = 0usize;
    let mut cursor_end = bytes.len().saturating_sub(1);
    while cursor_end < bytes.len() && bytes[cursor_end] == b' ' {
        cursor_end = cursor_end.saturating_sub(1);
    }
    let mut line_number = 0usize;
    let mut new_cursor_start = 0usize;
    let mut done = false;
    while !done {
        while cursor_start < bytes.len() && bytes[cursor_start] == b' ' {
            cursor_start += 1;
        }
        let mut current_cursor_position = cursor_end.saturating_sub(cursor_start) + 1;
        if current_cursor_position > 60 {
            current_cursor_position = 60;
            while bytes[cursor_start + current_cursor_position - 1] != b' ' {
                current_cursor_position -= 1;
            }
            new_cursor_start = cursor_start + current_cursor_position;
            while bytes[cursor_start + current_cursor_position - 1] == b' ' {
                current_cursor_position -= 1;
            }
        } else {
            done = true;
        }
        let end = cursor_start + current_cursor_position;
        let len = current_cursor_position.min(60);
        history[line_number][..len].copy_from_slice(&bytes[cursor_start..end]);
        if len < 60 {
            history[line_number][len] = 0;
        }
        line_number += 1;
        cursor_start = new_cursor_start;
    }

    (social_class as i16, history)
}

fn expected_character_set_age_height_weight(race_id: u8, is_male: bool) -> (u16, u16, u16) {
    let race = &CHARACTER_RACES[race_id as usize];
    let age = (race.base_age as u16).wrapping_add(random_number(race.max_age as i32) as u16);
    let (height_base, height_mod, weight_base, weight_mod) = if is_male {
        (
            race.male_height_base,
            race.male_height_mod,
            race.male_weight_base,
            race.male_weight_mod,
        )
    } else {
        (
            race.female_height_base,
            race.female_height_mod,
            race.female_weight_base,
            race.female_weight_mod,
        )
    };
    let height = random_number_normal_distribution(height_base as i32, height_mod as i32) as u16;
    let weight = random_number_normal_distribution(weight_base as i32, weight_mod as i32) as u16;
    (age, height, weight)
}

fn expected_generate_character_class(class_id: u8) {
    let klass = &CLASSES[class_id as usize];
    let stats_max = with_state(|s| s.py.stats.max);
    let stats_max = [
        expected_create_modify_player_stat(stats_max[PlayerAttr::A_STR as usize], klass.strength),
        expected_create_modify_player_stat(
            stats_max[PlayerAttr::A_INT as usize],
            klass.intelligence,
        ),
        expected_create_modify_player_stat(stats_max[PlayerAttr::A_WIS as usize], klass.wisdom),
        expected_create_modify_player_stat(stats_max[PlayerAttr::A_DEX as usize], klass.dexterity),
        expected_create_modify_player_stat(
            stats_max[PlayerAttr::A_CON as usize],
            klass.constitution,
        ),
        expected_create_modify_player_stat(stats_max[PlayerAttr::A_CHR as usize], klass.charisma),
    ];
    with_state_mut(|s| {
        s.py.stats.max = stats_max;
        s.py.misc.class_id = class_id;
        s.py.misc.hit_die = s.py.misc.hit_die.wrapping_add(klass.hit_points);
    });

    let hit_die = with_state(|s| s.py.misc.hit_die);
    let min_value = (i32::from(PLAYER_MAX_LEVEL) * 3 / 8 * i32::from(hit_die - 1))
        + i32::from(PLAYER_MAX_LEVEL);
    let max_value = (i32::from(PLAYER_MAX_LEVEL) * 5 / 8 * i32::from(hit_die - 1))
        + i32::from(PLAYER_MAX_LEVEL);

    let mut base_hp_levels = [0u16; PLAYER_MAX_LEVEL as usize];
    base_hp_levels[0] = hit_die as u16;
    loop {
        for i in 1..PLAYER_MAX_LEVEL as usize {
            base_hp_levels[i] = random_number(hit_die as i32) as u16;
            base_hp_levels[i] = base_hp_levels[i].wrapping_add(base_hp_levels[i - 1]);
        }
        if i32::from(base_hp_levels[PLAYER_MAX_LEVEL as usize - 1]) >= min_value
            && i32::from(base_hp_levels[PLAYER_MAX_LEVEL as usize - 1]) <= max_value
        {
            break;
        }
    }
    with_state_mut(|s| {
        s.py.base_hp_levels = base_hp_levels;
    });
}

fn expected_player_calculate_start_gold(is_male: bool) -> i32 {
    let stats = with_state(|s| s.py.stats.max);
    let social_class = with_state(|s| s.py.misc.social_class);
    let mut value = monetary_value_calculated_from_stat(stats[PlayerAttr::A_STR as usize]);
    value += monetary_value_calculated_from_stat(stats[PlayerAttr::A_INT as usize]);
    value += monetary_value_calculated_from_stat(stats[PlayerAttr::A_WIS as usize]);
    value += monetary_value_calculated_from_stat(stats[PlayerAttr::A_CON as usize]);
    value += monetary_value_calculated_from_stat(stats[PlayerAttr::A_DEX as usize]);
    let mut new_gold = i32::from(social_class) * 6 + random_number(25) + 325;
    new_gold -= value;
    new_gold += monetary_value_calculated_from_stat(stats[PlayerAttr::A_CHR as usize]);
    if !is_male {
        new_gold += 50;
    }
    if new_gold < 80 {
        new_gold = 80;
    }
    new_gold
}

fn history_to_strings(history: &[[u8; 60]; 4]) -> [String; 4] {
    std::array::from_fn(|i| {
        let end = history[i]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(history[i].len());
        String::from_utf8_lossy(&history[i][..end]).into_owned()
    })
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[test]
fn create_modify_player_stat_band_parity_exhaustive() {
    for stat in [3u8, 18, 19, 88, 89, 108, 109, 117] {
        for adjustment in -5i16..=5 {
            reset_for_new_game(Some(42));
            let rust = create_modify_player_stat(stat, adjustment);
            reset_for_new_game(Some(42));
            let expected = expected_create_modify_player_stat(stat, adjustment);
            assert_eq!(rust, expected, "stat={stat} adjustment={adjustment}");
        }
    }
}

#[test]
fn decrement_stat_clamps_at_18_and_increment_stat_caps_at_118() {
    reset_for_new_game(Some(42));
    let rust = decrement_stat(-1, 25);
    reset_for_new_game(Some(42));
    let expected = expected_decrement_stat(-1, 25);
    assert_eq!(rust, expected);
    assert_eq!(rust, 18);

    reset_for_new_game(Some(1));
    let mut stat = 108u8;
    for _ in 0..20 {
        stat = increment_stat(1, stat);
    }
    assert_eq!(stat, 118);
}

#[test]
fn character_generate_stats_distribution_and_reroll_parity() {
    for seed in [1u32, 42, 12345] {
        reset_for_new_game(Some(seed));
        character_generate_stats_and_race();
        let rust_stats = with_state(|s| s.py.stats.max);
        let rust_seed = get_seed();

        reset_for_new_game(Some(seed));
        expected_character_generate_stats_and_race(0);
        let expected_stats = with_state(|s| s.py.stats.max);
        assert_eq!(rust_stats, expected_stats, "seed={seed}");
        assert_eq!(rust_seed, get_seed(), "seed={seed}");
    }
}

#[test]
fn character_get_history_parity_all_races() {
    for race_id in 0..8u8 {
        for seed in [1u32, 42] {
            reset_for_new_game(Some(seed));
            with_state_mut(|s| s.py.misc.race_id = race_id);
            character_get_history();
            let (rust_social, rust_history) =
                with_state(|s| (s.py.misc.social_class, s.py.misc.history));
            let rust_seed = get_seed();

            reset_for_new_game(Some(seed));
            let (expected_social, expected_history) = expected_character_get_history(race_id);
            assert_eq!(rust_social, expected_social, "race={race_id} seed={seed}");
            assert_eq!(
                history_to_strings(&rust_history),
                history_to_strings(&expected_history),
                "race={race_id} seed={seed}"
            );
            assert_eq!(rust_seed, get_seed(), "race={race_id} seed={seed}");
        }
    }
}

#[test]
fn character_set_age_height_weight_parity() {
    for race_id in 0..8u8 {
        for (seed, is_male) in [(42u32, true), (12345, false)] {
            reset_for_new_game(Some(seed));
            with_state_mut(|s| {
                s.py.misc.race_id = race_id;
                s.py.misc.gender = is_male;
            });
            character_set_age_height_weight();
            let (age, height, weight) =
                with_state(|s| (s.py.misc.age, s.py.misc.height, s.py.misc.weight));
            let rust_seed = get_seed();

            reset_for_new_game(Some(seed));
            let (expected_age, expected_height, expected_weight) =
                expected_character_set_age_height_weight(race_id, is_male);
            assert_eq!(
                (age, height, weight),
                (expected_age, expected_height, expected_weight)
            );
            assert_eq!(rust_seed, get_seed());
        }
    }
}

#[test]
fn generate_character_class_base_hp_parity() {
    ui_io::test_set_ncurses_stub(true);
    for seed in [42u32, 999] {
        reset_for_new_game(Some(seed));
        with_state_mut(|s| {
            s.py.misc.race_id = 0;
            s.py.misc.level = 1;
            s.py.misc.hit_die = CHARACTER_RACES[0].hit_points_base;
            s.py.stats.max = [16, 14, 13, 12, 15, 10];
            for i in 0..6 {
                s.py.stats.current[i] = s.py.stats.max[i];
            }
        });
        generate_character_class(0);
        let rust_hp = with_state(|s| s.py.base_hp_levels);
        let rust_seed = get_seed();

        reset_for_new_game(Some(seed));
        with_state_mut(|s| {
            s.py.misc.race_id = 0;
            s.py.misc.level = 1;
            s.py.misc.hit_die = CHARACTER_RACES[0].hit_points_base;
            s.py.stats.max = [16, 14, 13, 12, 15, 10];
            for i in 0..6 {
                s.py.stats.current[i] = s.py.stats.max[i];
            }
        });
        expected_generate_character_class(0);
        let expected_hp = with_state(|s| s.py.base_hp_levels);
        assert_eq!(rust_hp, expected_hp, "seed={seed}");
        assert_eq!(rust_seed, get_seed(), "seed={seed}");
    }
    ui_io::test_set_ncurses_stub(false);
}

#[test]
fn player_calculate_start_gold_parity() {
    for seed in [42u32, 7] {
        for is_male in [true, false] {
            reset_for_new_game(Some(seed));
            with_state_mut(|s| {
                s.py.stats.max = [16, 14, 13, 12, 15, 10];
                s.py.misc.social_class = 55;
                s.py.misc.gender = is_male;
            });
            player_calculate_start_gold();
            let rust_au = with_state(|s| s.py.misc.au);
            let rust_seed = get_seed();

            reset_for_new_game(Some(seed));
            with_state_mut(|s| {
                s.py.stats.max = [16, 14, 13, 12, 15, 10];
                s.py.misc.social_class = 55;
                s.py.misc.gender = is_male;
            });
            let expected_au = expected_player_calculate_start_gold(is_male);
            assert_eq!(rust_au, expected_au, "seed={seed} male={is_male}");
            assert_eq!(rust_seed, get_seed());
        }
    }
}

#[test]
fn monetary_value_calculated_from_stat_matches_expected() {
    assert_eq!(monetary_value_calculated_from_stat(10), 0);
    assert_eq!(monetary_value_calculated_from_stat(18), 40);
    assert_eq!(monetary_value_calculated_from_stat(8), -10);
}

#[test]
fn player_clear_history_zeroes_lines() {
    reset_for_new_game(None);
    with_state_mut(|s| s.py.misc.history[0][0] = b'X');
    player_clear_history();
    with_state(|s| assert_eq!(s.py.misc.history[0][0], 0));
}

#[test]
fn character_get_history_nul_terminates_short_lines() {
    // writes '\0' after strncpy; shorter re-rolls must not leak.
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.misc.race_id = 0;
        for line in &mut s.py.misc.history {
            line.fill(b'Z');
        }
    });
    character_get_history();
    with_state(|s| {
        for line in &s.py.misc.history {
            let end = line.iter().position(|&b| b == 0).unwrap_or(line.len());
            assert!(end < 60 || line.iter().all(|&b| b != 0));
            if end < 60 {
                assert_eq!(line[end], 0);
                assert!(line[..end].iter().all(|&b| b != b'Z'));
            }
        }
    });
}

#[test]
fn full_character_creation_rng_parity_seed42() {
    ui_io::test_set_ncurses_stub(true);
    // Programmatic replay of the seed-42 Human/male/warrior path (matches recorded UI choices).
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.misc.race_id = 0;
        s.py.misc.gender = true;
    });
    character_generate_stats_and_race();
    character_get_history();
    character_set_age_height_weight();
    generate_character_class(0);
    player_calculate_start_gold();

    let snapshot = with_state(|s| {
        (
            s.py.misc.race_id,
            s.py.misc.gender,
            s.py.misc.class_id,
            s.py.stats.max,
            s.py.misc.social_class,
            s.py.misc.age,
            s.py.misc.height,
            s.py.misc.weight,
            s.py.misc.au,
            s.py.misc.hit_die,
            s.py.misc.max_hp,
            s.py.base_hp_levels,
            history_to_strings(&s.py.misc.history),
            get_seed(),
        )
    });

    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.misc.race_id = 0;
        s.py.misc.gender = true;
    });
    character_generate_stats_and_race();
    character_get_history();
    character_set_age_height_weight();
    generate_character_class(0);
    player_calculate_start_gold();
    let oracle = with_state(|s| {
        (
            s.py.misc.race_id,
            s.py.misc.gender,
            s.py.misc.class_id,
            s.py.stats.max,
            s.py.misc.social_class,
            s.py.misc.age,
            s.py.misc.height,
            s.py.misc.weight,
            s.py.misc.au,
            s.py.misc.hit_die,
            s.py.misc.max_hp,
            s.py.base_hp_levels,
            history_to_strings(&s.py.misc.history),
            get_seed(),
        )
    });

    assert_eq!(snapshot.0, oracle.0);
    assert_eq!(snapshot.1, oracle.1);
    assert_eq!(snapshot.2, oracle.2);
    assert_eq!(snapshot.3, oracle.3);
    assert_eq!(snapshot.4, oracle.4);
    assert_eq!(snapshot.5, oracle.5);
    assert_eq!(snapshot.6, oracle.6);
    assert_eq!(snapshot.7, oracle.7);
    assert_eq!(snapshot.8, oracle.8);
    assert_eq!(snapshot.9, oracle.9);
    assert_eq!(snapshot.10, oracle.10);
    assert_eq!(snapshot.11, oracle.11);
    assert_eq!(snapshot.12, oracle.12);
    assert_eq!(snapshot.13, oracle.13);
    assert_eq!(snapshot.0, 0);
    assert!(snapshot.1);
    assert_eq!(snapshot.2, 0);
    assert!(snapshot.13 > 0);
    ui_io::test_set_ncurses_stub(false);
}
