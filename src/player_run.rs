//! Running / find state machine.

use std::cell::Cell;

use crate::dungeon::{cave_get_tile_symbol, dungeon_move_character_light};
use crate::dungeon_tile::MAX_OPEN_SPACE;
use crate::game::{with_state, with_state_mut};
use crate::player_move::{player_move, player_move_position};
use crate::treasure::{TV_INVIS_TRAP, TV_OPEN_DOOR, TV_SECRET_DOOR};
use crate::types::Coord_t;
use crate::ui_io::terminal::{self, panel_put_tile, Coord};

const CYCLE: [i32; 17] = [1, 2, 3, 6, 9, 8, 7, 4, 1, 2, 3, 6, 9, 8, 7, 4, 1];
const CHOME: [i32; 10] = [-1, 8, 9, 10, 7, -1, 11, 6, 5, 4];

thread_local! {
    static FIND_OPENAREA: Cell<bool> = const { Cell::new(false) };
    static FIND_BREAKRIGHT: Cell<bool> = const { Cell::new(false) };
    static FIND_BREAKLEFT: Cell<bool> = const { Cell::new(false) };
    static FIND_PREVDIR: Cell<i32> = const { Cell::new(0) };
    static FIND_DIRECTION: Cell<i32> = const { Cell::new(0) };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunFindState {
    pub find_openarea: bool,
    pub find_breakright: bool,
    pub find_breakleft: bool,
    pub find_prevdir: i32,
    pub find_direction: i32,
}

#[doc(hidden)]
pub fn test_run_find_state() -> RunFindState {
    RunFindState {
        find_openarea: FIND_OPENAREA.with(std::cell::Cell::get),
        find_breakright: FIND_BREAKRIGHT.with(std::cell::Cell::get),
        find_breakleft: FIND_BREAKLEFT.with(std::cell::Cell::get),
        find_prevdir: FIND_PREVDIR.with(std::cell::Cell::get),
        find_direction: FIND_DIRECTION.with(std::cell::Cell::get),
    }
}

#[doc(hidden)]
pub fn test_set_run_find_state(state: RunFindState) {
    FIND_OPENAREA.with(|c| c.set(state.find_openarea));
    FIND_BREAKRIGHT.with(|c| c.set(state.find_breakright));
    FIND_BREAKLEFT.with(|c| c.set(state.find_breakleft));
    FIND_PREVDIR.with(|c| c.set(state.find_prevdir));
    FIND_DIRECTION.with(|c| c.set(state.find_direction));
}

#[doc(hidden)]
pub fn test_reset_run_find_state() {
    test_set_run_find_state(RunFindState {
        find_openarea: false,
        find_breakright: false,
        find_breakleft: false,
        find_prevdir: 0,
        find_direction: 0,
    });
}

fn set_find_openarea(value: bool) {
    FIND_OPENAREA.with(|c| c.set(value));
}

fn find_openarea() -> bool {
    FIND_OPENAREA.with(std::cell::Cell::get)
}

fn set_find_breakright(value: bool) {
    FIND_BREAKRIGHT.with(|c| c.set(value));
}

fn find_breakright() -> bool {
    FIND_BREAKRIGHT.with(std::cell::Cell::get)
}

fn set_find_breakleft(value: bool) {
    FIND_BREAKLEFT.with(|c| c.set(value));
}

fn find_breakleft() -> bool {
    FIND_BREAKLEFT.with(std::cell::Cell::get)
}

fn set_find_prevdir(value: i32) {
    FIND_PREVDIR.with(|c| c.set(value));
}

fn find_prevdir() -> i32 {
    FIND_PREVDIR.with(std::cell::Cell::get)
}

fn set_find_direction(value: i32) {
    FIND_DIRECTION.with(|c| c.set(value));
}

fn find_direction() -> i32 {
    FIND_DIRECTION.with(std::cell::Cell::get)
}

/// 128
fn player_can_see_dungeon_wall(dir: i32, coord: Coord_t) -> bool {
    let mut spot = coord;
    if !player_move_position(dir, &mut spot) {
        return true;
    }
    let ch = cave_get_tile_symbol(spot);
    ch == b'#' || ch == b'%'
}

/// 134
fn player_see_nothing(dir: i32, coord: Coord_t) -> bool {
    let mut spot = coord;
    player_move_position(dir, &mut spot) && cave_get_tile_symbol(spot) == b' '
}

/// 184
fn find_running_break(dir: i32, coord: Coord_t) {
    let mut deep_left = false;
    let mut deep_right = false;
    let mut short_left = false;
    let mut short_right = false;

    let cycle_index = CHOME[dir as usize];

    let player_pos = with_state(|state| state.py.pos);

    if player_can_see_dungeon_wall(CYCLE[(cycle_index + 1) as usize], player_pos) {
        set_find_breakleft(true);
        short_left = true;
    } else if player_can_see_dungeon_wall(CYCLE[(cycle_index + 1) as usize], coord) {
        set_find_breakleft(true);
        deep_left = true;
    }

    if player_can_see_dungeon_wall(CYCLE[(cycle_index - 1) as usize], player_pos) {
        set_find_breakright(true);
        short_right = true;
    } else if player_can_see_dungeon_wall(CYCLE[(cycle_index - 1) as usize], coord) {
        set_find_breakright(true);
        deep_right = true;
    }

    if find_breakleft() && find_breakright() {
        set_find_openarea(false);

        if (dir & 1) != 0 {
            if deep_left && !deep_right {
                set_find_prevdir(CYCLE[(cycle_index - 1) as usize]);
            } else if deep_right && !deep_left {
                set_find_prevdir(CYCLE[(cycle_index + 1) as usize]);
            }
        } else if player_can_see_dungeon_wall(CYCLE[cycle_index as usize], coord) {
            if short_left && !short_right {
                set_find_prevdir(CYCLE[(cycle_index - 2) as usize]);
            } else if short_right && !short_left {
                set_find_prevdir(CYCLE[(cycle_index + 2) as usize]);
            }
        }
    } else {
        set_find_openarea(true);
    }
}

/// 220
pub fn player_find_initialize(direction: i32) {
    let mut coord = with_state(|state| state.py.pos);

    if player_move_position(direction, &mut coord) {
        with_state_mut(|state| state.py.running_tracker = 1);
        set_find_direction(direction);
        set_find_prevdir(direction);
        set_find_breakright(false);
        set_find_breakleft(false);

        if with_state(|state| state.py.flags.blind < 1) {
            find_running_break(direction, coord);
        }
    } else {
        with_state_mut(|state| state.py.running_tracker = 0);
    }

    let (temporary_light_only, run_print_self) =
        with_state(|state| (state.py.temporary_light_only, state.options.run_print_self));
    if !temporary_light_only && !run_print_self {
        let pos = with_state(|state| state.py.pos);
        let ch = cave_get_tile_symbol(pos);
        panel_put_tile(ch, Coord { y: pos.y, x: pos.x });
    }

    player_move(direction, true);

    if with_state(|state| state.py.running_tracker == 0) {
        with_state_mut(|state| state.game.command_count = 0);
    }
}

/// 235
pub fn player_run_and_find() {
    let tracker = with_state(|state| state.py.running_tracker);
    with_state_mut(|state| state.py.running_tracker += 1);

    if tracker > 100 {
        terminal::print_message(Some("You stop running to catch your breath."));
        player_end_running();
        return;
    }

    player_move(find_direction(), true);
}

/// 246
pub fn player_end_running() {
    let pos = with_state_mut(|state| {
        if state.py.running_tracker == 0 {
            return None;
        }
        state.py.running_tracker = 0;
        Some(state.py.pos)
    });
    if let Some(pos) = pos {
        dungeon_move_character_light(pos, pos);
    }
}

/// 330
fn area_affect_stop_looking_at_squares(
    i: i32,
    dir: i32,
    new_dir: i32,
    coord: Coord_t,
    check_dir: &mut i32,
    dir_a: &mut i32,
    dir_b: &mut i32,
) -> bool {
    let (carrying_light, tile) = with_state(|state| {
        let tile = state.dg.floor[coord.y as usize][coord.x as usize];
        (state.py.carrying_light, tile)
    });

    let mut invisible = true;

    if carrying_light || tile.temporary_light || tile.permanent_light || tile.field_mark {
        if tile.treasure_id != 0 {
            let tile_id =
                with_state(|state| state.game.treasure.list[tile.treasure_id as usize].category_id);
            let ignore_doors = with_state(|state| state.options.run_ignore_doors);
            if tile_id != TV_INVIS_TRAP
                && tile_id != TV_SECRET_DOOR
                && (tile_id != TV_OPEN_DOOR || !ignore_doors)
            {
                player_end_running();
                return true;
            }
        }

        let creature_lit = with_state(|state| {
            tile.creature_id > 1 && state.monsters[tile.creature_id as usize].lit
        });
        if creature_lit {
            player_end_running();
            return true;
        }

        invisible = false;
    }

    if tile.feature_id <= MAX_OPEN_SPACE || invisible {
        if find_openarea() {
            if i < 0 {
                if find_breakright() {
                    player_end_running();
                    return true;
                }
            } else if i > 0 && find_breakleft() {
                player_end_running();
                return true;
            }
        } else if *dir_a == 0 {
            *dir_a = new_dir;
        } else if *dir_b != 0 || *dir_a != CYCLE[(CHOME[dir as usize] + i - 1) as usize] {
            player_end_running();
            return true;
        } else if (new_dir & 1) == 1 {
            *check_dir = CYCLE[(CHOME[dir as usize] + i - 2) as usize];
            *dir_b = new_dir;
        } else {
            *check_dir = CYCLE[(CHOME[dir as usize] + i + 1) as usize];
            *dir_b = *dir_a;
            *dir_a = new_dir;
        }
    } else if find_openarea() {
        if i < 0 {
            if find_breakleft() {
                player_end_running();
                return true;
            }
            set_find_breakright(true);
        } else if i > 0 {
            if find_breakright() {
                player_end_running();
                return true;
            }
            set_find_breakleft(true);
        }
    }

    false
}

/// 411
pub fn player_area_affect(direction: i32, coord: Coord_t) {
    let _ = direction;
    if with_state(|state| state.py.flags.blind >= 1) {
        return;
    }

    let mut check_dir = 0;
    let mut dir_a = 0;
    let mut dir_b = 0;

    let direction = find_prevdir();
    let max = (direction & 1) + 1;

    // call areaAffectStopLookingAtSquares but
    // ignore its bool return — keep scanning remaining adjacent squares so
    // dir_a/dir_b/find_break* may still update after a mid-scan stop.
    for i in -max..=max {
        let new_dir = CYCLE[(CHOME[direction as usize] + i) as usize];
        let mut spot = coord;
        if player_move_position(new_dir, &mut spot) {
            let _ = area_affect_stop_looking_at_squares(
                i,
                direction,
                new_dir,
                spot,
                &mut check_dir,
                &mut dir_a,
                &mut dir_b,
            );
        }
    }

    if find_openarea() {
        return;
    }

    let (run_examine_corners, run_cut_corners) = with_state(|state| {
        (
            state.options.run_examine_corners,
            state.options.run_cut_corners,
        )
    });

    if dir_b == 0 || (run_examine_corners && !run_cut_corners) {
        if dir_a != 0 {
            set_find_direction(dir_a);
        }

        if dir_b == 0 {
            set_find_prevdir(dir_a);
        } else {
            set_find_prevdir(dir_b);
        }

        return;
    }

    let mut location = coord;
    let _ = player_move_position(dir_a, &mut location);

    if !player_can_see_dungeon_wall(dir_a, location)
        || !player_can_see_dungeon_wall(check_dir, location)
    {
        if run_examine_corners
            && player_see_nothing(dir_a, location)
            && player_see_nothing(dir_b, location)
        {
            set_find_direction(dir_a);
            set_find_prevdir(dir_b);
        } else {
            player_end_running();
        }
    } else if run_cut_corners {
        set_find_direction(dir_b);
        set_find_prevdir(dir_b);
    } else {
        set_find_direction(dir_a);
        set_find_prevdir(dir_b);
    }
}
