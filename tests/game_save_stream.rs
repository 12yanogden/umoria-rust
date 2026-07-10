//! XOR stream primitives & serializers.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use std::fs;
use std::path::PathBuf;

use common::{byte_diff, golden_root, load_manifest, read_golden_bytes};
use umoria::dice::Dice;
use umoria::game_save::{
    self, get_byte, rd_bool, rd_byte, rd_bytes, rd_item, rd_long, rd_monster, rd_short, rd_shorts,
    rd_string, read_high_score, save_high_score, set_fileptr, set_xor_byte, test_buffer_bytes,
    test_buffer_len, test_reset_buffer, test_rewind_buffer, test_write_raw, wr_bool, wr_byte,
    wr_bytes, wr_item, wr_long, wr_monster, wr_short, wr_shorts, wr_string, HighScore,
    HIGH_SCORE_RECORD_SIZE,
};
use umoria::inventory::{Inventory, INSCRIP_SIZE};
use umoria::monster::Monster;
use umoria::types::Coord_t;

fn golden_game_save(name: &str) -> PathBuf {
    golden_root().join("game_save").join(name)
}

fn with_buffer<F>(initial_xor: u8, f: F)
where
    F: FnOnce(),
{
    test_reset_buffer();
    set_xor_byte(initial_xor);
    f();
}

fn assert_bytes(expected: &[u8]) {
    assert_eq!(test_buffer_bytes(), expected);
}

fn sentinel_item() -> Inventory {
    let mut inscription = [0i8; INSCRIP_SIZE as usize];
    for (slot, ch) in b"@sig!\0".iter().copied().enumerate() {
        inscription[slot] = ch as i8;
    }
    Inventory {
        id: 0xBEEF,
        special_name_id: 0xAB,
        inscription,
        flags: 0xDEADBEEF,
        category_id: 0x12,
        sprite: 0x34,
        misc_use: -1,
        cost: -2_147_483_647,
        sub_category_id: 0x56,
        items_count: 0x78,
        weight: 400,
        to_hit: -5,
        to_damage: -16,
        ac: 20,
        to_ac: -20,
        damage: Dice { dice: 3, sides: 6 },
        depth_first_found: 99,
        identification: 0xCC,
    }
}

fn sentinel_monster() -> Monster {
    Monster {
        hp: -360,
        sleep_count: 7,
        speed: -4,
        creature_id: 0x0042,
        pos: Coord_t { y: 15, x: 23 },
        distance_from_player: 3,
        lit: true,
        stunned_amount: 2,
        confused_amount: 1,
    }
}

fn assert_item_eq(expected: &Inventory, actual: &Inventory) {
    assert_eq!(expected.id, actual.id);
    assert_eq!(expected.special_name_id, actual.special_name_id);
    assert_eq!(expected.inscription, actual.inscription);
    assert_eq!(expected.flags, actual.flags);
    assert_eq!(expected.category_id, actual.category_id);
    assert_eq!(expected.sprite, actual.sprite);
    assert_eq!(expected.misc_use, actual.misc_use);
    assert_eq!(expected.cost, actual.cost);
    assert_eq!(expected.sub_category_id, actual.sub_category_id);
    assert_eq!(expected.items_count, actual.items_count);
    assert_eq!(expected.weight, actual.weight);
    assert_eq!(expected.to_hit, actual.to_hit);
    assert_eq!(expected.to_damage, actual.to_damage);
    assert_eq!(expected.ac, actual.ac);
    assert_eq!(expected.to_ac, actual.to_ac);
    assert_eq!(expected.damage, actual.damage);
    assert_eq!(expected.depth_first_found, actual.depth_first_found);
    assert_eq!(expected.identification, actual.identification);
}

fn assert_monster_eq(expected: &Monster, actual: &Monster) {
    assert_eq!(expected.hp, actual.hp);
    assert_eq!(expected.sleep_count, actual.sleep_count);
    assert_eq!(expected.speed, actual.speed);
    assert_eq!(expected.creature_id, actual.creature_id);
    assert_eq!(expected.pos.y, actual.pos.y);
    assert_eq!(expected.pos.x, actual.pos.x);
    assert_eq!(expected.distance_from_player, actual.distance_from_player);
    assert_eq!(expected.lit, actual.lit);
    assert_eq!(expected.stunned_amount, actual.stunned_amount);
    assert_eq!(expected.confused_amount, actual.confused_amount);
}

fn scores_initial_first_record() -> HighScore {
    let manifest = load_manifest().expect("manifest.json should parse");
    let entry = manifest
        .goldens
        .iter()
        .find(|g| g.id == "scores_scores_initial")
        .expect("scores_scores_initial golden must exist in manifest");
    let file = read_golden_bytes(entry);
    let record = &file[3..3 + HIGH_SCORE_RECORD_SIZE];

    inject_test_buffer(record);

    let mut score = HighScore::default();
    read_high_score(&mut score).expect("decode scores_initial record 0");
    score
}

fn inject_test_buffer(bytes: &[u8]) {
    umoria::game_save::test_buffer_inject(bytes);
}

// ---------------------------------------------------------------------------
// 1. wrByte XOR chain
// ---------------------------------------------------------------------------
#[test]
fn test_wrbyte_xor_chain() {
    with_buffer(0, || {
        wr_byte(0x41).unwrap();
        assert_eq!(game_save::xor_byte(), 0x41);
        wr_byte(0x42).unwrap();
        assert_eq!(game_save::xor_byte(), 0x41 ^ 0x42);
        assert_bytes(&[0x41, 0x03]);
    });
}

// ---------------------------------------------------------------------------
// 2. wrShort little-endian chained
// ---------------------------------------------------------------------------
#[test]
fn test_wrshort_little_endian_chained() {
    with_buffer(0xA5, || {
        wr_short(0x1234).unwrap();
        // low: 0xA5 ^ 0x34 = 0x91; high xor uses running 0x91 ^ 0x12 = 0x83
        assert_eq!(game_save::xor_byte(), 0x83);
        assert_bytes(&[0x91, 0x83]);
    });
}

// ---------------------------------------------------------------------------
// 3. wrLong four-byte order
// ---------------------------------------------------------------------------
#[test]
fn test_wrlong_four_byte_order() {
    with_buffer(0, || {
        wr_long(0x1122_3344).unwrap();
        assert_bytes(&[0x44, 0x77, 0x55, 0x44]);
    });
}

// ---------------------------------------------------------------------------
// 4. wrString includes NUL
// ---------------------------------------------------------------------------
#[test]
fn test_wrstring_includes_nul() {
    with_buffer(0, || {
        wr_string(b"Bob").unwrap();
        assert_eq!(test_buffer_len(), 4);
        assert_bytes(&[0x42, 0x2D, 0x4F, 0x4F]);
    });

    with_buffer(0, || {
        wr_string(b"").unwrap();
        assert_eq!(test_buffer_len(), 1);
        assert_bytes(&[0]);
    });
}

// ---------------------------------------------------------------------------
// 5. wrBytes / wrShorts counts
// ---------------------------------------------------------------------------
#[test]
fn test_wrbytes_and_wrshorts_counts() {
    with_buffer(0x10, || {
        wr_bytes(&[0x01, 0x02, 0x03]).unwrap();
        assert_eq!(test_buffer_len(), 3);
        assert_bytes(&[0x11, 0x13, 0x10]);
    });

    with_buffer(0x20, || {
        wr_shorts(&[0x0102, 0x0304]).unwrap();
        assert_eq!(test_buffer_len(), 4);
        assert_bytes(&[0x22, 0x23, 0x27, 0x24]);
    });
}

// ---------------------------------------------------------------------------
// 6. wrBool is 0/1
// ---------------------------------------------------------------------------
#[test]
fn test_wrbool_is_zero_or_one() {
    with_buffer(0x5C, || {
        wr_bool(true).unwrap();
        assert_eq!(test_buffer_bytes(), &[0x5C ^ 1]);
        wr_bool(false).unwrap();
        assert_eq!(test_buffer_bytes(), &[0x5C ^ 1, 0x5C ^ 1]);
    });
}

// ---------------------------------------------------------------------------
// 7. getByte is raw (no XOR)
// ---------------------------------------------------------------------------
#[test]
fn test_getbyte_is_raw_no_xor() {
    with_buffer(0xAB, || {
        test_write_raw(&[0xFE, 0x98]).unwrap();
        test_rewind_buffer().unwrap();
        assert_eq!(game_save::xor_byte(), 0xAB);
        assert_eq!(get_byte().unwrap(), 0xFE);
        assert_eq!(game_save::xor_byte(), 0xAB);
        assert_eq!(get_byte().unwrap(), 0x98);
        assert_eq!(game_save::xor_byte(), 0xAB);
    });
}

// ---------------------------------------------------------------------------
// 8. rd* mirror wr* round-trip
// ---------------------------------------------------------------------------
#[test]
fn test_rd_primitives_mirror_wr() {
    with_buffer(0x3C, || {
        wr_byte(0xDE).unwrap();
        wr_short(0xBEEF).unwrap();
        wr_long(0xCAFE_BABE).unwrap();
        wr_bytes(&[0x11, 0x22, 0x33]).unwrap();
        wr_string(b"xy").unwrap();
        wr_shorts(&[0x0102, 0x0304]).unwrap();
        wr_bool(true).unwrap();
        let final_xor = game_save::xor_byte();

        test_rewind_buffer().unwrap();
        set_xor_byte(0x3C);
        assert_eq!(rd_byte().unwrap(), 0xDE);
        assert_eq!(rd_short().unwrap(), 0xBEEF);
        assert_eq!(rd_long().unwrap(), 0xCAFE_BABE);
        let mut bytes = [0u8; 3];
        rd_bytes(&mut bytes).unwrap();
        assert_eq!(bytes, [0x11, 0x22, 0x33]);
        let mut s = [0u8; 8];
        rd_string(&mut s).unwrap();
        assert_eq!(&s[..3], b"xy\0");
        let mut shorts = [0u16; 2];
        rd_shorts(&mut shorts).unwrap();
        assert_eq!(shorts, [0x0102, 0x0304]);
        assert!(rd_bool().unwrap());
        assert_eq!(game_save::xor_byte(), final_xor);
    });
}

// ---------------------------------------------------------------------------
// 9. rdShort / rdLong xor_byte progression
// ---------------------------------------------------------------------------
#[test]
fn test_rdshort_rdlong_xor_state_progression() {
    with_buffer(0, || {
        wr_short(0xABCD).unwrap();
        wr_long(0x0123_4567).unwrap();
        let bytes = test_buffer_bytes();
        assert_eq!(bytes.len(), 6);

        test_rewind_buffer().unwrap();
        set_xor_byte(0);
        assert_eq!(rd_short().unwrap(), 0xABCD);
        assert_eq!(game_save::xor_byte(), bytes[1]);

        test_rewind_buffer().unwrap();
        set_xor_byte(0);
        assert_eq!(rd_short().unwrap(), 0xABCD);
        assert_eq!(rd_long().unwrap(), 0x0123_4567);
        assert_eq!(game_save::xor_byte(), bytes[5]);
    });
}

// ---------------------------------------------------------------------------
// 10. rdString reads terminator
// ---------------------------------------------------------------------------
#[test]
fn test_rdstring_reads_terminator() {
    with_buffer(0x7E, || {
        wr_string(b"Z").unwrap();
        test_rewind_buffer().unwrap();
        set_xor_byte(0x7E);
        let mut buf = [0xFF; 4];
        rd_string(&mut buf).unwrap();
        assert_eq!(&buf[..2], b"Z\0");
        assert_eq!(game_save::xor_byte(), test_buffer_bytes()[1]);
    });
}

// ---------------------------------------------------------------------------
// 11. Item round-trip all fields
// ---------------------------------------------------------------------------
#[test]
fn test_item_roundtrip_all_fields() {
    let item = sentinel_item();
    with_buffer(0x5A, || {
        wr_item(&item).unwrap();
        let written = test_buffer_bytes();
        test_rewind_buffer().unwrap();
        set_xor_byte(0x5A);
        let mut decoded = Inventory::default();
        rd_item(&mut decoded).unwrap();
        assert_item_eq(&item, &decoded);
        assert_eq!(
            written.len(),
            fs::read(golden_game_save("wr_item_seed5a.bin"))
                .unwrap()
                .len()
        );
    });
}

// ---------------------------------------------------------------------------
// 12. Monster round-trip all fields
// ---------------------------------------------------------------------------
#[test]
fn test_monster_roundtrip_all_fields() {
    let monster = sentinel_monster();
    with_buffer(0x5A, || {
        wr_monster(&monster).unwrap();
        let written = test_buffer_bytes();
        test_rewind_buffer().unwrap();
        set_xor_byte(0x5A);
        let mut decoded = Monster::default();
        rd_monster(&mut decoded).unwrap();
        assert_monster_eq(&monster, &decoded);
        assert_eq!(
            written.len(),
            fs::read(golden_game_save("wr_monster_seed5a.bin"))
                .unwrap()
                .len()
        );
    });
}

// ---------------------------------------------------------------------------
// 13. HighScore record size + round-trip
// ---------------------------------------------------------------------------
#[test]
fn test_highscore_record_is_64_bytes() {
    let score = scores_initial_first_record();
    with_buffer(0, || {
        set_xor_byte(0);
        save_high_score(&score).unwrap();
        assert_eq!(test_buffer_len(), HIGH_SCORE_RECORD_SIZE);
        assert_eq!(test_buffer_bytes()[0], 0);

        test_rewind_buffer().unwrap();
        let mut round = HighScore::default();
        read_high_score(&mut round).unwrap();
        assert_eq!(round, score);
    });
}

// ---------------------------------------------------------------------------
// 14. HighScore matches C++ golden (scores_initial.dat record 0)
// ---------------------------------------------------------------------------
#[test]
fn test_highscore_matches_cpp_golden() {
    let manifest = load_manifest().expect("manifest.json should parse");
    let entry = manifest
        .goldens
        .iter()
        .find(|g| g.id == "scores_scores_initial")
        .expect("scores_scores_initial golden must exist in manifest");
    let golden_file = read_golden_bytes(entry);
    let cpp_record = &golden_file[3..3 + HIGH_SCORE_RECORD_SIZE];

    let score = scores_initial_first_record();
    with_buffer(0, || {
        set_xor_byte(0);
        save_high_score(&score).unwrap();
        let rust_record = test_buffer_bytes();
        assert!(
            byte_diff(cpp_record, &rust_record).is_none(),
            "Rust save_high_score must match C++ scores_initial.dat record 0"
        );
    });
}

// ---------------------------------------------------------------------------
// 15. wrItem / wrMonster match C++ goldens (hand-captured XOR bytes)
// ---------------------------------------------------------------------------
#[test]
fn test_wr_item_matches_cpp_golden() {
    let golden = fs::read(golden_game_save("wr_item_seed5a.bin")).expect("item golden");
    let item = sentinel_item();
    with_buffer(0x5A, || {
        wr_item(&item).unwrap();
        assert!(
            byte_diff(&golden, &test_buffer_bytes()).is_none(),
            "wr_item ciphertext must match C++ golden"
        );
    });
}

#[test]
fn test_wr_monster_matches_cpp_golden() {
    let golden = fs::read(golden_game_save("wr_monster_seed5a.bin")).expect("monster golden");
    let monster = sentinel_monster();
    with_buffer(0x5A, || {
        wr_monster(&monster).unwrap();
        assert!(
            byte_diff(&golden, &test_buffer_bytes()).is_none(),
            "wr_monster ciphertext must match C++ golden"
        );
    });
}

// ---------------------------------------------------------------------------
// 16. setFileptr targets score file
// ---------------------------------------------------------------------------
#[test]
fn test_setfileptr_targets_score_file() {
    let dir = std::env::temp_dir().join(format!("umoria_gs511_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("score.bin");

    set_xor_byte(0x12);
    set_fileptr(fs::File::create(&path).unwrap());
    wr_byte(0x34).unwrap();

    set_fileptr(fs::File::open(&path).unwrap());
    set_xor_byte(0x12);
    assert_eq!(rd_byte().unwrap(), 0x34);

    let _ = fs::remove_dir_all(dir);
}
