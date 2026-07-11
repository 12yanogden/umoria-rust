//! Player type, enums, and constants

use crate::inventory::{Inventory, PlayerEquipment, PLAYER_INVENTORY_SIZE};
use crate::types::Coord_t;

pub const PLAYER_MAX_LEVEL: u8 = 40;
pub const PLAYER_MAX_CLASSES: u8 = 6;
pub const PLAYER_MAX_RACES: u8 = 8;
pub const PLAYER_MAX_BACKGROUNDS: u8 = 128;

pub const CLASS_MAX_LEVEL_ADJUST: u8 = 5;
pub const CLASS_MISC_HIT: u8 = 4;
pub const BTH_PER_PLUS_TO_HIT_ADJUST: u8 = 3;
pub const PLAYER_NAME_SIZE: u8 = 27;

pub type ClassRankTitle = &'static str;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
#[allow(
    non_camel_case_types,
    clippy::upper_case_acronyms,
    reason = "historical typedef / enum member names retained"
)]
pub enum PlayerClassLevelAdj {
    BTH = 0,
    BTHB,
    DEVICE,
    DISARM,
    SAVE,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
#[allow(
    non_camel_case_types,
    reason = "historical typedef / enum member names retained"
)]
pub enum PlayerAttr {
    A_STR = 0,
    A_INT,
    A_WIS,
    A_DEX,
    A_CON,
    A_CHR,
}

/// Nested `misc` sub-struct of `Player_t`.
#[derive(Clone, Copy, Debug)]
pub struct PlayerMisc {
    pub name: [u8; PLAYER_NAME_SIZE as usize],
    pub gender: bool,
    pub date_of_birth: i32,
    pub au: i32,
    pub max_exp: i32,
    pub exp: i32,
    pub exp_fraction: u16,
    pub age: u16,
    pub height: u16,
    pub weight: u16,
    pub level: u16,
    pub max_dungeon_depth: u16,
    pub chance_in_search: i16,
    pub fos: i16,
    pub bth: i16,
    pub bth_with_bows: i16,
    pub mana: i16,
    pub max_hp: i16,
    pub plusses_to_hit: i16,
    pub plusses_to_damage: i16,
    pub ac: i16,
    pub magical_ac: i16,
    pub display_to_hit: i16,
    pub display_to_damage: i16,
    pub display_ac: i16,
    pub display_to_ac: i16,
    pub disarm: i16,
    pub saving_throw: i16,
    pub social_class: i16,
    pub stealth_factor: i16,
    pub class_id: u8,
    pub race_id: u8,
    pub hit_die: u8,
    pub experience_factor: u8,
    pub current_mana: i16,
    pub current_mana_fraction: u16,
    pub current_hp: i16,
    pub current_hp_fraction: u16,
    pub history: [[u8; 60]; 4],
}

impl Default for PlayerMisc {
    fn default() -> Self {
        Self {
            name: [0; PLAYER_NAME_SIZE as usize],
            gender: false,
            date_of_birth: 0,
            au: 0,
            max_exp: 0,
            exp: 0,
            exp_fraction: 0,
            age: 0,
            height: 0,
            weight: 0,
            level: 0,
            max_dungeon_depth: 0,
            chance_in_search: 0,
            fos: 0,
            bth: 0,
            bth_with_bows: 0,
            mana: 0,
            max_hp: 0,
            plusses_to_hit: 0,
            plusses_to_damage: 0,
            ac: 0,
            magical_ac: 0,
            display_to_hit: 0,
            display_to_damage: 0,
            display_ac: 0,
            display_to_ac: 0,
            disarm: 0,
            saving_throw: 0,
            social_class: 0,
            stealth_factor: 0,
            class_id: 0,
            race_id: 0,
            hit_die: 0,
            experience_factor: 0,
            current_mana: 0,
            current_mana_fraction: 0,
            current_hp: 0,
            current_hp_fraction: 0,
            history: [[0; 60]; 4],
        }
    }
}

/// Nested `stats` sub-struct of `Player_t`.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerStats {
    pub max: [u8; 6],
    pub current: [u8; 6],
    pub modified: [i16; 6],
    pub used: [u8; 6],
}

/// Nested `flags` sub-struct of `Player_t`.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerFlags {
    pub status: u32,
    pub rest: i16,
    pub blind: i16,
    pub paralysis: i16,
    pub confused: i16,
    pub food: i16,
    pub food_digested: i16,
    pub protection: i16,
    pub speed: i16,
    pub fast: i16,
    pub slow: i16,
    pub afraid: i16,
    pub poisoned: i16,
    pub image: i16,
    pub protect_evil: i16,
    pub invulnerability: i16,
    pub heroism: i16,
    pub super_heroism: i16,
    pub blessed: i16,
    pub heat_resistance: i16,
    pub cold_resistance: i16,
    pub detect_invisible: i16,
    pub word_of_recall: i16,
    pub see_infra: i16,
    pub timed_infra: i16,
    pub see_invisible: bool,
    pub teleport: bool,
    pub free_action: bool,
    pub slow_digest: bool,
    pub aggravate: bool,
    pub resistant_to_fire: bool,
    pub resistant_to_cold: bool,
    pub resistant_to_acid: bool,
    pub regenerate_hp: bool,
    pub resistant_to_light: bool,
    pub free_fall: bool,
    pub sustain_str: bool,
    pub sustain_int: bool,
    pub sustain_wis: bool,
    pub sustain_con: bool,
    pub sustain_dex: bool,
    pub sustain_chr: bool,
    pub confuse_monster: bool,
    pub new_spells_to_learn: u8,
    pub spells_learnt: u32,
    pub spells_worked: u32,
    pub spells_forgotten: u32,
    pub spells_learned_order: [u8; 32],
}

/// Nested `pack` sub-struct of `Player_t`.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerPack {
    pub unique_items: i16,
    pub weight: i16,
    pub heaviness: i16,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub misc: PlayerMisc,
    pub stats: PlayerStats,
    pub flags: PlayerFlags,
    pub pos: Coord_t,
    pub prev_dir: u8,
    pub base_hp_levels: [u16; PLAYER_MAX_LEVEL as usize],
    pub base_exp_levels: [u32; PLAYER_MAX_LEVEL as usize],
    pub running_tracker: u8,
    pub temporary_light_only: bool,
    pub max_score: i32,
    pub pack: PlayerPack,
    pub inventory: [Inventory; PLAYER_INVENTORY_SIZE as usize],
    pub equipment_count: i16,
    pub weapon_is_heavy: bool,
    pub carrying_light: bool,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            misc: PlayerMisc::default(),
            stats: PlayerStats::default(),
            flags: PlayerFlags::default(),
            pos: Coord_t::default(),
            prev_dir: b' ',
            base_hp_levels: [0; PLAYER_MAX_LEVEL as usize],
            base_exp_levels: [0; PLAYER_MAX_LEVEL as usize],
            running_tracker: 0,
            temporary_light_only: false,
            max_score: 0,
            pack: PlayerPack::default(),
            inventory: [Inventory::default(); PLAYER_INVENTORY_SIZE as usize],
            equipment_count: 0,
            weapon_is_heavy: false,
            carrying_light: false,
        }
    }
}

use crate::config::dungeon::objects::{OBJ_CLOSED_DOOR, OBJ_NOTHING, OBJ_OPEN_DOOR};
use crate::config::monsters::{self, defense::CD_NO_SLEEP};
use crate::config::player::status::{PY_ARMOR, PY_REST, PY_SEARCH, PY_SPEED, PY_STR_WGT};
use crate::config::player::PLAYER_WEIGHT_CAP;
use crate::config::treasure::chests::{CH_LOCKED, CH_TRAPPED};
use crate::dungeon::{
    coord_distance_between, dungeon_lite_spot, dungeon_move_creature_record, trap_change_visibility,
};
use crate::dungeon_tile::{MIN_CLOSED_SPACE, TILE_BLOCKED_FLOOR, TILE_CORR_FLOOR};
use crate::helpers::string_to_number;
use crate::identification::{
    object_blocked_by_monster, spell_item_identify_and_remove_random_inscription,
    spell_item_identify_and_remove_random_inscription_for_state, SpecialNameIds,
};
use crate::monster::{monster_death, update_monsters};
use crate::player_run::player_end_running;
use crate::treasure::{
    TV_CHEST, TV_CLOSED_DOOR, TV_INVIS_TRAP, TV_NOTHING, TV_OPEN_DOOR, TV_SECRET_DOOR,
};
use crate::types::{Vtype_t, CNIL, MORIA_MESSAGE_SIZE, MORIA_OBJ_DESC_SIZE};
use crate::ui::{dungeon_reset_view, print_character_movement_state, print_character_speed};

const SHRT_MAX: i32 = 32_767;
use crate::config::monsters::move_flags::CM_WIN;
use crate::config::treasure::flags::{
    TR_AGGRAVATE, TR_BLIND, TR_FFALL, TR_FREE_ACT, TR_INFRA, TR_REGEN, TR_RES_ACID, TR_RES_COLD,
    TR_RES_FIRE, TR_RES_LIGHT, TR_SEARCH, TR_SEE_INVIS, TR_SLOW_DIGEST, TR_SPEED, TR_STATS,
    TR_STEALTH, TR_SUST_STAT, TR_TELEPORT, TR_TIMID,
};
use crate::data_creatures::CREATURES_LIST;
use crate::data_player::CLASS_LEVEL_ADJ;
use crate::dice::{dice_roll, Dice};
use crate::game::{random_number, random_number_state, with_state, with_state_mut};
use crate::helpers::is_vowel;
use crate::identification::{item_description, spell_item_identified};
use crate::inventory::{
    inventory_collect_all_item_flags, inventory_item_copy_to, inventory_item_is_cursed,
    inventory_item_remove_curse,
};
use crate::monster::{monster_take_hit, Creature, MON_MAX_LEVELS};
use crate::player_magic::item_magic_ability_damage;
use crate::treasure::{TV_BOW, TV_SLING_AMMO, TV_SPIKE};
use crate::ui::{display_character_experience, print_character_current_hit_points};
use crate::ui_io::get_direction_with_memory;
use crate::ui_io::terminal::print_message;

/// 1094
pub fn player_gain_kill_experience(creature: &Creature) {
    with_state_mut(|state| {
        let exp = i32::from(creature.kill_exp_value) * i32::from(creature.level);
        let level = i32::from(state.py.misc.level);
        let mut quotient = exp / level;
        let mut remainder = exp % level;
        remainder *= 0x1_0000;
        remainder /= level;
        remainder += i32::from(state.py.misc.exp_fraction);
        if remainder >= 0x1_0000 {
            quotient += 1;
            state.py.misc.exp_fraction = (remainder - 0x1_0000) as u16;
        } else {
            state.py.misc.exp_fraction = remainder as u16;
        }
        state.py.misc.exp += quotient;
    });
}

/// 1666
pub fn player_rank_title() -> &'static str {
    with_state(|state| {
        if state.py.misc.level < 1 {
            "Babe in arms"
        } else if state.py.misc.level <= u16::from(PLAYER_MAX_LEVEL) {
            crate::data_player::CLASS_RANK_TITLES[state.py.misc.class_id as usize]
                [state.py.misc.level as usize - 1]
        } else if state.py.misc.gender {
            "**KING**"
        } else {
            "**QUEEN**"
        }
    })
}

pub use crate::player_move::player_move_position;
pub use crate::player_traps::chest_trap;
pub use crate::player_tunnel::player_tunnel_wall;

/// 34
pub fn player_is_male() -> bool {
    with_state(|state| state.py.misc.gender)
}

/// 38
pub fn player_set_gender(is_male: bool) {
    with_state_mut(|state| {
        state.py.misc.gender = is_male;
    });
}

/// 45
pub fn player_get_gender_label() -> &'static str {
    if player_is_male() {
        "Male"
    } else {
        "Female"
    }
}

#[allow(
    unused_imports,
    reason = "re-exports kept for call-site convenience"
)]
pub use crate::player_stats::{
    player_armor_class_adjustment, player_attack_blows, player_calculate_hit_points,
    player_damage_adjustment, player_disarm_adjustment, player_initialize_base_experience_levels,
    player_modify_stat, player_set_and_use_stat, player_stat_adjustment_charisma,
    player_stat_adjustment_constitution, player_stat_adjustment_wisdom_intelligence,
    player_stat_boost, player_stat_random_decrease, player_stat_random_increase,
    player_stat_restore, player_to_hit_adjustment,
};

use crate::config::player::status::{PY_MANA, PY_STUDY};
use crate::config::spells::{NAME_OFFSET_PRAYERS, NAME_OFFSET_SPELLS, SPELL_TYPE_MAGE};
use crate::data_player::{CLASSES, MAGIC_SPELLS, SPELL_NAMES};
use crate::spells::Spell;
use crate::treasure::TV_MAGIC_BOOK;
use crate::ui::display_spells_list;
use crate::ui_io::terminal::{self, Coord};

/// 143
pub fn player_no_light() -> bool {
    with_state(|state| {
        let y = state.py.pos.y as usize;
        let x = state.py.pos.x as usize;
        let tile = &state.dg.floor[y][x];
        !tile.temporary_light && !tile.permanent_light
    })
}

pub use crate::player_magic::{player_bless, player_protect_evil};

/// 815
pub fn player_can_read() -> bool {
    if with_state(|state| state.py.flags.blind > 0) {
        terminal::print_message(Some("You can't see to read your spell book!"));
        return false;
    }

    if player_no_light() {
        terminal::print_message(Some("You have no light to read by."));
        return false;
    }

    true
}

/// 826
pub fn last_known_spell() -> i32 {
    for last_known in 0..32 {
        if with_state(|state| state.py.flags.spells_learned_order[last_known]) == 99 {
            return last_known as i32;
        }
    }
    0
}

/// 838
pub fn player_determine_learnable_spells() -> u32 {
    with_state(|state| {
        let mut spell_flag = 0u32;
        for i in 0..state.py.pack.unique_items {
            if state.py.inventory[i as usize].category_id == TV_MAGIC_BOOK {
                spell_flag |= state.py.inventory[i as usize].flags;
            }
        }
        spell_flag
    })
}

/// 1001
pub fn new_mana(stat: PlayerAttr) -> i32 {
    with_state(|state| {
        let levels = i32::from(state.py.misc.level)
            - i32::from(CLASSES[state.py.misc.class_id as usize].min_level_for_spell_casting)
            + 1;

        match player_stat_adjustment_wisdom_intelligence(stat) {
            1 | 2 => levels,
            3 => 3 * levels / 2,
            4 => 2 * levels,
            5 => 5 * levels / 2,
            6 => 3 * levels,
            7 => 4 * levels,
            _ => 0,
        }
    })
}

/// 1038
pub fn player_gain_mana(stat: PlayerAttr) {
    let mut new_mana_value = new_mana(stat);
    if new_mana_value > 0 {
        new_mana_value += 1;
    }

    with_state_mut(|state| {
        if state.py.flags.spells_learnt != 0 {
            if state.py.misc.mana != new_mana_value as i16 {
                if state.py.misc.mana != 0 {
                    let value = (((i64::from(state.py.misc.current_mana) << 16)
                        + i64::from(state.py.misc.current_mana_fraction))
                        / i64::from(state.py.misc.mana))
                        * i64::from(new_mana_value);
                    state.py.misc.current_mana = (value >> 16) as i16;
                    state.py.misc.current_mana_fraction = (value & 0xFFFF) as u16;
                } else {
                    state.py.misc.current_mana = new_mana_value as i16;
                    state.py.misc.current_mana_fraction = 0;
                }

                state.py.misc.mana = new_mana_value as i16;
                state.py.flags.status |= PY_MANA;
            }
        } else if state.py.misc.mana != 0 {
            state.py.misc.mana = 0;
            state.py.misc.current_mana = 0;
            state.py.flags.status |= PY_MANA;
        }
    });
}

/// 1472
fn eliminate_known_spells_greater_than_level(magic_type_str: &str, offset: i32) {
    let messages = with_state_mut(|state| {
        let class_id = state.py.misc.class_id as usize - 1;
        let level = state.py.misc.level;
        let mut mask = 0x8000_0000u32;
        let mut messages = Vec::new();

        for i in (0..=31).rev() {
            if (mask & state.py.flags.spells_learnt) != 0 {
                if MAGIC_SPELLS[class_id][i].level_required > level as u8 {
                    state.py.flags.spells_learnt &= !mask;
                    state.py.flags.spells_forgotten |= mask;
                    messages.push(format!(
                        "You have forgotten the {magic_type_str} of {}.",
                        SPELL_NAMES[i + offset as usize]
                    ));
                } else {
                    break;
                }
            }
            mask >>= 1;
        }

        messages
    });

    for msg in messages {
        terminal::print_message(Some(&msg));
    }
}

/// 1501
pub fn number_of_spells_allowed(stat: PlayerAttr) -> i32 {
    with_state(|state| {
        let levels = i32::from(state.py.misc.level)
            - i32::from(CLASSES[state.py.misc.class_id as usize].min_level_for_spell_casting)
            + 1;

        match player_stat_adjustment_wisdom_intelligence(stat) {
            1..=3 => levels,
            4 | 5 => 3 * levels / 2,
            6 => 2 * levels,
            7 => 5 * levels / 2,
            _ => 0,
        }
    })
}

/// 1513
pub fn number_of_spells_known() -> i32 {
    with_state(|state| {
        let mut known = 0;
        let mut mask = 1u32;
        while mask != 0 {
            if (mask & state.py.flags.spells_learnt) != 0 {
                known += 1;
            }
            mask <<= 1;
        }
        known
    })
}

/// 1548
fn remember_forgotten_spells(
    allowed_spells: i32,
    mut new_spells: i32,
    magic_type_str: &str,
    offset: i32,
) -> i32 {
    let (new_spells, messages) = with_state_mut(|state| {
        let class_id = state.py.misc.class_id as usize - 1;
        let level = state.py.misc.level;
        let mut allowed_spells = allowed_spells;
        let mut messages = Vec::new();

        for n in 0..32 {
            if state.py.flags.spells_forgotten == 0
                || new_spells == 0
                || n >= allowed_spells
                || n >= 32
            {
                break;
            }

            let order_id = state.py.flags.spells_learned_order[n as usize];
            let mask = if order_id == 99 { 0 } else { 1u32 << order_id };

            if (mask & state.py.flags.spells_forgotten) != 0 {
                if MAGIC_SPELLS[class_id][order_id as usize].level_required <= level as u8 {
                    new_spells -= 1;
                    state.py.flags.spells_forgotten &= !mask;
                    state.py.flags.spells_learnt |= mask;
                    messages.push(format!(
                        "You have remembered the {magic_type_str} of {}.",
                        SPELL_NAMES[order_id as usize + offset as usize]
                    ));
                } else {
                    allowed_spells += 1;
                }
            }
        }

        (new_spells, messages)
    });

    for msg in messages {
        terminal::print_message(Some(&msg));
    }

    new_spells
}

/// 1572
fn learnable_spells(spells: &[Spell; 31], mut new_spells: i32) -> i32 {
    with_state(|state| {
        let mut spell_flag = 0x7FFF_FFFFu32 & !state.py.flags.spells_learnt;
        let mut id = 0;
        let mut mask = 1u32;
        let mut i = 0;

        while spell_flag != 0 {
            if (spell_flag & mask) != 0 {
                spell_flag &= !mask;
                if spells[i].level_required <= state.py.misc.level as u8 {
                    id += 1;
                }
            }
            mask <<= 1;
            i += 1;
        }

        if new_spells > id {
            new_spells = id;
        }

        new_spells
    })
}

/// 1602
fn forget_spells(mut new_spells: i32, magic_type_str: &str, offset: i32) {
    let messages = with_state_mut(|state| {
        let mut messages = Vec::new();

        for i in (0..=31).rev() {
            if new_spells == 0 || state.py.flags.spells_learnt == 0 {
                break;
            }

            let order_id = state.py.flags.spells_learned_order[i];
            let mask = if order_id == 99 { 0 } else { 1u32 << order_id };

            if (mask & state.py.flags.spells_learnt) != 0 {
                state.py.flags.spells_learnt &= !mask;
                state.py.flags.spells_forgotten |= mask;
                new_spells += 1;
                messages.push(format!(
                    "You have forgotten the {magic_type_str} of {}.",
                    SPELL_NAMES[order_id as usize + offset as usize]
                ));
            }
        }

        messages
    });

    for msg in messages {
        terminal::print_message(Some(&msg));
    }
}

/// 1650
pub fn player_calculate_allowed_spells_count(stat: PlayerAttr) {
    let (magic_type_str, offset) = if stat == PlayerAttr::A_INT {
        ("spell", i32::from(NAME_OFFSET_SPELLS))
    } else {
        ("prayer", i32::from(NAME_OFFSET_PRAYERS))
    };

    eliminate_known_spells_greater_than_level(magic_type_str, offset);

    let num_allowed = number_of_spells_allowed(stat);
    let num_known = number_of_spells_known();
    let mut new_spells = num_allowed - num_known;

    if new_spells > 0 {
        new_spells = remember_forgotten_spells(num_allowed, new_spells, magic_type_str, offset);
        if new_spells > 0 {
            let class_id = with_state(|state| state.py.misc.class_id as usize - 1);
            new_spells = learnable_spells(&MAGIC_SPELLS[class_id], new_spells);
        }
    } else if new_spells < 0 {
        forget_spells(new_spells, magic_type_str, offset);
        new_spells = 0;
    }

    let old_new_spells = with_state(|state| state.py.flags.new_spells_to_learn);

    with_state_mut(|state| {
        if new_spells != i32::from(state.py.flags.new_spells_to_learn) {
            state.py.flags.new_spells_to_learn = new_spells as u8;
            state.py.flags.status |= PY_STUDY;
        }
    });

    if new_spells != i32::from(old_new_spells) && new_spells > 0 && old_new_spells == 0 {
        let msg = format!("You can learn some new {magic_type_str}s now.");
        terminal::print_message(Some(&msg));
    }
}

/// 979
pub fn player_gain_spells() {
    if with_state(|state| state.py.flags.confused > 0) {
        terminal::print_message(Some("You are too confused."));
        return;
    }

    let (mut new_spells, class_id) = with_state(|state| {
        (
            i32::from(state.py.flags.new_spells_to_learn),
            state.py.misc.class_id as usize,
        )
    });
    let mut diff_spells = 0;

    let (stat, offset) = if CLASSES[class_id].class_to_use_mage_spells == SPELL_TYPE_MAGE {
        if !player_can_read() {
            return;
        }
        (PlayerAttr::A_INT, i32::from(NAME_OFFSET_SPELLS))
    } else {
        (PlayerAttr::A_WIS, i32::from(NAME_OFFSET_PRAYERS))
    };

    let mut last_known = last_known_spell();

    if new_spells == 0 {
        let word = if stat == PlayerAttr::A_INT {
            "spell"
        } else {
            "prayer"
        };
        terminal::print_message(Some(&format!("You can't learn any new {word}s!")));
        with_state_mut(|state| state.game.player_free_turn = true);
        return;
    }

    let mut spell_flag = if stat == PlayerAttr::A_INT {
        player_determine_learnable_spells()
    } else {
        0x7FFF_FFFF
    } & !with_state(|state| state.py.flags.spells_learnt);

    let mut spell_id = 0i32;
    let mut spell_bank = [0i32; 31];
    let mut mask = 1u32;
    let level = with_state(|state| state.py.misc.level as u8);
    let mut i = 0;

    while spell_flag != 0 {
        if (spell_flag & mask) != 0 {
            spell_flag &= !mask;
            if MAGIC_SPELLS[class_id - 1][i].level_required <= level {
                spell_bank[spell_id as usize] = i as i32;
                spell_id += 1;
            }
        }
        mask <<= 1;
        i += 1;
    }

    if new_spells > spell_id {
        terminal::print_message(Some("You seem to be missing a book."));
        diff_spells = new_spells - spell_id;
        new_spells = spell_id;
    }

    if new_spells != 0 {
        if stat == PlayerAttr::A_INT {
            terminal::terminal_save_screen();
            display_spells_list(&spell_bank[..spell_id as usize], spell_id, false, -1);

            let mut query = 0u8;
            while new_spells != 0 && terminal::get_menu_item_id("Learn which spell?", &mut query) {
                let c = i32::from(query) - i32::from(b'a');

                if c >= 0 && c < spell_id && c < 22 {
                    new_spells -= 1;

                    with_state_mut(|state| {
                        state.py.flags.spells_learnt |= 1u32 << spell_bank[c as usize];
                        state.py.flags.spells_learned_order[last_known as usize] =
                            spell_bank[c as usize] as u8;
                    });
                    last_known += 1;

                    for slot in c as usize..(spell_id - 1) as usize {
                        spell_bank[slot] = spell_bank[slot + 1];
                    }
                    spell_id -= 1;

                    terminal::erase_line(Coord { y: c + 1, x: 31 });
                    display_spells_list(&spell_bank[..spell_id as usize], spell_id, false, -1);
                } else {
                    terminal::terminal_bell_sound();
                }
            }

            terminal::terminal_restore_screen();
        } else {
            while new_spells != 0 {
                let id = random_number(spell_id) - 1;

                with_state_mut(|state| {
                    state.py.flags.spells_learnt |= 1u32 << spell_bank[id as usize];
                    state.py.flags.spells_learned_order[last_known as usize] =
                        spell_bank[id as usize] as u8;
                });
                last_known += 1;

                let msg = format!(
                    "You have learned the prayer of {}.",
                    SPELL_NAMES[(spell_bank[id as usize] + offset) as usize]
                );
                terminal::print_message(Some(&msg));

                for slot in id as usize..(spell_id - 1) as usize {
                    spell_bank[slot] = spell_bank[slot + 1];
                }
                spell_id -= 1;
                new_spells -= 1;
            }
        }
    }

    let mana_was_zero = with_state_mut(|state| {
        state.py.flags.new_spells_to_learn = (new_spells + diff_spells) as u8;

        if state.py.flags.new_spells_to_learn == 0 {
            state.py.flags.status |= PY_STUDY;
        }

        state.py.misc.mana == 0
    });

    if mana_was_zero {
        player_gain_mana(stat);
    }
}

/// 138
pub fn player_teleport(new_distance: i32) {
    let location = with_state_mut(|state| {
        let mut location = Coord_t { y: 0, x: 0 };
        loop {
            location.y = random_number_state(state, i32::from(state.dg.height)) - 1;
            location.x = random_number_state(state, i32::from(state.dg.width)) - 1;

            while coord_distance_between(location, state.py.pos) > new_distance {
                location.y += (state.py.pos.y - location.y) / 2;
                location.x += (state.py.pos.x - location.x) / 2;
            }

            let tile = &state.dg.floor[location.y as usize][location.x as usize];
            if tile.feature_id >= MIN_CLOSED_SPACE || tile.creature_id >= 2 {
                continue;
            }
            break;
        }
        location
    });

    dungeon_move_creature_record(with_state(|state| state.py.pos), location);

    let old_pos = with_state(|state| state.py.pos);
    for spot_y in old_pos.y - 1..=old_pos.y + 1 {
        for spot_x in old_pos.x - 1..=old_pos.x + 1 {
            let spot = Coord_t {
                y: spot_y,
                x: spot_x,
            };
            with_state_mut(|state| {
                state.dg.floor[spot_y as usize][spot_x as usize].temporary_light = false;
            });
            dungeon_lite_spot(spot);
        }
    }

    dungeon_lite_spot(old_pos);

    with_state_mut(|state| {
        state.py.pos = location;
        state.game.teleport_player = false;
    });

    dungeon_reset_view();
    update_monsters(false);
}

/// 165
pub fn player_disturb(major_disturbance: i32, light_disturbance: i32) {
    with_state_mut(|state| {
        state.game.command_count = 0;
    });

    if major_disturbance != 0 && with_state(|state| state.py.flags.status & PY_SEARCH != 0) {
        player_search_off();
    }

    if with_state(|state| state.py.flags.rest != 0) {
        player_rest_off();
    }

    let need_reset_view = with_state_mut(|state| {
        if light_disturbance != 0 || state.py.running_tracker != 0 {
            state.py.running_tracker = 0;
            true
        } else {
            false
        }
    });
    if need_reset_view {
        dungeon_reset_view();
    }

    terminal::flush_input_buffer();
}

/// 177
pub fn player_search_on() {
    player_change_speed(1);
    with_state_mut(|state| {
        state.py.flags.status |= PY_SEARCH;
    });
    print_character_movement_state();
    print_character_speed();
    with_state_mut(|state| {
        state.py.flags.food_digested += 1;
    });
}

/// 188
pub fn player_search_off() {
    dungeon_reset_view();
    player_change_speed(-1);
    with_state_mut(|state| {
        state.py.flags.status &= !PY_SEARCH;
    });
    print_character_movement_state();
    print_character_speed();
    with_state_mut(|state| {
        state.py.flags.food_digested -= 1;
    });
}

/// 237
pub fn player_rest_on() {
    let rest_num = with_state_mut(|state| {
        if state.game.command_count > 0 {
            let rest_num = state.game.command_count as i32;
            state.game.command_count = 0;
            rest_num
        } else {
            0
        }
    });

    let rest_num = if rest_num != 0 {
        rest_num
    } else {
        let mut rest_str = [0u8; 6];
        terminal::put_string_clear_to_eol("Rest for how long? ", terminal::Coord { y: 0, x: 0 });
        if terminal::get_string_input(&mut rest_str, terminal::Coord { y: 0, x: 19 }, 5) {
            if rest_str[0] == b'*' {
                -SHRT_MAX
            } else {
                let mut parsed = 0i32;
                let end = rest_str
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(rest_str.len());
                let _ = string_to_number(
                    std::str::from_utf8(&rest_str[..end]).unwrap_or(""),
                    &mut parsed,
                );
                parsed
            }
        } else {
            0
        }
    };

    if rest_num == -SHRT_MAX || (rest_num > 0 && rest_num <= SHRT_MAX) {
        if with_state(|state| state.py.flags.status & PY_SEARCH != 0) {
            player_search_off();
        }
        with_state_mut(|state| {
            state.py.flags.rest = rest_num as i16;
            state.py.flags.status |= PY_REST;
        });
        print_character_movement_state();
        with_state_mut(|state| {
            state.py.flags.food_digested -= 1;
        });
        terminal::put_string_clear_to_eol(
            "Press any key to stop resting...",
            terminal::Coord { y: 0, x: 0 },
        );
        terminal::put_qio();
        return;
    }

    if rest_num != 0 {
        terminal::print_message(Some("Invalid rest count."));
    }
    terminal::message_line_clear();
    with_state_mut(|state| {
        state.game.player_free_turn = true;
    });
}

/// 249
pub fn player_rest_off() {
    with_state_mut(|state| {
        state.py.flags.rest = 0;
        state.py.flags.status &= !PY_REST;
    });
    print_character_movement_state();
    terminal::print_message(CNIL);
    with_state_mut(|state| {
        state.py.flags.food_digested += 1;
    });
}

/// 728
pub fn player_search(coord: Coord_t, chance: i32) {
    let mut chance = chance;
    let (confused, blind, image) = with_state(|state| {
        (
            state.py.flags.confused,
            state.py.flags.blind,
            state.py.flags.image,
        )
    });

    if confused > 0 {
        chance /= 10;
    }
    if blind > 0 || player_no_light() {
        chance /= 10;
    }
    if image > 0 {
        chance /= 10;
    }

    for spot_y in coord.y - 1..=coord.y + 1 {
        for spot_x in coord.x - 1..=coord.x + 1 {
            if random_number(100) >= chance {
                continue;
            }

            let treasure_id =
                with_state(|state| state.dg.floor[spot_y as usize][spot_x as usize].treasure_id);
            if treasure_id == 0 {
                continue;
            }

            let spot = Coord_t {
                y: spot_y,
                x: spot_x,
            };

            let category_id =
                with_state(|state| state.game.treasure.list[treasure_id as usize].category_id);

            if category_id == TV_INVIS_TRAP {
                let mut description = [0u8; MORIA_OBJ_DESC_SIZE as usize];
                let item = with_state(|state| state.game.treasure.list[treasure_id as usize]);
                item_description(&mut description, item, true);
                let end = description
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(description.len());
                let desc = String::from_utf8_lossy(&description[..end]);
                terminal::print_message(Some(&format!("You have found {desc}")));
                trap_change_visibility(spot);
                player_end_running();
            } else if category_id == TV_SECRET_DOOR {
                terminal::print_message(Some("You have found a secret door."));
                trap_change_visibility(spot);
                player_end_running();
            } else if category_id == TV_CHEST {
                let (flags, identified) = with_state(|state| {
                    let item = &state.game.treasure.list[treasure_id as usize];
                    (item.flags, spell_item_identified(*item))
                });
                if (flags & CH_TRAPPED) > 1 {
                    if identified {
                        terminal::print_message(Some("The chest is trapped!"));
                    } else {
                        with_state_mut(|state| {
                            spell_item_identify_and_remove_random_inscription(
                                &mut state.game.treasure.list[treasure_id as usize],
                            );
                        });
                        terminal::print_message(Some("You have discovered a trap on the chest!"));
                    }
                }
            }
        }
    }
}

/// 260
pub fn player_died_from_string(description: &mut Vtype_t, monster_name: &str, movement: u32) {
    let formatted = if (movement & CM_WIN) != 0 {
        format!("The {monster_name}")
    } else {
        let first = monster_name.as_bytes()[0];
        if is_vowel(first) {
            format!("an {monster_name}")
        } else {
            format!("a {monster_name}")
        }
    };
    let bytes = formatted.as_bytes();
    let n = bytes.len().min(MORIA_MESSAGE_SIZE - 1);
    description[..n].copy_from_slice(&bytes[..n]);
    description[n] = 0;
}

/// 639
pub fn player_test_being_hit(
    base_to_hit: i32,
    level: i32,
    plus_to_hit: i32,
    armor_class: i32,
    attack_type_id: u8,
) -> bool {
    player_disturb(1, 0);

    with_state_mut(|state| {
        let hit_chance = base_to_hit
            + plus_to_hit * i32::from(BTH_PER_PLUS_TO_HIT_ADJUST)
            + level
                * i32::from(
                    CLASS_LEVEL_ADJ[state.py.misc.class_id as usize][attack_type_id as usize],
                );

        let die = random_number_state(state, 20);
        die != 1
            && (die == 20
                || (hit_chance > 0 && random_number_state(state, hit_chance) > armor_class))
    })
}

/// 365
pub fn player_test_attack_hits(attack_id: i32, level: u8) -> bool {
    let (ac, player_level, unique_items, au) = with_state(|state| {
        (
            i32::from(state.py.misc.ac) + i32::from(state.py.misc.magical_ac),
            i32::from(state.py.misc.level),
            state.py.pack.unique_items,
            state.py.misc.au,
        )
    });

    match attack_id {
        1 => player_test_being_hit(60, i32::from(level), 0, ac, CLASS_MISC_HIT),
        2 => player_test_being_hit(-3, i32::from(level), 0, ac, CLASS_MISC_HIT),
        3..=5 => player_test_being_hit(10, i32::from(level), 0, ac, CLASS_MISC_HIT),
        6 | 9 => player_test_being_hit(0, i32::from(level), 0, ac, CLASS_MISC_HIT),
        7 | 8 => player_test_being_hit(10, i32::from(level), 0, ac, CLASS_MISC_HIT),
        10 | 11 => player_test_being_hit(2, i32::from(level), 0, ac, CLASS_MISC_HIT),
        12 => player_test_being_hit(5, i32::from(level), 0, player_level, CLASS_MISC_HIT) && au > 0,
        13 => {
            player_test_being_hit(2, i32::from(level), 0, player_level, CLASS_MISC_HIT)
                && unique_items > 0
        }
        14 => player_test_being_hit(5, i32::from(level), 0, ac, CLASS_MISC_HIT),
        15 | 16 => player_test_being_hit(0, i32::from(level), 0, ac, CLASS_MISC_HIT),
        17 | 18 => player_test_being_hit(2, i32::from(level), 0, ac, CLASS_MISC_HIT),
        19 => player_test_being_hit(5, i32::from(level), 0, ac, CLASS_MISC_HIT),
        20 => true,
        21 => player_test_being_hit(20, i32::from(level), 0, ac, CLASS_MISC_HIT),
        22 | 23 => player_test_being_hit(5, i32::from(level), 0, ac, CLASS_MISC_HIT),
        24 => {
            player_test_being_hit(15, i32::from(level), 0, ac, CLASS_MISC_HIT) && unique_items > 0
        }
        99 => true,
        _ => false,
    }
}

/// 662
pub fn player_takes_hit(damage: i32, creature_name_label: &Vtype_t) {
    let survived = with_state_mut(|state| {
        let mut damage = damage;
        if state.py.flags.invulnerability > 0 {
            damage = 0;
        }
        state.py.misc.current_hp -= damage as i16;

        if state.py.misc.current_hp >= 0 {
            return true;
        }

        if !state.game.character_is_dead {
            state.game.character_is_dead = true;
            state
                .game
                .character_died_from
                .copy_from_slice(creature_name_label);
            state.game.total_winner = false;
        }

        state.dg.generate_new_level = true;
        false
    });

    if survived {
        print_character_current_hit_points();
    }
}

/// 1074
pub fn player_saving_throw() -> bool {
    let (class_id, level, saving_throw, wis) = with_state(|state| {
        (
            state.py.misc.class_id,
            state.py.misc.level,
            state.py.misc.saving_throw,
            state.py.stats.used[PlayerAttr::A_WIS as usize],
        )
    });
    let wis_adj = {
        let value = i32::from(wis);
        if value > 117 {
            7
        } else if value > 107 {
            6
        } else if value > 87 {
            5
        } else if value > 67 {
            4
        } else if value > 17 {
            3
        } else if value > 14 {
            2
        } else {
            i32::from(value > 7)
        }
    };
    let class_level_adjustment =
        i32::from(CLASS_LEVEL_ADJ[class_id as usize][PlayerClassLevelAdj::SAVE as usize])
            * i32::from(level)
            / 3;
    let saving = i32::from(saving_throw) + wis_adj + class_level_adjustment;
    random_number(100) <= saving
}

/// 778
pub fn player_strength() {
    let wield = with_state(|state| state.py.inventory[PlayerEquipment::Wield as usize]);
    let used_str = with_state(|state| state.py.stats.used[PlayerAttr::A_STR as usize]);

    if wield.category_id != TV_NOTHING && i32::from(used_str) * 15 < i32::from(wield.weight) {
        if !with_state(|state| state.py.weapon_is_heavy) {
            terminal::print_message(Some("You have trouble wielding such a heavy weapon."));
            with_state_mut(|state| {
                state.py.weapon_is_heavy = true;
            });
            player_recalculate_bonuses();
        }
    } else if with_state(|state| state.py.weapon_is_heavy) {
        with_state_mut(|state| {
            state.py.weapon_is_heavy = false;
        });
        if wield.category_id != TV_NOTHING {
            terminal::print_message(Some("You are strong enough to wield your weapon."));
        }
        player_recalculate_bonuses();
    }

    let limit = player_carrying_load_limit();
    let pack_weight = with_state(|state| state.py.pack.weight);
    let new_heaviness = if limit < i32::from(pack_weight) {
        i32::from(pack_weight) / (limit + 1)
    } else {
        0
    };

    let old_heaviness = with_state(|state| i32::from(state.py.pack.heaviness));
    if old_heaviness != new_heaviness {
        if old_heaviness < new_heaviness {
            terminal::print_message(Some("Your pack is so heavy that it slows you down."));
        } else {
            terminal::print_message(Some("You move more easily under the weight of your pack."));
        }
        player_change_speed(new_heaviness - old_heaviness);
        with_state_mut(|state| {
            state.py.pack.heaviness = new_heaviness as i16;
        });
    }

    with_state_mut(|state| {
        state.py.flags.status &= !PY_STR_WGT;
    });
}

/// 31
fn player_reset_flags() {
    with_state_mut(|state| {
        state.py.flags.see_invisible = false;
        state.py.flags.teleport = false;
        state.py.flags.free_action = false;
        state.py.flags.slow_digest = false;
        state.py.flags.aggravate = false;
        state.py.flags.sustain_str = false;
        state.py.flags.sustain_int = false;
        state.py.flags.sustain_wis = false;
        state.py.flags.sustain_con = false;
        state.py.flags.sustain_dex = false;
        state.py.flags.sustain_chr = false;
        state.py.flags.resistant_to_fire = false;
        state.py.flags.resistant_to_acid = false;
        state.py.flags.resistant_to_cold = false;
        state.py.flags.regenerate_hp = false;
        state.py.flags.resistant_to_light = false;
        state.py.flags.free_fall = false;
    });
}

/// 377
pub fn player_change_speed(speed: i32) {
    with_state_mut(|state| {
        state.py.flags.speed += speed as i16;
        state.py.flags.status |= PY_SPEED;

        for i in
            (i32::from(monsters::MON_MIN_INDEX_ID)..i32::from(state.next_free_monster_id)).rev()
        {
            state.monsters[i as usize].speed += speed as i16;
        }
    });
}

/// 422
pub fn player_adjust_bonuses_for_item(item: Inventory, factor: i32) {
    let amount = item.misc_use * factor as i16;

    if (item.flags & TR_STATS) != 0 {
        const STATS: [PlayerAttr; 6] = [
            PlayerAttr::A_STR,
            PlayerAttr::A_INT,
            PlayerAttr::A_WIS,
            PlayerAttr::A_DEX,
            PlayerAttr::A_CON,
            PlayerAttr::A_CHR,
        ];
        for i in 0..6 {
            if ((1 << i) & item.flags) != 0 {
                player_stat_boost(STATS[i as usize], i32::from(amount));
            }
        }
    }

    if (item.flags & TR_SPEED) != 0 {
        player_change_speed(-i32::from(amount));
    }

    with_state_mut(|state| {
        if (item.flags & TR_SEARCH) != 0 {
            state.py.misc.chance_in_search += amount;
            state.py.misc.fos -= amount;
        }

        if (item.flags & TR_STEALTH) != 0 {
            state.py.misc.stealth_factor += amount;
        }

        if (item.flags & TR_BLIND) != 0 && factor > 0 {
            state.py.flags.blind += 1000;
        }

        if (item.flags & TR_TIMID) != 0 && factor > 0 {
            state.py.flags.afraid += 50;
        }

        if (item.flags & TR_INFRA) != 0 {
            state.py.flags.see_infra += amount;
        }
    });
}

/// 456
fn player_recalculate_bonuses_from_inventory() {
    with_state_mut(|state| {
        for i in PlayerEquipment::Wield as usize..PlayerEquipment::Light as usize {
            let item = &state.py.inventory[i];
            if item.category_id == TV_NOTHING {
                continue;
            }

            state.py.misc.plusses_to_hit = state.py.misc.plusses_to_hit.wrapping_add(item.to_hit);

            if item.category_id != TV_BOW {
                state.py.misc.plusses_to_damage =
                    state.py.misc.plusses_to_damage.wrapping_add(item.to_damage);
            }

            state.py.misc.magical_ac = state.py.misc.magical_ac.wrapping_add(item.to_ac);
            state.py.misc.ac = state.py.misc.ac.wrapping_add(item.ac);

            if spell_item_identified(*item) {
                state.py.misc.display_to_hit =
                    state.py.misc.display_to_hit.wrapping_add(item.to_hit);
                if item.category_id != TV_BOW {
                    state.py.misc.display_to_damage =
                        state.py.misc.display_to_damage.wrapping_add(item.to_damage);
                }
                state.py.misc.display_to_ac = state.py.misc.display_to_ac.wrapping_add(item.to_ac);
                state.py.misc.display_ac = state.py.misc.display_ac.wrapping_add(item.ac);
            } else if !inventory_item_is_cursed(*item) {
                state.py.misc.display_ac = state.py.misc.display_ac.wrapping_add(item.ac);
            }
        }
    });
}

/// 487
fn player_recalculate_sustain_stats_from_inventory() {
    with_state_mut(|state| {
        for i in PlayerEquipment::Wield as usize..PlayerEquipment::Light as usize {
            if (state.py.inventory[i].flags & TR_SUST_STAT) == 0 {
                continue;
            }

            match state.py.inventory[i].misc_use {
                1 => state.py.flags.sustain_str = true,
                2 => state.py.flags.sustain_int = true,
                3 => state.py.flags.sustain_wis = true,
                4 => state.py.flags.sustain_con = true,
                5 => state.py.flags.sustain_dex = true,
                6 => state.py.flags.sustain_chr = true,
                _ => {}
            }
        }
    });
}

/// 588
pub fn player_recalculate_bonuses() {
    let saved_display_ac = with_state(|state| state.py.misc.display_ac);

    with_state_mut(|state| {
        if state.py.flags.slow_digest {
            state.py.flags.food_digested += 1;
        }
        if state.py.flags.regenerate_hp {
            state.py.flags.food_digested -= 3;
        }
    });

    player_reset_flags();

    let plusses_to_hit = player_to_hit_adjustment() as i16;
    let plusses_to_damage = player_damage_adjustment() as i16;
    let magical_ac = player_armor_class_adjustment() as i16;

    with_state_mut(|state| {
        state.py.misc.plusses_to_hit = plusses_to_hit;
        state.py.misc.plusses_to_damage = plusses_to_damage;
        state.py.misc.magical_ac = magical_ac;
        state.py.misc.ac = 0;

        state.py.misc.display_to_hit = plusses_to_hit;
        state.py.misc.display_to_damage = plusses_to_damage;
        state.py.misc.display_ac = 0;
        state.py.misc.display_to_ac = magical_ac;
    });

    player_recalculate_bonuses_from_inventory();

    let item_flags = inventory_collect_all_item_flags();

    with_state_mut(|state| {
        state.py.misc.display_ac += state.py.misc.display_to_ac;

        if state.py.weapon_is_heavy {
            state.py.misc.display_to_hit = state.py.misc.display_to_hit.wrapping_add(
                state.py.stats.used[PlayerAttr::A_STR as usize] as i16 * 15
                    - state.py.inventory[PlayerEquipment::Wield as usize].weight as i16,
            );
        }

        if state.py.flags.invulnerability > 0 {
            state.py.misc.ac += 100;
            state.py.misc.display_ac += 100;
        }

        if state.py.flags.blessed > 0 {
            state.py.misc.ac += 2;
            state.py.misc.display_ac += 2;
        }

        if state.py.flags.detect_invisible > 0 {
            state.py.flags.see_invisible = true;
        }

        if saved_display_ac != state.py.misc.display_ac {
            state.py.flags.status |= PY_ARMOR;
        }

        if (item_flags & TR_SLOW_DIGEST) != 0 {
            state.py.flags.slow_digest = true;
        }
        if (item_flags & TR_AGGRAVATE) != 0 {
            state.py.flags.aggravate = true;
        }
        if (item_flags & TR_TELEPORT) != 0 {
            state.py.flags.teleport = true;
        }
        if (item_flags & TR_REGEN) != 0 {
            state.py.flags.regenerate_hp = true;
        }
        if (item_flags & TR_RES_FIRE) != 0 {
            state.py.flags.resistant_to_fire = true;
        }
        if (item_flags & TR_RES_ACID) != 0 {
            state.py.flags.resistant_to_acid = true;
        }
        if (item_flags & TR_RES_COLD) != 0 {
            state.py.flags.resistant_to_cold = true;
        }
        if (item_flags & TR_FREE_ACT) != 0 {
            state.py.flags.free_action = true;
        }
        if (item_flags & TR_SEE_INVIS) != 0 {
            state.py.flags.see_invisible = true;
        }
        if (item_flags & TR_RES_LIGHT) != 0 {
            state.py.flags.resistant_to_light = true;
        }
        if (item_flags & TR_FFALL) != 0 {
            state.py.flags.free_fall = true;
        }
    });

    player_recalculate_sustain_stats_from_inventory();

    with_state_mut(|state| {
        if state.py.flags.slow_digest {
            state.py.flags.food_digested -= 1;
        }
        if state.py.flags.regenerate_hp {
            state.py.flags.food_digested += 3;
        }
    });
}

/// 625
pub fn player_take_off(item_id: i32, pack_position_id: i32) {
    // Snapshot the item and mutate pack/equipment outside of item_description —
    // item_description calls with_state and must not nest under with_state_mut.
    let (prefix, item) = with_state_mut(|state| {
        state.py.flags.status |= PY_STR_WGT;

        let item = state.py.inventory[item_id as usize];
        state.py.pack.weight -= item.weight as i16 * item.items_count as i16;
        state.py.equipment_count -= 1;

        let prefix = if item_id == PlayerEquipment::Wield as i32
            || item_id == PlayerEquipment::Auxiliary as i32
        {
            "Was wielding "
        } else if item_id == PlayerEquipment::Light as i32 {
            "Light source was "
        } else {
            "Was wearing "
        };

        (prefix, item)
    });

    let mut description = [0u8; MORIA_OBJ_DESC_SIZE as usize];
    item_description(&mut description, item, true);

    let desc_end = description
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(description.len());
    let desc = String::from_utf8_lossy(&description[..desc_end]);
    let msg = if pack_position_id >= 0 {
        format!(
            "{prefix}{desc} ({})",
            (b'a' + pack_position_id as u8) as char
        )
    } else {
        format!("{prefix}{desc}")
    };
    print_message(Some(&msg));

    // For secondary weapon
    if item_id != PlayerEquipment::Auxiliary as i32 {
        player_adjust_bonuses_for_item(item, -1);
    }

    with_state_mut(|state| {
        inventory_item_copy_to(
            OBJ_NOTHING as i16,
            &mut state.py.inventory[item_id as usize],
        );
    });
}

/// 782
pub fn player_left_hand_ring_empty() -> bool {
    with_state(|state| state.py.inventory[PlayerEquipment::Left as usize].category_id == TV_NOTHING)
}

/// 786
pub fn player_right_hand_ring_empty() -> bool {
    with_state(|state| {
        state.py.inventory[PlayerEquipment::Right as usize].category_id == TV_NOTHING
    })
}

/// 793
pub fn player_is_wielding_item() -> bool {
    with_state(|state| {
        state.py.inventory[PlayerEquipment::Wield as usize].category_id != TV_NOTHING
            || state.py.inventory[PlayerEquipment::Auxiliary as usize].category_id != TV_NOTHING
    })
}

/// 797
pub fn player_worn_item_is_cursed(id: PlayerEquipment) -> bool {
    with_state(|state| inventory_item_is_cursed(state.py.inventory[id as usize]))
}

/// 801
pub fn player_worn_item_remove_curse(id: PlayerEquipment) {
    with_state_mut(|state| {
        inventory_item_remove_curse(&mut state.py.inventory[id as usize]);
    });
}

/// 1065
pub fn player_weapon_critical_blow(
    weapon_weight: i32,
    plus_to_hit: i32,
    damage: i32,
    attack_type_id: u8,
) -> i32 {
    let (class_id, level) = with_state(|state| (state.py.misc.class_id, state.py.misc.level));

    let mut critical = damage;
    let threshold = weapon_weight
        + 5 * plus_to_hit
        + i32::from(CLASS_LEVEL_ADJ[class_id as usize][attack_type_id as usize]) * i32::from(level);

    if random_number(5000) <= threshold {
        let weight = weapon_weight + random_number(650);
        critical = if weight < 400 {
            print_message(Some("It was a good hit! (x2 damage)"));
            2 * damage + 5
        } else if weight < 700 {
            print_message(Some("It was an excellent hit! (x3 damage)"));
            3 * damage + 10
        } else if weight < 900 {
            print_message(Some("It was a superb hit! (x4 damage)"));
            4 * damage + 15
        } else {
            print_message(Some("It was a *GREAT* hit! (x5 damage)"));
            5 * damage + 20
        };
    }

    critical
}

/// 1112
pub fn player_calculate_to_hit_blows(weapon_id: u8, weapon_weight: i32) -> (i32, i32) {
    let mut total_to_hit = 0;
    let mut blows = if weapon_id == TV_NOTHING {
        total_to_hit = -3;
        2
    } else {
        player_attack_blows(weapon_weight, &mut total_to_hit)
    };

    if (TV_SLING_AMMO..=TV_SPIKE).contains(&weapon_id) {
        blows = 1;
    }

    total_to_hit += with_state(|state| i32::from(state.py.misc.plusses_to_hit));
    (blows, total_to_hit)
}

/// 1127
pub fn player_calculate_base_to_hit(creature_lit: bool, tot_tohit: i32) -> i32 {
    if creature_lit {
        return with_state(|state| i32::from(state.py.misc.bth));
    }

    with_state(|state| {
        let mut bth = i32::from(state.py.misc.bth) / 2;
        bth -= tot_tohit * (i32::from(BTH_PER_PLUS_TO_HIT_ADJUST) - 1);
        bth -= i32::from(state.py.misc.level)
            * i32::from(
                CLASS_LEVEL_ADJ[state.py.misc.class_id as usize][PlayerClassLevelAdj::BTH as usize],
            )
            / 2;
        bth
    })
}

/// 1227
pub fn player_attack_monster(coord: Coord_t) {
    let creature_id =
        with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].creature_id as i32);

    with_state_mut(|state| {
        state.monsters[creature_id as usize].sleep_count = 0;
    });

    let (lit, monster_creature_id) = with_state(|state| {
        let monster = &state.monsters[creature_id as usize];
        (monster.lit, monster.creature_id)
    });

    let name = if lit {
        format!("the {}", CREATURES_LIST[monster_creature_id as usize].name)
    } else {
        "it".to_string()
    };

    let (weapon_id, weapon_weight, weapon_damage) = with_state(|state| {
        let item = &state.py.inventory[PlayerEquipment::Wield as usize];
        (item.category_id, i32::from(item.weight), item.damage)
    });

    let (mut blows, total_to_hit) = player_calculate_to_hit_blows(weapon_id, weapon_weight);
    let base_to_hit = player_calculate_base_to_hit(lit, total_to_hit);
    let creature_ac = i32::from(CREATURES_LIST[monster_creature_id as usize].ac);
    let player_level = with_state(|state| i32::from(state.py.misc.level));

    while blows > 0 {
        blows -= 1;

        if !player_test_being_hit(
            base_to_hit,
            player_level,
            total_to_hit,
            creature_ac,
            PlayerClassLevelAdj::BTH as u8,
        ) {
            print_message(Some(&format!("You miss {name}.")));
            continue;
        }

        print_message(Some(&format!("You hit {name}.")));

        let mut damage = if weapon_id == TV_NOTHING {
            let damage = dice_roll(Dice { dice: 1, sides: 1 });
            player_weapon_critical_blow(1, 0, damage, PlayerClassLevelAdj::BTH as u8)
        } else {
            let item = with_state(|state| state.py.inventory[PlayerEquipment::Wield as usize]);
            let mut damage = dice_roll(weapon_damage);
            damage = item_magic_ability_damage(item, damage, monster_creature_id as i32);
            player_weapon_critical_blow(
                weapon_weight,
                total_to_hit,
                damage,
                PlayerClassLevelAdj::BTH as u8,
            )
        };

        damage += with_state(|state| i32::from(state.py.misc.plusses_to_damage));
        if damage < 0 {
            damage = 0;
        }

        if with_state(|state| state.py.flags.confuse_monster) {
            with_state_mut(|state| {
                state.py.flags.confuse_monster = false;
            });
            print_message(Some("Your hands stop glowing."));

            let (defenses, creature_level) = with_state(|_state| {
                let creature = &CREATURES_LIST[monster_creature_id as usize];
                (creature.defenses, creature.level)
            });

            if (defenses & CD_NO_SLEEP) != 0
                || random_number(i32::from(MON_MAX_LEVELS)) < i32::from(creature_level)
            {
                print_message(Some(&format!("{name} is unaffected.")));
            } else {
                print_message(Some(&format!("{name} appears confused.")));
                with_state_mut(|state| {
                    let roll = random_number_state(state, 16);
                    let monster = &mut state.monsters[creature_id as usize];
                    if monster.confused_amount != 0 {
                        monster.confused_amount += 3;
                    } else {
                        monster.confused_amount = (2 + roll) as u8;
                    }
                });
            }

            if lit && random_number(4) == 1 {
                with_state_mut(|state| {
                    let creature = &CREATURES_LIST[monster_creature_id as usize];
                    state.creature_recall[monster_creature_id as usize].defenses |=
                        creature.defenses & CD_NO_SLEEP;
                });
            }
        }

        if monster_take_hit(creature_id, damage) >= 0 {
            print_message(Some(&format!("You have slain {name}.")));
            display_character_experience();
            return;
        }

        if (TV_SLING_AMMO..=TV_SPIKE).contains(&weapon_id) {
            let depleted = with_state_mut(|state| {
                let item = &mut state.py.inventory[PlayerEquipment::Wield as usize];
                item.items_count -= 1;
                state.py.pack.weight -= item.weight as i16;
                state.py.flags.status |= PY_STR_WGT;

                if item.items_count == 0 {
                    state.py.equipment_count -= 1;
                    let copy = *item;
                    inventory_item_copy_to(OBJ_NOTHING as i16, item);
                    Some(copy)
                } else {
                    None
                }
            });
            if let Some(copy) = depleted {
                player_adjust_bonuses_for_item(copy, -1);
                player_recalculate_bonuses();
            }
        }
    }
}

/// 1452
pub fn player_attack_position(coord: Coord_t) {
    if with_state(|state| state.py.flags.afraid > 0) {
        print_message(Some("You are too afraid!"));
        return;
    }

    player_attack_monster(coord);
}

/// 739
pub fn player_carrying_load_limit() -> i32 {
    with_state(|state| {
        let mut weight_cap = i32::from(state.py.stats.used[PlayerAttr::A_STR as usize])
            * i32::from(PLAYER_WEIGHT_CAP)
            + i32::from(state.py.misc.weight);
        if weight_cap > 3000 {
            weight_cap = 3000;
        }
        weight_cap
    })
}

/// 1237
fn player_lock_picking_skill_in(state: &crate::game::State) -> i16 {
    let dex = i32::from(state.py.stats.used[PlayerAttr::A_DEX as usize]);
    let dex_adj: i16 = if dex < 4 {
        -8
    } else if dex == 4 {
        -6
    } else if dex == 5 {
        -4
    } else if dex == 6 {
        -2
    } else if dex == 7 {
        -1
    } else if dex < 13 {
        0
    } else if dex < 16 {
        1
    } else if dex < 18 {
        2
    } else if dex < 59 {
        4
    } else if dex < 94 {
        5
    } else if dex < 117 {
        6
    } else {
        8
    };

    let int_val = i32::from(state.py.stats.used[PlayerAttr::A_INT as usize]);
    let int_adj: i16 = if int_val > 117 {
        7
    } else if int_val > 107 {
        6
    } else if int_val > 87 {
        5
    } else if int_val > 67 {
        4
    } else if int_val > 17 {
        3
    } else if int_val > 14 {
        2
    } else {
        i16::from(int_val > 7)
    };

    let mut skill = state.py.misc.disarm;
    skill += 2 * dex_adj;
    skill += int_adj;
    skill += (i32::from(
        CLASS_LEVEL_ADJ[state.py.misc.class_id as usize][PlayerClassLevelAdj::DISARM as usize],
    ) * i32::from(state.py.misc.level)
        / 3) as i16;
    skill
}

/// 1237
pub fn player_lock_picking_skill() -> i16 {
    with_state(player_lock_picking_skill_in)
}

/// 1268
pub fn open_closed_door(coord: Coord_t) {
    let treasure_id =
        with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].treasure_id);
    if treasure_id == 0 {
        return;
    }

    let mut picked_lock = false;
    let mut door_message: Option<&str> = None;
    let mut door_failed = false;
    with_state_mut(|state| {
        let item_misc = state.game.treasure.list[treasure_id as usize].misc_use;
        if item_misc > 0 {
            if state.py.flags.confused > 0 {
                door_message = Some("You are too confused to pick the lock.");
            } else {
                let skill = player_lock_picking_skill_in(state);
                let roll = random_number_state(state, 100);
                if i32::from(skill) - i32::from(item_misc) > roll {
                    door_message = Some("You have picked the lock.");
                    state.py.misc.exp += 1;
                    picked_lock = true;
                    state.game.treasure.list[treasure_id as usize].misc_use = 0;
                } else {
                    door_failed = true;
                }
            }
        } else if item_misc < 0 {
            door_message = Some("It appears to be stuck.");
        }
    });
    if door_failed {
        terminal::print_message_no_command_interrupt("You failed to pick the lock.");
    } else if let Some(msg) = door_message {
        terminal::print_message(Some(msg));
    }
    if picked_lock {
        display_character_experience();
    }

    let misc_use = with_state(|state| state.game.treasure.list[treasure_id as usize].misc_use);
    if misc_use == 0 {
        with_state_mut(|state| {
            inventory_item_copy_to(
                OBJ_OPEN_DOOR as i16,
                &mut state.game.treasure.list[treasure_id as usize],
            );
            state.dg.floor[coord.y as usize][coord.x as usize].feature_id = TILE_CORR_FLOOR;
        });
        dungeon_lite_spot(coord);
        with_state_mut(|state| {
            state.game.command_count = 0;
        });
    }
}

/// 1318
pub fn open_closed_chest(coord: Coord_t) {
    let treasure_id =
        with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].treasure_id);
    if treasure_id == 0 {
        return;
    }

    let mut success = false;
    let mut picked_lock = false;
    let mut exp_gain = 0i32;
    let mut chest_message: Option<&str> = None;
    let mut chest_failed = false;
    with_state_mut(|state| {
        let locked = (state.game.treasure.list[treasure_id as usize].flags & CH_LOCKED) != 0;
        if locked {
            if state.py.flags.confused > 0 {
                chest_message = Some("You are too confused to pick the lock.");
            } else {
                let depth = state.game.treasure.list[treasure_id as usize].depth_first_found;
                let skill = player_lock_picking_skill_in(state);
                let roll = random_number_state(state, 100);
                if i32::from(skill) - i32::from(depth) > roll {
                    chest_message = Some("You have picked the lock.");
                    exp_gain = i32::from(depth);
                    picked_lock = true;
                    success = true;
                } else {
                    chest_failed = true;
                }
            }
        } else {
            success = true;
        }
    });
    if chest_failed {
        terminal::print_message_no_command_interrupt("You failed to pick the lock.");
    } else if let Some(msg) = chest_message {
        terminal::print_message(Some(msg));
    }
    if picked_lock {
        with_state_mut(|state| {
            state.py.misc.exp += exp_gain;
        });
        display_character_experience();
    }

    if success {
        with_state_mut(|state| {
            let item = &mut state.game.treasure.list[treasure_id as usize];
            item.flags &= !CH_LOCKED;
            item.special_name_id = SpecialNameIds::SN_EMPTY as u8;
            item.cost = 0;
            // Must use the state-aware helper — the public wrapper calls
            // with_state_mut and cannot nest under this borrow.
            spell_item_identify_and_remove_random_inscription_for_state(
                state,
                treasure_id as usize,
            );
        });
    }

    if with_state(|state| (state.game.treasure.list[treasure_id as usize].flags & CH_LOCKED) != 0) {
        return;
    }

    chest_trap(coord);

    if with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].treasure_id != 0) {
        with_state_mut(|state| {
            inventory_item_remove_curse(&mut state.game.treasure.list[treasure_id as usize]);
            let flags = state.game.treasure.list[treasure_id as usize].flags;
            let _ = monster_death(coord, flags);
            state.game.treasure.list[treasure_id as usize].flags = 0;
        });
    }
}

/// 1353
pub fn player_open_closed_object() {
    let mut dir = 0i32;
    if !get_direction_with_memory(None, &mut dir) {
        return;
    }

    let mut coord = with_state(|state| state.py.pos);
    let _ = player_move_position(dir, &mut coord);

    let (creature_id, treasure_id, category_id) = with_state(|state| {
        let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
        let category_id = if tile.treasure_id != 0 {
            state.game.treasure.list[tile.treasure_id as usize].category_id
        } else {
            TV_NOTHING
        };
        (tile.creature_id, tile.treasure_id, category_id)
    });

    let mut no_object = false;
    if creature_id > 1
        && treasure_id != 0
        && (category_id == TV_CLOSED_DOOR || category_id == TV_CHEST)
    {
        object_blocked_by_monster(i32::from(creature_id));
    } else if treasure_id != 0 {
        if category_id == TV_CLOSED_DOOR {
            open_closed_door(coord);
        } else if category_id == TV_CHEST {
            open_closed_chest(coord);
        } else {
            no_object = true;
        }
    } else {
        no_object = true;
    }

    if no_object {
        with_state_mut(|state| {
            state.game.player_free_turn = true;
        });
        terminal::print_message(Some("I do not see anything you can open there."));
    }
}

/// 1395
pub fn player_close_door() {
    let mut dir = 0i32;
    if !get_direction_with_memory(None, &mut dir) {
        return;
    }

    let mut coord = with_state(|state| state.py.pos);
    let _ = player_move_position(dir, &mut coord);

    let (treasure_id, category_id, creature_id) = with_state(|state| {
        let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
        let category_id = if tile.treasure_id != 0 {
            state.game.treasure.list[tile.treasure_id as usize].category_id
        } else {
            TV_NOTHING
        };
        (tile.treasure_id, category_id, tile.creature_id)
    });

    let mut no_object = false;
    if treasure_id != 0 {
        if category_id == TV_OPEN_DOOR {
            if creature_id == 0 {
                let misc_use =
                    with_state(|state| state.game.treasure.list[treasure_id as usize].misc_use);
                if misc_use == 0 {
                    with_state_mut(|state| {
                        inventory_item_copy_to(
                            OBJ_CLOSED_DOOR as i16,
                            &mut state.game.treasure.list[treasure_id as usize],
                        );
                        state.dg.floor[coord.y as usize][coord.x as usize].feature_id =
                            TILE_BLOCKED_FLOOR;
                    });
                    dungeon_lite_spot(coord);
                } else {
                    terminal::print_message(Some("The door appears to be broken."));
                }
            } else {
                object_blocked_by_monster(i32::from(creature_id));
            }
        } else {
            no_object = true;
        }
    } else {
        no_object = true;
    }

    if no_object {
        with_state_mut(|state| {
            state.game.player_free_turn = true;
        });
        terminal::print_message(Some("I do not see anything you can close there."));
    }
}
