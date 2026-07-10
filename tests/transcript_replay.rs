//! Transcript replay / screen-diff skeleton.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]
#![allow(
    unused_imports,
    reason = "re-exports and C++-mirrored imports kept for call-site parity"
)]

mod common;

use common::{
    golden_root, load_manifest, read_golden_bytes, screen_diff, GoldenKind, ScreenBuffer,
};

/// Smoke check: golden screen artifact exists and has expected dimensions.
///
/// Live PTY replay lives in [`transcript_replay_live_matches_golden_screen`] and is
/// exercised by the Differential CI job (`cargo test --features differential_live -- --ignored`).
#[test]
fn transcript_replay_matches_golden_screen() {
    let manifest = load_manifest().expect("manifest.json should parse");
    let screen_entry = manifest
        .goldens
        .iter()
        .find(|g| g.file == "transcripts/newchar_seed42.screen")
        .expect("newchar_seed42.screen golden must exist in manifest");

    assert_eq!(screen_entry.kind, GoldenKind::Transcript);
    let seed = screen_entry
        .seed
        .expect("screen golden must record its seed");
    let keys_path = golden_root().join("transcripts/newchar_seed42.keys");
    assert!(keys_path.is_file(), "keystroke script must exist");

    let expected_screen = ScreenBuffer::from_bytes(&read_golden_bytes(screen_entry));
    assert_eq!(
        expected_screen.rows(),
        24,
        "golden screen should be 24 rows; PTY replay not yet wired"
    );
    assert!(
        expected_screen.cols() >= 79,
        "golden screen should be ~80 columns wide"
    );
    let _ = seed;
    let _ = keys_path;
}

/// Live PTY replay against the C++/Rust binary — ignored by default.
///
/// Run with: `cargo test --features differential_live -- --ignored`
#[cfg(feature = "differential_live")]
#[test]
#[ignore = "PTY live replay; run via Differential CI job"]
fn transcript_replay_live_matches_golden_screen() {
    let manifest = load_manifest().expect("manifest.json should parse");
    let screen_entry = manifest
        .goldens
        .iter()
        .find(|g| g.file == "transcripts/newchar_seed42.screen")
        .expect("newchar_seed42.screen golden must exist in manifest");

    assert_eq!(screen_entry.kind, GoldenKind::Transcript);
    let seed = screen_entry
        .seed
        .expect("screen golden must record its seed");
    let keys_path = golden_root().join("transcripts/newchar_seed42.keys");
    assert!(keys_path.is_file(), "keystroke script must exist");

    let expected_screen = ScreenBuffer::from_bytes(&read_golden_bytes(screen_entry));
    let actual_screen = common::replay_transcript(seed, &keys_path, screen_entry.env())
        .expect("replay transcript under PTY");
    if let Some(diff) = screen_diff(&expected_screen, &actual_screen) {
        panic!("screen mismatch:\n{}", diff.render());
    }
}
