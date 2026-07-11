//! High-score file I/O and display

use std::cell::{Cell, RefCell};
use std::fs::{File, OpenOptions};
use std::io::{self, SeekFrom};
use std::path::{Path, PathBuf};

use crate::config::files;
use crate::data_player::{CHARACTER_RACES, CLASSES};
use crate::game::{valid_game_version, with_state};
use crate::game_save::{
    fileptr_seek, fileptr_tell, putc_raw, read_high_score, save_high_score, score_getc,
    set_c_getc_eof_mode, set_fileptr, take_fileptr, HighScore, HIGH_SCORE_RECORD_SIZE,
};
use crate::player::{player_is_male, PLAYER_NAME_SIZE};
use crate::store_inventory::store_item_value_for_state;
use crate::types::CNIL;
use crate::ui_io::{self, terminal, ESCAPE};
use crate::version::{CURRENT_VERSION_MAJOR, CURRENT_VERSION_MINOR, CURRENT_VERSION_PATCH};

/// `MAX_HIGH_SCORE_ENTRIES`
pub const MAX_HIGH_SCORE_ENTRIES: u16 = 1000;

/// 73 bytes
pub const HIGH_SCORE_RECORD_STRIDE: usize = HIGH_SCORE_RECORD_SIZE;

pub use crate::game_save::HighScore as HighScore_t;

thread_local! {
    static HIGHSCORE_FP: RefCell<Option<File>> = const { RefCell::new(None) };
    static TEST_SCORES_PATH: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static TEST_CHARACTER_SAVED_AT_RECORD: Cell<Option<bool>> = const { Cell::new(None) };
}

pub fn highscore_fp_is_none() -> bool {
    HIGHSCORE_FP.with(|fp| fp.borrow().is_none())
}

#[doc(hidden)]
pub fn test_reset_score_test_hooks() {
    TEST_CHARACTER_SAVED_AT_RECORD.with(|c| c.set(None));
}

#[doc(hidden)]
pub fn test_character_saved_at_record() -> Option<bool> {
    TEST_CHARACTER_SAVED_AT_RECORD.with(std::cell::Cell::get)
}

fn scores_path() -> String {
    TEST_SCORES_PATH.with(|path| {
        path.borrow().as_ref().map_or_else(
            || files::scores.to_string(),
            |p| p.to_string_lossy().into_owned(),
        )
    })
}

#[doc(hidden)]
pub fn test_set_scores_path(path: Option<&Path>) {
    TEST_SCORES_PATH.with(|slot| *slot.borrow_mut() = path.map(Path::to_path_buf));
}

fn scores_feof() -> bool {
    ui_io::eof_flag() != 0
}

fn scores_clear_eof() {
    ui_io::test_set_eof_flag(0);
}

fn close_score_file() {
    let _ = take_fileptr();
    HIGHSCORE_FP.with(|fp| *fp.borrow_mut() = None);
    set_c_getc_eof_mode(false);
    scores_clear_eof();
}

pub fn high_score_gender_label() -> u8 {
    if player_is_male() {
        b'M'
    } else {
        b'F'
    }
}

pub fn player_calculate_total_points_for_state(state: &crate::game::State) -> i32 {
    let mut total = state
        .py
        .misc
        .max_exp
        .wrapping_add(100i32.wrapping_mul(i32::from(state.py.misc.max_dungeon_depth)));
    total = total.wrapping_add(state.py.misc.au / 100);

    for item in &state.py.inventory {
        total = total.wrapping_add(store_item_value_for_state(state, item));
    }

    total = total.wrapping_add(i32::from(state.dg.current_level).wrapping_mul(50));

    if state.py.max_score > total {
        state.py.max_score
    } else {
        total
    }
}

pub fn player_calculate_total_points() -> i32 {
    with_state(player_calculate_total_points_for_state)
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

fn c_string(bytes: &[u8]) -> &str {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..end]).unwrap_or("")
}

fn c_isspace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Strip leading article/space from a death cause before writing the score record.
pub fn strip_died_from_for_high_score(src: &[u8]) -> [u8; 25] {
    let mut tmp = 0usize;
    if src.first() == Some(&b'a') {
        tmp = 1;
        if src.get(1) == Some(&b'n') {
            tmp = 2;
        }
        while src.get(tmp).is_some_and(|&b| c_isspace(b)) {
            tmp += 1;
        }
    }
    let mut out = [0u8; 25];
    copy_cstr(&mut out, c_string(&src[tmp..]));
    out
}

fn build_new_high_score_entry() -> HighScore {
    with_state(|state| {
        let mut name = [0u8; PLAYER_NAME_SIZE as usize];
        copy_cstr(&mut name, c_string(&state.py.misc.name));
        HighScore {
            points: player_calculate_total_points_for_state(state),
            birth_date: state.py.misc.date_of_birth,
            uid: 0,
            mhp: state.py.misc.max_hp,
            chp: state.py.misc.current_hp,
            dungeon_depth: state.dg.current_level as u8,
            level: state.py.misc.level as u8,
            deepest_dungeon_depth: state.py.misc.max_dungeon_depth as u8,
            gender: high_score_gender_label(),
            race: state.py.misc.race_id,
            character_class: state.py.misc.class_id,
            name,
            died_from: strip_died_from_for_high_score(&state.game.character_died_from),
        }
    })
}

fn high_scores_duplicate(new_entry: &HighScore, old_entry: &HighScore) -> bool {
    ((new_entry.uid != 0 && new_entry.uid == old_entry.uid)
        || (new_entry.uid == 0
            && c_string(&old_entry.died_from) == "(saved)"
            && new_entry.birth_date == old_entry.birth_date))
        && new_entry.gender == old_entry.gender
        && new_entry.race == old_entry.race
        && new_entry.character_class == old_entry.character_class
}

fn open_score_file_read_write() -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(scores_path())
}

/// opens `"rb+"` while setuid
pub fn initialize_score_file() -> bool {
    if let Ok(file) = open_score_file_read_write() {
        HIGHSCORE_FP.with(|fp| *fp.borrow_mut() = Some(file));
        true
    } else {
        HIGHSCORE_FP.with(|fp| *fp.borrow_mut() = None);
        false
    }
}

#[doc(hidden)]
pub fn test_reset_highscore_fp() {
    HIGHSCORE_FP.with(|fp| *fp.borrow_mut() = None);
    TEST_SCORES_PATH.with(|path| *path.borrow_mut() = None);
}

fn open_score_file_read() -> io::Result<File> {
    OpenOptions::new().read(true).open(scores_path())
}

fn install_score_file(file: File) {
    HIGHSCORE_FP.with(|fp| *fp.borrow_mut() = None);
    set_c_getc_eof_mode(true);
    scores_clear_eof();
    set_fileptr(file);
}

pub fn record_new_high_score() {
    TEST_CHARACTER_SAVED_AT_RECORD.with(|c| {
        c.set(Some(with_state(|state| state.game.character_saved)));
    });
    terminal::clear_screen();

    if with_state(|state| state.game.noscore != 0) {
        return;
    }

    if ui_io::panic_save() {
        terminal::print_message(Some(
            "Sorry, scores for games restored from panic save files are not saved.",
        ));
        return;
    }

    let new_entry = build_new_high_score_entry();

    let Ok(file) = open_score_file_read_write() else {
        let path = scores_path();
        terminal::print_message(Some(&format!("Error opening score file '{path}'.")));
        terminal::print_message(CNIL);
        return;
    };

    install_score_file(file);

    let _ = fileptr_seek(SeekFrom::Start(0));

    let version_maj = score_getc();
    let version_min = score_getc();
    let patch_level = score_getc();

    if scores_feof() {
        let _ = fileptr_seek(SeekFrom::Start(0));
        let _ = putc_raw(CURRENT_VERSION_MAJOR);
        let _ = putc_raw(CURRENT_VERSION_MINOR);
        let _ = putc_raw(CURRENT_VERSION_PATCH);
        let _ = fileptr_seek(SeekFrom::Current(0));
    } else if !valid_game_version(version_maj, version_min, patch_level) {
        close_score_file();
        return;
    }

    let mut old_entry = HighScore::default();

    let mut i = 0i32;
    let mut curpos = fileptr_tell().unwrap_or(3);
    let _ = read_high_score(&mut old_entry);

    while !scores_feof() {
        if new_entry.points >= old_entry.points {
            break;
        }

        if high_scores_duplicate(&new_entry, &old_entry) {
            close_score_file();
            return;
        }

        i += 1;
        if i >= i32::from(MAX_HIGH_SCORE_ENTRIES) {
            close_score_file();
            return;
        }

        curpos = fileptr_tell().unwrap_or(curpos);
        let _ = read_high_score(&mut old_entry);
    }

    if scores_feof() {
        let _ = fileptr_seek(SeekFrom::Start(curpos));
        let _ = save_high_score(&new_entry);
    } else {
        let mut entry = new_entry;

        while !scores_feof() {
            let _ = fileptr_seek(SeekFrom::Current(-(HIGH_SCORE_RECORD_STRIDE as i64)));
            let _ = save_high_score(&entry);

            if high_scores_duplicate(&new_entry, &old_entry) {
                break;
            }

            entry = old_entry;
            let _ = fileptr_seek(SeekFrom::Current(0));
            curpos = fileptr_tell().unwrap_or(curpos);
            let _ = read_high_score(&mut old_entry);
        }

        if scores_feof() {
            let _ = fileptr_seek(SeekFrom::Start(curpos));
            let _ = save_high_score(&entry);
        }
    }

    close_score_file();
}

pub const SHOW_SCORES_HEADER: &str =
    "Rank  Points Name              Sex Race       Class  Lvl Killed By";

fn format_left_int(value: i32, width: usize) -> String {
    let text = value.to_string();
    if text.len() >= width {
        text
    } else {
        format!("{text:<width$}")
    }
}

fn format_right_int(value: i32, width: usize) -> String {
    format!("{value:>width$}")
}

fn format_left_str(value: &str, width: usize, precision: usize) -> String {
    let truncated: String = value.chars().take(precision).collect();
    if truncated.len() >= width {
        truncated
    } else {
        format!("{truncated:<width$}")
    }
}

/// Format one high-score row for display (100-byte cap).
pub fn format_show_scores_line(rank: i32, score: &HighScore) -> String {
    let race_name = CHARACTER_RACES
        .get(score.race as usize)
        .map_or("", |race| race.name);
    let class_title = CLASSES
        .get(score.character_class as usize)
        .map_or("", |class| class.title);

    let mut msg = format!(
        "{}{} {} {} {} {}{} {}",
        format_left_int(rank, 4),
        format_right_int(score.points, 8),
        format_left_str(c_string(&score.name), 19, 19),
        score.gender as char,
        format_left_str(race_name, 10, 10),
        format_left_str(class_title, 7, 7),
        format_right_int(i32::from(score.level), 3),
        format_left_str(c_string(&score.died_from), 22, 22),
    );

    if msg.len() >= 100 {
        msg.truncate(99);
    }
    msg
}

pub fn show_scores_screen() {
    let Ok(file) = open_score_file_read() else {
        let path = scores_path();
        terminal::print_message(Some(&format!("Error opening score file '{path}'.")));
        terminal::print_message(CNIL);
        return;
    };

    install_score_file(file);
    let _ = fileptr_seek(SeekFrom::Start(0));

    let version_maj = score_getc();
    let version_min = score_getc();
    let patch_level = score_getc();

    if !scores_feof() && !valid_game_version(version_maj, version_min, patch_level) {
        terminal::print_message(Some(
            "Sorry. This score file is from a different version of umoria.",
        ));
        terminal::print_message(CNIL);
        close_score_file();
        return;
    }

    let mut score = HighScore::default();
    let _ = read_high_score(&mut score);

    let mut rank = 1;

    while !scores_feof() {
        let mut i = 1;
        terminal::clear_screen();

        while !scores_feof() && i < 21 {
            let msg = format_show_scores_line(rank, &score);
            i += 1;
            terminal::put_string_clear_to_eol(&msg, terminal::Coord { y: i, x: 0 });
            rank += 1;
            let _ = read_high_score(&mut score);
        }

        terminal::put_string_clear_to_eol(SHOW_SCORES_HEADER, terminal::Coord { y: 0, x: 0 });
        terminal::erase_line(terminal::Coord { y: 1, x: 0 });
        terminal::put_string_clear_to_eol(
            "[ press any key to continue ]",
            terminal::Coord { y: 23, x: 23 },
        );

        if terminal::get_key_input() == ESCAPE {
            break;
        }
    }

    close_score_file();
}

#[doc(hidden)]
pub fn test_build_new_high_score_entry() -> HighScore {
    build_new_high_score_entry()
}
