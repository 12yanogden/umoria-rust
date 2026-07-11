//! Types, config, and version constants.
#![allow(
    clippy::assertions_on_constants,
    reason = "constant assertions document table sizes from headers"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::config;
use umoria::types;
use umoria::version;

// --------------------------------------------------------------------------
// 1. Size constants
// --------------------------------------------------------------------------
#[test]
fn moria_message_size_is_80() {
    assert_eq!(types::MORIA_MESSAGE_SIZE, 80);
}

#[test]
fn moria_obj_desc_size_is_160() {
    assert_eq!(types::MORIA_OBJ_DESC_SIZE, 160);
}

#[test]
fn vtype_and_obj_desc_buffer_lengths() {
    let v: types::Vtype_t = [0; types::MORIA_MESSAGE_SIZE];
    assert_eq!(v.len(), 80);
    let o: types::Obj_desc_t = [0; types::MORIA_OBJ_DESC_SIZE as usize];
    assert_eq!(o.len(), 160);
}

// --------------------------------------------------------------------------
// 2. Coord_t shape
// --------------------------------------------------------------------------
#[test]
fn coord_t_field_access_and_round_trip() {
    let c = types::Coord_t { y: -3, x: 7 };
    assert_eq!(c.y, -3);
    assert_eq!(c.x, 7);
    let c2 = types::Coord_t { y: c.y, x: c.x };
    assert_eq!(c2.y, -3);
    assert_eq!(c2.x, 7);
}

// --------------------------------------------------------------------------
// 3. options 11 bool defaults
// --------------------------------------------------------------------------
#[test]
fn options_defaults_match_expected() {
    use config::options::*;
    assert!(display_counts);
    assert!(!find_bound);
    assert!(run_cut_corners);
    assert!(run_examine_corners);
    assert!(!run_ignore_doors);
    assert!(!run_print_self);
    assert!(!highlight_seams);
    assert!(!prompt_to_pickup);
    assert!(!use_roguelike_keys);
    assert!(!show_inventory_weights);
    assert!(error_beep_sound);
}

// --------------------------------------------------------------------------
// 4. File path strings
// --------------------------------------------------------------------------
#[test]
fn files_paths_match_expected() {
    use config::files::*;
    assert_eq!(save_game, "game.sav");
    assert_eq!(scores, "scores.dat");
    assert_eq!(splash_screen, "data/splash.txt");
    assert_eq!(license, "LICENSE");
    assert_eq!(help_roguelike_wizard, "data/rl_help_wizard.txt");
    assert_eq!(death_royal, "data/death_royal.txt");
}

// --------------------------------------------------------------------------
// 5. Dungeon scalars
// --------------------------------------------------------------------------
#[test]
fn dungeon_scalars_match_expected() {
    use config::dungeon::*;
    assert_eq!(DUN_RANDOM_DIR, 9);
    assert_eq!(DUN_DIR_CHANGE, 70);
    assert_eq!(DUN_TUNNELING, 15);
    assert_eq!(DUN_ROOMS_MEAN, 32);
    assert_eq!(DUN_ROOM_DOORS, 25);
    assert_eq!(DUN_TUNNEL_DOORS, 15);
    assert_eq!(DUN_STREAMER_DENSITY, 5);
    assert_eq!(DUN_STREAMER_WIDTH, 2);
    assert_eq!(DUN_MAGMA_STREAMER, 3);
    assert_eq!(DUN_MAGMA_TREASURE, 90);
    assert_eq!(DUN_QUARTZ_STREAMER, 2);
    assert_eq!(DUN_QUARTZ_TREASURE, 40);
    assert_eq!(DUN_UNUSUAL_ROOMS, 300);
}

// --------------------------------------------------------------------------
// 6. Dungeon objects
// --------------------------------------------------------------------------
#[test]
fn dungeon_objects_match_expected() {
    use config::dungeon::objects::*;
    assert_eq!(OBJ_OPEN_DOOR, 367);
    assert_eq!(OBJ_CLOSED_DOOR, 368);
    assert_eq!(OBJ_SECRET_DOOR, 369);
    assert_eq!(OBJ_UP_STAIR, 370);
    assert_eq!(OBJ_DOWN_STAIR, 371);
    assert_eq!(OBJ_STORE_DOOR, 372);
    assert_eq!(OBJ_TRAP_LIST, 378);
    assert_eq!(OBJ_RUBBLE, 396);
    assert_eq!(OBJ_MUSH, 397);
    assert_eq!(OBJ_SCARE_MON, 398);
    assert_eq!(OBJ_GOLD_LIST, 399);
    assert_eq!(OBJ_NOTHING, 417);
    assert_eq!(OBJ_RUINED_CHEST, 418);
    assert_eq!(OBJ_WIZARD, 419);
    assert_eq!(MAX_GOLD_TYPES, 18);
    assert_eq!(MAX_TRAPS, 18);
    assert_eq!(LEVEL_OBJECTS_PER_ROOM, 7);
    assert_eq!(LEVEL_OBJECTS_PER_CORRIDOR, 2);
    assert_eq!(LEVEL_TOTAL_GOLD_AND_GEMS, 2);
}

// --------------------------------------------------------------------------
// 7. Treasure scalars
// --------------------------------------------------------------------------
#[test]
fn treasure_scalars_match_expected() {
    use config::treasure::*;
    assert_eq!(MIN_TREASURE_LIST_ID, 1);
    assert_eq!(TREASURE_CHANCE_OF_GREAT_ITEM, 12);
    assert_eq!(LEVEL_STD_OBJECT_ADJUST, 125);
    assert_eq!(LEVEL_MIN_OBJECT_STD, 7);
    assert_eq!(LEVEL_TOWN_OBJECTS, 7);
    assert_eq!(OBJECT_BASE_MAGIC, 15);
    assert_eq!(OBJECT_MAX_BASE_MAGIC, 70);
    assert_eq!(OBJECT_CHANCE_SPECIAL, 6);
    assert_eq!(OBJECT_CHANCE_CURSED, 13);
    assert_eq!(OBJECT_LAMP_MAX_CAPACITY, 15000);
    assert_eq!(OBJECT_BOLTS_MAX_RANGE, 18);
    assert_eq!(OBJECTS_RUNE_PROTECTION, 3000);
}

// --------------------------------------------------------------------------
// 8. Treasure flags TR_*
// --------------------------------------------------------------------------
#[test]
fn treasure_flags_match_expected() {
    use config::treasure::flags::*;
    assert_eq!(TR_STATS, 0x0000_003F);
    assert_eq!(TR_STR, 0x1);
    assert_eq!(TR_INT, 0x2);
    assert_eq!(TR_WIS, 0x4);
    assert_eq!(TR_DEX, 0x8);
    assert_eq!(TR_CON, 0x10);
    assert_eq!(TR_CHR, 0x20);
    assert_eq!(TR_SEARCH, 0x40);
    assert_eq!(TR_SLOW_DIGEST, 0x80);
    assert_eq!(TR_STEALTH, 0x100);
    assert_eq!(TR_AGGRAVATE, 0x200);
    assert_eq!(TR_TELEPORT, 0x400);
    assert_eq!(TR_REGEN, 0x800);
    assert_eq!(TR_SPEED, 0x1000);
    assert_eq!(TR_EGO_WEAPON, 0x0007_E000);
    assert_eq!(TR_SLAY_DRAGON, 0x2000);
    assert_eq!(TR_SLAY_ANIMAL, 0x4000);
    assert_eq!(TR_SLAY_EVIL, 0x8000);
    assert_eq!(TR_SLAY_UNDEAD, 0x10000);
    assert_eq!(TR_FROST_BRAND, 0x20000);
    assert_eq!(TR_FLAME_TONGUE, 0x40000);
    assert_eq!(TR_RES_FIRE, 0x80000);
    assert_eq!(TR_RES_ACID, 0x100000);
    assert_eq!(TR_RES_COLD, 0x200000);
    assert_eq!(TR_SUST_STAT, 0x400000);
    assert_eq!(TR_FREE_ACT, 0x800000);
    assert_eq!(TR_SEE_INVIS, 0x1000000);
    assert_eq!(TR_RES_LIGHT, 0x2000000);
    assert_eq!(TR_FFALL, 0x4000000);
    assert_eq!(TR_BLIND, 0x8000000);
    assert_eq!(TR_TIMID, 0x10000000);
    assert_eq!(TR_TUNNEL, 0x20000000);
    assert_eq!(TR_INFRA, 0x40000000);
    assert_eq!(TR_CURSED, 0x80000000);
}

// --------------------------------------------------------------------------
// 9. Treasure chests CH_*
// --------------------------------------------------------------------------
#[test]
fn treasure_chests_match_expected() {
    use config::treasure::chests::*;
    assert_eq!(CH_LOCKED, 0x1);
    assert_eq!(CH_TRAPPED, 0x1F0);
    assert_eq!(CH_LOSE_STR, 0x10);
    assert_eq!(CH_POISON, 0x20);
    assert_eq!(CH_PARALYSED, 0x40);
    assert_eq!(CH_EXPLODE, 0x80);
    assert_eq!(CH_SUMMON, 0x100);
}

// --------------------------------------------------------------------------
// 10. Monster scalars + move/spells/defense flags
// --------------------------------------------------------------------------
#[test]
fn monster_scalars_match_expected() {
    use config::monsters::*;
    assert_eq!(MON_CHANCE_OF_NEW, 160);
    assert_eq!(MON_MAX_SIGHT, 20);
    assert_eq!(MON_MAX_SPELL_CAST_DISTANCE, 20);
    assert_eq!(MON_MAX_MULTIPLY_PER_LEVEL, 75);
    assert_eq!(MON_MULTIPLY_ADJUST, 7);
    assert_eq!(MON_CHANCE_OF_NASTY, 50);
    assert_eq!(MON_MIN_PER_LEVEL, 14);
    assert_eq!(MON_MIN_TOWNSFOLK_DAY, 4);
    assert_eq!(MON_MIN_TOWNSFOLK_NIGHT, 8);
    assert_eq!(MON_ENDGAME_MONSTERS, 2);
    assert_eq!(MON_ENDGAME_LEVEL, 50);
    assert_eq!(MON_SUMMONED_LEVEL_ADJUST, 2);
    assert_eq!(MON_PLAYER_EXP_DRAINED_PER_HIT, 2);
    assert_eq!(MON_MIN_INDEX_ID, 2);
    assert_eq!(SCARE_MONSTER, 99);
}

#[test]
fn monster_move_flags_match_expected() {
    use config::monsters::move_flags::*;
    assert_eq!(CM_ALL_MV_FLAGS, 0x3F);
    assert_eq!(CM_ATTACK_ONLY, 0x1);
    assert_eq!(CM_MOVE_NORMAL, 0x2);
    assert_eq!(CM_ONLY_MAGIC, 0x4);
    assert_eq!(CM_RANDOM_MOVE, 0x38);
    assert_eq!(CM_20_RANDOM, 0x8);
    assert_eq!(CM_40_RANDOM, 0x10);
    assert_eq!(CM_75_RANDOM, 0x20);
    assert_eq!(CM_SPECIAL, 0x003F_0000);
    assert_eq!(CM_INVISIBLE, 0x10000);
    assert_eq!(CM_OPEN_DOOR, 0x20000);
    assert_eq!(CM_PHASE, 0x40000);
    assert_eq!(CM_EATS_OTHER, 0x80000);
    assert_eq!(CM_PICKS_UP, 0x100000);
    assert_eq!(CM_MULTIPLY, 0x200000);
    assert_eq!(CM_SMALL_OBJ, 0x800000);
    assert_eq!(CM_CARRY_OBJ, 0x1000000);
    assert_eq!(CM_CARRY_GOLD, 0x2000000);
    assert_eq!(CM_TREASURE, 0x7C00_0000);
    assert_eq!(CM_TR_SHIFT, 26);
    assert_eq!(CM_60_RANDOM, 0x4000000);
    assert_eq!(CM_90_RANDOM, 0x8000000);
    assert_eq!(CM_1D2_OBJ, 0x10000000);
    assert_eq!(CM_2D2_OBJ, 0x20000000);
    assert_eq!(CM_4D2_OBJ, 0x40000000);
    assert_eq!(CM_WIN, 0x80000000);
}

#[test]
fn monster_spell_flags_match_expected() {
    use config::monsters::spells::*;
    assert_eq!(CS_FREQ, 0xF);
    assert_eq!(CS_SPELLS, 0x0001_FFF0);
    assert_eq!(CS_TEL_SHORT, 0x10);
    assert_eq!(CS_TEL_LONG, 0x20);
    assert_eq!(CS_TEL_TO, 0x40);
    assert_eq!(CS_LGHT_WND, 0x80);
    assert_eq!(CS_SER_WND, 0x100);
    assert_eq!(CS_HOLD_PER, 0x200);
    assert_eq!(CS_BLIND, 0x400);
    assert_eq!(CS_CONFUSE, 0x800);
    assert_eq!(CS_FEAR, 0x1000);
    assert_eq!(CS_SUMMON_MON, 0x2000);
    assert_eq!(CS_SUMMON_UND, 0x4000);
    assert_eq!(CS_SLOW_PER, 0x8000);
    assert_eq!(CS_DRAIN_MANA, 0x10000);
    assert_eq!(CS_BREATHE, 0x00F8_0000);
    assert_eq!(CS_BR_LIGHT, 0x80000);
    assert_eq!(CS_BR_GAS, 0x100000);
    assert_eq!(CS_BR_ACID, 0x200000);
    assert_eq!(CS_BR_FROST, 0x400000);
    assert_eq!(CS_BR_FIRE, 0x800000);
}

#[test]
fn monster_defense_flags_match_expected() {
    use config::monsters::defense::*;
    assert_eq!(CD_DRAGON, 0x0001);
    assert_eq!(CD_ANIMAL, 0x0002);
    assert_eq!(CD_EVIL, 0x0004);
    assert_eq!(CD_UNDEAD, 0x0008);
    assert_eq!(CD_WEAKNESS, 0x03F0);
    assert_eq!(CD_FROST, 0x0010);
    assert_eq!(CD_FIRE, 0x0020);
    assert_eq!(CD_POISON, 0x0040);
    assert_eq!(CD_ACID, 0x0080);
    assert_eq!(CD_LIGHT, 0x0100);
    assert_eq!(CD_STONE, 0x0200);
    assert_eq!(CD_NO_SLEEP, 0x1000);
    assert_eq!(CD_INFRA, 0x2000);
    assert_eq!(CD_MAX_HP, 0x4000);
}

// --------------------------------------------------------------------------
// 11. Player, identification, spells, stores
// --------------------------------------------------------------------------
#[test]
fn player_scalars_and_status_match_expected() {
    use config::player::status::*;
    use config::player::*;
    assert_eq!(PLAYER_MAX_EXP, 9_999_999);
    assert_eq!(PLAYER_USE_DEVICE_DIFFICULTY, 3);
    assert_eq!(PLAYER_FOOD_FULL, 10000);
    assert_eq!(PLAYER_FOOD_MAX, 15000);
    assert_eq!(PLAYER_FOOD_FAINT, 300);
    assert_eq!(PLAYER_FOOD_WEAK, 1000);
    assert_eq!(PLAYER_FOOD_ALERT, 2000);
    assert_eq!(PLAYER_REGEN_FAINT, 33);
    assert_eq!(PLAYER_REGEN_WEAK, 98);
    assert_eq!(PLAYER_REGEN_NORMAL, 197);
    assert_eq!(PLAYER_REGEN_HPBASE, 1442);
    assert_eq!(PLAYER_REGEN_MNBASE, 524);
    assert_eq!(PLAYER_WEIGHT_CAP, 130);

    assert_eq!(PY_HUNGRY, 0x1);
    assert_eq!(PY_WEAK, 0x2);
    assert_eq!(PY_BLIND, 0x4);
    assert_eq!(PY_CONFUSED, 0x8);
    assert_eq!(PY_FEAR, 0x10);
    assert_eq!(PY_POISONED, 0x20);
    assert_eq!(PY_FAST, 0x40);
    assert_eq!(PY_SLOW, 0x80);
    assert_eq!(PY_SEARCH, 0x100);
    assert_eq!(PY_REST, 0x200);
    assert_eq!(PY_STUDY, 0x400);
    assert_eq!(PY_INVULN, 0x1000);
    assert_eq!(PY_HERO, 0x2000);
    assert_eq!(PY_SHERO, 0x4000);
    assert_eq!(PY_BLESSED, 0x8000);
    assert_eq!(PY_DET_INV, 0x10000);
    assert_eq!(PY_TIM_INFRA, 0x20000);
    assert_eq!(PY_SPEED, 0x40000);
    assert_eq!(PY_STR_WGT, 0x80000);
    assert_eq!(PY_PARALYSED, 0x100000);
    assert_eq!(PY_REPEAT, 0x200000);
    assert_eq!(PY_ARMOR, 0x400000);
    assert_eq!(PY_STATS, 0x3F00_0000);
    assert_eq!(PY_STR, 0x0100_0000);
    assert_eq!(PY_INT, 0x0200_0000);
    assert_eq!(PY_WIS, 0x0400_0000);
    assert_eq!(PY_DEX, 0x0800_0000);
    assert_eq!(PY_CON, 0x1000_0000);
    assert_eq!(PY_CHR, 0x2000_0000);
    assert_eq!(PY_HP, 0x4000_0000);
    assert_eq!(PY_MANA, 0x8000_0000);
}

#[test]
fn identification_spells_stores_match_expected() {
    use config::identification::*;
    use config::spells::*;
    use config::stores::*;
    assert_eq!(OD_TRIED, 0x1);
    assert_eq!(OD_KNOWN1, 0x2);
    assert_eq!(ID_MAGIK, 0x1);
    assert_eq!(ID_DAMD, 0x2);
    assert_eq!(ID_EMPTY, 0x4);
    assert_eq!(ID_KNOWN2, 0x8);
    assert_eq!(ID_STORE_BOUGHT, 0x10);
    assert_eq!(ID_SHOW_HIT_DAM, 0x20);
    assert_eq!(ID_NO_SHOW_P1, 0x40);
    assert_eq!(ID_SHOW_P1, 0x80);

    assert_eq!(SPELL_TYPE_NONE, 0);
    assert_eq!(SPELL_TYPE_MAGE, 1);
    assert_eq!(SPELL_TYPE_PRIEST, 2);
    assert_eq!(NAME_OFFSET_SPELLS, 0);
    assert_eq!(NAME_OFFSET_PRAYERS, 31);

    assert_eq!(STORE_MAX_AUTO_BUY_ITEMS, 18);
    assert_eq!(STORE_MIN_AUTO_SELL_ITEMS, 10);
    assert_eq!(STORE_STOCK_TURN_AROUND, 9);
}

// --------------------------------------------------------------------------
// 12. Version constants
// --------------------------------------------------------------------------
#[test]
fn version_constants_match_expected() {
    assert_eq!(version::CURRENT_VERSION_MAJOR, 5);
    assert_eq!(version::CURRENT_VERSION_MINOR, 7);
    assert_eq!(version::CURRENT_VERSION_PATCH, 15);
}

// --------------------------------------------------------------------------
// 13. Type-width / signedness compile-time guards
// --------------------------------------------------------------------------
#[test]
fn type_width_compile_time_guards() {
    const _: u8 = config::dungeon::DUN_ROOMS_MEAN;
    const _: u16 = config::dungeon::DUN_UNUSUAL_ROOMS;
    const _: u16 = config::dungeon::objects::OBJ_OPEN_DOOR;
    const _: u32 = config::treasure::flags::TR_CURSED;
    const _: u32 = config::monsters::move_flags::CM_WIN;
    const _: u16 = config::monsters::defense::CD_MAX_HP;
    const _: u32 = config::player::status::PY_MANA;
    const _: i32 = config::player::PLAYER_MAX_EXP;
    const _: u8 = config::identification::ID_SHOW_P1;
    const _: u8 = types::MORIA_MESSAGE_SIZE as u8;
    const _: u8 = types::MORIA_OBJ_DESC_SIZE;
    const _: i32 = {
        let c = types::Coord_t { y: 0, x: 0 };
        c.y
    };
    const _: u8 = version::CURRENT_VERSION_MAJOR;
    const _: u8 = version::CURRENT_VERSION_MINOR;
    const _: u8 = version::CURRENT_VERSION_PATCH;
}
