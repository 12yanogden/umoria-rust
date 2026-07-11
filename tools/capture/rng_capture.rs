//! Standalone RNG golden-capture harness.
//!
//! Usage: `cargo run --bin rng_capture -- [output_dir]`
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "capture CLI tool; stdout status and simple I/O errors are fine"
)]

use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process;

use umoria::game::{random_number, random_number_normal_distribution, with_state};
use umoria::rng::{rnd, set_seed};
use umoria::types::NORMAL_TABLE_SIZE;

const SEEDS: [u32; 4] = [1, 42, 12_345, 2_147_483_647];
const RN_MAXES: [i32; 5] = [2, 6, 10, 100, 32_767];
const ND_PARAMS: [(i32, i32); 3] = [(10, 4), (50, 10), (100, 25)];

fn open_out(dir: &Path, name: &str) -> BufWriter<File> {
    let path = dir.join(name);
    let file = File::create(&path).unwrap_or_else(|err| {
        eprintln!("rng_capture: cannot open {}: {err}", path.display());
        process::exit(1);
    });
    BufWriter::new(file)
}

fn capture_invariant(dir: &Path) {
    set_seed(0);
    let mut value = 0;
    for _ in 0..10_000 {
        value = rnd();
    }
    let mut fp = open_out(dir, "z10001.txt");
    writeln!(fp, "{value}").unwrap();
}

fn capture_rnd_sequences(dir: &Path) {
    for seed in SEEDS {
        set_seed(seed);
        let count = if seed == 1 { 10_001 } else { 1000 };
        let mut fp = open_out(dir, &format!("rnd_seed{seed}.txt"));
        for _ in 0..count {
            writeln!(fp, "{}", rnd()).unwrap();
        }
    }
}

fn capture_random_number(dir: &Path) {
    for seed in SEEDS {
        set_seed(seed);
        let mut fp = open_out(dir, &format!("randomNumber_seed{seed}.txt"));
        for max in RN_MAXES {
            for _ in 0..20 {
                writeln!(fp, "{max} {}", random_number(max)).unwrap();
            }
        }
    }
}

fn capture_normal_dist(dir: &Path) {
    for seed in SEEDS {
        set_seed(seed);
        let mut fp = open_out(dir, &format!("normalDist_seed{seed}.txt"));
        for (mean, sd) in ND_PARAMS {
            for _ in 0..20 {
                writeln!(
                    fp,
                    "{mean} {sd} {}",
                    random_number_normal_distribution(mean, sd)
                )
                .unwrap();
            }
        }
    }
}

fn capture_normal_table(dir: &Path) {
    let mut fp = open_out(dir, "normal_table.txt");
    with_state(|state| {
        for i in 0..NORMAL_TABLE_SIZE {
            writeln!(fp, "{}", state.normal_table[i]).unwrap();
        }
    });
}

fn main() {
    let dir: PathBuf = env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("tests/golden/rng"), PathBuf::from);
    fs::create_dir_all(&dir).unwrap_or_else(|err| {
        eprintln!("rng_capture: cannot create {}: {err}", dir.display());
        process::exit(1);
    });

    capture_invariant(&dir);
    capture_rnd_sequences(&dir);
    capture_random_number(&dir);
    capture_normal_dist(&dir);
    capture_normal_table(&dir);
    println!("rng_capture: wrote golden files to {}", dir.display());
}
