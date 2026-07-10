//! RNG, seed lifecycle, normal distribution, and dice parity.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use common::{golden_root, load_manifest, load_rng_sequence, GoldenKind};
use umoria::dice::{dice_roll, max_dice_roll, Dice};
use umoria::game::{
    game, random_number, random_number_normal_distribution, reset_for_new_game,
    seed_reset_to_old_seed, seed_set, seeds_initialize, set_test_unix_time, NORMAL_TABLE_SD,
};
use umoria::rng::{get_seed, rnd, set_seed, RNG_M};
use umoria::types::NORMAL_TABLE_SIZE;

const RN_MAXES: [i32; 5] = [2, 6, 10, 100, 32_767];
const ND_PARAMS: [(i32, i32); 3] = [(10, 4), (50, 10), (100, 25)];

#[test]
fn rnd_range_invariant_over_many_draws() {
    reset_for_new_game(None);
    for seed in [1u32, 42, 12_345, 2_147_483_647] {
        set_seed(seed);
        for _ in 0..100_000 {
            let value = rnd();
            assert!(value >= 1, "seed {seed}: rnd() returned {value}");
            assert!(value <= RNG_M - 1, "seed {seed}: rnd() returned {value}");
        }
    }
}

fn load_random_number_golden(path: &std::path::Path) -> std::io::Result<Vec<(u32, u32)>> {
    let raw = std::fs::read_to_string(path)?;
    let mut values = Vec::new();
    for (line_no, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let max_val: u32 = parts
            .next()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("missing max on line {} of {}", line_no + 1, path.display()),
                )
            })?
            .parse()
            .map_err(|err| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "invalid max on line {} of {}: {err}",
                        line_no + 1,
                        path.display()
                    ),
                )
            })?;
        let value: u32 = parts
            .next()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "missing value on line {} of {}",
                        line_no + 1,
                        path.display()
                    ),
                )
            })?
            .parse()
            .map_err(|err| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "invalid value on line {} of {}: {err}",
                        line_no + 1,
                        path.display()
                    ),
                )
            })?;
        values.push((max_val, value));
    }
    Ok(values)
}

#[test]
fn random_number_golden_sequences_match_cpp_capture() {
    let manifest = load_manifest().expect("manifest.json should parse");
    let root = golden_root();

    for entry in manifest
        .goldens
        .iter()
        .filter(|g| g.kind == GoldenKind::Rng && g.file.starts_with("rng/randomNumber_seed"))
    {
        let seed = entry
            .seed
            .expect("randomNumber golden must record its seed");
        let golden = load_random_number_golden(&root.join(&entry.file)).expect("load golden");
        assert_eq!(golden.len(), RN_MAXES.len() * 20);

        reset_for_new_game(Some(seed));
        for (idx, &(expected_max, expected)) in golden.iter().enumerate() {
            let actual = random_number(expected_max as i32);
            assert_eq!(
                actual as u32, expected,
                "{} draw {} (max={}): expected {}, got {}",
                entry.id, idx, expected_max, expected, actual
            );
            assert!((1..=expected_max as i32).contains(&actual));
        }
    }
}

#[test]
fn normal_dist_golden_sequences_match_cpp_capture() {
    let manifest = load_manifest().expect("manifest.json should parse");
    let root = golden_root();

    for entry in manifest
        .goldens
        .iter()
        .filter(|g| g.kind == GoldenKind::Rng && g.file.starts_with("rng/normalDist_seed"))
    {
        let seed = entry.seed.expect("normalDist golden must record its seed");
        let raw = std::fs::read_to_string(root.join(&entry.file)).expect("load golden");
        let rows: Vec<(i32, i32, i32)> = raw
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                let mut parts = line.split_whitespace();
                let mean: i32 = parts.next().unwrap().parse().unwrap();
                let sd: i32 = parts.next().unwrap().parse().unwrap();
                let value: i32 = parts.next().unwrap().parse().unwrap();
                (mean, sd, value)
            })
            .collect();
        assert_eq!(rows.len(), ND_PARAMS.len() * 20);

        reset_for_new_game(Some(seed));
        let mut idx = 0;
        for (mean, sd) in ND_PARAMS {
            for _ in 0..20 {
                let (exp_mean, exp_sd, expected) = rows[idx];
                assert_eq!(exp_mean, mean);
                assert_eq!(exp_sd, sd);
                let actual = random_number_normal_distribution(mean, sd);
                assert_eq!(
                    actual, expected,
                    "{} draw {} ({mean},{sd}): expected {expected}, got {actual}",
                    entry.id, idx
                );
                idx += 1;
            }
        }
    }
}

#[test]
fn normal_table_matches_cpp_capture() {
    reset_for_new_game(None);
    let golden = load_rng_sequence(&golden_root().join("rng/normal_table.txt")).expect("load");
    assert_eq!(golden.len(), NORMAL_TABLE_SIZE);
    umoria::game::with_state(|state| {
        assert_eq!(state.normal_table[0], 206);
        assert_eq!(state.normal_table[NORMAL_TABLE_SIZE - 1], 32766);
        for (i, &expected) in golden.iter().enumerate() {
            assert_eq!(state.normal_table[i] as u32, expected, "normal_table[{i}]");
        }
    });
}

#[test]
fn dice_roll_advances_rng_like_cpp() {
    let samples = [
        Dice { dice: 2, sides: 6 },
        Dice { dice: 1, sides: 4 },
        Dice { dice: 3, sides: 8 },
        Dice { dice: 0, sides: 6 },
        Dice { dice: 1, sides: 1 },
    ];
    for dice in samples {
        reset_for_new_game(Some(42));
        let mut expected = 0;
        for _ in 0..dice.dice {
            expected += random_number(dice.sides as i32);
        }
        reset_for_new_game(Some(42));
        assert_eq!(dice_roll(dice), expected);
        assert_eq!(max_dice_roll(dice), dice.dice as i32 * dice.sides as i32);
    }
}

#[test]
fn seeds_initialize_fixed_seed_sets_magic_town_and_rng_state() {
    reset_for_new_game(None);
    let input: u32 = 42;
    seeds_initialize(input);

    let magic = game(|g| g.magic_seed);
    let town = game(|g| g.town_seed);
    assert_eq!(magic, input);
    assert_eq!(town, input.wrapping_add(8762));

    // Post-init generator seed after setRandomSeed(seed+8762+113452) and warmup loop.
    let expected_init_seed = input.wrapping_add(8762).wrapping_add(113_452);
    reset_for_new_game(None);
    set_seed(expected_init_seed);
    let warmup = random_number(100) as u32;
    for _ in 0..warmup {
        rnd();
    }
    let expected_final = get_seed();

    reset_for_new_game(None);
    seeds_initialize(input);
    assert_eq!(get_seed(), expected_final);
}

#[test]
fn seed_set_and_reset_match_cpp_mapping() {
    reset_for_new_game(Some(99));
    let before = get_seed();
    seed_set(12345);
    assert_eq!(get_seed(), (12345 % (RNG_M as u32 - 1)) + 1);
    umoria::game::with_state(|s| assert_eq!(s.rng.old_seed, before));
    seed_reset_to_old_seed();
    // C++ `seedResetToOldSeed` calls `setRandomSeed(old_seed)`, re-applying the modulus map.
    assert_eq!(get_seed(), (before % (RNG_M as u32 - 1)) + 1);
}

#[test]
fn seeds_initialize_zero_uses_unix_time_stub() {
    reset_for_new_game(None);
    set_test_unix_time(Some(1_700_000_000));
    seeds_initialize(0);
    assert_eq!(game(|g| g.magic_seed), 1_700_000_000);
    set_test_unix_time(None);
}

#[test]
fn normal_distribution_off_scale_branch() {
    let mut hit_seed = None;
    for seed in 1..=2_000_000u32 {
        reset_for_new_game(Some(seed));
        if random_number(SHRT_MAX) == SHRT_MAX {
            hit_seed = Some(seed);
            break;
        }
    }
    let seed = hit_seed.expect("should find seed where randomNumber(SHRT_MAX)==SHRT_MAX");
    reset_for_new_game(Some(seed));
    let standard = 10;
    let mean = 50;
    assert_eq!(random_number(SHRT_MAX), SHRT_MAX);
    let mut offset = 4 * standard + random_number(standard);
    if random_number(2) == 1 {
        offset = -offset;
    }
    let expected = mean + offset;
    reset_for_new_game(Some(seed));
    assert_eq!(random_number_normal_distribution(mean, standard), expected);
}

const SHRT_MAX: i32 = 32_767;

#[test]
fn normal_table_sd_constant() {
    assert_eq!(NORMAL_TABLE_SD, 64);
    assert_eq!(NORMAL_TABLE_SIZE >> 1, 128);
}
