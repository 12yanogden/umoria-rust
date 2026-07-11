//! Magical treasure enchantment

use crate::config::identification::{ID_NO_SHOW_P1, ID_SHOW_HIT_DAM, ID_SHOW_P1};
use crate::config::treasure::chests::{
    CH_EXPLODE, CH_LOCKED, CH_LOSE_STR, CH_PARALYSED, CH_POISON, CH_SUMMON,
};
use crate::config::treasure::flags::{
    TR_AGGRAVATE, TR_BLIND, TR_CHR, TR_CON, TR_CURSED, TR_DEX, TR_FFALL, TR_FLAME_TONGUE,
    TR_FREE_ACT, TR_FROST_BRAND, TR_INFRA, TR_INT, TR_REGEN, TR_RES_ACID, TR_RES_COLD, TR_RES_FIRE,
    TR_RES_LIGHT, TR_SEARCH, TR_SEE_INVIS, TR_SLAY_ANIMAL, TR_SLAY_DRAGON, TR_SLAY_EVIL,
    TR_SLAY_UNDEAD, TR_SPEED, TR_STEALTH, TR_STR, TR_SUST_STAT, TR_TELEPORT, TR_TIMID, TR_WIS,
};
use crate::config::treasure::{
    LEVEL_MIN_OBJECT_STD, LEVEL_STD_OBJECT_ADJUST, OBJECT_BASE_MAGIC, OBJECT_CHANCE_CURSED,
    OBJECT_CHANCE_SPECIAL, OBJECT_MAX_BASE_MAGIC,
};
use crate::dice::max_dice_roll;
use crate::game::{
    random_number_normal_distribution_state, random_number_state, with_state_mut, State,
};
use crate::identification::SpecialNameIds;
use crate::inventory::Inventory;

pub const TV_NEVER: i8 = -1;
pub const TV_NOTHING: u8 = 0;
pub const TV_MISC: u8 = 1;
pub const TV_CHEST: u8 = 2;

pub const TV_MIN_WEAR: u8 = 10;
pub const TV_MIN_ENCHANT: u8 = 10;
pub const TV_SLING_AMMO: u8 = 10;
pub const TV_BOLT: u8 = 11;
pub const TV_ARROW: u8 = 12;
pub const TV_SPIKE: u8 = 13;
pub const TV_LIGHT: u8 = 15;
pub const TV_BOW: u8 = 20;
pub const TV_HAFTED: u8 = 21;
pub const TV_POLEARM: u8 = 22;
pub const TV_SWORD: u8 = 23;
pub const TV_DIGGING: u8 = 25;
pub const TV_BOOTS: u8 = 30;
pub const TV_GLOVES: u8 = 31;
pub const TV_CLOAK: u8 = 32;
pub const TV_HELM: u8 = 33;
pub const TV_SHIELD: u8 = 34;
pub const TV_HARD_ARMOR: u8 = 35;
pub const TV_SOFT_ARMOR: u8 = 36;
pub const TV_MAX_ENCHANT: u8 = 39;
pub const TV_AMULET: u8 = 40;
pub const TV_RING: u8 = 45;
pub const TV_MAX_WEAR: u8 = 50;

pub const TV_STAFF: u8 = 55;
pub const TV_WAND: u8 = 65;
pub const TV_SCROLL1: u8 = 70;
pub const TV_SCROLL2: u8 = 71;
pub const TV_POTION1: u8 = 75;
pub const TV_POTION2: u8 = 76;
pub const TV_FLASK: u8 = 77;
pub const TV_FOOD: u8 = 80;
pub const TV_MAGIC_BOOK: u8 = 90;
pub const TV_PRAYER_BOOK: u8 = 91;
pub const TV_MAX_OBJECT: u8 = 99;
pub const TV_GOLD: u8 = 100;
pub const TV_MAX_PICK_UP: u8 = 100;
pub const TV_INVIS_TRAP: u8 = 101;

pub const TV_MIN_VISIBLE: u8 = 102;
pub const TV_VIS_TRAP: u8 = 102;
pub const TV_RUBBLE: u8 = 103;
pub const TV_MIN_DOORS: u8 = 104;
pub const TV_OPEN_DOOR: u8 = 104;
pub const TV_CLOSED_DOOR: u8 = 105;
pub const TV_UP_STAIR: u8 = 107;
pub const TV_DOWN_STAIR: u8 = 108;
pub const TV_SECRET_DOOR: u8 = 109;
pub const TV_STORE_DOOR: u8 = 110;
pub const TV_MAX_VISIBLE: u8 = 110;

#[doc(hidden)]
pub fn magic_should_be_enchanted(state: &mut State, chance: i32) -> bool {
    random_number_state(state, 100) <= chance
}

#[doc(hidden)]
pub fn magic_enchantment_bonus(state: &mut State, base: i32, max_standard: i32, level: i32) -> i32 {
    let mut stand_deviation =
        (i32::from(LEVEL_STD_OBJECT_ADJUST) * level / 100) + i32::from(LEVEL_MIN_OBJECT_STD);

    if stand_deviation > max_standard || level > max_standard {
        stand_deviation = max_standard;
    }

    let abs_distribution = random_number_normal_distribution_state(state, 0, stand_deviation).abs();
    let mut bonus = (abs_distribution / 10) + base;

    if bonus < base {
        bonus = base;
    }

    bonus
}

fn item_ptr(state: &mut State, item_id: i32) -> *mut Inventory {
    std::ptr::addr_of_mut!(state.game.treasure.list[item_id as usize])
}

fn magical_armor(state: &mut State, item_id: i32, special: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        (*ptr).to_ac += magic_enchantment_bonus(state, 1, 30, level) as i16;

        if !magic_should_be_enchanted(state, special) {
            return;
        }

        match random_number_state(state, 9) {
            1 => {
                (*ptr).flags |= TR_RES_LIGHT | TR_RES_COLD | TR_RES_ACID | TR_RES_FIRE;
                (*ptr).special_name_id = SpecialNameIds::SN_R as u8;
                (*ptr).to_ac += 5;
                (*ptr).cost += 2500;
            }
            2 => {
                (*ptr).flags |= TR_RES_ACID;
                (*ptr).special_name_id = SpecialNameIds::SN_RA as u8;
                (*ptr).cost += 1000;
            }
            3 | 4 => {
                (*ptr).flags |= TR_RES_FIRE;
                (*ptr).special_name_id = SpecialNameIds::SN_RF as u8;
                (*ptr).cost += 600;
            }
            5 | 6 => {
                (*ptr).flags |= TR_RES_COLD;
                (*ptr).special_name_id = SpecialNameIds::SN_RC as u8;
                (*ptr).cost += 600;
            }
            7..=9 => {
                (*ptr).flags |= TR_RES_LIGHT;
                (*ptr).special_name_id = SpecialNameIds::SN_RL as u8;
                (*ptr).cost += 500;
            }
            _ => {}
        }
    }
}

fn cursed_armor(state: &mut State, item_id: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        (*ptr).to_ac -= magic_enchantment_bonus(state, 1, 40, level) as i16;
        (*ptr).cost = 0;
        (*ptr).flags |= TR_CURSED;
    }
}

fn magical_sword(state: &mut State, item_id: i32, special: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        (*ptr).to_hit += magic_enchantment_bonus(state, 0, 40, level) as i16;

        let damage_bonus = max_dice_roll((*ptr).damage);
        (*ptr).to_damage +=
            magic_enchantment_bonus(state, 0, 4 * damage_bonus, damage_bonus * level / 10) as i16;

        if magic_should_be_enchanted(state, 3 * special / 2) {
            match random_number_state(state, 16) {
                1 => {
                    (*ptr).flags |=
                        TR_SEE_INVIS | TR_SUST_STAT | TR_SLAY_UNDEAD | TR_SLAY_EVIL | TR_STR;
                    (*ptr).to_hit += 5;
                    (*ptr).to_damage += 5;
                    (*ptr).to_ac += random_number_state(state, 4) as i16;
                    (*ptr).misc_use = random_number_state(state, 4) as i16;
                    (*ptr).special_name_id = SpecialNameIds::SN_HA as u8;
                    (*ptr).cost += i32::from((*ptr).misc_use) * 500;
                    (*ptr).cost += 10000;
                }
                2 => {
                    (*ptr).flags |= TR_FFALL
                        | TR_RES_LIGHT
                        | TR_SEE_INVIS
                        | TR_FREE_ACT
                        | TR_RES_COLD
                        | TR_RES_ACID
                        | TR_RES_FIRE
                        | TR_REGEN
                        | TR_STEALTH;
                    (*ptr).to_hit += 3;
                    (*ptr).to_damage += 3;
                    (*ptr).to_ac += (5 + random_number_state(state, 5)) as i16;
                    (*ptr).special_name_id = SpecialNameIds::SN_DF as u8;
                    (*ptr).misc_use = random_number_state(state, 3) as i16;
                    (*ptr).cost += i32::from((*ptr).misc_use) * 500;
                    (*ptr).cost += 7500;
                }
                3 | 4 => {
                    (*ptr).flags |= TR_SLAY_ANIMAL;
                    (*ptr).to_hit += 2;
                    (*ptr).to_damage += 2;
                    (*ptr).special_name_id = SpecialNameIds::SN_SA as u8;
                    (*ptr).cost += 3000;
                }
                5 | 6 => {
                    (*ptr).flags |= TR_SLAY_DRAGON;
                    (*ptr).to_hit += 3;
                    (*ptr).to_damage += 3;
                    (*ptr).special_name_id = SpecialNameIds::SN_SD as u8;
                    (*ptr).cost += 4000;
                }
                7 | 8 => {
                    (*ptr).flags |= TR_SLAY_EVIL;
                    (*ptr).to_hit += 3;
                    (*ptr).to_damage += 3;
                    (*ptr).special_name_id = SpecialNameIds::SN_SE as u8;
                    (*ptr).cost += 4000;
                }
                9 | 10 => {
                    (*ptr).flags |= TR_SEE_INVIS | TR_SLAY_UNDEAD;
                    (*ptr).to_hit += 3;
                    (*ptr).to_damage += 3;
                    (*ptr).special_name_id = SpecialNameIds::SN_SU as u8;
                    (*ptr).cost += 5000;
                }
                11..=13 => {
                    (*ptr).flags |= TR_FLAME_TONGUE;
                    (*ptr).to_hit += 1;
                    (*ptr).to_damage += 3;
                    (*ptr).special_name_id = SpecialNameIds::SN_FT as u8;
                    (*ptr).cost += 2000;
                }
                14..=16 => {
                    (*ptr).flags |= TR_FROST_BRAND;
                    (*ptr).to_hit += 1;
                    (*ptr).to_damage += 1;
                    (*ptr).special_name_id = SpecialNameIds::SN_FB as u8;
                    (*ptr).cost += 1200;
                }
                _ => {}
            }
        }
    }
}

fn cursed_sword(state: &mut State, item_id: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        (*ptr).to_hit -= magic_enchantment_bonus(state, 1, 55, level) as i16;

        let damage_bonus = max_dice_roll((*ptr).damage);
        (*ptr).to_damage -=
            magic_enchantment_bonus(state, 1, 11 * damage_bonus / 2, damage_bonus * level / 10)
                as i16;
        (*ptr).flags |= TR_CURSED;
        (*ptr).cost = 0;
    }
}

fn magical_bow(state: &mut State, item_id: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        (*ptr).to_hit += magic_enchantment_bonus(state, 1, 30, level) as i16;
        (*ptr).to_damage += magic_enchantment_bonus(state, 1, 20, level) as i16;
    }
}

fn cursed_bow(state: &mut State, item_id: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        (*ptr).to_hit -= magic_enchantment_bonus(state, 1, 50, level) as i16;
        (*ptr).to_damage -= magic_enchantment_bonus(state, 1, 30, level) as i16;
        (*ptr).flags |= TR_CURSED;
        (*ptr).cost = 0;
    }
}

fn magical_digging_tool(state: &mut State, item_id: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        (*ptr).misc_use += magic_enchantment_bonus(state, 0, 25, level) as i16;
    }
}

fn cursed_digging_tool(state: &mut State, item_id: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        (*ptr).misc_use = (-magic_enchantment_bonus(state, 1, 30, level)) as i16;
        (*ptr).cost = 0;
        (*ptr).flags |= TR_CURSED;
    }
}

fn magical_gloves(state: &mut State, item_id: i32, special: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        (*ptr).to_ac += magic_enchantment_bonus(state, 1, 20, level) as i16;

        if !magic_should_be_enchanted(state, special) {
            return;
        }

        if random_number_state(state, 2) == 1 {
            (*ptr).flags |= TR_FREE_ACT;
            (*ptr).special_name_id = SpecialNameIds::SN_FREE_ACTION as u8;
            (*ptr).cost += 1000;
        } else {
            (*ptr).identification |= ID_SHOW_HIT_DAM;
            (*ptr).to_hit += (1 + random_number_state(state, 3)) as i16;
            (*ptr).to_damage += (1 + random_number_state(state, 3)) as i16;
            (*ptr).special_name_id = SpecialNameIds::SN_SLAYING as u8;
            (*ptr).cost += i32::from((*ptr).to_hit + (*ptr).to_damage) * 250;
        }
    }
}

fn cursed_gloves(state: &mut State, item_id: i32, special: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        if magic_should_be_enchanted(state, special) {
            if random_number_state(state, 2) == 1 {
                (*ptr).flags |= TR_DEX;
                (*ptr).special_name_id = SpecialNameIds::SN_CLUMSINESS as u8;
            } else {
                (*ptr).flags |= TR_STR;
                (*ptr).special_name_id = SpecialNameIds::SN_WEAKNESS as u8;
            }
            (*ptr).identification |= ID_SHOW_P1;
            (*ptr).misc_use = (-magic_enchantment_bonus(state, 1, 10, level)) as i16;
        }

        (*ptr).to_ac -= magic_enchantment_bonus(state, 1, 40, level) as i16;
        (*ptr).flags |= TR_CURSED;
        (*ptr).cost = 0;
    }
}

fn magical_boots(state: &mut State, item_id: i32, special: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        (*ptr).to_ac += magic_enchantment_bonus(state, 1, 20, level) as i16;

        if !magic_should_be_enchanted(state, special) {
            return;
        }

        let magic_type = random_number_state(state, 12);

        if magic_type > 5 {
            (*ptr).flags |= TR_FFALL;
            (*ptr).special_name_id = SpecialNameIds::SN_SLOW_DESCENT as u8;
            (*ptr).cost += 250;
        } else if magic_type == 1 {
            (*ptr).flags |= TR_SPEED;
            (*ptr).special_name_id = SpecialNameIds::SN_SPEED as u8;
            (*ptr).identification |= ID_SHOW_P1;
            (*ptr).misc_use = 1;
            (*ptr).cost += 5000;
        } else {
            (*ptr).flags |= TR_STEALTH;
            (*ptr).identification |= ID_SHOW_P1;
            (*ptr).misc_use = random_number_state(state, 3) as i16;
            (*ptr).special_name_id = SpecialNameIds::SN_STEALTH as u8;
            (*ptr).cost += 500;
        }
    }
}

fn cursed_boots(state: &mut State, item_id: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        let magic_type = random_number_state(state, 3);

        match magic_type {
            1 => {
                (*ptr).flags |= TR_SPEED;
                (*ptr).special_name_id = SpecialNameIds::SN_SLOWNESS as u8;
                (*ptr).identification |= ID_SHOW_P1;
                (*ptr).misc_use = -1;
            }
            2 => {
                (*ptr).flags |= TR_AGGRAVATE;
                (*ptr).special_name_id = SpecialNameIds::SN_NOISE as u8;
            }
            _ => {
                (*ptr).special_name_id = SpecialNameIds::SN_GREAT_MASS as u8;
                (*ptr).weight = ((*ptr).weight as u32 * 5) as u16;
            }
        }

        (*ptr).cost = 0;
        (*ptr).to_ac -= magic_enchantment_bonus(state, 2, 45, level) as i16;
        (*ptr).flags |= TR_CURSED;
    }
}

fn magical_helms(state: &mut State, item_id: i32, special: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        (*ptr).to_ac += magic_enchantment_bonus(state, 1, 20, level) as i16;

        if !magic_should_be_enchanted(state, special) {
            return;
        }

        if (*ptr).sub_category_id < 6 {
            (*ptr).identification |= ID_SHOW_P1;

            match random_number_state(state, 3) {
                1 => {
                    (*ptr).misc_use = random_number_state(state, 2) as i16;
                    (*ptr).flags |= TR_INT;
                    (*ptr).special_name_id = SpecialNameIds::SN_INTELLIGENCE as u8;
                    (*ptr).cost += i32::from((*ptr).misc_use) * 500;
                }
                2 => {
                    (*ptr).misc_use = random_number_state(state, 2) as i16;
                    (*ptr).flags |= TR_WIS;
                    (*ptr).special_name_id = SpecialNameIds::SN_WISDOM as u8;
                    (*ptr).cost += i32::from((*ptr).misc_use) * 500;
                }
                _ => {
                    (*ptr).misc_use = (1 + random_number_state(state, 4)) as i16;
                    (*ptr).flags |= TR_INFRA;
                    (*ptr).special_name_id = SpecialNameIds::SN_INFRAVISION as u8;
                    (*ptr).cost += i32::from((*ptr).misc_use) * 250;
                }
            }
            return;
        }

        match random_number_state(state, 6) {
            1 => {
                (*ptr).identification |= ID_SHOW_P1;
                (*ptr).misc_use = random_number_state(state, 3) as i16;
                (*ptr).flags |= TR_FREE_ACT | TR_CON | TR_DEX | TR_STR;
                (*ptr).special_name_id = SpecialNameIds::SN_MIGHT as u8;
                (*ptr).cost += 1000 + i32::from((*ptr).misc_use) * 500;
            }
            2 => {
                (*ptr).identification |= ID_SHOW_P1;
                (*ptr).misc_use = random_number_state(state, 3) as i16;
                (*ptr).flags |= TR_CHR | TR_WIS;
                (*ptr).special_name_id = SpecialNameIds::SN_LORDLINESS as u8;
                (*ptr).cost += 1000 + i32::from((*ptr).misc_use) * 500;
            }
            3 => {
                (*ptr).identification |= ID_SHOW_P1;
                (*ptr).misc_use = random_number_state(state, 3) as i16;
                (*ptr).flags |= TR_RES_LIGHT | TR_RES_COLD | TR_RES_ACID | TR_RES_FIRE | TR_INT;
                (*ptr).special_name_id = SpecialNameIds::SN_MAGI as u8;
                (*ptr).cost += 3000 + i32::from((*ptr).misc_use) * 500;
            }
            4 => {
                (*ptr).identification |= ID_SHOW_P1;
                (*ptr).misc_use = random_number_state(state, 3) as i16;
                (*ptr).flags |= TR_CHR;
                (*ptr).special_name_id = SpecialNameIds::SN_BEAUTY as u8;
                (*ptr).cost += 750;
            }
            5 => {
                (*ptr).identification |= ID_SHOW_P1;
                (*ptr).misc_use = (5 * (1 + random_number_state(state, 4))) as i16;
                (*ptr).flags |= TR_SEE_INVIS | TR_SEARCH;
                (*ptr).special_name_id = SpecialNameIds::SN_SEEING as u8;
                (*ptr).cost += 1000 + i32::from((*ptr).misc_use) * 100;
            }
            6 => {
                (*ptr).flags |= TR_REGEN;
                (*ptr).special_name_id = SpecialNameIds::SN_REGENERATION as u8;
                (*ptr).cost += 1500;
            }
            _ => {}
        }
    }
}

fn cursed_helms(state: &mut State, item_id: i32, special: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        (*ptr).to_ac -= magic_enchantment_bonus(state, 1, 45, level) as i16;
        (*ptr).flags |= TR_CURSED;
        (*ptr).cost = 0;

        if !magic_should_be_enchanted(state, special) {
            return;
        }

        match random_number_state(state, 7) {
            1 => {
                (*ptr).identification |= ID_SHOW_P1;
                (*ptr).misc_use = (-random_number_state(state, 5)) as i16;
                (*ptr).flags |= TR_INT;
                (*ptr).special_name_id = SpecialNameIds::SN_STUPIDITY as u8;
            }
            2 => {
                (*ptr).identification |= ID_SHOW_P1;
                (*ptr).misc_use = (-random_number_state(state, 5)) as i16;
                (*ptr).flags |= TR_WIS;
                (*ptr).special_name_id = SpecialNameIds::SN_DULLNESS as u8;
            }
            3 => {
                (*ptr).flags |= TR_BLIND;
                (*ptr).special_name_id = SpecialNameIds::SN_BLINDNESS as u8;
            }
            4 => {
                (*ptr).flags |= TR_TIMID;
                (*ptr).special_name_id = SpecialNameIds::SN_TIMIDNESS as u8;
            }
            5 => {
                (*ptr).identification |= ID_SHOW_P1;
                (*ptr).misc_use = (-random_number_state(state, 5)) as i16;
                (*ptr).flags |= TR_STR;
                (*ptr).special_name_id = SpecialNameIds::SN_WEAKNESS as u8;
            }
            6 => {
                (*ptr).flags |= TR_TELEPORT;
                (*ptr).special_name_id = SpecialNameIds::SN_TELEPORTATION as u8;
            }
            7 => {
                (*ptr).identification |= ID_SHOW_P1;
                (*ptr).misc_use = (-random_number_state(state, 5)) as i16;
                (*ptr).flags |= TR_CHR;
                (*ptr).special_name_id = SpecialNameIds::SN_UGLINESS as u8;
            }
            _ => {}
        }
    }
}

fn process_rings(state: &mut State, item_id: i32, level: i32, cursed: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        match (*ptr).sub_category_id {
            0..=3 => {
                if magic_should_be_enchanted(state, cursed) {
                    (*ptr).misc_use = (-magic_enchantment_bonus(state, 1, 20, level)) as i16;
                    (*ptr).flags |= TR_CURSED;
                    (*ptr).cost = -(*ptr).cost;
                } else {
                    (*ptr).misc_use = magic_enchantment_bonus(state, 1, 10, level) as i16;
                    (*ptr).cost += i32::from((*ptr).misc_use) * 100;
                }
            }
            4 => {
                if magic_should_be_enchanted(state, cursed) {
                    (*ptr).misc_use = (-random_number_state(state, 3)) as i16;
                    (*ptr).flags |= TR_CURSED;
                    (*ptr).cost = -(*ptr).cost;
                } else {
                    (*ptr).misc_use = 1;
                }
            }
            5 => {
                (*ptr).misc_use = (5 * magic_enchantment_bonus(state, 1, 20, level)) as i16;
                (*ptr).cost += i32::from((*ptr).misc_use) * 50;
                if magic_should_be_enchanted(state, cursed) {
                    (*ptr).misc_use = -(*ptr).misc_use;
                    (*ptr).flags |= TR_CURSED;
                    (*ptr).cost = -(*ptr).cost;
                }
            }
            19 => {
                (*ptr).to_damage += magic_enchantment_bonus(state, 1, 20, level) as i16;
                (*ptr).cost += i32::from((*ptr).to_damage) * 100;
                if magic_should_be_enchanted(state, cursed) {
                    (*ptr).to_damage = -(*ptr).to_damage;
                    (*ptr).flags |= TR_CURSED;
                    (*ptr).cost = -(*ptr).cost;
                }
            }
            20 => {
                (*ptr).to_hit += magic_enchantment_bonus(state, 1, 20, level) as i16;
                (*ptr).cost += i32::from((*ptr).to_hit) * 100;
                if magic_should_be_enchanted(state, cursed) {
                    (*ptr).to_hit = -(*ptr).to_hit;
                    (*ptr).flags |= TR_CURSED;
                    (*ptr).cost = -(*ptr).cost;
                }
            }
            21 => {
                (*ptr).to_ac += magic_enchantment_bonus(state, 1, 20, level) as i16;
                (*ptr).cost += i32::from((*ptr).to_ac) * 100;
                if magic_should_be_enchanted(state, cursed) {
                    (*ptr).to_ac = -(*ptr).to_ac;
                    (*ptr).flags |= TR_CURSED;
                    (*ptr).cost = -(*ptr).cost;
                }
            }
            24..=29 => {
                (*ptr).identification |= ID_NO_SHOW_P1;
            }
            30 => {
                (*ptr).identification |= ID_SHOW_HIT_DAM;
                (*ptr).to_damage += magic_enchantment_bonus(state, 1, 25, level) as i16;
                (*ptr).to_hit += magic_enchantment_bonus(state, 1, 25, level) as i16;
                (*ptr).cost += i32::from((*ptr).to_hit + (*ptr).to_damage) * 100;
                if magic_should_be_enchanted(state, cursed) {
                    (*ptr).to_hit = -(*ptr).to_hit;
                    (*ptr).to_damage = -(*ptr).to_damage;
                    (*ptr).flags |= TR_CURSED;
                    (*ptr).cost = -(*ptr).cost;
                }
            }
            _ => {}
        }
    }
}

fn process_amulets(state: &mut State, item_id: i32, level: i32, cursed: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        if (*ptr).sub_category_id < 2 {
            if magic_should_be_enchanted(state, cursed) {
                (*ptr).misc_use = (-magic_enchantment_bonus(state, 1, 20, level)) as i16;
                (*ptr).flags |= TR_CURSED;
                (*ptr).cost = -(*ptr).cost;
            } else {
                (*ptr).misc_use = magic_enchantment_bonus(state, 1, 10, level) as i16;
                (*ptr).cost += i32::from((*ptr).misc_use) * 100;
            }
        } else if (*ptr).sub_category_id == 2 {
            (*ptr).misc_use = (5 * magic_enchantment_bonus(state, 1, 25, level)) as i16;
            if magic_should_be_enchanted(state, cursed) {
                (*ptr).misc_use = -(*ptr).misc_use;
                (*ptr).cost = -(*ptr).cost;
                (*ptr).flags |= TR_CURSED;
            } else {
                (*ptr).cost += 50 * i32::from((*ptr).misc_use);
            }
        } else if (*ptr).sub_category_id == 8 {
            (*ptr).misc_use = (5 * magic_enchantment_bonus(state, 1, 25, level)) as i16;
            (*ptr).cost += 20 * i32::from((*ptr).misc_use);
        }
    }
}

fn wand_magic(state: &mut State, id: u8) -> i32 {
    match id {
        0 => random_number_state(state, 10) + 6,
        1 => random_number_state(state, 8) + 6,
        2 => random_number_state(state, 5) + 6,
        3 => random_number_state(state, 8) + 6,
        4 => random_number_state(state, 4) + 3,
        5 => random_number_state(state, 8) + 6,
        6 | 7 => random_number_state(state, 20) + 12,
        8 => random_number_state(state, 10) + 6,
        9 => random_number_state(state, 12) + 6,
        10 => random_number_state(state, 10) + 12,
        11 => random_number_state(state, 3) + 3,
        12 => random_number_state(state, 8) + 6,
        13 => random_number_state(state, 10) + 6,
        14 | 15 => random_number_state(state, 5) + 3,
        16 => random_number_state(state, 5) + 6,
        17 => random_number_state(state, 5) + 4,
        18 => random_number_state(state, 8) + 4,
        19 => random_number_state(state, 6) + 2,
        20 => random_number_state(state, 4) + 2,
        21 => random_number_state(state, 8) + 6,
        22 => random_number_state(state, 5) + 2,
        23 => random_number_state(state, 12) + 12,
        _ => -1,
    }
}

fn staff_magic(state: &mut State, id: u8) -> i32 {
    match id {
        0 => random_number_state(state, 20) + 12,
        1 => random_number_state(state, 8) + 6,
        2 => random_number_state(state, 5) + 6,
        3 => random_number_state(state, 20) + 12,
        4 => random_number_state(state, 15) + 6,
        5 => random_number_state(state, 4) + 5,
        6 => random_number_state(state, 5) + 3,
        7 | 8 => random_number_state(state, 3) + 1,
        9 => random_number_state(state, 5) + 6,
        10 => random_number_state(state, 10) + 12,
        11..=13 => random_number_state(state, 5) + 6,
        14 => random_number_state(state, 10) + 12,
        15 => random_number_state(state, 3) + 4,
        16 | 17 => random_number_state(state, 5) + 6,
        18 => random_number_state(state, 3) + 4,
        19 => random_number_state(state, 10) + 12,
        20 | 21 => random_number_state(state, 3) + 4,
        22 => random_number_state(state, 10) + 6,
        _ => -1,
    }
}

fn magical_cloak(state: &mut State, item_id: i32, special: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        if !magic_should_be_enchanted(state, special) {
            (*ptr).to_ac += magic_enchantment_bonus(state, 1, 20, level) as i16;
            return;
        }

        if random_number_state(state, 2) == 1 {
            (*ptr).special_name_id = SpecialNameIds::SN_PROTECTION as u8;
            (*ptr).to_ac += magic_enchantment_bonus(state, 2, 40, level) as i16;
            (*ptr).cost += 250;
            return;
        }

        (*ptr).to_ac += magic_enchantment_bonus(state, 1, 20, level) as i16;
        (*ptr).identification |= ID_SHOW_P1;
        (*ptr).misc_use = random_number_state(state, 3) as i16;
        (*ptr).flags |= TR_STEALTH;
        (*ptr).special_name_id = SpecialNameIds::SN_STEALTH as u8;
        (*ptr).cost += 500;
    }
}

fn cursed_cloak(state: &mut State, item_id: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        let magic_type = random_number_state(state, 3);

        match magic_type {
            1 => {
                (*ptr).flags |= TR_AGGRAVATE;
                (*ptr).special_name_id = SpecialNameIds::SN_IRRITATION as u8;
                (*ptr).to_ac -= magic_enchantment_bonus(state, 1, 10, level) as i16;
                (*ptr).identification |= ID_SHOW_HIT_DAM;
                (*ptr).to_hit -= magic_enchantment_bonus(state, 1, 10, level) as i16;
                (*ptr).to_damage -= magic_enchantment_bonus(state, 1, 10, level) as i16;
                (*ptr).cost = 0;
            }
            2 => {
                (*ptr).special_name_id = SpecialNameIds::SN_VULNERABILITY as u8;
                (*ptr).to_ac -= magic_enchantment_bonus(state, 10, 100, level + 50) as i16;
                (*ptr).cost = 0;
            }
            _ => {
                (*ptr).special_name_id = SpecialNameIds::SN_ENVELOPING as u8;
                (*ptr).to_ac -= magic_enchantment_bonus(state, 1, 10, level) as i16;
                (*ptr).identification |= ID_SHOW_HIT_DAM;
                (*ptr).to_hit -= magic_enchantment_bonus(state, 2, 40, level + 10) as i16;
                (*ptr).to_damage -= magic_enchantment_bonus(state, 2, 40, level + 10) as i16;
                (*ptr).cost = 0;
            }
        }

        (*ptr).flags |= TR_CURSED;
    }
}

fn magical_chests(state: &mut State, item_id: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        let magic_type = random_number_state(state, level + 4);

        match magic_type {
            1 => {
                (*ptr).flags = 0;
                (*ptr).special_name_id = SpecialNameIds::SN_EMPTY as u8;
            }
            2 => {
                (*ptr).flags |= CH_LOCKED;
                (*ptr).special_name_id = SpecialNameIds::SN_LOCKED as u8;
            }
            3 | 4 => {
                (*ptr).flags |= CH_LOSE_STR | CH_LOCKED;
                (*ptr).special_name_id = SpecialNameIds::SN_POISON_NEEDLE as u8;
            }
            5 | 6 => {
                (*ptr).flags |= CH_POISON | CH_LOCKED;
                (*ptr).special_name_id = SpecialNameIds::SN_POISON_NEEDLE as u8;
            }
            7..=9 => {
                (*ptr).flags |= CH_PARALYSED | CH_LOCKED;
                (*ptr).special_name_id = SpecialNameIds::SN_GAS_TRAP as u8;
            }
            10 | 11 => {
                (*ptr).flags |= CH_EXPLODE | CH_LOCKED;
                (*ptr).special_name_id = SpecialNameIds::SN_EXPLOSION_DEVICE as u8;
            }
            12..=14 => {
                (*ptr).flags |= CH_SUMMON | CH_LOCKED;
                (*ptr).special_name_id = SpecialNameIds::SN_SUMMONING_RUNES as u8;
            }
            15..=17 => {
                (*ptr).flags |= CH_PARALYSED | CH_POISON | CH_LOSE_STR | CH_LOCKED;
                (*ptr).special_name_id = SpecialNameIds::SN_MULTIPLE_TRAPS as u8;
            }
            _ => {
                (*ptr).flags |= CH_SUMMON | CH_EXPLODE | CH_LOCKED;
                (*ptr).special_name_id = SpecialNameIds::SN_MULTIPLE_TRAPS as u8;
            }
        }
    }
}

fn magical_projectile_adjustment(state: &mut State, item_id: i32, special: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        (*ptr).to_hit += magic_enchantment_bonus(state, 1, 35, level) as i16;
        (*ptr).to_damage += magic_enchantment_bonus(state, 1, 35, level) as i16;

        if magic_should_be_enchanted(state, 3 * special / 2) {
            match random_number_state(state, 10) {
                1..=3 => {
                    (*ptr).special_name_id = SpecialNameIds::SN_SLAYING as u8;
                    (*ptr).to_hit += 5;
                    (*ptr).to_damage += 5;
                    (*ptr).cost += 20;
                }
                4 | 5 => {
                    (*ptr).flags |= TR_FLAME_TONGUE;
                    (*ptr).to_hit += 2;
                    (*ptr).to_damage += 4;
                    (*ptr).special_name_id = SpecialNameIds::SN_FIRE as u8;
                    (*ptr).cost += 25;
                }
                6 | 7 => {
                    (*ptr).flags |= TR_SLAY_EVIL;
                    (*ptr).to_hit += 3;
                    (*ptr).to_damage += 3;
                    (*ptr).special_name_id = SpecialNameIds::SN_SLAY_EVIL as u8;
                    (*ptr).cost += 25;
                }
                8 | 9 => {
                    (*ptr).flags |= TR_SLAY_ANIMAL;
                    (*ptr).to_hit += 2;
                    (*ptr).to_damage += 2;
                    (*ptr).special_name_id = SpecialNameIds::SN_SLAY_ANIMAL as u8;
                    (*ptr).cost += 30;
                }
                10 => {
                    (*ptr).flags |= TR_SLAY_DRAGON;
                    (*ptr).to_hit += 3;
                    (*ptr).to_damage += 3;
                    (*ptr).special_name_id = SpecialNameIds::SN_DRAGON_SLAYING as u8;
                    (*ptr).cost += 35;
                }
                _ => {}
            }
        }
    }
}

fn cursed_projectile_adjustment(state: &mut State, item_id: i32, level: i32) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        (*ptr).to_hit -= magic_enchantment_bonus(state, 5, 55, level) as i16;
        (*ptr).to_damage -= magic_enchantment_bonus(state, 5, 55, level) as i16;
        (*ptr).flags |= TR_CURSED;
        (*ptr).cost = 0;
    }
}

fn magical_projectile(
    state: &mut State,
    item_id: i32,
    special: i32,
    level: i32,
    chance: i32,
    cursed: i32,
) {
    let ptr = item_ptr(state, item_id);
    unsafe {
        if (*ptr).category_id == TV_SLING_AMMO
            || (*ptr).category_id == TV_BOLT
            || (*ptr).category_id == TV_ARROW
        {
            (*ptr).identification |= ID_SHOW_HIT_DAM;

            if magic_should_be_enchanted(state, chance) {
                magical_projectile_adjustment(state, item_id, special, level);
            } else if magic_should_be_enchanted(state, cursed) {
                cursed_projectile_adjustment(state, item_id, level);
            }
        }

        (*ptr).items_count = 0;

        for _ in 0..7 {
            (*ptr).items_count = (*ptr)
                .items_count
                .wrapping_add(random_number_state(state, 6) as u8);
        }

        if state.missiles_counter == i16::MAX {
            state.missiles_counter = i16::MIN;
        } else {
            state.missiles_counter += 1;
        }

        (*ptr).misc_use = state.missiles_counter;
    }
}

pub(crate) fn magic_treasure_magical_ability_state(state: &mut State, item_id: i32, level: i32) {
    let mut chance = i32::from(OBJECT_BASE_MAGIC) + level;
    if chance > i32::from(OBJECT_MAX_BASE_MAGIC) {
        chance = i32::from(OBJECT_MAX_BASE_MAGIC);
    }

    let mut special = chance / i32::from(OBJECT_CHANCE_SPECIAL);
    let cursed = (10 * chance) / i32::from(OBJECT_CHANCE_CURSED);

    let (category_id, sub_category_id, cost) = {
        let item = &state.game.treasure.list[item_id as usize];
        (item.category_id, item.sub_category_id, item.cost)
    };

    match category_id {
        TV_SHIELD | TV_HARD_ARMOR | TV_SOFT_ARMOR => {
            if magic_should_be_enchanted(state, chance) {
                magical_armor(state, item_id, special, level);
            } else if magic_should_be_enchanted(state, cursed) {
                cursed_armor(state, item_id, level);
            }
        }
        TV_HAFTED | TV_POLEARM | TV_SWORD => {
            let ptr = item_ptr(state, item_id);
            unsafe {
                (*ptr).identification |= ID_SHOW_HIT_DAM;
            }

            if magic_should_be_enchanted(state, chance) {
                magical_sword(state, item_id, special, level);
            } else if magic_should_be_enchanted(state, cursed) {
                cursed_sword(state, item_id, level);
            }
        }
        TV_BOW => {
            let ptr = item_ptr(state, item_id);
            unsafe {
                (*ptr).identification |= ID_SHOW_HIT_DAM;
            }

            if magic_should_be_enchanted(state, chance) {
                magical_bow(state, item_id, level);
            } else if magic_should_be_enchanted(state, cursed) {
                cursed_bow(state, item_id, level);
            }
        }
        TV_DIGGING => {
            let ptr = item_ptr(state, item_id);
            unsafe {
                (*ptr).identification |= ID_SHOW_HIT_DAM;
            }

            if magic_should_be_enchanted(state, chance) {
                if random_number_state(state, 3) < 3 {
                    magical_digging_tool(state, item_id, level);
                } else {
                    cursed_digging_tool(state, item_id, level);
                }
            }
        }
        TV_GLOVES => {
            if magic_should_be_enchanted(state, chance) {
                magical_gloves(state, item_id, special, level);
            } else if magic_should_be_enchanted(state, cursed) {
                cursed_gloves(state, item_id, special, level);
            }
        }
        TV_BOOTS => {
            if magic_should_be_enchanted(state, chance) {
                magical_boots(state, item_id, special, level);
            } else if magic_should_be_enchanted(state, cursed) {
                cursed_boots(state, item_id, level);
            }
        }
        TV_HELM => {
            if (6..=8).contains(&sub_category_id) {
                chance += cost / 100;
                special += special;
            }

            if magic_should_be_enchanted(state, chance) {
                magical_helms(state, item_id, special, level);
            } else if magic_should_be_enchanted(state, cursed) {
                cursed_helms(state, item_id, special, level);
            }
        }
        TV_RING => {
            process_rings(state, item_id, level, cursed);
        }
        TV_AMULET => {
            process_amulets(state, item_id, level, cursed);
        }
        TV_LIGHT if (sub_category_id % 2) == 1 => {
            let ptr = item_ptr(state, item_id);
            unsafe {
                let misc_cap = (*ptr).misc_use;
                (*ptr).misc_use = random_number_state(state, i32::from(misc_cap)) as i16;
                (*ptr).sub_category_id -= 1;
            }
        }
        TV_WAND => {
            let magic_amount = wand_magic(state, sub_category_id);
            if magic_amount != -1 {
                let ptr = item_ptr(state, item_id);
                unsafe {
                    (*ptr).misc_use = magic_amount as u16 as i16;
                }
            }
        }
        TV_STAFF => {
            let magic_amount = staff_magic(state, sub_category_id);
            if magic_amount != -1 {
                let ptr = item_ptr(state, item_id);
                unsafe {
                    (*ptr).misc_use = magic_amount as u16 as i16;
                }
            }

            if sub_category_id == 7 {
                let ptr = item_ptr(state, item_id);
                unsafe {
                    (*ptr).depth_first_found = 10;
                }
            } else if sub_category_id == 22 {
                let ptr = item_ptr(state, item_id);
                unsafe {
                    (*ptr).depth_first_found = 5;
                }
            }
        }
        TV_CLOAK => {
            if magic_should_be_enchanted(state, chance) {
                magical_cloak(state, item_id, special, level);
            } else if magic_should_be_enchanted(state, cursed) {
                cursed_cloak(state, item_id, level);
            }
        }
        TV_CHEST => {
            magical_chests(state, item_id, level);
        }
        TV_SLING_AMMO | TV_SPIKE | TV_BOLT | TV_ARROW => {
            magical_projectile(state, item_id, special, level, chance, cursed);
        }
        TV_FOOD => {
            if sub_category_id == 90 {
                let ptr = item_ptr(state, item_id);
                unsafe {
                    (*ptr).depth_first_found = 0;
                }
            }
            if sub_category_id == 92 {
                let ptr = item_ptr(state, item_id);
                unsafe {
                    (*ptr).depth_first_found = 6;
                }
            }
        }
        TV_SCROLL1 => {
            let ptr = item_ptr(state, item_id);
            unsafe {
                if sub_category_id == 67 {
                    (*ptr).depth_first_found = 1;
                } else if sub_category_id == 69 {
                    (*ptr).depth_first_found = 0;
                } else if sub_category_id == 80 || sub_category_id == 81 {
                    (*ptr).depth_first_found = 5;
                }
            }
        }
        TV_POTION1 if sub_category_id == 76 => {
            let ptr = item_ptr(state, item_id);
            unsafe {
                (*ptr).depth_first_found = 0;
            }
        }
        _ => {}
    }
}

#[doc(hidden)]
pub fn wand_magic_charges(id: u8) -> i32 {
    with_state_mut(|state| wand_magic(state, id))
}

#[doc(hidden)]
pub fn staff_magic_charges(id: u8) -> i32 {
    with_state_mut(|state| staff_magic(state, id))
}

pub fn magic_treasure_magical_ability(item_id: i32, level: i32) {
    with_state_mut(|state| magic_treasure_magical_ability_state(state, item_id, level));
}
