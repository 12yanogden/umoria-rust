//! Port of src/dungeon_tile.h — tile type and fval constants.

pub const TILE_NULL_WALL: u8 = 0;
pub const TILE_DARK_FLOOR: u8 = 1;
pub const TILE_LIGHT_FLOOR: u8 = 2;
pub const MAX_CAVE_ROOM: u8 = 2;
pub const TILE_CORR_FLOOR: u8 = 3;
pub const TILE_BLOCKED_FLOOR: u8 = 4;
pub const MAX_CAVE_FLOOR: u8 = 4;

pub const MAX_OPEN_SPACE: u8 = 3;
pub const MIN_CLOSED_SPACE: u8 = 4;

pub const TMP1_WALL: u8 = 8;
pub const TMP2_WALL: u8 = 9;

pub const MIN_CAVE_WALL: u8 = 12;
pub const TILE_GRANITE_WALL: u8 = 12;
pub const TILE_MAGMA_WALL: u8 = 13;
pub const TILE_QUARTZ_WALL: u8 = 14;
pub const TILE_BOUNDARY_WALL: u8 = 15;

/// Port of `Tile_t` in dungeon_tile.h.
#[derive(Clone, Copy, Debug, Default)]
pub struct Tile {
    pub creature_id: u8,
    pub treasure_id: u8,
    pub feature_id: u8,
    pub perma_lit_room: bool,
    pub field_mark: bool,
    pub permanent_light: bool,
    pub temporary_light: bool,
}
