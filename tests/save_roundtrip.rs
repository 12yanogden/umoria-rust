//! Save byte-exact round-trip vs golden fixtures.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use std::collections::HashSet;

use common::{byte_diff, golden_root, load_manifest, read_golden_bytes, GoldenEntry, GoldenKind};
use umoria::game::{reset_for_new_game, with_state_mut};
use umoria::game_save::{
    save_char, test_buffer_bytes, test_load_save_from_bytes, test_reset_buffer,
    test_set_forced_seed_byte, test_set_unix_time,
};

fn decode_xor_chain(data: &[u8], resets: &[usize]) -> Vec<u8> {
    let reset_set: HashSet<usize> = resets.iter().copied().collect();
    let mut out = Vec::with_capacity(data.len());
    let mut prev = 0u8;
    for (index, &byte) in data.iter().enumerate() {
        if reset_set.contains(&index) {
            prev = 0;
        }
        out.push(byte ^ prev);
        prev = byte;
    }
    out
}

fn apply_mask(data: &[u8], ranges: &[common::VolatileByteRange]) -> Vec<u8> {
    let mut masked = data.to_vec();
    for range in ranges {
        for index in range.offset..range.offset.saturating_add(range.length).min(masked.len()) {
            masked[index] = 0;
        }
    }
    masked
}

fn masked_save_diff(entry: &GoldenEntry, expected: &[u8], actual: &[u8]) -> Option<common::Diff> {
    let decoded_expected = decode_xor_chain(expected, &[0, 1, 2, 3]);
    let decoded_actual = decode_xor_chain(actual, &[0, 1, 2, 3]);
    let masked_expected = apply_mask(&decoded_expected, &entry.volatile_byte_ranges);
    let masked_actual = apply_mask(&decoded_actual, &entry.volatile_byte_ranges);
    byte_diff(&masked_expected, &masked_actual)
}

fn setup_save_harness() {
    umoria::ui_io::test_set_ncurses_stub(true);
    reset_for_new_game(None);
}

#[test]
fn save_byte_exact_roundtrip_matches_expected_golden() {
    let manifest = load_manifest().expect("manifest.json should parse");
    let entry = manifest
        .goldens
        .iter()
        .find(|g| g.id == "save_newchar_seed42")
        .expect("save_newchar_seed42 golden must exist in manifest");

    assert_eq!(entry.kind, GoldenKind::Save);
    let golden = read_golden_bytes(entry);
    assert!(!golden.is_empty(), "golden save artifact must be non-empty");

    setup_save_harness();
    test_load_save_from_bytes(&golden).expect("load golden save");
    let seed = golden[3];
    test_set_forced_seed_byte(Some(seed));
    test_set_unix_time(None);
    with_state_mut(|state| {
        state.game.character_saved = false;
    });
    test_reset_buffer();
    assert!(save_char("game.sav"));

    let actual = test_buffer_bytes();
    assert_eq!(
        actual.len(),
        golden.len(),
        "save length must match expected"
    );
    if let Some(diff) = masked_save_diff(entry, &golden, &actual) {
        panic!(
            "masked save bytes must match expected golden: {}",
            diff.render()
        );
    }
}

#[test]
fn save_expected_golden_is_readable_by_rust() {
    let manifest = load_manifest().expect("manifest.json should parse");
    let entry = manifest
        .goldens
        .iter()
        .find(|g| g.id == "save_newchar_seed42")
        .expect("save_newchar_seed42 golden must exist in manifest");

    let golden = read_golden_bytes(entry);
    let path = golden_root().join(&entry.file);
    assert!(
        path.is_file(),
        "golden save file must exist at {}",
        path.display()
    );
    assert!(golden.len() > 4, "golden save must have header bytes");

    setup_save_harness();
    test_load_save_from_bytes(&golden).expect("Rust must decode golden save");
    assert!(umoria::game::with_state(|state| state.dg.game_turn >= 0));
    assert!(umoria::game::with_state(|state| state.game.magic_seed != 0));
}
