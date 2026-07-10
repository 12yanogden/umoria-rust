//! Port of `src/game_save.cpp` — XOR save stream primitives and score record I/O.

use std::cell::{Cell, RefCell};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

use crate::config::monsters::MON_MIN_INDEX_ID;
use crate::config::treasure::MIN_TREASURE_LIST_ID;
use crate::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use crate::game::{
    exit_program, is_current_game_version, random_number, valid_game_version, with_state,
    with_state_mut, State,
};
use crate::helpers::get_current_unix_time;
use crate::inventory::{PlayerEquipment, PLAYER_INVENTORY_SIZE};
use crate::monster::MON_MAX_CREATURES;
use crate::player::{player_disturb, player_strength};
use crate::scores::player_calculate_total_points_for_state;
use crate::store::{store_maintenance, STORE_MAX_DISCRETE_ITEMS};
use crate::types::{LEVEL_MAX_OBJECTS, MON_TOTAL_ALLOCATIONS};
use crate::ui_io::{self, eof_flag, panic_save, terminal};
use crate::version::{CURRENT_VERSION_MAJOR, CURRENT_VERSION_MINOR, CURRENT_VERSION_PATCH};

use crate::inventory::{Inventory, INSCRIP_SIZE};
use crate::monster::Monster;
use crate::player::PLAYER_NAME_SIZE;

const UCHAR_MAX: u8 = u8::MAX;

thread_local! {
    static FILEPTR: RefCell<Option<File>> = const { RefCell::new(None) };
    static C_GETC_EOF_MODE: Cell<bool> = const { Cell::new(false) };
    static TEST_BUFFER: RefCell<Option<Cursor<Vec<u8>>>> = const { RefCell::new(None) };
    static UNGET_BYTE: Cell<Option<u8>> = const { Cell::new(None) };
    static XOR_BYTE: Cell<u8> = const { Cell::new(0) };
    static FROM_SAVE_FILE: Cell<i32> = const { Cell::new(0) };
    static START_TIME: Cell<u32> = const { Cell::new(0) };
    static FORCED_SEED_BYTE: Cell<Option<u8>> = const { Cell::new(None) };
    static TEST_UNIX_TIME: Cell<Option<u32>> = const { Cell::new(None) };
    static TEST_SAVE_FAIL_FLUSH: Cell<bool> = const { Cell::new(false) };
    static TEST_FORCE_SAVE_CHAR_FAIL: Cell<bool> = const { Cell::new(false) };
    static TEST_STORE_MAINTENANCE_COUNT: Cell<u32> = const { Cell::new(0) };
}

/// Port of `HighScore_t` in scores.h (on-wire record is 73 ciphertext bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HighScore {
    pub points: i32,
    pub birth_date: i32,
    pub uid: i16,
    pub mhp: i16,
    pub chp: i16,
    pub dungeon_depth: u8,
    pub level: u8,
    pub deepest_dungeon_depth: u8,
    pub gender: u8,
    pub race: u8,
    pub character_class: u8,
    pub name: [u8; PLAYER_NAME_SIZE as usize],
    pub died_from: [u8; 25],
}

impl Default for HighScore {
    fn default() -> Self {
        Self {
            points: 0,
            birth_date: 0,
            uid: 0,
            mhp: 0,
            chp: 0,
            dungeon_depth: 0,
            level: 0,
            deepest_dungeon_depth: 0,
            gender: 0,
            race: 0,
            character_class: 0,
            name: [0; PLAYER_NAME_SIZE as usize],
            died_from: [0; 25],
        }
    }
}

/// Ciphertext bytes emitted by `save_high_score` (1 xor seed + 72 field bytes).
pub const HIGH_SCORE_RECORD_SIZE: usize = 73;

pub fn xor_byte() -> u8 {
    XOR_BYTE.with(std::cell::Cell::get)
}

pub fn set_xor_byte(value: u8) {
    XOR_BYTE.with(|c| c.set(value));
}

pub fn from_save_file() -> i32 {
    FROM_SAVE_FILE.with(std::cell::Cell::get)
}

pub fn start_time() -> u32 {
    START_TIME.with(std::cell::Cell::get)
}

pub fn set_from_save_file(value: i32) {
    FROM_SAVE_FILE.with(|c| c.set(value));
}

pub fn set_start_time(value: u32) {
    START_TIME.with(|c| c.set(value));
}

/// Test hook: force the 4th header byte (`randomNumber(256) - 1`).
#[doc(hidden)]
pub fn test_set_forced_seed_byte(seed: Option<u8>) {
    FORCED_SEED_BYTE.with(|c| c.set(seed));
}

#[doc(hidden)]
pub fn test_set_unix_time(clock: Option<u32>) {
    TEST_UNIX_TIME.with(|c| c.set(clock));
}

#[doc(hidden)]
pub fn test_set_save_fail_flush(fail: bool) {
    TEST_SAVE_FAIL_FLUSH.with(|c| c.set(fail));
}

#[doc(hidden)]
pub fn test_set_force_save_char_fail(fail: bool) {
    TEST_FORCE_SAVE_CHAR_FAIL.with(|c| c.set(fail));
}

#[doc(hidden)]
pub fn test_store_maintenance_count() -> u32 {
    TEST_STORE_MAINTENANCE_COUNT.with(std::cell::Cell::get)
}

#[doc(hidden)]
pub fn test_reset_store_maintenance_count() {
    TEST_STORE_MAINTENANCE_COUNT.with(|c| c.set(0));
}

fn test_buffer_active() -> bool {
    TEST_BUFFER.with(|buf| buf.borrow().is_some())
}

fn save_unix_time() -> u32 {
    TEST_UNIX_TIME
        .with(std::cell::Cell::get)
        .unwrap_or_else(get_current_unix_time)
}

fn flush_ok() -> bool {
    !TEST_SAVE_FAIL_FLUSH.with(std::cell::Cell::get)
}

/// Port of `setFileptr` in `game_save.cpp`.
pub fn set_fileptr(file: File) {
    FILEPTR.with(|fp| *fp.borrow_mut() = Some(file));
    TEST_BUFFER.with(|buf| *buf.borrow_mut() = None);
}

/// Take ownership of the active file stream (for `fclose` parity in scores).
pub fn take_fileptr() -> Option<File> {
    FILEPTR.with(|fp| fp.borrow_mut().take())
}

/// Enable C `getc` EOF semantics (`0xFF` byte + `feof`) for score-file I/O.
pub fn set_c_getc_eof_mode(on: bool) {
    C_GETC_EOF_MODE.with(|c| c.set(on));
}

/// Port of `fseek` / `ftell` on the active `fileptr`.
pub fn fileptr_seek(pos: SeekFrom) -> io::Result<u64> {
    if let Some(mut file) = FILEPTR.with(|fp| fp.borrow_mut().take()) {
        let result = file.seek(pos);
        FILEPTR.with(|fp| *fp.borrow_mut() = Some(file));
        return result;
    }
    TEST_BUFFER.with(|buf| {
        if let Some(cursor) = buf.borrow_mut().as_mut() {
            cursor.seek(pos)
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "game_save: no fileptr or test buffer",
            ))
        }
    })
}

pub fn fileptr_tell() -> io::Result<u64> {
    fileptr_seek(SeekFrom::Current(0))
}

/// Port of `putc` — raw byte write without XOR (score version header).
pub fn putc_raw(byte: u8) -> io::Result<()> {
    put_byte(byte)
}

/// Port of `(uint8_t)getc(fileptr)` — used for score version header reads.
pub fn score_getc() -> u8 {
    match get_byte_raw() {
        Ok(byte) => byte,
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
            ui_io::test_set_eof_flag(1);
            0xFF
        }
        Err(_) => {
            ui_io::test_set_eof_flag(1);
            0xFF
        }
    }
}

/// Test harness: route wr/rd through an in-memory buffer (see `tests/game_save_stream.rs`).
pub fn test_reset_buffer() {
    TEST_BUFFER.with(|buf| *buf.borrow_mut() = Some(Cursor::new(Vec::new())));
    FILEPTR.with(|fp| *fp.borrow_mut() = None);
}

pub fn test_buffer_inject(bytes: &[u8]) {
    TEST_BUFFER.with(|buf| *buf.borrow_mut() = Some(Cursor::new(bytes.to_vec())));
    FILEPTR.with(|fp| *fp.borrow_mut() = None);
}

pub fn test_write_raw(bytes: &[u8]) -> io::Result<()> {
    for &byte in bytes {
        put_byte(byte)?;
    }
    Ok(())
}

pub fn test_buffer_len() -> usize {
    TEST_BUFFER.with(|buf| buf.borrow().as_ref().map_or(0, |c| c.get_ref().len()))
}

pub fn test_buffer_remaining() -> usize {
    TEST_BUFFER.with(|buf| {
        buf.borrow().as_ref().map_or(0, |c| {
            c.get_ref().len().saturating_sub(c.position() as usize)
        })
    })
}

pub fn test_buffer_bytes() -> Vec<u8> {
    TEST_BUFFER.with(|buf| {
        buf.borrow()
            .as_ref()
            .map(|c| c.get_ref().clone())
            .unwrap_or_default()
    })
}

pub fn test_rewind_buffer() -> io::Result<()> {
    TEST_BUFFER.with(|buf| {
        if let Some(cursor) = buf.borrow_mut().as_mut() {
            cursor.seek(SeekFrom::Start(0))
        } else {
            Ok(0)
        }
        .map(|_| ())
    })
}

fn put_byte(value: u8) -> io::Result<()> {
    if let Some(file) = FILEPTR.with(|fp| fp.borrow_mut().take()) {
        let mut file = file;
        file.write_all(&[value])?;
        FILEPTR.with(|fp| *fp.borrow_mut() = Some(file));
        return Ok(());
    }
    TEST_BUFFER.with(|buf| {
        if let Some(cursor) = buf.borrow_mut().as_mut() {
            cursor.write_all(&[value])?;
        }
        Ok(())
    })
}

fn unget_byte_raw(byte: u8) {
    UNGET_BYTE.with(|c| c.set(Some(byte)));
}

fn get_byte_raw() -> io::Result<u8> {
    if let Some(byte) = UNGET_BYTE.with(std::cell::Cell::take) {
        return Ok(byte);
    }
    if let Some(file) = FILEPTR.with(|fp| fp.borrow_mut().take()) {
        let mut file = file;
        let mut byte = [0u8; 1];
        let n = file.read(&mut byte)?;
        FILEPTR.with(|fp| *fp.borrow_mut() = Some(file));
        if n == 0 {
            if C_GETC_EOF_MODE.with(std::cell::Cell::get) {
                ui_io::test_set_eof_flag(1);
                return Ok(0xFF);
            }
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        return Ok(byte[0]);
    }
    TEST_BUFFER.with(|buf| {
        if let Some(cursor) = buf.borrow_mut().as_mut() {
            let mut byte = [0u8; 1];
            let n = cursor.read(&mut byte)?;
            if n == 0 {
                if C_GETC_EOF_MODE.with(std::cell::Cell::get) {
                    ui_io::test_set_eof_flag(1);
                    return Ok(0xFF);
                }
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
            }
            Ok(byte[0])
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "game_save: no fileptr or test buffer",
            ))
        }
    })
}

/// Port of `getByte` — raw file byte, no XOR, does not modify `xor_byte`.
pub fn get_byte() -> io::Result<u8> {
    get_byte_raw()
}

/// Port of `wrByte`.
pub fn wr_byte(value: u8) -> io::Result<()> {
    let next = xor_byte() ^ value;
    set_xor_byte(next);
    put_byte(next)
}

/// Port of `rdByte`.
pub fn rd_byte() -> io::Result<u8> {
    let c = get_byte_raw()?;
    let decoded = c ^ xor_byte();
    set_xor_byte(c);
    Ok(decoded)
}

/// Port of `wrBool`.
pub fn wr_bool(value: bool) -> io::Result<()> {
    wr_byte(u8::from(value))
}

/// Port of `rdBool`.
pub fn rd_bool() -> io::Result<bool> {
    Ok(rd_byte()? != 0)
}

/// Port of `wrShort`.
pub fn wr_short(value: u16) -> io::Result<()> {
    wr_byte((value & 0xFF) as u8)?;
    wr_byte(((value >> 8) & 0xFF) as u8)
}

/// Port of `rdShort`.
pub fn rd_short() -> io::Result<u16> {
    let c = get_byte_raw()?;
    let mut decoded = u16::from(c ^ xor_byte());
    set_xor_byte(get_byte_raw()?);
    decoded |= u16::from(c ^ xor_byte()) << 8;
    Ok(decoded)
}

/// Port of `wrLong`.
pub fn wr_long(value: u32) -> io::Result<()> {
    wr_byte((value & 0xFF) as u8)?;
    wr_byte(((value >> 8) & 0xFF) as u8)?;
    wr_byte(((value >> 16) & 0xFF) as u8)?;
    wr_byte(((value >> 24) & 0xFF) as u8)
}

/// Port of `rdLong`.
pub fn rd_long() -> io::Result<u32> {
    let c = get_byte_raw()?;
    let mut decoded = u32::from(c ^ xor_byte());
    set_xor_byte(get_byte_raw()?);
    decoded |= u32::from(c ^ xor_byte()) << 8;

    let c = get_byte_raw()?;
    decoded |= u32::from(c ^ xor_byte()) << 16;
    set_xor_byte(get_byte_raw()?);
    decoded |= u32::from(c ^ xor_byte()) << 24;
    Ok(decoded)
}

/// Port of `wrBytes`.
pub fn wr_bytes(value: &[u8]) -> io::Result<()> {
    for &byte in value {
        wr_byte(byte)?;
    }
    Ok(())
}

/// Port of `rdBytes`.
pub fn rd_bytes(out: &mut [u8]) -> io::Result<()> {
    for byte in out {
        *byte = rd_byte()?;
    }
    Ok(())
}

/// Port of `wrString` — writes through and including the terminating NUL.
pub fn wr_string(bytes: &[u8]) -> io::Result<()> {
    let mut index = 0;
    while index < bytes.len() && bytes[index] != 0 {
        wr_byte(bytes[index])?;
        index += 1;
    }
    wr_byte(0)
}

/// Port of `rdString` — reads through and including the terminating NUL into `out`.
pub fn rd_string(out: &mut [u8]) -> io::Result<()> {
    let mut index = 0;
    loop {
        if index >= out.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "rd_string: buffer overflow before NUL",
            ));
        }
        let decoded = rd_byte()?;
        out[index] = decoded;
        if decoded == 0 {
            break;
        }
        index += 1;
    }
    Ok(())
}

fn rd_string_i8(out: &mut [i8]) -> io::Result<()> {
    let mut scratch = vec![0u8; out.len()];
    rd_string(&mut scratch)?;
    for (slot, byte) in out.iter_mut().zip(scratch) {
        *slot = byte as i8;
    }
    Ok(())
}

/// Port of `wrShorts`.
pub fn wr_shorts(values: &[u16]) -> io::Result<()> {
    for &value in values {
        wr_short(value)?;
    }
    Ok(())
}

/// Port of `rdShorts`.
pub fn rd_shorts(out: &mut [u16]) -> io::Result<()> {
    for slot in out {
        *slot = rd_short()?;
    }
    Ok(())
}

/// Port of `wrItem`.
pub fn wr_item(item: &Inventory) -> io::Result<()> {
    wr_short(item.id)?;
    wr_byte(item.special_name_id)?;
    wr_string(&c_buf(item.inscription))?;
    wr_long(item.flags)?;
    wr_byte(item.category_id)?;
    wr_byte(item.sprite)?;
    wr_short(item.misc_use as u16)?;
    wr_long(item.cost as u32)?;
    wr_byte(item.sub_category_id)?;
    wr_byte(item.items_count)?;
    wr_short(item.weight)?;
    wr_short(item.to_hit as u16)?;
    wr_short(item.to_damage as u16)?;
    wr_short(item.ac as u16)?;
    wr_short(item.to_ac as u16)?;
    wr_byte(item.damage.dice)?;
    wr_byte(item.damage.sides)?;
    wr_byte(item.depth_first_found)?;
    wr_byte(item.identification)?;
    Ok(())
}

/// Port of `rdItem`.
pub fn rd_item(item: &mut Inventory) -> io::Result<()> {
    item.id = rd_short()?;
    item.special_name_id = rd_byte()?;
    rd_string_i8(&mut item.inscription)?;
    item.flags = rd_long()?;
    item.category_id = rd_byte()?;
    item.sprite = rd_byte()?;
    item.misc_use = rd_short()? as i16;
    item.cost = rd_long()? as i32;
    item.sub_category_id = rd_byte()?;
    item.items_count = rd_byte()?;
    item.weight = rd_short()?;
    item.to_hit = rd_short()? as i16;
    item.to_damage = rd_short()? as i16;
    item.ac = rd_short()? as i16;
    item.to_ac = rd_short()? as i16;
    item.damage.dice = rd_byte()?;
    item.damage.sides = rd_byte()?;
    item.depth_first_found = rd_byte()?;
    item.identification = rd_byte()?;
    Ok(())
}

/// Port of `wrMonster`.
pub fn wr_monster(monster: &Monster) -> io::Result<()> {
    wr_short(monster.hp as u16)?;
    wr_short(monster.sleep_count as u16)?;
    wr_short(monster.speed as u16)?;
    wr_short(monster.creature_id)?;
    wr_byte(monster.pos.y as u8)?;
    wr_byte(monster.pos.x as u8)?;
    wr_byte(monster.distance_from_player)?;
    wr_bool(monster.lit)?;
    wr_byte(monster.stunned_amount)?;
    wr_byte(monster.confused_amount)?;
    Ok(())
}

/// Port of `rdMonster`.
pub fn rd_monster(monster: &mut Monster) -> io::Result<()> {
    monster.hp = rd_short()? as i16;
    monster.sleep_count = rd_short()? as i16;
    monster.speed = rd_short()? as i16;
    monster.creature_id = rd_short()?;
    monster.pos.y = i32::from(rd_byte()?);
    monster.pos.x = i32::from(rd_byte()?);
    monster.distance_from_player = rd_byte()?;
    monster.lit = rd_bool()?;
    monster.stunned_amount = rd_byte()?;
    monster.confused_amount = rd_byte()?;
    Ok(())
}

/// Port of `saveHighScore`.
pub fn save_high_score(score: &HighScore) -> io::Result<()> {
    wr_byte(xor_byte())?;
    wr_long(score.points as u32)?;
    wr_long(score.birth_date as u32)?;
    wr_short(score.uid as u16)?;
    wr_short(score.mhp as u16)?;
    wr_short(score.chp as u16)?;
    wr_byte(score.dungeon_depth)?;
    wr_byte(score.level)?;
    wr_byte(score.deepest_dungeon_depth)?;
    wr_byte(score.gender)?;
    wr_byte(score.race)?;
    wr_byte(score.character_class)?;
    wr_bytes(&score.name)?;
    wr_bytes(&score.died_from)?;
    Ok(())
}

/// Port of `readHighScore`.
pub fn read_high_score(score: &mut HighScore) -> io::Result<()> {
    set_xor_byte(get_byte()?);
    score.points = rd_long()? as i32;
    score.birth_date = rd_long()? as i32;
    score.uid = rd_short()? as i16;
    score.mhp = rd_short()? as i16;
    score.chp = rd_short()? as i16;
    score.dungeon_depth = rd_byte()?;
    score.level = rd_byte()?;
    score.deepest_dungeon_depth = rd_byte()?;
    score.gender = rd_byte()?;
    score.race = rd_byte()?;
    score.character_class = rd_byte()?;
    rd_bytes(&mut score.name)?;
    rd_bytes(&mut score.died_from)?;
    Ok(())
}

fn c_buf(bytes: [i8; INSCRIP_SIZE as usize]) -> [u8; INSCRIP_SIZE as usize] {
    bytes.map(|b| b as u8)
}

/// Test hook for option-bitfield construction.
#[doc(hidden)]
pub fn test_build_options_l() -> u32 {
    with_state(build_options_bitfield)
}

/// Test hook for timestamp clamp logic in `svWrite`.
#[doc(hidden)]
pub fn test_compute_save_timestamp() -> u32 {
    let mut l = save_unix_time();
    let start = START_TIME.with(std::cell::Cell::get);
    if l < start {
        l = start.wrapping_add(86_400);
    }
    l
}

fn build_options_bitfield(state: &State) -> u32 {
    let mut l = 0u32;
    if state.options.run_cut_corners {
        l |= 0x1;
    }
    if state.options.run_examine_corners {
        l |= 0x2;
    }
    if state.options.run_print_self {
        l |= 0x4;
    }
    if state.options.find_bound {
        l |= 0x8;
    }
    if state.options.prompt_to_pickup {
        l |= 0x10;
    }
    if state.options.use_roguelike_keys {
        l |= 0x20;
    }
    if state.options.show_inventory_weights {
        l |= 0x40;
    }
    if state.options.highlight_seams {
        l |= 0x80;
    }
    if state.options.run_ignore_doors {
        l |= 0x100;
    }
    if state.options.error_beep_sound {
        l |= 0x200;
    }
    if state.options.display_counts {
        l |= 0x400;
    }
    if state.game.character_is_dead {
        l |= 0x8000_0000;
    }
    if state.game.total_winner {
        l |= 0x4000_0000;
    }
    l
}

fn write_monster_memory(state: &State) -> io::Result<()> {
    for (i, recall) in state.creature_recall.iter().enumerate() {
        if recall.movement != 0
            || recall.defenses != 0
            || recall.kills != 0
            || recall.spells != 0
            || recall.deaths != 0
            || recall.attacks[0] != 0
            || recall.attacks[1] != 0
            || recall.attacks[2] != 0
            || recall.attacks[3] != 0
        {
            wr_short(i as u16)?;
            wr_long(recall.movement)?;
            wr_long(recall.spells)?;
            wr_short(recall.kills)?;
            wr_short(recall.deaths)?;
            wr_short(recall.defenses)?;
            wr_byte(recall.wake)?;
            wr_byte(recall.ignore)?;
            wr_bytes(&recall.attacks)?;
        }
    }
    wr_short(0xFFFF)
}

fn write_player_misc_block(state: &State) -> io::Result<()> {
    let misc = &state.py.misc;
    wr_string(&misc.name)?;
    wr_bool(misc.gender)?;
    wr_long(misc.au as u32)?;
    wr_long(misc.max_exp as u32)?;
    wr_long(misc.exp as u32)?;
    wr_short(misc.exp_fraction)?;
    wr_short(misc.age)?;
    wr_short(misc.height)?;
    wr_short(misc.weight)?;
    wr_short(misc.level)?;
    wr_short(misc.max_dungeon_depth)?;
    wr_short(misc.chance_in_search as u16)?;
    wr_short(misc.fos as u16)?;
    wr_short(misc.bth as u16)?;
    wr_short(misc.bth_with_bows as u16)?;
    wr_short(misc.mana as u16)?;
    wr_short(misc.max_hp as u16)?;
    wr_short(misc.plusses_to_hit as u16)?;
    wr_short(misc.plusses_to_damage as u16)?;
    wr_short(misc.ac as u16)?;
    wr_short(misc.magical_ac as u16)?;
    wr_short(misc.display_to_hit as u16)?;
    wr_short(misc.display_to_damage as u16)?;
    wr_short(misc.display_ac as u16)?;
    wr_short(misc.display_to_ac as u16)?;
    wr_short(misc.disarm as u16)?;
    wr_short(misc.saving_throw as u16)?;
    wr_short(misc.social_class as u16)?;
    wr_short(misc.stealth_factor as u16)?;
    wr_byte(misc.class_id)?;
    wr_byte(misc.race_id)?;
    wr_byte(misc.hit_die)?;
    wr_byte(misc.experience_factor)?;
    wr_short(misc.current_mana as u16)?;
    wr_short(misc.current_mana_fraction)?;
    wr_short(misc.current_hp as u16)?;
    wr_short(misc.current_hp_fraction)?;
    for entry in &misc.history {
        wr_string(entry)?;
    }
    Ok(())
}

fn write_player_stats_block(state: &State) -> io::Result<()> {
    wr_bytes(&state.py.stats.max)?;
    wr_bytes(&state.py.stats.current)?;
    wr_shorts(&state.py.stats.modified.map(|v| v as u16))?;
    wr_bytes(&state.py.stats.used)?;
    Ok(())
}

fn write_player_flags_block(state: &State) -> io::Result<()> {
    let flags = &state.py.flags;
    wr_long(flags.status)?;
    wr_short(flags.rest as u16)?;
    wr_short(flags.blind as u16)?;
    wr_short(flags.paralysis as u16)?;
    wr_short(flags.confused as u16)?;
    wr_short(flags.food as u16)?;
    wr_short(flags.food_digested as u16)?;
    wr_short(flags.protection as u16)?;
    wr_short(flags.speed as u16)?;
    wr_short(flags.fast as u16)?;
    wr_short(flags.slow as u16)?;
    wr_short(flags.afraid as u16)?;
    wr_short(flags.poisoned as u16)?;
    wr_short(flags.image as u16)?;
    wr_short(flags.protect_evil as u16)?;
    wr_short(flags.invulnerability as u16)?;
    wr_short(flags.heroism as u16)?;
    wr_short(flags.super_heroism as u16)?;
    wr_short(flags.blessed as u16)?;
    wr_short(flags.heat_resistance as u16)?;
    wr_short(flags.cold_resistance as u16)?;
    wr_short(flags.detect_invisible as u16)?;
    wr_short(flags.word_of_recall as u16)?;
    wr_short(flags.see_infra as u16)?;
    wr_short(flags.timed_infra as u16)?;
    wr_bool(flags.see_invisible)?;
    wr_bool(flags.teleport)?;
    wr_bool(flags.free_action)?;
    wr_bool(flags.slow_digest)?;
    wr_bool(flags.aggravate)?;
    wr_bool(flags.resistant_to_fire)?;
    wr_bool(flags.resistant_to_cold)?;
    wr_bool(flags.resistant_to_acid)?;
    wr_bool(flags.regenerate_hp)?;
    wr_bool(flags.resistant_to_light)?;
    wr_bool(flags.free_fall)?;
    wr_bool(flags.sustain_str)?;
    wr_bool(flags.sustain_int)?;
    wr_bool(flags.sustain_wis)?;
    wr_bool(flags.sustain_con)?;
    wr_bool(flags.sustain_dex)?;
    wr_bool(flags.sustain_chr)?;
    wr_bool(flags.confuse_monster)?;
    wr_byte(flags.new_spells_to_learn)
}

fn write_inventory_block(state: &State) -> io::Result<()> {
    wr_short(state.missiles_counter as u16)?;
    wr_long(state.dg.game_turn as u32)?;
    wr_short(state.py.pack.unique_items as u16)?;
    for i in 0..state.py.pack.unique_items as usize {
        wr_item(&state.py.inventory[i])?;
    }
    for i in PlayerEquipment::Wield as usize..PLAYER_INVENTORY_SIZE as usize {
        wr_item(&state.py.inventory[i])?;
    }
    wr_short(state.py.pack.weight as u16)?;
    wr_short(state.py.equipment_count as u16)?;
    Ok(())
}

fn write_spells_messages_block(state: &State) -> io::Result<()> {
    let flags = &state.py.flags;
    wr_long(flags.spells_learnt)?;
    wr_long(flags.spells_worked)?;
    wr_long(flags.spells_forgotten)?;
    wr_bytes(&flags.spells_learned_order)?;
    wr_bytes(&state.objects_identified)?;
    wr_long(state.game.magic_seed)?;
    wr_long(state.game.town_seed)?;
    wr_short(state.last_message_id as u16)?;
    for message in &state.messages {
        wr_string(message)?;
    }
    Ok(())
}

fn write_stores_block(state: &State) -> io::Result<()> {
    for store in &state.stores {
        wr_long(store.turns_left_before_closing as u32)?;
        wr_short(store.insults_counter as u16)?;
        wr_byte(store.owner_id)?;
        wr_byte(store.unique_items_counter)?;
        wr_short(store.good_purchases)?;
        wr_short(store.bad_purchases)?;
        for j in 0..store.unique_items_counter as usize {
            wr_long(store.inventory[j].cost as u32)?;
            wr_item(&store.inventory[j].item)?;
        }
    }
    Ok(())
}

fn write_timestamp_and_footer(state: &State) -> io::Result<()> {
    wr_long(test_compute_save_timestamp())?;
    wr_string(&state.game.character_died_from)?;
    wr_long(player_calculate_total_points_for_state(state) as u32)?;
    wr_long(state.py.misc.date_of_birth as u32)
}

fn write_creature_sparse_list(state: &State) -> io::Result<()> {
    for i in 0..MAX_HEIGHT as usize {
        for j in 0..MAX_WIDTH as usize {
            let creature_id = state.dg.floor[i][j].creature_id;
            if creature_id != 0 {
                wr_byte(i as u8)?;
                wr_byte(j as u8)?;
                wr_byte(creature_id)?;
            }
        }
    }
    wr_byte(0xFF)
}

fn write_treasure_sparse_list(state: &State) -> io::Result<()> {
    for i in 0..MAX_HEIGHT as usize {
        for j in 0..MAX_WIDTH as usize {
            let treasure_id = state.dg.floor[i][j].treasure_id;
            if treasure_id != 0 {
                wr_byte(i as u8)?;
                wr_byte(j as u8)?;
                wr_byte(treasure_id)?;
            }
        }
    }
    wr_byte(0xFF)
}

fn write_rle_cave(state: &State) -> io::Result<()> {
    let mut count = 0u8;
    let mut prev_char = 0u8;
    for row in &state.dg.floor {
        for tile in row {
            let char_tmp = tile.feature_id
                | ((tile.perma_lit_room as u8) << 4)
                | ((tile.field_mark as u8) << 5)
                | ((tile.permanent_light as u8) << 6)
                | ((tile.temporary_light as u8) << 7);
            if char_tmp != prev_char || count == UCHAR_MAX {
                wr_byte(count)?;
                wr_byte(prev_char)?;
                prev_char = char_tmp;
                count = 1;
            } else {
                count = count.wrapping_add(1);
            }
        }
    }
    wr_byte(count)?;
    wr_byte(prev_char)
}

fn write_level_block(state: &State) -> io::Result<()> {
    wr_short(state.dg.current_level as u16)?;
    wr_short(state.py.pos.y as u16)?;
    wr_short(state.py.pos.x as u16)?;
    wr_short(state.monster_multiply_total as u16)?;
    wr_short(state.dg.height as u16)?;
    wr_short(state.dg.width as u16)?;
    wr_short(state.dg.panel.max_rows as u16)?;
    wr_short(state.dg.panel.max_cols as u16)?;
    write_creature_sparse_list(state)?;
    write_treasure_sparse_list(state)?;
    write_rle_cave(state)?;
    wr_short(state.game.treasure.current_id as u16)?;
    for i in i32::from(MIN_TREASURE_LIST_ID)..i32::from(state.game.treasure.current_id) {
        wr_item(&state.game.treasure.list[i as usize])?;
    }
    wr_short(state.next_free_monster_id as u16)?;
    for i in i32::from(MON_MIN_INDEX_ID)..i32::from(state.next_free_monster_id) {
        wr_monster(&state.monsters[i as usize])?;
    }
    Ok(())
}

/// Port of `svWrite`.
pub fn sv_write() -> bool {
    with_state_mut(sv_write_state).unwrap_or(false)
}

/// Same as [`sv_write`] but returns I/O errors for tests.
#[doc(hidden)]
pub fn sv_write_result() -> io::Result<bool> {
    with_state_mut(sv_write_state)
}

fn sv_write_state(state: &mut State) -> io::Result<bool> {
    if eof_flag() != 0 {
        state.game.character_is_dead = false;
    }

    write_monster_memory(state)?;
    wr_long(build_options_bitfield(state))?;
    write_player_misc_block(state)?;
    write_player_stats_block(state)?;
    write_player_flags_block(state)?;
    write_inventory_block(state)?;
    write_spells_messages_block(state)?;
    wr_short(u16::from(panic_save()))?;
    wr_short(u16::from(state.game.total_winner))?;
    wr_short(state.game.noscore as u16)?;
    wr_shorts(&state.py.base_hp_levels)?;
    write_stores_block(state)?;
    write_timestamp_and_footer(state)?;

    if state.game.character_is_dead {
        return Ok(flush_ok());
    }

    write_level_block(state)?;
    Ok(flush_ok())
}

fn write_save_header() -> io::Result<u8> {
    set_xor_byte(0);
    wr_byte(CURRENT_VERSION_MAJOR)?;
    set_xor_byte(0);
    wr_byte(CURRENT_VERSION_MINOR)?;
    set_xor_byte(0);
    wr_byte(CURRENT_VERSION_PATCH)?;
    set_xor_byte(0);
    let char_tmp = FORCED_SEED_BYTE
        .with(std::cell::Cell::get)
        .unwrap_or_else(|| (random_number(256) - 1) as u8);
    wr_byte(char_tmp)?;
    Ok(char_tmp)
}

fn open_save_file(filename: &str) -> io::Result<(Option<i32>, bool)> {
    if TEST_FORCE_SAVE_CHAR_FAIL.with(std::cell::Cell::get) {
        return Ok((None, false));
    }

    if test_buffer_active() {
        return Ok((Some(0), true));
    }

    let path = Path::new(filename);
    let mut fd: Option<i32> = None;

    let mut create_opts = OpenOptions::new();
    create_opts.read(true).write(true).create_new(true);
    #[cfg(unix)]
    create_opts.mode(0o600);

    match create_opts.open(path) {
        Ok(file) => {
            drop(file);
            fd = Some(0);
        }
        Err(_) if path.exists() => {
            let overwrite = from_save_file() != 0
                || (with_state(|state| state.game.wizard_mode)
                    && terminal::get_input_confirmation(
                        "Can't make new save file. Overwrite old?",
                    ));
            if overwrite {
                #[cfg(unix)]
                {
                    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
                }
                let mut overwrite_opts = OpenOptions::new();
                overwrite_opts
                    .read(true)
                    .write(true)
                    .truncate(true)
                    .create(true);
                #[cfg(unix)]
                overwrite_opts.mode(0o600);
                overwrite_opts.open(path)?;
                fd = Some(0);
            }
        }
        Err(err) => return Err(err),
    }

    if fd.is_some() {
        let save_path = with_state(|state| state.config_save_game.clone());
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&save_path)?;
        set_fileptr(file);
    }

    Ok((fd, fd.is_some() && FILEPTR.with(|fp| fp.borrow().is_some())))
}

/// Port of `saveChar`.
pub fn save_char(filename: &str) -> bool {
    if with_state(|state| state.game.character_saved) {
        return true;
    }

    terminal::put_qio();
    player_disturb(1, 0);
    with_state_mut(|state| {
        let delta = -i32::from(state.py.pack.heaviness);
        state.py.flags.speed += delta as i16;
        state.py.flags.status |= crate::config::player::status::PY_SPEED;
        for i in (i32::from(MON_MIN_INDEX_ID)..i32::from(state.next_free_monster_id)).rev() {
            state.monsters[i as usize].speed += delta as i16;
        }
        state.py.pack.heaviness = 0;
    });

    let buffer_mode = test_buffer_active();
    let mut ok = false;

    let fd_opened = if buffer_mode {
        true
    } else {
        match open_save_file(filename) {
            Ok((Some(_) | None, true)) => true,
            Ok((_, false)) => {
                terminal::print_message(Some(&format!("Can't create new file '{filename}'")));
                return false;
            }
            Err(_) => {
                terminal::print_message(Some(&format!("Can't create new file '{filename}'")));
                return false;
            }
        }
    };

    if fd_opened && (buffer_mode || FILEPTR.with(|fp| fp.borrow().is_some())) {
        if write_save_header().is_ok() {
            ok = sv_write();
            if TEST_SAVE_FAIL_FLUSH.with(std::cell::Cell::get) {
                ok = false;
            }
        }
        if !buffer_mode {
            FILEPTR.with(|fp| *fp.borrow_mut() = None);
        }
    }

    if !ok {
        if fd_opened && !buffer_mode {
            let _ = fs::remove_file(filename);
            terminal::print_message(Some(&format!("Error writing to file '{filename}'")));
        }
        return false;
    }

    with_state_mut(|state| {
        state.game.character_saved = true;
        state.dg.game_turn = -1;
    });
    true
}

/// Port of `saveGame`.
pub fn save_game() -> bool {
    while !save_char(&with_state(|state| state.config_save_game.clone())) {
        let save_name = with_state(|state| state.config_save_game.clone());
        terminal::print_message(Some(&format!("Save file '{save_name}' fails.")));

        let path = Path::new(&save_name);
        let mut unlink_result = 0i32;
        if !path.exists()
            || !terminal::get_input_confirmation("File exists. Delete old save file?")
            || {
                unlink_result = if fs::remove_file(path).is_ok() { 0 } else { -1 };
                unlink_result < 0
            }
        {
            if unlink_result < 0 {
                terminal::print_message(Some(&format!("Can't delete '{save_name}'")));
            }
            terminal::put_string_clear_to_eol(
                "New Save file [ESC to give up]:",
                terminal::Coord { y: 0, x: 0 },
            );
            let mut input = [0u8; 80];
            if !terminal::get_string_input(&mut input, terminal::Coord { y: 0, x: 31 }, 45) {
                return false;
            }
            if input[0] != 0 {
                with_state_mut(|state| state.config_save_game = c_string(&input));
            }
        }
        let retry_name = with_state(|state| state.config_save_game.clone());
        terminal::put_string_clear_to_eol(
            &format!("Saving with '{retry_name}'..."),
            terminal::Coord { y: 0, x: 0 },
        );
    }
    true
}

fn c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
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

fn decode_options_from_l(state: &mut State, l: u32) {
    state.options.run_cut_corners = (l & 0x1) != 0;
    state.options.run_examine_corners = (l & 0x2) != 0;
    state.options.run_print_self = (l & 0x4) != 0;
    state.options.find_bound = (l & 0x8) != 0;
    state.options.prompt_to_pickup = (l & 0x10) != 0;
    state.options.use_roguelike_keys = (l & 0x20) != 0;
    state.options.show_inventory_weights = (l & 0x40) != 0;
    state.options.highlight_seams = (l & 0x80) != 0;
    state.options.run_ignore_doors = (l & 0x100) != 0;
    state.options.error_beep_sound = (l & 0x200) != 0;
    state.options.display_counts = (l & 0x400) != 0;
}

/// Test hook: apply save-file option bitfield to `State.options`.
#[doc(hidden)]
pub fn test_apply_options_from_l(l: u32) {
    with_state_mut(|state| decode_options_from_l(state, l));
}

fn read_save_header() -> io::Result<(u8, u8, u8)> {
    set_xor_byte(0);
    let version_maj = rd_byte()?;
    set_xor_byte(0);
    let version_min = rd_byte()?;
    set_xor_byte(0);
    let patch_level = rd_byte()?;
    set_xor_byte(get_byte()?);
    Ok((version_maj, version_min, patch_level))
}

fn read_monster_memory(state: &mut State) -> io::Result<()> {
    let mut index = rd_short()?;
    while index != 0xFFFF {
        if index >= MON_MAX_CREATURES {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "recall index"));
        }
        let slot = index as usize;
        let recall = &mut state.creature_recall[slot];
        recall.movement = rd_long()?;
        recall.spells = rd_long()?;
        recall.kills = rd_short()?;
        recall.deaths = rd_short()?;
        recall.defenses = rd_short()?;
        recall.wake = rd_byte()?;
        recall.ignore = rd_byte()?;
        rd_bytes(&mut recall.attacks)?;
        index = rd_short()?;
    }
    Ok(())
}

fn read_player_body(state: &mut State, time_saved: &mut u32) -> io::Result<()> {
    rd_string(&mut state.py.misc.name)?;
    state.py.misc.gender = rd_bool()?;
    state.py.misc.au = rd_long()? as i32;
    state.py.misc.max_exp = rd_long()? as i32;
    state.py.misc.exp = rd_long()? as i32;
    state.py.misc.exp_fraction = rd_short()?;
    state.py.misc.age = rd_short()?;
    state.py.misc.height = rd_short()?;
    state.py.misc.weight = rd_short()?;
    state.py.misc.level = rd_short()?;
    state.py.misc.max_dungeon_depth = rd_short()?;
    state.py.misc.chance_in_search = rd_short()? as i16;
    state.py.misc.fos = rd_short()? as i16;
    state.py.misc.bth = rd_short()? as i16;
    state.py.misc.bth_with_bows = rd_short()? as i16;
    state.py.misc.mana = rd_short()? as i16;
    state.py.misc.max_hp = rd_short()? as i16;
    state.py.misc.plusses_to_hit = rd_short()? as i16;
    state.py.misc.plusses_to_damage = rd_short()? as i16;
    state.py.misc.ac = rd_short()? as i16;
    state.py.misc.magical_ac = rd_short()? as i16;
    state.py.misc.display_to_hit = rd_short()? as i16;
    state.py.misc.display_to_damage = rd_short()? as i16;
    state.py.misc.display_ac = rd_short()? as i16;
    state.py.misc.display_to_ac = rd_short()? as i16;
    state.py.misc.disarm = rd_short()? as i16;
    state.py.misc.saving_throw = rd_short()? as i16;
    state.py.misc.social_class = rd_short()? as i16;
    state.py.misc.stealth_factor = rd_short()? as i16;
    state.py.misc.class_id = rd_byte()?;
    state.py.misc.race_id = rd_byte()?;
    state.py.misc.hit_die = rd_byte()?;
    state.py.misc.experience_factor = rd_byte()?;
    state.py.misc.current_mana = rd_short()? as i16;
    state.py.misc.current_mana_fraction = rd_short()?;
    state.py.misc.current_hp = rd_short()? as i16;
    state.py.misc.current_hp_fraction = rd_short()?;
    for entry in &mut state.py.misc.history {
        rd_string(entry)?;
    }

    rd_bytes(&mut state.py.stats.max)?;
    rd_bytes(&mut state.py.stats.current)?;
    let mut modified = [0u16; 6];
    rd_shorts(&mut modified)?;
    for (slot, value) in state.py.stats.modified.iter_mut().zip(modified) {
        *slot = value as i16;
    }
    rd_bytes(&mut state.py.stats.used)?;

    state.py.flags.status = rd_long()?;
    state.py.flags.rest = rd_short()? as i16;
    state.py.flags.blind = rd_short()? as i16;
    state.py.flags.paralysis = rd_short()? as i16;
    state.py.flags.confused = rd_short()? as i16;
    state.py.flags.food = rd_short()? as i16;
    state.py.flags.food_digested = rd_short()? as i16;
    state.py.flags.protection = rd_short()? as i16;
    state.py.flags.speed = rd_short()? as i16;
    state.py.flags.fast = rd_short()? as i16;
    state.py.flags.slow = rd_short()? as i16;
    state.py.flags.afraid = rd_short()? as i16;
    state.py.flags.poisoned = rd_short()? as i16;
    state.py.flags.image = rd_short()? as i16;
    state.py.flags.protect_evil = rd_short()? as i16;
    state.py.flags.invulnerability = rd_short()? as i16;
    state.py.flags.heroism = rd_short()? as i16;
    state.py.flags.super_heroism = rd_short()? as i16;
    state.py.flags.blessed = rd_short()? as i16;
    state.py.flags.heat_resistance = rd_short()? as i16;
    state.py.flags.cold_resistance = rd_short()? as i16;
    state.py.flags.detect_invisible = rd_short()? as i16;
    state.py.flags.word_of_recall = rd_short()? as i16;
    state.py.flags.see_infra = rd_short()? as i16;
    state.py.flags.timed_infra = rd_short()? as i16;
    state.py.flags.see_invisible = rd_bool()?;
    state.py.flags.teleport = rd_bool()?;
    state.py.flags.free_action = rd_bool()?;
    state.py.flags.slow_digest = rd_bool()?;
    state.py.flags.aggravate = rd_bool()?;
    state.py.flags.resistant_to_fire = rd_bool()?;
    state.py.flags.resistant_to_cold = rd_bool()?;
    state.py.flags.resistant_to_acid = rd_bool()?;
    state.py.flags.regenerate_hp = rd_bool()?;
    state.py.flags.resistant_to_light = rd_bool()?;
    state.py.flags.free_fall = rd_bool()?;
    state.py.flags.sustain_str = rd_bool()?;
    state.py.flags.sustain_int = rd_bool()?;
    state.py.flags.sustain_wis = rd_bool()?;
    state.py.flags.sustain_con = rd_bool()?;
    state.py.flags.sustain_dex = rd_bool()?;
    state.py.flags.sustain_chr = rd_bool()?;
    state.py.flags.confuse_monster = rd_bool()?;
    state.py.flags.new_spells_to_learn = rd_byte()?;

    state.missiles_counter = rd_short()? as i16;
    state.dg.game_turn = rd_long()? as i32;
    state.py.pack.unique_items = rd_short()? as i16;
    if state.py.pack.unique_items > PlayerEquipment::Wield as i16 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "unique_items"));
    }
    for i in 0..state.py.pack.unique_items as usize {
        rd_item(&mut state.py.inventory[i])?;
    }
    for i in PlayerEquipment::Wield as usize..PLAYER_INVENTORY_SIZE as usize {
        rd_item(&mut state.py.inventory[i])?;
    }
    state.py.pack.weight = rd_short()? as i16;
    state.py.equipment_count = rd_short()? as i16;
    state.py.flags.spells_learnt = rd_long()?;
    state.py.flags.spells_worked = rd_long()?;
    state.py.flags.spells_forgotten = rd_long()?;
    rd_bytes(&mut state.py.flags.spells_learned_order)?;
    rd_bytes(&mut state.objects_identified)?;
    state.game.magic_seed = rd_long()?;
    state.game.town_seed = rd_long()?;
    state.last_message_id = rd_short()? as i16;
    for message in &mut state.messages {
        rd_string(message)?;
    }

    let panic_save_short = rd_short()?;
    let total_winner_short = rd_short()?;
    ui_io::test_set_panic_save(panic_save_short != 0);
    state.game.total_winner = total_winner_short != 0;
    state.game.noscore = rd_short()? as i16;
    rd_shorts(&mut state.py.base_hp_levels)?;

    for store in &mut state.stores {
        store.turns_left_before_closing = rd_long()? as i32;
        store.insults_counter = rd_short()? as i16;
        store.owner_id = rd_byte()?;
        store.unique_items_counter = rd_byte()?;
        store.good_purchases = rd_short()?;
        store.bad_purchases = rd_short()?;
        if store.unique_items_counter > STORE_MAX_DISCRETE_ITEMS {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "store items"));
        }
        for j in 0..store.unique_items_counter as usize {
            store.inventory[j].cost = rd_long()? as i32;
            rd_item(&mut store.inventory[j].item)?;
        }
    }

    *time_saved = rd_long()?;
    rd_string(&mut state.game.character_died_from)?;
    state.py.max_score = rd_long()? as i32;
    state.py.misc.date_of_birth = rd_long()? as i32;
    Ok(())
}

fn read_creature_sparse_list(state: &mut State) -> io::Result<()> {
    let mut char_tmp = rd_byte()?;
    while char_tmp != 0xFF {
        let ychar = char_tmp;
        let xchar = rd_byte()?;
        char_tmp = rd_byte()?;
        if xchar > MAX_WIDTH || ychar > MAX_HEIGHT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "creature coords",
            ));
        }
        state.dg.floor[ychar as usize][xchar as usize].creature_id = char_tmp;
        char_tmp = rd_byte()?;
    }
    Ok(())
}

fn read_treasure_sparse_list(state: &mut State) -> io::Result<()> {
    let mut char_tmp = rd_byte()?;
    while char_tmp != 0xFF {
        let ychar = char_tmp;
        let xchar = rd_byte()?;
        char_tmp = rd_byte()?;
        if xchar > MAX_WIDTH || ychar > MAX_HEIGHT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "treasure coords",
            ));
        }
        state.dg.floor[ychar as usize][xchar as usize].treasure_id = char_tmp;
        char_tmp = rd_byte()?;
    }
    Ok(())
}

fn read_rle_cave(state: &mut State) -> io::Result<()> {
    let total_tiles = MAX_HEIGHT as usize * MAX_WIDTH as usize;
    let mut total_count = 0usize;
    while total_count != total_tiles {
        let count = rd_byte()? as usize;
        let char_tmp = rd_byte()?;
        if total_count + count > total_tiles {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "rle overrun"));
        }
        for _ in 0..count {
            let y = total_count / MAX_WIDTH as usize;
            let x = total_count % MAX_WIDTH as usize;
            let tile = &mut state.dg.floor[y][x];
            tile.feature_id = char_tmp & 0xF;
            tile.perma_lit_room = ((char_tmp >> 4) & 0x1) != 0;
            tile.field_mark = ((char_tmp >> 5) & 0x1) != 0;
            tile.permanent_light = ((char_tmp >> 6) & 0x1) != 0;
            tile.temporary_light = ((char_tmp >> 7) & 0x1) != 0;
            total_count += 1;
        }
    }
    Ok(())
}

fn read_level_block(state: &mut State) -> io::Result<()> {
    state.dg.current_level = rd_short()? as i16;
    state.py.pos.y = i32::from(rd_short()?);
    state.py.pos.x = i32::from(rd_short()?);
    state.monster_multiply_total = rd_short()? as i16;
    state.dg.height = rd_short()? as i16;
    state.dg.width = rd_short()? as i16;
    state.dg.panel.max_rows = rd_short()? as i16;
    state.dg.panel.max_cols = rd_short()? as i16;
    read_creature_sparse_list(state)?;
    read_treasure_sparse_list(state)?;
    read_rle_cave(state)?;

    state.game.treasure.current_id = rd_short()? as i16;
    if state.game.treasure.current_id > i16::from(LEVEL_MAX_OBJECTS) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "treasure id"));
    }
    for i in i32::from(MIN_TREASURE_LIST_ID)..i32::from(state.game.treasure.current_id) {
        rd_item(&mut state.game.treasure.list[i as usize])?;
    }
    state.next_free_monster_id = rd_short()? as i16;
    if state.next_free_monster_id > i16::from(MON_TOTAL_ALLOCATIONS) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "monster id"));
    }
    for i in i32::from(MON_MIN_INDEX_ID)..i32::from(state.next_free_monster_id) {
        rd_monster(&mut state.monsters[i as usize])?;
    }
    Ok(())
}

struct RestoreOutcome {
    ok: bool,
    version_maj: u8,
    version_min: u8,
    patch_level: u8,
    time_saved: u32,
    early_close: bool,
}

fn load_game_restore(generate: &mut bool) -> RestoreOutcome {
    let mut outcome = RestoreOutcome {
        ok: true,
        version_maj: 0,
        version_min: 0,
        patch_level: 0,
        time_saved: 0,
        early_close: false,
    };

    terminal::put_string_clear_to_eol("Restoring Memory...", terminal::Coord { y: 0, x: 0 });
    terminal::put_qio();

    let Ok(header) = read_save_header() else {
        outcome.ok = false;
        return outcome;
    };
    outcome.version_maj = header.0;
    outcome.version_min = header.1;
    outcome.patch_level = header.2;

    if !valid_game_version(header.0, header.1, header.2) {
        terminal::put_string_clear_to_eol(
            "Sorry. This save file is from a different version of umoria.",
            terminal::Coord { y: 2, x: 0 },
        );
        outcome.ok = false;
        return outcome;
    }

    let Ok(mut l) = with_state_mut(|state| read_monster_memory(state).and_then(|()| rd_long()))
    else {
        outcome.ok = false;
        return outcome;
    };

    with_state_mut(|state| decode_options_from_l(state, l));

    if with_state(|state| state.game.to_be_wizard) && (l & 0x4000_0000) != 0 {
        terminal::print_message(Some("Sorry, this character is retired from moria."));
        terminal::print_message(Some("You can not resurrect a retired character."));
    } else if with_state(|state| state.game.to_be_wizard)
        && (l & 0x8000_0000) != 0
        && terminal::get_input_confirmation("Resurrect a dead character?")
    {
        l &= !0x8000_0000;
    }

    if (l & 0x8000_0000) == 0
        && with_state_mut(|state| read_player_body(state, &mut outcome.time_saved)).is_err()
    {
        outcome.ok = false;
        return outcome;
    }

    let peek = match get_byte_raw() {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
            if (l & 0x8000_0000) == 0 {
                if !with_state(|state| state.game.to_be_wizard)
                    || with_state(|state| state.dg.game_turn < 0)
                {
                    outcome.ok = false;
                    return outcome;
                }
                terminal::put_string_clear_to_eol(
                    "Attempting a resurrection!",
                    terminal::Coord { y: 0, x: 0 },
                );
                with_state_mut(|state| {
                    if state.py.misc.current_hp < 0 {
                        state.py.misc.current_hp = 0;
                        state.py.misc.current_hp_fraction = 0;
                    }
                    if state.py.flags.food < 0 {
                        state.py.flags.food = 0;
                    }
                    if state.py.flags.poisoned > 1 {
                        state.py.flags.poisoned = 1;
                    }
                    state.dg.current_level = 0;
                    state.game.character_generated = true;
                    state.game.to_be_wizard = false;
                    state.game.noscore |= 0x1;
                });
            } else {
                terminal::print_message(Some("Restoring Memory of a departed spirit..."));
                with_state_mut(|state| state.dg.game_turn = -1);
            }
            terminal::put_qio();
            outcome.early_close = true;
            return outcome;
        }
        Err(_) => {
            outcome.ok = false;
            return outcome;
        }
    };

    if (l & 0x8000_0000) != 0 {
        if (l & 0x8000_0000) == 0 {
            if !with_state(|state| state.game.to_be_wizard)
                || with_state(|state| state.dg.game_turn < 0)
            {
                outcome.ok = false;
                return outcome;
            }
            terminal::put_string_clear_to_eol(
                "Attempting a resurrection!",
                terminal::Coord { y: 0, x: 0 },
            );
            with_state_mut(|state| {
                if state.py.misc.current_hp < 0 {
                    state.py.misc.current_hp = 0;
                    state.py.misc.current_hp_fraction = 0;
                }
                if state.py.flags.food < 0 {
                    state.py.flags.food = 0;
                }
                if state.py.flags.poisoned > 1 {
                    state.py.flags.poisoned = 1;
                }
                state.dg.current_level = 0;
                state.game.character_generated = true;
                state.game.to_be_wizard = false;
                state.game.noscore |= 0x1;
            });
        } else {
            terminal::print_message(Some("Restoring Memory of a departed spirit..."));
            with_state_mut(|state| state.dg.game_turn = -1);
        }
        terminal::put_qio();
        outcome.early_close = true;
        return outcome;
    }

    unget_byte_raw(peek);

    terminal::put_string_clear_to_eol("Restoring Character...", terminal::Coord { y: 0, x: 0 });
    terminal::put_qio();

    if with_state_mut(read_level_block).is_err() {
        outcome.ok = false;
        return outcome;
    }

    *generate = false;

    if with_state(|state| state.dg.game_turn < 0) {
        outcome.ok = false;
    }

    outcome
}

fn load_game_failure_tail() -> bool {
    with_state_mut(|state| state.dg.game_turn = -1);
    terminal::put_string_clear_to_eol(
        "Please try again without that save file.",
        terminal::Coord { y: 1, x: 0 },
    );
    terminal::print_message(None);
    exit_program();
    false
}

fn close_load_streams() {
    FILEPTR.with(|fp| *fp.borrow_mut() = None);
    UNGET_BYTE.with(|c| c.set(None));
}

fn finish_load_success(
    outcome: &RestoreOutcome,
    version_maj: u8,
    version_min: u8,
    patch_level: u8,
) -> bool {
    set_from_save_file(1);

    if panic_save() {
        terminal::print_message(Some(
            "This game is from a panic save.  Score will not be added to scoreboard.",
        ));
    } else if (i32::from(!with_state(|state| state.game.noscore != 0)) & 0x04) != 0 {
        terminal::print_message(Some(
            "This character is already on the scoreboard; it will not be scored again.",
        ));
        with_state_mut(|state| state.game.noscore |= 0x4);
    }

    if with_state(|state| state.dg.game_turn >= 0) {
        with_state_mut(|state| {
            state.py.weapon_is_heavy = false;
            state.py.pack.heaviness = 0;
        });
        player_strength();

        let start = save_unix_time();
        set_start_time(start);
        let time_saved = outcome.time_saved;
        let mut age = start.saturating_sub(time_saved);
        age = (age + 43_200) / 86_400;
        if age > 10 {
            age = 10;
        }
        for _ in 0..age {
            store_maintenance();
            TEST_STORE_MAINTENANCE_COUNT.with(|c| c.set(c.get() + 1));
        }
    }

    if with_state(|state| state.game.noscore != 0) {
        terminal::print_message(Some(
            "This save file cannot be used to get on the score board.",
        ));
    }
    if valid_game_version(version_maj, version_min, patch_level)
        && !is_current_game_version(version_maj, version_min, patch_level)
    {
        terminal::print_message(Some(&format!(
            "Save file version {version_maj}.{version_min} accepted on game version {CURRENT_VERSION_MAJOR}.{CURRENT_VERSION_MINOR}."
        )));
    }

    with_state(|state| state.dg.game_turn >= 0)
}

/// Port of `loadGame`.
pub fn load_game(generate: &mut bool) -> bool {
    *generate = true;
    let buffer_mode = test_buffer_active();
    let save_path = with_state(|state| state.config_save_game.clone());

    if !buffer_mode && !Path::new(&save_path).exists() {
        terminal::print_message(Some("Save file does not exist."));
        return false;
    }

    terminal::clear_screen();
    terminal::put_string(
        &format!("Save file '{save_path}' present. Attempting restore."),
        terminal::Coord { y: 23, x: 0 },
    );

    if with_state(|state| state.dg.game_turn >= 0) {
        terminal::print_message(Some("IMPOSSIBLE! Attempt to restore while still alive!"));
        return load_game_failure_tail();
    }

    if !buffer_mode {
        let path = Path::new(&save_path);
        let opened = OpenOptions::new().read(true).open(path);
        if opened.is_err() {
            #[cfg(unix)]
            let retry = {
                let chmod_ok = fs::set_permissions(path, fs::Permissions::from_mode(0o400)).is_ok();
                if chmod_ok {
                    OpenOptions::new().read(true).open(path)
                } else {
                    Err(io::Error::new(io::ErrorKind::PermissionDenied, "chmod"))
                }
            };
            #[cfg(not(unix))]
            let retry: io::Result<File> = Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "open failed",
            ));
            if retry.is_err() {
                terminal::print_message(Some("Can't open file for reading."));
                return load_game_failure_tail();
            }
        }
    }

    with_state_mut(|state| state.dg.game_turn = -1);

    if !buffer_mode {
        match OpenOptions::new().read(true).open(&save_path) {
            Ok(file) => set_fileptr(file),
            Err(_) => return load_game_failure_tail(),
        }
    }

    let outcome = load_game_restore(generate);
    let version_maj = outcome.version_maj;
    let version_min = outcome.version_min;
    let patch_level = outcome.patch_level;

    close_load_streams();

    if !outcome.ok {
        terminal::print_message(Some("Error during reading of file."));
        return load_game_failure_tail();
    }

    if !outcome.early_close && with_state(|state| state.dg.game_turn >= 0) {
        with_state_mut(|state| {
            if state.py.misc.current_hp >= 0 {
                copy_cstr(&mut state.game.character_died_from, "(alive and well)");
            }
            state.game.character_generated = true;
        });
    }

    finish_load_success(&outcome, version_maj, version_min, patch_level)
}

/// Decode save bytes into game state (shared with [`load_game`], without post-load aging/UI).
#[doc(hidden)]
pub fn test_load_save_from_bytes(bytes: &[u8]) -> io::Result<()> {
    test_buffer_inject(bytes);
    let mut generate = true;
    with_state_mut(|state| state.dg.game_turn = -1);
    let outcome = load_game_restore(&mut generate);
    close_load_streams();
    if !outcome.ok {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "load decode failed",
        ));
    }
    Ok(())
}

/// Return the byte offset where the level block begins (after player-body peek).
#[doc(hidden)]
pub fn test_player_body_end_offset(bytes: &[u8]) -> io::Result<usize> {
    test_buffer_inject(bytes);
    with_state_mut(|state| state.dg.game_turn = -1);
    let _ = read_save_header()?;
    with_state_mut(read_monster_memory)?;
    let l = rd_long()?;
    if (l & 0x8000_0000) == 0 {
        let mut time_saved = 0u32;
        with_state_mut(|state| read_player_body(state, &mut time_saved))?;
    }
    let offset = TEST_BUFFER.with(|buf| buf.borrow().as_ref().map_or(0, |c| c.position() as usize));
    close_load_streams();
    Ok(offset)
}

/// Re-encode loaded state to bytes (header + body) for golden comparison.
#[doc(hidden)]
pub fn test_resave_to_buffer() -> io::Result<()> {
    with_state_mut(|state| {
        state.game.character_saved = false;
        state.dg.game_turn = 1;
    });
    test_reset_buffer();
    save_char("game.sav")
        .then_some(())
        .ok_or_else(|| io::Error::other("save_char failed"))
}
