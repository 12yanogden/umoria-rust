//! Phase 1.2 — ncurses integration strategy tests.
//! See `.cursor/plans/rust-translation/phase_1.2.md`.

use umoria::ui_io::{self, terminal, Coord};

// ---------------------------------------------------------------------------
// 1. Crate-links smoke test (no initscr)
// ---------------------------------------------------------------------------
#[test]
fn ncurses_crate_links_without_initscr() {
    // ^R = CTRL_KEY('R') = 0x12; keyname does not require initscr.
    let name = ncurses::keyname(0x12);
    assert!(
        name.as_ref().is_some_and(|s| !s.is_empty()),
        "ncurses::keyname(0x12) should return a non-empty string"
    );
    // Prove ERR/OK constants are linkable.
    let _ = ncurses::ERR;
    let _ = ncurses::OK;
}

// ---------------------------------------------------------------------------
// 2. 80-col truncation helper (pure, no TTY)
// ---------------------------------------------------------------------------
#[test]
fn truncate_for_put_string_clamps_x_and_truncates() {
    assert_eq!(ui_io::truncate_for_put_string("ABCDEFGH", 75), "ABCD");
    assert_eq!(ui_io::truncate_for_put_string("AB", 79), "");
    assert_eq!(ui_io::truncate_for_put_string("AB", 85), "");
    assert_eq!(ui_io::truncate_for_put_string("Hi", 10), "Hi");
}

#[test]
fn truncate_for_put_string_output_length_is_79_minus_x() {
    for x in [0, 10, 50, 75, 79] {
        let x_clamped = if x > 79 { 79 } else { x };
        let out = ui_io::truncate_for_put_string("X".repeat(100).as_str(), x);
        assert_eq!(out.len(), (79 - x_clamped) as usize);
    }
}

// ---------------------------------------------------------------------------
// 3. message_line resize(79) cap (pure)
// ---------------------------------------------------------------------------
#[test]
fn cap_message_line_truncates_overlong_input() {
    let capped = ui_io::cap_message_line("A".repeat(100));
    assert_eq!(capped.len(), 79);
    assert_eq!(&capped[..79], &"A".repeat(79));
}

#[test]
fn cap_message_line_nul_pads_shorter_input() {
    let capped = ui_io::cap_message_line("hello".to_string());
    assert_eq!(capped.len(), 79);
    assert_eq!(&capped.as_bytes()[..5], b"hello");
    assert!(capped.as_bytes()[5..].iter().all(|&b| b == 0));
}

// ---------------------------------------------------------------------------
// 4. Standout sign-bit encoding (pure)
// ---------------------------------------------------------------------------
#[test]
fn encode_tile_plain_byte() {
    assert_eq!(ui_io::encode_tile(b'@'), 0x40);
}

#[test]
fn encode_tile_standout_high_bit_sign_extends() {
    // C passes `char` to mvaddch; signed char 0xC0 (-64) sign-extends into chtype.
    assert_eq!(
        ui_io::encode_tile(0xC0),
        0xC0_u8 as i8 as ncurses::ll::chtype
    );
}

// ---------------------------------------------------------------------------
// 5. API-surface compile test
// ---------------------------------------------------------------------------
#[test]
fn terminal_api_surface_signatures_exist() {
    #[allow(dead_code)]
    fn assert_signatures() {
        let _: fn() -> bool = terminal::terminal_initialize;
        let _: fn() = terminal::terminal_restore;
        let _: fn() = terminal::terminal_save_screen;
        let _: fn() = terminal::terminal_restore_screen;
        let _: fn() -> isize = terminal::terminal_bell_sound;

        let _: fn() = terminal::put_qio;
        let _: fn() = terminal::flush_input_buffer;
        let _: fn() = terminal::clear_screen;
        let _: fn(i32) = terminal::clear_to_bottom;

        let _: fn(Coord) = terminal::move_cursor;
        let _: fn(u8, Coord) = terminal::add_char;
        let _: fn(&str, Coord) = terminal::put_string;
        let _: fn(&str, Coord) = terminal::put_string_clear_to_eol;
        let _: fn(Coord) = terminal::erase_line;
        let _: fn(Coord) = terminal::panel_move_cursor;
        let _: fn(u8, Coord) = terminal::panel_put_tile;

        let _: fn(String) = terminal::message_line_print_message;
        let _: fn() = terminal::message_line_clear;
        let _: fn(Option<&str>) = terminal::print_message;
        let _: fn(&str) = terminal::print_message_no_command_interrupt;

        let _: fn() -> u8 = terminal::get_key_input;
        let _: fn(&str, &mut u8) -> bool = terminal::get_command;
        let _: fn(&str, &mut u8) -> bool = terminal::get_menu_item_id;
        let _: fn(&str, &mut u8) -> bool = terminal::get_tile_character;
        let _: fn(&mut [u8], Coord, i32) -> bool = terminal::get_string_input;
        let _: fn(&str) -> bool = terminal::get_input_confirmation;
        let _: fn(i32, &str) -> i32 = terminal::get_input_confirmation_with_abort;
        let _: fn(i32) = terminal::wait_for_continue_key;
        let _: fn(i32) -> bool = terminal::check_for_non_blocking_key_press;

        let _: fn(&mut [u8]) = terminal::get_default_player_name;
        let _: fn() -> bool = terminal::check_file_permissions;

        #[cfg(unix)]
        {
            let _: fn(&str) -> Option<String> = terminal::tilde;
        }
    }
    assert_signatures();
}
