//! Phase 5.2 — high-score file (`scores.cpp` → `crate::scores`).

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{load_manifest, read_golden_bytes};
use umoria::game::{reset_for_new_game, with_state, with_state_mut};
use umoria::game_save::{
    read_high_score, save_high_score, set_c_getc_eof_mode, set_xor_byte,
    test_buffer_bytes, test_reset_buffer, HighScore, HIGH_SCORE_RECORD_SIZE,
};
use umoria::inventory::{Inventory, PLAYER_INVENTORY_SIZE};
use umoria::player::{player_set_gender, PLAYER_NAME_SIZE};
use umoria::scores::{
    format_show_scores_line, high_score_gender_label, player_calculate_total_points,
    record_new_high_score, show_scores_screen,
    strip_died_from_for_high_score, test_build_new_high_score_entry, test_set_scores_path,
    HighScore_t, HIGH_SCORE_RECORD_STRIDE, MAX_HIGH_SCORE_ENTRIES, SHOW_SCORES_HEADER,
};
use umoria::store_inventory::store_item_value;
use umoria::ui_io;
use umoria::version::{CURRENT_VERSION_MAJOR, CURRENT_VERSION_MINOR, CURRENT_VERSION_PATCH};

fn temp_scores_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("umoria_scores52_{tag}_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn setup_harness() {
    ui_io::test_set_ncurses_stub(true);
    ui_io::test_set_ui_capture(true);
    ui_io::test_set_panic_save(false);
    ui_io::test_set_eof_flag(0);
    reset_for_new_game(None);
}

fn copy_cstr(dest: &mut [u8], src: &str) {
    for (index, byte) in dest.iter_mut().enumerate() {
        *byte = if index < src.len() {
            src.as_bytes()[index]
        } else {
            0
        };
    }
}

fn append_encrypted_record(out: &mut Vec<u8>, score: &HighScore) {
    test_reset_buffer();
    set_xor_byte(0);
    save_high_score(score).unwrap();
    out.extend_from_slice(&test_buffer_bytes());
}

fn build_score_file(records: &[HighScore]) -> Vec<u8> {
    let mut data = vec![CURRENT_VERSION_MAJOR, CURRENT_VERSION_MINOR, CURRENT_VERSION_PATCH];
    for record in records {
        append_encrypted_record(&mut data, record);
    }
    data
}

fn sample_score(points: i32, died_from: &str) -> HighScore {
    let mut score = HighScore::default();
    score.points = points;
    score.birth_date = 42_000;
    score.mhp = 100;
    score.chp = 0;
    score.dungeon_depth = 5;
    score.level = 10;
    score.deepest_dungeon_depth = 20;
    score.gender = b'M';
    score.race = 0;
    score.character_class = 0;
    copy_cstr(&mut score.name, "Hero");
    copy_cstr(&mut score.died_from, died_from);
    score
}

fn decode_score_file(data: &[u8]) -> Vec<HighScore> {
    let mut out = Vec::new();
    let mut offset = 3usize;
    while offset + HIGH_SCORE_RECORD_STRIDE <= data.len() {
        umoria::game_save::test_buffer_inject(&data[offset..]);
        set_c_getc_eof_mode(true);
        ui_io::test_set_eof_flag(0);
        let mut score = HighScore::default();
        read_high_score(&mut score).unwrap();
        out.push(score);
        offset += HIGH_SCORE_RECORD_STRIDE;
    }
    out
}

fn install_scores_path(dir: &Path) -> PathBuf {
    let path = dir.join("scores.dat");
    test_set_scores_path(Some(&path));
    path
}

fn setup_player(died_from: &str) {
    with_state_mut(|state| {
        state.game.noscore = 0;
        state.py.misc.max_exp = 1_000;
        state.py.misc.max_dungeon_depth = 5;
        state.py.misc.au = 500;
        state.py.misc.date_of_birth = 99_999;
        state.py.misc.max_hp = 120;
        state.py.misc.current_hp = 0;
        state.py.misc.level = 12;
        state.py.misc.race_id = 0;
        state.py.misc.class_id = 0;
        state.py.misc.gender = true;
        copy_cstr(&mut state.py.misc.name, "Tester");
        copy_cstr(&mut state.game.character_died_from, died_from);
        state.dg.current_level = 3;
        state.py.max_score = 0;
        state.py.inventory = [Inventory::default(); PLAYER_INVENTORY_SIZE as usize];
    });
}

// ---------------------------------------------------------------------------
// Step 0 — stride / field size
// ---------------------------------------------------------------------------

#[test]
fn test_record_stride_matches_cpp() {
    assert_eq!(HIGH_SCORE_RECORD_STRIDE, 73);
    assert_eq!(HIGH_SCORE_RECORD_SIZE, 73);
}

#[test]
fn test_high_score_field_byte_sum_is_72() {
    let mut score = HighScore::default();
    score.points = 123;
    score.birth_date = 456;
    score.uid = 7;
    score.mhp = 80;
    score.chp = 10;
    score.dungeon_depth = 1;
    score.level = 2;
    score.deepest_dungeon_depth = 3;
    score.gender = b'F';
    score.race = 4;
    score.character_class = 5;
    copy_cstr(&mut score.name, "Name");
    copy_cstr(&mut score.died_from, "orc");

    test_reset_buffer();
    set_xor_byte(0);
    let start = umoria::game_save::test_buffer_len();
    save_high_score(&score).unwrap();
    let end = umoria::game_save::test_buffer_len();
    assert_eq!(end - start, HIGH_SCORE_RECORD_STRIDE);

    test_reset_buffer();
    set_xor_byte(0);
    save_high_score(&score).unwrap();
    umoria::game_save::test_rewind_buffer().unwrap();
    let mut round = HighScore::default();
    read_high_score(&mut round).unwrap();
    assert_eq!(round, score);
}

// ---------------------------------------------------------------------------
// Step 1 — gender label
// ---------------------------------------------------------------------------

#[test]
fn test_gender_label_male() {
    setup_harness();
    player_set_gender(true);
    assert_eq!(high_score_gender_label(), b'M');
}

#[test]
fn test_gender_label_female() {
    setup_harness();
    player_set_gender(false);
    assert_eq!(high_score_gender_label(), b'F');
}

// ---------------------------------------------------------------------------
// Step 2 — playerCalculateTotalPoints
// ---------------------------------------------------------------------------

#[test]
fn test_points_basic() {
    setup_harness();
    with_state_mut(|state| {
        state.py.misc.max_exp = 1_000;
        state.py.misc.max_dungeon_depth = 5;
        state.py.misc.au = 500;
        state.dg.current_level = 3;
        state.py.max_score = 0;
        state.py.inventory = [Inventory::default(); PLAYER_INVENTORY_SIZE as usize];
    });
    assert_eq!(player_calculate_total_points(), 1_655);
}

#[test]
fn test_points_inventory_sum() {
    setup_harness();
    with_state_mut(|state| {
        state.py.misc.max_exp = 0;
        state.py.misc.max_dungeon_depth = 0;
        state.py.misc.au = 0;
        state.dg.current_level = 0;
        state.py.max_score = 0;
        state.py.inventory[0].cost = 250;
        state.py.inventory[0].category_id = 99;
    });
    let item_value = with_state(|state| store_item_value(&state.py.inventory[0]));
    assert_eq!(player_calculate_total_points(), item_value);
}

#[test]
fn test_points_au_integer_division() {
    setup_harness();
    with_state_mut(|state| {
        state.py.misc.max_exp = 0;
        state.py.misc.max_dungeon_depth = 0;
        state.dg.current_level = 0;
        state.py.max_score = 0;
        state.py.inventory = [Inventory::default(); PLAYER_INVENTORY_SIZE as usize];
        state.py.misc.au = 199;
    });
    assert_eq!(player_calculate_total_points(), 1);

    with_state_mut(|state| state.py.misc.au = 99);
    assert_eq!(player_calculate_total_points(), 0);
}

#[test]
fn test_points_clamped_to_max_score() {
    setup_harness();
    with_state_mut(|state| {
        state.py.misc.max_exp = 10;
        state.py.misc.max_dungeon_depth = 0;
        state.py.misc.au = 0;
        state.dg.current_level = 0;
        state.py.max_score = 9_999;
        state.py.inventory = [Inventory::default(); PLAYER_INVENTORY_SIZE as usize];
    });
    assert_eq!(player_calculate_total_points(), 9_999);
}

#[test]
fn test_points_current_level_factor() {
    setup_harness();
    with_state_mut(|state| {
        state.py.misc.max_exp = 0;
        state.py.misc.max_dungeon_depth = 0;
        state.py.misc.au = 0;
        state.dg.current_level = 4;
        state.py.max_score = 0;
        state.py.inventory = [Inventory::default(); PLAYER_INVENTORY_SIZE as usize];
    });
    assert_eq!(player_calculate_total_points(), 200);
}

// ---------------------------------------------------------------------------
// Step 3 — recordNewHighScore
// ---------------------------------------------------------------------------

#[test]
fn test_new_empty_file_writes_version_header() {
    setup_harness();
    let dir = temp_scores_dir("empty");
    let path = install_scores_path(&dir);
    fs::write(&path, []).unwrap();
    setup_player("an orc");

    record_new_high_score();

    let data = fs::read(&path).unwrap();
    assert_eq!(&data[..3], &[5, 7, 15]);
    assert_eq!(data.len(), 3 + HIGH_SCORE_RECORD_STRIDE);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_insert_descending_order() {
    setup_harness();
    let dir = temp_scores_dir("insert");
    let path = install_scores_path(&dir);
    let seed = build_score_file(&[
        sample_score(300, "alpha"),
        sample_score(200, "beta"),
        sample_score(100, "gamma"),
    ]);
    fs::write(&path, &seed).unwrap();
    setup_player("a goblin");
    with_state_mut(|state| {
        state.py.misc.max_exp = 150;
        state.py.misc.max_dungeon_depth = 0;
        state.py.misc.au = 0;
        state.dg.current_level = 0;
        state.py.inventory = [Inventory::default(); PLAYER_INVENTORY_SIZE as usize];
    });

    record_new_high_score();

    let data = fs::read(&path).unwrap();
    assert_eq!(&data[..3], &seed[..3]);
    let records = decode_score_file(&data);
    assert_eq!(records.len(), 4);
    assert!(records[0].points >= records[1].points);
    assert!(records[1].points >= records[2].points);
    assert!(records[2].points >= records[3].points);
    assert_eq!(records[2].points, 150);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_append_lowest_score_at_eof() {
    setup_harness();
    let dir = temp_scores_dir("append");
    let path = install_scores_path(&dir);
    let seed = build_score_file(&[sample_score(500, "top"), sample_score(400, "mid")]);
    fs::write(&path, &seed).unwrap();
    setup_player("starvation");
    with_state_mut(|state| {
        state.py.misc.max_exp = 50;
        state.py.misc.max_dungeon_depth = 0;
        state.py.misc.au = 0;
        state.dg.current_level = 0;
        state.py.inventory = [Inventory::default(); PLAYER_INVENTORY_SIZE as usize];
    });

    record_new_high_score();

    let data = fs::read(&path).unwrap();
    assert_eq!(&data[..seed.len()], &seed);
    let records = decode_score_file(&data);
    assert_eq!(records.len(), 3);
    assert_eq!(records[2].points, 50);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_shuffle_down_preserves_all_records() {
    setup_harness();
    let dir = temp_scores_dir("shuffle");
    let path = install_scores_path(&dir);
    let seed = build_score_file(&[
        sample_score(900, "a"),
        sample_score(700, "b"),
        sample_score(500, "c"),
        sample_score(300, "d"),
    ]);
    fs::write(&path, &seed).unwrap();
    setup_player("trap");
    with_state_mut(|state| {
        state.py.misc.max_exp = 600;
        state.py.misc.max_dungeon_depth = 0;
        state.py.misc.au = 0;
        state.dg.current_level = 0;
        state.py.inventory = [Inventory::default(); PLAYER_INVENTORY_SIZE as usize];
    });

    record_new_high_score();

    let records = decode_score_file(&fs::read(&path).unwrap());
    assert_eq!(records.len(), 5);
    let points: Vec<i32> = records.iter().map(|r| r.points).collect();
    assert_eq!(points, vec![900, 700, 600, 500, 300]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_dedupe_uid_zero_saved_same_birthdate() {
    setup_harness();
    let dir = temp_scores_dir("dedupe");
    let path = install_scores_path(&dir);
    let mut existing = sample_score(500, "(saved)");
    existing.birth_date = 99_999;
    fs::write(&path, build_score_file(&[existing])).unwrap();
    setup_player("anything");

    record_new_high_score();

    let data = fs::read(&path).unwrap();
    assert_eq!(data.len(), 3 + HIGH_SCORE_RECORD_STRIDE);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_dedupe_no_match_when_gender_race_class_differ() {
    setup_harness();
    let dir = temp_scores_dir("nodupe");
    let path = install_scores_path(&dir);
    let mut existing = sample_score(500, "(saved)");
    existing.birth_date = 99_999;
    existing.gender = b'F';
    fs::write(&path, build_score_file(&[existing])).unwrap();
    setup_player("anything");

    record_new_high_score();

    let records = decode_score_file(&fs::read(&path).unwrap());
    assert_eq!(records.len(), 2);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_invalid_version_returns_without_write() {
    setup_harness();
    let dir = temp_scores_dir("badver");
    let path = install_scores_path(&dir);
    let seed = build_score_file(&[sample_score(100, "old")]);
    let mut bad = seed;
    bad[0] = 1;
    fs::write(&path, &bad).unwrap();
    setup_player("orc");

    record_new_high_score();

    assert_eq!(fs::read(&path).unwrap(), bad);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_noscore_flag_short_circuits() {
    setup_harness();
    let dir = temp_scores_dir("noscore");
    let path = install_scores_path(&dir);
    fs::write(&path, []).unwrap();
    setup_player("orc");
    with_state_mut(|state| state.game.noscore = 1);

    record_new_high_score();

    assert!(fs::read(&path).unwrap().is_empty());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_panic_save_short_circuits() {
    setup_harness();
    let dir = temp_scores_dir("panic");
    let path = install_scores_path(&dir);
    fs::write(&path, []).unwrap();
    setup_player("orc");
    ui_io::test_set_panic_save(true);

    record_new_high_score();

    assert!(fs::read(&path).unwrap().is_empty());
    let messages = ui_io::test_ui_messages();
    assert!(messages.iter().any(|m| m.contains("panic save")));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_cap_at_1000_entries() {
    setup_harness();
    let dir = temp_scores_dir("cap");
    let path = install_scores_path(&dir);
    let mut records = Vec::with_capacity(1_000);
    for i in 0..1_000 {
        records.push(sample_score(10_000 - i as i32, "cap"));
    }
    fs::write(&path, build_score_file(&records)).unwrap();
    setup_player("orc");
    with_state_mut(|state| {
        state.py.misc.max_exp = 1;
        state.py.misc.max_dungeon_depth = 0;
        state.py.misc.au = 0;
        state.dg.current_level = 0;
        state.py.inventory = [Inventory::default(); PLAYER_INVENTORY_SIZE as usize];
    });

    record_new_high_score();

    let data = fs::read(&path).unwrap();
    assert_eq!(decode_score_file(&data).len(), MAX_HIGH_SCORE_ENTRIES as usize);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_died_from_article_stripping() {
    let cases = [
        ("an orc", "orc"),
        ("a bat", "bat"),
        ("apple", "pple"),
        ("a  ogre", "ogre"),
        ("killed", "killed"),
    ];
    for (input, expected) in cases {
        let mut src = [0u8; 80];
        copy_cstr(&mut src, input);
        let out = strip_died_from_for_high_score(&src);
        assert_eq!(
            std::str::from_utf8(&out)
                .unwrap()
                .trim_end_matches('\0'),
            expected,
            "input={input:?}"
        );
    }
}

#[test]
fn test_golden_record_roundtrip_matches_cpp_initial() {
    let manifest = load_manifest().expect("manifest");
    let entry = manifest
        .goldens
        .iter()
        .find(|g| g.id == "scores_scores_initial")
        .expect("scores_scores_initial");
    let golden = read_golden_bytes(entry);

    setup_harness();
    let dir = temp_scores_dir("golden");
    let path = install_scores_path(&dir);
    fs::write(&path, &golden).unwrap();
    setup_player("an orc");
    with_state_mut(|state| {
        state.py.misc.max_exp = 1;
        state.py.misc.max_dungeon_depth = 0;
        state.py.misc.au = 0;
        state.dg.current_level = 0;
        state.py.inventory = [Inventory::default(); PLAYER_INVENTORY_SIZE as usize];
    });

    record_new_high_score();

    let actual = fs::read(&path).unwrap();
    assert!(
        actual.len() >= golden.len(),
        "new score should append one record"
    );
    assert_eq!(&actual[..golden.len()], &golden);
    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Step 4 — showScoresScreen
// ---------------------------------------------------------------------------

#[test]
fn test_show_scores_line_format_exact() {
    let mut score = HighScore_t::default();
    score.points = 12_345;
    score.name = {
        let mut name = [0u8; PLAYER_NAME_SIZE as usize];
        copy_cstr(&mut name, "Gandalf");
        name
    };
    score.gender = b'M';
    score.race = 0;
    score.character_class = 1;
    score.level = 7;
    score.died_from = {
        let mut died = [0u8; 25];
        copy_cstr(&mut died, "a fiery dragon");
        died
    };

    let stripped = strip_died_from_for_high_score(b"a fiery dragon\0");
    score.died_from = stripped;

    let line = format_show_scores_line(1, &score);
    assert_eq!(
        line,
        "1      12345 Gandalf             M Human      Mage     7 fiery dragon          "
    );
}

#[test]
fn test_show_scores_string_truncation() {
    let mut score = HighScore_t::default();
    score.points = 1;
    copy_cstr(&mut score.name, "ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    score.gender = b'F';
    score.race = 0;
    score.character_class = 0;
    score.level = 1;
    copy_cstr(&mut score.died_from, "abcdefghijklmnopqrstuvwxyz");

    let line = format_show_scores_line(99, &score);
    assert!(line.contains("ABCDEFGHIJKLMNOPQRS"));
    assert!(line.contains("abcdefghijklmnopqrs"));
}

#[test]
fn test_show_scores_header_line_literal() {
    assert_eq!(
        SHOW_SCORES_HEADER,
        "Rank  Points Name              Sex Race       Class  Lvl Killed By"
    );
}

#[test]
fn test_show_scores_pagination_20_per_page() {
    setup_harness();
    let dir = temp_scores_dir("pages");
    let path = install_scores_path(&dir);
    let records: Vec<_> = (0..25)
        .map(|i| sample_score(1_000 - i, "mob"))
        .collect();
    fs::write(&path, build_score_file(&records)).unwrap();
    let data = fs::read(&path).unwrap();
    assert_eq!(data.len(), 3 + 25 * HIGH_SCORE_RECORD_STRIDE);
    assert_eq!(decode_score_file(&data).len(), 25);

    ui_io::test_clear_getch_keys();
    ui_io::test_push_getch_keys(&[b' ' as i32; 5]);
    show_scores_screen();

    let messages = ui_io::test_ui_messages();
    let score_lines: Vec<_> = messages
        .iter()
        .filter(|line| {
            !line.starts_with("Rank")
                && !line.contains("press any key")
                && line.len() > 30
        })
        .collect();
    assert!(score_lines.len() >= 25);
    assert!(messages.iter().any(|m| m == SHOW_SCORES_HEADER));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_show_scores_bad_version_message() {
    setup_harness();
    let dir = temp_scores_dir("showbad");
    let path = install_scores_path(&dir);
    fs::write(&path, [1, 0, 0]).unwrap();

    show_scores_screen();

    let messages = ui_io::test_ui_messages();
    assert!(messages
        .iter()
        .any(|m| m.contains("different version of umoria")));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_show_scores_empty_file_no_crash() {
    setup_harness();
    let dir = temp_scores_dir("showempty");
    let path = install_scores_path(&dir);
    fs::write(&path, []).unwrap();

    show_scores_screen();
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_show_scores_rank_increments_across_pages() {
    setup_harness();
    let dir = temp_scores_dir("rankpage");
    let path = install_scores_path(&dir);
    let records: Vec<_> = (0..25)
        .map(|i| sample_score(2_000 - i, "x"))
        .collect();
    fs::write(&path, build_score_file(&records)).unwrap();

    ui_io::test_clear_getch_keys();
    ui_io::test_push_getch_keys(&[b' ' as i32; 5]);
    show_scores_screen();

    let messages = ui_io::test_ui_messages();
    assert!(messages.iter().any(|m| m.starts_with("21  ")));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_new_entry_fields_match_player_state() {
    setup_harness();
    setup_player("an orc");
    let entry = test_build_new_high_score_entry();
    assert_eq!(entry.points, player_calculate_total_points());
    assert_eq!(
        std::str::from_utf8(&entry.died_from)
            .unwrap()
            .trim_end_matches('\0'),
        "orc"
    );
}
