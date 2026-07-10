//! Port of src/dungeon_generate.cpp — transient level-generation scratch (not saved).

use std::cell::{Cell, RefCell};

use crate::config::dungeon::{
    objects, DUN_DIR_CHANGE, DUN_MAGMA_STREAMER, DUN_MAGMA_TREASURE, DUN_QUARTZ_STREAMER,
    DUN_QUARTZ_TREASURE, DUN_RANDOM_DIR, DUN_ROOMS_MEAN, DUN_ROOM_DOORS, DUN_STREAMER_DENSITY,
    DUN_STREAMER_WIDTH, DUN_TUNNELING, DUN_TUNNEL_DOORS, DUN_UNUSUAL_ROOMS,
};
use crate::config::monsters::{
    MON_ENDGAME_LEVEL, MON_MIN_PER_LEVEL, MON_MIN_TOWNSFOLK_DAY, MON_MIN_TOWNSFOLK_NIGHT,
};
use crate::config::treasure::MIN_TREASURE_LIST_ID;
use crate::dungeon::{
    coord_corridor_walls_next_to, coord_in_bounds, coord_walls_next_to,
    dungeon_allocate_and_place_object, dungeon_delete_object, dungeon_place_gold,
    dungeon_place_random_object_at, dungeon_place_random_object_near, dungeon_set_trap, MAX_HEIGHT,
    MAX_WIDTH, QUART_HEIGHT, QUART_WIDTH, SCREEN_HEIGHT, SCREEN_WIDTH,
};
use crate::dungeon_tile::{
    MAX_CAVE_FLOOR, MAX_OPEN_SPACE, MIN_CAVE_WALL, MIN_CLOSED_SPACE, TILE_BLOCKED_FLOOR,
    TILE_BOUNDARY_WALL, TILE_CORR_FLOOR, TILE_DARK_FLOOR, TILE_GRANITE_WALL, TILE_LIGHT_FLOOR,
    TILE_MAGMA_WALL, TILE_NULL_WALL, TILE_QUARTZ_WALL, TMP1_WALL, TMP2_WALL,
};
use crate::game::{
    random_number, random_number_normal_distribution, seed_reset_to_old_seed, seed_set, with_state,
    with_state_mut,
};
use crate::game_objects::popt;
use crate::inventory::inventory_item_copy_to;
use crate::monster::BLANK_MONSTER;
use crate::monster_manager::monster_summon;
use crate::monster_manager::{monster_place_new_within_distance, monster_place_winning};
use crate::player_move::player_move_position;
use crate::store::store_maintenance;
use crate::types::Coord_t;

thread_local! {
    static DOORS_TK: RefCell<[Coord_t; 100]> = const { RefCell::new([Coord_t { y: 0, x: 0 }; 100]) };
    static DOOR_INDEX: Cell<i32> = const { Cell::new(0) };
}

pub fn door_index() -> i32 {
    DOOR_INDEX.with(|c| c.get())
}

pub fn reset_door_queue() {
    DOOR_INDEX.with(|c| c.set(0));
    DOORS_TK.with(|q| *q.borrow_mut() = [Coord_t { y: 0, x: 0 }; 100]);
}

pub fn doors_tk_at(i: i32) -> Coord_t {
    DOORS_TK.with(|q| q.borrow()[i as usize])
}

fn push_door_candidate(coord: Coord_t) {
    DOOR_INDEX.with(|idx| {
        let door_index = idx.get();
        if door_index < 100 {
            DOORS_TK.with(|q| {
                q.borrow_mut()[door_index as usize] = coord;
            });
            idx.set(door_index + 1);
        }
    });
}

/// C++ dungeon_generate.cpp lines 14–18.
pub fn dungeon_floor_tile_for_level() -> u8 {
    let level = with_state(|state| state.dg.current_level);
    if i32::from(level) <= random_number(25) {
        TILE_LIGHT_FLOOR
    } else {
        TILE_DARK_FLOOR
    }
}

/// C++ dungeon_generate.cpp lines 22–46.
pub fn pick_correct_direction(start: Coord_t, end: Coord_t) -> (i32, i32) {
    let mut vertical = if start.y < end.y {
        1
    } else if start.y == end.y {
        0
    } else {
        -1
    };

    let mut horizontal = if start.x < end.x {
        1
    } else if start.x == end.x {
        0
    } else {
        -1
    };

    if vertical != 0 && horizontal != 0 {
        if random_number(2) == 1 {
            vertical = 0;
        } else {
            horizontal = 0;
        }
    }

    (vertical, horizontal)
}

/// C++ dungeon_generate.cpp lines 49–58.
pub fn chance_of_random_direction() -> (i32, i32) {
    let direction = random_number(4);

    if direction < 3 {
        (-3 + (direction << 1), 0)
    } else {
        (0, -7 + (direction << 1))
    }
}

/// C++ dungeon_generate.cpp lines 62–64.
pub fn dungeon_blank_entire_cave() {
    with_state_mut(|state| {
        state.dg.floor = [[Default::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
    });
}

/// C++ dungeon_generate.cpp lines 68–79.
#[allow(clippy::explicit_counter_loop)]
pub fn dungeon_fill_empty_tiles_with(rock_type: u8) {
    with_state_mut(|state| {
        for y in (1..state.dg.height - 1).rev() {
            let mut x = 1;
            for _ in 0..state.dg.width - 2 {
                let tile = &mut state.dg.floor[y as usize][x as usize];
                if tile.feature_id == TILE_NULL_WALL
                    || tile.feature_id == TMP1_WALL
                    || tile.feature_id == TMP2_WALL
                {
                    tile.feature_id = rock_type;
                }
                x += 1;
            }
        }
    });
}

/// C++ dungeon_generate.cpp lines 87–122.
pub fn dungeon_place_boundary_walls() {
    with_state_mut(|state| {
        let height = state.dg.height as usize;
        let width = state.dg.width as usize;

        for y in 0..height {
            state.dg.floor[y][0].feature_id = TILE_BOUNDARY_WALL;
            state.dg.floor[y][width - 1].feature_id = TILE_BOUNDARY_WALL;
        }

        for x in 0..width {
            state.dg.floor[0][x].feature_id = TILE_BOUNDARY_WALL;
            state.dg.floor[height - 1][x].feature_id = TILE_BOUNDARY_WALL;
        }
    });
}

/// C++ dungeon_generate.cpp lines 126–161.
pub fn dungeon_place_streamer_rock(rock_type: u8, chance_of_treasure: i32) {
    let (height, width) = with_state(|state| (state.dg.height, state.dg.width));
    let mut coord = Coord_t {
        y: i32::from(height) / 2 + 11 - random_number(23),
        x: i32::from(width) / 2 + 16 - random_number(33),
    };

    let mut dir = random_number(8);
    if dir > 4 {
        dir += 1;
    }

    let t1 = 2 * i32::from(DUN_STREAMER_WIDTH) + 1;
    let t2 = i32::from(DUN_STREAMER_WIDTH) + 1;

    loop {
        for _ in 0..i32::from(DUN_STREAMER_DENSITY) {
            let spot = Coord_t {
                y: coord.y + random_number(t1) - t2,
                x: coord.x + random_number(t1) - t2,
            };

            if coord_in_bounds(spot) {
                let is_granite = with_state(|state| {
                    state.dg.floor[spot.y as usize][spot.x as usize].feature_id == TILE_GRANITE_WALL
                });
                if is_granite {
                    with_state_mut(|state| {
                        state.dg.floor[spot.y as usize][spot.x as usize].feature_id = rock_type;
                    });
                    if random_number(chance_of_treasure) == 1 {
                        dungeon_place_gold(spot);
                    }
                }
            }
        }

        if !player_move_position(dir, &mut coord) {
            break;
        }
    }
}

/// C++ dungeon_generate.cpp lines 163–168.
pub fn dungeon_place_open_door(coord: Coord_t) {
    let cur_pos = popt();
    with_state_mut(|state| {
        state.dg.floor[coord.y as usize][coord.x as usize].treasure_id = cur_pos as u8;
        inventory_item_copy_to(
            objects::OBJ_OPEN_DOOR as i16,
            &mut state.game.treasure.list[cur_pos as usize],
        );
        state.dg.floor[coord.y as usize][coord.x as usize].feature_id = TILE_CORR_FLOOR;
    });
}

/// C++ dungeon_generate.cpp lines 170–176.
pub fn dungeon_place_broken_door(coord: Coord_t) {
    let cur_pos = popt();
    with_state_mut(|state| {
        state.dg.floor[coord.y as usize][coord.x as usize].treasure_id = cur_pos as u8;
        inventory_item_copy_to(
            objects::OBJ_OPEN_DOOR as i16,
            &mut state.game.treasure.list[cur_pos as usize],
        );
        state.dg.floor[coord.y as usize][coord.x as usize].feature_id = TILE_CORR_FLOOR;
        state.game.treasure.list[cur_pos as usize].misc_use = 1;
    });
}

/// C++ dungeon_generate.cpp lines 178–183.
pub fn dungeon_place_closed_door(coord: Coord_t) {
    let cur_pos = popt();
    with_state_mut(|state| {
        state.dg.floor[coord.y as usize][coord.x as usize].treasure_id = cur_pos as u8;
        inventory_item_copy_to(
            objects::OBJ_CLOSED_DOOR as i16,
            &mut state.game.treasure.list[cur_pos as usize],
        );
        state.dg.floor[coord.y as usize][coord.x as usize].feature_id = TILE_BLOCKED_FLOOR;
    });
}

/// C++ dungeon_generate.cpp lines 185–191.
pub fn dungeon_place_locked_door(coord: Coord_t) {
    let cur_pos = popt();
    let misc_use = (random_number(10) + 10) as i16;
    with_state_mut(|state| {
        state.dg.floor[coord.y as usize][coord.x as usize].treasure_id = cur_pos as u8;
        inventory_item_copy_to(
            objects::OBJ_CLOSED_DOOR as i16,
            &mut state.game.treasure.list[cur_pos as usize],
        );
        state.dg.floor[coord.y as usize][coord.x as usize].feature_id = TILE_BLOCKED_FLOOR;
        state.game.treasure.list[cur_pos as usize].misc_use = misc_use;
    });
}

/// C++ dungeon_generate.cpp lines 193–199.
pub fn dungeon_place_stuck_door(coord: Coord_t) {
    let cur_pos = popt();
    let misc_use = (-random_number(10) - 10) as i16;
    with_state_mut(|state| {
        state.dg.floor[coord.y as usize][coord.x as usize].treasure_id = cur_pos as u8;
        inventory_item_copy_to(
            objects::OBJ_CLOSED_DOOR as i16,
            &mut state.game.treasure.list[cur_pos as usize],
        );
        state.dg.floor[coord.y as usize][coord.x as usize].feature_id = TILE_BLOCKED_FLOOR;
        state.game.treasure.list[cur_pos as usize].misc_use = misc_use;
    });
}

/// C++ dungeon_generate.cpp lines 201–206.
pub fn dungeon_place_secret_door(coord: Coord_t) {
    let cur_pos = popt();
    with_state_mut(|state| {
        state.dg.floor[coord.y as usize][coord.x as usize].treasure_id = cur_pos as u8;
        inventory_item_copy_to(
            objects::OBJ_SECRET_DOOR as i16,
            &mut state.game.treasure.list[cur_pos as usize],
        );
        state.dg.floor[coord.y as usize][coord.x as usize].feature_id = TILE_BLOCKED_FLOOR;
    });
}

/// C++ dungeon_generate.cpp lines 208–230.
pub fn dungeon_place_door(coord: Coord_t) {
    let door_type = random_number(3);

    if door_type == 1 {
        if random_number(4) == 1 {
            dungeon_place_broken_door(coord);
        } else {
            dungeon_place_open_door(coord);
        }
    } else if door_type == 2 {
        let sub_type = random_number(12);

        if sub_type > 3 {
            dungeon_place_closed_door(coord);
        } else if sub_type == 3 {
            dungeon_place_stuck_door(coord);
        } else {
            dungeon_place_locked_door(coord);
        }
    } else {
        dungeon_place_secret_door(coord);
    }
}

/// C++ dungeon_generate.cpp lines 233–241.
pub fn dungeon_place_up_stairs(coord: Coord_t) {
    if with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].treasure_id != 0) {
        let _ = dungeon_delete_object(coord);
    }

    let cur_pos = popt();
    with_state_mut(|state| {
        state.dg.floor[coord.y as usize][coord.x as usize].treasure_id = cur_pos as u8;
        inventory_item_copy_to(
            objects::OBJ_UP_STAIR as i16,
            &mut state.game.treasure.list[cur_pos as usize],
        );
    });
}

/// C++ dungeon_generate.cpp lines 244–252.
pub fn dungeon_place_down_stairs(coord: Coord_t) {
    if with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].treasure_id != 0) {
        let _ = dungeon_delete_object(coord);
    }

    let cur_pos = popt();
    with_state_mut(|state| {
        state.dg.floor[coord.y as usize][coord.x as usize].treasure_id = cur_pos as u8;
        inventory_item_copy_to(
            objects::OBJ_DOWN_STAIR as i16,
            &mut state.game.treasure.list[cur_pos as usize],
        );
    });
}

/// C++ dungeon_generate.cpp lines 255–298.
pub fn dungeon_place_stairs(stair_type: i32, number: i32, mut walls: i32) {
    for _ in 0..number {
        let mut placed = false;

        while !placed {
            let mut j = 0;

            while !placed && j <= 30 {
                let (height, width) = with_state(|state| (state.dg.height, state.dg.width));
                let mut coord1 = Coord_t {
                    y: random_number(i32::from(height) - 14),
                    x: random_number(i32::from(width) - 14),
                };
                let coord2 = Coord_t {
                    y: coord1.y + 12,
                    x: coord1.x + 12,
                };

                while coord1.y != coord2.y && !placed {
                    while coord1.x != coord2.x && !placed {
                        let (feature_id, treasure_id) = with_state(|state| {
                            let tile = &state.dg.floor[coord1.y as usize][coord1.x as usize];
                            (tile.feature_id, tile.treasure_id)
                        });

                        if feature_id <= MAX_OPEN_SPACE
                            && treasure_id == 0
                            && coord_walls_next_to(coord1) >= walls
                        {
                            placed = true;
                            if stair_type == 1 {
                                dungeon_place_up_stairs(coord1);
                            } else {
                                dungeon_place_down_stairs(coord1);
                            }
                        }
                        coord1.x += 1;
                    }

                    coord1.x = coord2.x - 12;
                    coord1.y += 1;
                }

                j += 1;
            }

            walls -= 1;
        }
    }
}

/// C++ dungeon_generate.cpp lines 301–317.
pub fn dungeon_place_vault_trap(coord: Coord_t, displacement: Coord_t, number: i32) {
    for _ in 0..number {
        let mut placed = false;

        for count in 0..=5 {
            if placed {
                break;
            }

            let spot = Coord_t {
                y: coord.y - displacement.y - 1 + random_number(2 * displacement.y + 1),
                x: coord.x - displacement.x - 1 + random_number(2 * displacement.x + 1),
            };

            let valid = with_state(|state| {
                let tile = &state.dg.floor[spot.y as usize][spot.x as usize];
                tile.feature_id != TILE_NULL_WALL
                    && tile.feature_id <= MAX_CAVE_FLOOR
                    && tile.treasure_id == 0
            });

            if valid {
                dungeon_set_trap(spot, random_number(i32::from(objects::MAX_TRAPS)) - 1);
                placed = true;
            }

            let _ = count;
        }
    }
}

/// C++ dungeon_generate.cpp lines 320–328.
pub fn dungeon_place_vault_monster(coord: Coord_t, number: i32) {
    for _ in 0..number {
        let mut spot = Coord_t {
            y: coord.y,
            x: coord.x,
        };
        let _ = monster_summon(&mut spot, true);
    }
}

/// C++ dungeon_generate.cpp lines 331–366.
pub fn dungeon_build_room(coord: Coord_t) {
    let floor = dungeon_floor_tile_for_level();
    let height = coord.y - random_number(4);
    let depth = coord.y + random_number(3);
    let left = coord.x - random_number(11);
    let right = coord.x + random_number(11);

    with_state_mut(|state| {
        for y in height..=depth {
            for x in left..=right {
                let tile = &mut state.dg.floor[y as usize][x as usize];
                tile.feature_id = floor;
                tile.perma_lit_room = true;
            }
        }

        for y in (height - 1)..=(depth + 1) {
            state.dg.floor[y as usize][(left - 1) as usize].feature_id = TILE_GRANITE_WALL;
            state.dg.floor[y as usize][(left - 1) as usize].perma_lit_room = true;
            state.dg.floor[y as usize][(right + 1) as usize].feature_id = TILE_GRANITE_WALL;
            state.dg.floor[y as usize][(right + 1) as usize].perma_lit_room = true;
        }

        for x in left..=right {
            state.dg.floor[(height - 1) as usize][x as usize].feature_id = TILE_GRANITE_WALL;
            state.dg.floor[(height - 1) as usize][x as usize].perma_lit_room = true;
            state.dg.floor[(depth + 1) as usize][x as usize].feature_id = TILE_GRANITE_WALL;
            state.dg.floor[(depth + 1) as usize][x as usize].perma_lit_room = true;
        }
    });
}

/// C++ dungeon_generate.cpp lines 370–416.
pub fn dungeon_build_room_overlapping_rectangles(coord: Coord_t) {
    let floor = dungeon_floor_tile_for_level();
    let limit = 1 + random_number(2);

    for _ in 0..limit {
        let height = coord.y - random_number(4);
        let depth = coord.y + random_number(3);
        let left = coord.x - random_number(11);
        let right = coord.x + random_number(11);

        with_state_mut(|state| {
            for y in height..=depth {
                for x in left..=right {
                    let tile = &mut state.dg.floor[y as usize][x as usize];
                    tile.feature_id = floor;
                    tile.perma_lit_room = true;
                }
            }

            for y in (height - 1)..=(depth + 1) {
                if state.dg.floor[y as usize][(left - 1) as usize].feature_id != floor {
                    state.dg.floor[y as usize][(left - 1) as usize].feature_id = TILE_GRANITE_WALL;
                    state.dg.floor[y as usize][(left - 1) as usize].perma_lit_room = true;
                }
                if state.dg.floor[y as usize][(right + 1) as usize].feature_id != floor {
                    state.dg.floor[y as usize][(right + 1) as usize].feature_id = TILE_GRANITE_WALL;
                    state.dg.floor[y as usize][(right + 1) as usize].perma_lit_room = true;
                }
            }

            for x in left..=right {
                if state.dg.floor[(height - 1) as usize][x as usize].feature_id != floor {
                    state.dg.floor[(height - 1) as usize][x as usize].feature_id =
                        TILE_GRANITE_WALL;
                    state.dg.floor[(height - 1) as usize][x as usize].perma_lit_room = true;
                }
                if state.dg.floor[(depth + 1) as usize][x as usize].feature_id != floor {
                    state.dg.floor[(depth + 1) as usize][x as usize].feature_id = TILE_GRANITE_WALL;
                    state.dg.floor[(depth + 1) as usize][x as usize].perma_lit_room = true;
                }
            }
        });
    }
}

/// C++ dungeon_generate.cpp lines 418–433.
pub fn dungeon_place_random_secret_door(
    coord: Coord_t,
    depth: i32,
    height: i32,
    left: i32,
    right: i32,
) {
    match random_number(4) {
        1 => dungeon_place_secret_door(Coord_t {
            y: height - 1,
            x: coord.x,
        }),
        2 => dungeon_place_secret_door(Coord_t {
            y: depth + 1,
            x: coord.x,
        }),
        3 => dungeon_place_secret_door(Coord_t {
            y: coord.y,
            x: left - 1,
        }),
        _ => dungeon_place_secret_door(Coord_t {
            y: coord.y,
            x: right + 1,
        }),
    }
}

/// C++ dungeon_generate.cpp lines 435–443.
pub fn dungeon_place_vault(coord: Coord_t) {
    with_state_mut(|state| {
        for y in (coord.y - 1)..=(coord.y + 1) {
            state.dg.floor[y as usize][(coord.x - 1) as usize].feature_id = TMP1_WALL;
            state.dg.floor[y as usize][(coord.x + 1) as usize].feature_id = TMP1_WALL;
        }
        state.dg.floor[(coord.y - 1) as usize][coord.x as usize].feature_id = TMP1_WALL;
        state.dg.floor[(coord.y + 1) as usize][coord.x as usize].feature_id = TMP1_WALL;
    });
}

/// C++ dungeon_generate.cpp lines 445–457.
pub fn dungeon_place_treasure_vault(
    coord: Coord_t,
    depth: i32,
    height: i32,
    left: i32,
    right: i32,
) {
    dungeon_place_random_secret_door(coord, depth, height, left, right);
    dungeon_place_vault(coord);

    let offset = random_number(4);
    if offset < 3 {
        dungeon_place_locked_door(Coord_t {
            y: coord.y - 3 + (offset << 1),
            x: coord.x,
        });
    } else {
        dungeon_place_locked_door(Coord_t {
            y: coord.y,
            x: coord.x - 7 + (offset << 1),
        });
    }
}

/// C++ dungeon_generate.cpp lines 459–485.
pub fn dungeon_place_inner_pillars(coord: Coord_t) {
    with_state_mut(|state| {
        for y in (coord.y - 1)..=(coord.y + 1) {
            for x in (coord.x - 1)..=(coord.x + 1) {
                state.dg.floor[y as usize][x as usize].feature_id = TMP1_WALL;
            }
        }
    });

    if random_number(2) != 1 {
        return;
    }

    let offset = random_number(2);

    with_state_mut(|state| {
        for y in (coord.y - 1)..=(coord.y + 1) {
            for x in (coord.x - 5 - offset)..=(coord.x - 3 - offset) {
                state.dg.floor[y as usize][x as usize].feature_id = TMP1_WALL;
            }
        }

        for y in (coord.y - 1)..=(coord.y + 1) {
            for x in (coord.x + 3 + offset)..=(coord.x + 5 + offset) {
                state.dg.floor[y as usize][x as usize].feature_id = TMP1_WALL;
            }
        }
    });
}

/// C++ dungeon_generate.cpp lines 487–495.
pub fn dungeon_place_maze_inside_room(depth: i32, height: i32, left: i32, right: i32) {
    with_state_mut(|state| {
        for y in height..=depth {
            for x in left..=right {
                if (0x1 & (x + y)) != 0 {
                    state.dg.floor[y as usize][x as usize].feature_id = TMP1_WALL;
                }
            }
        }
    });
}

/// C++ dungeon_generate.cpp lines 497–520.
pub fn dungeon_place_four_small_rooms(
    coord: Coord_t,
    depth: i32,
    height: i32,
    left: i32,
    right: i32,
) {
    with_state_mut(|state| {
        for y in height..=depth {
            state.dg.floor[y as usize][coord.x as usize].feature_id = TMP1_WALL;
        }

        for x in left..=right {
            state.dg.floor[coord.y as usize][x as usize].feature_id = TMP1_WALL;
        }
    });

    if random_number(2) == 1 {
        let offset = random_number(10);
        dungeon_place_secret_door(Coord_t {
            y: height - 1,
            x: coord.x - offset,
        });
        dungeon_place_secret_door(Coord_t {
            y: height - 1,
            x: coord.x + offset,
        });
        dungeon_place_secret_door(Coord_t {
            y: depth + 1,
            x: coord.x - offset,
        });
        dungeon_place_secret_door(Coord_t {
            y: depth + 1,
            x: coord.x + offset,
        });
    } else {
        let offset = random_number(3);
        dungeon_place_secret_door(Coord_t {
            y: coord.y + offset,
            x: left - 1,
        });
        dungeon_place_secret_door(Coord_t {
            y: coord.y - offset,
            x: left - 1,
        });
        dungeon_place_secret_door(Coord_t {
            y: coord.y + offset,
            x: right + 1,
        });
        dungeon_place_secret_door(Coord_t {
            y: coord.y - offset,
            x: right + 1,
        });
    }
}

/// C++ dungeon_generate.cpp lines 537–667.
pub fn dungeon_build_room_with_inner_rooms(coord: Coord_t) {
    let floor = dungeon_floor_tile_for_level();

    let mut height = coord.y - 4;
    let mut depth = coord.y + 4;
    let mut left = coord.x - 11;
    let mut right = coord.x + 11;

    with_state_mut(|state| {
        for i in height..=depth {
            for j in left..=right {
                let tile = &mut state.dg.floor[i as usize][j as usize];
                tile.feature_id = floor;
                tile.perma_lit_room = true;
            }
        }

        for i in (height - 1)..=(depth + 1) {
            state.dg.floor[i as usize][(left - 1) as usize].feature_id = TILE_GRANITE_WALL;
            state.dg.floor[i as usize][(left - 1) as usize].perma_lit_room = true;
            state.dg.floor[i as usize][(right + 1) as usize].feature_id = TILE_GRANITE_WALL;
            state.dg.floor[i as usize][(right + 1) as usize].perma_lit_room = true;
        }

        for i in left..=right {
            state.dg.floor[(height - 1) as usize][i as usize].feature_id = TILE_GRANITE_WALL;
            state.dg.floor[(height - 1) as usize][i as usize].perma_lit_room = true;
            state.dg.floor[(depth + 1) as usize][i as usize].feature_id = TILE_GRANITE_WALL;
            state.dg.floor[(depth + 1) as usize][i as usize].perma_lit_room = true;
        }
    });

    height += 2;
    depth -= 2;
    left += 2;
    right -= 2;

    with_state_mut(|state| {
        for i in (height - 1)..=(depth + 1) {
            state.dg.floor[i as usize][(left - 1) as usize].feature_id = TMP1_WALL;
            state.dg.floor[i as usize][(right + 1) as usize].feature_id = TMP1_WALL;
        }

        for i in left..=right {
            state.dg.floor[(height - 1) as usize][i as usize].feature_id = TMP1_WALL;
            state.dg.floor[(depth + 1) as usize][i as usize].feature_id = TMP1_WALL;
        }
    });

    match random_number(5) {
        1 => {
            dungeon_place_random_secret_door(coord, depth, height, left, right);
            dungeon_place_vault_monster(coord, 1);
        }
        2 => {
            dungeon_place_treasure_vault(coord, depth, height, left, right);
            dungeon_place_vault_monster(coord, 2 + random_number(3));
            dungeon_place_vault_trap(coord, Coord_t { y: 4, x: 10 }, 2 + random_number(3));
        }
        3 => {
            dungeon_place_random_secret_door(coord, depth, height, left, right);
            dungeon_place_inner_pillars(coord);

            if random_number(3) == 1 {
                with_state_mut(|state| {
                    for i in (coord.x - 5)..=(coord.x + 5) {
                        state.dg.floor[(coord.y - 1) as usize][i as usize].feature_id = TMP1_WALL;
                        state.dg.floor[(coord.y + 1) as usize][i as usize].feature_id = TMP1_WALL;
                    }
                    state.dg.floor[coord.y as usize][(coord.x - 5) as usize].feature_id = TMP1_WALL;
                    state.dg.floor[coord.y as usize][(coord.x + 5) as usize].feature_id = TMP1_WALL;
                });

                dungeon_place_secret_door(Coord_t {
                    y: coord.y - 3 + (random_number(2) << 1),
                    x: coord.x - 3,
                });
                dungeon_place_secret_door(Coord_t {
                    y: coord.y - 3 + (random_number(2) << 1),
                    x: coord.x + 3,
                });

                if random_number(3) == 1 {
                    dungeon_place_random_object_at(
                        Coord_t {
                            y: coord.y,
                            x: coord.x - 2,
                        },
                        false,
                    );
                }

                if random_number(3) == 1 {
                    dungeon_place_random_object_at(
                        Coord_t {
                            y: coord.y,
                            x: coord.x + 2,
                        },
                        false,
                    );
                }

                dungeon_place_vault_monster(
                    Coord_t {
                        y: coord.y,
                        x: coord.x - 2,
                    },
                    random_number(2),
                );
                dungeon_place_vault_monster(
                    Coord_t {
                        y: coord.y,
                        x: coord.x + 2,
                    },
                    random_number(2),
                );
            }
        }
        4 => {
            dungeon_place_random_secret_door(coord, depth, height, left, right);
            dungeon_place_maze_inside_room(depth, height, left, right);
            dungeon_place_vault_monster(
                Coord_t {
                    y: coord.y,
                    x: coord.x - 5,
                },
                random_number(3),
            );
            dungeon_place_vault_monster(
                Coord_t {
                    y: coord.y,
                    x: coord.x + 5,
                },
                random_number(3),
            );
            dungeon_place_vault_trap(
                Coord_t {
                    y: coord.y,
                    x: coord.x - 3,
                },
                Coord_t { y: 2, x: 8 },
                random_number(3),
            );
            dungeon_place_vault_trap(
                Coord_t {
                    y: coord.y,
                    x: coord.x + 3,
                },
                Coord_t { y: 2, x: 8 },
                random_number(3),
            );
            for _ in 0..3 {
                dungeon_place_random_object_near(coord, 1);
            }
        }
        _ => {
            dungeon_place_four_small_rooms(coord, depth, height, left, right);
            dungeon_place_random_object_near(coord, 2 + random_number(2));
            dungeon_place_vault_monster(
                Coord_t {
                    y: coord.y + 2,
                    x: coord.x - 4,
                },
                random_number(2),
            );
            dungeon_place_vault_monster(
                Coord_t {
                    y: coord.y + 2,
                    x: coord.x + 4,
                },
                random_number(2),
            );
            dungeon_place_vault_monster(
                Coord_t {
                    y: coord.y - 2,
                    x: coord.x - 4,
                },
                random_number(2),
            );
            dungeon_place_vault_monster(
                Coord_t {
                    y: coord.y - 2,
                    x: coord.x + 4,
                },
                random_number(2),
            );
        }
    }
}

/// C++ dungeon_generate.cpp lines 669–675.
pub fn dungeon_place_large_middle_pillar(coord: Coord_t) {
    with_state_mut(|state| {
        for y in (coord.y - 1)..=(coord.y + 1) {
            for x in (coord.x - 1)..=(coord.x + 1) {
                state.dg.floor[y as usize][x as usize].feature_id = TMP1_WALL;
            }
        }
    });
}

/// C++ dungeon_generate.cpp lines 679–808.
pub fn dungeon_build_room_cross_shaped(coord: Coord_t) {
    let floor = dungeon_floor_tile_for_level();

    let mut random_offset = 2 + random_number(2);
    let mut height = coord.y - random_offset;
    let mut depth = coord.y + random_offset;
    let mut left = coord.x - 1;
    let mut right = coord.x + 1;

    with_state_mut(|state| {
        for i in height..=depth {
            for j in left..=right {
                let tile = &mut state.dg.floor[i as usize][j as usize];
                tile.feature_id = floor;
                tile.perma_lit_room = true;
            }
        }

        for i in (height - 1)..=(depth + 1) {
            state.dg.floor[i as usize][(left - 1) as usize].feature_id = TILE_GRANITE_WALL;
            state.dg.floor[i as usize][(left - 1) as usize].perma_lit_room = true;
            state.dg.floor[i as usize][(right + 1) as usize].feature_id = TILE_GRANITE_WALL;
            state.dg.floor[i as usize][(right + 1) as usize].perma_lit_room = true;
        }

        for i in left..=right {
            state.dg.floor[(height - 1) as usize][i as usize].feature_id = TILE_GRANITE_WALL;
            state.dg.floor[(height - 1) as usize][i as usize].perma_lit_room = true;
            state.dg.floor[(depth + 1) as usize][i as usize].feature_id = TILE_GRANITE_WALL;
            state.dg.floor[(depth + 1) as usize][i as usize].perma_lit_room = true;
        }
    });

    random_offset = 2 + random_number(9);
    height = coord.y - 1;
    depth = coord.y + 1;
    left = coord.x - random_offset;
    right = coord.x + random_offset;

    with_state_mut(|state| {
        for i in height..=depth {
            for j in left..=right {
                let tile = &mut state.dg.floor[i as usize][j as usize];
                tile.feature_id = floor;
                tile.perma_lit_room = true;
            }
        }

        for i in (height - 1)..=(depth + 1) {
            if state.dg.floor[i as usize][(left - 1) as usize].feature_id != floor {
                state.dg.floor[i as usize][(left - 1) as usize].feature_id = TILE_GRANITE_WALL;
                state.dg.floor[i as usize][(left - 1) as usize].perma_lit_room = true;
            }
            if state.dg.floor[i as usize][(right + 1) as usize].feature_id != floor {
                state.dg.floor[i as usize][(right + 1) as usize].feature_id = TILE_GRANITE_WALL;
                state.dg.floor[i as usize][(right + 1) as usize].perma_lit_room = true;
            }
        }

        for i in left..=right {
            if state.dg.floor[(height - 1) as usize][i as usize].feature_id != floor {
                state.dg.floor[(height - 1) as usize][i as usize].feature_id = TILE_GRANITE_WALL;
                state.dg.floor[(height - 1) as usize][i as usize].perma_lit_room = true;
            }
            if state.dg.floor[(depth + 1) as usize][i as usize].feature_id != floor {
                state.dg.floor[(depth + 1) as usize][i as usize].feature_id = TILE_GRANITE_WALL;
                state.dg.floor[(depth + 1) as usize][i as usize].perma_lit_room = true;
            }
        }
    });

    match random_number(4) {
        1 => dungeon_place_large_middle_pillar(coord),
        2 => {
            dungeon_place_vault(coord);
            random_offset = random_number(4);
            if random_offset < 3 {
                dungeon_place_secret_door(Coord_t {
                    y: coord.y - 3 + (random_offset << 1),
                    x: coord.x,
                });
            } else {
                dungeon_place_secret_door(Coord_t {
                    y: coord.y,
                    x: coord.x - 7 + (random_offset << 1),
                });
            }
            dungeon_place_random_object_at(coord, false);
            dungeon_place_vault_monster(coord, 2 + random_number(2));
            dungeon_place_vault_trap(coord, Coord_t { y: 4, x: 4 }, 1 + random_number(3));
        }
        3 => {
            if random_number(3) == 1 {
                with_state_mut(|state| {
                    state.dg.floor[(coord.y - 1) as usize][(coord.x - 2) as usize].feature_id =
                        TMP1_WALL;
                    state.dg.floor[(coord.y + 1) as usize][(coord.x - 2) as usize].feature_id =
                        TMP1_WALL;
                    state.dg.floor[(coord.y - 1) as usize][(coord.x + 2) as usize].feature_id =
                        TMP1_WALL;
                    state.dg.floor[(coord.y + 1) as usize][(coord.x + 2) as usize].feature_id =
                        TMP1_WALL;
                    state.dg.floor[(coord.y - 2) as usize][(coord.x - 1) as usize].feature_id =
                        TMP1_WALL;
                    state.dg.floor[(coord.y - 2) as usize][(coord.x + 1) as usize].feature_id =
                        TMP1_WALL;
                    state.dg.floor[(coord.y + 2) as usize][(coord.x - 1) as usize].feature_id =
                        TMP1_WALL;
                    state.dg.floor[(coord.y + 2) as usize][(coord.x + 1) as usize].feature_id =
                        TMP1_WALL;
                });

                if random_number(3) == 1 {
                    dungeon_place_secret_door(Coord_t {
                        y: coord.y,
                        x: coord.x - 2,
                    });
                    dungeon_place_secret_door(Coord_t {
                        y: coord.y,
                        x: coord.x + 2,
                    });
                    dungeon_place_secret_door(Coord_t {
                        y: coord.y - 2,
                        x: coord.x,
                    });
                    dungeon_place_secret_door(Coord_t {
                        y: coord.y + 2,
                        x: coord.x,
                    });
                }
            } else if random_number(3) == 1 {
                with_state_mut(|state| {
                    state.dg.floor[coord.y as usize][coord.x as usize].feature_id = TMP1_WALL;
                    state.dg.floor[(coord.y - 1) as usize][coord.x as usize].feature_id = TMP1_WALL;
                    state.dg.floor[(coord.y + 1) as usize][coord.x as usize].feature_id = TMP1_WALL;
                    state.dg.floor[coord.y as usize][(coord.x - 1) as usize].feature_id = TMP1_WALL;
                    state.dg.floor[coord.y as usize][(coord.x + 1) as usize].feature_id = TMP1_WALL;
                });
            } else if random_number(3) == 1 {
                with_state_mut(|state| {
                    state.dg.floor[coord.y as usize][coord.x as usize].feature_id = TMP1_WALL;
                });
            }
        }
        _ => {}
    }
}

/// C++ dungeon_generate.cpp lines 811–946.
pub fn dungeon_build_tunnel(start: Coord_t, end: Coord_t) {
    let mut tunnels_tk = [Coord_t { y: 0, x: 0 }; 1000];
    let mut walls_tk = [Coord_t { y: 0, x: 0 }; 1000];
    let mut door_flag = false;
    let mut stop_flag = false;
    let mut main_loop_count = 0;
    let start_row = start.y;
    let start_col = start.x;
    let mut tunnel_index = 0;
    let mut wall_index = 0;
    let mut start = start;
    let (mut y_direction, mut x_direction) = pick_correct_direction(start, end);

    loop {
        main_loop_count += 1;
        if main_loop_count > 2000 {
            stop_flag = true;
        }

        if random_number(100) > i32::from(DUN_DIR_CHANGE) {
            if random_number(i32::from(DUN_RANDOM_DIR)) == 1 {
                (y_direction, x_direction) = chance_of_random_direction();
            } else {
                (y_direction, x_direction) = pick_correct_direction(start, end);
            }
        }

        let mut tmp_row = start.y + y_direction;
        let mut tmp_col = start.x + x_direction;

        while !coord_in_bounds(Coord_t {
            y: tmp_row,
            x: tmp_col,
        }) {
            if random_number(i32::from(DUN_RANDOM_DIR)) == 1 {
                (y_direction, x_direction) = chance_of_random_direction();
            } else {
                (y_direction, x_direction) = pick_correct_direction(start, end);
            }
            tmp_row = start.y + y_direction;
            tmp_col = start.x + x_direction;
        }

        let feature_id =
            with_state(|state| state.dg.floor[tmp_row as usize][tmp_col as usize].feature_id);

        match feature_id {
            TILE_NULL_WALL => {
                start.y = tmp_row;
                start.x = tmp_col;
                if tunnel_index < 1000 {
                    tunnels_tk[tunnel_index as usize] = start;
                    tunnel_index += 1;
                }
                door_flag = false;
            }
            TMP2_WALL => {}
            TILE_GRANITE_WALL => {
                start.y = tmp_row;
                start.x = tmp_col;

                if wall_index < 1000 {
                    walls_tk[wall_index as usize] = start;
                    wall_index += 1;
                }

                with_state_mut(|state| {
                    let height = i32::from(state.dg.height);
                    let width = i32::from(state.dg.width);
                    for y in start.y - 1..=start.y + 1 {
                        for x in start.x - 1..=start.x + 1 {
                            if y > 0 && y < height - 1 && x > 0 && x < width - 1 {
                                let tile = &mut state.dg.floor[y as usize][x as usize];
                                if tile.feature_id == TILE_GRANITE_WALL {
                                    tile.feature_id = TMP2_WALL;
                                }
                            }
                        }
                    }
                });
            }
            TILE_CORR_FLOOR | TILE_BLOCKED_FLOOR => {
                start.y = tmp_row;
                start.x = tmp_col;

                if !door_flag {
                    push_door_candidate(start);
                    door_flag = true;
                }

                if random_number(100) > i32::from(DUN_TUNNELING) {
                    tmp_row = start.y - start_row;
                    if tmp_row < 0 {
                        tmp_row = -tmp_row;
                    }

                    tmp_col = start.x - start_col;
                    if tmp_col < 0 {
                        tmp_col = -tmp_col;
                    }

                    if tmp_row > 10 || tmp_col > 10 {
                        stop_flag = true;
                    }
                }
            }
            _ => {
                start.y = tmp_row;
                start.x = tmp_col;
            }
        }

        if (start.y == end.y && start.x == end.x) || stop_flag {
            break;
        }
    }

    with_state_mut(|state| {
        for i in 0..tunnel_index {
            let coord = tunnels_tk[i as usize];
            state.dg.floor[coord.y as usize][coord.x as usize].feature_id = TILE_CORR_FLOOR;
        }
    });

    for i in 0..wall_index {
        let coord = walls_tk[i as usize];
        let is_tmp2 = with_state(|state| {
            state.dg.floor[coord.y as usize][coord.x as usize].feature_id == TMP2_WALL
        });
        if is_tmp2 {
            if random_number(100) < i32::from(DUN_ROOM_DOORS) {
                dungeon_place_door(coord);
            } else {
                with_state_mut(|state| {
                    state.dg.floor[coord.y as usize][coord.x as usize].feature_id = TILE_CORR_FLOOR;
                });
            }
        }
    }
}

/// C++ dungeon_generate.cpp lines 948–957.
#[must_use]
pub fn dungeon_is_next_to(coord: Coord_t) -> bool {
    if coord_corridor_walls_next_to(coord) > 2 {
        let vertical = with_state(|state| {
            state.dg.floor[(coord.y - 1) as usize][coord.x as usize].feature_id >= MIN_CAVE_WALL
                && state.dg.floor[(coord.y + 1) as usize][coord.x as usize].feature_id
                    >= MIN_CAVE_WALL
        });
        let horizontal = with_state(|state| {
            state.dg.floor[coord.y as usize][(coord.x - 1) as usize].feature_id >= MIN_CAVE_WALL
                && state.dg.floor[coord.y as usize][(coord.x + 1) as usize].feature_id
                    >= MIN_CAVE_WALL
        });
        vertical || horizontal
    } else {
        false
    }
}

/// C++ dungeon_generate.cpp lines 960–964.
pub fn dungeon_place_door_if_next_to_two_walls(coord: Coord_t) {
    let is_corr = with_state(|state| {
        state.dg.floor[coord.y as usize][coord.x as usize].feature_id == TILE_CORR_FLOOR
    });
    if is_corr && random_number(100) > i32::from(DUN_TUNNEL_DOORS) && dungeon_is_next_to(coord) {
        dungeon_place_door(coord);
    }
}

/// C++ dungeon_generate.cpp lines 967–979.
pub fn dungeon_new_spot() -> Coord_t {
    loop {
        let (height, width) = with_state(|state| (state.dg.height, state.dg.width));
        let position = Coord_t {
            y: random_number(i32::from(height) - 2),
            x: random_number(i32::from(width) - 2),
        };

        let valid = with_state(|state| {
            let tile = &state.dg.floor[position.y as usize][position.x as usize];
            i32::from(tile.feature_id) < i32::from(MIN_CLOSED_SPACE)
                && tile.creature_id == 0
                && tile.treasure_id == 0
        });

        if valid {
            return position;
        }
    }
}

/// C++ dungeon_generate.cpp lines 982–984.
pub fn set_rooms(tile_id: i32) -> bool {
    tile_id == i32::from(TILE_DARK_FLOOR) || tile_id == i32::from(TILE_LIGHT_FLOOR)
}

/// C++ dungeon_generate.cpp lines 986–988.
pub fn set_corridors(tile_id: i32) -> bool {
    tile_id == i32::from(TILE_CORR_FLOOR) || tile_id == i32::from(TILE_BLOCKED_FLOOR)
}

/// C++ dungeon_generate.cpp lines 990–992.
pub fn set_floors(tile_id: i32) -> bool {
    tile_id <= i32::from(MAX_CAVE_FLOOR)
}

/// C++ dungeon_generate.cpp lines 995–1105.
pub fn dungeon_generate() {
    let (height, width, current_level) =
        with_state(|state| (state.dg.height, state.dg.width, state.dg.current_level));

    let row_rooms = 2 * (i32::from(height) / i32::from(SCREEN_HEIGHT));
    let col_rooms = 2 * (i32::from(width) / i32::from(SCREEN_WIDTH));

    let mut room_map = [[false; 20]; 20];
    for row in room_map.iter_mut().take(row_rooms as usize) {
        for col in row.iter_mut().take(col_rooms as usize) {
            *col = false;
        }
    }

    let random_room_count = random_number_normal_distribution(i32::from(DUN_ROOMS_MEAN), 2);
    for _ in 0..random_room_count {
        let row = random_number(row_rooms) - 1;
        let col = random_number(col_rooms) - 1;
        room_map[row as usize][col as usize] = true;
    }

    let mut location_id = 0i32;
    let mut locations = [Coord_t { y: 0, x: 0 }; 400];

    for row in 0..row_rooms {
        for col in 0..col_rooms {
            if room_map[row as usize][col as usize] {
                locations[location_id as usize] = Coord_t {
                    y: row * (i32::from(SCREEN_HEIGHT) >> 1) + i32::from(QUART_HEIGHT),
                    x: col * (i32::from(SCREEN_WIDTH) >> 1) + i32::from(QUART_WIDTH),
                };
                if i32::from(current_level) > random_number(i32::from(DUN_UNUSUAL_ROOMS)) {
                    match random_number(3) {
                        1 => dungeon_build_room_overlapping_rectangles(
                            locations[location_id as usize],
                        ),
                        2 => dungeon_build_room_with_inner_rooms(locations[location_id as usize]),
                        _ => dungeon_build_room_cross_shaped(locations[location_id as usize]),
                    }
                } else {
                    dungeon_build_room(locations[location_id as usize]);
                }
                location_id += 1;
            }
        }
    }

    for _ in 0..location_id {
        let pick1 = random_number(location_id) - 1;
        let pick2 = random_number(location_id) - 1;

        let y = locations[pick1 as usize].y;
        let x = locations[pick1 as usize].x;
        locations[pick1 as usize].y = locations[pick2 as usize].y;
        locations[pick1 as usize].x = locations[pick2 as usize].x;
        locations[pick2 as usize].y = y;
        locations[pick2 as usize].x = x;
    }

    reset_door_queue();

    locations[location_id as usize] = locations[0];
    for i in 0..location_id {
        dungeon_build_tunnel(locations[(i + 1) as usize], locations[i as usize]);
    }

    dungeon_fill_empty_tiles_with(TILE_GRANITE_WALL);
    for _ in 0..i32::from(DUN_MAGMA_STREAMER) {
        dungeon_place_streamer_rock(TILE_MAGMA_WALL, i32::from(DUN_MAGMA_TREASURE));
    }
    for _ in 0..i32::from(DUN_QUARTZ_STREAMER) {
        dungeon_place_streamer_rock(TILE_QUARTZ_WALL, i32::from(DUN_QUARTZ_TREASURE));
    }
    dungeon_place_boundary_walls();

    let door_count = door_index();
    for i in 0..door_count {
        let coord = doors_tk_at(i);
        dungeon_place_door_if_next_to_two_walls(Coord_t {
            y: coord.y,
            x: coord.x - 1,
        });
        dungeon_place_door_if_next_to_two_walls(Coord_t {
            y: coord.y,
            x: coord.x + 1,
        });
        dungeon_place_door_if_next_to_two_walls(Coord_t {
            y: coord.y - 1,
            x: coord.x,
        });
        dungeon_place_door_if_next_to_two_walls(Coord_t {
            y: coord.y + 1,
            x: coord.x,
        });
    }

    let mut alloc_level = i32::from(current_level) / 3;
    #[allow(clippy::manual_clamp)] // C++ dungeon_generate.cpp lines 1079–1084
    if alloc_level < 2 {
        alloc_level = 2;
    } else if alloc_level > 10 {
        alloc_level = 10;
    }

    dungeon_place_stairs(2, random_number(2) + 2, 3);
    dungeon_place_stairs(1, random_number(2), 3);

    let coord = dungeon_new_spot();
    with_state_mut(|state| {
        state.py.pos.y = coord.y;
        state.py.pos.x = coord.x;
    });

    monster_place_new_within_distance(
        random_number(8) + i32::from(MON_MIN_PER_LEVEL) + alloc_level,
        0,
        true,
    );
    dungeon_allocate_and_place_object(set_corridors, 3, random_number(alloc_level));
    dungeon_allocate_and_place_object(
        set_rooms,
        5,
        random_number_normal_distribution(i32::from(objects::LEVEL_OBJECTS_PER_ROOM), 3),
    );
    dungeon_allocate_and_place_object(
        set_floors,
        5,
        random_number_normal_distribution(i32::from(objects::LEVEL_OBJECTS_PER_CORRIDOR), 3),
    );
    dungeon_allocate_and_place_object(
        set_floors,
        4,
        random_number_normal_distribution(i32::from(objects::LEVEL_TOTAL_GOLD_AND_GEMS), 3),
    );
    dungeon_allocate_and_place_object(set_floors, 1, random_number(alloc_level));

    if i32::from(current_level) >= i32::from(MON_ENDGAME_LEVEL) {
        monster_place_winning();
    }
}

/// C++ dungeon_generate.cpp lines 1108–1149.
pub fn dungeon_build_store(store_id: i32, coord: Coord_t) {
    let yval = coord.y * 10 + 5;
    let xval = coord.x * 16 + 16;
    let height = yval - random_number(3);
    let depth = yval + random_number(4);
    let left = xval - random_number(6);
    let right = xval + random_number(6);

    with_state_mut(|state| {
        for y in height..=depth {
            for x in left..=right {
                state.dg.floor[y as usize][x as usize].feature_id = TILE_BOUNDARY_WALL;
            }
        }
    });

    let tmp = random_number(4);
    let (y, x) = if tmp < 3 {
        let y = random_number(depth - height) + height - 1;
        let x = if tmp == 1 { left } else { right };
        (y, x)
    } else {
        let x = random_number(right - left) + left - 1;
        let y = if tmp == 3 { depth } else { height };
        (y, x)
    };

    with_state_mut(|state| {
        state.dg.floor[y as usize][x as usize].feature_id = TILE_CORR_FLOOR;
    });

    let cur_pos = popt();
    with_state_mut(|state| {
        state.dg.floor[y as usize][x as usize].treasure_id = cur_pos as u8;
        inventory_item_copy_to(
            (objects::OBJ_STORE_DOOR + store_id as u16) as i16,
            &mut state.game.treasure.list[cur_pos as usize],
        );
    });
}

/// C++ dungeon_generate.cpp lines 1152–1157.
pub fn treasure_linker() {
    with_state_mut(|state| {
        for item in &mut state.game.treasure.list {
            inventory_item_copy_to(objects::OBJ_NOTHING as i16, item);
        }
        state.game.treasure.current_id = i16::from(MIN_TREASURE_LIST_ID);
    });
}

/// C++ dungeon_generate.cpp lines 1160–1165.
pub fn monster_linker() {
    with_state_mut(|state| {
        for monster in &mut state.monsters {
            *monster = BLANK_MONSTER;
        }
        state.next_free_monster_id = i16::from(crate::config::monsters::MON_MIN_INDEX_ID);
    });
}

/// C++ dungeon_generate.cpp lines 1167–1187.
pub fn dungeon_place_town_stores() {
    let mut rooms = [0i32; 6];
    for i in 0..6i32 {
        rooms[i as usize] = i;
    }

    let mut rooms_count = 6;

    for y in 0..2 {
        for x in 0..3 {
            let room_id = random_number(rooms_count) - 1;
            dungeon_build_store(rooms[room_id as usize], Coord_t { y, x });

            for i in room_id..rooms_count - 1 {
                rooms[i as usize] = rooms[(i + 1) as usize];
            }

            rooms_count -= 1;
        }
    }
}

/// C++ dungeon_generate.cpp lines 1189–1191.
#[must_use]
pub fn is_nigh_time() -> bool {
    with_state(|state| (0x1 & (state.dg.game_turn / 5000)) != 0)
}

/// C++ dungeon_generate.cpp lines 1194–1213.
pub fn light_town() {
    if is_nigh_time() {
        with_state_mut(|state| {
            for y in 0..state.dg.height {
                for x in 0..state.dg.width {
                    if state.dg.floor[y as usize][x as usize].feature_id != TILE_DARK_FLOOR {
                        state.dg.floor[y as usize][x as usize].permanent_light = true;
                    }
                }
            }
        });
        monster_place_new_within_distance(i32::from(MON_MIN_TOWNSFOLK_NIGHT), 3, true);
    } else {
        with_state_mut(|state| {
            for y in 0..state.dg.height {
                for x in 0..state.dg.width {
                    state.dg.floor[y as usize][x as usize].permanent_light = true;
                }
            }
        });
        monster_place_new_within_distance(i32::from(MON_MIN_TOWNSFOLK_DAY), 3, true);
    }
}

/// C++ dungeon_generate.cpp lines 1220–1242.
pub fn town_generation() {
    let town_seed = with_state(|state| state.game.town_seed);
    seed_set(town_seed);

    dungeon_place_town_stores();
    dungeon_fill_empty_tiles_with(TILE_DARK_FLOOR);
    dungeon_place_boundary_walls();
    dungeon_place_stairs(2, 1, 0);

    seed_reset_to_old_seed();

    let coord = dungeon_new_spot();
    with_state_mut(|state| {
        state.py.pos.y = coord.y;
        state.py.pos.x = coord.x;
    });

    light_town();
    store_maintenance();
}

/// C++ dungeon_generate.cpp lines 1245–1278.
pub fn generate_cave() {
    with_state_mut(|state| {
        state.dg.panel.top = 0;
        state.dg.panel.bottom = 0;
        state.dg.panel.left = 0;
        state.dg.panel.right = 0;

        state.py.pos.y = -1;
        state.py.pos.x = -1;
    });

    treasure_linker();
    monster_linker();
    dungeon_blank_entire_cave();

    with_state_mut(|state| {
        state.dg.height = i16::from(MAX_HEIGHT);
        state.dg.width = i16::from(MAX_WIDTH);

        if state.dg.current_level == 0 {
            state.dg.height = i16::from(SCREEN_HEIGHT);
            state.dg.width = i16::from(SCREEN_WIDTH);
        }

        state.dg.panel.max_rows =
            ((i32::from(state.dg.height) / i32::from(SCREEN_HEIGHT)) * 2 - 2) as i16;
        state.dg.panel.max_cols =
            ((i32::from(state.dg.width) / i32::from(SCREEN_WIDTH)) * 2 - 2) as i16;

        state.dg.panel.row = i32::from(state.dg.panel.max_rows);
        state.dg.panel.col = i32::from(state.dg.panel.max_cols);
    });

    let current_level = with_state(|state| state.dg.current_level);
    if current_level == 0 {
        town_generation();
    } else {
        dungeon_generate();
    }
}
