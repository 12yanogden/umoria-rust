//! Port of src/helpers.cpp — see phase_2.

use std::ffi::CString;
use std::fmt::Write;
use std::ptr;

use libc::{c_char, ERANGE};

use crate::types::{Vtype_t, MORIA_MESSAGE_SIZE};

/// Returns position of first set bit and clears that bit — C++ `getAndClearFirstBit`.
pub fn get_and_clear_first_bit(flag: &mut u32) -> i32 {
    let mut mask = 0x1u32;

    for i in 0..(std::mem::size_of::<u32>() * 8) {
        if *flag & mask != 0 {
            *flag &= !mask;
            return i as i32;
        }
        mask <<= 1;
    }

    -1
}

/// C NUL-terminated byte slice length (excluding terminator).
fn c_strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

/// C `strchr`-style search for `needle` starting at `from` within `haystack`.
fn c_strchr(haystack: &[u8], from: usize, needle: u8) -> Option<usize> {
    haystack[from..]
        .iter()
        .position(|&b| b == needle)
        .map(|pos| from + pos)
}

/// Write `formatted` into `buf` like `snprintf(buf, MORIA_MESSAGE_SIZE, …)`.
fn snprintf_vtype(buf: &mut Vtype_t, formatted: &str) {
    let max = MORIA_MESSAGE_SIZE;
    let bytes = formatted.as_bytes();
    let n = bytes.len().min(max - 1);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf[n] = 0;
    if n + 1 < max {
        buf[n + 1..].fill(0);
    }
}

/// Insert a long number into a string — C++ `insertNumberIntoString`.
pub fn insert_number_into_string(
    to_string: &mut Vtype_t,
    from_string: &[u8],
    number: i32,
    show_sign: bool,
) {
    let from_len = c_strlen(from_string);
    if from_len == 0 {
        return;
    }

    let to_len = c_strlen(to_string);
    let mut to_str_tmp = 0usize;
    let mut match_start: Option<usize> = None;

    while let Some(str_pos) = c_strchr(to_string, to_str_tmp, from_string[0]) {
        let hay = &to_string[str_pos..];
        let matches = hay.len() >= from_len
            && hay[..from_len]
                .iter()
                .zip(from_string[..from_len].iter())
                .all(|(a, b)| a == b);

        if matches {
            match_start = Some(str_pos);
            break;
        }
        to_str_tmp = str_pos + 1;
    }

    let Some(str_pos) = match_start else {
        return;
    };

    let prefix = &to_string[..str_pos];
    let suffix = &to_string[str_pos + from_len..to_len];

    let prefix_str = c_bytes_to_str(prefix);
    let suffix_str = c_bytes_to_str(suffix);

    let mut formatted = String::new();
    if number >= 0 && show_sign {
        let _ = write!(formatted, "{prefix_str}+{number}{suffix_str}");
    } else {
        let _ = write!(formatted, "{prefix_str}{number}{suffix_str}");
    }

    snprintf_vtype(to_string, &formatted);
}

/// Inserts a string into a string — C++ `insertStringIntoString`.
pub fn insert_string_into_string(
    to_string: &mut Vtype_t,
    from_string: &[u8],
    str_to_insert: Option<&[u8]>,
) {
    let from_len = c_strlen(from_string) as i32;
    let to_len = c_strlen(to_string) as i32;

    if from_len > to_len {
        return;
    }

    let bound = (to_len - from_len) as usize;
    let mut pc = 0usize;
    let mut found = false;

    while pc <= bound {
        let mut i = 0;
        while i < from_len {
            if to_string[pc + i as usize] != from_string[i as usize] {
                break;
            }
            i += 1;
        }
        if i == from_len {
            found = true;
            break;
        }
        pc += 1;
    }

    if !found {
        return;
    }

    let mut new_string = [0u8; MORIA_MESSAGE_SIZE];

    // strncpy(new_string, to_string, pc - to_string); NUL at pc
    new_string[..pc].copy_from_slice(&to_string[..pc]);
    new_string[pc] = 0;

    let mut write_pos = pc;

    if let Some(insert) = str_to_insert {
        let insert_len = c_strlen(insert);
        for &b in &insert[..insert_len] {
            if write_pos >= MORIA_MESSAGE_SIZE - 1 {
                break;
            }
            new_string[write_pos] = b;
            write_pos += 1;
        }
        new_string[write_pos] = 0;
    }

    let suffix_start = pc + from_len as usize;
    let suffix = &to_string[suffix_start..];
    let suffix_len = c_strlen(suffix);
    for &b in &suffix[..suffix_len] {
        if write_pos >= MORIA_MESSAGE_SIZE - 1 {
            break;
        }
        new_string[write_pos] = b;
        write_pos += 1;
    }
    new_string[write_pos] = 0;

    // strcpy(to_string, new_string)
    to_string.copy_from_slice(&new_string);
}

pub fn is_vowel(ch: u8) -> bool {
    matches!(
        ch,
        b'a' | b'e' | b'i' | b'o' | b'u' | b'A' | b'E' | b'I' | b'O' | b'U'
    )
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn errno_ptr() -> *mut libc::c_int {
    unsafe { libc::__error() }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
fn errno_ptr() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}

/// Parse unsigned hex with C `sscanf("%lx")` semantics — wizard flag entry.
pub fn sscanf_lx(str: &str, number: &mut i32) -> i32 {
    let c_str = match CString::new(str) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    unsafe {
        *errno_ptr() = 0;
        let mut endptr: *mut c_char = ptr::null_mut();
        let num = libc::strtoul(c_str.as_ptr(), &mut endptr, 16);

        if endptr == c_str.as_ptr() as *mut c_char {
            return 0;
        }

        *number = num as i32;
        1
    }
}

/// Parse base-10 integer with C `strtol` semantics — C++ `stringToNumber`.
pub fn string_to_number(str: &str, number: &mut i32) -> bool {
    let c_str = match CString::new(str) {
        Ok(s) => s,
        Err(_) => return false,
    };

    unsafe {
        *errno_ptr() = 0;
        let mut endptr: *mut c_char = ptr::null_mut();
        let num = libc::strtol(c_str.as_ptr(), &mut endptr, 10);

        if *errno_ptr() == ERANGE {
            return false;
        }

        if *errno_ptr() != 0 {
            return false;
        }

        if *endptr != 0 {
            return false;
        }

        *number = num as i32;
        true
    }
}

/// C++ `getCurrentUnixTime` — `time(nullptr)` cast to `u32`.
pub fn get_current_unix_time() -> u32 {
    unsafe { libc::time(ptr::null_mut()) as u32 }
}

/// C++ `humanDateString` — `localtime` + `strftime` into 11-byte buffer.
pub fn human_date_string(day: &mut [u8; 11]) {
    unsafe {
        let now = libc::time(ptr::null_mut());
        let datetime = libc::localtime(&now);
        if datetime.is_null() {
            day[0] = 0;
            return;
        }

        #[cfg(windows)]
        let fmt = b"%a %b %d\0";
        #[cfg(not(windows))]
        let fmt = b"%a %b %e\0";

        libc::strftime(
            day.as_mut_ptr() as *mut c_char,
            11,
            fmt.as_ptr() as *const c_char,
            datetime,
        );
    }
}

fn c_bytes_to_str(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snprintf_vtype_truncates_like_c() {
        let mut buf = [0u8; MORIA_MESSAGE_SIZE];
        let s = "x".repeat(100);
        snprintf_vtype(&mut buf, &s);
        assert_eq!(c_strlen(&buf), MORIA_MESSAGE_SIZE - 1);
    }
}
