//! Port of src/ui_inventory.cpp — see phase_3.3.

use std::os::raw::c_char;

use crate::config::identification::ID_DAMD;
use crate::game::{with_state, with_state_mut};
use crate::identification::{item_append_to_inscription, item_description};
use crate::inventory::{
    inventory_can_carry_item_count, inventory_carry_item, inventory_destroy_item,
    inventory_drop_item, inventory_item_is_cursed, Inventory, PlayerEquipment,
    PLAYER_INVENTORY_SIZE,
};
use crate::player::{
    player_adjust_bonuses_for_item, player_carrying_load_limit, player_is_wielding_item,
    player_left_hand_ring_empty, player_recalculate_bonuses, player_right_hand_ring_empty,
    player_strength, player_take_off, player_worn_item_is_cursed, PlayerAttr,
};
use crate::treasure::{
    TV_AMULET, TV_ARROW, TV_BOLT, TV_BOOTS, TV_BOW, TV_CLOAK, TV_DIGGING, TV_GLOVES, TV_HAFTED,
    TV_HARD_ARMOR, TV_HELM, TV_LIGHT, TV_MAX_WEAR, TV_MIN_WEAR, TV_NOTHING, TV_POLEARM, TV_RING,
    TV_SHIELD, TV_SLING_AMMO, TV_SOFT_ARMOR, TV_SPIKE, TV_SWORD,
};
use crate::types::{Obj_desc_t, Screen, CNIL, MORIA_OBJ_DESC_SIZE_LEN};
use crate::ui_io::terminal::{self, Coord};
use crate::ui_io::ESCAPE;
use crate::{inventory::ITEM_GROUP_MIN, inventory::ITEM_SINGLE_STACK_MAX};

/// C++ ui_inventory.cpp:8-14 — `inventoryItemWeightText`.
#[must_use]
pub fn inventory_item_weight_text(item_id: usize) -> String {
    with_state_mut(|state| {
        let item = &state.py.inventory[item_id];
        let total_weight = i32::from(item.weight) * i32::from(item.items_count);
        let quotient = total_weight / 10;
        let remainder = total_weight % 10;
        format!("{quotient:3}.{remainder} lb")
    })
}

fn inventory_item_weight_text_buf(text: &mut Obj_desc_t, item_id: usize) {
    let s = inventory_item_weight_text(item_id);
    write_c_string(text, &s);
}

/// C++ ui_inventory.cpp:93-122 — `playerItemWearingDescription`.
#[must_use]
pub fn player_item_wearing_description(body_location: u8) -> &'static str {
    match body_location {
        x if x == PlayerEquipment::Wield as u8 => "wielding",
        x if x == PlayerEquipment::Head as u8 => "wearing on your head",
        x if x == PlayerEquipment::Neck as u8 => "wearing around your neck",
        x if x == PlayerEquipment::Body as u8 => "wearing on your body",
        x if x == PlayerEquipment::Arm as u8 => "wearing on your arm",
        x if x == PlayerEquipment::Hands as u8 => "wearing on your hands",
        x if x == PlayerEquipment::Right as u8 => "wearing on your right hand",
        x if x == PlayerEquipment::Left as u8 => "wearing on your left hand",
        x if x == PlayerEquipment::Feet as u8 => "wearing on your feet",
        x if x == PlayerEquipment::Outer as u8 => "wearing about your body",
        x if x == PlayerEquipment::Light as u8 => "using to light the way",
        x if x == PlayerEquipment::Auxiliary as u8 => "holding ready by your side",
        _ => "carrying in your pack",
    }
}

/// C++ ui_inventory.cpp:124-156 — `equipmentPositionDescription`.
#[must_use]
pub fn equipment_position_description(id: u8, weight: u16, str_used: u8) -> &'static str {
    match id {
        x if x == PlayerEquipment::Wield as u8 => {
            if i32::from(str_used) * 15 < i32::from(weight) {
                "Just lifting"
            } else {
                "Wielding"
            }
        }
        x if x == PlayerEquipment::Head as u8 => "On head",
        x if x == PlayerEquipment::Neck as u8 => "Around neck",
        x if x == PlayerEquipment::Body as u8 => "On body",
        x if x == PlayerEquipment::Arm as u8 => "On arm",
        x if x == PlayerEquipment::Hands as u8 => "On hands",
        x if x == PlayerEquipment::Right as u8 => "On right hand",
        x if x == PlayerEquipment::Left as u8 => "On left hand",
        x if x == PlayerEquipment::Feet as u8 => "On feet",
        x if x == PlayerEquipment::Outer as u8 => "About body",
        x if x == PlayerEquipment::Light as u8 => "Light source",
        x if x == PlayerEquipment::Auxiliary as u8 => "Spare weapon",
        _ => "Unknown equipment position ID",
    }
}

fn equipment_position_description_state(id: u8, weight: u16) -> &'static str {
    with_state_mut(|state| {
        let str_used = state.py.stats.used[PlayerAttr::A_STR as usize];
        equipment_position_description(id, weight, str_used)
    })
}

fn write_c_string(out: &mut Obj_desc_t, s: &str) {
    let bytes = s.as_bytes();
    let n = bytes.len().min(MORIA_OBJ_DESC_SIZE_LEN - 1);
    out[..n].copy_from_slice(&bytes[..n]);
    out[n] = 0;
}

fn truncate_desc(desc: &mut Obj_desc_t, lim: usize) {
    if lim < MORIA_OBJ_DESC_SIZE_LEN {
        desc[lim] = 0;
    }
}

fn mask_skipped(mask: Option<&[u8]>, index: usize) -> bool {
    mask.is_some_and(|m| index < m.len() && m[index] == 0)
}

/// C++ ui_inventory.cpp:22-90 — `displayInventoryItems`.
pub fn display_inventory_items(
    item_id_start: i32,
    item_id_end: i32,
    weighted: bool,
    mut column: i32,
    mask: Option<&[u8]>,
) -> i32 {
    let mut descriptions: Vec<String> = vec![String::new(); (item_id_end + 1).max(0) as usize + 1];

    let mut len = 79 - column;

    let lim = if weighted { 68 } else { 76 };

    with_state_mut(|state| {
        for i in item_id_start..=item_id_end {
            let iu = i as usize;
            if mask_skipped(mask, iu) {
                continue;
            }

            let mut description = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
            item_description(&mut description, state.py.inventory[iu], true);
            truncate_desc(&mut description, lim);

            let desc_str = c_string_to_str(&description);
            descriptions[iu] = format!("{}) {}", (b'a' + i as u8) as char, desc_str);

            let mut l = descriptions[iu].len() as i32 + 2;
            if weighted {
                l += 9;
            }
            if l > len {
                len = l;
            }
        }
    });

    column = 79 - len;
    if column < 0 {
        column = 0;
    }

    let mut current_line = 1;

    for i in item_id_start..=item_id_end {
        let iu = i as usize;
        if mask_skipped(mask, iu) {
            continue;
        }

        if column == 0 {
            terminal::put_string_clear_to_eol(
                &descriptions[iu],
                Coord {
                    y: current_line,
                    x: column,
                },
            );
        } else {
            terminal::put_string(
                "  ",
                Coord {
                    y: current_line,
                    x: column,
                },
            );
            terminal::put_string_clear_to_eol(
                &descriptions[iu],
                Coord {
                    y: current_line,
                    x: column + 2,
                },
            );
        }

        if weighted {
            let mut text = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
            inventory_item_weight_text_buf(&mut text, iu);
            terminal::put_string_clear_to_eol(
                c_string_to_str(&text),
                Coord {
                    y: current_line,
                    x: 71,
                },
            );
        }

        current_line += 1;
    }

    column
}

fn c_string_to_str(buf: &[u8]) -> &str {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    std::str::from_utf8(&buf[..end]).unwrap_or("")
}

/// C++ ui_inventory.cpp:160-234 — `displayEquipment`.
pub fn display_equipment(show_weights: bool, mut column: i32) -> i32 {
    let mut descriptions: Vec<String> = Vec::new();

    let mut len = 79 - column;
    let lim = if show_weights { 52 } else { 60 };

    with_state_mut(|state| {
        let mut line = 0usize;
        for i in PlayerEquipment::Wield as usize..PLAYER_INVENTORY_SIZE as usize {
            if state.py.inventory[i].category_id == TV_NOTHING {
                continue;
            }

            let equipped =
                equipment_position_description_state(i as u8, state.py.inventory[i].weight);

            let mut description = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
            item_description(&mut description, state.py.inventory[i], true);
            truncate_desc(&mut description, lim);

            let desc_str = c_string_to_str(&description);
            descriptions.push(format!(
                "{}) {:14}: {}",
                (line as u8 + b'a') as char,
                equipped,
                desc_str
            ));

            let mut l = descriptions[line].len() as i32 + 2;
            if show_weights {
                l += 9;
            }
            if l > len {
                len = l;
            }
            line += 1;
        }
    });

    column = 79 - len;
    if column < 0 {
        column = 0;
    }

    let mut line = 0;
    with_state_mut(|state| {
        for i in PlayerEquipment::Wield as usize..PLAYER_INVENTORY_SIZE as usize {
            if state.py.inventory[i].category_id == TV_NOTHING {
                continue;
            }

            if column == 0 {
                terminal::put_string_clear_to_eol(
                    &descriptions[line],
                    Coord {
                        y: line as i32 + 1,
                        x: column,
                    },
                );
            } else {
                terminal::put_string(
                    "  ",
                    Coord {
                        y: line as i32 + 1,
                        x: column,
                    },
                );
                terminal::put_string_clear_to_eol(
                    &descriptions[line],
                    Coord {
                        y: line as i32 + 1,
                        x: column + 2,
                    },
                );
            }

            if show_weights {
                let mut text = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
                inventory_item_weight_text_buf(&mut text, i);
                terminal::put_string_clear_to_eol(
                    c_string_to_str(&text),
                    Coord {
                        y: line as i32 + 1,
                        x: 71,
                    },
                );
            }

            line += 1;
        }
    });

    terminal::erase_line(Coord {
        y: line as i32 + 1,
        x: column,
    });

    column
}

/// C++ ui_inventory.cpp:236-250 — `showEquipmentHelpMenu`.
fn show_equipment_help_menu(mut left_column: i32) -> i32 {
    if left_column > 52 {
        left_column = 52;
    }

    terminal::put_string_clear_to_eol(
        "  ESC: exit",
        Coord {
            y: 1,
            x: left_column,
        },
    );
    terminal::put_string_clear_to_eol(
        "  w  : wear or wield object",
        Coord {
            y: 2,
            x: left_column,
        },
    );
    terminal::put_string_clear_to_eol(
        "  t  : take off item",
        Coord {
            y: 3,
            x: left_column,
        },
    );
    terminal::put_string_clear_to_eol(
        "  d  : drop object",
        Coord {
            y: 4,
            x: left_column,
        },
    );
    terminal::put_string_clear_to_eol(
        "  x  : exchange weapons",
        Coord {
            y: 5,
            x: left_column,
        },
    );
    terminal::put_string_clear_to_eol(
        "  i  : inventory of pack",
        Coord {
            y: 6,
            x: left_column,
        },
    );
    terminal::put_string_clear_to_eol(
        "  e  : list used equipment",
        Coord {
            y: 7,
            x: left_column,
        },
    );

    7
}

/// Pure helper — per-branch `currentLinePos` from ui_inventory.cpp:287-310.
#[must_use]
pub fn switch_screen_line_pos(
    next_screen: Screen,
    unique_items: i16,
    wear_low_id: i32,
    wear_high_id: i32,
    equipment_count: i16,
) -> i32 {
    match next_screen {
        Screen::Blank | Screen::Wrong => 0,
        Screen::Help => 7,
        Screen::Inventory => i32::from(unique_items),
        Screen::Wear => wear_high_id - wear_low_id + 1,
        Screen::Equipment => i32::from(equipment_count),
    }
}

/// Pure helper — trailing-clear branch from ui_inventory.cpp:312-322.
/// Returns `(screen_bottom_pos, single_erase_at_bottom)`.
#[must_use]
pub fn apply_switch_screen_bottom_pos(
    current_line_pos: i32,
    screen_bottom_pos: i32,
) -> (i32, bool) {
    if current_line_pos >= screen_bottom_pos {
        (current_line_pos + 1, true)
    } else {
        (screen_bottom_pos, false)
    }
}

/// C++ ui_inventory.cpp:281-323 — `uiCommandSwitchScreen`.
pub fn ui_command_switch_screen(next_screen: Screen) {
    let prep = with_state_mut(|state| {
        if next_screen == state.game.screen.current_screen_id {
            return None;
        }
        state.game.screen.current_screen_id = next_screen;
        Some((
            state.game.screen.screen_left_pos,
            state.game.screen.screen_bottom_pos,
            state.options.show_inventory_weights,
            state.game.screen.wear_low_id,
            state.game.screen.wear_high_id,
            i32::from(state.py.pack.unique_items),
            i32::from(state.py.equipment_count),
        ))
    });

    let Some((mut left, mut bottom, weighted, wear_low, wear_high, unique_items, equipment_count)) =
        prep
    else {
        return;
    };

    let current_line_pos = match next_screen {
        Screen::Blank | Screen::Wrong => 0,
        Screen::Help => show_equipment_help_menu(left),
        Screen::Inventory => {
            left = display_inventory_items(0, unique_items - 1, weighted, left, None);
            with_state_mut(|state| state.game.screen.screen_left_pos = left);
            unique_items
        }
        Screen::Wear => {
            left = display_inventory_items(wear_low, wear_high, weighted, left, None);
            with_state_mut(|state| state.game.screen.screen_left_pos = left);
            wear_high - wear_low + 1
        }
        Screen::Equipment => {
            left = display_equipment(weighted, left);
            with_state_mut(|state| state.game.screen.screen_left_pos = left);
            equipment_count
        }
    };

    let (new_bottom, single) = apply_switch_screen_bottom_pos(current_line_pos, bottom);
    bottom = new_bottom;
    with_state_mut(|state| state.game.screen.screen_bottom_pos = bottom);

    if single {
        terminal::erase_line(Coord { y: bottom, x: left });
        return;
    }

    let mut pos = current_line_pos + 1;
    while pos <= bottom {
        terminal::erase_line(Coord { y: pos, x: left });
        pos += 1;
    }
}

/// C++ ui_inventory.cpp:326-337 — `verifyAction`.
fn verify_action(prompt: &str, item: i32) -> bool {
    let mut description = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
    with_state_mut(|state| {
        item_description(&mut description, state.py.inventory[item as usize], true);
    });

    let len = description
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(description.len());
    if len > 0 {
        description[len - 1] = b'?';
    }

    let msg = format!("{} {}", prompt, c_string_to_str(&description));
    terminal::get_input_confirmation(&msg)
}

/// C++ ui_inventory.cpp:339-364 — `requestAndShowInventoryScreen`.
pub fn request_and_show_inventory_screen(recover_screen: bool) {
    enum ResumeAction {
        Done,
        Abort,
        Confirm,
        Switch(Screen),
    }

    let action = with_state_mut(|state| {
        if state.game.doing_inventory_command == 0 {
            state.game.screen.screen_left_pos = 50;
            state.game.screen.screen_bottom_pos = 0;
            state.game.screen.current_screen_id = Screen::Blank;
            return ResumeAction::Done;
        }

        if state.screen_has_changed {
            if recover_screen {
                state.game.doing_inventory_command = 0;
                return ResumeAction::Abort;
            }
            return ResumeAction::Confirm;
        }

        let current_screen = state.game.screen.current_screen_id;
        state.game.screen.current_screen_id = Screen::Wrong;
        ResumeAction::Switch(current_screen)
    });

    match action {
        ResumeAction::Done | ResumeAction::Abort => {}
        ResumeAction::Confirm => {
            if !terminal::get_input_confirmation("Continuing with inventory command?") {
                with_state_mut(|state| state.game.doing_inventory_command = 0);
            } else {
                with_state_mut(|state| {
                    state.game.screen.screen_left_pos = 50;
                    state.game.screen.screen_bottom_pos = 0;
                });
                let screen = with_state_mut(|state| {
                    let current = state.game.screen.current_screen_id;
                    state.game.screen.current_screen_id = Screen::Wrong;
                    current
                });
                ui_command_switch_screen(screen);
            }
        }
        ResumeAction::Switch(screen) => ui_command_switch_screen(screen),
    }
}

/// C++ ui_inventory.cpp:366-383.
pub fn ui_command_inventory_take_off_item(selecting: bool) -> bool {
    enum Outcome {
        KeepSelecting,
        Switch(Screen),
        Done,
        Message(&'static str),
    }

    let outcome = with_state_mut(|state| {
        if state.py.equipment_count == 0 {
            return Outcome::Message("You are not using any equipment.");
        }

        if state.py.pack.unique_items >= i16::from(PlayerEquipment::Wield as u8)
            && state.game.doing_inventory_command == 0
        {
            return Outcome::Message("You will have to drop something first.");
        }

        if state.game.screen.current_screen_id != Screen::Blank {
            Outcome::Switch(Screen::Equipment)
        } else {
            Outcome::Done
        }
    });

    match outcome {
        Outcome::KeepSelecting => selecting,
        Outcome::Message(msg) => {
            terminal::print_message(Some(msg));
            selecting
        }
        Outcome::Switch(screen) => {
            ui_command_switch_screen(screen);
            true
        }
        Outcome::Done => true,
    }
}

/// C++ ui_inventory.cpp:385-406.
pub fn ui_command_inventory_drop_item(command: &mut u8, selecting: bool) -> bool {
    enum Outcome {
        KeepSelecting,
        RemapRemove(SwitchScreen),
        SwitchInventory,
        Done,
        Message(&'static str),
    }

    enum SwitchScreen {
        None,
        Equipment,
    }

    let outcome = with_state_mut(|state| {
        if state.py.pack.unique_items == 0 && state.py.equipment_count == 0 {
            return Outcome::Message("But you're not carrying anything.");
        }

        let y = state.py.pos.y as usize;
        let x = state.py.pos.x as usize;
        if state.dg.floor[y][x].treasure_id != 0 {
            return Outcome::Message("There's no room to drop anything here.");
        }

        if (state.game.screen.current_screen_id == Screen::Equipment
            && state.py.equipment_count > 0)
            || state.py.pack.unique_items == 0
        {
            let switch = if state.game.screen.current_screen_id != Screen::Blank {
                SwitchScreen::Equipment
            } else {
                SwitchScreen::None
            };
            Outcome::RemapRemove(switch)
        } else if state.game.screen.current_screen_id != Screen::Blank {
            Outcome::SwitchInventory
        } else {
            Outcome::Done
        }
    });

    match outcome {
        Outcome::KeepSelecting => selecting,
        Outcome::Message(msg) => {
            terminal::print_message(Some(msg));
            selecting
        }
        Outcome::RemapRemove(switch) => {
            *command = b'r';
            if matches!(switch, SwitchScreen::Equipment) {
                ui_command_switch_screen(Screen::Equipment);
            }
            true
        }
        Outcome::SwitchInventory => {
            ui_command_switch_screen(Screen::Inventory);
            true
        }
        Outcome::Done => true,
    }
}

/// C++ ui_inventory.cpp:408-432.
pub fn ui_command_inventory_wear_wield_item(selecting: bool) -> bool {
    enum Outcome {
        KeepSelecting,
        SwitchWear,
        Done,
        Message(&'static str),
    }

    let outcome = with_state_mut(|state| {
        state.game.screen.wear_low_id = 0;
        while state.game.screen.wear_low_id < i32::from(state.py.pack.unique_items)
            && state.py.inventory[state.game.screen.wear_low_id as usize].category_id > TV_MAX_WEAR
        {
            state.game.screen.wear_low_id += 1;
        }

        state.game.screen.wear_high_id = state.game.screen.wear_low_id;
        while state.game.screen.wear_high_id < i32::from(state.py.pack.unique_items)
            && state.py.inventory[state.game.screen.wear_high_id as usize].category_id
                >= TV_MIN_WEAR
        {
            state.game.screen.wear_high_id += 1;
        }
        state.game.screen.wear_high_id -= 1;

        if state.game.screen.wear_low_id > state.game.screen.wear_high_id {
            return Outcome::Message("You have nothing to wear or wield.");
        }

        if state.game.screen.current_screen_id != Screen::Blank
            && state.game.screen.current_screen_id != Screen::Inventory
        {
            Outcome::SwitchWear
        } else {
            Outcome::Done
        }
    });

    match outcome {
        Outcome::KeepSelecting => selecting,
        Outcome::Message(msg) => {
            terminal::print_message(Some(msg));
            selecting
        }
        Outcome::SwitchWear => {
            ui_command_switch_screen(Screen::Wear);
            true
        }
        Outcome::Done => true,
    }
}

fn ui_command_inventory_unwield_item() {
    if !player_is_wielding_item() {
        terminal::print_message(Some("But you are wielding no weapons."));
        return;
    }

    if player_worn_item_is_cursed(PlayerEquipment::Wield) {
        let mut description = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
        with_state_mut(|state| {
            item_description(
                &mut description,
                state.py.inventory[PlayerEquipment::Wield as usize],
                false,
            );
        });
        let msg = format!(
            "The {} you are wielding appears to be cursed.",
            c_string_to_str(&description)
        );
        terminal::print_message(Some(&msg));
        return;
    }

    with_state_mut(|state| {
        state.game.player_free_turn = false;
    });

    with_state_mut(|state| {
        state.py.inventory.swap(
            PlayerEquipment::Auxiliary as usize,
            PlayerEquipment::Wield as usize,
        );
    });

    with_state_mut(|state| {
        if state.game.screen.current_screen_id == Screen::Equipment {
            state.game.screen.screen_left_pos = display_equipment(
                state.options.show_inventory_weights,
                state.game.screen.screen_left_pos,
            );
        }
    });

    with_state_mut(|state| {
        player_adjust_bonuses_for_item(state.py.inventory[PlayerEquipment::Auxiliary as usize], -1);
        player_adjust_bonuses_for_item(state.py.inventory[PlayerEquipment::Wield as usize], 1);
    });

    with_state_mut(|state| {
        if state.py.inventory[PlayerEquipment::Wield as usize].category_id != TV_NOTHING {
            let label = *b"Primary weapon   : \0";
            let mut description = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
            item_description(
                &mut description,
                state.py.inventory[PlayerEquipment::Wield as usize],
                true,
            );
            let label_str = c_string_to_str(&label);
            let msg = format!("{}{}", label_str, c_string_to_str(&description));
            terminal::print_message(Some(&msg));
        } else {
            terminal::print_message(Some("No primary weapon."));
        }
    });

    with_state_mut(|state| {
        state.py.weapon_is_heavy = false;
    });
    player_strength();
}

/// C++ ui_inventory.cpp:484-506 — `inventoryGetItemMatchingInscription`.
#[must_use]
pub fn inventory_get_item_matching_inscription(which: u8, command: u8, from: i32, to: i32) -> i32 {
    with_state_mut(|state| {
        if which.is_ascii_digit() && command != b'r' && command != b't' {
            let mut m = from;
            while m <= to
                && m < i32::from(PLAYER_INVENTORY_SIZE)
                && (state.py.inventory[m as usize].inscription[0] != which as c_char
                    || state.py.inventory[m as usize].inscription[1] != 0)
            {
                m += 1;
            }
            if m <= to {
                m
            } else {
                -1
            }
        } else if which.is_ascii_uppercase() {
            i32::from(which - b'A')
        } else {
            i32::from(which).wrapping_sub(i32::from(b'a'))
        }
    })
}

/// C++ ui_inventory.cpp:508-523 — `buildCommandHeading`.
#[must_use]
pub fn build_command_heading(
    from: i32,
    to: i32,
    swap: &str,
    command: u8,
    prompt: &str,
    current_screen_id: Screen,
) -> String {
    let from_c = (from + i32::from(b'a')) as u8 as char;
    let to_c = (to + i32::from(b'a')) as u8 as char;
    let list = if current_screen_id == Screen::Blank {
        ", * to list"
    } else {
        ""
    };
    let digits = if command == b'w' || command == b'd' {
        ", 0-9"
    } else {
        ""
    };
    format!(
        "({from_c}-{to_c}{list}{swap}{digits}, space to break, ESC to exit) {prompt} which one?"
    )
}

fn build_command_heading_state(
    from: i32,
    to: i32,
    swap: &str,
    command: u8,
    prompt: &str,
) -> String {
    with_state_mut(|state| {
        build_command_heading(
            from,
            to,
            swap,
            command,
            prompt,
            state.game.screen.current_screen_id,
        )
    })
}

fn change_screen_for_command(command: u8) {
    let screen = with_state_mut(|state| {
        if command == b't' || command == b'r' {
            Screen::Equipment
        } else if command == b'w' && state.game.screen.current_screen_id != Screen::Inventory {
            Screen::Wear
        } else {
            Screen::Inventory
        }
    });
    ui_command_switch_screen(screen);
}

fn flip_inventory_equipment_screens() {
    let next = with_state_mut(|state| {
        if state.game.screen.current_screen_id == Screen::Equipment {
            Some(Screen::Inventory)
        } else if state.game.screen.current_screen_id == Screen::Inventory {
            Some(Screen::Equipment)
        } else {
            None
        }
    });
    if let Some(screen) = next {
        ui_command_switch_screen(screen);
    }
}

fn request_put_ring_on_which_hand() -> i32 {
    let mut hand = 0i32;

    loop {
        let mut query = 0u8;
        if !terminal::get_menu_item_id("Put ring on which hand (l/r/L/R)?", &mut query) {
            hand = -1;
        } else if query == b'l' {
            hand = PlayerEquipment::Left as i32;
        } else if query == b'r' {
            hand = PlayerEquipment::Right as i32;
        } else {
            if query == b'L' {
                hand = PlayerEquipment::Left as i32;
            } else if query == b'R' {
                hand = PlayerEquipment::Right as i32;
            } else {
                terminal::terminal_bell_sound();
            }
            if hand != 0 && !verify_action("Replace", hand) {
                hand = 0;
            }
        }

        if hand != 0 {
            break;
        }
    }

    hand
}

/// C++ ui_inventory.cpp:572-628 — `inventoryGetSlotToWearEquipment`.
#[must_use]
pub fn inventory_get_slot_to_wear_equipment(category_id: u8) -> i32 {
    let mut slot = -1i32;

    match category_id {
        TV_SLING_AMMO | TV_BOLT | TV_ARROW | TV_BOW | TV_HAFTED | TV_POLEARM | TV_SWORD
        | TV_DIGGING | TV_SPIKE => slot = PlayerEquipment::Wield as i32,
        TV_LIGHT => slot = PlayerEquipment::Light as i32,
        TV_BOOTS => slot = PlayerEquipment::Feet as i32,
        TV_GLOVES => slot = PlayerEquipment::Hands as i32,
        TV_CLOAK => slot = PlayerEquipment::Outer as i32,
        TV_HELM => slot = PlayerEquipment::Head as i32,
        TV_SHIELD => slot = PlayerEquipment::Arm as i32,
        TV_HARD_ARMOR | TV_SOFT_ARMOR => slot = PlayerEquipment::Body as i32,
        TV_AMULET => slot = PlayerEquipment::Neck as i32,
        TV_RING => {
            if player_right_hand_ring_empty() {
                slot = PlayerEquipment::Right as i32;
            } else if player_left_hand_ring_empty() {
                slot = PlayerEquipment::Left as i32;
            } else {
                slot = request_put_ring_on_which_hand();
            }
        }
        _ => {
            terminal::print_message(Some("IMPOSSIBLE: I don't see how you can use that."));
        }
    }

    slot
}

fn inventory_item_is_cursed_message(item_id: i32) {
    let mut description = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
    with_state_mut(|state| {
        item_description(
            &mut description,
            state.py.inventory[item_id as usize],
            false,
        );
    });

    let mut msg = format!("The {} you are ", c_string_to_str(&description));
    if item_id == PlayerEquipment::Head as i32 {
        msg.push_str("wielding ");
    } else {
        msg.push_str("wearing ");
    }
    msg.push_str("appears to be cursed.");
    terminal::print_message(Some(&msg));
}

fn execute_remove_item_command(
    mut selecting: bool,
    item_id: i32,
    command: &mut u8,
    which: &[u8],
    prompt: &str,
) -> bool {
    let mut item_id_to_take_off = item_id;
    let mut eq_id = 21i32;

    with_state_mut(|state| loop {
        eq_id += 1;
        if state.py.inventory[eq_id as usize].category_id != TV_NOTHING {
            item_id_to_take_off -= 1;
        }
        if item_id_to_take_off < 0 {
            break;
        }
    });

    let which_ch = which.first().copied().unwrap_or(0);
    let mut resolved = eq_id;

    if which_ch.is_ascii_uppercase() && !verify_action(prompt, resolved) {
        resolved = -1;
    } else {
        with_state_mut(|state| {
            if inventory_item_is_cursed(state.py.inventory[resolved as usize]) {
                resolved = -1;
                terminal::print_message(Some("Hmmm, it seems to be cursed."));
            } else if *command == b't'
                && !inventory_can_carry_item_count(state.py.inventory[resolved as usize])
            {
                let y = state.py.pos.y as usize;
                let x = state.py.pos.x as usize;
                if state.dg.floor[y][x].treasure_id != 0 {
                    resolved = -1;
                    terminal::print_message(Some("You can't carry it."));
                } else if terminal::get_input_confirmation("You can't carry it.  Drop it?") {
                    *command = b'r';
                } else {
                    resolved = -1;
                }
            }
        });
    }

    if resolved >= 0 {
        if *command == b'r' {
            inventory_drop_item(resolved, true);
            with_state_mut(|state| {
                if state.py.pack.unique_items == 0 && state.py.equipment_count == 0 {
                    state.py.pack.weight = 0;
                }
            });
        } else {
            // inventory_carry_item / player_take_off each borrow state; do not nest.
            let item = with_state(|state| state.py.inventory[resolved as usize]);
            let id = inventory_carry_item(item);
            player_take_off(resolved, id);
        }

        player_strength();
        with_state_mut(|state| {
            state.game.player_free_turn = false;
        });

        if *command == b'r' {
            selecting = false;
        }
    }

    selecting
}

fn execute_wear_item_command(item_id: i32, which: &[u8], prompt: &str) {
    let which_ch = which.first().copied().unwrap_or(0);
    let mut slot = 0i32;
    let mut resolved = item_id;

    if which_ch.is_ascii_uppercase() && !verify_action(prompt, resolved) {
        resolved = -1;
    } else {
        with_state_mut(|state| {
            slot = inventory_get_slot_to_wear_equipment(
                state.py.inventory[resolved as usize].category_id,
            );
            if slot == -1 {
                resolved = -1;
            }
        });
    }

    if resolved >= 0 {
        with_state_mut(|state| {
            if state.py.inventory[slot as usize].category_id != TV_NOTHING {
                if inventory_item_is_cursed(state.py.inventory[slot as usize]) {
                    inventory_item_is_cursed_message(slot);
                    resolved = -1;
                } else if state.py.inventory[resolved as usize].sub_category_id == ITEM_GROUP_MIN
                    && state.py.inventory[resolved as usize].items_count > 1
                    && !inventory_can_carry_item_count(state.py.inventory[slot as usize])
                {
                    terminal::print_message(Some("You will have to drop something first."));
                    resolved = -1;
                }
            }
        });
    }

    if resolved == -1 {
        return;
    }

    with_state_mut(|state| {
        state.game.player_free_turn = false;
    });

    // Snapshot / mutate without nesting helpers that themselves borrow state.
    let saved_item = with_state_mut(|state| {
        let mut saved = state.py.inventory[resolved as usize];
        state.game.screen.wear_high_id -= 1;

        if saved.items_count > 1 && saved.sub_category_id <= ITEM_SINGLE_STACK_MAX {
            saved.items_count = 1;
            state.game.screen.wear_high_id += 1;
        }

        state.py.pack.weight +=
            (i32::from(saved.weight) * i32::from(saved.items_count)) as i16;
        saved
    });
    inventory_destroy_item(resolved);

    let old_item = with_state(|state| state.py.inventory[slot as usize]);
    if old_item.category_id != TV_NOTHING {
        let uniq = with_state(|state| state.py.pack.unique_items);
        let id = inventory_carry_item(old_item);
        with_state_mut(|state| {
            if state.py.pack.unique_items != uniq {
                state.game.screen.wear_high_id += 1;
            }
        });
        player_take_off(slot, id);
    }

    with_state_mut(|state| {
        state.py.inventory[slot as usize] = saved_item;
        state.py.equipment_count += 1;
    });
    player_adjust_bonuses_for_item(saved_item, 1);

    let text = if slot == PlayerEquipment::Wield as i32 {
        "You are wielding"
    } else if slot == PlayerEquipment::Light as i32 {
        "Your light source is"
    } else {
        "You are wearing"
    };

    // item_description borrows game state; must not run under with_state_mut.
    let mut description = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
    item_description(&mut description, saved_item, true);

    let letter_id = with_state(|state| {
        let mut item_id_to_take_off = PlayerEquipment::Wield as i32;
        let mut letter_id = 0i32;
        while item_id_to_take_off != slot {
            if state.py.inventory[item_id_to_take_off as usize].category_id != TV_NOTHING {
                letter_id += 1;
            }
            item_id_to_take_off += 1;
        }
        letter_id
    });

    let msg = format!(
        "{} {} ({})",
        text,
        c_string_to_str(&description),
        (b'a' + letter_id as u8) as char
    );
    terminal::print_message(Some(&msg));

    if slot == PlayerEquipment::Wield as i32 {
        with_state_mut(|state| {
            state.py.weapon_is_heavy = false;
        });
    }

    player_strength();

    with_state_mut(|state| {
        if inventory_item_is_cursed(state.py.inventory[slot as usize]) {
            terminal::print_message(Some("Oops! It feels deathly cold!"));
            item_append_to_inscription(&mut state.py.inventory[slot as usize], ID_DAMD);
            state.py.inventory[slot as usize].cost = -1;
        }
    });
}

fn execute_drop_item_command(item_id: i32, which: &[u8], prompt: &str) {
    let which_ch = which.first().copied().unwrap_or(0);
    let mut confirmed = -1i32;
    let mut resolved = item_id;

    with_state_mut(|state| {
        if state.py.inventory[resolved as usize].items_count > 1 {
            let mut description = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
            item_description(
                &mut description,
                state.py.inventory[resolved as usize],
                true,
            );
            let len = description
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(description.len());
            if len > 0 {
                description[len - 1] = b'?';
            }
            let msg = format!("Drop all {}", c_string_to_str(&description));
            confirmed = terminal::get_input_confirmation_with_abort(0, &msg);
            if confirmed == -1 {
                resolved = -1;
            }
        } else if which_ch.is_ascii_uppercase() && !verify_action(prompt, resolved) {
            resolved = -1;
        }
    });

    if resolved >= 0 {
        with_state_mut(|state| {
            state.game.player_free_turn = false;
        });
        inventory_drop_item(resolved, confirmed == 1);
        player_strength();
    }

    with_state_mut(|state| {
        if state.py.pack.unique_items == 0 && state.py.equipment_count == 0 {
            state.py.pack.weight = 0;
        }
    });
}

fn select_item_commands(command: &mut u8, which: &mut u8, mut selecting: bool) -> bool {
    while selecting && with_state_mut(|state| state.game.player_free_turn) {
        let (from_line, to_line, prompt, swap) = with_state_mut(|state| {
            let swap: &str;
            let (from_line, to_line, prompt): (i32, i32, &str);

            if *command == b'w' {
                from_line = state.game.screen.wear_low_id;
                to_line = state.game.screen.wear_high_id;
                prompt = "Wear/Wield";
                swap = "";
            } else {
                from_line = 0;
                if *command == b'd' {
                    to_line = i32::from(state.py.pack.unique_items) - 1;
                    prompt = "Drop";
                    swap = if state.py.equipment_count > 0 {
                        ", / for Equip"
                    } else {
                        ""
                    };
                } else {
                    to_line = i32::from(state.py.equipment_count) - 1;
                    if *command == b't' {
                        prompt = "Take off";
                        swap = "";
                    } else {
                        prompt = "Throw off";
                        swap = if state.py.pack.unique_items > 0 {
                            ", / for Inven"
                        } else {
                            ""
                        };
                    }
                }
            }
            (from_line, to_line, prompt, swap)
        });

        if from_line > to_line {
            selecting = false;
            continue;
        }

        let heading = build_command_heading_state(from_line, to_line, swap, *command, prompt);
        if !terminal::get_command(&heading, which) {
            *which = ESCAPE;
            selecting = false;
            continue;
        }

        if *which == b' ' || *which == b'*' {
            change_screen_for_command(*command);
            if *which == b' ' {
                selecting = false;
            }
            continue;
        }

        if *which == b'/' && !swap.is_empty() {
            if *command == b'd' {
                *command = b'r';
            } else {
                *command = b'd';
            }
            flip_inventory_equipment_screens();
            continue;
        }

        let matched = inventory_get_item_matching_inscription(*which, *command, from_line, to_line);
        if matched < from_line || matched > to_line {
            terminal::terminal_bell_sound();
            continue;
        }

        if *command == b'r' || *command == b't' {
            selecting = execute_remove_item_command(selecting, matched, command, &[*which], prompt);
        } else if *command == b'w' {
            execute_wear_item_command(matched, &[*which], prompt);
        } else {
            execute_drop_item_command(matched, &[*which], prompt);
            selecting = false;
        }

        let stop = with_state_mut(|state| {
            !state.game.player_free_turn && state.game.screen.current_screen_id == Screen::Blank
        });
        if stop {
            selecting = false;
        }
    }

    selecting
}

fn inventory_display_appropriate_header() {
    with_state_mut(|state| {
        let msg = if state.game.screen.current_screen_id == Screen::Inventory {
            let weight_quotient = state.py.pack.weight / 10;
            let weight_remainder = state.py.pack.weight % 10;

            if !state.options.show_inventory_weights || state.py.pack.unique_items == 0 {
                format!(
                    "You are carrying {weight_quotient}.{weight_remainder} pounds. In your pack there is {}",
                    if state.py.pack.unique_items == 0 {
                        "nothing."
                    } else {
                        "-"
                    }
                )
            } else {
                let cap = player_carrying_load_limit();
                let capacity_quotient = cap / 10;
                let capacity_remainder = cap % 10;
                format!(
                    "You are carrying {weight_quotient}.{weight_remainder} pounds. Your capacity is {capacity_quotient}.{capacity_remainder} pounds. In your pack is -"
                )
            }
        } else if state.game.screen.current_screen_id == Screen::Wear {
            if state.game.screen.wear_high_id < state.game.screen.wear_low_id {
                "You have nothing you could wield.".to_string()
            } else {
                "You could wield -".to_string()
            }
        } else if state.game.screen.current_screen_id == Screen::Equipment {
            if state.py.equipment_count == 0 {
                "You are not using anything.".to_string()
            } else {
                "You are using -".to_string()
            }
        } else {
            "Allowed commands:".to_string()
        };

        terminal::put_string_clear_to_eol(&msg, Coord { y: 0, x: 0 });
        terminal::erase_line(Coord {
            y: state.game.screen.screen_bottom_pos,
            x: state.game.screen.screen_left_pos,
        });
    });
}

fn ui_command_display_inventory() {
    let empty = with_state_mut(|state| state.py.pack.unique_items == 0);
    if empty {
        terminal::print_message(Some("You are not carrying anything."));
    } else {
        ui_command_switch_screen(Screen::Inventory);
    }
}

fn ui_command_display_equipment() {
    let empty = with_state_mut(|state| state.py.equipment_count == 0);
    if empty {
        terminal::print_message(Some("You are not using any equipment."));
    } else {
        ui_command_switch_screen(Screen::Equipment);
    }
}

/// C++ ui_inventory.cpp:1005-1096 — `inventoryExecuteCommand`.
pub fn inventory_execute_command(mut command: u8) {
    with_state_mut(|state| {
        state.game.player_free_turn = true;
    });

    terminal::terminal_save_screen();

    let recover_screen = command == b' ';
    request_and_show_inventory_screen(recover_screen);

    loop {
        if command.is_ascii_uppercase() {
            command = command.to_ascii_lowercase();
        }

        let mut selecting = false;
        match command {
            b'i' => ui_command_display_inventory(),
            b'e' => ui_command_display_equipment(),
            b't' => selecting = ui_command_inventory_take_off_item(selecting),
            b'd' => selecting = ui_command_inventory_drop_item(&mut command, selecting),
            b'w' => selecting = ui_command_inventory_wear_wield_item(selecting),
            b'x' => ui_command_inventory_unwield_item(),
            b'?' => ui_command_switch_screen(Screen::Help),
            b' ' => {}
            _ => {
                terminal::terminal_bell_sound();
            }
        }

        with_state_mut(|state| {
            state.game.doing_inventory_command = 0;
        });

        let mut which = b'z';
        selecting = select_item_commands(&mut command, &mut which, selecting);

        let exit = with_state_mut(|state| {
            which == ESCAPE || state.game.screen.current_screen_id == Screen::Blank
        });
        if exit {
            command = ESCAPE;
        } else if !with_state_mut(|state| state.game.player_free_turn) {
            with_state_mut(|state| {
                if selecting {
                    state.game.doing_inventory_command = command;
                } else {
                    state.game.doing_inventory_command = b' ';
                }
                state.screen_has_changed = false;
            });
            terminal::print_message(CNIL);
            command = ESCAPE;
        } else {
            inventory_display_appropriate_header();
            terminal::put_string(
                "e/i/t/w/x/d/?/ESC:",
                Coord {
                    y: with_state_mut(|s| s.game.screen.screen_bottom_pos),
                    x: 60,
                },
            );
            command = terminal::get_key_input();
            with_state_mut(|state| {
                terminal::erase_line(Coord {
                    y: state.game.screen.screen_bottom_pos,
                    x: state.game.screen.screen_left_pos,
                });
            });
        }

        if command == ESCAPE {
            break;
        }
    }

    if with_state_mut(|state| state.game.screen.current_screen_id != Screen::Blank) {
        terminal::terminal_restore_screen();
    }

    player_recalculate_bonuses();
}

/// C++ ui_inventory.cpp:1098-1102 — `PackMenu`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PackMenu {
    CloseMenu,
    Equipment,
    Inventory,
}

/// C++ ui_inventory.cpp:1106-1149 — `inventorySwitchPackMenu`.
fn inventory_switch_pack_menu(
    prompt: &mut str,
    menu: &mut PackMenu,
    menu_active: bool,
    item_id_end: &mut i32,
) -> bool {
    if *menu == PackMenu::Inventory {
        let equipment_count = with_state_mut(|s| s.py.equipment_count);
        if equipment_count == 0 {
            terminal::put_string_clear_to_eol(
                "But you're not using anything -more-",
                Coord { y: 0, x: 0 },
            );
            let _ = terminal::get_key_input();
        } else {
            *menu = PackMenu::Equipment;
            if menu_active {
                *item_id_end = i32::from(equipment_count);
                let unique = with_state_mut(|s| i32::from(s.py.pack.unique_items));
                let mut end = *item_id_end;
                while end < unique {
                    end += 1;
                    terminal::erase_line(Coord { y: end, x: 0 });
                }
            }
            *item_id_end = i32::from(equipment_count) - 1;
            terminal::put_string_clear_to_eol(prompt, Coord { y: 0, x: 0 });
            return true;
        }
        terminal::put_string_clear_to_eol(prompt, Coord { y: 0, x: 0 });
        return false;
    }

    let unique_items = with_state_mut(|s| s.py.pack.unique_items);
    if unique_items == 0 {
        terminal::put_string_clear_to_eol(
            "But you're not carrying anything -more-",
            Coord { y: 0, x: 0 },
        );
        let _ = terminal::get_key_input();
        return false;
    }

    *menu = PackMenu::Inventory;
    if menu_active {
        *item_id_end = i32::from(unique_items);
        let equipment_count = with_state_mut(|s| i32::from(s.py.equipment_count));
        let mut end = *item_id_end;
        while end < equipment_count {
            end += 1;
            terminal::erase_line(Coord { y: end, x: 0 });
        }
    }
    *item_id_end = i32::from(unique_items) - 1;
    true
}

/// C++ ui_inventory.cpp:1152-1304 — `inventoryGetInputForItemId`.
// `done = true;` immediately preceding `break;` mirrors ui_inventory.cpp:1273-1277
// verbatim; the assignment is dead but preserved for fidelity.
#[allow(unused_assignments)]
pub fn inventory_get_input_for_item_id(
    command_key_id: &mut i32,
    prompt: &str,
    mut item_id_start: i32,
    mut item_id_end: i32,
    mask: Option<&[u8]>,
    message: Option<&str>,
) -> bool {
    let mut menu = PackMenu::Inventory;
    let mut pack_full = false;

    if item_id_end > PlayerEquipment::Wield as i32 {
        pack_full = true;
        let (unique, equip) = with_state_mut(|s| (s.py.pack.unique_items, s.py.equipment_count));
        if unique == 0 {
            menu = PackMenu::Equipment;
            item_id_end = i32::from(equip) - 1;
        } else {
            item_id_end = i32::from(unique) - 1;
        }
    }

    let (unique, equip) = with_state_mut(|s| (s.py.pack.unique_items, s.py.equipment_count));
    if unique < 1 && (!pack_full || equip < 1) {
        terminal::put_string_clear_to_eol("You are not carrying anything.", Coord { y: 0, x: 0 });
        return false;
    }

    *command_key_id = 0;
    let mut item_found = false;
    let mut menu_active = false;

    while menu != PackMenu::CloseMenu {
        if menu_active {
            if menu == PackMenu::Inventory {
                let _ = display_inventory_items(item_id_start, item_id_end, false, 80, mask);
            } else {
                let _ = display_equipment(false, 80);
            }
        }

        let description = if pack_full {
            format!(
                "({}: {}-{},{} {} / for {}, or ESC) {}",
                if menu == PackMenu::Inventory {
                    "Inven"
                } else {
                    "Equip"
                },
                (item_id_start as u8 + b'a') as char,
                (item_id_end as u8 + b'a') as char,
                if menu == PackMenu::Inventory {
                    " 0-9,"
                } else {
                    ""
                },
                if menu_active { "" } else { " * to see," },
                if menu == PackMenu::Inventory {
                    "Equip"
                } else {
                    "Inven"
                },
                prompt
            )
        } else {
            format!(
                "(Items {}-{},{} {} ESC to exit) {}",
                (item_id_start as u8 + b'a') as char,
                (item_id_end as u8 + b'a') as char,
                if menu == PackMenu::Inventory {
                    " 0-9,"
                } else {
                    ""
                },
                if menu_active {
                    ""
                } else {
                    " * for inventory list,"
                },
                prompt
            )
        };

        terminal::put_string_clear_to_eol(&description, Coord { y: 0, x: 0 });

        let mut done = false;
        while !done {
            let which = terminal::get_key_input();

            match which {
                ESCAPE => {
                    menu = PackMenu::CloseMenu;
                    done = true;
                    with_state_mut(|state| {
                        state.game.player_free_turn = true;
                    });
                }
                b'/' => {
                    let mut prompt_buf = description.clone();
                    done = inventory_switch_pack_menu(
                        &mut prompt_buf,
                        &mut menu,
                        menu_active,
                        &mut item_id_end,
                    );
                }
                b'*' if !menu_active => {
                    done = true;
                    terminal::terminal_save_screen();
                    menu_active = true;
                }
                _ => {
                    // C++ declares `int commandKeyId;` uninitialized; every branch
                    // below assigns it before use (ui_inventory.cpp:1229-1251).
                    let mut key_id: i32;
                    if which.is_ascii_digit() && menu != PackMenu::Equipment {
                        let mut m = item_id_start;
                        while m < i32::from(PlayerEquipment::Wield as u8) {
                            let ins0 =
                                with_state_mut(|s| s.py.inventory[m as usize].inscription[0]);
                            let ins1 =
                                with_state_mut(|s| s.py.inventory[m as usize].inscription[1]);
                            if ins0 == which as c_char && ins1 == 0 {
                                break;
                            }
                            m += 1;
                        }
                        if m < i32::from(PlayerEquipment::Wield as u8) {
                            key_id = m;
                        } else {
                            key_id = -1;
                        }
                    } else if which.is_ascii_uppercase() {
                        key_id = i32::from(which.wrapping_sub(b'A'));
                    } else {
                        key_id = i32::from(which.wrapping_sub(b'a'));
                    }

                    let mask_ok = mask.map_or(true, |m| {
                        key_id as usize >= m.len() || m[key_id as usize] != 0
                    });

                    if key_id >= item_id_start && key_id <= item_id_end && mask_ok {
                        if menu == PackMenu::Equipment {
                            item_id_start = 21;
                            item_id_end = key_id;
                            loop {
                                item_id_start += 1;
                                while with_state_mut(|s| {
                                    s.py.inventory[item_id_start as usize].category_id == TV_NOTHING
                                }) {
                                    item_id_start += 1;
                                }
                                item_id_end -= 1;
                                if item_id_end < 0 {
                                    break;
                                }
                            }
                            key_id = item_id_start;
                        }

                        if which.is_ascii_uppercase() && !verify_action("Try", key_id) {
                            menu = PackMenu::CloseMenu;
                            done = true;
                            with_state_mut(|state| {
                                state.game.player_free_turn = true;
                            });
                            break;
                        }

                        menu = PackMenu::CloseMenu;
                        done = true;
                        *command_key_id = key_id;
                        item_found = true;
                    } else if let Some(msg) = message {
                        terminal::print_message(Some(msg));
                        done = true;
                    } else {
                        terminal::terminal_bell_sound();
                    }
                }
            }
        }
    }

    if menu_active {
        terminal::terminal_restore_screen();
    }

    terminal::message_line_clear();
    item_found
}
