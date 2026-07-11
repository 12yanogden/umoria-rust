//! Player stat calculations and adjustments

use crate::config::player::status::{PY_HP, PY_STR, PY_STR_WGT};
use crate::config::spells::{SPELL_TYPE_MAGE, SPELL_TYPE_PRIEST};
use crate::data_player::{BLOWS_TABLE, CLASSES};
use crate::game::{random_number, with_state, with_state_mut};
use crate::player::{
    player_calculate_allowed_spells_count, player_gain_mana, player_recalculate_bonuses, PlayerAttr,
};
use crate::ui::display_character_stats;

fn constitution_adj_from_value(con: i32) -> i32 {
    if con < 7 {
        con - 7
    } else if con < 17 {
        0
    } else if con == 17 {
        1
    } else if con < 94 {
        2
    } else if con < 117 {
        3
    } else {
        4
    }
}

fn modify_stat_value(current: u8, amount: i16) -> u8 {
    let mut new_stat = current;
    let loop_count = if amount < 0 {
        i32::from(-amount)
    } else {
        i32::from(amount)
    };

    for _ in 0..loop_count {
        if amount > 0 {
            if new_stat < 18 {
                new_stat = new_stat.wrapping_add(1);
            } else if new_stat < 108 {
                new_stat = new_stat.wrapping_add(10);
            } else {
                new_stat = 118;
            }
        } else if new_stat > 27 {
            new_stat = new_stat.wrapping_sub(10);
        } else if new_stat > 18 {
            new_stat = 18;
        } else if new_stat > 3 {
            new_stat = new_stat.wrapping_sub(1);
        }
    }

    new_stat
}

/// 21
pub fn player_initialize_base_experience_levels() {
    const LEVELS: [u32; 40] = [
        10, 25, 45, 70, 100, 140, 200, 280, 380, 500, 650, 850, 1100, 1400, 1800, 2300, 2900, 3600,
        4400, 5400, 6800, 8400, 10200, 12500, 17500, 25000, 35000, 50000, 75000, 100000, 150000,
        200000, 300000, 400000, 500000, 750000, 1500000, 2500000, 5000000, 10000000,
    ];

    with_state_mut(|state| {
        state.py.base_exp_levels.copy_from_slice(&LEVELS);
    });
}

/// 52
pub fn player_calculate_hit_points() {
    with_state_mut(|state| {
        let level = i32::from(state.py.misc.level);
        let base_hp = i32::from(state.py.base_hp_levels[(state.py.misc.level - 1) as usize]);
        let con = i32::from(state.py.stats.used[PlayerAttr::A_CON as usize]);
        let mut hp = base_hp + constitution_adj_from_value(con) * level;

        if hp < level + 1 {
            hp = level + 1;
        }

        if (state.py.flags.status & crate::config::player::status::PY_HERO) != 0 {
            hp += 10;
        }

        if (state.py.flags.status & crate::config::player::status::PY_SHERO) != 0 {
            hp += 20;
        }

        if hp != i32::from(state.py.misc.max_hp) && state.py.misc.max_hp != 0 {
            let value = ((i32::from(state.py.misc.current_hp) << 16)
                + i32::from(state.py.misc.current_hp_fraction))
                / i32::from(state.py.misc.max_hp)
                * hp;
            state.py.misc.current_hp = (value >> 16) as i16;
            state.py.misc.current_hp_fraction = (value & 0xFFFF) as u16;
            state.py.misc.max_hp = hp as i16;
            state.py.flags.status |= PY_HP;
        }
    });
}

fn player_attack_blows_dexterity(dexterity: i32) -> i32 {
    if dexterity < 10 {
        0
    } else if dexterity < 19 {
        1
    } else if dexterity < 68 {
        2
    } else if dexterity < 108 {
        3
    } else if dexterity < 118 {
        4
    } else {
        5
    }
}

fn player_attack_blows_strength(strength: i32, weight: i32) -> i32 {
    let adj_weight = strength * 10 / weight;

    if adj_weight < 2 {
        0
    } else if adj_weight < 3 {
        1
    } else if adj_weight < 4 {
        2
    } else if adj_weight < 5 {
        3
    } else if adj_weight < 7 {
        4
    } else if adj_weight < 9 {
        5
    } else {
        6
    }
}

/// 113
pub fn player_attack_blows(weight: i32, weight_to_hit: &mut i32) -> i32 {
    *weight_to_hit = 0;

    let (player_strength, dexterity) = with_state(|state| {
        (
            i32::from(state.py.stats.used[PlayerAttr::A_STR as usize]),
            i32::from(state.py.stats.used[PlayerAttr::A_DEX as usize]),
        )
    });

    if player_strength * 15 < weight {
        *weight_to_hit = player_strength * 15 - weight;
        return 1;
    }

    let dex = player_attack_blows_dexterity(dexterity);
    let strength = player_attack_blows_strength(player_strength, weight);

    i32::from(BLOWS_TABLE[strength as usize][dex as usize])
}

/// 140
pub fn player_stat_adjustment_wisdom_intelligence(stat: PlayerAttr) -> i32 {
    let value = i32::from(with_state(|state| state.py.stats.used[stat as usize]));

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
}

/// 203
pub fn player_stat_adjustment_charisma() -> i32 {
    let charisma = i32::from(with_state(|state| {
        state.py.stats.used[PlayerAttr::A_CHR as usize]
    }));

    if charisma > 117 {
        90
    } else if charisma > 107 {
        92
    } else if charisma > 87 {
        94
    } else if charisma > 67 {
        96
    } else if charisma > 18 {
        98
    } else {
        match charisma {
            18 => 100,
            17 => 101,
            16 => 102,
            15 => 103,
            14 => 104,
            13 => 106,
            12 => 108,
            11 => 110,
            10 => 112,
            9 => 114,
            8 => 116,
            7 => 118,
            6 => 120,
            5 => 122,
            4 => 125,
            3 => 130,
            _ => 100,
        }
    }
}

/// 230
pub fn player_stat_adjustment_constitution() -> i32 {
    let con = i32::from(with_state(|state| {
        state.py.stats.used[PlayerAttr::A_CON as usize]
    }));
    constitution_adj_from_value(con)
}

/// 258
pub fn player_modify_stat(stat: PlayerAttr, amount: i16) -> u8 {
    let current = with_state(|state| state.py.stats.current[stat as usize]);
    modify_stat_value(current, amount)
}

/// 278
pub fn player_set_and_use_stat(stat: PlayerAttr) {
    let class_id = with_state(|state| state.py.misc.class_id);
    with_state_mut(|state| {
        let modified = state.py.stats.modified[stat as usize];
        let current = state.py.stats.current[stat as usize];
        state.py.stats.used[stat as usize] = modify_stat_value(current, modified);
    });

    if stat == PlayerAttr::A_STR {
        with_state_mut(|state| {
            state.py.flags.status |= PY_STR_WGT;
        });
        player_recalculate_bonuses();
    } else if stat == PlayerAttr::A_DEX {
        player_recalculate_bonuses();
    } else if stat == PlayerAttr::A_INT
        && CLASSES[class_id as usize].class_to_use_mage_spells == SPELL_TYPE_MAGE
    {
        player_calculate_allowed_spells_count(PlayerAttr::A_INT);
        player_gain_mana(PlayerAttr::A_INT);
    } else if stat == PlayerAttr::A_WIS
        && CLASSES[class_id as usize].class_to_use_mage_spells == SPELL_TYPE_PRIEST
    {
        player_calculate_allowed_spells_count(PlayerAttr::A_WIS);
        player_gain_mana(PlayerAttr::A_WIS);
    } else if stat == PlayerAttr::A_CON {
        player_calculate_hit_points();
    }
}

/// 307
pub fn player_stat_random_increase(stat: PlayerAttr) -> bool {
    let current = with_state(|state| state.py.stats.current[stat as usize]);
    let mut new_stat = i32::from(current);

    if new_stat >= 118 {
        return false;
    }

    if (18..116).contains(&new_stat) {
        let gain = ((118 - new_stat) / 3 + 1) >> 1;
        new_stat += random_number(gain) + gain;
    } else {
        new_stat += 1;
    }

    with_state_mut(|state| {
        state.py.stats.current[stat as usize] = new_stat as u8;
        if new_stat > i32::from(state.py.stats.max[stat as usize]) {
            state.py.stats.max[stat as usize] = new_stat as u8;
        }
    });

    player_set_and_use_stat(stat);
    display_character_stats(stat as i32);

    true
}

/// 334
pub fn player_stat_random_decrease(stat: PlayerAttr) -> bool {
    let current = with_state(|state| state.py.stats.current[stat as usize]);
    let mut new_stat = i32::from(current);

    if new_stat <= 3 {
        return false;
    }

    if (19..117).contains(&new_stat) {
        let loss = (((118 - new_stat) >> 1) + 1) >> 1;
        new_stat += -random_number(loss) - loss;
        if new_stat < 18 {
            new_stat = 18;
        }
    } else {
        new_stat -= 1;
    }

    with_state_mut(|state| {
        state.py.stats.current[stat as usize] = new_stat as u8;
    });

    player_set_and_use_stat(stat);
    display_character_stats(stat as i32);

    true
}

/// 350
pub fn player_stat_restore(stat: PlayerAttr) -> bool {
    let (current, max) = with_state(|state| {
        (
            state.py.stats.current[stat as usize],
            state.py.stats.max[stat as usize],
        )
    });
    let delta = i32::from(max) - i32::from(current);

    if delta == 0 {
        return false;
    }

    with_state_mut(|state| {
        state.py.stats.current[stat as usize] = (i32::from(current) + delta) as u8;
    });

    player_set_and_use_stat(stat);
    display_character_stats(stat as i32);

    true
}

/// 363
pub fn player_stat_boost(stat: PlayerAttr, amount: i32) {
    with_state_mut(|state| {
        state.py.stats.modified[stat as usize] += amount as i16;
    });

    player_set_and_use_stat(stat);

    with_state_mut(|state| {
        state.py.flags.status |= PY_STR << (stat as u32);
    });
}

/// 410
pub fn player_to_hit_adjustment() -> i32 {
    let (dexterity, strength) = with_state(|state| {
        (
            i32::from(state.py.stats.used[PlayerAttr::A_DEX as usize]),
            i32::from(state.py.stats.used[PlayerAttr::A_STR as usize]),
        )
    });

    let mut total: i16 = if dexterity < 4 {
        -3
    } else if dexterity < 6 {
        -2
    } else if dexterity < 8 {
        -1
    } else if dexterity < 16 {
        0
    } else if dexterity < 17 {
        1
    } else if dexterity < 18 {
        2
    } else if dexterity < 69 {
        3
    } else if dexterity < 118 {
        4
    } else {
        5
    };

    if strength < 4 {
        total -= 3;
    } else if strength < 5 {
        total -= 2;
    } else if strength < 7 {
        total -= 1;
    } else if strength < 18 {
        // no change
    } else if strength < 94 {
        total += 1;
    } else if strength < 109 {
        total += 2;
    } else if strength < 117 {
        total += 3;
    } else {
        total += 4;
    }

    i32::from(total)
}

/// 441
pub fn player_armor_class_adjustment() -> i32 {
    let stat = i32::from(with_state(|state| {
        state.py.stats.used[PlayerAttr::A_DEX as usize]
    }));

    let adjustment: i16 = if stat < 4 {
        -4
    } else if stat == 4 {
        -3
    } else if stat == 5 {
        -2
    } else if stat == 6 {
        -1
    } else if stat < 15 {
        0
    } else if stat < 18 {
        1
    } else if stat < 59 {
        2
    } else if stat < 94 {
        3
    } else if stat < 117 {
        4
    } else {
        5
    };

    i32::from(adjustment)
}

/// 476
pub fn player_disarm_adjustment() -> i32 {
    let stat = i32::from(with_state(|state| {
        state.py.stats.used[PlayerAttr::A_DEX as usize]
    }));

    let adjustment: i16 = if stat < 4 {
        -8
    } else if stat == 4 {
        -6
    } else if stat == 5 {
        -4
    } else if stat == 6 {
        -2
    } else if stat == 7 {
        -1
    } else if stat < 13 {
        0
    } else if stat < 16 {
        1
    } else if stat < 18 {
        2
    } else if stat < 59 {
        4
    } else if stat < 94 {
        5
    } else if stat < 117 {
        6
    } else {
        8
    };

    i32::from(adjustment)
}

/// 505
pub fn player_damage_adjustment() -> i32 {
    let stat = i32::from(with_state(|state| {
        state.py.stats.used[PlayerAttr::A_STR as usize]
    }));

    let adjustment: i16 = if stat < 4 {
        -2
    } else if stat < 5 {
        -1
    } else if stat < 16 {
        0
    } else if stat < 17 {
        1
    } else if stat < 18 {
        2
    } else if stat < 94 {
        3
    } else if stat < 109 {
        4
    } else if stat < 117 {
        5
    } else {
        6
    };

    i32::from(adjustment)
}
