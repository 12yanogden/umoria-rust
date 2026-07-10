//! Port of src/wizard.cpp — wizard/debug commands.

use crate::config::dungeon::objects::OBJ_WIZARD;
use crate::config::identification::{ID_KNOWN2, ID_STORE_BOUGHT};
use crate::dungeon::{coord_in_bounds, dungeon_delete_object, dungeon_place_random_object_near};
use crate::dungeon_tile::MAX_CAVE_FLOOR;
use crate::game::{random_number, with_state, with_state_mut};
use crate::game_objects::popt;
use crate::helpers::{sscanf_lx, string_to_number};
use crate::identification::{item_identify_for_slot, item_replace_inscription, ItemIdentifySlot};
use crate::inventory::{inventory_item_copy_to, Inventory};
use crate::monster::update_monsters;
use crate::monster_manager::monster_summon;
use crate::player::{player_change_speed, PlayerAttr};
use crate::player_magic::{
    player_cure_blindness, player_cure_confusion, player_cure_poison, player_remove_fear,
};
use crate::player_stats::player_stat_restore;
use crate::spells::spell_remove_curse_from_all_worn_items;
use crate::treasure::magic_treasure_magical_ability;
use crate::types::{Coord_t, MORIA_MESSAGE_SIZE};
use crate::ui::{
    display_character_experience, draw_dungeon_panel, print_character_current_hit_points,
    print_character_current_mana, print_character_gold_value, print_character_max_hit_points,
    print_character_speed,
};
use crate::ui_io::terminal::{
    self, get_command, get_input_confirmation, get_string_input, message_line_clear,
    put_string_clear_to_eol, Coord,
};

const SHRT_MAX: i32 = 32_767;

fn ui_coord(x: i32) -> Coord {
    Coord { y: 0, x }
}

/// C++ wizard.cpp lines 13–28.
pub fn enter_wizard_mode() -> bool {
    let mut answer = false;

    if with_state(|state| state.game.noscore) == 0 {
        terminal::print_message(Some("Wizard mode is for debugging and experimenting."));
        answer = get_input_confirmation(
            "The game will not be scored if you enter wizard mode. Are you sure?",
        );
    }

    if with_state(|state| state.game.noscore != 0) || answer {
        with_state_mut(|state| {
            state.game.noscore |= 0x2;
            state.game.wizard_mode = true;
        });
        return true;
    }

    false
}

/// C++ wizard.cpp lines 30–49.
pub fn wizard_cure_all() {
    let _ = spell_remove_curse_from_all_worn_items();
    let _ = player_cure_blindness();
    let _ = player_cure_confusion();
    let _ = player_cure_poison();
    let _ = player_remove_fear();
    let _ = player_stat_restore(PlayerAttr::A_STR);
    let _ = player_stat_restore(PlayerAttr::A_INT);
    let _ = player_stat_restore(PlayerAttr::A_WIS);
    let _ = player_stat_restore(PlayerAttr::A_CON);
    let _ = player_stat_restore(PlayerAttr::A_DEX);
    let _ = player_stat_restore(PlayerAttr::A_CHR);

    with_state_mut(|state| {
        if state.py.flags.slow > 1 {
            state.py.flags.slow = 1;
        }
        if state.py.flags.image > 1 {
            state.py.flags.image = 1;
        }
    });
}

/// C++ wizard.cpp lines 52–64.
pub fn wizard_drop_random_items() {
    let count = with_state_mut(|state| {
        if state.game.command_count > 0 {
            let i = state.game.command_count as i32;
            state.game.command_count = 0;
            i
        } else {
            1
        }
    });

    let pos = with_state(|state| state.py.pos);
    dungeon_place_random_object_near(pos, count);
    draw_dungeon_panel();
}

/// C++ wizard.cpp lines 67–97.
pub fn wizard_jump_level() {
    let command_count = with_state(|state| state.game.command_count);
    let i = if command_count > 0 {
        let value = if command_count > 99 {
            0
        } else {
            command_count as i32
        };
        with_state_mut(|state| state.game.command_count = 0);
        value
    } else {
        let mut input = [0u8; MORIA_MESSAGE_SIZE];
        put_string_clear_to_eol("Go to which level (0-99) ? ", ui_coord(0));

        let mut value = -1;
        if get_string_input(&mut input, ui_coord(27), 10) {
            let _ = string_to_number(c_str(&input), &mut value);
        }
        value
    };

    if i >= 0 {
        with_state_mut(|state| {
            state.dg.current_level = i as i16;
            if state.dg.current_level > 99 {
                state.dg.current_level = 99;
            }
            state.dg.generate_new_level = true;
        });
    } else {
        message_line_clear();
    }
}

/// C++ wizard.cpp lines 100–110.
pub fn wizard_gain_experience() {
    with_state_mut(|state| {
        if state.game.command_count > 0 {
            state.py.misc.exp = state.game.command_count as i32;
            state.game.command_count = 0;
        } else if state.py.misc.exp == 0 {
            state.py.misc.exp = 1;
        } else {
            state.py.misc.exp = state.py.misc.exp.wrapping_mul(2);
        }
    });
    display_character_experience();
}

/// C++ wizard.cpp lines 113–119.
pub fn wizard_summon_monster() {
    let mut coord = with_state(|state| Coord_t {
        y: state.py.pos.y,
        x: state.py.pos.x,
    });

    let _ = monster_summon(&mut coord, true);
    update_monsters(false);
}

/// C++ wizard.cpp lines 122–143.
pub fn wizard_light_up_dungeon() {
    let flag = with_state(|state| {
        !state.dg.floor[state.py.pos.y as usize][state.py.pos.x as usize].permanent_light
    });

    with_state_mut(|state| {
        // C++ wizard.cpp:130-136 writes neighbors with no bounds check (UB on OOB).
        // Match all in-array writes; skip only indices outside the allocated floor.
        for y in 0..state.dg.height {
            for x in 0..state.dg.width {
                if state.dg.floor[y as usize][x as usize].feature_id <= MAX_CAVE_FLOOR {
                    for yy in y - 1..=y + 1 {
                        for xx in x - 1..=x + 1 {
                            if yy < 0
                                || xx < 0
                                || yy >= i16::from(crate::dungeon::MAX_HEIGHT)
                                || xx >= i16::from(crate::dungeon::MAX_WIDTH)
                            {
                                continue;
                            }
                            let tile = &mut state.dg.floor[yy as usize][xx as usize];
                            tile.permanent_light = flag;
                            if !flag {
                                tile.field_mark = false;
                            }
                        }
                    }
                }
            }
        }
    });

    draw_dungeon_panel();
}

/// C++ wizard.cpp lines 146–352.
pub fn wizard_character_adjustment() {
    let mut number = 0i32;
    let mut input = [0u8; MORIA_MESSAGE_SIZE];

    put_string_clear_to_eol("(3 - 118) Strength     = ", ui_coord(0));
    if get_string_input(&mut input, ui_coord(25), 3) {
        let valid_number = string_to_number(c_str(&input), &mut number);
        if valid_number && number > 2 && number < 119 {
            with_state_mut(|state| {
                state.py.stats.max[PlayerAttr::A_STR as usize] = number as u8;
            });
            let _ = player_stat_restore(PlayerAttr::A_STR);
        }
    } else {
        return;
    }

    put_string_clear_to_eol("(3 - 118) Intelligence = ", ui_coord(0));
    if get_string_input(&mut input, ui_coord(25), 3) {
        let valid_number = string_to_number(c_str(&input), &mut number);
        if valid_number && number > 2 && number < 119 {
            with_state_mut(|state| {
                state.py.stats.max[PlayerAttr::A_INT as usize] = number as u8;
            });
            let _ = player_stat_restore(PlayerAttr::A_INT);
        }
    } else {
        return;
    }

    put_string_clear_to_eol("(3 - 118) Wisdom       = ", ui_coord(0));
    if get_string_input(&mut input, ui_coord(25), 3) {
        let valid_number = string_to_number(c_str(&input), &mut number);
        if valid_number && number > 2 && number < 119 {
            with_state_mut(|state| {
                state.py.stats.max[PlayerAttr::A_WIS as usize] = number as u8;
            });
            let _ = player_stat_restore(PlayerAttr::A_WIS);
        }
    } else {
        return;
    }

    put_string_clear_to_eol("(3 - 118) Dexterity    = ", ui_coord(0));
    if get_string_input(&mut input, ui_coord(25), 3) {
        let valid_number = string_to_number(c_str(&input), &mut number);
        if valid_number && number > 2 && number < 119 {
            with_state_mut(|state| {
                state.py.stats.max[PlayerAttr::A_DEX as usize] = number as u8;
            });
            let _ = player_stat_restore(PlayerAttr::A_DEX);
        }
    } else {
        return;
    }

    put_string_clear_to_eol("(3 - 118) Constitution = ", ui_coord(0));
    if get_string_input(&mut input, ui_coord(25), 3) {
        let valid_number = string_to_number(c_str(&input), &mut number);
        if valid_number && number > 2 && number < 119 {
            with_state_mut(|state| {
                state.py.stats.max[PlayerAttr::A_CON as usize] = number as u8;
            });
            let _ = player_stat_restore(PlayerAttr::A_CON);
        }
    } else {
        return;
    }

    put_string_clear_to_eol("(3 - 118) Charisma     = ", ui_coord(0));
    if get_string_input(&mut input, ui_coord(25), 3) {
        let valid_number = string_to_number(c_str(&input), &mut number);
        if valid_number && number > 2 && number < 119 {
            with_state_mut(|state| {
                state.py.stats.max[PlayerAttr::A_CHR as usize] = number as u8;
            });
            let _ = player_stat_restore(PlayerAttr::A_CHR);
        }
    } else {
        return;
    }

    put_string_clear_to_eol("(1 - 32767) Hit points = ", ui_coord(0));
    if get_string_input(&mut input, ui_coord(25), 5) {
        let valid_number = string_to_number(c_str(&input), &mut number);
        if valid_number && number > 0 && number <= SHRT_MAX {
            with_state_mut(|state| {
                state.py.misc.max_hp = number as i16;
                state.py.misc.current_hp = number as i16;
                state.py.misc.current_hp_fraction = 0;
            });
            print_character_max_hit_points();
            print_character_current_hit_points();
        }
    } else {
        return;
    }

    put_string_clear_to_eol("(0 - 32767) Mana       = ", ui_coord(0));
    if get_string_input(&mut input, ui_coord(25), 5) {
        let valid_number = string_to_number(c_str(&input), &mut number);
        if valid_number && number > -1 && number <= SHRT_MAX {
            with_state_mut(|state| {
                state.py.misc.mana = number as i16;
                state.py.misc.current_mana = number as i16;
                state.py.misc.current_mana_fraction = 0;
            });
            print_character_current_mana();
        }
    } else {
        return;
    }

    let au = with_state(|state| state.py.misc.au);
    write_prompt_current(&mut input, &format!("Current={au}  Gold = "));
    number = c_strlen(&input) as i32;
    put_string_clear_to_eol(c_str(&input), ui_coord(0));
    if get_string_input(&mut input, ui_coord(number), 7) {
        let mut new_gold = 0;
        let valid_number = string_to_number(c_str(&input), &mut new_gold);
        if valid_number && new_gold > -1 {
            with_state_mut(|state| state.py.misc.au = new_gold);
            print_character_gold_value();
        }
    } else {
        return;
    }

    let chance = with_state(|state| state.py.misc.chance_in_search);
    write_prompt_current(
        &mut input,
        &format!("Current={chance}  (0-200) Searching = "),
    );
    number = c_strlen(&input) as i32;
    put_string_clear_to_eol(c_str(&input), ui_coord(0));
    if get_string_input(&mut input, ui_coord(number), 3) {
        let mut new_gold = 0;
        let valid_number = string_to_number(c_str(&input), &mut new_gold);
        if valid_number && number > -1 && number < 201 {
            with_state_mut(|state| state.py.misc.chance_in_search = number as i16);
        }
    } else {
        return;
    }

    let stealth = with_state(|state| state.py.misc.stealth_factor);
    write_prompt_current(
        &mut input,
        &format!("Current={stealth}  (-1-18) Stealth = "),
    );
    number = c_strlen(&input) as i32;
    put_string_clear_to_eol(c_str(&input), ui_coord(0));
    if get_string_input(&mut input, ui_coord(number), 3) {
        let valid_number = string_to_number(c_str(&input), &mut number);
        if valid_number && number > -2 && number < 19 {
            with_state_mut(|state| state.py.misc.stealth_factor = number as i16);
        }
    } else {
        return;
    }

    let disarm = with_state(|state| state.py.misc.disarm);
    write_prompt_current(
        &mut input,
        &format!("Current={disarm}  (0-200) Disarming = "),
    );
    number = c_strlen(&input) as i32;
    put_string_clear_to_eol(c_str(&input), ui_coord(0));
    if get_string_input(&mut input, ui_coord(number), 3) {
        let valid_number = string_to_number(c_str(&input), &mut number);
        if valid_number && number > -1 && number < 201 {
            with_state_mut(|state| state.py.misc.disarm = number as i16);
        }
    } else {
        return;
    }

    let saving_throw = with_state(|state| state.py.misc.saving_throw);
    write_prompt_current(
        &mut input,
        &format!("Current={saving_throw}  (0-100) Save = "),
    );
    number = c_strlen(&input) as i32;
    put_string_clear_to_eol(c_str(&input), ui_coord(0));
    if get_string_input(&mut input, ui_coord(number), 3) {
        let valid_number = string_to_number(c_str(&input), &mut number);
        if valid_number && number > -1 && number < 201 {
            with_state_mut(|state| state.py.misc.saving_throw = number as i16);
        }
    } else {
        return;
    }

    let bth = with_state(|state| state.py.misc.bth);
    write_prompt_current(
        &mut input,
        &format!("Current={bth}  (0-200) Base to hit = "),
    );
    number = c_strlen(&input) as i32;
    put_string_clear_to_eol(c_str(&input), ui_coord(0));
    if get_string_input(&mut input, ui_coord(number), 3) {
        let valid_number = string_to_number(c_str(&input), &mut number);
        if valid_number && number > -1 && number < 201 {
            with_state_mut(|state| state.py.misc.bth = number as i16);
        }
    } else {
        return;
    }

    let bth_bows = with_state(|state| state.py.misc.bth_with_bows);
    write_prompt_current(
        &mut input,
        &format!("Current={bth_bows}  (0-200) Bows/Throwing = "),
    );
    number = c_strlen(&input) as i32;
    put_string_clear_to_eol(c_str(&input), ui_coord(0));
    if get_string_input(&mut input, ui_coord(number), 3) {
        let valid_number = string_to_number(c_str(&input), &mut number);
        if valid_number && number > -1 && number < 201 {
            with_state_mut(|state| state.py.misc.bth_with_bows = number as i16);
        }
    } else {
        return;
    }

    let weight = with_state(|state| state.py.misc.weight);
    write_prompt_current(&mut input, &format!("Current={weight}  Weight = "));
    number = c_strlen(&input) as i32;
    put_string_clear_to_eol(c_str(&input), ui_coord(0));
    if get_string_input(&mut input, ui_coord(number), 3) {
        let valid_number = string_to_number(c_str(&input), &mut number);
        if valid_number && number > -1 {
            with_state_mut(|state| state.py.misc.weight = number as u16);
        }
    } else {
        return;
    }

    let mut command = 0u8;
    while get_command("Alter speed? (+/-)", &mut command) {
        if command == b'+' {
            player_change_speed(-1);
        } else if command == b'-' {
            player_change_speed(1);
        } else {
            break;
        }
        print_character_speed();
    }
}

/// C++ wizard.cpp lines 355–379.
pub fn wizard_request_object_id(id: &mut i32, label: &str, start_id: i32, end_id: i32) -> bool {
    let id_str = format!("{start_id}-{end_id}");
    let msg = format!("{label} ID ({id_str}): ");
    put_string_clear_to_eol(&msg, ui_coord(0));

    let mut input = [0u8; MORIA_MESSAGE_SIZE];
    if !get_string_input(&mut input, ui_coord(msg.len() as i32), 3) {
        return false;
    }

    let mut given_id = 0;
    if !string_to_number(c_str(&input), &mut given_id) {
        return false;
    }

    if given_id < start_id || given_id > end_id {
        put_string_clear_to_eol(&format!("Invalid ID. Must be {id_str}"), ui_coord(0));
        return false;
    }
    *id = given_id;
    true
}

/// C++ wizard.cpp lines 382–412.
pub fn wizard_generate_object() {
    let mut id = 0;
    if !wizard_request_object_id(&mut id, "Dungeon/Store object", 0, 366) {
        return;
    }

    let py_pos = with_state(|state| state.py.pos);
    let mut coord = Coord_t { y: 0, x: 0 };

    for i in 0..10 {
        coord.y = py_pos.y - 3 + random_number(5);
        coord.x = py_pos.x - 4 + random_number(7);

        let placement_ok = with_state(|state| {
            coord_in_bounds(coord)
                && state.dg.floor[coord.y as usize][coord.x as usize].feature_id <= MAX_CAVE_FLOOR
                && state.dg.floor[coord.y as usize][coord.x as usize].treasure_id == 0
        });

        if placement_ok {
            if with_state(|state| {
                state.dg.floor[coord.y as usize][coord.x as usize].treasure_id != 0
            }) {
                let _ = dungeon_delete_object(coord);
            }

            let free_treasure_id = popt();
            with_state_mut(|state| {
                state.dg.floor[coord.y as usize][coord.x as usize].treasure_id =
                    free_treasure_id as u8;
                inventory_item_copy_to(
                    id as i16,
                    &mut state.game.treasure.list[free_treasure_id as usize],
                );
            });
            let level = with_state(|state| i32::from(state.dg.current_level));
            magic_treasure_magical_ability(free_treasure_id, level);

            with_state_mut(|state| {
                let mut item_id = free_treasure_id;
                let _ = item_identify_for_slot(
                    state,
                    ItemIdentifySlot::Treasure,
                    free_treasure_id,
                    &mut item_id,
                );
            });

            let _ = i;
            break;
        }
    }
}

/// C++ wizard.cpp lines 415–571.
pub fn wizard_create_objects() {
    let mut number = 0i32;
    let mut input = [0u8; MORIA_MESSAGE_SIZE];

    terminal::print_message(Some("Warning: This routine can cause a fatal error."));

    let mut forge = Inventory {
        id: OBJ_WIZARD,
        special_name_id: 0,
        identification: ID_KNOWN2 | ID_STORE_BOUGHT,
        ..Default::default()
    };
    item_replace_inscription(&mut forge, b"wizard item");

    put_string_clear_to_eol("Tval   : ", ui_coord(0));
    if !get_string_input(&mut input, ui_coord(9), 3) {
        return;
    }
    if string_to_number(c_str(&input), &mut number) {
        forge.category_id = number as u8;
    }

    put_string_clear_to_eol("Tchar  : ", ui_coord(0));
    if !get_string_input(&mut input, ui_coord(9), 1) {
        return;
    }
    forge.sprite = input[0];

    put_string_clear_to_eol("Subval : ", ui_coord(0));
    if !get_string_input(&mut input, ui_coord(9), 5) {
        return;
    }
    if string_to_number(c_str(&input), &mut number) {
        forge.sub_category_id = number as u8;
    }

    put_string_clear_to_eol("Weight : ", ui_coord(0));
    if !get_string_input(&mut input, ui_coord(9), 5) {
        return;
    }
    if string_to_number(c_str(&input), &mut number) {
        forge.weight = number as u16;
    }

    put_string_clear_to_eol("Number : ", ui_coord(0));
    if !get_string_input(&mut input, ui_coord(9), 5) {
        return;
    }
    if string_to_number(c_str(&input), &mut number) {
        forge.items_count = number as u8;
    }

    put_string_clear_to_eol("Damage (dice): ", ui_coord(0));
    if !get_string_input(&mut input, ui_coord(15), 3) {
        return;
    }
    if string_to_number(c_str(&input), &mut number) {
        forge.damage.dice = number as u8;
    }

    put_string_clear_to_eol("Damage (sides): ", ui_coord(0));
    if !get_string_input(&mut input, ui_coord(16), 3) {
        return;
    }
    if string_to_number(c_str(&input), &mut number) {
        forge.damage.sides = number as u8;
    }

    put_string_clear_to_eol("+To hit: ", ui_coord(0));
    if !get_string_input(&mut input, ui_coord(9), 3) {
        return;
    }
    if string_to_number(c_str(&input), &mut number) {
        forge.to_hit = number as i16;
    }

    put_string_clear_to_eol("+To dam: ", ui_coord(0));
    if !get_string_input(&mut input, ui_coord(9), 3) {
        return;
    }
    if string_to_number(c_str(&input), &mut number) {
        forge.to_damage = number as i16;
    }

    put_string_clear_to_eol("AC     : ", ui_coord(0));
    if !get_string_input(&mut input, ui_coord(9), 3) {
        return;
    }
    if string_to_number(c_str(&input), &mut number) {
        forge.ac = number as i16;
    }

    put_string_clear_to_eol("+To AC : ", ui_coord(0));
    if !get_string_input(&mut input, ui_coord(9), 3) {
        return;
    }
    if string_to_number(c_str(&input), &mut number) {
        forge.to_ac = number as i16;
    }

    put_string_clear_to_eol("P1     : ", ui_coord(0));
    if !get_string_input(&mut input, ui_coord(9), 5) {
        return;
    }
    if string_to_number(c_str(&input), &mut number) {
        forge.misc_use = number as i16;
    }

    put_string_clear_to_eol("Flags (In HEX): ", ui_coord(0));
    if !get_string_input(&mut input, ui_coord(16), 8) {
        return;
    }

    let mut input_number = 0i32;
    let _ = sscanf_lx(c_str(&input), &mut input_number);
    forge.flags = input_number as u32;

    put_string_clear_to_eol("Cost : ", ui_coord(0));
    if !get_string_input(&mut input, ui_coord(9), 8) {
        return;
    }
    if string_to_number(c_str(&input), &mut input_number) {
        forge.cost = input_number;
    }

    put_string_clear_to_eol("Level : ", ui_coord(0));
    if !get_string_input(&mut input, ui_coord(10), 3) {
        return;
    }
    if string_to_number(c_str(&input), &mut number) {
        forge.depth_first_found = number as u8;
    }

    if get_input_confirmation("Allocate?") {
        let py_pos = with_state(|state| state.py.pos);
        if with_state(|state| state.dg.floor[py_pos.y as usize][py_pos.x as usize].treasure_id != 0)
        {
            let _ = dungeon_delete_object(py_pos);
        }

        let slot = popt();
        with_state_mut(|state| {
            state.game.treasure.list[slot as usize] = forge;
            state.dg.floor[py_pos.y as usize][py_pos.x as usize].treasure_id = slot as u8;
        });
        terminal::print_message(Some("Allocated."));
    } else {
        terminal::print_message(Some("Aborted."));
    }
}

fn write_prompt_current(input: &mut [u8; MORIA_MESSAGE_SIZE], prompt: &str) {
    input.fill(0);
    let bytes = prompt.as_bytes();
    let len = bytes.len().min(MORIA_MESSAGE_SIZE - 1);
    input[..len].copy_from_slice(&bytes[..len]);
}

fn c_str(buf: &[u8]) -> &str {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    std::str::from_utf8(&buf[..end]).unwrap_or("")
}

fn c_strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}
