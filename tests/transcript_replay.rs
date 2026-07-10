//! Phase 1.5 — transcript replay / screen-diff skeleton (staged until phase_5).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]
#![allow(unused_imports)]

mod common;

use common::{
    golden_root, load_manifest, read_golden_bytes, screen_diff, GoldenKind, ScreenBuffer,
};

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

    #[cfg(feature = "differential_live")]
    {
        let actual_screen = common::replay_transcript(seed, &keys_path, screen_entry.env())
            .expect("replay transcript under PTY");
        if let Some(diff) = screen_diff(&expected_screen, &actual_screen) {
            panic!("screen mismatch:\n{}", diff.render());
        }
    }

    #[cfg(not(feature = "differential_live"))]
    {
        assert_eq!(
            expected_screen.rows(),
            24,
            "golden screen should be 24 rows; PTY replay lands in phase_5"
        );
        assert!(
            expected_screen.cols() >= 79,
            "golden screen should be ~80 columns wide"
        );
        let _ = seed;
        let _ = keys_path;
    }
}
