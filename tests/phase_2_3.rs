//! Phase 2.3 — helpers.{h,cpp} string/number helpers.
//! See `.cursor/plans/rust-translation/phase_2.3.md`.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use std::ffi::CStr;
use umoria::helpers::{
    get_and_clear_first_bit, get_current_unix_time, human_date_string, insert_number_into_string,
    insert_string_into_string, is_vowel, string_to_number,
};
use umoria::types::{Vtype_t, MORIA_MESSAGE_SIZE_LEN};

fn vtype_from(s: &str) -> Vtype_t {
    let mut buf = [0u8; MORIA_MESSAGE_SIZE_LEN];
    let bytes = s.as_bytes();
    let n = bytes.len().min(MORIA_MESSAGE_SIZE_LEN - 1);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf
}

fn vtype_str(buf: &Vtype_t) -> String {
    CStr::from_bytes_until_nul(buf)
        .map(|c| c.to_string_lossy().into_owned())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// 1. getAndClearFirstBit
// ---------------------------------------------------------------------------
#[test]
fn get_and_clear_first_bit_cases() {
    let mut flag = 0x1u32;
    assert_eq!(get_and_clear_first_bit(&mut flag), 0);
    assert_eq!(flag, 0x0);

    flag = 0x8000_0000;
    assert_eq!(get_and_clear_first_bit(&mut flag), 31);
    assert_eq!(flag, 0x0);

    flag = 0x0;
    assert_eq!(get_and_clear_first_bit(&mut flag), -1);
    assert_eq!(flag, 0x0);

    flag = 0x0000_0006;
    assert_eq!(get_and_clear_first_bit(&mut flag), 1);
    assert_eq!(flag, 0x4);
    assert_eq!(get_and_clear_first_bit(&mut flag), 2);
    assert_eq!(flag, 0x0);
    assert_eq!(get_and_clear_first_bit(&mut flag), -1);

    flag = 0xF000_0000;
    assert_eq!(get_and_clear_first_bit(&mut flag), 28);
    assert_eq!(flag, 0xE000_0000);

    flag = 0xFFFF_FFFF;
    assert_eq!(get_and_clear_first_bit(&mut flag), 0);
    assert_eq!(flag, 0xFFFF_FFFE);
}

// ---------------------------------------------------------------------------
// 2. insertNumberIntoString
// ---------------------------------------------------------------------------
#[test]
fn insert_number_into_string_show_sign_positive() {
    let mut to = vtype_from("Deal %d damage");
    insert_number_into_string(&mut to, b"%d", 5, true);
    assert_eq!(vtype_str(&to), "Deal +5 damage");
}

#[test]
fn insert_number_into_string_no_sign_positive() {
    let mut to = vtype_from("Deal %d damage");
    insert_number_into_string(&mut to, b"%d", 5, false);
    assert_eq!(vtype_str(&to), "Deal 5 damage");
}

#[test]
fn insert_number_into_string_negative_ignores_show_sign() {
    let mut to = vtype_from("Deal %d damage");
    insert_number_into_string(&mut to, b"%d", -3, true);
    assert_eq!(vtype_str(&to), "Deal -3 damage");
}

#[test]
fn insert_number_into_string_zero_with_show_sign() {
    let mut to = vtype_from("Deal %d damage");
    insert_number_into_string(&mut to, b"%d", 0, true);
    assert_eq!(vtype_str(&to), "Deal +0 damage");
}

#[test]
fn insert_number_into_string_missing_marker_noop() {
    let mut to = vtype_from("no marker here");
    insert_number_into_string(&mut to, b"%d", 42, true);
    assert_eq!(vtype_str(&to), "no marker here");
}

#[test]
fn insert_number_into_string_skips_false_percent_match() {
    let mut to = vtype_from("50% of %d");
    insert_number_into_string(&mut to, b"%d", 7, false);
    assert_eq!(vtype_str(&to), "50% of 7");
}

#[test]
fn insert_number_into_string_truncates_at_moria_message_size() {
    // Prefix + 5-digit number exceeds 79 chars; snprintf(to, 80, …) clips tail.
    let prefix = "x".repeat(75);
    let mut to = vtype_from(&format!("{prefix}%d"));
    insert_number_into_string(&mut to, b"%d", 12345, false);
    let got = vtype_str(&to);
    assert!(got.len() <= MORIA_MESSAGE_SIZE_LEN - 1);
    assert_eq!(got.len(), MORIA_MESSAGE_SIZE_LEN - 1);
    assert_eq!(
        got,
        format!("{}{}", prefix, "12345")
            .chars()
            .take(MORIA_MESSAGE_SIZE_LEN - 1)
            .collect::<String>()
    );
}

#[test]
fn insert_number_into_string_truncates_near_end_marker() {
    let mut to =
        vtype_from("0123456789012345678901234567890123456789012345678901234567890123456789%dXY");
    insert_number_into_string(&mut to, b"%d", 999999, true);
    let got = vtype_str(&to);
    assert!(got.len() <= MORIA_MESSAGE_SIZE_LEN - 1);
    assert!(!got.contains("%d"));
    assert!(!got.ends_with("XY") || got.len() == MORIA_MESSAGE_SIZE_LEN - 1);
}

// ---------------------------------------------------------------------------
// 3. insertStringIntoString
// ---------------------------------------------------------------------------
#[test]
fn insert_string_into_string_match() {
    let mut to = vtype_from("hit the @ hard");
    insert_string_into_string(&mut to, b"@", Some(b"goblin"));
    assert_eq!(vtype_str(&to), "hit the goblin hard");
}

#[test]
fn insert_string_into_string_no_match() {
    let mut to = vtype_from("hello world");
    insert_string_into_string(&mut to, b"@", Some(b"goblin"));
    assert_eq!(vtype_str(&to), "hello world");
}

#[test]
fn insert_string_into_string_null_insert_deletes() {
    let mut to = vtype_from("an XX item");
    insert_string_into_string(&mut to, b"XX", None);
    assert_eq!(vtype_str(&to), "an  item");
}

#[test]
fn insert_string_into_string_overlap_sliding_compare() {
    let mut to = vtype_from("aaab");
    insert_string_into_string(&mut to, b"aab", Some(b"Z"));
    assert_eq!(vtype_str(&to), "aZ"); // C++ match at index 1: suffix after pc+from_len is empty
}

#[test]
fn insert_string_into_string_from_longer_than_to_noop() {
    let mut to = vtype_from("hi");
    insert_string_into_string(&mut to, b"hello", Some(b"X"));
    assert_eq!(vtype_str(&to), "hi");
}

#[test]
fn insert_string_into_string_match_at_start() {
    let mut to = vtype_from("@end");
    insert_string_into_string(&mut to, b"@", Some(b"start"));
    assert_eq!(vtype_str(&to), "startend");
}

#[test]
fn insert_string_into_string_match_at_end() {
    let mut to = vtype_from("begin@");
    insert_string_into_string(&mut to, b"@", Some(b"finish"));
    assert_eq!(vtype_str(&to), "beginfinish");
}

// ---------------------------------------------------------------------------
// 4. isVowel
// ---------------------------------------------------------------------------
#[test]
fn is_vowel_lowercase() {
    for ch in [b'a', b'e', b'i', b'o', b'u'] {
        assert!(is_vowel(ch), "expected vowel for {ch}");
    }
}

#[test]
fn is_vowel_uppercase() {
    for ch in [b'A', b'E', b'I', b'O', b'U'] {
        assert!(is_vowel(ch), "expected vowel for {ch}");
    }
}

#[test]
fn is_vowel_consonants() {
    for ch in [b'b', b'z', b'Y', b'y'] {
        assert!(!is_vowel(ch), "expected consonant for {ch}");
    }
}

#[test]
fn is_vowel_non_letters() {
    for ch in [b'0', b' ', 0] {
        assert!(!is_vowel(ch), "expected non-vowel for {ch}");
    }
}

// ---------------------------------------------------------------------------
// 5. stringToNumber (strtol semantics)
// ---------------------------------------------------------------------------
#[test]
fn string_to_number_basic() {
    let mut n = 0;
    assert!(string_to_number("123", &mut n));
    assert_eq!(n, 123);

    assert!(string_to_number("-5", &mut n));
    assert_eq!(n, -5);

    assert!(string_to_number("0", &mut n));
    assert_eq!(n, 0);
}

#[test]
fn string_to_number_no_digits() {
    let mut n = 0;
    assert!(!string_to_number("abc", &mut n));
}

#[test]
fn string_to_number_trailing_garbage() {
    let mut n = 0;
    assert!(!string_to_number("12x", &mut n));
}

#[test]
fn string_to_number_empty() {
    // macOS strtol("") sets errno=EINVAL; C++ `stringToNumber` returns false (see helpers.cpp).
    let mut n = 99;
    assert!(!string_to_number("", &mut n));
    assert_eq!(n, 99);
}

#[test]
fn string_to_number_leading_whitespace() {
    let mut n = 0;
    assert!(string_to_number(" 42", &mut n));
    assert_eq!(n, 42);
}

#[test]
fn string_to_number_erange_overflow() {
    let mut n = 0;
    assert!(!string_to_number("99999999999999999999999999", &mut n));
}

#[test]
fn string_to_number_erange_underflow() {
    let mut n = 0;
    assert!(!string_to_number("-99999999999999999999999999", &mut n));
}

#[test]
fn string_to_number_int_truncation_on_64bit_long() {
    // macOS/Linux reference build: long is 64-bit; value fits in long but not i32.
    if std::mem::size_of::<libc::c_long>() == 8 {
        let mut n = 0;
        assert!(string_to_number("3000000000", &mut n));
        assert_eq!(n, 3_000_000_000i64 as i32); // faithful (int) truncation
    }
}

// ---------------------------------------------------------------------------
// 6. humanDateString
// ---------------------------------------------------------------------------
#[test]
fn human_date_string_shape() {
    let mut day = [0u8; 11];
    human_date_string(&mut day);

    let nul = day.iter().position(|&b| b == 0).expect("NUL-terminated");
    assert!(
        nul <= 10,
        "formatted length must fit 11-byte buffer incl. NUL"
    );

    let s = CStr::from_bytes_until_nul(&day).unwrap().to_str().unwrap();
    assert!(s.len() <= 10);

    // Shape: "Mon Jul  5" — `%a %b %e` (space-padded day 1–9 on Unix).
    let bytes = s.as_bytes();
    assert!((8..=10).contains(&bytes.len()));
    assert!(bytes[0..3].iter().all(u8::is_ascii_alphabetic));
    assert_eq!(bytes[3], b' ');
    assert!(bytes[4..7].iter().all(u8::is_ascii_alphabetic));
    assert_eq!(bytes[7], b' ');
    assert!(bytes[8].is_ascii_whitespace() || bytes[8].is_ascii_digit());
    assert!(bytes[9].is_ascii_digit());
}

// ---------------------------------------------------------------------------
// 7. getCurrentUnixTime
// ---------------------------------------------------------------------------
#[test]
fn get_current_unix_time_plausible_and_non_decreasing() {
    let t1 = get_current_unix_time();
    std::thread::sleep(std::time::Duration::from_millis(1));
    let t2 = get_current_unix_time();

    assert!(t1 > 1_600_000_000);
    assert!(t2 >= t1);
}
