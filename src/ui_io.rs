//! Transient terminal I/O statics (not saved)

use std::cell::{Cell, RefCell};
use std::io::{self, Write};
use std::ptr;

use ncurses::ll::chtype;
use ncurses::{self, WINDOW};

use crate::game::{self, with_state_mut, GameUiHooks};
#[cfg(windows)]
use crate::player::PLAYER_NAME_SIZE;
use crate::types::Coord_t;
use crate::types::{Vtype_t, MESSAGE_HISTORY_SIZE, MORIA_MESSAGE_SIZE};

pub const MSG_LINE: i32 = 0;

#[must_use]
pub const fn ctrl_key(x: u8) -> u8 {
    x & 0x1f
}

pub const DELETE: u8 = 0x7f;

/// ASCII escape (`0x1B`).
pub const ESCAPE: u8 = 0o33;

const MORE_THRESHOLD: i32 = 73;
const SCREEN_LAST_COL: i32 = 79;
const CONFIRM_PROMPT_COL: i32 = 73;
const TILDE_USER_MAX: usize = 127;
const TILDE_EXPANDED_MAX: usize = 1024;

/// Screen I/O events recorded when UI trace mode is enabled ( tests)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiTraceEvent {
    ClearScreen,
    PutString { text: String, y: i32, x: i32 },
    PutStringClearToEol { text: String, y: i32, x: i32 },
    TerminalSaveScreen,
    TerminalRestoreScreen,
    WaitForContinueKey { line: i32 },
    PutQio,
}

thread_local! {
    static CURSES_ON: Cell<bool> = const { Cell::new(false) };
    static EOF_FLAG: Cell<i32> = const { Cell::new(0) };
    static PANIC_SAVE: Cell<bool> = const { Cell::new(false) };
    static TEST_UI_MESSAGES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static TEST_UI_CAPTURE: Cell<bool> = const { Cell::new(false) };
    static TEST_UI_TRACE: RefCell<Vec<UiTraceEvent>> = const { RefCell::new(Vec::new()) };
    static TEST_UI_TRACE_ENABLED: Cell<bool> = const { Cell::new(false) };
    static SAVE_SCREEN: RefCell<Option<WINDOW>> = const { RefCell::new(None) };
    static TEST_NCURSES_STUB: Cell<bool> = const { Cell::new(false) };
    static TEST_GETCH_KEYS: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
    static TEST_SELECT_READY: Cell<Option<bool>> = const { Cell::new(None) };
    static TEST_LOGIN_NAME: RefCell<Option<String>> = const { RefCell::new(None) };
    static TEST_FORCE_DEFAULT_NAME: Cell<bool> = const { Cell::new(false) };
    static TEST_UI_DETAIL: Cell<bool> = const { Cell::new(false) };
    static TEST_MOVE_CURSORS: RefCell<Vec<terminal::Coord>> = const { RefCell::new(Vec::new()) };
    static TEST_PUT_STRINGS: RefCell<Vec<(i32, i32, String)>> = const { RefCell::new(Vec::new()) };
    static TEST_BELL_COUNT: Cell<u32> = const { Cell::new(0) };
    static TEST_FLUSH_COUNT: Cell<u32> = const { Cell::new(0) };
    static TEST_ERASE_LINES: RefCell<Vec<(i32, i32)>> = const { RefCell::new(Vec::new()) };
    static TEST_WAIT_CONTINUE_LINES: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
}

pub fn curses_on() -> bool {
    CURSES_ON.with(std::cell::Cell::get)
}

pub fn eof_flag() -> i32 {
    EOF_FLAG.with(std::cell::Cell::get)
}

pub fn panic_save() -> bool {
    PANIC_SAVE.with(std::cell::Cell::get)
}

#[doc(hidden)]
pub fn test_set_eof_flag(value: i32) {
    EOF_FLAG.with(|c| c.set(value));
}

#[doc(hidden)]
pub fn test_set_panic_save(on: bool) {
    PANIC_SAVE.with(|c| c.set(on));
}

#[doc(hidden)]
pub fn test_set_ui_capture(enabled: bool) {
    TEST_UI_CAPTURE.with(|c| c.set(enabled));
    if enabled {
        TEST_UI_MESSAGES.with(|m| m.borrow_mut().clear());
        TEST_PUT_STRINGS.with(|m| m.borrow_mut().clear());
        TEST_FLUSH_COUNT.with(|c| c.set(0));
        TEST_ERASE_LINES.with(|m| m.borrow_mut().clear());
        TEST_WAIT_CONTINUE_LINES.with(|m| m.borrow_mut().clear());
    }
}

#[doc(hidden)]
pub fn test_ui_messages() -> Vec<String> {
    TEST_UI_MESSAGES.with(|m| std::mem::take(&mut *m.borrow_mut()))
}

#[doc(hidden)]
pub fn test_ui_messages_peek() -> Vec<String> {
    TEST_UI_MESSAGES.with(|m| m.borrow().clone())
}

#[doc(hidden)]
pub fn test_put_strings() -> Vec<(i32, i32, String)> {
    TEST_PUT_STRINGS.with(|m| std::mem::take(&mut *m.borrow_mut()))
}

#[doc(hidden)]
pub fn test_put_strings_peek() -> Vec<(i32, i32, String)> {
    TEST_PUT_STRINGS.with(|m| m.borrow().clone())
}

#[doc(hidden)]
pub fn test_flush_input_buffer_count() -> u32 {
    TEST_FLUSH_COUNT.with(std::cell::Cell::get)
}

#[doc(hidden)]
pub fn test_erase_lines() -> Vec<(i32, i32)> {
    TEST_ERASE_LINES.with(|m| std::mem::take(&mut *m.borrow_mut()))
}

#[doc(hidden)]
pub fn test_wait_continue_lines() -> Vec<i32> {
    TEST_WAIT_CONTINUE_LINES.with(|m| std::mem::take(&mut *m.borrow_mut()))
}

fn capture_put_string(coord_y: i32, coord_x: i32, text: &str) {
    if TEST_UI_CAPTURE.with(std::cell::Cell::get) || TEST_UI_DETAIL.with(std::cell::Cell::get) {
        TEST_PUT_STRINGS.with(|m| {
            m.borrow_mut().push((coord_y, coord_x, text.to_owned()));
        });
    }
}

#[doc(hidden)]
pub fn test_set_ui_trace(enabled: bool) {
    TEST_UI_TRACE_ENABLED.with(|c| c.set(enabled));
    if enabled {
        TEST_UI_TRACE.with(|t| t.borrow_mut().clear());
    }
}

#[doc(hidden)]
pub fn test_ui_trace_events() -> Vec<UiTraceEvent> {
    TEST_UI_TRACE.with(|t| std::mem::take(&mut *t.borrow_mut()))
}

fn trace_ui(event: UiTraceEvent) {
    if TEST_UI_TRACE_ENABLED.with(std::cell::Cell::get) {
        TEST_UI_TRACE.with(|t| t.borrow_mut().push(event));
    }
}

fn capture_ui_message(text: &str) {
    if TEST_UI_CAPTURE.with(std::cell::Cell::get) {
        TEST_UI_MESSAGES.with(|m| m.borrow_mut().push(text.to_owned()));
    }
}

#[doc(hidden)]
pub fn test_set_ncurses_stub(enabled: bool) {
    TEST_NCURSES_STUB.with(|c| c.set(enabled));
    register_game_ui_hooks();
}

#[doc(hidden)]
pub fn test_push_getch_keys(keys: &[i32]) {
    TEST_GETCH_KEYS.with(|q| q.borrow_mut().extend_from_slice(keys));
}

#[doc(hidden)]
pub fn test_clear_getch_keys() {
    TEST_GETCH_KEYS.with(|q| q.borrow_mut().clear());
}

#[doc(hidden)]
pub fn test_set_select_ready(ready: Option<bool>) {
    TEST_SELECT_READY.with(|c| c.set(ready));
}

#[doc(hidden)]
pub fn test_set_force_default_name(force: bool) {
    TEST_FORCE_DEFAULT_NAME.with(|c| c.set(force));
}

#[doc(hidden)]
pub fn test_set_ui_detail_capture(enabled: bool) {
    TEST_UI_DETAIL.with(|c| c.set(enabled));
    if enabled {
        TEST_MOVE_CURSORS.with(|m| m.borrow_mut().clear());
        TEST_PUT_STRINGS.with(|m| m.borrow_mut().clear());
        TEST_BELL_COUNT.with(|c| c.set(0));
        TEST_FLUSH_COUNT.with(|c| c.set(0));
        TEST_ERASE_LINES.with(|m| m.borrow_mut().clear());
        TEST_WAIT_CONTINUE_LINES.with(|m| m.borrow_mut().clear());
    }
}

#[doc(hidden)]
pub fn test_move_cursors() -> Vec<terminal::Coord> {
    TEST_MOVE_CURSORS.with(|m| m.borrow().clone())
}

#[doc(hidden)]
pub fn test_bell_count() -> u32 {
    TEST_BELL_COUNT.with(std::cell::Cell::get)
}

/// Wire `game` lifecycle helpers to this module's terminal API.
pub fn register_game_ui_hooks() {
    fn put_string_hook(s: &str, coord: Coord_t) {
        terminal::put_string(
            s,
            terminal::Coord {
                y: coord.y,
                x: coord.x,
            },
        );
    }
    fn put_string_clear_to_eol_hook(s: &str, coord: Coord_t) {
        terminal::put_string_clear_to_eol(
            s,
            terminal::Coord {
                y: coord.y,
                x: coord.x,
            },
        );
    }
    fn move_cursor_hook(coord: Coord_t) {
        terminal::move_cursor(terminal::Coord {
            y: coord.y,
            x: coord.x,
        });
    }
    fn erase_line_hook(coord: Coord_t) {
        terminal::erase_line(terminal::Coord {
            y: coord.y,
            x: coord.x,
        });
    }

    game::install_game_ui_hooks(GameUiHooks {
        flush_input_buffer: terminal::flush_input_buffer,
        terminal_restore: terminal::terminal_restore,
        get_command: terminal::get_command,
        get_key_input: terminal::get_key_input,
        put_string: put_string_hook,
        put_string_clear_to_eol: put_string_clear_to_eol_hook,
        move_cursor: move_cursor_hook,
        erase_line: erase_line_hook,
        terminal_bell_sound: terminal::terminal_bell_sound,
    });
}

#[doc(hidden)]
pub fn test_set_login_name(name: Option<&str>) {
    TEST_LOGIN_NAME.with(|n| *n.borrow_mut() = name.map(str::to_owned));
}

fn record_move_cursor(coord: terminal::Coord) {
    if TEST_UI_DETAIL.with(std::cell::Cell::get) {
        TEST_MOVE_CURSORS.with(|m| m.borrow_mut().push(coord));
    }
}

fn record_bell() {
    if TEST_UI_DETAIL.with(std::cell::Cell::get) {
        TEST_BELL_COUNT.with(|c| c.set(c.get() + 1));
    }
}

fn ncurses_stubbed() -> bool {
    TEST_NCURSES_STUB.with(std::cell::Cell::get)
}

fn set_curses_on(on: bool) {
    CURSES_ON.with(|c| c.set(on));
}

fn bump_eof_flag() {
    EOF_FLAG.with(|c| c.set(c.get() + 1));
}

fn set_panic_save(on: bool) {
    PANIC_SAVE.with(|c| c.set(on));
}

fn read_getch() -> i32 {
    if let Some(key) = TEST_GETCH_KEYS.with(|q| q.borrow_mut().pop()) {
        return key;
    }
    if ncurses_stubbed() {
        return ESCAPE as i32;
    }
    ncurses::getch()
}

// ---------------------------------------------------------------------------
// Pure helpers (C-faithful; unit-tested without a TTY)
// ---------------------------------------------------------------------------

/// Truncate a string for screen output: clamp `x` to 79, copy at most `79 - x` bytes.
/// 154
pub fn truncate_for_put_string(s: &str, x: i32) -> String {
    let x = if x > SCREEN_LAST_COL {
        SCREEN_LAST_COL
    } else {
        x
    };
    let max_len = (SCREEN_LAST_COL - x) as usize;
    let take = max_len.min(s.len());
    String::from_utf8_lossy(&s.as_bytes()[..take]).into_owned()
}

/// Truncate or NUL-pad a message to 79 bytes.
pub fn cap_message_line(mut message: String) -> String {
    if message.len() > 79 {
        message.truncate(79);
    } else {
        message.reserve(79 - message.len());
        while message.len() < 79 {
            message.push('\0');
        }
    }
    message
}

/// Sign-extend a display byte to `chtype` for the standout high-bit path.
pub fn encode_tile(byte: u8) -> chtype {
    (byte as i8) as chtype
}

///  line 251
pub fn message_old_len(msg: &[u8]) -> i32 {
    vtype_strlen(msg) as i32 + 1
}

pub fn should_combine_messages(old_len: i32, new_len: i32) -> bool {
    new_len + old_len + 2 < MORE_THRESHOLD
}

/// Whether to show `-more-` when the message is absent or the combined length exceeds the threshold.
pub fn should_show_more(msg_is_none: bool, old_len: i32, new_len: i32) -> bool {
    msg_is_none || new_len + old_len + 2 >= MORE_THRESHOLD
}

pub fn clamp_more_column(old_len: i32) -> i32 {
    if old_len > MORE_THRESHOLD {
        MORE_THRESHOLD
    } else {
        old_len
    }
}

pub fn advance_message_ring_index(last_message_id: i16) -> i16 {
    let next = last_message_id + 1;
    if next >= MESSAGE_HISTORY_SIZE as i16 {
        0
    } else {
        next
    }
}

fn vtype_strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

pub fn copy_message_to_ring_slot(slot: &mut Vtype_t, msg: &str) {
    let bytes = msg.as_bytes();
    let copy_len = bytes.len().min(MORIA_MESSAGE_SIZE);
    slot[..copy_len].copy_from_slice(&bytes[..copy_len]);
    if copy_len < MORIA_MESSAGE_SIZE {
        slot[copy_len..].fill(0);
    }
    slot[MORIA_MESSAGE_SIZE - 1] = 0;
}

pub fn append_message_slot(slot: &mut Vtype_t, msg: &str) {
    c_strcat(slot, "  ");
    c_strcat(slot, msg);
}

fn c_strcat(dst: &mut Vtype_t, src: &str) {
    let base = vtype_strlen(dst);
    let bytes = src.as_bytes();
    let copy_len = bytes.len().min(MORIA_MESSAGE_SIZE - 1 - base);
    dst[base..base + copy_len].copy_from_slice(&bytes[..copy_len]);
    dst[base + copy_len] = 0;
}

pub fn panel_screen_coord(coord: terminal::Coord, row_prt: i32, col_prt: i32) -> terminal::Coord {
    terminal::Coord {
        y: coord.y - row_prt,
        x: coord.x - col_prt,
    }
}

pub fn clamp_string_input_end_col(start_col: i32, slen: i32) -> i32 {
    let end_col = start_col + slen - 1;
    if end_col > SCREEN_LAST_COL {
        SCREEN_LAST_COL
    } else {
        end_col
    }
}

/// C `isprint` for ASCII locale —  line 441
pub fn is_printable_key(key: i32) -> bool {
    (0x20..=0x7e).contains(&key)
}

pub fn trim_trailing_spaces(buf: &mut [u8]) -> usize {
    let mut len = vtype_strlen(buf);
    while len > 0 && buf[len - 1] == b' ' {
        len -= 1;
    }
    if len < buf.len() {
        buf[len] = 0;
    }
    len
}

/// Accepted keys in `-more-` loop —  line 274
pub fn more_prompt_accepts_key(key: u8) -> bool {
    key == b' ' || key == ESCAPE || key == b'\n' || key == b'\r'
}

pub fn confirmation_key_result(key: u8) -> i32 {
    if key == b'N' || key == b'n' {
        0
    } else if key == b'Y' || key == b'y' {
        1
    } else {
        -1
    }
}

fn c_strcpy(dst: &mut [u8], src: &str) {
    let bytes = src.as_bytes();
    let n = bytes.len().min(dst.len() - 1);
    dst[..n].copy_from_slice(&bytes[..n]);
    dst[n] = 0;
}

fn abort_on_err(result: i32) {
    if result == ncurses::ERR && !ncurses_stubbed() {
        std::process::abort();
    }
}

// ---------------------------------------------------------------------------
// Terminal module API surface
// ---------------------------------------------------------------------------

pub mod terminal {
    // ! Thin Rust terminal module mirroring  /  public surface

    use super::*;
    use crate::game_death::end_game;
    use crate::game_save::save_game;
    use crate::player::player_disturb;

    /// Mirrors `Coord_t { int y; int x; }`
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Coord {
        pub y: i32,
        pub x: i32,
    }

    fn current_cursor_position() -> Coord {
        if ncurses_stubbed() {
            return Coord { y: 0, x: 0 };
        }
        let mut y = 0;
        let mut x = 0;
        ncurses::getyx(ncurses::stdscr(), &mut y, &mut x);
        Coord { y, x }
    }

    fn panel_offsets() -> (i32, i32) {
        with_state_mut(|state| (state.dg.panel.row_prt, state.dg.panel.col_prt))
    }

    // --- init / teardown ---

    /// 58
    pub fn terminal_initialize() -> bool {
        if ncurses_stubbed() {
            set_curses_on(true);
            return true;
        }

        ncurses::initscr();

        if ncurses::LINES() < 24 || ncurses::COLS() < 80 {
            let _ = writeln!(io::stdout(), "Screen too small for moria.");
            return false;
        }

        let win = ncurses::newwin(0, 0, 0, 0);
        if win.is_null() {
            let _ = writeln!(io::stdout(), "Out of memory in starting up curses.");
            return false;
        }
        SAVE_SCREEN.with(|s| *s.borrow_mut() = Some(win));

        moria_terminal_initialize();

        let _ = ncurses::clear();
        let _ = ncurses::refresh();
        true
    }

    /// 34
    pub(super) fn moria_terminal_initialize() {
        register_game_ui_hooks();
        if ncurses_stubbed() {
            set_curses_on(true);
            return;
        }

        let _ = ncurses::raw();
        let _ = ncurses::noecho();
        let _ = ncurses::nonl();
        let _ = ncurses::keypad(ncurses::stdscr(), false);
        ncurses::timeout(-1);
        #[cfg(target_os = "macos")]
        {
            let _ = ncurses::set_escdelay(50);
        }
        set_curses_on(true);
    }

    /// 80
    pub fn terminal_restore() {
        if !curses_on() {
            return;
        }

        put_qio();

        if !ncurses_stubbed() {
            let mut y = 0;
            let mut x = 0;
            ncurses::getyx(ncurses::stdscr(), &mut y, &mut x);
            let _ = ncurses::mvcur(y, x, ncurses::LINES() - 1, 0);
            let _ = ncurses::endwin();
        }

        let _ = io::stdout().flush();
        set_curses_on(false);
    }

    /// 84
    pub fn terminal_save_screen() {
        trace_ui(UiTraceEvent::TerminalSaveScreen);
        if ncurses_stubbed() {
            return;
        }
        SAVE_SCREEN.with(|s| {
            if let Some(save) = *s.borrow() {
                let _ = ncurses::overwrite(ncurses::stdscr(), save);
            }
        });
    }

    /// 89
    pub fn terminal_restore_screen() {
        trace_ui(UiTraceEvent::TerminalRestoreScreen);
        if ncurses_stubbed() {
            return;
        }
        SAVE_SCREEN.with(|s| {
            if let Some(save) = *s.borrow() {
                let _ = ncurses::overwrite(save, ncurses::stdscr());
                let _ = ncurses::touchwin(ncurses::stdscr());
            }
        });
    }

    /// 100
    pub fn terminal_bell_sound() -> isize {
        put_qio();

        let beep = with_state_mut(|state| state.options.error_beep_sound);
        if beep {
            record_bell();
            if ncurses_stubbed() {
                return 1;
            }
            let wrote = unsafe { libc::write(libc::STDOUT_FILENO, b"\x07".as_ptr().cast(), 1) };
            wrote as isize
        } else {
            0
        }
    }

    /// 108
    pub fn put_qio() {
        trace_ui(UiTraceEvent::PutQio);
        with_state_mut(|state| state.screen_has_changed = true);
        if !ncurses_stubbed() {
            let _ = ncurses::refresh();
        }
    }

    /// 118
    pub fn flush_input_buffer() {
        if TEST_UI_CAPTURE.with(std::cell::Cell::get) || TEST_UI_DETAIL.with(std::cell::Cell::get) {
            TEST_FLUSH_COUNT.with(|c| c.set(c.get().wrapping_add(1)));
        }
        if eof_flag() != 0 {
            return;
        }
        while check_for_non_blocking_key_press(0) {}
    }

    /// 126
    pub fn clear_screen() {
        trace_ui(UiTraceEvent::ClearScreen);
        let flush = with_state_mut(|state| state.message_ready_to_print);
        if flush {
            print_message(None);
        }
        if !ncurses_stubbed() {
            let _ = ncurses::clear();
        }
    }

    /// 131
    pub fn clear_to_bottom(row: i32) {
        if !ncurses_stubbed() {
            let _ = ncurses::mv(row, 0);
            let _ = ncurses::clrtobot();
        }
    }

    /// 136
    pub fn move_cursor(coord: Coord) {
        record_move_cursor(coord);
        if ncurses_stubbed() {
            return;
        }
        let _ = ncurses::mv(coord.y, coord.x);
    }

    /// 142
    pub fn add_char(ch: u8, coord: Coord) {
        if ncurses_stubbed() {
            return;
        }
        abort_on_err(ncurses::mvaddch(coord.y, coord.x, encode_tile(ch)));
    }

    /// 158
    pub fn put_string(out_str: &str, coord: Coord) {
        capture_put_string(coord.y, coord.x, out_str);
        trace_ui(UiTraceEvent::PutString {
            text: out_str.to_owned(),
            y: coord.y,
            x: coord.x,
        });
        let truncated = truncate_for_put_string(out_str, coord.x);
        if ncurses_stubbed() {
            return;
        }
        abort_on_err(ncurses::mvaddstr(coord.y, coord.x, &truncated).unwrap_or(ncurses::ERR));
    }

    /// 169
    pub fn put_string_clear_to_eol(s: &str, coord: Coord) {
        trace_ui(UiTraceEvent::PutStringClearToEol {
            text: s.to_owned(),
            y: coord.y,
            x: coord.x,
        });
        capture_ui_message(s);
        let flush = with_state_mut(|state| coord.y == MSG_LINE && state.message_ready_to_print);
        if flush {
            print_message(None);
        }
        if !ncurses_stubbed() {
            let _ = ncurses::mv(coord.y, coord.x);
            let _ = ncurses::clrtoeol();
        }
        put_string(s, coord);
    }

    /// 179
    pub fn erase_line(coord: Coord) {
        if TEST_UI_CAPTURE.with(std::cell::Cell::get) || TEST_UI_DETAIL.with(std::cell::Cell::get) {
            TEST_ERASE_LINES.with(|m| m.borrow_mut().push((coord.y, coord.x)));
        }
        let flush = with_state_mut(|state| coord.y == MSG_LINE && state.message_ready_to_print);
        if flush {
            print_message(None);
        }
        if !ncurses_stubbed() {
            let _ = ncurses::mv(coord.y, coord.x);
            let _ = ncurses::clrtoeol();
        }
    }

    /// 190
    pub fn panel_move_cursor(coord: Coord) {
        let (row_prt, col_prt) = panel_offsets();
        let screen = panel_screen_coord(coord, row_prt, col_prt);
        if ncurses_stubbed() {
            return;
        }
        abort_on_err(ncurses::mv(screen.y, screen.x));
    }

    /// 202
    pub fn panel_put_tile(ch: u8, coord: Coord) {
        let (row_prt, col_prt) = panel_offsets();
        let screen = panel_screen_coord(coord, row_prt, col_prt);
        if ncurses_stubbed() {
            return;
        }
        abort_on_err(ncurses::mvaddch(screen.y, screen.x, encode_tile(ch)));
    }

    /// 227
    pub fn message_line_print_message(message: String) {
        if ncurses_stubbed() {
            return;
        }
        let mut visible = message;
        if visible.len() > 79 {
            visible.truncate(79);
        }
        let saved = current_cursor_position();
        let _ = ncurses::mv(0, 0);
        let _ = ncurses::clrtoeol();
        let _ = ncurses::addstr(&visible);
        let _ = ncurses::mv(saved.y, saved.x);
    }

    /// 241
    pub fn message_line_clear() {
        if ncurses_stubbed() {
            return;
        }
        let saved = current_cursor_position();
        let _ = ncurses::mv(0, 0);
        let _ = ncurses::clrtoeol();
        let _ = ncurses::mv(saved.y, saved.x);
    }

    /// 313
    pub fn print_message(msg: Option<&str>) {
        if let Some(text) = msg {
            capture_ui_message(text);
        }
        let (more_col, combine_messages, old_len) = with_state_mut(|state| {
            if !state.message_ready_to_print {
                return (None, false, 0);
            }
            let old_len = message_old_len(&state.messages[state.last_message_id as usize]);
            let new_len = msg.map_or(0, |m| m.len() as i32);
            if should_show_more(msg.is_none(), old_len, new_len) {
                (Some(clamp_more_column(old_len)), false, old_len)
            } else {
                (None, true, old_len)
            }
        });

        if let Some(col) = more_col {
            put_string(
                " -more-",
                Coord {
                    y: MSG_LINE,
                    x: col,
                },
            );
            loop {
                let key = get_key_input();
                if more_prompt_accepts_key(key) {
                    break;
                }
            }
        }

        with_state_mut(|state| {
            if !combine_messages && !ncurses_stubbed() {
                let _ = ncurses::mv(MSG_LINE, 0);
                let _ = ncurses::clrtoeol();
            }

            let Some(msg) = msg else {
                state.message_ready_to_print = false;
                return;
            };
            state.game.command_count = 0;
            state.message_ready_to_print = true;

            if combine_messages {
                put_string(
                    msg,
                    Coord {
                        y: MSG_LINE,
                        x: old_len + 2,
                    },
                );
                let id = state.last_message_id as usize;
                append_message_slot(&mut state.messages[id], msg);
            } else {
                message_line_print_message(msg.to_string());
                state.last_message_id = advance_message_ring_index(state.last_message_id);
                copy_message_to_ring_slot(&mut state.messages[state.last_message_id as usize], msg);
            }
        });
    }

    /// 324
    pub fn print_message_no_command_interrupt(msg: &str) {
        let saved = with_state_mut(|state| state.game.command_count);
        print_message(Some(msg));
        with_state_mut(|state| state.game.command_count = saved);
    }

    /// 374
    pub fn get_key_input() -> u8 {
        put_qio();
        with_state_mut(|state| state.game.command_count = 0);

        loop {
            let ch = read_getch();

            if ch == libc::EOF {
                with_state_mut(|state| state.message_ready_to_print = false);
                bump_eof_flag();
                if !ncurses_stubbed() {
                    let _ = ncurses::refresh();
                }

                with_state_mut(|state| {
                    if !state.game.character_generated || state.game.character_saved {
                        end_game();
                    }
                });
                player_disturb(1, 0);

                if eof_flag() > 100 {
                    set_panic_save(true);
                    with_state_mut(|state| {
                        c_strcpy(
                            &mut state.game.character_died_from,
                            "(end of input: panic saved)",
                        );
                        if !save_game() {
                            c_strcpy(&mut state.game.character_died_from, "panic: unexpected eof");
                            state.game.character_is_dead = true;
                        }
                    });
                    end_game();
                }
                return ESCAPE;
            }

            if ch != i32::from(ctrl_key(b'R')) {
                return ch as u8;
            }

            if !ncurses_stubbed() {
                let _ = ncurses::wrefresh(ncurses::curscr());
            }
            moria_terminal_initialize();
        }
    }

    /// 387
    pub fn get_command(prompt: &str, command: &mut u8) -> bool {
        if !prompt.is_empty() {
            put_string_clear_to_eol(prompt, Coord { y: 0, x: 0 });
        }
        *command = get_key_input();
        message_line_clear();
        *command != ESCAPE
    }

    /// 396
    pub fn get_menu_item_id(prompt: &str, command: &mut u8) -> bool {
        get_command(prompt, command)
    }

    /// 392
    pub fn get_tile_character(prompt: &str, command: &mut u8) -> bool {
        get_command(prompt, command)
    }

    /// 463
    pub fn get_string_input(in_str: &mut [u8], mut coord: Coord, slen: i32) -> bool {
        if !ncurses_stubbed() {
            let _ = ncurses::mv(coord.y, coord.x);
            for _ in 0..slen {
                let _ = ncurses::addch(b' ' as ncurses::ll::chtype);
            }
            let _ = ncurses::mv(coord.y, coord.x);
        }

        let start_col = coord.x;
        let end_col = clamp_string_input_end_col(start_col, slen);
        let mut p = 0usize;
        let mut flag = false;
        let mut aborted = false;

        while !flag && !aborted {
            let key = i32::from(get_key_input());
            match key as u8 {
                ESCAPE => aborted = true,
                k if k == ctrl_key(b'J') || k == ctrl_key(b'M') => flag = true,
                DELETE => {
                    if coord.x > start_col {
                        coord.x -= 1;
                        put_string(" ", coord);
                        move_cursor(coord);
                        p = p.saturating_sub(1);
                        in_str[p] = 0;
                    }
                }
                k if k == ctrl_key(b'H') => {
                    if coord.x > start_col {
                        coord.x -= 1;
                        put_string(" ", coord);
                        move_cursor(coord);
                        p = p.saturating_sub(1);
                        in_str[p] = 0;
                    }
                }
                _ => {
                    if !is_printable_key(key) || coord.x > end_col {
                        let _ = terminal_bell_sound();
                    } else if !ncurses_stubbed() {
                        let _ =
                            ncurses::mvaddch(coord.y, coord.x, key as u8 as ncurses::ll::chtype);
                        in_str[p] = key as u8;
                        p += 1;
                        coord.x += 1;
                    } else {
                        in_str[p] = key as u8;
                        p += 1;
                        coord.x += 1;
                    }
                }
            }
        }

        if aborted {
            return false;
        }

        // trim trailing blanks, then terminate at p
        while p > 0 && in_str[p - 1] == b' ' {
            p -= 1;
        }
        if p < in_str.len() {
            in_str[p] = 0;
        }

        true
    }

    /// 469
    pub fn get_input_confirmation(prompt: &str) -> bool {
        get_input_confirmation_with_abort(0, prompt) == 1
    }

    /// 501
    pub fn get_input_confirmation_with_abort(column: i32, prompt: &str) -> i32 {
        put_string_clear_to_eol(prompt, Coord { y: 0, x: column });

        if !ncurses_stubbed() {
            let mut y = 0;
            let mut x = 0;
            ncurses::getyx(ncurses::stdscr(), &mut y, &mut x);
            if x > CONFIRM_PROMPT_COL {
                let _ = ncurses::mv(0, CONFIRM_PROMPT_COL);
            } else if y != 0 {
                // 483
            }
            let _ = ncurses::addstr(" [y/n]");
        }

        let mut key = b' ';
        while key == b' ' {
            key = get_key_input();
        }

        message_line_clear();
        confirmation_key_result(key)
    }

    /// 508
    pub fn wait_for_continue_key(line_number: i32) {
        trace_ui(UiTraceEvent::WaitForContinueKey { line: line_number });
        if TEST_UI_CAPTURE.with(std::cell::Cell::get) || TEST_UI_DETAIL.with(std::cell::Cell::get) {
            TEST_WAIT_CONTINUE_LINES.with(|m| m.borrow_mut().push(line_number));
        }
        put_string_clear_to_eol(
            "[ press any key to continue ]",
            Coord {
                y: line_number,
                x: 23,
            },
        );
        let _ = get_key_input();
        erase_line(Coord {
            y: line_number,
            x: 0,
        });
    }

    /// 556
    pub fn check_for_non_blocking_key_press(microseconds: i32) -> bool {
        #[cfg(windows)]
        {
            let _ = microseconds;
            if ncurses_stubbed() {
                return TEST_SELECT_READY.with(|c| c.get()).unwrap_or(false);
            }
            ncurses::timeout(8);
            let result = ncurses::getch();
            ncurses::timeout(-1);
            return result > 0;
        }

        #[cfg(unix)]
        {
            if let Some(ready) = TEST_SELECT_READY.with(std::cell::Cell::get) {
                if !ready {
                    return false;
                }
                if ncurses_stubbed() {
                    if TEST_GETCH_KEYS.with(|q| !q.borrow().is_empty()) {
                        let ch = read_getch();
                        return ch != -1;
                    }
                    // Timed select (interrupt check): ready; flush drain: no queued keys.
                    return microseconds > 0;
                }
                let ch = read_getch();
                if ch == -1 {
                    bump_eof_flag();
                    return false;
                }
                return true;
            }

            if ncurses_stubbed() {
                return false;
            }

            let mut tbuf = libc::timeval {
                tv_sec: 0,
                tv_usec: microseconds as _,
            };
            let mut smask: libc::fd_set = unsafe { std::mem::zeroed() };
            unsafe {
                libc::FD_ZERO(&mut smask);
                libc::FD_SET(libc::STDIN_FILENO, &mut smask);
            }
            if unsafe { libc::select(1, &mut smask, ptr::null_mut(), ptr::null_mut(), &mut tbuf) }
                == 1
            {
                let ch = ncurses::getch();
                if ch == -1 {
                    bump_eof_flag();
                    return false;
                }
                return true;
            }
            false
        }
    }

    /// 585
    pub fn get_default_player_name(buffer: &mut [u8]) {
        buffer.fill(0);
        let default_name = "X";

        if TEST_FORCE_DEFAULT_NAME.with(std::cell::Cell::get) {
            c_strcpy(buffer, default_name);
            return;
        }

        if let Some(name) = TEST_LOGIN_NAME.with(|n| n.borrow().clone()) {
            c_strcpy(buffer, &name);
            return;
        }

        #[cfg(windows)]
        {
            let mut buf_char_count = u32::from(PLAYER_NAME_SIZE);
            let ok = unsafe { windows_get_user_name(buffer.as_mut_ptr(), &mut buf_char_count) };
            if ok == 0 {
                c_strcpy(buffer, default_name);
            }
            return;
        }

        #[cfg(unix)]
        {
            let login = unsafe { libc::getlogin() };
            if !login.is_null() {
                let name = unsafe { std::ffi::CStr::from_ptr(login) };
                if let Ok(s) = name.to_str() {
                    if !s.is_empty() {
                        c_strcpy(buffer, s);
                    }
                }
            }

            if vtype_strlen(buffer) == 0 {
                let pw = unsafe { libc::getpwuid(libc::getuid()) };
                if !pw.is_null() {
                    let name = unsafe { std::ffi::CStr::from_ptr((*pw).pw_name) };
                    if let Ok(s) = name.to_str() {
                        c_strcpy(buffer, s);
                    }
                }
            }

            if vtype_strlen(buffer) == 0 {
                c_strcpy(buffer, default_name);
            }
        }
    }

    /// 671
    pub fn check_file_permissions() -> bool {
        #[cfg(unix)]
        {
            if unsafe { libc::setuid(libc::getuid()) } != 0 {
                let _ = writeln!(
                    io::stderr(),
                    "Can't set permissions correctly!  Setuid call failed.\n"
                );
                return false;
            }
            if unsafe { libc::setgid(libc::getgid()) } != 0 {
                let _ = writeln!(
                    io::stderr(),
                    "Can't set permissions correctly!  Setgid call failed.\n"
                );
                return false;
            }
        }
        true
    }

    #[cfg(unix)]
    /// 652
    pub fn tilde(file: &str) -> Option<String> {
        let mut expanded = String::new();

        if let Some(rest) = file.strip_prefix('~') {
            let mut user = [0u8; 128];
            let mut i = 0usize;
            let mut file_ptr = rest;
            while i < TILDE_USER_MAX {
                let Some(ch) = file_ptr.chars().next() else {
                    break;
                };
                if ch == '/' {
                    break;
                }
                user[i] = ch as u8;
                i += 1;
                file_ptr = &file_ptr[ch.len_utf8()..];
            }
            user[i] = 0;

            let mut pw = ptr::null::<libc::passwd>();
            if i == 0 {
                let login = unsafe { libc::getlogin() };
                if login.is_null() {
                    pw = unsafe { libc::getpwuid(libc::getuid()) };
                    if pw.is_null() {
                        return None;
                    }
                } else {
                    let name = unsafe { std::ffi::CStr::from_ptr(login) };
                    if let Ok(s) = name.to_str() {
                        c_strcpy(&mut user, s);
                    }
                }
            }

            if pw.is_null() {
                let user_str = std::str::from_utf8(&user[..vtype_strlen(&user)]).ok()?;
                pw = unsafe { libc::getpwnam(user_str.as_ptr().cast()) };
                if pw.is_null() {
                    return None;
                }
            }

            let dir = unsafe { std::ffi::CStr::from_ptr((*pw).pw_dir) };
            expanded = dir.to_string_lossy().into_owned();
            expanded.push_str(file_ptr);
        } else {
            expanded.push_str(file);
        }

        if expanded.len() >= TILDE_EXPANDED_MAX {
            expanded.truncate(TILDE_EXPANDED_MAX - 1);
        }
        Some(expanded)
    }

    #[cfg(unix)]
    /// 603
    pub fn tfopen(file: &str, mode: &str) -> Option<std::fs::File> {
        // only sets errno=ENOENT when tilde() fails; fopen errno is preserved
        let expanded = tilde(file)?;
        std::fs::OpenOptions::new()
            .read(mode.contains('r'))
            .write(mode.contains('w') || mode.contains('+'))
            .create(mode.contains('w') || mode.contains('a'))
            .append(mode.contains('a'))
            .open(&expanded)
            .ok()
    }

    #[cfg(unix)]
    /// 613
    pub fn topen(file: &str, flags: i32, mode: libc::mode_t) -> i32 {
        // only sets errno=ENOENT when tilde() fails; open errno is preserved
        let Some(expanded) = tilde(file) else {
            set_errno(libc::ENOENT);
            return -1;
        };
        let Ok(c_path) = std::ffi::CString::new(expanded) else {
            set_errno(libc::ENOENT);
            return -1;
        };
        unsafe { libc::open(c_path.as_ptr(), flags, mode as libc::c_uint) }
    }

    #[cfg(windows)]
    fn windows_get_user_name(buf: *mut u8, size: *mut u32) -> i32 {
        extern "system" {
            fn GetUserNameA(lpBuffer: *mut u8, pcbBuffer: *mut u32) -> i32;
        }
        unsafe { GetUserNameA(buf, size) }
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn errno_ptr() -> *mut libc::c_int {
    unsafe { libc::__error() }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
fn errno_ptr() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}

fn set_errno(err: i32) {
    unsafe {
        *errno_ptr() = err;
    }
}

#[allow(unused_imports, reason = "re-exports kept for call-site convenience")]
pub use terminal::Coord;

#[allow(unused_imports, reason = "re-exports kept for call-site convenience")]
pub use crate::game::get_direction_with_memory;

/// Test hook for direction-taking player actions (open/close door).
#[doc(hidden)]
pub fn test_set_direction(direction: Option<i32>) {
    game::test_set_direction(direction);
}
