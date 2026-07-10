//! Port of src/game_files.cpp — see phase_5.3.

use std::cell::{Cell, RefCell};
use std::fs::File;
use std::io::{self, Read, Write};

use crate::config::files;
use crate::config::identification::{ID_DAMD, ID_STORE_BOUGHT};
use crate::data_player::{CHARACTER_RACES, CLASSES, CLASS_LEVEL_ADJ};
use crate::game::{with_state, with_state_mut, State};
use crate::game_objects::{item_get_random_object_id, popt, pusht};
use crate::helpers::string_to_number;
use crate::identification::{
    item_append_to_inscription, item_description_for_state,
    spell_item_identify_and_remove_random_inscription_for_state,
};
use crate::inventory::{
    inventory_item_copy_to, inventory_item_is_cursed, PlayerEquipment, PLAYER_INVENTORY_SIZE,
};
use crate::player::{PlayerAttr, PlayerClassLevelAdj, BTH_PER_PLUS_TO_HIT_ADJUST, PLAYER_MAX_LEVEL};
use crate::scores;
use crate::treasure::TV_NOTHING;
use crate::types::{MORIA_MESSAGE_SIZE, MORIA_OBJ_DESC_SIZE_LEN};
use crate::ui::{stat_rating, stats_as_string};
use crate::ui_io::{self, terminal, ESCAPE};
use crate::ui_io::terminal::Coord;

fn stat_adjustment_wisdom_intelligence(state: &State, stat: PlayerAttr) -> i32 {
    let value = i32::from(state.py.stats.used[stat as usize]);
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
    } else if value > 7 {
        1
    } else {
        0
    }
}

fn disarm_adjustment_for_state(state: &State) -> i32 {
    let stat = i32::from(state.py.stats.used[PlayerAttr::A_DEX as usize]);
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

fn rank_title_for_state(state: &State) -> &'static str {
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
}

fn gender_label_for_state(state: &State) -> &'static str {
    if state.py.misc.gender {
        "Male"
    } else {
        "Female"
    }
}

thread_local! {
    static TEST_OUTPUT_RESULTS: RefCell<Vec<bool>> = const { RefCell::new(Vec::new()) };
    static TEST_OUTPUT_CALL_COUNT: Cell<u32> = const { Cell::new(0) };
    static TEST_OUTPUT_LAST_PATH: RefCell<String> = const { RefCell::new(String::new()) };
}

#[doc(hidden)]
pub fn test_reset_output_player_character_hooks() {
    TEST_OUTPUT_RESULTS.with(|r| *r.borrow_mut() = Vec::new());
    TEST_OUTPUT_CALL_COUNT.with(|c| c.set(0));
    TEST_OUTPUT_LAST_PATH.with(|p| p.borrow_mut().clear());
}

#[doc(hidden)]
pub fn test_set_output_player_character_results(results: &[bool]) {
    TEST_OUTPUT_RESULTS.with(|r| *r.borrow_mut() = results.to_vec());
}

#[doc(hidden)]
pub fn test_output_player_character_call_count() -> u32 {
    TEST_OUTPUT_CALL_COUNT.with(|c| c.get())
}

#[doc(hidden)]
pub fn test_output_player_character_last_path() -> String {
    TEST_OUTPUT_LAST_PATH.with(|p| p.borrow().clone())
}

fn take_test_output_result() -> Option<bool> {
    TEST_OUTPUT_RESULTS.with(|results| {
        let mut queue = results.borrow_mut();
        if queue.is_empty() {
            None
        } else if queue.len() > 1 {
            Some(queue.remove(0))
        } else {
            Some(queue[0])
        }
    })
}

/// C `fgets(buf, n, stream)` — reads at most `n-1` bytes, keeps `\n`, NUL-terminates.
#[doc(hidden)]
pub fn fgets(buf: &mut [u8], n: i32, reader: &mut impl Read) -> bool {
    if n <= 0 {
        return false;
    }
    if n == 1 {
        buf[0] = 0;
        return false;
    }

    let max_read = (n - 1) as usize;
    let mut index = 0usize;
    loop {
        if index >= max_read {
            break;
        }
        let mut byte = [0u8; 1];
        match reader.read(&mut byte) {
            Ok(0) => {
                if index == 0 {
                    buf[0] = 0;
                    return false;
                }
                break;
            }
            Ok(_) => {
                buf[index] = byte[0];
                index += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => {
                if index == 0 {
                    buf[0] = 0;
                    return false;
                }
                break;
            }
        }
    }
    if index < buf.len() {
        buf[index] = 0;
    }
    true
}

fn c_str_bytes(bytes: &[u8]) -> &str {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..end]).unwrap_or("")
}

fn fgets_line_string(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

fn copy_cstr(dest: &mut [u8], src: &str) {
    let bytes = src.as_bytes();
    let n = bytes.len().min(dest.len().saturating_sub(1));
    dest[..n].copy_from_slice(&bytes[..n]);
    dest[n] = 0;
}

/// Port of `initializeScoreFile` in game_files.cpp.
pub fn initialize_score_file() -> bool {
    scores::initialize_score_file()
}

/// Port of `displaySplashScreen` in game_files.cpp.
pub fn display_splash_screen() {
    let Ok(mut screen_file) = File::open(files::splash_screen) else {
        return;
    };

    terminal::clear_screen();
    let mut in_line = [0u8; MORIA_MESSAGE_SIZE];
    let mut row = 0;
    while fgets(&mut in_line, 80, &mut screen_file) {
        terminal::put_string(&fgets_line_string(&in_line), Coord { y: row, x: 0 });
        row += 1;
    }
    terminal::wait_for_continue_key(23);
}

/// Port of `displayTextHelpFile` in game_files.cpp.
pub fn display_text_help_file(filename: &str) {
    let Ok(mut file) = File::open(filename) else {
        terminal::put_string_clear_to_eol(
            &format!("Can not find help file '{filename}'."),
            Coord { y: 0, x: 0 },
        );
        return;
    };

    terminal::terminal_save_screen();

    let mut line_buffer = [0u8; MORIA_MESSAGE_SIZE];
    let mut eof = false;
    while !eof {
        terminal::clear_screen();

        for row in 0..23 {
            if fgets(&mut line_buffer, 79, &mut file) {
                terminal::put_string(&fgets_line_string(&line_buffer), Coord { y: row, x: 0 });
            } else {
                eof = true;
            }
        }

        terminal::put_string_clear_to_eol(
            "[ press any key to continue ]",
            Coord { y: 23, x: 23 },
        );
        if terminal::get_key_input() == ESCAPE {
            break;
        }
    }

    drop(file);
    terminal::terminal_restore_screen();
}

/// Port of `displayDeathFile` in game_files.cpp.
pub fn display_death_file(filename: &str) {
    let Ok(mut file) = File::open(filename) else {
        terminal::put_string_clear_to_eol(
            &format!("Can not find help file '{filename}'."),
            Coord { y: 0, x: 0 },
        );
        return;
    };

    terminal::clear_screen();

    let mut line_buffer = [0u8; MORIA_MESSAGE_SIZE];
    let mut eof = false;
    for row in 0..23 {
        if eof {
            break;
        }
        if fgets(&mut line_buffer, 79, &mut file) {
            terminal::put_string(&fgets_line_string(&line_buffer), Coord { y: row, x: 0 });
        } else {
            eof = true;
        }
    }
    drop(file);
}

/// Port of `equipmentPlacementDescription` in game_files.cpp.
#[must_use]
pub fn equipment_placement_description(item_id: i32) -> &'static str {
    match item_id {
        x if x == PlayerEquipment::Wield as i32 => "You are wielding",
        x if x == PlayerEquipment::Head as i32 => "Worn on head",
        x if x == PlayerEquipment::Neck as i32 => "Worn around neck",
        x if x == PlayerEquipment::Body as i32 => "Worn on body",
        x if x == PlayerEquipment::Arm as i32 => "Worn on shield arm",
        x if x == PlayerEquipment::Hands as i32 => "Worn on hands",
        x if x == PlayerEquipment::Right as i32 => "Right ring finger",
        x if x == PlayerEquipment::Left as i32 => "Left  ring finger",
        x if x == PlayerEquipment::Feet as i32 => "Worn on feet",
        x if x == PlayerEquipment::Outer as i32 => "Worn about body",
        x if x == PlayerEquipment::Light as i32 => "Light source is",
        x if x == PlayerEquipment::Auxiliary as i32 => "Secondary weapon",
        _ => "*Unknown value*",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedAbilities {
    pub fighting: crate::types::Coord_t,
    pub bows: crate::types::Coord_t,
    pub perception: crate::types::Coord_t,
    pub stealth: crate::types::Coord_t,
    pub disarming: crate::types::Coord_t,
    pub searching: crate::types::Coord_t,
    pub saving_throw: crate::types::Coord_t,
    pub magic_device: crate::types::Coord_t,
    pub infra: String,
}

fn compute_derived_abilities(state: &State) -> DerivedAbilities {
    let class = state.py.misc.class_id as usize;
    let level = i32::from(state.py.misc.level);
    let misc = &state.py.misc;

    let xbth = i32::from(misc.bth)
        + i32::from(misc.plusses_to_hit) * i32::from(BTH_PER_PLUS_TO_HIT_ADJUST)
        + i32::from(CLASS_LEVEL_ADJ[class][PlayerClassLevelAdj::BTH as usize]) * level;
    let xbthb = i32::from(misc.bth_with_bows)
        + i32::from(misc.plusses_to_hit) * i32::from(BTH_PER_PLUS_TO_HIT_ADJUST)
        + i32::from(CLASS_LEVEL_ADJ[class][PlayerClassLevelAdj::BTHB as usize]) * level;

    let mut xfos = 40 - i32::from(misc.fos);
    if xfos < 0 {
        xfos = 0;
    }
    let xsrh = i32::from(misc.chance_in_search);
    let xstl = i32::from(misc.stealth_factor) + 1;
    let xdis = i32::from(misc.disarm)
        + 2 * disarm_adjustment_for_state(state)
        + stat_adjustment_wisdom_intelligence(state, PlayerAttr::A_INT)
        + i32::from(CLASS_LEVEL_ADJ[class][PlayerClassLevelAdj::DISARM as usize]) * level / 3;
    let xsave = i32::from(misc.saving_throw)
        + stat_adjustment_wisdom_intelligence(state, PlayerAttr::A_WIS)
        + i32::from(CLASS_LEVEL_ADJ[class][PlayerClassLevelAdj::SAVE as usize]) * level / 3;
    let xdev = i32::from(misc.saving_throw)
        + stat_adjustment_wisdom_intelligence(state, PlayerAttr::A_INT)
        + i32::from(CLASS_LEVEL_ADJ[class][PlayerClassLevelAdj::DEVICE as usize]) * level / 3;

    DerivedAbilities {
        fighting: crate::types::Coord_t { y: 12, x: xbth },
        bows: crate::types::Coord_t { y: 12, x: xbthb },
        perception: crate::types::Coord_t { y: 3, x: xfos },
        stealth: crate::types::Coord_t { y: 1, x: xstl },
        disarming: crate::types::Coord_t { y: 8, x: xdis },
        searching: crate::types::Coord_t { y: 6, x: xsrh },
        saving_throw: crate::types::Coord_t { y: 6, x: xsave },
        magic_device: crate::types::Coord_t { y: 6, x: xdev },
        infra: format!("{} feet", i32::from(state.py.flags.see_infra) * 10),
    }
}

#[doc(hidden)]
pub fn compute_derived_abilities_for_test(state: &State) -> DerivedAbilities {
    compute_derived_abilities(state)
}

fn write_character_sheet_to_file(char_file: &mut impl Write) -> io::Result<()> {
    terminal::put_string_clear_to_eol("Writing character sheet...", Coord { y: 0, x: 0 });
    terminal::put_qio();

    with_state(|state| {
        let colon = ":";
        let blank = " ";

        write!(char_file, "{}\n\n", ui_io::ctrl_key(b'L') as char)?;

        write!(
            char_file,
            " Name{:>9} {:<23}",
            colon,
            c_str_bytes(&state.py.misc.name)
        )?;
        write!(char_file, " Age{:>11} {:>6}", colon, i32::from(state.py.misc.age))?;
        write!(
            char_file,
            "   STR : {}\n",
            stats_as_string(state.py.stats.used[PlayerAttr::A_STR as usize])
        )?;

        write!(
            char_file,
            " Race{:>9} {:<23}",
            colon,
            CHARACTER_RACES[state.py.misc.race_id as usize].name
        )?;
        write!(
            char_file,
            " Height{:>8} {:>6}",
            colon,
            i32::from(state.py.misc.height)
        )?;
        write!(
            char_file,
            "   INT : {}\n",
            stats_as_string(state.py.stats.used[PlayerAttr::A_INT as usize])
        )?;

        write!(
            char_file,
            " Sex{:>10} {:<23}",
            colon,
            gender_label_for_state(state)
        )?;
        write!(
            char_file,
            " Weight{:>8} {:>6}",
            colon,
            i32::from(state.py.misc.weight)
        )?;
        write!(
            char_file,
            "   WIS : {}\n",
            stats_as_string(state.py.stats.used[PlayerAttr::A_WIS as usize])
        )?;

        write!(
            char_file,
            " Class{:>8} {:<23}",
            colon,
            CLASSES[state.py.misc.class_id as usize].title
        )?;
        write!(
            char_file,
            " Social Class : {:>6}",
            state.py.misc.social_class
        )?;
        write!(
            char_file,
            "   DEX : {}\n",
            stats_as_string(state.py.stats.used[PlayerAttr::A_DEX as usize])
        )?;

        write!(
            char_file,
            " Title{:>8} {:<23}",
            colon,
            rank_title_for_state(state)
        )?;
        write!(char_file, "{blank:>22}")?;
        write!(
            char_file,
            "   CON : {}\n",
            stats_as_string(state.py.stats.used[PlayerAttr::A_CON as usize])
        )?;

        write!(char_file, "{blank:>34}")?;
        write!(char_file, "{blank:>26}")?;
        write!(
            char_file,
            "   CHR : {}\n\n",
            stats_as_string(state.py.stats.used[PlayerAttr::A_CHR as usize])
        )?;

        let misc = &state.py.misc;
        write!(
            char_file,
            " + To Hit    : {:>6}{:>7}Level      : {:>7}",
            misc.display_to_hit,
            blank,
            i32::from(misc.level)
        )?;
        write!(char_file, "    Max Hit Points : {:>6}\n", misc.max_hp)?;
        write!(
            char_file,
            " + To Damage : {:>6}{:>7}Experience : {:>7}",
            misc.display_to_damage,
            blank,
            misc.exp
        )?;
        write!(char_file, "    Cur Hit Points : {:>6}\n", misc.current_hp)?;
        write!(
            char_file,
            " + To AC     : {:>6}{:>7}Max Exp    : {:>7}",
            misc.display_to_ac,
            blank,
            misc.max_exp
        )?;
        write!(char_file, "    Max Mana{:>8} {:>6}\n", colon, misc.mana)?;
        write!(char_file, "   Total AC  : {:>6}", misc.display_ac)?;
        if misc.level >= u16::from(PLAYER_MAX_LEVEL) {
            write!(char_file, "{:>7}Exp to Adv : *******", blank)?;
        } else {
            let exp_to_adv = (state.py.base_exp_levels[usize::from(misc.level.wrapping_sub(1))]
                * u32::from(misc.experience_factor)
                / 100) as i32;
            write!(char_file, "{:>7}Exp to Adv : {:>7}", blank, exp_to_adv)?;
        }
        write!(char_file, "    Cur Mana{:>8} {:>6}\n", colon, misc.current_mana)?;
        write!(char_file, "{:>28}Gold{:>8} {:>7}\n\n", blank, colon, misc.au)?;

        let derived = compute_derived_abilities(state);
        write!(char_file, "(Miscellaneous Abilities)\n\n")?;
        write!(
            char_file,
            " Fighting    : {:<10}   Stealth     : {:<10}   Perception  : {}\n",
            stat_rating(derived.fighting),
            stat_rating(derived.stealth),
            stat_rating(derived.perception)
        )?;
        write!(
            char_file,
            " Bows/Throw  : {:<10}   Disarming   : {:<10}   Searching   : {}\n",
            stat_rating(derived.bows),
            stat_rating(derived.disarming),
            stat_rating(derived.searching)
        )?;
        write!(
            char_file,
            " Saving Throw: {:<10}   Magic Device: {:<10}   Infra-Vision: {}\n\n",
            stat_rating(derived.saving_throw),
            stat_rating(derived.magic_device),
            derived.infra
        )?;

        write!(char_file, "Character Background\n")?;
        for entry in &state.py.misc.history {
            write!(char_file, " {}\n", c_str_bytes(entry))?;
        }

        Ok(())
    })
}

fn write_equipment_list_to_file(equip_file: &mut impl Write) -> io::Result<()> {
    writeln!(equip_file, "\n  [Character's Equipment List]\n")?;

    with_state(|state| {
        if state.py.equipment_count == 0 {
            writeln!(equip_file, "  Character has no equipment in use.")?;
            return Ok(());
        }

        let mut description = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
        let mut item_slot_id = 0i32;

        for slot in PlayerEquipment::Wield as i32..PLAYER_INVENTORY_SIZE as i32 {
            let item = state.py.inventory[slot as usize];
            if item.category_id == TV_NOTHING {
                continue;
            }

            item_description_for_state(&mut description, item, true, state);
            write!(
                equip_file,
                "  {}) {:<19}: {}\n",
                (item_slot_id as u8 + b'a') as char,
                equipment_placement_description(slot),
                c_str_bytes(&description)
            )?;
            item_slot_id += 1;
        }

        write!(equip_file, "{}\n\n", ui_io::ctrl_key(b'L') as char)?;
        Ok(())
    })
}

fn write_inventory_to_file(inv_file: &mut impl Write) -> io::Result<()> {
    writeln!(inv_file, "  [General Inventory List]\n")?;

    with_state(|state| {
        if state.py.pack.unique_items == 0 {
            writeln!(inv_file, "  Character has no objects in inventory.")?;
            return Ok(());
        }

        let mut description = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
        for index in 0..state.py.pack.unique_items {
            let item = state.py.inventory[index as usize];
            item_description_for_state(&mut description, item, true, state);
            write!(
                inv_file,
                "{}) {}\n",
                (index as u8 + b'a') as char,
                c_str_bytes(&description)
            )?;
        }

        write!(inv_file, "{}", ui_io::ctrl_key(b'L') as char)?;
        Ok(())
    })
}

/// Port of `outputRandomLevelObjectsToFile` in game_files.cpp.
pub fn output_random_level_objects_to_file() {
    let mut input = [0u8; MORIA_OBJ_DESC_SIZE_LEN];

    terminal::put_string_clear_to_eol("Produce objects on what level?: ", Coord { y: 0, x: 0 });
    if !terminal::get_string_input(&mut input, Coord { y: 0, x: 32 }, 10) {
        return;
    }

    let mut level = 0i32;
    if !string_to_number(c_str_bytes(&input), &mut level) {
        return;
    }

    terminal::put_string_clear_to_eol("Produce how many objects?: ", Coord { y: 0, x: 0 });
    if !terminal::get_string_input(&mut input, Coord { y: 0, x: 27 }, 10) {
        return;
    }

    let mut count = 0i32;
    if !string_to_number(c_str_bytes(&input), &mut count) {
        return;
    }

    if count < 1 || level < 0 || level > 1200 {
        terminal::put_string_clear_to_eol("Parameters no good.", Coord { y: 0, x: 0 });
        return;
    }

    if count > 10000 {
        count = 10000;
    }

    let small_objects = terminal::get_input_confirmation("Small objects only?");

    terminal::put_string_clear_to_eol("File name: ", Coord { y: 0, x: 0 });

    let mut filename = [0u8; MORIA_MESSAGE_SIZE];
    if !terminal::get_string_input(&mut filename, Coord { y: 0, x: 11 }, 64) {
        return;
    }
    if c_str_bytes(&filename).is_empty() {
        return;
    }

    let path = c_str_bytes(&filename);
    let Ok(mut file_ptr) = File::create(path) else {
        terminal::put_string_clear_to_eol("File could not be opened.", Coord { y: 0, x: 0 });
        return;
    };

    let progress_text = format!("{count} random objects being produced...");
    terminal::put_string_clear_to_eol(&progress_text, Coord { y: 0, x: 0 });
    terminal::put_qio();

    let _ = writeln!(file_ptr, "*** Random Object Sampling:");
    let _ = writeln!(file_ptr, "*** {count} objects");
    let _ = writeln!(file_ptr, "*** For Level {level}");
    let _ = writeln!(file_ptr);
    let _ = writeln!(file_ptr);

    let treasure_id = popt();

    for _ in 0..count {
        let object_id = item_get_random_object_id(level, small_objects);
        with_state_mut(|state| {
            inventory_item_copy_to(
                state.sorted_objects[object_id as usize],
                &mut state.game.treasure.list[treasure_id as usize],
            );
        });
        crate::treasure::magic_treasure_magical_ability(treasure_id, level);

        with_state_mut(|state| {
            state.game.treasure.list[treasure_id as usize].identification |= ID_STORE_BOUGHT;
            spell_item_identify_and_remove_random_inscription_for_state(
                state,
                treasure_id as usize,
            );
            let item = &mut state.game.treasure.list[treasure_id as usize];
            if inventory_item_is_cursed(*item) {
                item_append_to_inscription(item, ID_DAMD);
            }
        });
        let line = with_state(|state| {
            let item = state.game.treasure.list[treasure_id as usize];
            let mut desc = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
            item_description_for_state(&mut desc, item, true, state);
            format!("{} {}", item.depth_first_found, c_str_bytes(&desc))
        });
        let _ = writeln!(file_ptr, "{line}");
    }

    pusht(treasure_id as u8);
    drop(file_ptr);

    terminal::put_string_clear_to_eol("Completed.", Coord { y: 0, x: 0 });
}

/// Port of `outputPlayerCharacterToFile` in game_files.cpp.
pub fn output_player_character_to_file(filename: &str) -> bool {
    TEST_OUTPUT_CALL_COUNT.with(|c| c.set(c.get().wrapping_add(1)));
    TEST_OUTPUT_LAST_PATH.with(|p| *p.borrow_mut() = filename.to_string());

    if let Some(result) = take_test_output_result() {
        if !result {
            terminal::print_message(Some(&format!("Can't open file {filename}:")));
        }
        return result;
    }

    output_player_character_to_file_impl(filename)
}

fn output_player_character_to_file_impl(filename: &str) -> bool {
    // C++ game_files.cpp:360-397 — open O_EXCL, optional replace, then write.
    // Use the same create-exclusive / replace flow on all platforms.
    let path = std::path::Path::new(filename);
    let mut created_exclusive = false;
    let open_ok = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(_) => {
            created_exclusive = true;
            true
        }
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            if terminal::get_input_confirmation(&format!("Replace existing file {filename}?")) {
                std::fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(path)
                    .is_ok()
            } else {
                false
            }
        }
        Err(_) => false,
    };

    if !open_ok {
        terminal::print_message(Some(&format!("Can't open file {filename}:")));
        return false;
    }

    // C++ closes the fd then fopen("w") — recreate/truncate for the write path.
    let mut file = match File::create(filename) {
        Ok(f) => f,
        Err(_) => {
            if created_exclusive {
                let _ = std::fs::remove_file(path);
            }
            terminal::print_message(Some(&format!("Can't open file {filename}:")));
            return false;
        }
    };

    if write_character_sheet_to_file(&mut file).is_err()
        || write_equipment_list_to_file(&mut file).is_err()
        || write_inventory_to_file(&mut file).is_err()
    {
        drop(file);
        terminal::print_message(Some(&format!("Can't open file {filename}:")));
        return false;
    }
    drop(file);

    terminal::put_string_clear_to_eol("Completed.", Coord { y: 0, x: 0 });
    true
}
