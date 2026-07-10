//! Port of src/character.h — race/class/background data types.
//! Port of src/character.cpp — character creation flow.

/// Port of `Race_t` in character.h.
#[derive(Clone, Copy, Debug, Default)]
pub struct Race {
    pub name: &'static str,
    pub str_adjustment: i16,
    pub int_adjustment: i16,
    pub wis_adjustment: i16,
    pub dex_adjustment: i16,
    pub con_adjustment: i16,
    pub chr_adjustment: i16,
    pub base_age: u8,
    pub max_age: u8,
    pub male_height_base: u8,
    pub male_height_mod: u8,
    pub male_weight_base: u8,
    pub male_weight_mod: u8,
    pub female_height_base: u8,
    pub female_height_mod: u8,
    pub female_weight_base: u8,
    pub female_weight_mod: u8,
    pub disarm_chance_base: i16,
    pub search_chance_base: i16,
    pub stealth: i16,
    pub fos: i16,
    pub base_to_hit: i16,
    pub base_to_hit_bows: i16,
    pub saving_throw_base: i16,
    pub hit_points_base: u8,
    pub infra_vision: u8,
    pub exp_factor_base: u8,
    pub classes_bit_field: u8,
}

/// Port of `Class_t` in character.h.
#[derive(Clone, Copy, Debug, Default)]
pub struct Class {
    pub title: &'static str,
    pub hit_points: u8,
    pub disarm_traps: u8,
    pub searching: u8,
    pub stealth: u8,
    pub fos: u8,
    pub base_to_hit: u8,
    pub base_to_hit_with_bows: u8,
    pub saving_throw: u8,
    pub strength: i16,
    pub intelligence: i16,
    pub wisdom: i16,
    pub dexterity: i16,
    pub constitution: i16,
    pub charisma: i16,
    pub class_to_use_mage_spells: u8,
    pub experience_factor: u8,
    pub min_level_for_spell_casting: u8,
}

/// Port of `Background_t` in character.h.
#[derive(Clone, Copy, Debug, Default)]
pub struct Background {
    pub info: &'static str,
    pub roll: u8,
    pub chart: u8,
    pub next: u8,
    pub bonus: u8,
}

use crate::config::files;
use crate::data_player::{CHARACTER_BACKGROUNDS, CHARACTER_RACES, CLASSES};
use crate::game::{random_number, random_number_normal_distribution, with_state, with_state_mut};
use crate::game_files::display_text_help_file;
use crate::player::{
    player_armor_class_adjustment, player_damage_adjustment, player_disarm_adjustment,
    player_is_male, player_set_and_use_stat, player_set_gender,
    player_stat_adjustment_constitution, player_to_hit_adjustment, PlayerAttr, PLAYER_MAX_CLASSES,
    PLAYER_MAX_LEVEL, PLAYER_MAX_RACES,
};
use crate::ui::{
    get_character_name, print_character_abilities, print_character_information,
    print_character_level_experience, print_character_stats, print_character_vital_statistics,
};
use crate::ui_io::terminal::{self, Coord};

use crate::ui_io::ESCAPE;

const ALL_STATS: [PlayerAttr; 6] = [
    PlayerAttr::A_STR,
    PlayerAttr::A_INT,
    PlayerAttr::A_WIS,
    PlayerAttr::A_DEX,
    PlayerAttr::A_CON,
    PlayerAttr::A_CHR,
];

/// C++ character.cpp lines 29–45.
#[must_use]
pub fn decrement_stat(adjustment: i16, current_stat: u8) -> u8 {
    let mut stat = current_stat;
    let mut i = 0i16;
    while i > adjustment {
        if stat > 108 {
            stat -= 1;
        } else if stat > 88 {
            stat = stat.wrapping_sub((random_number(6) + 2) as u8);
        } else if stat > 18 {
            stat = stat.wrapping_sub((random_number(15) + 5) as u8);
            if stat < 18 {
                stat = 18;
            }
        } else if stat > 3 {
            stat -= 1;
        }
        i -= 1;
    }
    stat
}

/// C++ character.cpp lines 48–61.
#[must_use]
pub fn increment_stat(adjustment: i16, current_stat: u8) -> u8 {
    let mut stat = current_stat;
    for _ in 0..adjustment {
        if stat < 18 {
            stat += 1;
        } else if stat < 88 {
            stat = stat.wrapping_add((random_number(15) + 5) as u8);
        } else if stat < 108 {
            stat = stat.wrapping_add((random_number(6) + 2) as u8);
        } else if stat < 118 {
            stat += 1;
        }
    }
    stat
}

/// C++ character.cpp lines 67–71.
#[must_use]
pub fn create_modify_player_stat(stat: u8, adjustment: i16) -> u8 {
    if adjustment < 0 {
        decrement_stat(adjustment, stat)
    } else {
        increment_stat(adjustment, stat)
    }
}

/// C++ character.cpp lines 11–26.
fn character_generate_stats() {
    let mut dice = [0i32; 18];
    loop {
        let mut total = 0;
        for (i, die) in dice.iter_mut().enumerate() {
            *die = random_number(3 + (i % 3) as i32);
            total += *die;
        }
        if total > 42 && total < 54 {
            break;
        }
    }

    with_state_mut(|state| {
        for i in 0..6 {
            state.py.stats.max[i] = (5 + dice[3 * i] + dice[3 * i + 1] + dice[3 * i + 2]) as u8;
        }
    });
}

/// C++ character.cpp lines 76–107.
pub fn character_generate_stats_and_race() {
    let race_id = with_state(|state| state.py.misc.race_id);
    let race = &CHARACTER_RACES[race_id as usize];

    character_generate_stats();

    let stats_max = with_state(|state| state.py.stats.max);
    let stats_max = [
        create_modify_player_stat(stats_max[PlayerAttr::A_STR as usize], race.str_adjustment),
        create_modify_player_stat(stats_max[PlayerAttr::A_INT as usize], race.int_adjustment),
        create_modify_player_stat(stats_max[PlayerAttr::A_WIS as usize], race.wis_adjustment),
        create_modify_player_stat(stats_max[PlayerAttr::A_DEX as usize], race.dex_adjustment),
        create_modify_player_stat(stats_max[PlayerAttr::A_CON as usize], race.con_adjustment),
        create_modify_player_stat(stats_max[PlayerAttr::A_CHR as usize], race.chr_adjustment),
    ];

    with_state_mut(|state| {
        state.py.stats.max = stats_max;
        state.py.misc.level = 1;

        for i in 0..6 {
            state.py.stats.current[i] = state.py.stats.max[i];
        }

        state.py.misc.chance_in_search = race.search_chance_base;
        state.py.misc.bth = race.base_to_hit;
        state.py.misc.bth_with_bows = race.base_to_hit_bows;
        state.py.misc.fos = race.fos;
        state.py.misc.stealth_factor = race.stealth;
        state.py.misc.saving_throw = race.saving_throw_base;
        state.py.misc.hit_die = race.hit_points_base;
        state.py.misc.magical_ac = 0;
        state.py.misc.experience_factor = race.exp_factor_base;
        state.py.flags.see_infra = race.infra_vision as i16;
    });

    for stat in ALL_STATS {
        player_set_and_use_stat(stat);
    }

    let plusses_to_damage = player_damage_adjustment() as i16;
    let plusses_to_hit = player_to_hit_adjustment() as i16;
    let ac = player_armor_class_adjustment() as i16;

    with_state_mut(|state| {
        state.py.misc.plusses_to_damage = plusses_to_damage;
        state.py.misc.plusses_to_hit = plusses_to_hit;
        state.py.misc.ac = ac;
    });
}

/// C++ character.cpp lines 111–128.
pub fn display_character_races() {
    terminal::clear_to_bottom(20);
    terminal::put_string("Choose a race (? for Help):", Coord { y: 20, x: 2 });

    let mut coord = Coord { y: 21, x: 2 };
    for i in 0..PLAYER_MAX_RACES as usize {
        let description = format!("{}) {}", (i as u8 + b'a') as char, CHARACTER_RACES[i].name);
        terminal::put_string(&description, coord);
        coord.x += 15;
        if coord.x > 70 {
            coord.x = 2;
            coord.y += 1;
        }
    }
}

/// C++ character.cpp lines 132–151.
pub fn character_choose_race() {
    display_character_races();

    loop {
        terminal::move_cursor(Coord { y: 20, x: 30 });
        let key = terminal::get_key_input();

        let id = i32::from(key) - 97;
        if (0..i32::from(PLAYER_MAX_RACES)).contains(&id) {
            with_state_mut(|state| state.py.misc.race_id = id as u8);
            terminal::put_string(CHARACTER_RACES[id as usize].name, Coord { y: 3, x: 15 });
            break;
        } else if key == b'?' {
            display_text_help_file(files::WELCOME_SCREEN);
        } else {
            terminal::terminal_bell_sound();
        }
    }
}

/// C++ character.cpp lines 155–160.
pub fn display_character_history() {
    terminal::put_string("Character Background", Coord { y: 14, x: 27 });
    let history = with_state(|state| state.py.misc.history);
    for (i, entry) in history.iter().enumerate() {
        let line = c_str_from_history(entry);
        terminal::put_string_clear_to_eol(
            &line,
            Coord {
                y: 15 + i as i32,
                x: 10,
            },
        );
    }
}

/// C++ character.cpp lines 164–167.
pub fn player_clear_history() {
    with_state_mut(|state| {
        for entry in &mut state.py.misc.history {
            entry[0] = 0;
        }
    });
}

fn c_str_from_history(buf: &[u8; 60]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

/// C++ character.cpp lines 175–263.
pub fn character_get_history() {
    let race_id = with_state(|state| state.py.misc.race_id);
    let mut history_id = i32::from(race_id) * 3 + 1;
    let mut social_class = random_number(4);
    let mut history_block = String::new();
    let mut background_id = 0usize;

    loop {
        let mut flag = false;
        while !flag {
            if CHARACTER_BACKGROUNDS[background_id].chart == history_id as u8 {
                let test_roll = random_number(100);
                while test_roll > i32::from(CHARACTER_BACKGROUNDS[background_id].roll) {
                    background_id += 1;
                }

                let background = &CHARACTER_BACKGROUNDS[background_id];
                history_block.push_str(background.info);
                social_class += i32::from(background.bonus) - 50;

                if history_id > i32::from(background.next) {
                    background_id = 0;
                }
                history_id = i32::from(background.next);
                flag = true;
            } else {
                background_id += 1;
            }
        }
        if history_id < 1 {
            break;
        }
    }

    player_clear_history();

    let bytes = history_block.as_bytes();
    let mut cursor_start = 0usize;
    let mut cursor_end = bytes.len().saturating_sub(1);
    while cursor_end < bytes.len() && bytes[cursor_end] == b' ' {
        cursor_end = cursor_end.saturating_sub(1);
    }

    let mut line_number = 0usize;
    let mut new_cursor_start = 0usize;
    let mut done = false;

    with_state_mut(|state| {
        while !done {
            while cursor_start < bytes.len() && bytes[cursor_start] == b' ' {
                cursor_start += 1;
            }

            let mut current_cursor_position = cursor_end.saturating_sub(cursor_start) + 1;
            if current_cursor_position > 60 {
                current_cursor_position = 60;
                while bytes[cursor_start + current_cursor_position - 1] != b' ' {
                    current_cursor_position -= 1;
                }
                new_cursor_start = cursor_start + current_cursor_position;
                while bytes[cursor_start + current_cursor_position - 1] == b' ' {
                    current_cursor_position -= 1;
                }
            } else {
                done = true;
            }

            let end = cursor_start + current_cursor_position;
            let len = current_cursor_position.min(60);
            state.py.misc.history[line_number][..len].copy_from_slice(&bytes[cursor_start..end]);
            // C++ character.cpp:250 — NUL-terminate when len < 60 (array is [60];
            // writing at index 60 is UB in C++ and omitted here).
            if len < 60 {
                state.py.misc.history[line_number][len] = 0;
            }
            line_number += 1;
            cursor_start = new_cursor_start;
        }
    });

    social_class = social_class.clamp(1, 100);

    with_state_mut(|state| {
        state.py.misc.social_class = social_class as i16;
    });
}

/// C++ character.cpp lines 267–289.
pub fn character_set_gender() {
    terminal::clear_to_bottom(20);
    terminal::put_string("Choose a sex (? for Help):", Coord { y: 20, x: 2 });
    terminal::put_string("m) Male       f) Female", Coord { y: 21, x: 2 });

    loop {
        terminal::move_cursor(Coord { y: 20, x: 29 });
        let key = terminal::get_key_input();

        if key == b'f' || key == b'F' {
            player_set_gender(false);
            terminal::put_string("Female", Coord { y: 4, x: 15 });
            break;
        } else if key == b'm' || key == b'M' {
            player_set_gender(true);
            terminal::put_string("Male", Coord { y: 4, x: 15 });
            break;
        } else if key == b'?' {
            display_text_help_file(files::WELCOME_SCREEN);
        } else {
            terminal::terminal_bell_sound();
        }
    }
}

/// C++ character.cpp lines 293–313.
pub fn character_set_age_height_weight() {
    let race_id = with_state(|state| state.py.misc.race_id);
    let is_male = player_is_male();
    let disarm_adj = player_disarm_adjustment();
    let race = &CHARACTER_RACES[race_id as usize];

    let (height_base, height_mod, weight_base, weight_mod) = if is_male {
        (
            race.male_height_base,
            race.male_height_mod,
            race.male_weight_base,
            race.male_weight_mod,
        )
    } else {
        (
            race.female_height_base,
            race.female_height_mod,
            race.female_weight_base,
            race.female_weight_mod,
        )
    };

    let age = (race.base_age as u16).wrapping_add(random_number(race.max_age as i32) as u16);
    let height = random_number_normal_distribution(height_base as i32, height_mod as i32) as u16;
    let weight = random_number_normal_distribution(weight_base as i32, weight_mod as i32) as u16;

    with_state_mut(|state| {
        state.py.misc.age = age;
        state.py.misc.height = height;
        state.py.misc.weight = weight;
        state.py.misc.disarm = race.disarm_chance_base + disarm_adj as i16;
    });
}

/// C++ character.cpp lines 318–345.
pub fn display_race_classes(race_id: u8, class_list: &mut [u8; PLAYER_MAX_CLASSES as usize]) -> u8 {
    let mut coord = Coord { y: 21, x: 2 };
    let mut class_id = 0u8;
    let mut mask = 0x1u32;

    terminal::clear_to_bottom(20);
    terminal::put_string("Choose a class (? for Help):", Coord { y: 20, x: 2 });

    for i in 0..PLAYER_MAX_CLASSES as usize {
        if (CHARACTER_RACES[race_id as usize].classes_bit_field as u32 & mask) != 0 {
            let description = format!("{}) {}", (class_id + b'a') as char, CLASSES[i].title);
            terminal::put_string(&description, coord);
            class_list[class_id as usize] = i as u8;

            coord.x += 15;
            if coord.x > 70 {
                coord.x = 2;
                coord.y += 1;
            }
            class_id += 1;
        }
        mask <<= 1;
    }

    class_id
}

/// C++ character.cpp lines 348–408.
pub fn generate_character_class(class_id: u8) {
    with_state_mut(|state| {
        state.py.misc.class_id = class_id;
    });

    let klass = &CLASSES[class_id as usize];
    terminal::clear_to_bottom(20);
    terminal::put_string(klass.title, Coord { y: 5, x: 15 });

    let stats_max = with_state(|state| state.py.stats.max);
    let stats_max = [
        create_modify_player_stat(stats_max[PlayerAttr::A_STR as usize], klass.strength),
        create_modify_player_stat(stats_max[PlayerAttr::A_INT as usize], klass.intelligence),
        create_modify_player_stat(stats_max[PlayerAttr::A_WIS as usize], klass.wisdom),
        create_modify_player_stat(stats_max[PlayerAttr::A_DEX as usize], klass.dexterity),
        create_modify_player_stat(stats_max[PlayerAttr::A_CON as usize], klass.constitution),
        create_modify_player_stat(stats_max[PlayerAttr::A_CHR as usize], klass.charisma),
    ];

    with_state_mut(|state| {
        state.py.stats.max = stats_max;
        for i in 0..6 {
            state.py.stats.current[i] = state.py.stats.max[i];
        }
    });

    for stat in ALL_STATS {
        player_set_and_use_stat(stat);
    }

    let plusses_to_damage = player_damage_adjustment() as i16;
    let plusses_to_hit = player_to_hit_adjustment() as i16;
    let magical_ac = player_armor_class_adjustment() as i16;
    let hit_die = with_state(|state| state.py.misc.hit_die.wrapping_add(klass.hit_points));
    let max_hp = (player_stat_adjustment_constitution() + i32::from(hit_die)) as i16;

    let min_value = (i32::from(PLAYER_MAX_LEVEL) * 3 / 8 * i32::from(hit_die - 1))
        + i32::from(PLAYER_MAX_LEVEL);
    let max_value = (i32::from(PLAYER_MAX_LEVEL) * 5 / 8 * i32::from(hit_die - 1))
        + i32::from(PLAYER_MAX_LEVEL);

    let mut base_hp_levels = [0u16; PLAYER_MAX_LEVEL as usize];
    base_hp_levels[0] = hit_die as u16;
    loop {
        for i in 1..PLAYER_MAX_LEVEL as usize {
            base_hp_levels[i] = random_number(hit_die as i32) as u16;
            base_hp_levels[i] = base_hp_levels[i].wrapping_add(base_hp_levels[i - 1]);
        }
        if i32::from(base_hp_levels[PLAYER_MAX_LEVEL as usize - 1]) >= min_value
            && i32::from(base_hp_levels[PLAYER_MAX_LEVEL as usize - 1]) <= max_value
        {
            break;
        }
    }

    with_state_mut(|state| {
        state.py.misc.plusses_to_damage = plusses_to_damage;
        state.py.misc.plusses_to_hit = plusses_to_hit;
        state.py.misc.magical_ac = magical_ac;
        state.py.misc.ac = 0;

        state.py.misc.display_to_damage = plusses_to_damage;
        state.py.misc.display_to_hit = plusses_to_hit;
        state.py.misc.display_to_ac = magical_ac;
        state.py.misc.display_ac = magical_ac;

        state.py.misc.hit_die = hit_die;
        state.py.misc.max_hp = max_hp;
        state.py.misc.current_hp = max_hp;
        state.py.misc.current_hp_fraction = 0;
        state.py.base_hp_levels = base_hp_levels;

        state.py.misc.bth += klass.base_to_hit as i16;
        state.py.misc.bth_with_bows += klass.base_to_hit_with_bows as i16;
        state.py.misc.chance_in_search += klass.searching as i16;
        state.py.misc.disarm += klass.disarm_traps as i16;
        state.py.misc.fos += klass.fos as i16;
        state.py.misc.stealth_factor += klass.stealth as i16;
        state.py.misc.saving_throw += klass.saving_throw as i16;
        state.py.misc.experience_factor = state
            .py
            .misc
            .experience_factor
            .wrapping_add(klass.experience_factor);
    });
}

/// C++ character.cpp lines 412–435.
pub fn character_get_class() {
    let mut class_list = [0u8; PLAYER_MAX_CLASSES as usize];
    let race_id = with_state(|state| state.py.misc.race_id);
    let class_count = display_race_classes(race_id, &mut class_list);

    with_state_mut(|state| state.py.misc.class_id = 0);

    loop {
        terminal::move_cursor(Coord { y: 20, x: 31 });
        let key = terminal::get_key_input();

        let id = i32::from(key) - 97;
        if (0..i32::from(class_count)).contains(&id) {
            generate_character_class(class_list[id as usize]);
            break;
        } else if key == b'?' {
            display_text_help_file(files::WELCOME_SCREEN);
        } else {
            terminal::terminal_bell_sound();
        }
    }
}

/// C++ character.cpp lines 440–441.
#[must_use]
pub fn monetary_value_calculated_from_stat(stat: u8) -> i32 {
    5 * (i32::from(stat) - 10)
}

/// C++ character.cpp lines 444–470.
pub fn player_calculate_start_gold() {
    let is_male = player_is_male();
    let (stats, social_class) =
        with_state(|state| (state.py.stats.max, state.py.misc.social_class));
    let gold_roll = random_number(25);

    let mut value = monetary_value_calculated_from_stat(stats[PlayerAttr::A_STR as usize]);
    value += monetary_value_calculated_from_stat(stats[PlayerAttr::A_INT as usize]);
    value += monetary_value_calculated_from_stat(stats[PlayerAttr::A_WIS as usize]);
    value += monetary_value_calculated_from_stat(stats[PlayerAttr::A_CON as usize]);
    value += monetary_value_calculated_from_stat(stats[PlayerAttr::A_DEX as usize]);

    let mut new_gold = i32::from(social_class) * 6 + gold_roll + 325;
    new_gold -= value;
    new_gold += monetary_value_calculated_from_stat(stats[PlayerAttr::A_CHR as usize]);

    if !is_male {
        new_gold += 50;
    }

    if new_gold < 80 {
        new_gold = 80;
    }

    with_state_mut(|state| {
        state.py.misc.au = new_gold;
    });
}

/// C++ character.cpp lines 474–516.
pub fn character_create() {
    print_character_information();
    character_choose_race();
    character_set_gender();

    let mut done = false;
    while !done {
        character_generate_stats_and_race();
        character_get_history();
        character_set_age_height_weight();
        display_character_history();
        print_character_vital_statistics();
        print_character_stats();

        terminal::clear_to_bottom(20);
        terminal::put_string(
            "Hit space to re-roll or ESC to accept characteristics: ",
            Coord { y: 20, x: 2 },
        );

        loop {
            let key = terminal::get_key_input();
            if key == ESCAPE {
                done = true;
                break;
            } else if key == b' ' {
                break;
            }
            terminal::terminal_bell_sound();
        }
    }

    character_get_class();
    player_calculate_start_gold();
    print_character_stats();
    print_character_level_experience();
    print_character_abilities();
    get_character_name();

    terminal::put_string_clear_to_eol(
        "[ press any key to continue, or Q to exit ]",
        Coord { y: 23, x: 17 },
    );
    if terminal::get_key_input() == b'Q' {
        crate::game::exit_program();
    }
    terminal::erase_line(Coord { y: 23, x: 0 });
}
