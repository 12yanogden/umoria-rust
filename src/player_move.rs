//! Port of `src/player_move.cpp` — see `phase_4.4`.

use crate::config::options::prompt_to_pickup;
use crate::config::player::status::PY_SEARCH;
use crate::dice::dice_roll;
use crate::dungeon::{
    dungeon_delete_object, dungeon_light_room, dungeon_move_character_light,
    dungeon_move_creature_record, dungeon_place_random_object_at, dungeon_place_rubble,
    dungeon_set_trap, trap_change_visibility,
};
use crate::dungeon_tile::{MAX_OPEN_SPACE, MIN_CLOSED_SPACE, TILE_LIGHT_FLOOR};
use crate::game::{random_number, with_state, with_state_mut};
use crate::identification::item_description;
use crate::inventory::{
    damage_acid, damage_corroding_gas, damage_fire, damage_poisoned_gas, inventory_can_carry_item,
    inventory_can_carry_item_count, inventory_carry_item, Inventory,
};
use crate::monster_manager::monster_summon;
use crate::player::{
    player_attack_position, player_search, player_takes_hit, player_test_being_hit, PlayerAttr,
    CLASS_MISC_HIT,
};
use crate::player_run::{player_area_affect, player_end_running};
use crate::player_stats::player_stat_random_decrease;
use crate::store::store_enter;
use crate::treasure::{
    TV_CLOSED_DOOR, TV_GOLD, TV_INVIS_TRAP, TV_MAX_PICK_UP, TV_RUBBLE, TV_STORE_DOOR, TV_VIS_TRAP,
};
use crate::types::{Coord_t, Obj_desc_t, Vtype_t, CNIL, MORIA_MESSAGE_SIZE, MORIA_OBJ_DESC_SIZE};
use crate::ui::{coord_outside_panel, draw_dungeon_panel, print_character_gold_value};
use crate::ui_io::terminal::{self, get_input_confirmation};

#[repr(u8)]
enum TrapTypes {
    OpenPit = 1,
    ArrowPit,
    CoveredPit,
    TrapDoor,
    SleepingGas,
    HiddenObject,
    DartOfStr,
    Teleport,
    Rockfall,
    CorrodingGas,
    SummonMonster,
    FireTrap,
    AcidTrap,
    PoisonGasTrap,
    BlindingGas,
    ConfuseGas,
    SlowDart,
    DartOfCon,
    SecretDoor = 19,
    ScareMonster = 99,
    GeneralStore = 101,
    Armory,
    Weaponsmith,
    Temple,
    Alchemist,
    MagicShop,
}

fn obj_desc_as_vtype(description: &Obj_desc_t) -> Vtype_t {
    let mut out = [0u8; MORIA_MESSAGE_SIZE];
    out.copy_from_slice(&description[..MORIA_MESSAGE_SIZE]);
    out
}

fn vtype_label(text: &str) -> Vtype_t {
    let mut buf = [0u8; MORIA_MESSAGE_SIZE];
    let bytes = text.as_bytes();
    let n = bytes.len().min(MORIA_MESSAGE_SIZE - 1);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf
}

fn trap_open_pit(item: Inventory, dam: i32) {
    terminal::print_message(Some("You fell into a pit!"));
    if with_state(|state| state.py.flags.free_fall) {
        terminal::print_message(Some("You gently float down."));
        return;
    }
    let mut description = [0u8; MORIA_OBJ_DESC_SIZE as usize];
    item_description(&mut description, item, true);
    player_takes_hit(dam, &obj_desc_as_vtype(&description));
}

fn trap_arrow(item: Inventory, dam: i32) {
    let ac = with_state(|state| i32::from(state.py.misc.ac) + i32::from(state.py.misc.magical_ac));
    if player_test_being_hit(125, 0, 0, ac, CLASS_MISC_HIT) {
        let mut description = [0u8; MORIA_OBJ_DESC_SIZE as usize];
        item_description(&mut description, item, true);
        player_takes_hit(dam, &obj_desc_as_vtype(&description));
        terminal::print_message(Some("An arrow hits you."));
    } else {
        terminal::print_message(Some("An arrow barely misses you."));
    }
}

fn trap_covered_pit(item: Inventory, dam: i32, coord: Coord_t) {
    terminal::print_message(Some("You fell into a covered pit."));
    if with_state(|state| state.py.flags.free_fall) {
        terminal::print_message(Some("You gently float down."));
    } else {
        let mut description = [0u8; MORIA_OBJ_DESC_SIZE as usize];
        item_description(&mut description, item, true);
        player_takes_hit(dam, &obj_desc_as_vtype(&description));
    }
    dungeon_set_trap(coord, 0);
}

fn trap_door(item: Inventory, dam: i32) {
    with_state_mut(|state| {
        state.dg.generate_new_level = true;
        state.dg.current_level += 1;
    });
    terminal::print_message(Some("You fell through a trap door!"));
    if with_state(|state| state.py.flags.free_fall) {
        terminal::print_message(Some("You gently float down."));
    } else {
        let mut description = [0u8; MORIA_OBJ_DESC_SIZE as usize];
        item_description(&mut description, item, true);
        player_takes_hit(dam, &obj_desc_as_vtype(&description));
    }
    terminal::print_message(CNIL);
}

fn trap_sleeping_gas() {
    if with_state(|state| state.py.flags.paralysis != 0) {
        return;
    }
    terminal::print_message(Some("A strange white mist surrounds you!"));
    if with_state(|state| state.py.flags.free_action) {
        terminal::print_message(Some("You are unaffected."));
        return;
    }
    let added = random_number(10) + 4;
    with_state_mut(|state| {
        state.py.flags.paralysis += added as i16;
    });
    terminal::print_message(Some("You fall asleep."));
}

fn trap_hidden_object(coord: Coord_t) {
    let _ = dungeon_delete_object(coord);
    dungeon_place_random_object_at(coord, false);
    terminal::print_message(Some("Hmmm, there was something under this rock."));
}

fn trap_strength_dart(item: Inventory, dam: i32) {
    let ac = with_state(|state| i32::from(state.py.misc.ac) + i32::from(state.py.misc.magical_ac));
    if player_test_being_hit(125, 0, 0, ac, CLASS_MISC_HIT) {
        if with_state(|state| state.py.flags.sustain_str) {
            terminal::print_message(Some("A small dart hits you."));
        } else {
            let _ = player_stat_random_decrease(PlayerAttr::A_STR);
            let mut description = [0u8; MORIA_OBJ_DESC_SIZE as usize];
            item_description(&mut description, item, true);
            player_takes_hit(dam, &obj_desc_as_vtype(&description));
            terminal::print_message(Some("A small dart weakens you!"));
        }
    } else {
        terminal::print_message(Some("A small dart barely misses you."));
    }
}

fn trap_teleport(coord: Coord_t) {
    with_state_mut(|state| state.game.teleport_player = true);
    terminal::print_message(Some("You hit a teleport trap!"));
    dungeon_move_character_light(coord, coord);
}

fn trap_rockfall(coord: Coord_t, dam: i32) {
    player_takes_hit(dam, &vtype_label("a falling rock"));
    let _ = dungeon_delete_object(coord);
    dungeon_place_rubble(coord);
    terminal::print_message(Some("You are hit by falling rock."));
}

fn trap_corrode_gas() {
    terminal::print_message(Some("A strange red gas surrounds you."));
    damage_corroding_gas(&vtype_label("corrosion gas"));
}

fn trap_summon_monster(coord: Coord_t) {
    let _ = dungeon_delete_object(coord);
    let num = 2 + random_number(3);
    let mut location = coord;
    for _ in 0..num {
        location.y = coord.y;
        location.x = coord.x;
        let _ = monster_summon(&mut location, false);
    }
}

fn trap_fire(dam: i32) {
    terminal::print_message(Some("You are enveloped in flames!"));
    damage_fire(dam, &vtype_label("a fire trap"));
}

fn trap_acid(dam: i32) {
    terminal::print_message(Some("You are splashed with acid!"));
    damage_acid(dam, &vtype_label("an acid trap"));
}

fn trap_poison_gas(dam: i32) {
    terminal::print_message(Some("A pungent green gas surrounds you!"));
    damage_poisoned_gas(dam, &vtype_label("a poison gas trap"));
}

fn trap_blind_gas() {
    terminal::print_message(Some("A black gas surrounds you!"));
    let added = random_number(50) + 50;
    with_state_mut(|state| {
        state.py.flags.blind += added as i16;
    });
}

fn trap_confuse_gas() {
    terminal::print_message(Some("A gas of scintillating colors surrounds you!"));
    let added = random_number(15) + 15;
    with_state_mut(|state| {
        state.py.flags.confused += added as i16;
    });
}

fn trap_slow_dart(item: Inventory, dam: i32) {
    let ac = with_state(|state| i32::from(state.py.misc.ac) + i32::from(state.py.misc.magical_ac));
    if player_test_being_hit(125, 0, 0, ac, CLASS_MISC_HIT) {
        let mut description = [0u8; MORIA_OBJ_DESC_SIZE as usize];
        item_description(&mut description, item, true);
        player_takes_hit(dam, &obj_desc_as_vtype(&description));
        terminal::print_message(Some("A small dart hits you!"));
        if with_state(|state| state.py.flags.free_action) {
            terminal::print_message(Some("You are unaffected."));
        } else {
            let added = random_number(20) + 10;
            with_state_mut(|state| {
                state.py.flags.slow += added as i16;
            });
        }
    } else {
        terminal::print_message(Some("A small dart barely misses you."));
    }
}

fn trap_constitution_dart(item: Inventory, dam: i32) {
    let ac = with_state(|state| i32::from(state.py.misc.ac) + i32::from(state.py.misc.magical_ac));
    if player_test_being_hit(125, 0, 0, ac, CLASS_MISC_HIT) {
        if with_state(|state| state.py.flags.sustain_con) {
            terminal::print_message(Some("A small dart hits you."));
        } else {
            let _ = player_stat_random_decrease(PlayerAttr::A_CON);
            let mut description = [0u8; MORIA_OBJ_DESC_SIZE as usize];
            item_description(&mut description, item, true);
            player_takes_hit(dam, &obj_desc_as_vtype(&description));
            terminal::print_message(Some("A small dart saps your health!"));
        }
    } else {
        terminal::print_message(Some("A small dart barely misses you."));
    }
}

fn player_steps_on_trap(coord: Coord_t) {
    player_end_running();
    trap_change_visibility(coord);

    let item = with_state(|state| {
        let treasure_id = state.dg.floor[coord.y as usize][coord.x as usize].treasure_id;
        state.game.treasure.list[treasure_id as usize]
    });
    let damage = dice_roll(item.damage);

    match item.sub_category_id {
        x if x == TrapTypes::OpenPit as u8 => trap_open_pit(item, damage),
        x if x == TrapTypes::ArrowPit as u8 => trap_arrow(item, damage),
        x if x == TrapTypes::CoveredPit as u8 => trap_covered_pit(item, damage, coord),
        x if x == TrapTypes::TrapDoor as u8 => trap_door(item, damage),
        x if x == TrapTypes::SleepingGas as u8 => trap_sleeping_gas(),
        x if x == TrapTypes::HiddenObject as u8 => trap_hidden_object(coord),
        x if x == TrapTypes::DartOfStr as u8 => trap_strength_dart(item, damage),
        x if x == TrapTypes::Teleport as u8 => trap_teleport(coord),
        x if x == TrapTypes::Rockfall as u8 => trap_rockfall(coord, damage),
        x if x == TrapTypes::CorrodingGas as u8 => trap_corrode_gas(),
        x if x == TrapTypes::SummonMonster as u8 => trap_summon_monster(coord),
        x if x == TrapTypes::FireTrap as u8 => trap_fire(damage),
        x if x == TrapTypes::AcidTrap as u8 => trap_acid(damage),
        x if x == TrapTypes::PoisonGasTrap as u8 => trap_poison_gas(damage),
        x if x == TrapTypes::BlindingGas as u8 => trap_blind_gas(),
        x if x == TrapTypes::ConfuseGas as u8 => trap_confuse_gas(),
        x if x == TrapTypes::SlowDart as u8 => trap_slow_dart(item, damage),
        x if x == TrapTypes::DartOfCon as u8 => trap_constitution_dart(item, damage),
        x if x == TrapTypes::SecretDoor as u8 || x == TrapTypes::ScareMonster as u8 => {}
        x if x == TrapTypes::GeneralStore as u8 => store_enter(0),
        x if x == TrapTypes::Armory as u8 => store_enter(1),
        x if x == TrapTypes::Weaponsmith as u8 => store_enter(2),
        x if x == TrapTypes::Temple as u8 => store_enter(3),
        x if x == TrapTypes::Alchemist as u8 => store_enter(4),
        x if x == TrapTypes::MagicShop as u8 => store_enter(5),
        _ => terminal::print_message(Some("Unknown trap value.")),
    }
}

fn carry(coord: Coord_t, pickup: bool) {
    let (mut pickup, tile_flags, item) = with_state(|state| {
        let treasure_id = state.dg.floor[coord.y as usize][coord.x as usize].treasure_id;
        let item = state.game.treasure.list[treasure_id as usize];
        (pickup, item.category_id, item)
    });

    if tile_flags > TV_MAX_PICK_UP {
        if tile_flags == TV_INVIS_TRAP || tile_flags == TV_VIS_TRAP || tile_flags == TV_STORE_DOOR {
            player_steps_on_trap(coord);
        }
        return;
    }

    let mut description: Obj_desc_t = [0; MORIA_OBJ_DESC_SIZE as usize];

    player_end_running();

    if tile_flags == TV_GOLD {
        with_state_mut(|state| {
            let treasure_id = state.dg.floor[coord.y as usize][coord.x as usize].treasure_id;
            let cost = state.game.treasure.list[treasure_id as usize].cost;
            state.py.misc.au += cost;
        });
        item_description(&mut description, item, true);
        let end = description
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(description.len());
        let desc = String::from_utf8_lossy(&description[..end]);
        let cost = with_state(|state| {
            state.game.treasure.list
                [state.dg.floor[coord.y as usize][coord.x as usize].treasure_id as usize]
                .cost
        });
        let formatted = format!("You have found {cost} gold pieces worth of {desc}");
        print_character_gold_value();
        let _ = dungeon_delete_object(coord);
        terminal::print_message(Some(&formatted));
        return;
    }

    if inventory_can_carry_item_count(item) {
        if pickup && prompt_to_pickup {
            item_description(&mut description, item, true);
            if let Some(last) = description.iter().rposition(|&b| b != 0) {
                description[last] = b'?';
            }
            let end = description
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(description.len());
            let desc = String::from_utf8_lossy(&description[..end]);
            pickup = get_input_confirmation(&format!("Pick up {desc}"));
        }

        if pickup && !inventory_can_carry_item(item) {
            item_description(&mut description, item, true);
            if let Some(last) = description.iter().rposition(|&b| b != 0) {
                description[last] = b'?';
            }
            let end = description
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(description.len());
            let desc = String::from_utf8_lossy(&description[..end]);
            pickup = get_input_confirmation(&format!("Exceed your weight limit to pick up {desc}"));
        }

        if pickup {
            let locn = inventory_carry_item(item);
            let carried = with_state(|state| state.py.inventory[locn as usize]);
            item_description(&mut description, carried, true);
            let end = description
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(description.len());
            let desc = String::from_utf8_lossy(&description[..end]);
            let formatted = format!("You have {desc} ({})", (locn as u8 + b'a') as char);
            terminal::print_message(Some(&formatted));
            let _ = dungeon_delete_object(coord);
        }
    } else {
        item_description(&mut description, item, true);
        let end = description
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(description.len());
        let desc = String::from_utf8_lossy(&description[..end]);
        terminal::print_message(Some(&format!("You can't carry {desc}")));
    }
}

fn player_random_movement(dir: i32) -> bool {
    // Never random if sitting — C++ player_move.cpp lines 339–350.
    // Always consume randomNumber(4) when dir != 5, even if not confused.
    if dir == 5 {
        return false;
    }
    let player_random_move = random_number(4) > 1;
    let player_is_confused = with_state(|state| state.py.flags.confused > 0);
    player_is_confused && player_random_move
}

/// C++ player.cpp lines 49–103 — stepping helper used by streamer walk (4.1.2.1).
pub fn player_move_position(dir: i32, coord: &mut Coord_t) -> bool {
    let new_coord = match dir {
        1 => Coord_t {
            y: coord.y + 1,
            x: coord.x - 1,
        },
        2 => Coord_t {
            y: coord.y + 1,
            x: coord.x,
        },
        3 => Coord_t {
            y: coord.y + 1,
            x: coord.x + 1,
        },
        4 => Coord_t {
            y: coord.y,
            x: coord.x - 1,
        },
        5 => Coord_t {
            y: coord.y,
            x: coord.x,
        },
        6 => Coord_t {
            y: coord.y,
            x: coord.x + 1,
        },
        7 => Coord_t {
            y: coord.y - 1,
            x: coord.x - 1,
        },
        8 => Coord_t {
            y: coord.y - 1,
            x: coord.x,
        },
        9 => Coord_t {
            y: coord.y - 1,
            x: coord.x + 1,
        },
        _ => Coord_t { y: 0, x: 0 },
    };

    with_state_mut(|state| {
        if new_coord.y >= 0
            && new_coord.y < i32::from(state.dg.height)
            && new_coord.x >= 0
            && new_coord.x < i32::from(state.dg.width)
        {
            *coord = new_coord;
            true
        } else {
            false
        }
    })
}

/// C++ `player_move.cpp` lines 426–548.
pub fn player_move(direction: i32, do_pickup: bool) {
    let mut direction = direction;
    if player_random_movement(direction) {
        direction = random_number(9);
        player_end_running();
    }

    let mut coord = with_state(|state| state.py.pos);

    if !player_move_position(direction, &mut coord) {
        return;
    }

    let (creature_id, feature_id, treasure_id, monster_lit) = with_state(|state| {
        let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
        let lit = if tile.creature_id > 1 {
            state.monsters[tile.creature_id as usize].lit
        } else {
            false
        };
        (tile.creature_id, tile.feature_id, tile.treasure_id, lit)
    });

    if creature_id < 2 || (!monster_lit && feature_id >= MIN_CLOSED_SPACE) {
        if feature_id <= MAX_OPEN_SPACE {
            let old_coord = with_state(|state| state.py.pos);
            with_state_mut(|state| {
                state.py.pos = coord;
            });
            dungeon_move_creature_record(old_coord, coord);

            if coord_outside_panel(coord, false) {
                draw_dungeon_panel();
            }

            if with_state(|state| state.py.running_tracker != 0) {
                player_area_affect(direction, coord);
            }

            let (fos, chance, search_on) = with_state(|state| {
                (
                    state.py.misc.fos,
                    state.py.misc.chance_in_search,
                    (state.py.flags.status & PY_SEARCH) != 0,
                )
            });
            if fos <= 1 || random_number(i32::from(fos)) == 1 || search_on {
                player_search(coord, i32::from(chance));
            }

            let (tile_feature, blind, perma_lit_room) = with_state(|state| {
                let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
                (tile.feature_id, state.py.flags.blind, tile.perma_lit_room)
            });

            if tile_feature == TILE_LIGHT_FLOOR {
                let permanent_light = with_state(|state| {
                    state.dg.floor[coord.y as usize][coord.x as usize].permanent_light
                });
                if !permanent_light && blind == 0 {
                    dungeon_light_room(coord);
                }
            } else if perma_lit_room && blind < 1 {
                for row in coord.y - 1..=coord.y + 1 {
                    for col in coord.x - 1..=coord.x + 1 {
                        let needs_light = with_state(|state| {
                            let tile = &state.dg.floor[row as usize][col as usize];
                            tile.feature_id == TILE_LIGHT_FLOOR && !tile.permanent_light
                        });
                        if needs_light {
                            dungeon_light_room(Coord_t { y: row, x: col });
                        }
                    }
                }
            }

            dungeon_move_character_light(old_coord, coord);

            if treasure_id != 0 {
                carry(coord, do_pickup);

                let rubble_category =
                    with_state(|state| state.game.treasure.list[treasure_id as usize].category_id);
                if rubble_category == TV_RUBBLE {
                    with_state_mut(|state| {
                        dungeon_move_creature_record(state.py.pos, old_coord);
                        state.py.pos = old_coord;
                    });
                    dungeon_move_character_light(coord, old_coord);

                    let trap_id = with_state(|state| {
                        state.dg.floor[old_coord.y as usize][old_coord.x as usize].treasure_id
                    });
                    if trap_id != 0 {
                        let val = with_state(|state| {
                            state.game.treasure.list[trap_id as usize].category_id
                        });
                        if val == TV_INVIS_TRAP || val == TV_VIS_TRAP || val == TV_STORE_DOOR {
                            player_steps_on_trap(old_coord);
                        }
                    }
                }
            }
        } else {
            let running = with_state(|state| state.py.running_tracker);
            if running == 0 && treasure_id != 0 {
                let category_id =
                    with_state(|state| state.game.treasure.list[treasure_id as usize].category_id);
                if category_id == TV_RUBBLE {
                    terminal::print_message(Some("There is rubble blocking your way."));
                } else if category_id == TV_CLOSED_DOOR {
                    terminal::print_message(Some("There is a closed door blocking your way."));
                }
            } else {
                player_end_running();
            }
            with_state_mut(|state| state.game.player_free_turn = true);
        }
    } else {
        let old_find_flag = with_state(|state| state.py.running_tracker);
        player_end_running();
        if monster_lit && old_find_flag != 0 {
            with_state_mut(|state| state.game.player_free_turn = true);
        } else {
            player_attack_position(coord);
        }
    }
}
