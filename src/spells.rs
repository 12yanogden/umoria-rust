//! Spell selection, detection, and utility spells

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MagicSpellFlags {
    MagicMissile = 0,
    Lightning,
    PoisonGas,
    Acid,
    Frost,
    Fire,
    HolyOrb,
}

use crate::config::dungeon::objects::{MAX_TRAPS, OBJ_CLOSED_DOOR, OBJ_MUSH, OBJ_SCARE_MON};
use crate::config::monsters::defense::{
    CD_ACID, CD_EVIL, CD_FIRE, CD_FROST, CD_LIGHT, CD_NO_SLEEP, CD_POISON, CD_STONE, CD_UNDEAD,
};
use crate::config::monsters::move_flags::{
    CM_ATTACK_ONLY, CM_PHASE, CM_TREASURE, CM_TR_SHIFT, CM_WIN,
};
use crate::config::monsters::spells as monster_spells;
use crate::config::monsters::{self, MON_MAX_SIGHT, MON_MIN_INDEX_ID};
use crate::config::player::status::PY_BLIND;
use crate::config::spells::{
    NAME_OFFSET_PRAYERS, NAME_OFFSET_SPELLS, SPELL_TYPE_MAGE, SPELL_TYPE_PRIEST,
};
use crate::config::treasure::chests::{CH_LOCKED, CH_TRAPPED};
use crate::config::treasure::OBJECT_BOLTS_MAX_RANGE;
use crate::data_creatures::CREATURES_LIST;
use crate::data_player::{CLASSES, MAGIC_SPELLS};
use crate::dice::{dice_roll, Dice};
use crate::dungeon::{
    cave_tile_visible, coord_distance_between, coord_in_bounds, dungeon_delete_monster,
    dungeon_delete_object, dungeon_light_room, dungeon_lite_spot, dungeon_move_creature_record,
    dungeon_place_random_object_at, dungeon_remove_monster_from_level, dungeon_set_trap,
    trap_change_visibility,
};
use crate::dungeon_los::los;
use crate::dungeon_tile::{
    MAX_CAVE_FLOOR, MAX_OPEN_SPACE, MIN_CAVE_WALL, MIN_CLOSED_SPACE, TILE_BLOCKED_FLOOR,
    TILE_BOUNDARY_WALL, TILE_CORR_FLOOR, TILE_DARK_FLOOR, TILE_GRANITE_WALL, TILE_LIGHT_FLOOR,
    TILE_MAGMA_WALL, TILE_QUARTZ_WALL,
};
use crate::game::{random_number, random_number_state, with_state, with_state_mut, State};
use crate::game_objects::popt;
use crate::helpers::get_and_clear_first_bit;
use crate::identification::{
    item_description, item_identification_clear_empty, item_identify, spell_item_identified,
    spell_item_identify_and_remove_random_inscription,
    spell_item_identify_and_remove_random_inscription_for_state, spell_item_remove_identification,
    SpecialNameIds,
};
use crate::inventory::{
    inventory_destroy_item, inventory_find_range, inventory_item_copy_to,
    set_acid_destroyable_items, set_fire_destroyable_items, set_frost_destroyable_items,
    set_lightning_destroyable_items, set_null, Inventory, PlayerEquipment, PLAYER_INVENTORY_SIZE,
};
use crate::monster::{
    monster_death, monster_multiply, monster_name_description, monster_take_hit,
    monster_update_visibility, print_monster_action_text, update_monsters, MON_MAX_LEVELS,
};
use crate::monster_manager::monster_place_new;
use crate::player::{
    player_calculate_allowed_spells_count, player_gain_mana, player_recalculate_bonuses,
    player_worn_item_is_cursed, player_worn_item_remove_curse, PlayerAttr,
};
use crate::player_move::player_move_position;
use crate::player_stats::player_calculate_hit_points;
use crate::player_tunnel::player_tunnel_wall;
use crate::treasure::{
    TV_CHEST, TV_CLOSED_DOOR, TV_DOWN_STAIR, TV_GOLD, TV_INVIS_TRAP, TV_MAX_OBJECT, TV_MAX_VISIBLE,
    TV_MIN_VISIBLE, TV_OPEN_DOOR, TV_RUBBLE, TV_SECRET_DOOR, TV_STAFF, TV_UP_STAIR, TV_VIS_TRAP,
    TV_WAND,
};
use crate::types::{Coord_t, Vtype_t, MORIA_OBJ_DESC_SIZE_LEN};
use crate::ui::{
    coord_inside_panel, coord_inside_panel_bounds, display_character_experience,
    display_spells_list, draw_dungeon_panel, dungeon_reset_view,
    print_character_current_hit_points, print_character_level, print_character_title,
};
use crate::ui_inventory::{inventory_get_input_for_item_id, player_item_wearing_description};
use crate::ui_io::terminal::{self, panel_put_tile, put_qio, Coord};

const SCREEN_HEIGHT: i32 = 22;
const SCREEN_WIDTH: i32 = 66;

fn wis_int_adj_from_used(value: u8) -> i32 {
    let value = i32::from(value);
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

/// 295
pub(crate) fn spell_chance_of_success_for_state(state: &State, spell_id: i32) -> i32 {
    let class_id = state.py.misc.class_id as usize;
    let spell = &MAGIC_SPELLS[class_id - 1][spell_id as usize];

    let mut chance = i32::from(spell.failure_chance)
        - 3 * (i32::from(state.py.misc.level) - i32::from(spell.level_required));

    let stat = if CLASSES[class_id].class_to_use_mage_spells == SPELL_TYPE_MAGE {
        PlayerAttr::A_INT
    } else {
        PlayerAttr::A_WIS
    };

    chance -= 3 * (wis_int_adj_from_used(state.py.stats.used[stat as usize]) - 1);

    if i32::from(spell.mana_required) > i32::from(state.py.misc.current_mana) {
        chance += 5 * (i32::from(spell.mana_required) - i32::from(state.py.misc.current_mana));
    }

    chance.clamp(5, 95)
}

/// 295 (needed by `display_spells_list` during learning)
pub fn spell_chance_of_success(spell_id: i32) -> i32 {
    with_state(|state| spell_chance_of_success_for_state(state, spell_id))
}

/// Build castable spell ids from a spellbook — castSpellGetId lines 109–130
pub fn build_castable_spell_list(item_id: i32) -> Option<(i32, Vec<i32>)> {
    with_state(|state| {
        let mut flags = state.py.inventory[item_id as usize].flags;
        let first_spell = get_and_clear_first_bit(&mut flags);
        flags = state.py.inventory[item_id as usize].flags & state.py.flags.spells_learnt;

        let class_id = state.py.misc.class_id as usize;
        let spells = &MAGIC_SPELLS[class_id - 1];
        let mut spell_list = Vec::new();

        while flags != 0 {
            let pos = get_and_clear_first_bit(&mut flags);
            if u16::from(spells[pos as usize].level_required) <= state.py.misc.level {
                spell_list.push(pos);
            }
        }

        if spell_list.is_empty() {
            None
        } else {
            Some((first_spell, spell_list))
        }
    })
}

/// spell-menu selection
fn spell_get_id(
    spell_ids: &[i32],
    number_of_choices: i32,
    spell_id: &mut i32,
    spell_chance: &mut i32,
    prompt: &str,
    first_spell: i32,
) -> bool {
    *spell_id = -1;

    let offset = with_state(|state| {
        if CLASSES[state.py.misc.class_id as usize].class_to_use_mage_spells == SPELL_TYPE_MAGE {
            i32::from(NAME_OFFSET_SPELLS)
        } else {
            i32::from(NAME_OFFSET_PRAYERS)
        }
    });

    let menu_prompt = format!(
        "(Spells {}-{}, *=List, <ESCAPE>=exit) {}",
        (spell_ids[0] + b'a' as i32 - first_spell) as u8 as char,
        (spell_ids[(number_of_choices - 1) as usize] + b'a' as i32 - first_spell) as u8 as char,
        prompt
    );

    let mut spell_found = false;
    let mut redraw = false;

    while !spell_found {
        let mut spell_choice = 0u8;
        if !terminal::get_menu_item_id(&menu_prompt, &mut spell_choice) {
            break;
        }

        if spell_choice.is_ascii_uppercase() {
            *spell_id = i32::from(spell_choice - b'A') + first_spell;

            let mut test_spell_id = 0;
            while test_spell_id < number_of_choices {
                if *spell_id == spell_ids[test_spell_id as usize] {
                    break;
                }
                test_spell_id += 1;
            }

            if test_spell_id == number_of_choices {
                *spell_id = -2;
            } else {
                let spell = &MAGIC_SPELLS[with_state(|s| s.py.misc.class_id as usize) - 1]
                    [*spell_id as usize];
                let name = crate::data_player::SPELL_NAMES[(*spell_id + offset) as usize];
                let fail = spell_chance_of_success(*spell_id);
                let confirm_msg = format!(
                    "Cast {} ({} mana, {}% fail)?",
                    name, spell.mana_required, fail
                );
                if terminal::get_input_confirmation(&confirm_msg) {
                    spell_found = true;
                } else {
                    *spell_id = -1;
                }
            }
        } else if spell_choice.is_ascii_lowercase() {
            *spell_id = i32::from(spell_choice - b'a') + first_spell;

            let mut test_spell_id = 0;
            while test_spell_id < number_of_choices {
                if *spell_id == spell_ids[test_spell_id as usize] {
                    break;
                }
                test_spell_id += 1;
            }

            if test_spell_id == number_of_choices {
                *spell_id = -2;
            } else {
                spell_found = true;
            }
        } else if spell_choice == b'*' {
            if !redraw {
                terminal::terminal_save_screen();
                redraw = true;
                display_spells_list(spell_ids, number_of_choices, false, first_spell);
            }
        } else if spell_choice.is_ascii_alphabetic() {
            *spell_id = -2;
        } else {
            *spell_id = -1;
            let _ = terminal::terminal_bell_sound();
        }

        if *spell_id == -2 {
            let kind = if offset == i32::from(NAME_OFFSET_SPELLS) {
                "spell"
            } else {
                "prayer"
            };
            terminal::print_message(Some(&format!("You don't know that {kind}.")));
        }
    }

    if redraw {
        terminal::terminal_restore_screen();
    }

    terminal::message_line_clear();

    if spell_found {
        *spell_chance = spell_chance_of_success(*spell_id);
    }

    spell_found
}

/// 150
pub fn cast_spell_get_id(
    prompt: &str,
    item_id: i32,
    spell_id: &mut i32,
    spell_chance: &mut i32,
) -> i32 {
    let Some((first_spell, spell_list)) = build_castable_spell_list(item_id) else {
        return -1;
    };

    let spell_count = spell_list.len() as i32;
    let mut result = 0;

    if spell_get_id(
        &spell_list,
        spell_count,
        spell_id,
        spell_chance,
        prompt,
        first_spell,
    ) {
        result = 1;
    }

    if result != 0 {
        let needs_confirm = with_state(|state| {
            let class_id = state.py.misc.class_id as usize;
            i16::from(MAGIC_SPELLS[class_id - 1][*spell_id as usize].mana_required)
                > state.py.misc.current_mana
        });
        if needs_confirm {
            let confirmed = if with_state(|state| {
                CLASSES[state.py.misc.class_id as usize].class_to_use_mage_spells == SPELL_TYPE_MAGE
            }) {
                terminal::get_input_confirmation(
                    "You summon your limited strength to cast this one! Confirm?",
                )
            } else {
                terminal::get_input_confirmation(
                    "The gods may think you presumptuous for this! Confirm?",
                )
            };
            if !confirmed {
                result = 0;
            }
        }
    }

    result
}

/// 176
#[must_use]
pub fn spell_detect_treasure_within_vicinity() -> bool {
    let (top, bottom, left, right) = with_state(|state| {
        (
            state.dg.panel.top,
            state.dg.panel.bottom,
            state.dg.panel.left,
            state.dg.panel.right,
        )
    });

    let mut detected = false;

    for coord_y in top..=bottom {
        for coord_x in left..=right {
            let coord = Coord_t {
                y: coord_y,
                x: coord_x,
            };
            let (treasure_id, category_id, visible) = with_state(|state| {
                let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
                let treasure_id = tile.treasure_id;
                let category_id = if treasure_id != 0 {
                    state.game.treasure.list[treasure_id as usize].category_id
                } else {
                    0
                };
                (treasure_id, category_id, cave_tile_visible(coord))
            });

            if treasure_id != 0 && category_id == TV_GOLD && !visible {
                with_state_mut(|state| {
                    state.dg.floor[coord.y as usize][coord.x as usize].field_mark = true;
                });
                dungeon_lite_spot(coord);
                detected = true;
            }
        }
    }

    detected
}

/// 197
#[must_use]
pub fn spell_detect_objects_within_vicinity() -> bool {
    let (top, bottom, left, right) = with_state(|state| {
        (
            state.dg.panel.top,
            state.dg.panel.bottom,
            state.dg.panel.left,
            state.dg.panel.right,
        )
    });

    let mut detected = false;

    for coord_y in top..=bottom {
        for coord_x in left..=right {
            let coord = Coord_t {
                y: coord_y,
                x: coord_x,
            };
            let (treasure_id, category_id, visible) = with_state(|state| {
                let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
                let treasure_id = tile.treasure_id;
                let category_id = if treasure_id != 0 {
                    state.game.treasure.list[treasure_id as usize].category_id
                } else {
                    0
                };
                (treasure_id, category_id, cave_tile_visible(coord))
            });

            if treasure_id != 0 && category_id < TV_MAX_OBJECT && !visible {
                with_state_mut(|state| {
                    state.dg.floor[coord.y as usize][coord.x as usize].field_mark = true;
                });
                dungeon_lite_spot(coord);
                detected = true;
            }
        }
    }

    detected
}

/// 225
#[must_use]
pub fn spell_detect_traps_within_vicinity() -> bool {
    let (top, bottom, left, right) = with_state(|state| {
        (
            state.dg.panel.top,
            state.dg.panel.bottom,
            state.dg.panel.left,
            state.dg.panel.right,
        )
    });

    let mut detected = false;

    for coord_y in top..=bottom {
        for coord_x in left..=right {
            let coord = Coord_t {
                y: coord_y,
                x: coord_x,
            };

            let treasure_id =
                with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].treasure_id);
            if treasure_id == 0 {
                continue;
            }

            let category_id =
                with_state(|state| state.game.treasure.list[treasure_id as usize].category_id);

            if category_id == TV_INVIS_TRAP {
                with_state_mut(|state| {
                    state.dg.floor[coord.y as usize][coord.x as usize].field_mark = true;
                });
                trap_change_visibility(coord);
                detected = true;
            } else if category_id == TV_CHEST {
                with_state_mut(|state| {
                    let item = &mut state.game.treasure.list[treasure_id as usize];
                    spell_item_identify_and_remove_random_inscription(item);
                });
            }
        }
    }

    detected
}

/// Detect secret doors within the player vicinity.
#[must_use]
pub fn spell_detect_secret_doors_within_vicinity() -> bool {
    let (top, bottom, left, right) = with_state(|state| {
        (
            state.dg.panel.top,
            state.dg.panel.bottom,
            state.dg.panel.left,
            state.dg.panel.right,
        )
    });

    let mut detected = false;

    for coord_y in top..=bottom {
        for coord_x in left..=right {
            let coord = Coord_t {
                y: coord_y,
                x: coord_x,
            };

            let treasure_id =
                with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].treasure_id);
            if treasure_id == 0 {
                continue;
            }

            let (category_id, field_mark) = with_state(|state| {
                let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
                (
                    state.game.treasure.list[treasure_id as usize].category_id,
                    tile.field_mark,
                )
            });

            if category_id == TV_SECRET_DOOR {
                with_state_mut(|state| {
                    state.dg.floor[coord.y as usize][coord.x as usize].field_mark = true;
                });
                trap_change_visibility(coord);
                detected = true;
            } else if (category_id == TV_UP_STAIR || category_id == TV_DOWN_STAIR) && !field_mark {
                with_state_mut(|state| {
                    state.dg.floor[coord.y as usize][coord.x as usize].field_mark = true;
                });
                dungeon_lite_spot(coord);
                detected = true;
            }
        }
    }

    detected
}

/// 286
#[must_use]
pub fn spell_detect_invisible_creatures_within_vicinity() -> bool {
    let mut detected = false;

    let (start_id, min_id) = with_state(|state| {
        (
            i32::from(state.next_free_monster_id - 1),
            i32::from(MON_MIN_INDEX_ID),
        )
    });

    for id in (min_id..=start_id).rev() {
        let should_light = with_state(|state| {
            let monster = &state.monsters[id as usize];
            coord_inside_panel(monster.pos)
                && (CREATURES_LIST[monster.creature_id as usize].movement
                    & monsters::move_flags::CM_INVISIBLE)
                    != 0
        });

        if should_light {
            with_state_mut(|state| {
                state.monsters[id as usize].lit = true;
            });
            let (sprite, pos) = with_state(|state| {
                let monster = &state.monsters[id as usize];
                (
                    CREATURES_LIST[monster.creature_id as usize].sprite,
                    monster.pos,
                )
            });
            terminal::panel_put_tile(sprite, Coord { y: pos.y, x: pos.x });
            detected = true;
        }
    }

    if detected {
        terminal::print_message(Some("You sense the presence of invisible creatures!"));
        terminal::print_message(None);
        update_monsters(false);
    }

    detected
}

fn dungeon_light_area_around_floor_tile(coord: Coord_t) {
    for spot_y in coord.y - 1..=coord.y + 1 {
        for spot_x in coord.x - 1..=coord.x + 1 {
            let spot = Coord_t {
                y: spot_y,
                x: spot_x,
            };
            with_state_mut(|state| {
                let tile = &mut state.dg.floor[spot.y as usize][spot.x as usize];
                if tile.feature_id >= MIN_CAVE_WALL {
                    tile.permanent_light = true;
                } else if tile.treasure_id != 0 {
                    let category_id =
                        state.game.treasure.list[tile.treasure_id as usize].category_id;
                    if (TV_MIN_VISIBLE..=TV_MAX_VISIBLE).contains(&category_id) {
                        tile.field_mark = true;
                    }
                }
            });
        }
    }
}

/// 402
pub fn spell_map_current_area() {
    let (panel_top, panel_bottom, panel_left, panel_right) = with_state(|state| {
        (
            state.dg.panel.top,
            state.dg.panel.bottom,
            state.dg.panel.left,
            state.dg.panel.right,
        )
    });

    let row_min = panel_top - random_number(10);
    let row_max = panel_bottom + random_number(10);
    let col_min = panel_left - random_number(20);
    let col_max = panel_right + random_number(20);

    for coord_y in row_min..=row_max {
        for coord_x in col_min..=col_max {
            let coord = Coord_t {
                y: coord_y,
                x: coord_x,
            };
            let in_bounds_floor = with_state(|state| {
                coord_in_bounds(coord)
                    && state.dg.floor[coord.y as usize][coord.x as usize].feature_id
                        <= MAX_CAVE_FLOOR
            });
            if in_bounds_floor {
                dungeon_light_area_around_floor_tile(coord);
            }
        }
    }

    draw_dungeon_panel();
}

/// 429
#[must_use]
pub fn spell_identify_item() -> bool {
    let mut item_id = 0;
    if !inventory_get_input_for_item_id(
        &mut item_id,
        "Item you wish identified?",
        0,
        i32::from(PLAYER_INVENTORY_SIZE),
        None,
        None,
    ) {
        return false;
    }

    item_identify(&mut item_id);

    with_state_mut(|state| {
        let item = &mut state.py.inventory[item_id as usize];
        spell_item_identify_and_remove_random_inscription(item);
    });

    let mut description = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
    let item = with_state(|state| state.py.inventory[item_id as usize]);
    item_description(&mut description, item, true);

    let msg = if item_id >= PlayerEquipment::Wield as i32 {
        player_recalculate_bonuses();
        format!(
            "{}: {}",
            player_item_wearing_description(item_id as u8),
            String::from_utf8_lossy(&description)
                .trim_end_matches('\0')
                .trim_end()
        )
    } else {
        format!(
            "{} {}",
            (item_id as u8 + 97) as char,
            String::from_utf8_lossy(&description)
                .trim_end_matches('\0')
                .trim_end()
        )
    };
    terminal::print_message(Some(&msg));

    true
}

/// 450
#[must_use]
pub fn spell_aggravate_monsters(affect_distance: i32) -> bool {
    let mut aggravated = false;

    let (start_id, min_id) = with_state(|state| {
        (
            i32::from(state.next_free_monster_id - 1),
            i32::from(MON_MIN_INDEX_ID),
        )
    });

    for id in (min_id..=start_id).rev() {
        with_state_mut(|state| {
            let monster = &mut state.monsters[id as usize];
            monster.sleep_count = 0;
            if i32::from(monster.distance_from_player) <= affect_distance && monster.speed < 2 {
                monster.speed += 1;
                aggravated = true;
            }
        });
    }

    if aggravated {
        terminal::print_message(Some("You hear a sudden stirring in the distance!"));
    }

    aggravated
}

/// 486
#[must_use]
pub fn spell_surround_player_with_traps() -> bool {
    let player_pos = with_state(|state| state.py.pos);

    for coord_y in player_pos.y - 1..=player_pos.y + 1 {
        for coord_x in player_pos.x - 1..=player_pos.x + 1 {
            let coord = Coord_t {
                y: coord_y,
                x: coord_x,
            };

            if coord.y == player_pos.y && coord.x == player_pos.x {
                continue;
            }

            let is_floor = with_state(|state| {
                state.dg.floor[coord.y as usize][coord.x as usize].feature_id <= MAX_CAVE_FLOOR
            });
            if !is_floor {
                continue;
            }

            let had_object = with_state(|state| {
                state.dg.floor[coord.y as usize][coord.x as usize].treasure_id != 0
            });
            if had_object {
                let _ = dungeon_delete_object(coord);
            }

            dungeon_set_trap(coord, random_number(i32::from(MAX_TRAPS)) - 1);

            with_state_mut(|state| {
                let treasure_id = state.dg.floor[coord.y as usize][coord.x as usize].treasure_id;
                state.game.treasure.list[treasure_id as usize].misc_use = 0;
            });

            dungeon_lite_spot(coord);
        }
    }

    true
}

/// 521
#[must_use]
pub fn spell_surround_player_with_doors() -> bool {
    let mut created = false;
    let player_pos = with_state(|state| state.py.pos);

    for coord_y in player_pos.y - 1..=player_pos.y + 1 {
        for coord_x in player_pos.x - 1..=player_pos.x + 1 {
            let coord = Coord_t {
                y: coord_y,
                x: coord_x,
            };

            if coord.y == player_pos.y && coord.x == player_pos.x {
                continue;
            }

            let is_floor = with_state(|state| {
                state.dg.floor[coord.y as usize][coord.x as usize].feature_id <= MAX_CAVE_FLOOR
            });
            if !is_floor {
                continue;
            }

            let had_object = with_state(|state| {
                state.dg.floor[coord.y as usize][coord.x as usize].treasure_id != 0
            });
            if had_object {
                let _ = dungeon_delete_object(coord);
            }

            let free_id = popt();
            with_state_mut(|state| {
                let tile = &mut state.dg.floor[coord.y as usize][coord.x as usize];
                tile.feature_id = TILE_BLOCKED_FLOOR;
                tile.treasure_id = free_id as u8;
                inventory_item_copy_to(
                    OBJ_CLOSED_DOOR as i16,
                    &mut state.game.treasure.list[free_id as usize],
                );
            });
            dungeon_lite_spot(coord);
            created = true;
        }
    }

    created
}

/// 557
#[must_use]
pub fn spell_destroy_adjacent_doors_traps() -> bool {
    let mut destroyed = false;
    let player_pos = with_state(|state| state.py.pos);

    for coord_y in player_pos.y - 1..=player_pos.y + 1 {
        for coord_x in player_pos.x - 1..=player_pos.x + 1 {
            let coord = Coord_t {
                y: coord_y,
                x: coord_x,
            };

            let treasure_id =
                with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].treasure_id);
            if treasure_id == 0 {
                continue;
            }

            let category_id =
                with_state(|state| state.game.treasure.list[treasure_id as usize].category_id);
            let flags = with_state(|state| state.game.treasure.list[treasure_id as usize].flags);

            if ((TV_INVIS_TRAP..=TV_CLOSED_DOOR).contains(&category_id) && category_id != TV_RUBBLE)
                || category_id == TV_SECRET_DOOR
            {
                if dungeon_delete_object(coord) {
                    destroyed = true;
                }
            } else if category_id == TV_CHEST && flags != 0 {
                with_state_mut(|state| {
                    let item = &mut state.game.treasure.list[treasure_id as usize];
                    item.flags &= !(CH_TRAPPED | CH_LOCKED);
                    item.special_name_id = SpecialNameIds::SN_UNLOCKED as u8;
                    spell_item_identify_and_remove_random_inscription_for_state(
                        state,
                        treasure_id as usize,
                    );
                });
                destroyed = true;
                terminal::print_message(Some("You have disarmed the chest."));
            }
        }
    }

    destroyed
}

/// 584
#[must_use]
pub fn spell_detect_monsters() -> bool {
    let mut detected = false;

    let (start_id, min_id) = with_state(|state| {
        (
            i32::from(state.next_free_monster_id - 1),
            i32::from(MON_MIN_INDEX_ID),
        )
    });

    for id in (min_id..=start_id).rev() {
        let should_light = with_state(|state| {
            let monster = &state.monsters[id as usize];
            coord_inside_panel(monster.pos)
                && (CREATURES_LIST[monster.creature_id as usize].movement
                    & monsters::move_flags::CM_INVISIBLE)
                    == 0
        });

        if should_light {
            with_state_mut(|state| {
                state.monsters[id as usize].lit = true;
            });
            let (sprite, pos) = with_state(|state| {
                let monster = &state.monsters[id as usize];
                (
                    CREATURES_LIST[monster.creature_id as usize].sprite,
                    monster.pos,
                )
            });
            terminal::panel_put_tile(sprite, Coord { y: pos.y, x: pos.x });
            detected = true;
        }
    }

    if detected {
        terminal::print_message(Some("You sense the presence of monsters!"));
        terminal::print_message(None);
        update_monsters(false);
    }

    detected
}

/// 1906
#[must_use]
pub fn spell_detect_evil() -> bool {
    let mut detected = false;

    let (start_id, min_id) = with_state(|state| {
        (
            i32::from(state.next_free_monster_id - 1),
            i32::from(MON_MIN_INDEX_ID),
        )
    });

    for id in (min_id..=start_id).rev() {
        let should_light = with_state(|state| {
            let monster = &state.monsters[id as usize];
            coord_inside_panel(monster.pos)
                && (CREATURES_LIST[monster.creature_id as usize].defenses
                    & monsters::defense::CD_EVIL)
                    != 0
        });

        if should_light {
            with_state_mut(|state| {
                state.monsters[id as usize].lit = true;
            });
            let (sprite, pos) = with_state(|state| {
                let monster = &state.monsters[id as usize];
                (
                    CREATURES_LIST[monster.creature_id as usize].sprite,
                    monster.pos,
                )
            });
            terminal::panel_put_tile(sprite, Coord { y: pos.y, x: pos.x });
            detected = true;
        }
    }

    if detected {
        terminal::print_message(Some("You sense the presence of evil!"));
        terminal::print_message(None);
        update_monsters(false);
    }

    detected
}

// 608
fn spell_light_line_touches_monster(monster_id: i32) {
    monster_update_visibility(monster_id);

    let (creature_id, lit, creature_defenses, creature_name) = with_state(|state| {
        let monster = &state.monsters[monster_id as usize];
        let creature = &CREATURES_LIST[monster.creature_id as usize];
        (
            monster.creature_id,
            monster.lit,
            creature.defenses,
            creature.name,
        )
    });

    let name = monster_name_description(creature_name, lit);

    if (creature_defenses & CD_LIGHT) != 0 {
        with_state_mut(|state| {
            if state.monsters[monster_id as usize].lit {
                state.creature_recall[creature_id as usize].defenses |= CD_LIGHT;
            }
        });

        if monster_take_hit(monster_id, dice_roll(Dice { dice: 2, sides: 8 })) >= 0 {
            print_monster_action_text(&name, "shrivels away in the light!");
            display_character_experience();
        } else {
            print_monster_action_text(&name, "cringes from the light!");
        }
    }
}

/// 314
#[must_use]
pub fn spell_light_area(coord: Coord_t) -> bool {
    if with_state(|state| state.py.flags.blind < 1) {
        terminal::print_message(Some("You are surrounded by a white light."));
    }

    let lit = true;

    if with_state(|state| {
        state.dg.floor[coord.y as usize][coord.x as usize].perma_lit_room
            && state.dg.current_level > 0
    }) {
        dungeon_light_room(coord);
    }

    for spot_y in coord.y - 1..=coord.y + 1 {
        for spot_x in coord.x - 1..=coord.x + 1 {
            let spot = Coord_t {
                y: spot_y,
                x: spot_x,
            };
            with_state_mut(|state| {
                state.dg.floor[spot_y as usize][spot_x as usize].permanent_light = true;
            });
            dungeon_lite_spot(spot);
        }
    }

    lit
}

/// 365
#[must_use]
pub fn spell_darken_area(coord: Coord_t) -> bool {
    let mut darkened = false;

    if with_state(|state| {
        state.dg.floor[coord.y as usize][coord.x as usize].perma_lit_room
            && state.dg.current_level > 0
    }) {
        let half_height = SCREEN_HEIGHT / 2;
        let half_width = SCREEN_WIDTH / 2;
        let start_row = (coord.y / half_height) * half_height + 1;
        let start_col = (coord.x / half_width) * half_width + 1;
        let end_row = start_row + half_height - 1;
        let end_col = start_col + half_width - 1;

        for spot_y in start_row..=end_row {
            for spot_x in start_col..=end_col {
                let spot = Coord_t {
                    y: spot_y,
                    x: spot_x,
                };
                let changed = with_state_mut(|state| {
                    let tile = &mut state.dg.floor[spot_y as usize][spot_x as usize];
                    if tile.perma_lit_room && tile.feature_id <= MAX_CAVE_FLOOR {
                        tile.permanent_light = false;
                        tile.feature_id = TILE_DARK_FLOOR;
                        true
                    } else {
                        false
                    }
                });
                if changed {
                    dungeon_lite_spot(spot);
                    if !cave_tile_visible(spot) {
                        darkened = true;
                    }
                }
            }
        }
    } else {
        for spot_y in coord.y - 1..=coord.y + 1 {
            for spot_x in coord.x - 1..=coord.x + 1 {
                let changed = with_state_mut(|state| {
                    let tile = &mut state.dg.floor[spot_y as usize][spot_x as usize];
                    if tile.feature_id == TILE_CORR_FLOOR && tile.permanent_light {
                        tile.permanent_light = false;
                        true
                    } else {
                        false
                    }
                });
                if changed {
                    darkened = true;
                }
            }
        }
    }

    if darkened && with_state(|state| state.py.flags.blind < 1) {
        terminal::print_message(Some("Darkness surrounds you."));
    }

    darkened
}

/// 654
pub fn spell_light_line(coord: Coord_t, direction: i32) {
    let mut coord = coord;
    let mut distance = 0;
    let mut finished = false;

    while !finished {
        let (feature_id, permanent_light, temporary_light, creature_id) = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            (
                tile.feature_id,
                tile.permanent_light,
                tile.temporary_light,
                tile.creature_id,
            )
        });

        if distance > i32::from(OBJECT_BOLTS_MAX_RANGE) || feature_id >= MIN_CLOSED_SPACE {
            let _ = player_move_position(direction, &mut coord);
            finished = true;
            continue;
        }

        if !permanent_light && !temporary_light {
            with_state_mut(|state| {
                state.dg.floor[coord.y as usize][coord.x as usize].permanent_light = true;
            });

            let tmp_coord = coord;

            if feature_id == TILE_LIGHT_FLOOR {
                if coord_inside_panel(tmp_coord) {
                    dungeon_light_room(tmp_coord);
                }
            } else {
                dungeon_lite_spot(tmp_coord);
            }
        }

        with_state_mut(|state| {
            state.dg.floor[coord.y as usize][coord.x as usize].permanent_light = true;
        });

        if creature_id > 1 {
            spell_light_line_touches_monster(i32::from(creature_id));
        }

        let _ = player_move_position(direction, &mut coord);
        distance += 1;
    }
}

/// 667
pub fn spell_starlite(coord: Coord_t) {
    if with_state(|state| state.py.flags.blind < 1) {
        terminal::print_message(Some(
            "The end of the staff bursts into a blue shimmering light.",
        ));
    }

    for dir in 1..=9 {
        if dir != 5 {
            spell_light_line(coord, dir);
        }
    }
}

/// 713
#[must_use]
pub fn spell_disarm_all_in_direction(coord: Coord_t, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut disarmed = false;

    loop {
        let category_and_flags = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            if tile.treasure_id == 0 {
                return None;
            }
            let item = &state.game.treasure.list[tile.treasure_id as usize];
            Some((item.category_id, item.flags))
        });

        if let Some((category_id, flags)) = category_and_flags {
            match category_id {
                TV_INVIS_TRAP | TV_VIS_TRAP if dungeon_delete_object(coord) => {
                    disarmed = true;
                }
                TV_CLOSED_DOOR => {
                    with_state_mut(|state| {
                        let tid = state.dg.floor[coord.y as usize][coord.x as usize].treasure_id;
                        state.game.treasure.list[tid as usize].misc_use = 0;
                    });
                }
                TV_SECRET_DOOR => {
                    with_state_mut(|state| {
                        state.dg.floor[coord.y as usize][coord.x as usize].field_mark = true;
                    });
                    trap_change_visibility(coord);
                    disarmed = true;
                }
                TV_CHEST if flags != 0 => {
                    terminal::print_message(Some("Click!"));
                    let tid = with_state(|state| {
                        state.dg.floor[coord.y as usize][coord.x as usize].treasure_id
                    });
                    let mut item = with_state(|state| state.game.treasure.list[tid as usize]);
                    item.flags &= !(CH_TRAPPED | CH_LOCKED);
                    item.special_name_id = SpecialNameIds::SN_UNLOCKED as u8;
                    spell_item_identify_and_remove_random_inscription(&mut item);
                    with_state_mut(|state| {
                        state.game.treasure.list[tid as usize] = item;
                    });
                    disarmed = true;
                }
                _ => {}
            }
        }

        let _ = player_move_position(direction, &mut coord);
        distance += 1;

        let feature_id =
            with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].feature_id);
        if distance > i32::from(OBJECT_BOLTS_MAX_RANGE) || feature_id > MAX_OPEN_SPACE {
            break;
        }
    }

    disarmed
}

pub use crate::inventory::{
    damage_acid, damage_cold, damage_corroding_gas, damage_fire, damage_lightning_bolt,
    damage_poisoned_gas, execute_disenchant_attack,
};

/// Core recharge logic after item selection —
pub fn spell_recharge_item_at(item_id: i32, number_of_charges: i32) {
    let mut fail_chance = with_state(|state| {
        let item = &state.py.inventory[item_id as usize];
        number_of_charges + 50 - i32::from(item.depth_first_found) - i32::from(item.misc_use)
    });

    if fail_chance < 19 {
        fail_chance = 1;
    } else {
        fail_chance = random_number(fail_chance / 10);
    }

    if fail_chance == 1 {
        terminal::print_message(Some("There is a bright flash of light."));
        inventory_destroy_item(item_id);
    } else {
        let charge_divisor = with_state(|state| {
            let item = &state.py.inventory[item_id as usize];
            number_of_charges / (i32::from(item.depth_first_found) + 2) + 1
        });
        let charge_roll = random_number(charge_divisor);
        with_state_mut(|state| {
            let item = &mut state.py.inventory[item_id as usize];
            item.misc_use = item.misc_use.wrapping_add((2 + charge_roll) as i16);
            if spell_item_identified(*item) {
                spell_item_remove_identification(item);
            }
            item_identification_clear_empty(item);
        });
    }
}

/// 1146
#[must_use]
pub fn spell_recharge_item(number_of_charges: i32) -> bool {
    let mut item_pos_start = -1;
    let mut item_pos_end = -1;
    if !inventory_find_range(
        i32::from(TV_STAFF),
        i32::from(TV_WAND),
        &mut item_pos_start,
        &mut item_pos_end,
    ) {
        terminal::print_message(Some("You have nothing to recharge."));
        return false;
    }

    let mut item_id = 0;
    if !inventory_get_input_for_item_id(
        &mut item_id,
        "Recharge which item?",
        item_pos_start,
        item_pos_end,
        None,
        None,
    ) {
        return false;
    }

    spell_recharge_item_at(item_id, number_of_charges);

    true
}

/// 1662
pub fn spell_teleport_away_monster(monster_id: i32, mut distance_from_player: i32) {
    let mut counter = 0;
    let mut coord = Coord_t { y: 0, x: 0 };

    let monster_pos = with_state(|state| state.monsters[monster_id as usize].pos);

    loop {
        loop {
            let pos = with_state(|state| state.monsters[monster_id as usize].pos);
            coord.y =
                pos.y + (random_number(2 * distance_from_player + 1) - (distance_from_player + 1));
            coord.x =
                pos.x + (random_number(2 * distance_from_player + 1) - (distance_from_player + 1));
            if coord_in_bounds(coord) {
                break;
            }
        }

        counter += 1;
        if counter > 9 {
            counter = 0;
            distance_from_player += 5;
        }

        let blocked = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            i32::from(tile.feature_id) >= i32::from(MIN_CLOSED_SPACE) || tile.creature_id != 0
        });
        if !blocked {
            break;
        }
    }

    dungeon_move_creature_record(monster_pos, coord);
    dungeon_lite_spot(monster_pos);

    with_state_mut(|state| {
        let monster = &mut state.monsters[monster_id as usize];
        monster.pos.y = coord.y;
        monster.pos.x = coord.x;
        monster.lit = false;
        monster.distance_from_player = coord_distance_between(state.py.pos, coord) as u8;
    });

    monster_update_visibility(monster_id);
}

/// 1700
pub fn spell_teleport_player_to(coord: Coord_t) {
    let mut distance = 1;
    let mut counter = 0;
    let mut rnd_coord = Coord_t { y: 0, x: 0 };

    loop {
        loop {
            rnd_coord.y = coord.y + (random_number(2 * distance + 1) - (distance + 1));
            rnd_coord.x = coord.x + (random_number(2 * distance + 1) - (distance + 1));
            if coord_in_bounds(rnd_coord) {
                break;
            }
        }

        counter += 1;
        if counter > 9 {
            counter = 0;
            distance += 1;
        }

        let blocked = with_state(|state| {
            let tile = &state.dg.floor[rnd_coord.y as usize][rnd_coord.x as usize];
            i32::from(tile.feature_id) >= i32::from(MIN_CLOSED_SPACE) || tile.creature_id >= 2
        });
        if !blocked {
            break;
        }
    }

    let player_pos = with_state(|state| state.py.pos);
    dungeon_move_creature_record(player_pos, rnd_coord);

    for spot_y in player_pos.y - 1..=player_pos.y + 1 {
        for spot_x in player_pos.x - 1..=player_pos.x + 1 {
            let spot = Coord_t {
                y: spot_y,
                x: spot_x,
            };
            with_state_mut(|state| {
                state.dg.floor[spot.y as usize][spot.x as usize].temporary_light = false;
            });
            dungeon_lite_spot(spot);
        }
    }

    dungeon_lite_spot(player_pos);

    with_state_mut(|state| {
        state.py.pos.y = rnd_coord.y;
        state.py.pos.x = rnd_coord.x;
    });

    dungeon_reset_view();
    update_monsters(false);
}

/// 1938
#[must_use]
pub fn spell_change_player_hit_points(adjustment: i32) -> bool {
    if with_state(|state| state.py.misc.current_hp >= state.py.misc.max_hp) {
        return false;
    }

    with_state_mut(|state| {
        state.py.misc.current_hp += adjustment as i16;
        if state.py.misc.current_hp > state.py.misc.max_hp {
            state.py.misc.current_hp = state.py.misc.max_hp;
            state.py.misc.current_hp_fraction = 0;
        }
    });
    print_character_current_hit_points();

    let adjustment = adjustment / 5;
    if adjustment < 3 {
        if adjustment == 0 {
            terminal::print_message(Some("You feel a little better."));
        } else {
            terminal::print_message(Some("You feel better."));
        }
    } else if adjustment < 7 {
        terminal::print_message(Some("You feel much better."));
    } else {
        terminal::print_message(Some("You feel very good."));
    }

    true
}

/// 1456
#[must_use]
pub fn spell_wall_to_mud(coord: Coord_t, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut turned = false;
    let mut finished = false;

    while !finished {
        let _ = player_move_position(direction, &mut coord);
        distance += 1;

        let (feature_id, treasure_id, creature_id) = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            (tile.feature_id, tile.treasure_id, tile.creature_id)
        });

        if distance == i32::from(OBJECT_BOLTS_MAX_RANGE) {
            finished = true;
        }

        if i32::from(feature_id) >= i32::from(MIN_CAVE_WALL) && feature_id != TILE_BOUNDARY_WALL {
            finished = true;
            let _ = player_tunnel_wall(coord, 1, 0);
            if cave_tile_visible(coord) {
                turned = true;
                terminal::print_message(Some("The wall turns into mud."));
            }
        } else if treasure_id != 0 && i32::from(feature_id) >= i32::from(MIN_CLOSED_SPACE) {
            finished = true;

            if coord_inside_panel(coord) && cave_tile_visible(coord) {
                turned = true;
                let description = with_state(|state| {
                    let mut description = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
                    item_description(
                        &mut description,
                        state.game.treasure.list[treasure_id as usize],
                        false,
                    );
                    description
                });
                let end = description
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(description.len());
                let desc = String::from_utf8_lossy(&description[..end]);
                terminal::print_message(Some(&format!("The {desc} turns into mud.")));
            }

            let category_id =
                with_state(|state| state.game.treasure.list[treasure_id as usize].category_id);
            if category_id == TV_RUBBLE {
                let _ = dungeon_delete_object(coord);
                if random_number(10) == 1 {
                    dungeon_place_random_object_at(coord, false);
                    if cave_tile_visible(coord) {
                        terminal::print_message(Some("You have found something!"));
                    }
                }
                dungeon_lite_spot(coord);
            } else {
                let _ = dungeon_delete_object(coord);
            }
        }

        if creature_id > 1 {
            let (defenses, _lit, name, recall_creature_id) = with_state(|state| {
                let monster = &state.monsters[creature_id as usize];
                let creature = &CREATURES_LIST[monster.creature_id as usize];
                (
                    creature.defenses,
                    monster.lit,
                    monster_name_description(creature.name, monster.lit),
                    monster.creature_id,
                )
            });

            if (defenses & CD_STONE) != 0 {
                let hit_result = monster_take_hit(i32::from(creature_id), 100);
                if hit_result >= 0 {
                    with_state_mut(|state| {
                        state.creature_recall[recall_creature_id as usize].defenses |= CD_STONE;
                    });
                    print_monster_action_text(&name, "dissolves!");
                    display_character_experience();
                } else {
                    with_state_mut(|state| {
                        state.creature_recall[recall_creature_id as usize].defenses |= CD_STONE;
                    });
                    print_monster_action_text(&name, "grunts in pain!");
                }
                finished = true;
            }
        }
    }

    turned
}

/// 1495
#[must_use]
pub fn spell_destroy_doors_traps_in_direction(coord: Coord_t, direction: i32) -> bool {
    let mut coord = coord;
    let mut destroyed = false;
    let mut distance = 0;

    loop {
        let _ = player_move_position(direction, &mut coord);
        distance += 1;

        let treasure_id =
            with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].treasure_id);

        if treasure_id != 0 {
            let (category_id, flags) = with_state(|state| {
                let item = &state.game.treasure.list[treasure_id as usize];
                (item.category_id, item.flags)
            });

            if category_id == TV_INVIS_TRAP
                || category_id == TV_CLOSED_DOOR
                || category_id == TV_VIS_TRAP
                || category_id == TV_OPEN_DOOR
                || category_id == TV_SECRET_DOOR
            {
                if dungeon_delete_object(coord) {
                    destroyed = true;
                    terminal::print_message(Some("There is a bright flash of light!"));
                }
            } else if category_id == TV_CHEST && flags != 0 {
                destroyed = true;
                terminal::print_message(Some("Click!"));

                let tid = treasure_id;
                let mut item = with_state(|state| state.game.treasure.list[tid as usize]);
                item.flags &= !(CH_TRAPPED | CH_LOCKED);
                item.special_name_id = SpecialNameIds::SN_UNLOCKED as u8;
                spell_item_identify_and_remove_random_inscription(&mut item);
                with_state_mut(|state| {
                    state.game.treasure.list[tid as usize] = item;
                });
            }
        }

        let feature_id =
            with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].feature_id);

        if distance > i32::from(OBJECT_BOLTS_MAX_RANGE)
            && i32::from(feature_id) > i32::from(MAX_OPEN_SPACE)
        {
            break;
        }
    }

    destroyed
}

/// 1604
#[must_use]
pub fn spell_build_wall(coord: Coord_t, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut built = false;
    let mut finished = false;

    while !finished {
        let _ = player_move_position(direction, &mut coord);
        distance += 1;

        let (feature_id, treasure_id, creature_id) = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            (tile.feature_id, tile.treasure_id, tile.creature_id)
        });

        if distance > i32::from(OBJECT_BOLTS_MAX_RANGE)
            || i32::from(feature_id) >= i32::from(MIN_CLOSED_SPACE)
        {
            finished = true;
            continue;
        }

        if treasure_id != 0 {
            let _ = dungeon_delete_object(coord);
        }

        if creature_id > 1 {
            finished = true;

            let (movement, sprite, name) = with_state(|state| {
                let monster = &state.monsters[creature_id as usize];
                let creature = &CREATURES_LIST[monster.creature_id as usize];
                (
                    creature.movement,
                    creature.sprite,
                    monster_name_description(creature.name, monster.lit),
                )
            });

            if (movement & CM_PHASE) == 0 {
                let damage = if (movement & CM_ATTACK_ONLY) != 0 {
                    3000
                } else {
                    dice_roll(Dice { dice: 4, sides: 8 })
                };

                print_monster_action_text(&name, "wails out in pain!");

                if monster_take_hit(i32::from(creature_id), damage) >= 0 {
                    print_monster_action_text(&name, "is embedded in the rock.");
                    display_character_experience();
                }
            } else if sprite == b'E' || sprite == b'X' {
                let roll = dice_roll(Dice { dice: 4, sides: 8 });
                with_state_mut(|state| {
                    state.monsters[creature_id as usize].hp += roll as i16;
                });
            }
        }

        with_state_mut(|state| {
            let tile = &mut state.dg.floor[coord.y as usize][coord.x as usize];
            tile.feature_id = TILE_MAGMA_WALL;
            tile.field_mark = false;
            tile.permanent_light = tile.temporary_light || tile.permanent_light;
        });
        dungeon_lite_spot(coord);

        built = true;
    }

    built
}

/// 1966
fn earthquake_hits_monster(monster_id: i32) {
    let (movement, sprite, name) = with_state(|state| {
        let monster = &state.monsters[monster_id as usize];
        let creature = &CREATURES_LIST[monster.creature_id as usize];
        (
            creature.movement,
            creature.sprite,
            monster_name_description(creature.name, monster.lit),
        )
    });

    if (movement & CM_PHASE) == 0 {
        let damage = if (movement & CM_ATTACK_ONLY) != 0 {
            3000
        } else {
            dice_roll(Dice { dice: 4, sides: 8 })
        };

        print_monster_action_text(&name, "wails out in pain!");

        if monster_take_hit(monster_id, damage) >= 0 {
            print_monster_action_text(&name, "is embedded in the rock.");
            display_character_experience();
        }
    } else if sprite == b'E' || sprite == b'X' {
        let roll = dice_roll(Dice { dice: 4, sides: 8 });
        with_state_mut(|state| {
            state.monsters[monster_id as usize].hp += roll as i16;
        });
    }
}

/// 2008
pub fn spell_earthquake() {
    let player_pos = with_state(|state| state.py.pos);

    for coord_y in player_pos.y - 8..=player_pos.y + 8 {
        for coord_x in player_pos.x - 8..=player_pos.x + 8 {
            let coord = Coord_t {
                y: coord_y,
                x: coord_x,
            };

            if (coord.y != player_pos.y || coord.x != player_pos.x)
                && coord_in_bounds(coord)
                && random_number(8) == 1
            {
                let (treasure_id, creature_id, feature_id) = with_state(|state| {
                    let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
                    (tile.treasure_id, tile.creature_id, tile.feature_id)
                });

                if treasure_id != 0 {
                    let _ = dungeon_delete_object(coord);
                }

                if creature_id > 1 {
                    earthquake_hits_monster(i32::from(creature_id));
                }

                with_state_mut(|state| {
                    let tile = &mut state.dg.floor[coord.y as usize][coord.x as usize];

                    if i32::from(tile.feature_id) >= i32::from(MIN_CAVE_WALL)
                        && tile.feature_id != TILE_BOUNDARY_WALL
                    {
                        tile.feature_id = TILE_CORR_FLOOR;
                        tile.permanent_light = false;
                        tile.field_mark = false;
                    } else if i32::from(tile.feature_id) <= i32::from(MAX_CAVE_FLOOR) {
                        tile.field_mark = false;
                    }
                });

                let wall_type = if i32::from(feature_id) <= i32::from(MAX_CAVE_FLOOR) {
                    Some(random_number(10))
                } else {
                    None
                };

                if let Some(tmp) = wall_type {
                    with_state_mut(|state| {
                        let tile = &mut state.dg.floor[coord.y as usize][coord.x as usize];
                        tile.feature_id = if tmp < 6 {
                            TILE_QUARTZ_WALL
                        } else if tmp < 9 {
                            TILE_MAGMA_WALL
                        } else {
                            TILE_GRANITE_WALL
                        };
                    });
                }

                dungeon_lite_spot(coord);
            }
        }
    }
}

/// 2028
pub fn spell_create_food() {
    let player_pos = with_state(|state| state.py.pos);
    let has_object = with_state(|state| {
        state.dg.floor[player_pos.y as usize][player_pos.x as usize].treasure_id != 0
    });

    if has_object {
        with_state_mut(|state| state.game.player_free_turn = true);
        terminal::print_message(Some("There is already an object under you."));
        return;
    }

    dungeon_place_random_object_at(player_pos, false);

    with_state_mut(|state| {
        let treasure_id = state.dg.floor[player_pos.y as usize][player_pos.x as usize].treasure_id;
        inventory_item_copy_to(
            OBJ_MUSH as i16,
            &mut state.game.treasure.list[treasure_id as usize],
        );
    });
}

/// 2113
pub fn spell_lose_str() {
    if with_state(|state| !state.py.flags.sustain_str) {
        let _ = crate::player_stats::player_stat_random_decrease(PlayerAttr::A_STR);
        terminal::print_message(Some("You feel very sick."));
    } else {
        terminal::print_message(Some("You feel sick for a moment,  it passes."));
    }
}

/// 2123
pub fn spell_lose_int() {
    if with_state(|state| !state.py.flags.sustain_int) {
        let _ = crate::player_stats::player_stat_random_decrease(PlayerAttr::A_INT);
        terminal::print_message(Some("You become very dizzy."));
    } else {
        terminal::print_message(Some("You become dizzy for a moment,  it passes."));
    }
}

/// 2133
pub fn spell_lose_wis() {
    if with_state(|state| !state.py.flags.sustain_wis) {
        let _ = crate::player_stats::player_stat_random_decrease(PlayerAttr::A_WIS);
        terminal::print_message(Some("You feel very naive."));
    } else {
        terminal::print_message(Some("You feel naive for a moment,  it passes."));
    }
}

/// 2143
pub fn spell_lose_dex() {
    if with_state(|state| !state.py.flags.sustain_dex) {
        let _ = crate::player_stats::player_stat_random_decrease(PlayerAttr::A_DEX);
        terminal::print_message(Some("You feel very sore."));
    } else {
        terminal::print_message(Some("You feel sore for a moment,  it passes."));
    }
}

/// 2153
pub fn spell_lose_con() {
    if with_state(|state| !state.py.flags.sustain_con) {
        let _ = crate::player_stats::player_stat_random_decrease(PlayerAttr::A_CON);
        terminal::print_message(Some("You feel very sick."));
    } else {
        terminal::print_message(Some("You feel sick for a moment,  it passes."));
    }
}

/// 2163
pub fn spell_lose_chr() {
    if with_state(|state| !state.py.flags.sustain_chr) {
        let _ = crate::player_stats::player_stat_random_decrease(PlayerAttr::A_CHR);
        terminal::print_message(Some("Your skin starts to itch."));
    } else {
        terminal::print_message(Some("Your skin starts to itch, but feels better now."));
    }
}

/// 2199
pub fn spell_lose_exp(adjustment: i32) {
    with_state_mut(|state| {
        if adjustment > state.py.misc.exp {
            state.py.misc.exp = 0;
        } else {
            state.py.misc.exp -= adjustment;
        }
    });

    display_character_experience();

    let new_level = with_state(|state| {
        let mut exp = 0usize;
        while (state.py.base_exp_levels[exp] * u32::from(state.py.misc.experience_factor) / 100)
            as i32
            <= state.py.misc.exp
        {
            exp += 1;
        }
        exp + 1
    });

    let level_changed = with_state(|state| state.py.misc.level != new_level as u16);
    if level_changed {
        let class_id = with_state(|state| state.py.misc.class_id);
        with_state_mut(|state| {
            state.py.misc.level = new_level as u16;
        });
        player_calculate_hit_points();

        let spell_type = CLASSES[class_id as usize].class_to_use_mage_spells;
        if spell_type == SPELL_TYPE_MAGE {
            player_calculate_allowed_spells_count(PlayerAttr::A_INT);
            player_gain_mana(PlayerAttr::A_INT);
        } else if spell_type == SPELL_TYPE_PRIEST {
            player_calculate_allowed_spells_count(PlayerAttr::A_WIS);
            player_gain_mana(PlayerAttr::A_WIS);
        }

        print_character_level();
        print_character_title();
    }
}

/// 2213
#[must_use]
pub fn spell_slow_poison() -> bool {
    let poisoned = with_state(|state| state.py.flags.poisoned);
    if poisoned <= 0 {
        return false;
    }

    with_state_mut(|state| {
        state.py.flags.poisoned /= 2;
        if state.py.flags.poisoned < 1 {
            state.py.flags.poisoned = 1;
        }
    });
    terminal::print_message(Some("The effect of the poison has been reduced."));
    true
}

/// 2312
#[must_use]
pub fn spell_enchant_item(plusses: &mut i16, max_bonus_limit: i16) -> bool {
    if max_bonus_limit <= 0 {
        return false;
    }

    let mut chance = 0;

    if *plusses > 0 {
        chance = i32::from(*plusses);
        if random_number(100) == 1 {
            chance = random_number(chance) - 1;
        }
    }

    if random_number(i32::from(max_bonus_limit)) > chance {
        *plusses += 1;
        return true;
    }

    false
}

/// 2327
#[must_use]
pub fn spell_remove_curse_from_all_worn_items() -> bool {
    let mut removed = false;

    for slot in [
        PlayerEquipment::Wield,
        PlayerEquipment::Head,
        PlayerEquipment::Neck,
        PlayerEquipment::Body,
        PlayerEquipment::Arm,
        PlayerEquipment::Hands,
        PlayerEquipment::Right,
        PlayerEquipment::Left,
        PlayerEquipment::Feet,
        PlayerEquipment::Outer,
    ] {
        if player_worn_item_is_cursed(slot) {
            player_worn_item_remove_curse(slot);
            player_recalculate_bonuses();
            removed = true;
        }
    }

    removed
}

/// 2344
#[must_use]
pub fn spell_restore_player_levels() -> bool {
    if !with_state(|state| state.py.misc.max_exp > state.py.misc.exp) {
        return false;
    }

    terminal::print_message(Some("You feel your life energies returning."));

    loop {
        let needs_restore = with_state(|state| state.py.misc.exp < state.py.misc.max_exp);
        if !needs_restore {
            break;
        }
        with_state_mut(|state| {
            state.py.misc.exp = state.py.misc.max_exp;
        });
        display_character_experience();
    }

    true
}

/// Predicate mirroring `bool (*)(Inventory_t *)` destroy hooks
pub type ItemDestroyPredicate = fn(&Inventory) -> bool;

/// Flags describing which area-affect spell effects apply.
#[derive(Clone, Copy, Debug)]
pub struct SpellAreaAffect {
    pub weapon_type: u32,
    pub harm_type: u16,
    pub destroy: ItemDestroyPredicate,
}

/// 756
pub fn spell_get_area_affect_flags(spell_type: MagicSpellFlags) -> SpellAreaAffect {
    match spell_type {
        MagicSpellFlags::MagicMissile => SpellAreaAffect {
            weapon_type: 0,
            harm_type: 0,
            destroy: set_null,
        },
        MagicSpellFlags::Lightning => SpellAreaAffect {
            weapon_type: monster_spells::CS_BR_LIGHT,
            harm_type: CD_LIGHT,
            destroy: set_lightning_destroyable_items,
        },
        MagicSpellFlags::PoisonGas => SpellAreaAffect {
            weapon_type: monster_spells::CS_BR_GAS,
            harm_type: CD_POISON,
            destroy: set_null,
        },
        MagicSpellFlags::Acid => SpellAreaAffect {
            weapon_type: monster_spells::CS_BR_ACID,
            harm_type: CD_ACID,
            destroy: set_acid_destroyable_items,
        },
        MagicSpellFlags::Frost => SpellAreaAffect {
            weapon_type: monster_spells::CS_BR_FROST,
            harm_type: CD_FROST,
            destroy: set_frost_destroyable_items,
        },
        MagicSpellFlags::Fire => SpellAreaAffect {
            weapon_type: monster_spells::CS_BR_FIRE,
            harm_type: CD_FIRE,
            destroy: set_fire_destroyable_items,
        },
        MagicSpellFlags::HolyOrb => SpellAreaAffect {
            weapon_type: 0,
            harm_type: CD_EVIL,
            destroy: set_null,
        },
    }
}

/// Resist/immune scaling shared by bolt, ball, and breath monster paths.
pub fn spell_apply_monster_damage_scaling(
    mut damage: i32,
    harm_type: u16,
    weapon_type: u32,
    creature_defenses: u16,
    creature_spells: u32,
) -> i32 {
    if (harm_type & creature_defenses) != 0 {
        damage *= 2;
    } else if (weapon_type & creature_spells) != 0 {
        damage /= 4;
    }
    damage
}

/// Ball/breath distance falloff: `damage / (distance + 1)`.
pub fn spell_apply_area_distance_falloff(damage: i32, distance: i32) -> i32 {
    damage / (distance + 1)
}

fn print_bolt_strikes_monster_message(creature_name: &str, bolt_name: &str, is_lit: bool) {
    let monster_name = if is_lit {
        format!("the {creature_name}")
    } else {
        "it".to_string()
    };
    terminal::print_message(Some(&format!("The {bolt_name} strikes {monster_name}.")));
}

/// 806
fn spell_fire_bolt_touches_monster(
    coord: Coord_t,
    damage: i32,
    harm_type: u16,
    weapon_type: u32,
    bolt_name: &str,
) {
    let (monster_id, creature_name, was_lit) = with_state(|state| {
        let monster_id = i32::from(state.dg.floor[coord.y as usize][coord.x as usize].creature_id);
        let monster = &state.monsters[monster_id as usize];
        let creature = &CREATURES_LIST[monster.creature_id as usize];
        (monster_id, creature.name, monster.lit)
    });

    let saved_lit_status =
        with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].permanent_light);
    with_state_mut(|state| {
        state.dg.floor[coord.y as usize][coord.x as usize].permanent_light = true;
    });
    monster_update_visibility(monster_id);
    with_state_mut(|state| {
        state.dg.floor[coord.y as usize][coord.x as usize].permanent_light = saved_lit_status;
    });

    put_qio();
    print_bolt_strikes_monster_message(creature_name, bolt_name, was_lit);

    let (scaled_damage, lit) = with_state_mut(|state| {
        let monster = &state.monsters[monster_id as usize];
        let creature = &CREATURES_LIST[monster.creature_id as usize];
        let damage = spell_apply_monster_damage_scaling(
            damage,
            harm_type,
            weapon_type,
            creature.defenses,
            creature.spells,
        );
        if (harm_type & creature.defenses) != 0 {
            if monster.lit {
                state.creature_recall[monster.creature_id as usize].defenses |= harm_type;
            }
        } else if (weapon_type & creature.spells) != 0 && monster.lit {
            state.creature_recall[monster.creature_id as usize].spells |= weapon_type;
        }
        (damage, monster.lit)
    });

    let name = monster_name_description(creature_name, lit);
    if monster_take_hit(monster_id, scaled_damage) >= 0 {
        print_monster_action_text(&name, "dies in a fit of agony.");
        display_character_experience();
    } else if scaled_damage > 0 {
        print_monster_action_text(&name, "screams in agony.");
    }
}

/// 846
pub fn spell_fire_bolt(
    mut coord: Coord_t,
    direction: i32,
    damage_hp: i32,
    spell_type: MagicSpellFlags,
    spell_name: &str,
) {
    let affect = spell_get_area_affect_flags(spell_type);

    let mut distance = 0;
    let mut finished = false;

    while !finished {
        let old_coord = coord;
        let _ = player_move_position(direction, &mut coord);
        distance += 1;

        dungeon_lite_spot(old_coord);

        let (feature_id, creature_id, show_bolt) = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            (
                tile.feature_id,
                tile.creature_id,
                coord_inside_panel_bounds(&state.dg.panel, coord) && state.py.flags.blind < 1,
            )
        });

        if distance > i32::from(OBJECT_BOLTS_MAX_RANGE) || feature_id >= MIN_CLOSED_SPACE {
            finished = true;
            continue;
        }

        if creature_id > 1 {
            finished = true;
            spell_fire_bolt_touches_monster(
                coord,
                damage_hp,
                affect.harm_type,
                affect.weapon_type,
                spell_name,
            );
        } else if show_bolt {
            panel_put_tile(
                b'*',
                Coord {
                    y: coord.y,
                    x: coord.x,
                },
            );
            put_qio();
        }
    }
}

fn spell_explosion_at(
    center: Coord_t,
    damage_hp: i32,
    spell_type: MagicSpellFlags,
    spell_name: &str,
    count_hits: bool,
) -> (i32, i32) {
    let max_distance = 2;
    let affect = spell_get_area_affect_flags(spell_type);
    let mut total_hits = 0;
    let mut total_kills = 0;

    for row in (center.y - max_distance)..=(center.y + max_distance) {
        for col in (center.x - max_distance)..=(center.x + max_distance) {
            let spot = Coord_t { y: row, x: col };
            if !coord_in_bounds(spot)
                || coord_distance_between(center, spot) > max_distance
                || !los(center, spot)
            {
                continue;
            }

            let destroy_item = with_state(|state| {
                let tile = &state.dg.floor[spot.y as usize][spot.x as usize];
                tile.treasure_id != 0
                    && (affect.destroy)(&state.game.treasure.list[tile.treasure_id as usize])
            });
            if destroy_item {
                let _ = dungeon_delete_object(spot);
            }

            let (feature_id, creature_id, show_blast) = with_state(|state| {
                let tile = &state.dg.floor[spot.y as usize][spot.x as usize];
                (
                    tile.feature_id,
                    tile.creature_id,
                    coord_inside_panel_bounds(&state.dg.panel, spot) && state.py.flags.blind < 1,
                )
            });

            if feature_id > MAX_OPEN_SPACE {
                continue;
            }

            if creature_id > 1 {
                let monster_id = i32::from(creature_id);
                let saved_lit_status = with_state(|state| {
                    state.dg.floor[spot.y as usize][spot.x as usize].permanent_light
                });
                with_state_mut(|state| {
                    state.dg.floor[spot.y as usize][spot.x as usize].permanent_light = true;
                });
                monster_update_visibility(monster_id);
                with_state_mut(|state| {
                    state.dg.floor[spot.y as usize][spot.x as usize].permanent_light =
                        saved_lit_status;
                });

                if count_hits {
                    total_hits += 1;
                }

                let scaled_damage = with_state_mut(|state| {
                    let monster = &state.monsters[monster_id as usize];
                    let creature = &CREATURES_LIST[monster.creature_id as usize];
                    let mut damage = spell_apply_monster_damage_scaling(
                        damage_hp,
                        affect.harm_type,
                        affect.weapon_type,
                        creature.defenses,
                        creature.spells,
                    );
                    if (affect.harm_type & creature.defenses) != 0 {
                        if monster.lit {
                            state.creature_recall[monster.creature_id as usize].defenses |=
                                affect.harm_type;
                        }
                    } else if (affect.weapon_type & creature.spells) != 0 && monster.lit {
                        state.creature_recall[monster.creature_id as usize].spells |=
                            affect.weapon_type;
                    }
                    damage = spell_apply_area_distance_falloff(
                        damage,
                        coord_distance_between(spot, center),
                    );
                    damage
                });

                if monster_take_hit(monster_id, scaled_damage) >= 0 {
                    total_kills += 1;
                }
            } else if show_blast {
                panel_put_tile(
                    b'*',
                    Coord {
                        y: spot.y,
                        x: spot.x,
                    },
                );
            }
        }
    }

    put_qio();

    for row in (center.y - 2)..=(center.y + 2) {
        for col in (center.x - 2)..=(center.x + 2) {
            let spot = Coord_t { y: row, x: col };
            if coord_in_bounds(spot)
                && coord_inside_panel(spot)
                && coord_distance_between(center, spot) <= max_distance
            {
                dungeon_lite_spot(spot);
            }
        }
    }

    if count_hits {
        if total_hits == 1 {
            terminal::print_message(Some(&format!("The {spell_name} envelops a creature!")));
        } else if total_hits > 1 {
            terminal::print_message(Some(&format!(
                "The {spell_name} envelops several creatures!"
            )));
        }

        if total_kills == 1 {
            terminal::print_message(Some("There is a scream of agony!"));
        } else if total_kills > 1 {
            terminal::print_message(Some("There are several screams of agony!"));
        }

        if total_kills >= 0 {
            display_character_experience();
        }
    }

    (total_hits, total_kills)
}

/// 981
pub fn spell_fire_ball(
    mut coord: Coord_t,
    direction: i32,
    damage_hp: i32,
    spell_type: MagicSpellFlags,
    spell_name: &str,
) {
    let mut distance = 0;
    let mut finished = false;

    while !finished {
        let old_coord = coord;
        let _ = player_move_position(direction, &mut coord);
        distance += 1;

        dungeon_lite_spot(old_coord);

        if distance > i32::from(OBJECT_BOLTS_MAX_RANGE) {
            finished = true;
            continue;
        }

        let (feature_id, creature_id, show_bolt) = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            (
                tile.feature_id,
                tile.creature_id,
                coord_inside_panel_bounds(&state.dg.panel, coord) && state.py.flags.blind < 1,
            )
        });

        if feature_id >= MIN_CLOSED_SPACE || creature_id > 1 {
            finished = true;

            if feature_id >= MIN_CLOSED_SPACE {
                coord = old_coord;
            }

            let _ = spell_explosion_at(coord, damage_hp, spell_type, spell_name, true);
        } else if show_bolt {
            panel_put_tile(
                b'*',
                Coord {
                    y: coord.y,
                    x: coord.x,
                },
            );
            put_qio();
        }
    }
}

/// 1099
pub fn spell_breath(
    coord: Coord_t,
    monster_id: i32,
    damage_hp: i32,
    spell_type: MagicSpellFlags,
    spell_name: &Vtype_t,
) {
    let max_distance = 2;
    let affect = spell_get_area_affect_flags(spell_type);

    for row in (coord.y - 2)..=(coord.y + 2) {
        for col in (coord.x - 2)..=(coord.x + 2) {
            let location = Coord_t { y: row, x: col };
            if !coord_in_bounds(location)
                || coord_distance_between(coord, location) > max_distance
                || !los(coord, location)
            {
                continue;
            }

            let destroy_item = with_state(|state| {
                let tile = &state.dg.floor[location.y as usize][location.x as usize];
                tile.treasure_id != 0
                    && (affect.destroy)(&state.game.treasure.list[tile.treasure_id as usize])
            });
            if destroy_item {
                let _ = dungeon_delete_object(location);
            }

            let (feature_id, creature_id, show_blast) = with_state(|state| {
                let tile = &state.dg.floor[location.y as usize][location.x as usize];
                (
                    tile.feature_id,
                    tile.creature_id,
                    coord_inside_panel_bounds(&state.dg.panel, location)
                        && (state.py.flags.status & PY_BLIND) == 0,
                )
            });

            if feature_id > MAX_OPEN_SPACE {
                continue;
            }

            if show_blast {
                panel_put_tile(
                    b'*',
                    Coord {
                        y: location.y,
                        x: location.x,
                    },
                );
            }

            if creature_id > 1 {
                let tile_creature_id = i32::from(creature_id);
                let scaled_damage = with_state(|state| {
                    let monster = &state.monsters[tile_creature_id as usize];
                    let creature = &CREATURES_LIST[monster.creature_id as usize];
                    let mut damage = spell_apply_monster_damage_scaling(
                        damage_hp,
                        affect.harm_type,
                        affect.weapon_type,
                        creature.defenses,
                        creature.spells,
                    );
                    damage = spell_apply_area_distance_falloff(
                        damage,
                        coord_distance_between(location, coord),
                    );
                    damage
                });

                with_state_mut(|state| {
                    let monster = &mut state.monsters[tile_creature_id as usize];
                    monster.hp = (i32::from(monster.hp) - scaled_damage) as i16;
                    monster.sleep_count = 0;
                });

                let kill_info = with_state(|state| {
                    let monster = &state.monsters[tile_creature_id as usize];
                    if monster.hp < 0 {
                        Some((monster.pos, monster.creature_id, monster.lit))
                    } else {
                        None
                    }
                });

                if let Some((death_coord, recall_creature_id, lit)) = kill_info {
                    let movement = CREATURES_LIST[recall_creature_id as usize].movement;
                    let treasure_id = monster_death(death_coord, movement);

                    if lit {
                        with_state_mut(|state| {
                            let memory = &mut state.creature_recall[recall_creature_id as usize];
                            let tmp = (memory.movement & CM_TREASURE) >> CM_TR_SHIFT;
                            let mut tf = treasure_id;
                            if tmp > (tf & CM_TREASURE) >> CM_TR_SHIFT {
                                tf = (tf & !CM_TREASURE) | (tmp << CM_TR_SHIFT);
                            }
                            memory.movement = (memory.movement & !CM_TREASURE) | tf;
                        });
                    }

                    if monster_id < tile_creature_id {
                        dungeon_delete_monster(tile_creature_id);
                    } else {
                        dungeon_remove_monster_from_level(tile_creature_id);
                    }
                }
            } else if creature_id == 1 {
                let mut damage = spell_apply_area_distance_falloff(
                    damage_hp,
                    coord_distance_between(location, coord),
                );
                if damage == 0 {
                    damage = 1;
                }

                match spell_type {
                    MagicSpellFlags::Lightning => damage_lightning_bolt(damage, spell_name),
                    MagicSpellFlags::PoisonGas => damage_poisoned_gas(damage, spell_name),
                    MagicSpellFlags::Acid => damage_acid(damage, spell_name),
                    MagicSpellFlags::Frost => damage_cold(damage, spell_name),
                    MagicSpellFlags::Fire => damage_fire(damage, spell_name),
                    _ => {}
                }
            }
        }
    }

    put_qio();

    for row in (coord.y - 2)..=(coord.y + 2) {
        for col in (coord.x - 2)..=(coord.x + 2) {
            let spot = Coord_t { y: row, x: col };
            if coord_in_bounds(spot)
                && coord_inside_panel(spot)
                && coord_distance_between(coord, spot) <= max_distance
            {
                dungeon_lite_spot(spot);
            }
        }
    }
}

/// 1185
#[must_use]
pub fn spell_change_monster_hit_points(coord: Coord_t, direction: i32, damage_hp: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut changed = false;
    let mut finished = false;

    while !finished {
        let _ = player_move_position(direction, &mut coord);
        distance += 1;

        let (feature_id, creature_id) = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            (tile.feature_id, tile.creature_id)
        });

        if distance > i32::from(OBJECT_BOLTS_MAX_RANGE)
            || i32::from(feature_id) >= i32::from(MIN_CLOSED_SPACE)
        {
            finished = true;
            continue;
        }

        if creature_id > 1 {
            finished = true;

            let name = with_state(|state| {
                let monster = &state.monsters[creature_id as usize];
                let creature = &CREATURES_LIST[monster.creature_id as usize];
                monster_name_description(creature.name, monster.lit)
            });
            let hit_result = monster_take_hit(i32::from(creature_id), damage_hp);

            if hit_result >= 0 {
                print_monster_action_text(&name, "dies in a fit of agony.");
                display_character_experience();
            } else if damage_hp > 0 {
                print_monster_action_text(&name, "screams in agony.");
            }

            changed = true;
        }
    }

    changed
}

/// 1228
#[must_use]
pub fn spell_drain_life_from_monster(coord: Coord_t, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut drained = false;
    let mut finished = false;

    while !finished {
        let _ = player_move_position(direction, &mut coord);
        distance += 1;

        let (feature_id, creature_id) = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            (tile.feature_id, tile.creature_id)
        });

        if distance > i32::from(OBJECT_BOLTS_MAX_RANGE)
            || i32::from(feature_id) >= i32::from(MIN_CLOSED_SPACE)
        {
            finished = true;
            continue;
        }

        if creature_id > 1 {
            finished = true;

            let undead = with_state(|state| {
                let monster = &state.monsters[creature_id as usize];
                (CREATURES_LIST[monster.creature_id as usize].defenses & CD_UNDEAD) != 0
            });

            if undead {
                with_state_mut(|state| {
                    let cid = state.monsters[creature_id as usize].creature_id;
                    state.creature_recall[cid as usize].defenses |= CD_UNDEAD;
                });
            } else {
                let name = with_state(|state| {
                    let monster = &state.monsters[creature_id as usize];
                    let creature = &CREATURES_LIST[monster.creature_id as usize];
                    monster_name_description(creature.name, monster.lit)
                });

                let hit_result = monster_take_hit(i32::from(creature_id), 75);
                if hit_result >= 0 {
                    print_monster_action_text(&name, "dies in a fit of agony.");
                    display_character_experience();
                } else {
                    print_monster_action_text(&name, "screams in agony.");
                }

                drained = true;
            }
        }
    }

    drained
}

/// 1279
#[must_use]
pub fn spell_speed_monster(coord: Coord_t, direction: i32, speed: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut changed = false;
    let mut finished = false;

    while !finished {
        let _ = player_move_position(direction, &mut coord);
        distance += 1;

        let (feature_id, creature_id) = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            (tile.feature_id, tile.creature_id)
        });

        if distance > i32::from(OBJECT_BOLTS_MAX_RANGE)
            || i32::from(feature_id) >= i32::from(MIN_CLOSED_SPACE)
        {
            finished = true;
            continue;
        }

        if creature_id > 1 {
            finished = true;

            let (name, creature_level) = with_state(|state| {
                let monster = &state.monsters[creature_id as usize];
                let creature = &CREATURES_LIST[monster.creature_id as usize];
                (
                    monster_name_description(creature.name, monster.lit),
                    creature.level,
                )
            });

            if speed > 0 {
                with_state_mut(|state| {
                    let monster = &mut state.monsters[creature_id as usize];
                    monster.speed += speed as i16;
                    monster.sleep_count = 0;
                });
                changed = true;
                print_monster_action_text(&name, "starts moving faster.");
            } else if random_number(i32::from(MON_MAX_LEVELS)) > i32::from(creature_level) {
                with_state_mut(|state| {
                    let monster = &mut state.monsters[creature_id as usize];
                    monster.speed += speed as i16;
                    monster.sleep_count = 0;
                });
                changed = true;
                print_monster_action_text(&name, "starts moving slower.");
            } else {
                with_state_mut(|state| {
                    state.monsters[creature_id as usize].sleep_count = 0;
                });
                print_monster_action_text(&name, "is unaffected.");
            }
        }
    }

    changed
}

/// 1334
#[must_use]
pub fn spell_confuse_monster(coord: Coord_t, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut confused = false;
    let mut finished = false;

    while !finished {
        let _ = player_move_position(direction, &mut coord);
        distance += 1;

        let (feature_id, creature_id) = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            (tile.feature_id, tile.creature_id)
        });

        if distance > i32::from(OBJECT_BOLTS_MAX_RANGE)
            || i32::from(feature_id) >= i32::from(MIN_CLOSED_SPACE)
        {
            finished = true;
            continue;
        }

        if creature_id > 1 {
            finished = true;

            let (name, creature_level, creature_defenses, recall_creature_id, lit) =
                with_state(|state| {
                    let monster = &state.monsters[creature_id as usize];
                    let creature = &CREATURES_LIST[monster.creature_id as usize];
                    (
                        monster_name_description(creature.name, monster.lit),
                        creature.level,
                        creature.defenses,
                        monster.creature_id,
                        monster.lit,
                    )
                });

            if random_number(i32::from(MON_MAX_LEVELS)) < i32::from(creature_level)
                || (creature_defenses & CD_NO_SLEEP) != 0
            {
                if lit && (creature_defenses & CD_NO_SLEEP) != 0 {
                    with_state_mut(|state| {
                        state.creature_recall[recall_creature_id as usize].defenses |= CD_NO_SLEEP;
                    });
                }

                if (creature_defenses & CD_NO_SLEEP) == 0 {
                    with_state_mut(|state| {
                        state.monsters[creature_id as usize].sleep_count = 0;
                    });
                }

                print_monster_action_text(&name, "is unaffected.");
            } else {
                let roll = with_state_mut(|state| random_number_state(state, 16));
                with_state_mut(|state| {
                    let monster = &mut state.monsters[creature_id as usize];
                    if monster.confused_amount != 0 {
                        monster.confused_amount += 3;
                    } else {
                        monster.confused_amount = (2 + roll) as u8;
                    }
                    monster.sleep_count = 0;
                });
                confused = true;
                print_monster_action_text(&name, "appears confused.");
            }
        }
    }

    confused
}

/// 1378
#[must_use]
pub fn spell_sleep_monster(coord: Coord_t, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut asleep = false;
    let mut finished = false;

    while !finished {
        let _ = player_move_position(direction, &mut coord);
        distance += 1;

        let (feature_id, creature_id) = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            (tile.feature_id, tile.creature_id)
        });

        if distance > i32::from(OBJECT_BOLTS_MAX_RANGE)
            || i32::from(feature_id) >= i32::from(MIN_CLOSED_SPACE)
        {
            finished = true;
            continue;
        }

        if creature_id > 1 {
            finished = true;

            let (name, creature_level, creature_defenses, recall_creature_id, lit) =
                with_state(|state| {
                    let monster = &state.monsters[creature_id as usize];
                    let creature = &CREATURES_LIST[monster.creature_id as usize];
                    (
                        monster_name_description(creature.name, monster.lit),
                        creature.level,
                        creature.defenses,
                        monster.creature_id,
                        monster.lit,
                    )
                });

            if random_number(i32::from(MON_MAX_LEVELS)) < i32::from(creature_level)
                || (creature_defenses & CD_NO_SLEEP) != 0
            {
                if lit && (creature_defenses & CD_NO_SLEEP) != 0 {
                    with_state_mut(|state| {
                        state.creature_recall[recall_creature_id as usize].defenses |= CD_NO_SLEEP;
                    });
                }

                print_monster_action_text(&name, "is unaffected.");
            } else {
                with_state_mut(|state| {
                    state.monsters[creature_id as usize].sleep_count = 500;
                });
                asleep = true;
                print_monster_action_text(&name, "falls asleep.");
            }
        }
    }

    asleep
}

/// 1539
#[must_use]
pub fn spell_polymorph_monster(coord: Coord_t, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut morphed = false;
    let mut finished = false;

    while !finished {
        let _ = player_move_position(direction, &mut coord);
        distance += 1;

        let (feature_id, creature_id, temporary_light, permanent_light) = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            (
                tile.feature_id,
                tile.creature_id,
                tile.temporary_light,
                tile.permanent_light,
            )
        });

        if distance > i32::from(OBJECT_BOLTS_MAX_RANGE)
            || i32::from(feature_id) >= i32::from(MIN_CLOSED_SPACE)
        {
            finished = true;
            continue;
        }

        if creature_id > 1 {
            let creature_level = with_state(|state| {
                let monster = &state.monsters[creature_id as usize];
                CREATURES_LIST[monster.creature_id as usize].level
            });

            if random_number(i32::from(MON_MAX_LEVELS)) > i32::from(creature_level) {
                finished = true;

                dungeon_delete_monster(i32::from(creature_id));

                let level_arg = with_state_mut(|state| {
                    random_number_state(
                        state,
                        i32::from(
                            state.monster_levels[MON_MAX_LEVELS as usize] - state.monster_levels[0],
                        ),
                    ) - 1
                        + i32::from(state.monster_levels[0])
                });
                morphed = monster_place_new(coord, level_arg, false);

                if morphed && coord_inside_panel(coord) && (temporary_light || permanent_light) {
                    morphed = true;
                }
            } else {
                let name = with_state(|state| {
                    let monster = &state.monsters[creature_id as usize];
                    let creature = &CREATURES_LIST[monster.creature_id as usize];
                    monster_name_description(creature.name, monster.lit)
                });
                print_monster_action_text(&name, "is unaffected.");
            }
        }
    }

    morphed
}

/// 1628
#[must_use]
pub fn spell_clone_monster(coord: Coord_t, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut finished = false;

    while !finished {
        let _ = player_move_position(direction, &mut coord);
        distance += 1;

        let (feature_id, creature_id) = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            (tile.feature_id, tile.creature_id)
        });

        if distance > i32::from(OBJECT_BOLTS_MAX_RANGE)
            || i32::from(feature_id) >= i32::from(MIN_CLOSED_SPACE)
        {
            finished = true;
        } else if creature_id > 1 {
            with_state_mut(|state| {
                state.monsters[creature_id as usize].sleep_count = 0;
            });

            let multiply_creature_id =
                with_state(|state| i32::from(state.monsters[creature_id as usize].creature_id));
            return monster_multiply(coord, multiply_creature_id, 0);
        }
    }

    false
}

/// 1730
#[must_use]
pub fn spell_teleport_away_monster_in_direction(coord: Coord_t, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut teleported = false;
    let mut finished = false;

    while !finished {
        let _ = player_move_position(direction, &mut coord);
        distance += 1;

        let (feature_id, creature_id) = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            (tile.feature_id, tile.creature_id)
        });

        if distance > i32::from(OBJECT_BOLTS_MAX_RANGE)
            || i32::from(feature_id) >= i32::from(MIN_CLOSED_SPACE)
        {
            finished = true;
            continue;
        }

        if creature_id > 1 {
            with_state_mut(|state| {
                state.monsters[creature_id as usize].sleep_count = 0;
            });

            spell_teleport_away_monster(i32::from(creature_id), i32::from(MON_MAX_SIGHT));
            teleported = true;
        }
    }

    teleported
}

/// 1748
#[must_use]
pub fn spell_mass_genocide() -> bool {
    let mut killed = false;

    let ids: Vec<i32> = with_state(|state| {
        (i32::from(MON_MIN_INDEX_ID)..i32::from(state.next_free_monster_id))
            .rev()
            .collect()
    });

    for id in ids {
        let should_kill = with_state(|state| {
            let monster = &state.monsters[id as usize];
            let creature = &CREATURES_LIST[monster.creature_id as usize];
            monster.distance_from_player <= MON_MAX_SIGHT && (creature.movement & CM_WIN) == 0
        });

        if should_kill {
            killed = true;
            dungeon_delete_monster(id);
        }
    }

    killed
}

/// 1779
#[must_use]
pub fn spell_genocide() -> bool {
    let mut creature_char = 0u8;
    if !terminal::get_tile_character(
        "Which type of creature do you wish exterminated?",
        &mut creature_char,
    ) {
        return false;
    }

    let mut killed = false;

    let ids: Vec<i32> = with_state(|state| {
        (i32::from(MON_MIN_INDEX_ID)..i32::from(state.next_free_monster_id))
            .rev()
            .collect()
    });

    for id in ids {
        let (sprite, is_win, creature_name) = with_state(|state| {
            let monster = &state.monsters[id as usize];
            let creature = &CREATURES_LIST[monster.creature_id as usize];
            (
                creature.sprite,
                (creature.movement & CM_WIN) != 0,
                creature.name,
            )
        });

        if sprite == creature_char {
            if is_win {
                terminal::print_message(Some(&format!("The {creature_name} is unaffected.")));
            } else {
                killed = true;
                dungeon_delete_monster(id);
            }
        }
    }

    killed
}

/// 1819
#[must_use]
pub fn spell_speed_all_monsters(speed: i32) -> bool {
    let mut speedy = false;

    let ids: Vec<i32> = with_state(|state| {
        (i32::from(MON_MIN_INDEX_ID)..i32::from(state.next_free_monster_id))
            .rev()
            .collect()
    });

    let player_pos = with_state(|state| state.py.pos);

    for id in ids {
        let (distance, pos, lit, creature_level, name) = with_state(|state| {
            let monster = &state.monsters[id as usize];
            let creature = &CREATURES_LIST[monster.creature_id as usize];
            (
                monster.distance_from_player,
                monster.pos,
                monster.lit,
                creature.level,
                monster_name_description(creature.name, monster.lit),
            )
        });

        if distance > MON_MAX_SIGHT || !los(player_pos, pos) {
            continue;
        }

        if speed > 0 {
            with_state_mut(|state| {
                let monster = &mut state.monsters[id as usize];
                monster.speed += speed as i16;
                monster.sleep_count = 0;
            });
            if lit {
                speedy = true;
                print_monster_action_text(&name, "starts moving faster.");
            }
        } else if random_number(i32::from(MON_MAX_LEVELS)) > i32::from(creature_level) {
            with_state_mut(|state| {
                let monster = &mut state.monsters[id as usize];
                monster.speed += speed as i16;
                monster.sleep_count = 0;
            });
            if lit {
                speedy = true;
                print_monster_action_text(&name, "starts moving slower.");
            }
        } else if lit {
            with_state_mut(|state| {
                state.monsters[id as usize].sleep_count = 0;
            });
            print_monster_action_text(&name, "is unaffected.");
        }
    }

    speedy
}

/// 1852
#[must_use]
pub fn spell_sleep_all_monsters() -> bool {
    let mut asleep = false;

    let ids: Vec<i32> = with_state(|state| {
        (i32::from(MON_MIN_INDEX_ID)..i32::from(state.next_free_monster_id))
            .rev()
            .collect()
    });

    let player_pos = with_state(|state| state.py.pos);

    for id in ids {
        let (distance, pos, lit, creature_level, creature_defenses, name, recall_creature_id) =
            with_state(|state| {
                let monster = &state.monsters[id as usize];
                let creature = &CREATURES_LIST[monster.creature_id as usize];
                (
                    monster.distance_from_player,
                    monster.pos,
                    monster.lit,
                    creature.level,
                    creature.defenses,
                    monster_name_description(creature.name, monster.lit),
                    monster.creature_id,
                )
            });

        if distance > MON_MAX_SIGHT || !los(player_pos, pos) {
            continue;
        }

        if random_number(i32::from(MON_MAX_LEVELS)) < i32::from(creature_level)
            || (creature_defenses & CD_NO_SLEEP) != 0
        {
            if lit {
                if (creature_defenses & CD_NO_SLEEP) != 0 {
                    with_state_mut(|state| {
                        state.creature_recall[recall_creature_id as usize].defenses |= CD_NO_SLEEP;
                    });
                }
                print_monster_action_text(&name, "is unaffected.");
            }
        } else {
            with_state_mut(|state| {
                state.monsters[id as usize].sleep_count = 500;
            });
            if lit {
                asleep = true;
                print_monster_action_text(&name, "falls asleep.");
            }
        }
    }

    asleep
}

/// 1878
#[must_use]
pub fn spell_mass_polymorph() -> bool {
    let mut morphed = false;

    let ids: Vec<i32> = with_state(|state| {
        (i32::from(MON_MIN_INDEX_ID)..i32::from(state.next_free_monster_id))
            .rev()
            .collect()
    });

    for id in ids {
        let (distance, pos, is_win) = with_state(|state| {
            let monster = &state.monsters[id as usize];
            let creature = &CREATURES_LIST[monster.creature_id as usize];
            (
                monster.distance_from_player,
                monster.pos,
                (creature.movement & CM_WIN) != 0,
            )
        });

        if distance <= MON_MAX_SIGHT && !is_win {
            dungeon_delete_monster(id);

            let level_arg = with_state_mut(|state| {
                random_number_state(
                    state,
                    i32::from(
                        state.monster_levels[MON_MAX_LEVELS as usize] - state.monster_levels[0],
                    ),
                ) - 1
                    + i32::from(state.monster_levels[0])
            });
            morphed = monster_place_new(pos, level_arg, false);
        }
    }

    morphed
}

/// 2064
#[must_use]
pub fn spell_dispel_creature(creature_defense: i32, damage: i32) -> bool {
    let mut dispelled = false;

    let ids: Vec<i32> = with_state(|state| {
        (i32::from(MON_MIN_INDEX_ID)..i32::from(state.next_free_monster_id))
            .rev()
            .collect()
    });

    let player_pos = with_state(|state| state.py.pos);

    for id in ids {
        let (distance, pos, defenses, lit, creature_name) = with_state(|state| {
            let monster = &state.monsters[id as usize];
            let creature = &CREATURES_LIST[monster.creature_id as usize];
            (
                monster.distance_from_player,
                monster.pos,
                creature.defenses,
                monster.lit,
                creature.name,
            )
        });

        if distance <= MON_MAX_SIGHT
            && (creature_defense & i32::from(defenses)) != 0
            && los(player_pos, pos)
        {
            let recall_creature_id = with_state(|state| state.monsters[id as usize].creature_id);

            with_state_mut(|state| {
                state.creature_recall[recall_creature_id as usize].defenses |=
                    creature_defense as u16;
            });

            dispelled = true;

            let name = monster_name_description(creature_name, lit);
            let hit = monster_take_hit(id, random_number(damage));

            if hit >= 0 {
                print_monster_action_text(&name, "dissolves!");
            } else {
                print_monster_action_text(&name, "shudders.");
            }

            if hit >= 0 {
                display_character_experience();
            }
        }
    }

    dispelled
}

/// 2094
#[must_use]
pub fn spell_turn_undead() -> bool {
    let mut turned = false;

    let ids: Vec<i32> = with_state(|state| {
        (i32::from(MON_MIN_INDEX_ID)..i32::from(state.next_free_monster_id))
            .rev()
            .collect()
    });

    let player_pos = with_state(|state| state.py.pos);

    for id in ids {
        let (distance, pos, defenses, lit, creature_level, name, recall_creature_id) =
            with_state(|state| {
                let monster = &state.monsters[id as usize];
                let creature = &CREATURES_LIST[monster.creature_id as usize];
                (
                    monster.distance_from_player,
                    monster.pos,
                    creature.defenses,
                    monster.lit,
                    creature.level,
                    monster_name_description(creature.name, monster.lit),
                    monster.creature_id,
                )
            });

        if distance <= MON_MAX_SIGHT && (defenses & CD_UNDEAD) != 0 && los(player_pos, pos) {
            let player_level = with_state(|state| state.py.misc.level);

            if i32::from(player_level) + 1 > i32::from(creature_level) || random_number(5) == 1 {
                if lit {
                    with_state_mut(|state| {
                        state.creature_recall[recall_creature_id as usize].defenses |= CD_UNDEAD;
                    });

                    turned = true;
                    print_monster_action_text(&name, "runs frantically!");
                }

                with_state_mut(|state| {
                    state.monsters[id as usize].confused_amount = player_level as u8;
                });
            } else if lit {
                print_monster_action_text(&name, "is unaffected.");
            }
        }
    }

    turned
}

/// 2103
pub fn spell_warding_glyph() {
    let player_pos = with_state(|state| state.py.pos);
    let y = player_pos.y as usize;
    let x = player_pos.x as usize;
    if with_state(|state| state.dg.floor[y][x].treasure_id != 0) {
        return;
    }

    let free_id = popt();
    with_state_mut(|state| {
        state.dg.floor[y][x].treasure_id = free_id as u8;
        inventory_item_copy_to(
            OBJ_SCARE_MON as i16,
            &mut state.game.treasure.list[free_id as usize],
        );
    });
}

fn replace_spot(coord: Coord_t, typ: i32) {
    with_state_mut(|state| {
        let tile = &mut state.dg.floor[coord.y as usize][coord.x as usize];

        match typ {
            1..=3 => tile.feature_id = TILE_CORR_FLOOR,
            4 | 7 | 10 => tile.feature_id = TILE_GRANITE_WALL,
            5 | 8 | 11 => tile.feature_id = TILE_MAGMA_WALL,
            6 | 9 | 12 => tile.feature_id = TILE_QUARTZ_WALL,
            _ => {}
        }

        tile.permanent_light = false;
        tile.field_mark = false;
        tile.perma_lit_room = false;

        if tile.treasure_id != 0 {
            let _ = dungeon_delete_object(coord);
        }

        if tile.creature_id > 1 {
            dungeon_delete_monster(i32::from(tile.creature_id));
        }
    });
}

/// 2284
pub fn spell_destroy_area(coord: Coord_t) {
    if with_state(|state| state.dg.current_level) > 0 {
        for spot_y in coord.y - 15..=coord.y + 15 {
            for spot_x in coord.x - 15..=coord.x + 15 {
                let spot = Coord_t {
                    y: spot_y,
                    x: spot_x,
                };
                if coord_in_bounds(spot)
                    && with_state(|state| {
                        state.dg.floor[spot.y as usize][spot.x as usize].feature_id
                            != TILE_BOUNDARY_WALL
                    })
                {
                    let distance = coord_distance_between(spot, coord);
                    if distance == 0 {
                        replace_spot(spot, 1);
                    } else if distance < 13 {
                        replace_spot(spot, random_number(6));
                    } else if distance < 16 {
                        replace_spot(spot, random_number(9));
                    }
                }
            }
        }
    }

    terminal::print_message(Some("There is a searing blast of light!"));
    with_state_mut(|state| {
        state.py.flags.blind += (10 + random_number_state(state, 10)) as i16;
    });
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Spell {
    pub level_required: u8,
    pub mana_required: u8,
    pub failure_chance: u8,
    pub exp_gain_for_learning: u8,
}
