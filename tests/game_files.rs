//! `game_files`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use umoria::data_treasure::GAME_OBJECTS;
use umoria::game::{reset_for_new_game, with_state, with_state_mut};
use umoria::game_files::{
    compute_derived_abilities_for_test, display_death_file, display_splash_screen,
    display_text_help_file, equipment_placement_description, fgets, initialize_score_file,
    output_player_character_to_file, output_random_level_objects_to_file,
};
use umoria::inventory::{Inventory, PlayerEquipment};
use umoria::player::{player_initialize_base_experience_levels, PlayerAttr, PLAYER_MAX_LEVEL};
use umoria::scores::{highscore_fp_is_none, test_reset_highscore_fp, test_set_scores_path};
use umoria::types::{MAX_DUNGEON_OBJECTS, TREASURE_MAX_LEVELS};
use umoria::ui_io::{
    ctrl_key, test_clear_getch_keys, test_push_getch_keys, test_set_eof_flag,
    test_set_ncurses_stub, test_set_select_ready, test_set_ui_capture, test_set_ui_trace,
    test_ui_messages, test_ui_trace_events, UiTraceEvent, ESCAPE,
};

/// Splash tests `chdir`; serialize against any CWD-relative file I/O in this suite.
static CWD_LOCK: Mutex<()> = Mutex::new(());

fn init_treasure_levels() {
    with_state_mut(|state| {
        state.treasure_levels = [0; TREASURE_MAX_LEVELS as usize + 1];
        for i in 0..MAX_DUNGEON_OBJECTS as usize {
            let level = GAME_OBJECTS[i].depth_first_found as usize;
            state.treasure_levels[level] += 1;
        }
        for i in 1..=TREASURE_MAX_LEVELS as usize {
            state.treasure_levels[i] += state.treasure_levels[i - 1];
        }

        let mut indexes = [1i16; TREASURE_MAX_LEVELS as usize + 1];
        for i in 0..MAX_DUNGEON_OBJECTS as usize {
            let level = GAME_OBJECTS[i].depth_first_found as usize;
            let object_id = state.treasure_levels[level] - indexes[level];
            state.sorted_objects[object_id as usize] = i as i16;
            indexes[level] += 1;
        }
    });
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/game_files")
}

fn push_keys_in_consume_order(keys: &[i32]) {
    let mut reversed = keys.to_vec();
    reversed.reverse();
    test_push_getch_keys(&reversed);
}

fn push_string_input(s: &str) {
    let mut keys: Vec<i32> = s.bytes().map(i32::from).collect();
    keys.push(i32::from(ctrl_key(b'J')));
    push_keys_in_consume_order(&keys);
}

fn push_confirm_yes() {
    push_keys_in_consume_order(&[i32::from(b'y')]);
}

fn push_confirm_no() {
    push_keys_in_consume_order(&[i32::from(b'n')]);
}

fn push_any_key() {
    push_keys_in_consume_order(&[i32::from(b' ')]); // space
}

fn setup_ui_harness() {
    // Full TLS reset: cargo reuses worker threads across tests.
    test_set_ncurses_stub(true);
    test_set_eof_flag(0);
    test_set_select_ready(None);
    test_set_ui_trace(true);
    test_set_ui_capture(true);
    test_clear_getch_keys();
    let _ = test_ui_trace_events();
    let _ = test_ui_messages();
    reset_for_new_game(None);
}

fn put_strings(trace: &[UiTraceEvent]) -> Vec<(String, i32, i32)> {
    trace
        .iter()
        .filter_map(|event| match event {
            UiTraceEvent::PutString { text, y, x } => Some((text.clone(), *y, *x)),
            _ => None,
        })
        .collect()
}

fn count_events(trace: &[UiTraceEvent], pred: impl Fn(&UiTraceEvent) -> bool) -> usize {
    trace.iter().filter(|e| pred(e)).count()
}

fn setup_player_for_character_output() {
    player_initialize_base_experience_levels();
    with_state_mut(|s| {
        s.py.misc.level = 1;
        s.py.misc.experience_factor = 100;
    });
}

fn write_name(out: &mut [u8], name: &str) {
    for (index, byte) in name.bytes().enumerate() {
        if index < out.len() {
            out[index] = byte;
        }
    }
}

// ---------------------------------------------------------------------------
// Step 1 — fgets + text readers + initializeScoreFile
// ---------------------------------------------------------------------------

#[test]
fn fgets_parity_79_and_78_byte_caps() {
    let path = fixture_root().join("fgets_fixture.txt");
    let mut file = std::fs::File::open(&path).expect("fixture");

    let mut buf79 = [0u8; 80];
    assert!(fgets(&mut buf79, 80, &mut file));
    assert_eq!(&buf79[..6], b"short\n");

    let mut buf78 = [0u8; 80];
    assert!(fgets(&mut buf78, 79, &mut file));
    let end = buf78.iter().position(|&b| b == 0).unwrap();
    let line = &buf78[..end];
    assert_eq!(line.len(), 78);
    assert!(!line.contains(&b'\n'));

    assert!(fgets(&mut buf78, 79, &mut file));
    let end = buf78.iter().position(|&b| b == 0).unwrap();
    assert_eq!(&buf78[..end], b"no-newline-at-eof\n");

    assert!(fgets(&mut buf78, 79, &mut file));
    let end = buf78.iter().position(|&b| b == 0).unwrap();
    assert_eq!(&buf78[..end], b"\n");

    assert!(fgets(&mut buf78, 79, &mut file));
    let end = buf78.iter().position(|&b| b == 0).unwrap();
    assert_eq!(&buf78[..end], b"trailing-empty\n");

    assert!(!fgets(&mut buf78, 79, &mut file));
}

#[test]
fn display_splash_screen_golden_trace() {
    let _cwd = CWD_LOCK.lock().unwrap();
    let temp = tempfile_dir("splash");
    fs::create_dir_all(temp.join("data")).expect("data dir");
    fs::copy(
        fixture_root().join("splash.txt"),
        temp.join("data/splash.txt"),
    )
    .expect("copy splash");
    let prev = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&temp).expect("chdir");

    setup_ui_harness();
    push_any_key();
    display_splash_screen();

    let trace = test_ui_trace_events();
    assert_eq!(
        count_events(&trace, |e| matches!(e, UiTraceEvent::ClearScreen)),
        1
    );
    assert_eq!(
        put_strings(&trace)
            .into_iter()
            .filter(|(_, y, _)| *y < 23)
            .collect::<Vec<_>>(),
        vec![
            ("Line one\n".to_string(), 0, 0),
            ("Line two\n".to_string(), 1, 0),
        ]
    );
    assert_eq!(
        count_events(&trace, |e| matches!(
            e,
            UiTraceEvent::WaitForContinueKey { line: 23 }
        )),
        1
    );

    std::env::set_current_dir(prev).expect("restore cwd");
    let _ = fs::remove_dir_all(temp);
}

#[test]
fn display_splash_screen_missing_file_is_noop() {
    let _cwd = CWD_LOCK.lock().unwrap();
    let temp = tempfile_dir("splash-missing");
    fs::create_dir_all(temp.join("data")).expect("data dir");
    let prev = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&temp).expect("chdir");

    setup_ui_harness();
    display_splash_screen();
    let trace = test_ui_trace_events();
    assert_eq!(
        count_events(&trace, |e| matches!(e, UiTraceEvent::ClearScreen)),
        0
    );

    std::env::set_current_dir(prev).expect("restore cwd");
    let _ = fs::remove_dir_all(temp);
}

#[test]
fn display_text_help_file_pagination_and_escape() {
    let lines: Vec<String> = (0..60).map(|i| format!("help line {i}\n")).collect();
    let path = write_temp_text("help-60.txt", &lines.join(""));

    setup_ui_harness();
    push_keys_in_consume_order(&[i32::from(b' '), i32::from(b' '), i32::from(ESCAPE)]);

    display_text_help_file(path.to_str().unwrap());
    let trace = test_ui_trace_events();

    assert_eq!(
        count_events(&trace, |e| matches!(e, UiTraceEvent::TerminalSaveScreen)),
        1
    );
    assert_eq!(
        count_events(&trace, |e| matches!(e, UiTraceEvent::TerminalRestoreScreen)),
        1
    );
    assert_eq!(
        count_events(&trace, |e| matches!(e, UiTraceEvent::ClearScreen)),
        3
    );
    assert_eq!(
        count_events(&trace, |e| matches!(
            e,
            UiTraceEvent::PutStringClearToEol { y: 23, x: 23, .. }
        )),
        3
    );

    let _ = fs::remove_file(path);
}

#[test]
fn display_text_help_file_missing_file_message() {
    setup_ui_harness();
    display_text_help_file("no-such-help-file-xyz");
    let messages = test_ui_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0],
        "Can not find help file 'no-such-help-file-xyz'."
    );
    let trace = test_ui_trace_events();
    assert!(!trace
        .iter()
        .any(|e| matches!(e, UiTraceEvent::TerminalSaveScreen)));
}

#[test]
fn display_death_file_caps_at_23_lines() {
    let lines: Vec<String> = (0..30).map(|i| format!("death {i}\n")).collect();
    let path = write_temp_text("death-30.txt", &lines.join(""));

    setup_ui_harness();
    display_death_file(path.to_str().unwrap());
    let trace = test_ui_trace_events();

    assert_eq!(
        count_events(&trace, |e| matches!(e, UiTraceEvent::ClearScreen)),
        1
    );
    assert_eq!(put_strings(&trace).len(), 23);
    assert!(!trace
        .iter()
        .any(|e| matches!(e, UiTraceEvent::TerminalSaveScreen)));

    let _ = fs::remove_file(path);
}

#[test]
fn initialize_score_file_opens_rb_plus() {
    test_reset_highscore_fp();
    let path = write_temp_binary("scores.dat", &[1, 2, 3]);
    test_set_scores_path(Some(&path));
    assert!(initialize_score_file());
    assert!(!highscore_fp_is_none());
    test_reset_highscore_fp();
    assert!(highscore_fp_is_none());

    test_set_scores_path(Some(Path::new("/no/such/scores/path.dat")));
    assert!(!initialize_score_file());
    assert!(highscore_fp_is_none());
    test_reset_highscore_fp();
}

// ---------------------------------------------------------------------------
// Step 2 — wizard random-object dump
// ---------------------------------------------------------------------------

#[test]
fn output_random_level_objects_prompt_flow_and_bounds() {
    setup_ui_harness();
    push_keys_in_consume_order(&[i32::from(ESCAPE)]);
    output_random_level_objects_to_file();
    assert!(test_ui_messages()
        .iter()
        .any(|m| m.contains("Produce objects on what level")));

    setup_ui_harness();
    push_string_input("abc");
    output_random_level_objects_to_file();

    setup_ui_harness();
    push_string_input("5");
    push_keys_in_consume_order(&[i32::from(ESCAPE)]);
    output_random_level_objects_to_file();

    setup_ui_harness();
    push_string_input("5");
    push_string_input("xy");
    output_random_level_objects_to_file();

    setup_ui_harness();
    push_string_input("0");
    push_string_input("5");
    output_random_level_objects_to_file();
    assert!(test_ui_messages()
        .iter()
        .any(|m| m == "Parameters no good."));
}

#[test]
fn output_random_level_objects_writes_header_and_completed() {
    // Relative filename + splash tests' chdir must not race.
    let _cwd = CWD_LOCK.lock().unwrap();
    let out = PathBuf::from("w5.txt");
    let _ = fs::remove_file(&out);

    setup_ui_harness();
    reset_for_new_game(Some(12345));
    init_treasure_levels();
    test_clear_getch_keys();
    // LIFO: last push consumed first → level, count, small?, filename.
    push_string_input(out.to_str().unwrap());
    push_confirm_no();
    push_string_input("2");
    push_string_input("10");

    output_random_level_objects_to_file();

    let messages = test_ui_messages();
    let body = fs::read_to_string(&out).unwrap_or_else(|e| {
        panic!(
            "output file missing ({e}); ui messages={messages:?}; path={}",
            out.display()
        );
    });
    assert!(body.starts_with("*** Random Object Sampling:\n"));
    assert!(body.contains("*** 2 objects\n"));
    assert!(body.contains("*** For Level 10\n"));
    assert!(
        messages.iter().any(|m| m == "Completed."),
        "messages={messages:?}"
    );
    let _ = fs::remove_file(out);
}

// ---------------------------------------------------------------------------
// Step 3 — character sheet writer
// ---------------------------------------------------------------------------

#[test]
fn character_sheet_header_form_feed_and_exp_to_adv() {
    setup_ui_harness();
    setup_player_for_character_output();
    with_state_mut(|s| {
        write_name(&mut s.py.misc.name, "TestHero");
        s.py.misc.race_id = 0;
        s.py.misc.class_id = 0;
        s.py.misc.level = 5;
        s.py.misc.experience_factor = 100;
        s.py.misc.age = 20;
        s.py.misc.height = 72;
        s.py.misc.weight = 180;
        s.py.stats.used = [16, 14, 12, 15, 13, 10];
        s.py.misc.history[0][..11].copy_from_slice(b"Born brave\0".as_ref());
    });

    let path = tempfile_dir("sheet").join("hero.chr");
    let _ = fs::remove_file(&path);
    assert!(output_player_character_to_file(path.to_str().unwrap()));

    let bytes = fs::read(&path).expect("sheet bytes");
    assert_eq!(bytes[0], ctrl_key(b'L'));
    assert_eq!(&bytes[1..3], b"\n\n");
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("Exp to Adv :"));
    assert!(!text.contains("Writing character sheet...")); // UI message, not file
    assert!(text.contains("Born brave"));

    with_state_mut(|s| s.py.misc.level = u16::from(PLAYER_MAX_LEVEL));
    let path2 = tempfile_dir("sheet-max").join("hero-max.chr");
    let _ = fs::remove_file(&path2);
    assert!(output_player_character_to_file(path2.to_str().unwrap()));
    let max_text = fs::read_to_string(&path2).expect("max sheet");
    assert!(max_text.contains("Exp to Adv : *******"));

    let _ = fs::remove_file(path);
    let _ = fs::remove_file(path2);
}

#[test]
fn derived_ability_arithmetic_matches_cpp_truncation() {
    player_initialize_base_experience_levels();
    with_state_mut(|s| {
        s.py.misc.class_id = 0;
        s.py.misc.level = 9;
        s.py.misc.bth = 10;
        s.py.misc.bth_with_bows = 8;
        s.py.misc.plusses_to_hit = 2;
        s.py.misc.fos = 45;
        s.py.misc.chance_in_search = 5;
        s.py.misc.stealth_factor = 3;
        s.py.misc.disarm = 4;
        s.py.misc.saving_throw = 6;
        s.py.stats.used[PlayerAttr::A_DEX as usize] = 14;
        s.py.stats.used[PlayerAttr::A_INT as usize] = 12;
        s.py.stats.used[PlayerAttr::A_WIS as usize] = 10;
        s.py.flags.see_infra = 2;
    });

    let derived = with_state(compute_derived_abilities_for_test);
    assert_eq!(derived.perception.x, 0); // 40 - 45 clamped
    assert_eq!(derived.stealth.x, 4); // stealth_factor + 1
    assert_eq!(derived.infra, "20 feet");
}

// ---------------------------------------------------------------------------
// Step 4 — equipment & inventory writers
// ---------------------------------------------------------------------------

#[test]
fn equipment_placement_description_literals() {
    assert_eq!(
        equipment_placement_description(PlayerEquipment::Left as i32),
        "Left  ring finger"
    );
    assert_eq!(
        equipment_placement_description(PlayerEquipment::Wield as i32),
        "You are wielding"
    );
    assert_eq!(equipment_placement_description(999), "*Unknown value*");
}

#[test]
fn equipment_and_inventory_sections_in_output_file() {
    setup_ui_harness();
    setup_player_for_character_output();
    with_state_mut(|s| {
        s.py.equipment_count = 0;
        s.py.pack.unique_items = 0;
    });

    let empty_path = tempfile_dir("empty-chr").join("empty.chr");
    let _ = fs::remove_file(&empty_path);
    assert!(output_player_character_to_file(
        empty_path.to_str().unwrap()
    ));
    let empty = fs::read_to_string(&empty_path).expect("empty chr");
    assert!(empty.contains("\n  [Character's Equipment List]\n\n"));
    assert!(empty.contains("  Character has no equipment in use.\n"));
    assert!(empty.contains("  [General Inventory List]\n\n"));
    assert!(empty.contains("  Character has no objects in inventory.\n"));

    with_state_mut(|s| {
        s.py.equipment_count = 1;
        s.py.inventory[PlayerEquipment::Wield as usize] = Inventory {
            category_id: umoria::treasure::TV_SWORD,
            ..Inventory::default()
        };
        s.py.pack.unique_items = 1;
        s.py.inventory[0] = Inventory {
            category_id: umoria::treasure::TV_FOOD,
            ..Inventory::default()
        };
    });

    let full_path = tempfile_dir("full-chr").join("full.chr");
    let _ = fs::remove_file(&full_path);
    assert!(output_player_character_to_file(full_path.to_str().unwrap()));
    let full = fs::read(&full_path).expect("full chr bytes");
    let full_text = String::from_utf8_lossy(&full);
    assert!(full_text.contains("  a) You are wielding"));
    assert!(full_text.contains("  a) "));
    assert_eq!(full.last().copied(), Some(ctrl_key(b'L')));

    let _ = fs::remove_file(empty_path);
    let _ = fs::remove_file(full_path);
}

// ---------------------------------------------------------------------------
// Step 5 — outputPlayerCharacterToFile open/replace logic
// ---------------------------------------------------------------------------

#[test]
fn output_player_character_new_and_replace_paths() {
    let path = tempfile_dir("replace").join("player.chr");
    let _ = fs::remove_file(&path);

    setup_ui_harness();
    setup_player_for_character_output();
    assert!(output_player_character_to_file(path.to_str().unwrap()));
    assert!(path.is_file());

    setup_ui_harness();
    setup_player_for_character_output();
    push_confirm_no();
    assert!(!output_player_character_to_file(path.to_str().unwrap()));
    assert!(test_ui_messages()
        .iter()
        .any(|m| m.starts_with("Can't open file")));

    setup_ui_harness();
    setup_player_for_character_output();
    push_confirm_yes();
    assert!(output_player_character_to_file(path.to_str().unwrap()));
    assert!(test_ui_messages().iter().any(|m| m == "Completed."));

    let _ = fs::remove_file(path);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn tempfile_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("umoria-phase-5-3-{name}"));
    let _ = fs::create_dir_all(&path);
    path
}

fn write_temp_text(name: &str, body: &str) -> PathBuf {
    let path = tempfile_dir("text").join(name);
    let _ = fs::create_dir_all(path.parent().unwrap());
    fs::write(&path, body).expect("write temp");
    path
}

fn write_temp_binary(name: &str, body: &[u8]) -> PathBuf {
    let path = tempfile_dir("bin").join(name);
    let _ = fs::create_dir_all(path.parent().unwrap());
    fs::write(&path, body).expect("write temp bin");
    path
}
