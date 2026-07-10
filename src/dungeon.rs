//! Port of src/dungeon.h / dungeon.cpp — dungeon grid, tile helpers, object placement.

use crate::config::dungeon::objects::{
    MAX_GOLD_TYPES, MAX_TRAPS, OBJ_CLOSED_DOOR, OBJ_GOLD_LIST, OBJ_RUBBLE, OBJ_TRAP_LIST,
};
use crate::config::player::status::PY_BLIND;
use crate::config::treasure::TREASURE_CHANCE_OF_GREAT_ITEM;
use crate::data_creatures::CREATURES_LIST;
use crate::data_treasure::GAME_OBJECTS;
use crate::dice::Dice;
use crate::dungeon_los::los;
use crate::dungeon_tile::{
    Tile, MAX_CAVE_FLOOR, MAX_OPEN_SPACE, MIN_CAVE_WALL, TILE_BLOCKED_FLOOR, TILE_BOUNDARY_WALL,
    TILE_CORR_FLOOR, TILE_DARK_FLOOR, TILE_GRANITE_WALL, TILE_LIGHT_FLOOR,
};
use crate::game::{random_number, random_number_state, with_state, with_state_mut};
use crate::game_objects::{item_get_random_object_id, popt, pusht_state};
use crate::inventory::inventory_item_copy_to;
use crate::monster::BLANK_MONSTER;
use crate::treasure::{
    magic_treasure_magical_ability, TV_INVIS_TRAP, TV_MAX_VISIBLE, TV_MIN_VISIBLE, TV_SECRET_DOOR,
    TV_VIS_TRAP,
};
use crate::types::Coord_t;
use crate::ui::{coord_inside_panel, Panel};
use crate::ui_io::terminal::{self, Coord};

fn panel_put_dungeon_tile(ch: u8, coord: Coord_t) {
    terminal::panel_put_tile(
        ch,
        Coord {
            y: coord.y,
            x: coord.x,
        },
    );
}

pub const RATIO: u8 = 3;
pub const MAX_HEIGHT: u8 = 66;
pub const MAX_WIDTH: u8 = 198;
pub const SCREEN_HEIGHT: u8 = 22;
pub const SCREEN_WIDTH: u8 = 66;
pub const QUART_HEIGHT: u8 = SCREEN_HEIGHT / 4;
pub const QUART_WIDTH: u8 = SCREEN_WIDTH / 4;

/// Port of `DungeonObject_t` in dungeon.h.
#[derive(Clone, Copy, Debug, Default)]
pub struct DungeonObject {
    pub name: &'static str,
    pub flags: u32,
    pub category_id: u8,
    pub sprite: u8,
    pub misc_use: i16,
    pub cost: i32,
    pub sub_category_id: u8,
    pub items_count: u8,
    pub weight: u16,
    pub to_hit: i16,
    pub to_damage: i16,
    pub ac: i16,
    pub to_ac: i16,
    pub damage: Dice,
    pub depth_first_found: u8,
}

/// Port of `Dungeon_t` in dungeon.h.
#[derive(Clone, Debug)]
pub struct Dungeon {
    pub height: i16,
    pub width: i16,
    pub panel: Panel,
    pub game_turn: i32,
    pub current_level: i16,
    pub generate_new_level: bool,
    pub floor: [[Tile; MAX_WIDTH as usize]; MAX_HEIGHT as usize],
}

impl Default for Dungeon {
    fn default() -> Self {
        Self {
            height: 0,
            width: 0,
            panel: Panel::default(),
            // C++: Dungeon_t{0, 0, {}, -1, 0, true, {}} — positional init puts
            // -1 in game_turn (4th field) and 0 in current_level (5th field).
            game_turn: -1,
            current_level: 0,
            generate_new_level: true,
            floor: [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize],
        }
    }
}

/// C++ dungeon.cpp lines 15–96.
pub fn dungeon_display_map() {
    terminal::terminal_save_screen();
    terminal::clear_screen();

    let mut priority = [0i16; 256];
    priority[b'<' as usize] = 5;
    priority[b'>' as usize] = 5;
    priority[b'@' as usize] = 10;
    priority[b'#' as usize] = -5;
    priority[b'.' as usize] = -10;
    priority[b'\\' as usize] = -3;
    priority[b' ' as usize] = -15;

    let panel_width = MAX_WIDTH / RATIO;
    let panel_height = MAX_HEIGHT / RATIO;

    let mut map = vec![b' '; panel_width as usize];
    let mut line = -1i32;
    let mut player_y = 0i32;
    let mut player_x = 0i32;

    terminal::add_char(b'+', Coord { y: 0, x: 0 });
    terminal::add_char(
        b'+',
        Coord {
            y: 0,
            x: i32::from(panel_width) + 1,
        },
    );
    for i in 0..panel_width {
        terminal::add_char(
            b'-',
            Coord {
                y: 0,
                x: i32::from(i) + 1,
            },
        );
        terminal::add_char(
            b'-',
            Coord {
                y: i32::from(panel_height) + 1,
                x: i32::from(i) + 1,
            },
        );
    }
    for i in 0..panel_height {
        terminal::add_char(
            b'|',
            Coord {
                y: i32::from(i) + 1,
                x: 0,
            },
        );
        terminal::add_char(
            b'|',
            Coord {
                y: i32::from(i) + 1,
                x: i32::from(panel_width) + 1,
            },
        );
    }
    terminal::add_char(
        b'+',
        Coord {
            y: i32::from(panel_height) + 1,
            x: 0,
        },
    );
    terminal::add_char(
        b'+',
        Coord {
            y: i32::from(panel_height) + 1,
            x: i32::from(panel_width) + 1,
        },
    );
    terminal::put_string("Hit any key to continue", Coord { y: 23, x: 23 });

    for y in 0..MAX_HEIGHT {
        let row = y / RATIO;
        if i32::from(row) != line {
            if line >= 0 {
                let line_buffer = format!("|{}|", String::from_utf8_lossy(&map));
                terminal::put_string(&line_buffer, Coord { y: line + 1, x: 0 });
            }
            map.fill(b' ');
            line = i32::from(row);
        }

        for x in 0..MAX_WIDTH {
            let col = x / RATIO;
            let cave_char = cave_get_tile_symbol(Coord_t {
                y: i32::from(y),
                x: i32::from(x),
            });
            if priority[map[col as usize] as usize] < priority[cave_char as usize] {
                map[col as usize] = cave_char;
            }
            if map[col as usize] == b'@' {
                player_x = i32::from(col) + 1;
                player_y = i32::from(row) + 1;
            }
        }
    }

    if line >= 0 {
        let line_buffer = format!("|{}|", String::from_utf8_lossy(&map));
        terminal::put_string(&line_buffer, Coord { y: line + 1, x: 0 });
    }

    terminal::move_cursor(Coord {
        y: player_y,
        x: player_x,
    });
    let _ = terminal::get_key_input();
    terminal::terminal_restore_screen();
}

/// C++ dungeon.cpp lines 99–104.
#[must_use]
pub fn coord_in_bounds(coord: Coord_t) -> bool {
    with_state(|state| {
        let y = coord.y > 0 && coord.y < i32::from(state.dg.height) - 1;
        let x = coord.x > 0 && coord.x < i32::from(state.dg.width) - 1;
        y && x
    })
}

/// C++ dungeon.cpp lines 107–122.
#[must_use]
pub fn coord_distance_between(from: Coord_t, to: Coord_t) -> i32 {
    let mut dy = from.y - to.y;
    if dy < 0 {
        dy = -dy;
    }
    let mut dx = from.x - to.x;
    if dx < 0 {
        dx = -dx;
    }
    let a = (dy + dx) << 1;
    let b = if dy > dx { dx } else { dy };
    (a - b) >> 1
}

/// C++ dungeon.cpp lines 127–147.
#[must_use]
pub fn coord_walls_next_to(coord: Coord_t) -> i32 {
    with_state(|state| {
        let mut walls = 0;
        let y = coord.y as usize;
        let x = coord.x as usize;
        if state.dg.floor[y - 1][x].feature_id >= MIN_CAVE_WALL {
            walls += 1;
        }
        if state.dg.floor[y + 1][x].feature_id >= MIN_CAVE_WALL {
            walls += 1;
        }
        if state.dg.floor[y][x - 1].feature_id >= MIN_CAVE_WALL {
            walls += 1;
        }
        if state.dg.floor[y][x + 1].feature_id >= MIN_CAVE_WALL {
            walls += 1;
        }
        walls
    })
}

/// C++ dungeon.cpp lines 152–168.
#[must_use]
pub fn coord_corridor_walls_next_to(coord: Coord_t) -> i32 {
    with_state(|state| {
        let mut walls = 0;
        for y in coord.y - 1..=coord.y + 1 {
            for x in coord.x - 1..=coord.x + 1 {
                let tile = &state.dg.floor[y as usize][x as usize];
                let tile_id = tile.feature_id;
                let treasure_id = tile.treasure_id;
                if tile_id == TILE_CORR_FLOOR
                    && (treasure_id == 0
                        || state.game.treasure.list[treasure_id as usize].category_id
                            < crate::treasure::TV_MIN_DOORS)
                {
                    walls += 1;
                }
            }
        }
        walls
    })
}

/// C++ dungeon.cpp lines 171–209.
#[must_use]
pub fn cave_get_tile_symbol(coord: Coord_t) -> u8 {
    with_state_mut(|state| {
        let y = coord.y as usize;
        let x = coord.x as usize;
        let creature_id = state.dg.floor[y][x].creature_id;

        if creature_id == 1 && (state.py.running_tracker == 0 || state.options.run_print_self) {
            return b'@';
        }

        if (state.py.flags.status & PY_BLIND) != 0 {
            return b' ';
        }

        if state.py.flags.image > 0 && random_number_state(state, 12) == 1 {
            return (random_number_state(state, 95) + 31) as u8;
        }

        let tile = &state.dg.floor[y][x];

        if creature_id > 1 && state.monsters[creature_id as usize].lit {
            let cid = state.monsters[creature_id as usize].creature_id as usize;
            return CREATURES_LIST[cid].sprite;
        }

        if !tile.permanent_light && !tile.temporary_light && !tile.field_mark {
            return b' ';
        }

        if tile.treasure_id != 0
            && state.game.treasure.list[tile.treasure_id as usize].category_id != TV_INVIS_TRAP
        {
            return state.game.treasure.list[tile.treasure_id as usize].sprite;
        }

        if tile.feature_id <= MAX_CAVE_FLOOR {
            return b'.';
        }

        if tile.feature_id == TILE_GRANITE_WALL
            || tile.feature_id == TILE_BOUNDARY_WALL
            || !state.options.highlight_seams
        {
            return b'#';
        }

        b'%'
    })
}

/// C++ dungeon.cpp lines 212–214.
#[must_use]
pub fn cave_tile_visible(coord: Coord_t) -> bool {
    with_state(|state| {
        let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
        tile.permanent_light || tile.temporary_light || tile.field_mark
    })
}

/// C++ dungeon.cpp lines 217–221.
pub fn dungeon_set_trap(coord: Coord_t, sub_type_id: i32) {
    let free_treasure_id = popt();
    with_state_mut(|state| {
        state.dg.floor[coord.y as usize][coord.x as usize].treasure_id = free_treasure_id as u8;
        inventory_item_copy_to(
            (OBJ_TRAP_LIST + sub_type_id as u16) as i16,
            &mut state.game.treasure.list[free_treasure_id as usize],
        );
    });
}

/// C++ dungeon.cpp lines 225–243.
pub fn trap_change_visibility(coord: Coord_t) {
    let lite = with_state_mut(|state| {
        let treasure_id = state.dg.floor[coord.y as usize][coord.x as usize].treasure_id;
        let item = &mut state.game.treasure.list[treasure_id as usize];

        if item.category_id == TV_INVIS_TRAP {
            item.category_id = TV_VIS_TRAP;
            return true;
        }

        if item.category_id == TV_SECRET_DOOR {
            item.id = OBJ_CLOSED_DOOR;
            item.category_id = GAME_OBJECTS[OBJ_CLOSED_DOOR as usize].category_id;
            item.sprite = GAME_OBJECTS[OBJ_CLOSED_DOOR as usize].sprite;
            return true;
        }
        false
    });
    if lite {
        dungeon_lite_spot(coord);
    }
}

/// C++ dungeon.cpp lines 246–251.
pub fn dungeon_place_rubble(coord: Coord_t) {
    let free_treasure_id = popt();
    with_state_mut(|state| {
        state.dg.floor[coord.y as usize][coord.x as usize].treasure_id = free_treasure_id as u8;
        state.dg.floor[coord.y as usize][coord.x as usize].feature_id = TILE_BLOCKED_FLOOR;
        inventory_item_copy_to(
            OBJ_RUBBLE as i16,
            &mut state.game.treasure.list[free_treasure_id as usize],
        );
    });
}

/// C++ dungeon.cpp lines 254–274.
pub fn dungeon_place_gold(coord: Coord_t) {
    let free_treasure_id = popt();
    let creature_id = with_state_mut(|state| {
        let mut gold_type_id =
            ((random_number_state(state, i32::from(state.dg.current_level) + 2) + 2) / 2) - 1;

        if random_number_state(state, i32::from(TREASURE_CHANCE_OF_GREAT_ITEM)) == 1 {
            gold_type_id += random_number_state(state, i32::from(state.dg.current_level) + 1);
        }

        if gold_type_id >= i32::from(MAX_GOLD_TYPES) {
            gold_type_id = i32::from(MAX_GOLD_TYPES) - 1;
        }

        state.dg.floor[coord.y as usize][coord.x as usize].treasure_id = free_treasure_id as u8;
        inventory_item_copy_to(
            (OBJ_GOLD_LIST + gold_type_id as u16) as i16,
            &mut state.game.treasure.list[free_treasure_id as usize],
        );
        let base_cost = state.game.treasure.list[free_treasure_id as usize].cost;
        // C++ dungeon.cpp line 269: (int32_t) randomNumber((int) cost) cast.
        let bonus = 8 * random_number_state(state, base_cost) + random_number_state(state, 8);
        state.game.treasure.list[free_treasure_id as usize].cost += bonus;
        state.dg.floor[coord.y as usize][coord.x as usize].creature_id
    });

    if creature_id == 1 {
        terminal::print_message(Some("You feel something roll beneath your feet."));
    }
}

/// C++ dungeon.cpp lines 277–290.
pub fn dungeon_place_random_object_at(coord: Coord_t, must_be_small: bool) {
    let free_treasure_id = popt();
    let (creature_id, level) = with_state(|state| {
        (
            state.dg.floor[coord.y as usize][coord.x as usize].creature_id,
            i32::from(state.dg.current_level),
        )
    });
    let object_id = item_get_random_object_id(level, must_be_small);

    with_state_mut(|state| {
        state.dg.floor[coord.y as usize][coord.x as usize].treasure_id = free_treasure_id as u8;
        inventory_item_copy_to(
            state.sorted_objects[object_id as usize],
            &mut state.game.treasure.list[free_treasure_id as usize],
        );
    });
    magic_treasure_magical_ability(free_treasure_id, level);

    if creature_id == 1 {
        terminal::print_message(Some("You feel something roll beneath your feet."));
    }
}

/// C++ dungeon.cpp lines 293–324.
pub fn dungeon_allocate_and_place_object(
    set_function: impl Fn(i32) -> bool,
    object_type: i32,
    number: i32,
) {
    let mut coord = Coord_t { y: 0, x: 0 };
    for _ in 0..number {
        loop {
            let (height, width, py_pos) =
                with_state(|state| (state.dg.height, state.dg.width, state.py.pos));
            coord.y = random_number(i32::from(height)) - 1;
            coord.x = random_number(i32::from(width)) - 1;
            let feature_id =
                with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].feature_id);
            let treasure_id =
                with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].treasure_id);
            if set_function(i32::from(feature_id))
                && treasure_id == 0
                && (coord.y != py_pos.y || coord.x != py_pos.x)
            {
                break;
            }
        }

        match object_type {
            1 => dungeon_set_trap(coord, random_number(i32::from(MAX_TRAPS)) - 1),
            2 | 3 => dungeon_place_rubble(coord),
            4 => dungeon_place_gold(coord),
            5 => dungeon_place_random_object_at(coord, false),
            _ => {}
        }
    }
}

/// C++ dungeon.cpp lines 327–347.
pub fn dungeon_place_random_object_near(coord: Coord_t, mut tries: i32) {
    loop {
        // C++ `for (i = 0; i <= 10; i++)` with `i = 9` on success: the for-increment
        // then yields `i == 10`, so the body runs one more time after a placement.
        let mut i = 0;
        while i <= 10 {
            let at = Coord_t {
                y: coord.y - 3 + random_number(5),
                x: coord.x - 4 + random_number(7),
            };

            if coord_in_bounds(at)
                && with_state(|state| {
                    state.dg.floor[at.y as usize][at.x as usize].feature_id <= MAX_CAVE_FLOOR
                })
                && with_state(|state| state.dg.floor[at.y as usize][at.x as usize].treasure_id == 0)
            {
                if random_number(100) < 75 {
                    dungeon_place_random_object_at(at, false);
                } else {
                    dungeon_place_gold(at);
                }
                i = 9;
            }
            i += 1;
        }

        tries -= 1;
        if tries == 0 {
            break;
        }
    }
}

/// C++ dungeon.cpp lines 351–355.
pub fn dungeon_move_creature_record(from: Coord_t, to: Coord_t) {
    with_state_mut(|state| {
        let id = state.dg.floor[from.y as usize][from.x as usize].creature_id;
        state.dg.floor[from.y as usize][from.x as usize].creature_id = 0;
        state.dg.floor[to.y as usize][to.x as usize].creature_id = id;
    });
}

/// C++ dungeon.cpp lines 358–389.
pub fn dungeon_light_room(coord: Coord_t) {
    let height_middle = i32::from(SCREEN_HEIGHT / 2);
    let width_middle = i32::from(SCREEN_WIDTH / 2);

    let top = (coord.y / height_middle) * height_middle;
    let left = (coord.x / width_middle) * width_middle;
    let bottom = top + height_middle - 1;
    let right = left + width_middle - 1;

    for location_y in top..=bottom {
        for location_x in left..=right {
            let location = Coord_t {
                y: location_y,
                x: location_x,
            };
            let should_draw = with_state_mut(|state| {
                let tile = &mut state.dg.floor[location_y as usize][location_x as usize];
                if tile.perma_lit_room && !tile.permanent_light {
                    tile.permanent_light = true;
                    if tile.feature_id == TILE_DARK_FLOOR {
                        tile.feature_id = TILE_LIGHT_FLOOR;
                    }
                    if !tile.field_mark && tile.treasure_id != 0 {
                        let tval = state.game.treasure.list[tile.treasure_id as usize].category_id;
                        if (TV_MIN_VISIBLE..=TV_MAX_VISIBLE).contains(&tval) {
                            tile.field_mark = true;
                        }
                    }
                    true
                } else {
                    false
                }
            });
            if should_draw {
                panel_put_dungeon_tile(cave_get_tile_symbol(location), location);
            }
        }
    }
}

/// C++ dungeon.cpp lines 392–399.
pub fn dungeon_lite_spot(coord: Coord_t) {
    if !coord_inside_panel(coord) {
        return;
    }
    let symbol = cave_get_tile_symbol(coord);
    panel_put_dungeon_tile(symbol, coord);
}

/// C++ dungeon.cpp lines 403–464.
fn sub1_move_light(from: Coord_t, to: Coord_t) {
    with_state_mut(|state| {
        if state.py.temporary_light_only {
            for y in from.y - 1..=from.y + 1 {
                for x in from.x - 1..=from.x + 1 {
                    state.dg.floor[y as usize][x as usize].temporary_light = false;
                }
            }
            if state.py.running_tracker != 0 && !state.options.run_print_self {
                state.py.temporary_light_only = false;
            }
        } else if state.py.running_tracker == 0 || state.options.run_print_self {
            state.py.temporary_light_only = true;
        }
    });

    with_state_mut(|state| {
        for y in to.y - 1..=to.y + 1 {
            for x in to.x - 1..=to.x + 1 {
                let tile = &mut state.dg.floor[y as usize][x as usize];
                if state.py.temporary_light_only {
                    tile.temporary_light = true;
                }
                if tile.feature_id >= MIN_CAVE_WALL {
                    tile.permanent_light = true;
                } else if !tile.field_mark && tile.treasure_id != 0 {
                    let tval = state.game.treasure.list[tile.treasure_id as usize].category_id;
                    if (TV_MIN_VISIBLE..=TV_MAX_VISIBLE).contains(&tval) {
                        tile.field_mark = true;
                    }
                }
            }
        }
    });

    let (top, bottom, left, right) = {
        let (top, bottom) = if from.y < to.y {
            (from.y - 1, to.y + 1)
        } else {
            (to.y - 1, from.y + 1)
        };
        let (left, right) = if from.x < to.x {
            (from.x - 1, to.x + 1)
        } else {
            (to.x - 1, from.x + 1)
        };
        (top, bottom, left, right)
    };

    for coord_y in top..=bottom {
        for coord_x in left..=right {
            let coord = Coord_t {
                y: coord_y,
                x: coord_x,
            };
            panel_put_dungeon_tile(cave_get_tile_symbol(coord), coord);
        }
    }
}

/// C++ dungeon.cpp lines 468–487.
fn sub3_move_light(from: Coord_t, to: Coord_t) {
    let (temporary_light_only, running_tracker, run_print_self) = with_state(|state| {
        (
            state.py.temporary_light_only,
            state.py.running_tracker,
            state.options.run_print_self,
        )
    });

    if temporary_light_only {
        for coord_y in from.y - 1..=from.y + 1 {
            for coord_x in from.x - 1..=from.x + 1 {
                let coord = Coord_t {
                    y: coord_y,
                    x: coord_x,
                };
                with_state_mut(|state| {
                    state.dg.floor[coord_y as usize][coord_x as usize].temporary_light = false;
                });
                panel_put_dungeon_tile(cave_get_tile_symbol(coord), coord);
            }
        }
        with_state_mut(|state| {
            state.py.temporary_light_only = false;
        });
    } else if running_tracker == 0 || run_print_self {
        panel_put_dungeon_tile(cave_get_tile_symbol(from), from);
    }

    if running_tracker == 0 || run_print_self {
        panel_put_dungeon_tile(b'@', to);
    }
}

/// C++ dungeon.cpp lines 491–497.
pub fn dungeon_move_character_light(from: Coord_t, to: Coord_t) {
    let use_sub3 = with_state(|state| state.py.flags.blind > 0 || !state.py.carrying_light);
    if use_sub3 {
        sub3_move_light(from, to);
    } else {
        sub1_move_light(from, to);
    }
}

/// C++ dungeon.cpp lines 506–509.
pub fn dungeon_delete_monster(id: i32) {
    dungeon_remove_monster_from_level(id);
    dungeon_delete_monster_record(id);
}

/// C++ dungeon.cpp lines 514–530.
pub fn dungeon_remove_monster_from_level(id: i32) {
    let lite_pos = with_state_mut(|state| {
        let monster = &mut state.monsters[id as usize];
        monster.hp = -1;
        let pos = monster.pos;
        let lit = monster.lit;
        state.dg.floor[pos.y as usize][pos.x as usize].creature_id = 0;
        if state.monster_multiply_total > 0 {
            state.monster_multiply_total -= 1;
        }
        if lit {
            Some(pos)
        } else {
            None
        }
    });
    if let Some(pos) = lite_pos {
        dungeon_lite_spot(pos);
    }
}

/// C++ dungeon.cpp lines 534–545.
pub fn dungeon_delete_monster_record(id: i32) {
    with_state_mut(|state| {
        let last_id = i32::from(state.next_free_monster_id - 1);
        if id != last_id {
            let monster_pos = state.monsters[last_id as usize].pos;
            state.dg.floor[monster_pos.y as usize][monster_pos.x as usize].creature_id = id as u8;
            state.monsters[id as usize] = state.monsters[last_id as usize];
        }
        state.monsters[last_id as usize] = BLANK_MONSTER;
        state.next_free_monster_id -= 1;
    });
}

/// C++ dungeon.cpp lines 548–598.
pub fn dungeon_summon_object(coord: Coord_t, mut amount: i32, object_type: i32) -> i32 {
    let mut real_type = if object_type == 1 || object_type == 5 {
        1
    } else {
        256
    };

    let mut result = 0;

    while amount != 0 {
        for tries in 0..=20 {
            let at = Coord_t {
                y: coord.y - 3 + random_number(5),
                x: coord.x - 3 + random_number(5),
            };

            if coord_in_bounds(at) && los(coord, at) {
                let (feature_id, treasure_id) = with_state(|state| {
                    (
                        state.dg.floor[at.y as usize][at.x as usize].feature_id,
                        state.dg.floor[at.y as usize][at.x as usize].treasure_id,
                    )
                });
                if feature_id <= MAX_OPEN_SPACE && treasure_id == 0 {
                    if object_type == 3 || object_type == 7 {
                        if random_number(100) < 50 {
                            real_type = 1;
                        } else {
                            real_type = 256;
                        }
                    }

                    if real_type == 1 {
                        dungeon_place_random_object_at(at, object_type >= 4);
                    } else {
                        dungeon_place_gold(at);
                    }

                    dungeon_lite_spot(at);

                    if cave_tile_visible(at) {
                        result += real_type;
                    }

                    break;
                }
            }
            let _ = tries;
        }

        amount -= 1;
    }

    result
}

/// C++ dungeon.cpp lines 601–616.
#[must_use]
pub fn dungeon_delete_object(coord: Coord_t) -> bool {
    with_state_mut(|state| {
        let treasure_id = {
            let tile = &mut state.dg.floor[coord.y as usize][coord.x as usize];
            if tile.feature_id == TILE_BLOCKED_FLOOR {
                tile.feature_id = TILE_CORR_FLOOR;
            }
            let treasure_id = tile.treasure_id;
            tile.treasure_id = 0;
            tile.field_mark = false;
            treasure_id
        };
        pusht_state(state, treasure_id);
    });
    dungeon_lite_spot(coord);
    // C++ returns caveTileVisible after clearing field_mark.
    cave_tile_visible(coord)
}
