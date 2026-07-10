//! Port of src/config.cpp / src/config.h — const namespaces and file paths.
//!
//! C++ nested `namespace` names map to nested Rust `mod`s with identical constant names.
//! The C++ `monsters::move` namespace is `move_flags` here because `move` is a Rust keyword.
//!
//! Runtime-mutable C++ globals (`config::options`, `config::files::save_game`) are owned by
//! `game::State` (`phase_1.3`). This module exposes their default values only.

#![allow(
    non_upper_case_globals,
    reason = "C++ config::* names are lowercase snake_case"
)]

pub mod files {
    pub const splash_screen: &str = "data/splash.txt";
    pub const welcome_screen: &str = "data/welcome.txt";
    pub const license: &str = "LICENSE";
    pub const versions_history: &str = "data/versions.txt";
    pub const help: &str = "data/help.txt";
    pub const help_wizard: &str = "data/help_wizard.txt";
    pub const help_roguelike: &str = "data/rl_help.txt";
    pub const help_roguelike_wizard: &str = "data/rl_help_wizard.txt";
    pub const death_tomb: &str = "data/death_tomb.txt";
    pub const death_royal: &str = "data/death_royal.txt";
    pub const scores: &str = "scores.dat";
    /// Default for the runtime-mutable C++ `std::string save_game`.
    pub const save_game: &str = "game.sav";

    // phase_1.3 scaffold aliases (SCREAMING_SNAKE)
    pub const SPLASH_SCREEN: &str = splash_screen;
    pub const WELCOME_SCREEN: &str = welcome_screen;
    pub const LICENSE: &str = license;
    pub const VERSIONS_HISTORY: &str = versions_history;
    pub const HELP: &str = help;
    pub const HELP_WIZARD: &str = help_wizard;
    pub const HELP_ROGUELIKE: &str = help_roguelike;
    pub const HELP_ROGUELIKE_WIZARD: &str = help_roguelike_wizard;
    pub const DEATH_TOMB: &str = death_tomb;
    pub const DEATH_ROYAL: &str = death_royal;
    pub const SCORES: &str = scores;
    pub const SAVE_GAME: &str = save_game;
}

/// Default values for C++ `config::options` (runtime-mutable in game state).
pub mod options {
    pub const display_counts: bool = true;
    pub const find_bound: bool = false;
    pub const run_cut_corners: bool = true;
    pub const run_examine_corners: bool = true;
    pub const run_ignore_doors: bool = false;
    pub const run_print_self: bool = false;
    pub const highlight_seams: bool = false;
    pub const prompt_to_pickup: bool = false;
    pub const use_roguelike_keys: bool = false;
    pub const show_inventory_weights: bool = false;
    pub const error_beep_sound: bool = true;
}

pub mod dungeon {
    pub const DUN_RANDOM_DIR: u8 = 9;
    pub const DUN_DIR_CHANGE: u8 = 70;
    pub const DUN_TUNNELING: u8 = 15;
    pub const DUN_ROOMS_MEAN: u8 = 32;
    pub const DUN_ROOM_DOORS: u8 = 25;
    pub const DUN_TUNNEL_DOORS: u8 = 15;
    pub const DUN_STREAMER_DENSITY: u8 = 5;
    pub const DUN_STREAMER_WIDTH: u8 = 2;
    pub const DUN_MAGMA_STREAMER: u8 = 3;
    pub const DUN_MAGMA_TREASURE: u8 = 90;
    pub const DUN_QUARTZ_STREAMER: u8 = 2;
    pub const DUN_QUARTZ_TREASURE: u8 = 40;
    pub const DUN_UNUSUAL_ROOMS: u16 = 300;

    pub mod objects {
        pub const OBJ_OPEN_DOOR: u16 = 367;
        pub const OBJ_CLOSED_DOOR: u16 = 368;
        pub const OBJ_SECRET_DOOR: u16 = 369;
        pub const OBJ_UP_STAIR: u16 = 370;
        pub const OBJ_DOWN_STAIR: u16 = 371;
        pub const OBJ_STORE_DOOR: u16 = 372;
        pub const OBJ_TRAP_LIST: u16 = 378;
        pub const OBJ_RUBBLE: u16 = 396;
        pub const OBJ_MUSH: u16 = 397;
        pub const OBJ_SCARE_MON: u16 = 398;
        pub const OBJ_GOLD_LIST: u16 = 399;
        pub const OBJ_NOTHING: u16 = 417;
        pub const OBJ_RUINED_CHEST: u16 = 418;
        pub const OBJ_WIZARD: u16 = 419;

        pub const MAX_GOLD_TYPES: u8 = 18;
        pub const MAX_TRAPS: u8 = 18;

        pub const LEVEL_OBJECTS_PER_ROOM: u8 = 7;
        pub const LEVEL_OBJECTS_PER_CORRIDOR: u8 = 2;
        pub const LEVEL_TOTAL_GOLD_AND_GEMS: u8 = 2;
    }
}

pub mod treasure {
    pub const MIN_TREASURE_LIST_ID: u8 = 1;
    pub const TREASURE_CHANCE_OF_GREAT_ITEM: u8 = 12;
    pub const LEVEL_STD_OBJECT_ADJUST: u8 = 125;
    pub const LEVEL_MIN_OBJECT_STD: u8 = 7;
    pub const LEVEL_TOWN_OBJECTS: u8 = 7;
    pub const OBJECT_BASE_MAGIC: u8 = 15;
    pub const OBJECT_MAX_BASE_MAGIC: u8 = 70;
    pub const OBJECT_CHANCE_SPECIAL: u8 = 6;
    pub const OBJECT_CHANCE_CURSED: u8 = 13;
    pub const OBJECT_LAMP_MAX_CAPACITY: u16 = 15_000;
    pub const OBJECT_BOLTS_MAX_RANGE: u8 = 18;
    pub const OBJECTS_RUNE_PROTECTION: u16 = 3_000;

    pub mod flags {
        pub const TR_STATS: u32 = 0x0000_003F;
        pub const TR_STR: u32 = 0x0000_0001;
        pub const TR_INT: u32 = 0x0000_0002;
        pub const TR_WIS: u32 = 0x0000_0004;
        pub const TR_DEX: u32 = 0x0000_0008;
        pub const TR_CON: u32 = 0x0000_0010;
        pub const TR_CHR: u32 = 0x0000_0020;
        pub const TR_SEARCH: u32 = 0x0000_0040;
        pub const TR_SLOW_DIGEST: u32 = 0x0000_0080;
        pub const TR_STEALTH: u32 = 0x0000_0100;
        pub const TR_AGGRAVATE: u32 = 0x0000_0200;
        pub const TR_TELEPORT: u32 = 0x0000_0400;
        pub const TR_REGEN: u32 = 0x0000_0800;
        pub const TR_SPEED: u32 = 0x0000_1000;

        pub const TR_EGO_WEAPON: u32 = 0x0007_E000;
        pub const TR_SLAY_DRAGON: u32 = 0x0000_2000;
        pub const TR_SLAY_ANIMAL: u32 = 0x0000_4000;
        pub const TR_SLAY_EVIL: u32 = 0x0000_8000;
        pub const TR_SLAY_UNDEAD: u32 = 0x0001_0000;
        pub const TR_FROST_BRAND: u32 = 0x0002_0000;
        pub const TR_FLAME_TONGUE: u32 = 0x0004_0000;

        pub const TR_RES_FIRE: u32 = 0x0008_0000;
        pub const TR_RES_ACID: u32 = 0x0010_0000;
        pub const TR_RES_COLD: u32 = 0x0020_0000;
        pub const TR_SUST_STAT: u32 = 0x0040_0000;
        pub const TR_FREE_ACT: u32 = 0x0080_0000;
        pub const TR_SEE_INVIS: u32 = 0x0100_0000;
        pub const TR_RES_LIGHT: u32 = 0x0200_0000;
        pub const TR_FFALL: u32 = 0x0400_0000;
        pub const TR_BLIND: u32 = 0x0800_0000;
        pub const TR_TIMID: u32 = 0x1000_0000;
        pub const TR_TUNNEL: u32 = 0x2000_0000;
        pub const TR_INFRA: u32 = 0x4000_0000;
        pub const TR_CURSED: u32 = 0x8000_0000;
    }

    pub mod chests {
        pub const CH_LOCKED: u32 = 0x0000_0001;
        pub const CH_TRAPPED: u32 = 0x0000_01F0;
        pub const CH_LOSE_STR: u32 = 0x0000_0010;
        pub const CH_POISON: u32 = 0x0000_0020;
        pub const CH_PARALYSED: u32 = 0x0000_0040;
        pub const CH_EXPLODE: u32 = 0x0000_0080;
        pub const CH_SUMMON: u32 = 0x0000_0100;
    }
}

pub mod monsters {
    pub const MON_CHANCE_OF_NEW: u8 = 160;
    pub const MON_MAX_SIGHT: u8 = 20;
    pub const MON_MAX_SPELL_CAST_DISTANCE: u8 = 20;
    pub const MON_MAX_MULTIPLY_PER_LEVEL: u8 = 75;
    pub const MON_MULTIPLY_ADJUST: u8 = 7;
    pub const MON_CHANCE_OF_NASTY: u8 = 50;
    pub const MON_MIN_PER_LEVEL: u8 = 14;
    pub const MON_MIN_TOWNSFOLK_DAY: u8 = 4;
    pub const MON_MIN_TOWNSFOLK_NIGHT: u8 = 8;
    pub const MON_ENDGAME_MONSTERS: u8 = 2;
    pub const MON_ENDGAME_LEVEL: u8 = 50;
    pub const MON_SUMMONED_LEVEL_ADJUST: u8 = 2;
    pub const MON_PLAYER_EXP_DRAINED_PER_HIT: u8 = 2;
    pub const MON_MIN_INDEX_ID: u8 = 2;
    pub const SCARE_MONSTER: u8 = 99;

    /// C++ `config::monsters::move` — `move_flags` avoids the Rust keyword.
    pub mod move_flags {
        pub const CM_ALL_MV_FLAGS: u32 = 0x0000_003F;
        pub const CM_ATTACK_ONLY: u32 = 0x0000_0001;
        pub const CM_MOVE_NORMAL: u32 = 0x0000_0002;
        pub const CM_ONLY_MAGIC: u32 = 0x0000_0004;

        pub const CM_RANDOM_MOVE: u32 = 0x0000_0038;
        pub const CM_20_RANDOM: u32 = 0x0000_0008;
        pub const CM_40_RANDOM: u32 = 0x0000_0010;
        pub const CM_75_RANDOM: u32 = 0x0000_0020;

        pub const CM_SPECIAL: u32 = 0x003F_0000;
        pub const CM_INVISIBLE: u32 = 0x0001_0000;
        pub const CM_OPEN_DOOR: u32 = 0x0002_0000;
        pub const CM_PHASE: u32 = 0x0004_0000;
        pub const CM_EATS_OTHER: u32 = 0x0008_0000;
        pub const CM_PICKS_UP: u32 = 0x0010_0000;
        pub const CM_MULTIPLY: u32 = 0x0020_0000;

        pub const CM_SMALL_OBJ: u32 = 0x0080_0000;
        pub const CM_CARRY_OBJ: u32 = 0x0100_0000;
        pub const CM_CARRY_GOLD: u32 = 0x0200_0000;
        pub const CM_TREASURE: u32 = 0x7C00_0000;
        pub const CM_TR_SHIFT: u32 = 26;
        pub const CM_60_RANDOM: u32 = 0x0400_0000;
        pub const CM_90_RANDOM: u32 = 0x0800_0000;
        pub const CM_1D2_OBJ: u32 = 0x1000_0000;
        pub const CM_2D2_OBJ: u32 = 0x2000_0000;
        pub const CM_4D2_OBJ: u32 = 0x4000_0000;
        pub const CM_WIN: u32 = 0x8000_0000;
    }

    pub mod spells {
        pub const CS_FREQ: u32 = 0x0000_000F;
        pub const CS_SPELLS: u32 = 0x0001_FFF0;
        pub const CS_TEL_SHORT: u32 = 0x0000_0010;
        pub const CS_TEL_LONG: u32 = 0x0000_0020;
        pub const CS_TEL_TO: u32 = 0x0000_0040;
        pub const CS_LGHT_WND: u32 = 0x0000_0080;
        pub const CS_SER_WND: u32 = 0x0000_0100;
        pub const CS_HOLD_PER: u32 = 0x0000_0200;
        pub const CS_BLIND: u32 = 0x0000_0400;
        pub const CS_CONFUSE: u32 = 0x0000_0800;
        pub const CS_FEAR: u32 = 0x0000_1000;
        pub const CS_SUMMON_MON: u32 = 0x0000_2000;
        pub const CS_SUMMON_UND: u32 = 0x0000_4000;
        pub const CS_SLOW_PER: u32 = 0x0000_8000;
        pub const CS_DRAIN_MANA: u32 = 0x0001_0000;

        pub const CS_BREATHE: u32 = 0x00F8_0000;
        pub const CS_BR_LIGHT: u32 = 0x0008_0000;
        pub const CS_BR_GAS: u32 = 0x0010_0000;
        pub const CS_BR_ACID: u32 = 0x0020_0000;
        pub const CS_BR_FROST: u32 = 0x0040_0000;
        pub const CS_BR_FIRE: u32 = 0x0080_0000;
    }

    pub mod defense {
        pub const CD_DRAGON: u16 = 0x0001;
        pub const CD_ANIMAL: u16 = 0x0002;
        pub const CD_EVIL: u16 = 0x0004;
        pub const CD_UNDEAD: u16 = 0x0008;
        pub const CD_WEAKNESS: u16 = 0x03F0;
        pub const CD_FROST: u16 = 0x0010;
        pub const CD_FIRE: u16 = 0x0020;
        pub const CD_POISON: u16 = 0x0040;
        pub const CD_ACID: u16 = 0x0080;
        pub const CD_LIGHT: u16 = 0x0100;
        pub const CD_STONE: u16 = 0x0200;
        pub const CD_NO_SLEEP: u16 = 0x1000;
        pub const CD_INFRA: u16 = 0x2000;
        pub const CD_MAX_HP: u16 = 0x4000;
    }
}

pub mod player {
    pub const PLAYER_MAX_EXP: i32 = 9_999_999;
    pub const PLAYER_USE_DEVICE_DIFFICULTY: u8 = 3;
    pub const PLAYER_FOOD_FULL: u16 = 10_000;
    pub const PLAYER_FOOD_MAX: u16 = 15_000;
    pub const PLAYER_FOOD_FAINT: u16 = 300;
    pub const PLAYER_FOOD_WEAK: u16 = 1_000;
    pub const PLAYER_FOOD_ALERT: u16 = 2_000;
    pub const PLAYER_REGEN_FAINT: u8 = 33;
    pub const PLAYER_REGEN_WEAK: u8 = 98;
    pub const PLAYER_REGEN_NORMAL: u8 = 197;
    pub const PLAYER_REGEN_HPBASE: u16 = 1_442;
    pub const PLAYER_REGEN_MNBASE: u16 = 524;
    pub const PLAYER_WEIGHT_CAP: u8 = 130;

    pub mod status {
        pub const PY_HUNGRY: u32 = 0x0000_0001;
        pub const PY_WEAK: u32 = 0x0000_0002;
        pub const PY_BLIND: u32 = 0x0000_0004;
        pub const PY_CONFUSED: u32 = 0x0000_0008;
        pub const PY_FEAR: u32 = 0x0000_0010;
        pub const PY_POISONED: u32 = 0x0000_0020;
        pub const PY_FAST: u32 = 0x0000_0040;
        pub const PY_SLOW: u32 = 0x0000_0080;
        pub const PY_SEARCH: u32 = 0x0000_0100;
        pub const PY_REST: u32 = 0x0000_0200;
        pub const PY_STUDY: u32 = 0x0000_0400;

        pub const PY_INVULN: u32 = 0x0000_1000;
        pub const PY_HERO: u32 = 0x0000_2000;
        pub const PY_SHERO: u32 = 0x0000_4000;
        pub const PY_BLESSED: u32 = 0x0000_8000;
        pub const PY_DET_INV: u32 = 0x0001_0000;
        pub const PY_TIM_INFRA: u32 = 0x0002_0000;
        pub const PY_SPEED: u32 = 0x0004_0000;
        pub const PY_STR_WGT: u32 = 0x0008_0000;
        pub const PY_PARALYSED: u32 = 0x0010_0000;
        pub const PY_REPEAT: u32 = 0x0020_0000;
        pub const PY_ARMOR: u32 = 0x0040_0000;

        pub const PY_STATS: u32 = 0x3F00_0000;
        pub const PY_STR: u32 = 0x0100_0000;
        pub const PY_INT: u32 = 0x0200_0000;
        pub const PY_WIS: u32 = 0x0400_0000;
        pub const PY_DEX: u32 = 0x0800_0000;
        pub const PY_CON: u32 = 0x1000_0000;
        pub const PY_CHR: u32 = 0x2000_0000;

        pub const PY_HP: u32 = 0x4000_0000;
        pub const PY_MANA: u32 = 0x8000_0000;
    }
}

pub mod identification {
    pub const OD_TRIED: u8 = 0x1;
    pub const OD_KNOWN1: u8 = 0x2;

    pub const ID_MAGIK: u8 = 0x1;
    pub const ID_DAMD: u8 = 0x2;
    pub const ID_EMPTY: u8 = 0x4;
    pub const ID_KNOWN2: u8 = 0x8;
    pub const ID_STORE_BOUGHT: u8 = 0x10;
    pub const ID_SHOW_HIT_DAM: u8 = 0x20;
    pub const ID_NO_SHOW_P1: u8 = 0x40;
    pub const ID_SHOW_P1: u8 = 0x80;
}

pub mod spells {
    pub const SPELL_TYPE_NONE: u8 = 0;
    pub const SPELL_TYPE_MAGE: u8 = 1;
    pub const SPELL_TYPE_PRIEST: u8 = 2;

    pub const NAME_OFFSET_SPELLS: u8 = 0;
    pub const NAME_OFFSET_PRAYERS: u8 = 31;
}

pub mod stores {
    pub const STORE_MAX_AUTO_BUY_ITEMS: u8 = 18;
    pub const STORE_MIN_AUTO_SELL_ITEMS: u8 = 10;
    pub const STORE_STOCK_TURN_AROUND: u8 = 9;
}
