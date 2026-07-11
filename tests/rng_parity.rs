//! RNG golden parity checks.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use common::{golden_root, load_manifest, load_rng_sequence, GoldenKind};
use umoria::rng::{rnd, set_seed};

#[test]
fn rng_z10001_invariant_after_ten_thousand_discards() {
 // setRandomSeed(0) maps to internal seed 1; the 10000th rnd() is z[10001].
    set_seed(0);
    let mut last = 0;
    for _ in 0..10_000 {
        last = rnd();
    }
    assert_eq!(last as u32, 1_043_618_065);
}

#[test]
fn rng_golden_sequences_match_expected_capture() {
    let manifest = load_manifest().expect("manifest.json should parse");
    let root = golden_root();

    for entry in manifest
        .goldens
        .iter()
        .filter(|g| g.kind == GoldenKind::Rng && g.file.starts_with("rng/rnd_seed"))
    {
        let seed = entry.seed.expect("rnd golden must record its seed");
        let golden = load_rng_sequence(&root.join(&entry.file)).expect("load rng golden");

        set_seed(seed);
        for (i, &expected) in golden.iter().enumerate() {
            let actual = rnd() as u32;
            assert_eq!(
                actual, expected,
                "{} draw {}: expected {}, got {}",
                entry.id, i, expected, actual
            );
        }
    }
}
