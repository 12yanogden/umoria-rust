//! Phase 1.5 — score byte-exact round-trip skeleton (staged until phase_5).

mod common;

use common::{byte_diff, load_manifest, read_golden_bytes, GoldenKind};

#[test]
#[ignore = "enable when phase_5 lands (scores)"]
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
        "golden scores artifact must be non-empty; round-trip lands in phase_5"
    );

    // Phase 5: round-trip via umoria::scores, byte_diff vs golden.
    let _ = byte_diff(&golden, &golden);
}
