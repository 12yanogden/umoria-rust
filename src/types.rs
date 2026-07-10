//! Port of src/types.h and shared type aliases — see phase_2 for full layouts.
#![allow(unused_imports)]
//!
//! ## Fixed char-buffer semantics (phase_2.1 decision)
//!
//! C++ `vtype_t` / `obj_desc_t` are fixed-length C string buffers used by string
//! helpers (phase_2.3) and the byte-exact save format (phase_5). They are modeled as
//! `[u8; N]` (not `String`/`&str`) so NUL-terminated C-string semantics, fixed bounds,
//! and save-file layout stay exact. Use `u8` rather than `i8`/`c_char` because the save
//! format and terminal I/O treat these as byte buffers and the sign bit is used as a
//! standout flag in UI (phase_3).
//!
//! ## CNIL (phase_2.1 decision)
//!
//! C++ `constexpr char *CNIL = nullptr` is a null `char*` sentinel. Call sites that
//! passed `CNIL` map to `Option::None`; `CNIL` is provided as a documented convenience.

/// C++ `constexpr uint8_t MORIA_MESSAGE_SIZE` (typed as `usize` for Rust `[T; N]` repeat counts in phase_1.3).
pub const MORIA_MESSAGE_SIZE: usize = 80;
/// Alias for callers that distinguish the C++ `uint8_t` value from the Rust array length.
pub const MORIA_MESSAGE_SIZE_LEN: usize = MORIA_MESSAGE_SIZE;

pub const MESSAGE_HISTORY_SIZE: usize = 22;

/// C++ `constexpr uint8_t MORIA_OBJ_DESC_SIZE`.
pub const MORIA_OBJ_DESC_SIZE: u8 = 160;
/// Rust fixed-array repeat count for [`MORIA_OBJ_DESC_SIZE`] (same numeric value).
pub const MORIA_OBJ_DESC_SIZE_LEN: usize = MORIA_OBJ_DESC_SIZE as usize;

/// C++ `constexpr char *CNIL = nullptr` — prefer `None` at call sites.
pub const CNIL: Option<&str> = None;

/// C++ `typedef char vtype_t[MORIA_MESSAGE_SIZE]`.
#[allow(non_camel_case_types)]
pub type Vtype_t = [u8; MORIA_MESSAGE_SIZE];
/// Alias retained for phase_1.3 callers.
pub type Vtype = Vtype_t;

/// C++ `typedef char obj_desc_t[MORIA_OBJ_DESC_SIZE]`.
#[allow(non_camel_case_types)]
pub type Obj_desc_t = [u8; MORIA_OBJ_DESC_SIZE_LEN];
/// Alias matching common Rust naming.
pub type ObjDesc = Obj_desc_t;

// game.h size constants
pub const TREASURE_MAX_LEVELS: u8 = 50;
pub const MAX_OBJECTS_IN_GAME: u16 = 420;
pub const MAX_DUNGEON_OBJECTS: u16 = 344;
pub const OBJECT_IDENT_SIZE: u16 = 448;
pub const LEVEL_MAX_OBJECTS: u8 = 175;
pub const NORMAL_TABLE_SIZE: usize = 256;

// monster.h size constants (re-exported for backward compat)
pub use crate::monster::{
    MON_ATTACK_TYPES, MON_MAX_ATTACKS, MON_MAX_CREATURES, MON_MAX_LEVELS, MON_TOTAL_ALLOCATIONS,
};

// dungeon.h grid size constants
pub use crate::dungeon::{MAX_HEIGHT, MAX_WIDTH};

// store.h size constants
pub use crate::store::{MAX_OWNERS, MAX_STORES, STORE_MAX_ITEM_TYPES};

// player.h size constants
pub use crate::player::{PLAYER_MAX_BACKGROUNDS, PLAYER_MAX_CLASSES, PLAYER_MAX_RACES};

// identification.h — SN_ARRAY_SIZE sentinel (56 enum members 0..55 plus sentinel at 56)
pub use crate::identification::SpecialNameIds;
pub const SN_ARRAY_SIZE: u8 = crate::identification::SpecialNameIds::SN_ARRAY_SIZE as u8;

// identification.h descriptive-array size constants
pub use crate::identification::{
    MAX_AMULETS, MAX_COLORS, MAX_METALS, MAX_MUSHROOMS, MAX_ROCKS, MAX_SYLLABLES, MAX_WOODS,
};

/// Inventory command screen states (game.h).
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

/// C++ `typedef struct { int y; int x; } Coord_t`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub struct Coord_t {
    pub y: i32,
    pub x: i32,
}

/// Alias retained for early Rust callers.
pub type Coord = Coord_t;

// phase_2.4 full layouts — re-exported from module-per-header modules
pub use crate::character::{Background, Class, Race};
pub use crate::dungeon::{Dungeon, DungeonObject};
pub use crate::dungeon_tile::Tile;
pub use crate::inventory::Inventory;
pub use crate::monster::{Creature, Monster, MonsterAttack};
pub use crate::player::Player;
pub use crate::recall::Recall;
pub use crate::store::Store;
pub use crate::ui::Panel;
