//! Differential fidelity harness smoke + helper unit tests.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use common::{
    byte_diff, golden_root, load_manifest, load_rng_sequence, manifest_path, verify_manifest,
    GoldenKind,
};
use std::fs;

#[test]
fn byte_diff_equal_buffers_returns_none() {
    let buf = b"hello world";
    assert!(byte_diff(buf, buf).is_none());
}

#[test]
fn byte_diff_single_byte_mismatch_reports_offset_and_values() {
    let expected = b"hello world";
    let mut actual = expected.to_vec();
    actual[7] = b'X';

    let diff = byte_diff(expected, &actual).expect("single-byte mismatch should produce a diff");
    assert_eq!(diff.offset, 7);
    assert_eq!(diff.expected, b'o');
    assert_eq!(diff.actual, b'X');
    assert!(
        diff.render().contains("0007"),
        "hex window should include the mismatch offset: {}",
        diff.render()
    );
    assert!(
        diff.render().contains("6f"),
        "hex window should show expected byte 0x6f: {}",
        diff.render()
    );
    assert!(
        diff.render().contains("58"),
        "hex window should show actual byte 0x58: {}",
        diff.render()
    );
}

#[test]
fn byte_diff_length_mismatch_reports_shorter_length_and_offset() {
    let expected = b"abcdef";
    let actual = b"abc";

    let diff = byte_diff(expected, actual).expect("length mismatch should produce a diff");
    assert_eq!(diff.offset, 3);
    assert_eq!(diff.expected_len, 6);
    assert_eq!(diff.actual_len, 3);
    assert!(
        diff.render().contains("length mismatch"),
        "rendered diff should mention length mismatch: {}",
        diff.render()
    );
}

#[test]
fn manifest_parser_loads_real_manifest_and_verifies_hashes() {
    let manifest = load_manifest().expect("manifest.json should parse");
    assert_eq!(manifest.umoria_version, "5.7.15");
    assert!(
        !manifest.goldens.is_empty(),
        "manifest should list at least one golden entry"
    );

    let rng_count = manifest
        .goldens
        .iter()
        .filter(|g| g.kind == GoldenKind::Rng)
        .count();
    assert!(
        rng_count >= 10,
        "expected multiple rng goldens, got {rng_count}"
    );

    verify_manifest(&manifest).expect("on-disk golden files should match manifest hashes");
}

#[test]
fn golden_layout_self_check() {
    let root = golden_root();
    for sub in ["rng", "save", "scores", "transcripts"] {
        let dir = root.join(sub);
        assert!(
            dir.is_dir(),
            "missing golden subdirectory: {}",
            dir.display()
        );
    }

    let manifest = load_manifest().expect("manifest.json should parse");
    for entry in &manifest.goldens {
        let path = root.join(&entry.file);
        assert!(
            path.is_file(),
            "manifest entry {} points at missing file {}",
            entry.id,
            path.display()
        );
    }
}

#[test]
fn rng_sequence_loader_parses_golden_framing() {
    let path = golden_root().join("rng/z10001.txt");
    let values = load_rng_sequence(&path).expect("z10001 golden should load");
    assert_eq!(values.len(), 1);
    assert_eq!(values[0], 1_043_618_065);

    let path = golden_root().join("rng/rnd_seed1.txt");
    let values = load_rng_sequence(&path).expect("rnd_seed1 golden should load");
    assert_eq!(values.len(), 10_001);
    assert_eq!(values[0], 33_614);
}

#[test]
fn harness_smoke_loads_shared_helpers() {
    assert!(manifest_path().is_file());
    assert_eq!(
        common::regen_enabled(),
        std::env::var("UMORIA_REGEN_GOLDEN").is_ok_and(|v| v == "1")
    );

    let makefile = fs::read_to_string(common::repo_root().join("Makefile"))
        .expect("Makefile with ci/goldens targets must exist");
    assert!(makefile.contains("ci:"), "Makefile must define a ci target");
    assert!(
        makefile.contains("goldens:"),
        "Makefile must define a goldens target"
    );
}
