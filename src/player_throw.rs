//! Port of `src/player_throw.cpp` — see `phase_4.4.6`.

use crate::config::player::status::PY_STR_WGT;
use crate::data_creatures::CREATURES_LIST;
use crate::data_player::CLASS_LEVEL_ADJ;
use crate::dice::dice_roll;
use crate::dungeon::{coord_in_bounds, dungeon_lite_spot};
use crate::dungeon_tile::MAX_OPEN_SPACE;
use crate::game::{get_random_direction, random_number, with_state, with_state_mut};
use crate::game_objects::popt;
use crate::identification::{item_description, item_type_remaining_count_description};
use crate::inventory::{inventory_destroy_item, Inventory, PlayerEquipment};
use crate::monster::monster_take_hit;
use crate::player::{
    player_test_being_hit, player_weapon_critical_blow, PlayerAttr, PlayerClassLevelAdj,
    BTH_PER_PLUS_TO_HIT_ADJUST,
};
use crate::player_magic::item_magic_ability_damage;
use crate::player_move::player_move_position;
use crate::treasure::{TV_ARROW, TV_BOLT, TV_BOW, TV_NOTHING, TV_SLING_AMMO};
use crate::types::{Coord_t, Obj_desc_t, MORIA_OBJ_DESC_SIZE_LEN};
use crate::ui::{coord_inside_panel, display_character_experience};
use crate::ui_inventory::inventory_get_input_for_item_id;
use crate::ui_io::{
    get_direction_with_memory,
    terminal::{self, panel_put_tile, put_qio, Coord},
};

/// C++ `player_throw.cpp` lines 10–23.
fn inventory_throw(item_id: i32, treasure: &mut Inventory) {
    let item = with_state(|state| state.py.inventory[item_id as usize]);
    *treasure = item;

    if item.items_count > 1 {
        with_state_mut(|state| {
            treasure.items_count = 1;
            state.py.inventory[item_id as usize].items_count -= 1;
            state.py.pack.weight -= item.weight as i16;
            state.py.flags.status |= PY_STR_WGT;
        });
    } else {
        inventory_destroy_item(item_id);
    }
}

/// C++ player_throw.cpp lines 26–114.
#[doc(hidden)]
pub fn weapon_missile_facts(
    item: Inventory,
    base_to_hit: &mut i32,
    plus_to_hit: &mut i32,
    damage: &mut i32,
    distance: &mut i32,
) {
    let mut weight = i32::from(item.weight);
    if weight < 1 {
        weight = 1;
    }

    *damage = dice_roll(item.damage) + i32::from(item.to_damage);
    *base_to_hit = with_state(|state| i32::from(state.py.misc.bth_with_bows) * 75 / 100);
    *plus_to_hit =
        with_state(|state| i32::from(state.py.misc.plusses_to_hit) + i32::from(item.to_hit));

    if with_state(|state| state.py.inventory[PlayerEquipment::Wield as usize].category_id)
        != TV_NOTHING
    {
        *plus_to_hit -= with_state(|state| {
            i32::from(state.py.inventory[PlayerEquipment::Wield as usize].to_hit)
        });
    }

    *distance =
        (with_state(|state| i32::from(state.py.stats.used[PlayerAttr::A_STR as usize]) + 20) * 10)
            / weight;
    if *distance > 10 {
        *distance = 10;
    }

    if with_state(|state| state.py.inventory[PlayerEquipment::Wield as usize].category_id) != TV_BOW
    {
        return;
    }

    let bow_misc = with_state(|state| state.py.inventory[PlayerEquipment::Wield as usize].misc_use);

    match bow_misc {
        1 if item.category_id == TV_SLING_AMMO => {
            *base_to_hit = with_state(|state| i32::from(state.py.misc.bth_with_bows));
            *plus_to_hit += 2 * with_state(|state| {
                i32::from(state.py.inventory[PlayerEquipment::Wield as usize].to_hit)
            });
            *damage += with_state(|state| {
                i32::from(state.py.inventory[PlayerEquipment::Wield as usize].to_damage)
            });
            *damage *= 2;
            *distance = 20;
        }
        2 if item.category_id == TV_ARROW => {
            *base_to_hit = with_state(|state| i32::from(state.py.misc.bth_with_bows));
            *plus_to_hit += 2 * with_state(|state| {
                i32::from(state.py.inventory[PlayerEquipment::Wield as usize].to_hit)
            });
            *damage += with_state(|state| {
                i32::from(state.py.inventory[PlayerEquipment::Wield as usize].to_damage)
            });
            *damage *= 2;
            *distance = 25;
        }
        3 if item.category_id == TV_ARROW => {
            *base_to_hit = with_state(|state| i32::from(state.py.misc.bth_with_bows));
            *plus_to_hit += 2 * with_state(|state| {
                i32::from(state.py.inventory[PlayerEquipment::Wield as usize].to_hit)
            });
            *damage += with_state(|state| {
                i32::from(state.py.inventory[PlayerEquipment::Wield as usize].to_damage)
            });
            *damage *= 3;
            *distance = 30;
        }
        4 if item.category_id == TV_ARROW => {
            *base_to_hit = with_state(|state| i32::from(state.py.misc.bth_with_bows));
            *plus_to_hit += 2 * with_state(|state| {
                i32::from(state.py.inventory[PlayerEquipment::Wield as usize].to_hit)
            });
            *damage += with_state(|state| {
                i32::from(state.py.inventory[PlayerEquipment::Wield as usize].to_damage)
            });
            *damage *= 4;
            *distance = 35;
        }
        5 if item.category_id == TV_BOLT => {
            *base_to_hit = with_state(|state| i32::from(state.py.misc.bth_with_bows));
            *plus_to_hit += 2 * with_state(|state| {
                i32::from(state.py.inventory[PlayerEquipment::Wield as usize].to_hit)
            });
            *damage += with_state(|state| {
                i32::from(state.py.inventory[PlayerEquipment::Wield as usize].to_damage)
            });
            *damage *= 3;
            *distance = 25;
        }
        6 if item.category_id == TV_BOLT => {
            *base_to_hit = with_state(|state| i32::from(state.py.misc.bth_with_bows));
            *plus_to_hit += 2 * with_state(|state| {
                i32::from(state.py.inventory[PlayerEquipment::Wield as usize].to_hit)
            });
            *damage += with_state(|state| {
                i32::from(state.py.inventory[PlayerEquipment::Wield as usize].to_damage)
            });
            *damage *= 4;
            *distance = 35;
        }
        _ => {}
    }
}

/// C++ player_throw.cpp lines 116–150.
#[doc(hidden)]
pub fn inventory_drop_or_throw_item(coord: Coord_t, item: Inventory) {
    let mut position = Coord_t {
        y: coord.y,
        x: coord.x,
    };
    let mut flag = false;

    if random_number(10) > 1 {
        let mut k = 0;
        while !flag && k <= 9 {
            if coord_in_bounds(position) {
                let (feature_id, treasure_id) = with_state(|state| {
                    let tile = &state.dg.floor[position.y as usize][position.x as usize];
                    (tile.feature_id, tile.treasure_id)
                });
                if feature_id <= MAX_OPEN_SPACE && treasure_id == 0 {
                    flag = true;
                }
            }

            if !flag {
                position.y = coord.y + random_number(3) - 2;
                position.x = coord.x + random_number(3) - 2;
                k += 1;
            }
        }
    }

    if flag {
        let cur_pos = popt();
        with_state_mut(|state| {
            state.dg.floor[position.y as usize][position.x as usize].treasure_id = cur_pos as u8;
            state.game.treasure.list[cur_pos as usize] = item;
        });
        dungeon_lite_spot(position);
    } else {
        let mut description = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
        let mut msg = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
        item_description(&mut description, item, false);

        let desc = c_str_to_string(&description);
        let formatted = format!("The {desc} disappears.");
        copy_str_into_obj_desc(&mut msg, &formatted);
        terminal::print_message(Some(&formatted));
    }
}

fn c_str_to_string(buf: &Obj_desc_t) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

fn copy_str_into_obj_desc(out: &mut Obj_desc_t, s: &str) {
    let bytes = s.as_bytes();
    let n = bytes.len().min(MORIA_OBJ_DESC_SIZE_LEN - 1);
    out[..n].copy_from_slice(&bytes[..n]);
    out[n] = 0;
}

/// C++ `player_throw.cpp` lines 156–276.
pub fn player_throw_item() {
    if with_state(|state| state.py.pack.unique_items) == 0 {
        terminal::print_message(Some("But you are not carrying anything."));
        with_state_mut(|state| state.game.player_free_turn = true);
        return;
    }

    let mut item_id = 0;
    if !inventory_get_input_for_item_id(
        &mut item_id,
        "Fire/Throw which one?",
        0,
        with_state(|state| i32::from(state.py.pack.unique_items) - 1),
        None,
        None,
    ) {
        return;
    }

    let mut dir = 0;
    if !get_direction_with_memory(None, &mut dir) {
        return;
    }

    item_type_remaining_count_description(item_id);

    if with_state(|state| state.py.flags.confused > 0) {
        terminal::print_message(Some("You are confused."));
        dir = get_random_direction();
    }

    let mut thrown_item = Inventory::default();
    inventory_throw(item_id, &mut thrown_item);

    let mut tbth = 0;
    let mut tpth = 0;
    let mut tdam = 0;
    let mut tdis = 0;
    weapon_missile_facts(thrown_item, &mut tbth, &mut tpth, &mut tdam, &mut tdis);

    let tile_char = thrown_item.sprite;
    let mut visible;
    let mut current_distance = 0;

    let mut coord = with_state(|state| state.py.pos);
    let mut old_coord = coord;
    let mut flag = false;

    while !flag {
        let _ = player_move_position(dir, &mut coord);
        current_distance += 1;
        dungeon_lite_spot(old_coord);

        if current_distance > tdis {
            flag = true;
        }

        let (feature_id, creature_id) = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            (tile.feature_id, tile.creature_id)
        });

        if feature_id <= MAX_OPEN_SPACE && !flag {
            if creature_id > 1 {
                flag = true;

                let (lit, monster_creature_id) = with_state(|state| {
                    let monster = &state.monsters[creature_id as usize];
                    (monster.lit, monster.creature_id)
                });

                tbth -= current_distance;

                if !lit {
                    tbth /= current_distance + 2;
                    tbth -= with_state(|state| {
                        i32::from(state.py.misc.level)
                            * i32::from(
                                CLASS_LEVEL_ADJ[state.py.misc.class_id as usize]
                                    [PlayerClassLevelAdj::BTHB as usize],
                            )
                            / 2
                    });
                    tbth -= tpth * (i32::from(BTH_PER_PLUS_TO_HIT_ADJUST) - 1);
                }

                if player_test_being_hit(
                    tbth,
                    with_state(|state| i32::from(state.py.misc.level)),
                    tpth,
                    i32::from(CREATURES_LIST[monster_creature_id as usize].ac),
                    PlayerClassLevelAdj::BTHB as u8,
                ) {
                    let mut description = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
                    let mut msg = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
                    item_description(&mut description, thrown_item, false);
                    let desc = c_str_to_string(&description);

                    if lit {
                        let creature_name = CREATURES_LIST[monster_creature_id as usize].name;
                        let formatted = format!("The {desc} hits the {creature_name}.");
                        copy_str_into_obj_desc(&mut msg, &formatted);
                        terminal::print_message(Some(&formatted));
                        visible = true;
                    } else {
                        let formatted = format!("You hear a cry as the {desc} finds a mark.");
                        copy_str_into_obj_desc(&mut msg, &formatted);
                        terminal::print_message(Some(&formatted));
                        visible = false;
                    }

                    tdam = item_magic_ability_damage(
                        thrown_item,
                        tdam,
                        i32::from(monster_creature_id),
                    );
                    tdam = player_weapon_critical_blow(
                        i32::from(thrown_item.weight),
                        tpth,
                        tdam,
                        PlayerClassLevelAdj::BTHB as u8,
                    );

                    if tdam < 0 {
                        tdam = 0;
                    }

                    let damage = monster_take_hit(creature_id as i32, tdam);

                    if damage >= 0 {
                        if visible {
                            let creature_name = CREATURES_LIST[damage as usize].name;
                            let formatted = format!("You have killed the {creature_name}.");
                            terminal::print_message(Some(&formatted));
                        } else {
                            terminal::print_message(Some("You have killed something!"));
                        }
                        display_character_experience();
                    }
                } else {
                    inventory_drop_or_throw_item(old_coord, thrown_item);
                }
            } else {
                let (inside_panel, blind, temporary_light, permanent_light) = with_state(|state| {
                    let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
                    (
                        coord_inside_panel(coord),
                        state.py.flags.blind,
                        tile.temporary_light,
                        tile.permanent_light,
                    )
                });

                if inside_panel && blind < 1 && (temporary_light || permanent_light) {
                    panel_put_tile(
                        tile_char,
                        Coord {
                            y: coord.y,
                            x: coord.x,
                        },
                    );
                    put_qio();
                }
            }
        } else {
            flag = true;
            inventory_drop_or_throw_item(old_coord, thrown_item);
        }

        old_coord = coord;
    }
}
