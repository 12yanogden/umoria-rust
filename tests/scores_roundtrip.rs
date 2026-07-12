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
use umoria::game_save::{
    read_high_score, save_high_score, set_c_getc_eof_mode, set_xor_byte, test_buffer_bytes,
    test_buffer_inject, test_reset_buffer, HighScore, HIGH_SCORE_RECORD_SIZE,
};

#[test]
fn scores_byte_exact_roundtrip_matches_expected_golden() {
    let manifest = load_manifest().expect("manifest.json should parse");
    let entry = manifest
        .goldens
        .iter()
        .find(|g| g.id == "scores_scores_initial")
        .expect("scores_scores_initial golden must exist in manifest");

    assert_eq!(entry.kind, GoldenKind::Scores);
    let golden = read_golden_bytes(entry);
    assert!(
        golden.len() >= 3 + HIGH_SCORE_RECORD_SIZE,
        "golden scores artifact must contain a version header and at least one record"
    );

    // Preserve the golden version header; decode each HighScore record and re-encode.
    let mut rebuilt = golden[..3].to_vec();
    let mut offset = 3usize;
    while offset + HIGH_SCORE_RECORD_SIZE <= golden.len() {
        test_buffer_inject(&golden[offset..]);
        set_c_getc_eof_mode(true);
        umoria::ui_io::test_set_eof_flag(0);

        let mut score = HighScore::default();
        read_high_score(&mut score).expect("decode high-score record");

        test_reset_buffer();
        set_xor_byte(0);
        save_high_score(&score).expect("re-encode high-score record");
        rebuilt.extend_from_slice(&test_buffer_bytes());

        offset += HIGH_SCORE_RECORD_SIZE;
    }

    assert_eq!(
        rebuilt.len(),
        golden.len(),
        "rebuilt scores file length must match golden"
    );
    if let Some(diff) = byte_diff(&golden, &rebuilt) {
        panic!("{}", diff.render());
    }
}
