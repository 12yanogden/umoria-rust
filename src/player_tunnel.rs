//! Port of src/player_tunnel.cpp — see phase_4.4.

use crate::config::treasure::flags::TR_TUNNEL;
use crate::dice::max_dice_roll;
use crate::dungeon::{
    cave_tile_visible, dungeon_delete_object, dungeon_lite_spot, dungeon_place_random_object_at,
    MAX_HEIGHT, MAX_WIDTH,
};
use crate::dungeon_tile::{
    MAX_CAVE_ROOM, MIN_CAVE_WALL, TILE_BOUNDARY_WALL, TILE_CORR_FLOOR, TILE_GRANITE_WALL,
    TILE_MAGMA_WALL, TILE_QUARTZ_WALL,
};
use crate::game::{random_number, with_state, with_state_mut};
use crate::identification::object_blocked_by_monster;
use crate::inventory::{Inventory, PlayerEquipment};
use crate::player::{player_attack_position, player_search, PlayerAttr};
use crate::player_move::player_move_position;
use crate::treasure::{TV_NOTHING, TV_RUBBLE, TV_SECRET_DOOR};
use crate::types::Coord_t;
use crate::ui::coord_inside_panel_bounds;
use crate::ui_io::terminal::{self, print_message, print_message_no_command_interrupt};

/// C++ player.cpp lines 1399–1441.
pub fn player_tunnel_wall(coord: Coord_t, digging_ability: i32, digging_chance: i32) -> bool {
    if digging_ability <= digging_chance {
        return false;
    }

    let found_message = with_state_mut(|state| {
        let y = coord.y as usize;
        let x = coord.x as usize;
        let perma_lit_room = state.dg.floor[y][x].perma_lit_room;

        // C++ player.cpp: inner `break` only exits the x-loop; the y-loop continues,
        // so the last qualifying row's first room neighbor wins.
        let (feature_id, permanent_light) = if perma_lit_room {
            let mut found_neighbor = None;
            let mut ny = coord.y - 1;
            while ny <= coord.y + 1 && ny < i32::from(MAX_HEIGHT) {
                let mut nx = coord.x - 1;
                while nx <= coord.x + 1 && nx < i32::from(MAX_WIDTH) {
                    if ny >= 0 && nx >= 0 {
                        let neighbor = &state.dg.floor[ny as usize][nx as usize];
                        if neighbor.feature_id <= MAX_CAVE_ROOM {
                            found_neighbor =
                                Some((neighbor.feature_id, neighbor.permanent_light));
                            break;
                        }
                    }
                    nx += 1;
                }
                ny += 1;
            }
            found_neighbor.unwrap_or((TILE_CORR_FLOOR, false))
        } else {
            (TILE_CORR_FLOOR, false)
        };

        let tile = &mut state.dg.floor[y][x];
        tile.feature_id = feature_id;
        tile.permanent_light = permanent_light;
        tile.field_mark = false;

        coord_inside_panel_bounds(&state.dg.panel, coord)
            && (tile.temporary_light || tile.permanent_light)
            && tile.treasure_id != 0
    });

    if found_message {
        terminal::print_message(Some("You have found something!"));
    }

    dungeon_lite_spot(coord);
    true
}

/// C++ player_tunnel.cpp lines 30–54.
pub fn player_digging_ability(weapon: Inventory) -> i32 {
    let (used_str, weapon_is_heavy) = with_state(|state| {
        (
            state.py.stats.used[PlayerAttr::A_STR as usize],
            state.py.weapon_is_heavy,
        )
    });

    let mut digging_ability = i32::from(used_str);

    if (weapon.flags & TR_TUNNEL) != 0 {
        digging_ability += 25 + i32::from(weapon.misc_use) * 50;
    } else {
        let max_roll = max_dice_roll(weapon.damage);
        digging_ability += max_roll
            + i32::from(weapon.to_hit)
            + i32::from(weapon.to_damage);
        digging_ability >>= 1;
    }

    if weapon_is_heavy {
        digging_ability += i32::from(used_str) * 15 - i32::from(weapon.weight);
        if digging_ability < 0 {
            digging_ability = 0;
        }
    }

    digging_ability
}

fn player_can_tunnel(treasure_id: u8, tile_id: u8) -> bool {
    if i32::from(tile_id) < i32::from(MIN_CAVE_WALL) {
        let illegal = treasure_id == 0
            || with_state(|state| {
                let category_id = state.game.treasure.list[treasure_id as usize].category_id;
                category_id != TV_RUBBLE && category_id != TV_SECRET_DOOR
            });
        if illegal {
            with_state_mut(|state| state.game.player_free_turn = true);
            if treasure_id == 0 {
                print_message(Some("Tunnel through what?  Empty air?!?"));
            } else {
                print_message(Some("You can't tunnel through that."));
            }
            return false;
        }
    }
    true
}

fn dungeon_dig_granite_wall(coord: Coord_t, digging_ability: i32) {
    let digging_chance = random_number(1200) + 80;
    if player_tunnel_wall(coord, digging_ability, digging_chance) {
        print_message(Some("You have finished the tunnel."));
    } else {
        print_message_no_command_interrupt("You tunnel into the granite wall.");
    }
}

fn dungeon_dig_magma_wall(coord: Coord_t, digging_ability: i32) {
    let digging_chance = random_number(600) + 10;
    if player_tunnel_wall(coord, digging_ability, digging_chance) {
        print_message(Some("You have finished the tunnel."));
    } else {
        print_message_no_command_interrupt("You tunnel into the magma intrusion.");
    }
}

fn dungeon_dig_quartz_wall(coord: Coord_t, digging_ability: i32) {
    let digging_chance = random_number(400) + 10;
    if player_tunnel_wall(coord, digging_ability, digging_chance) {
        print_message(Some("You have finished the tunnel."));
    } else {
        print_message_no_command_interrupt("You tunnel into the quartz vein.");
    }
}

fn dungeon_dig_rubble(coord: Coord_t, digging_ability: i32) {
    if digging_ability > random_number(180) {
        let _ = dungeon_delete_object(coord);
        print_message(Some("You have removed the rubble."));

        if random_number(10) == 1 {
            dungeon_place_random_object_at(coord, false);
            if cave_tile_visible(coord) {
                print_message(Some("You have found something!"));
            }
        }

        dungeon_lite_spot(coord);
    } else {
        print_message_no_command_interrupt("You dig in the rubble.");
    }
}

fn dungeon_dig_at_location(coord: Coord_t, wall_type: u8, digging_ability: i32) -> bool {
    match wall_type {
        TILE_GRANITE_WALL => {
            dungeon_dig_granite_wall(coord, digging_ability);
            true
        }
        TILE_MAGMA_WALL => {
            dungeon_dig_magma_wall(coord, digging_ability);
            true
        }
        TILE_QUARTZ_WALL => {
            dungeon_dig_quartz_wall(coord, digging_ability);
            true
        }
        TILE_BOUNDARY_WALL => {
            print_message(Some("This seems to be permanent rock."));
            true
        }
        _ => false,
    }
}

/// C++ player_tunnel.cpp lines 130–176.
pub fn player_tunnel(direction: i32) {
    let mut direction = direction;

    if with_state(|state| state.py.flags.confused > 0) && random_number(4) > 1 {
        direction = random_number(9);
    }

    let mut coord = with_state(|state| state.py.pos);
    let _ = player_move_position(direction, &mut coord);

    let (tile_id, treasure_id, creature_id) = with_state(|state| {
        let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
        (tile.feature_id, tile.treasure_id, tile.creature_id)
    });

    if !player_can_tunnel(treasure_id, tile_id) {
        return;
    }

    if creature_id > 1 {
        object_blocked_by_monster(i32::from(creature_id));
        player_attack_position(coord);
        return;
    }

    let weapon = with_state(|state| state.py.inventory[PlayerEquipment::Wield as usize]);
    if weapon.category_id != TV_NOTHING {
        let digging_ability = player_digging_ability(weapon);

        if !dungeon_dig_at_location(coord, tile_id, digging_ability) {
            if treasure_id != 0 {
                let category_id = with_state(|state| {
                    state.game.treasure.list[treasure_id as usize].category_id
                });
                if category_id == TV_RUBBLE {
                    dungeon_dig_rubble(coord, digging_ability);
                } else if category_id == TV_SECRET_DOOR {
                    print_message_no_command_interrupt("You tunnel into the granite wall.");
                    let chance = with_state(|state| state.py.misc.chance_in_search);
                    let search_coord = with_state(|state| state.py.pos);
                    player_search(search_coord, i32::from(chance));
                } else {
                    panic!("player_tunnel: unexpected treasure category {category_id}");
                }
            } else {
                panic!("player_tunnel: dig failed with no treasure on tile");
            }
        }
        return;
    }

    print_message(Some("You dig with your hands, making no progress."));
}
