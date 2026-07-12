//! Shared type aliases and fixed-size string buffers.
#![allow(unused_imports, reason = "re-exports kept for call-site convenience")]
//!
//! ## Fixed char-buffer semantics
//!
//! `vtype_t` / `obj_desc_t` are fixed-length byte buffers used by string
//! helpers and the save format. They are modeled as `[u8; N]` (not
//! `String`/`&str`) so NUL-terminated semantics, fixed bounds, and
//! save-file layout stay exact. Use `u8` rather than `i8`/`c_char` because
//! the save format and terminal I/O treat these as byte buffers and the
//! sign bit is used as a standout flag in UI.
//!
//! ## CNIL
//!
//! `CNIL` is a null-string sentinel equivalent to `Option::None`.

/// Maximum message / `vtype_t` buffer length.
pub const MORIA_MESSAGE_SIZE: usize = 80;
/// Alias when distinguishing the numeric constant from the array length.
pub const MORIA_MESSAGE_SIZE_LEN: usize = MORIA_MESSAGE_SIZE;

pub const MESSAGE_HISTORY_SIZE: usize = 22;

/// Maximum object-description buffer length.
pub const MORIA_OBJ_DESC_SIZE: u8 = 160;
/// Rust fixed-array repeat count for [`MORIA_OBJ_DESC_SIZE`] (same numeric value).
pub const MORIA_OBJ_DESC_SIZE_LEN: usize = MORIA_OBJ_DESC_SIZE as usize;

/// Null-string sentinel — prefer `None` at call sites.
pub const CNIL: Option<&str> = None;

#[allow(
    non_camel_case_types,
    reason = "historical typedef / enum member names retained"
)]
pub type Vtype_t = [u8; MORIA_MESSAGE_SIZE];
/// Alias for [`Vtype_t`].
pub type Vtype = Vtype_t;

#[allow(
    non_camel_case_types,
    reason = "historical typedef / enum member names retained"
)]
pub type Obj_desc_t = [u8; MORIA_OBJ_DESC_SIZE_LEN];
/// Alias matching common Rust naming.
pub type ObjDesc = Obj_desc_t;

// Game / treasure size constants
pub const TREASURE_MAX_LEVELS: u8 = 50;
pub const MAX_OBJECTS_IN_GAME: u16 = 420;
pub const MAX_DUNGEON_OBJECTS: u16 = 344;
pub const OBJECT_IDENT_SIZE: u16 = 448;
pub const LEVEL_MAX_OBJECTS: u8 = 175;
pub const NORMAL_TABLE_SIZE: usize = 256;

// Monster size constants (re-exported for convenience)
pub use crate::monster::{
    MON_ATTACK_TYPES, MON_MAX_ATTACKS, MON_MAX_CREATURES, MON_MAX_LEVELS, MON_TOTAL_ALLOCATIONS,
};

// Dungeon grid size constants
pub use crate::dungeon::{MAX_HEIGHT, MAX_WIDTH};

// Store size constants
pub use crate::store::{MAX_OWNERS, MAX_STORES, STORE_MAX_ITEM_TYPES};

// Player size constants
pub use crate::player::{PLAYER_MAX_BACKGROUNDS, PLAYER_MAX_CLASSES, PLAYER_MAX_RACES};

// Special-name array sentinel (56 enum members 0..55 plus sentinel at 56)
pub use crate::identification::SpecialNameIds;
pub const SN_ARRAY_SIZE: u8 = crate::identification::SpecialNameIds::SN_ARRAY_SIZE as u8;

// Identification descriptive-array size constants
pub use crate::identification::{
    MAX_AMULETS, MAX_COLORS, MAX_METALS, MAX_MUSHROOMS, MAX_ROCKS, MAX_SYLLABLES, MAX_WOODS,
};

/// Inventory command screen states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Screen {
    #[default]
    Blank = 0,
    Equipment,
    Inventory,
    Wear,
    Help,
    Wrong,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(
    non_camel_case_types,
    reason = "historical typedef / enum member names retained"
)]
pub struct Coord_t {
    pub y: i32,
    pub x: i32,
}

/// Alias for [`Obj_desc_t`].
pub type Coord = Coord_t;

// Re-exported from domain modules
pub use crate::character::{Background, Class, Race};
pub use crate::dungeon::{Dungeon, DungeonObject};
pub use crate::dungeon_tile::Tile;
pub use crate::inventory::Inventory;
pub use crate::monster::{Creature, Monster, MonsterAttack};
pub use crate::player::Player;
pub use crate::recall::Recall;
pub use crate::store::Store;
pub use crate::ui::Panel;
