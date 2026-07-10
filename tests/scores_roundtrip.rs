//! Score byte-exact round-trip against golden fixtures.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use common::{byte_diff, load_manifest, read_golden_bytes, GoldenKind};

#[test]
#[ignore = "enable when score round-trip is wired"]
fn scores_byte_exact_roundtrip_matches_cpp_golden() {
    let manifest = load_manifest().expect("manifest.json should parse");
    let entry = manifest
        .goldens
        .iter()
        .find(|g| g.id == "scores_scores_initial")
        .expect("scores_scores_initial golden must exist in manifest");

    assert_eq!(entry.kind, GoldenKind::Scores);
    let golden = read_golden_bytes(entry);
    assert!(
        !golden.is_empty(),
        "golden scores artifact must be non-empty; round-trip not yet wired"
    );

    // Phase 5: round-trip via umoria::scores, byte_diff vs golden.
    let _ = byte_diff(&golden, &golden);
}
