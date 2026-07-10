//! Port of src/dungeon_los.cpp — line-of-sight and look commands.

use std::cell::RefCell;

use crate::config::monsters::MON_MAX_SIGHT;
use crate::data_creatures::CREATURES_LIST;
use crate::dungeon_tile::{
    Tile, MAX_OPEN_SPACE, MIN_CLOSED_SPACE, TILE_BOUNDARY_WALL, TILE_GRANITE_WALL, TILE_MAGMA_WALL,
    TILE_QUARTZ_WALL,
};
use crate::game::{get_all_directions, with_state, with_state_mut};
use crate::helpers::is_vowel;
use crate::identification::item_description;
use crate::recall::memory_recall;
use crate::treasure::{TV_INVIS_TRAP, TV_SECRET_DOOR};
use crate::types::{Coord_t, Obj_desc_t, MORIA_OBJ_DESC_SIZE_LEN};
use crate::ui::coord_inside_panel;
use crate::ui_io::terminal::{self, Coord};
use crate::ui_io::ESCAPE;

const GRADF: i32 = 10_000;

// C++ dungeon_los.cpp lines 234–237
const LOS_DIR_SET_FXY: [i32; 5] = [0, 1, 0, 0, -1];
const LOS_DIR_SET_FXX: [i32; 5] = [0, 0, -1, 1, 0];
const LOS_DIR_SET_FYY: [i32; 5] = [0, 0, 1, -1, 0];
const LOS_DIR_SET_FYX: [i32; 5] = [0, 1, 0, 0, -1];
const LOS_MAP_DIAGONALS1: [i32; 5] = [1, 3, 0, 2, 4];
const LOS_MAP_DIAGONALS2: [i32; 5] = [2, 1, 0, 4, 3];

#[derive(Default)]
struct LosLookState {
    fxx: i32,
    fxy: i32,
    fyx: i32,
    fyy: i32,
    num_places_seen: i32,
    hack_no_query: bool,
    rocks_and_objects: i32,
}

thread_local! {
    static LOS_LOOK: RefCell<LosLookState> = const { RefCell::new(LosLookState {
        fxx: 0,
        fxy: 0,
        fyx: 0,
        fyy: 0,
        num_places_seen: 0,
        hack_no_query: false,
        rocks_and_objects: 0,
    }) };
}

fn with_los_look<R>(f: impl FnOnce(&LosLookState) -> R) -> R {
    LOS_LOOK.with(|s| f(&s.borrow()))
}

fn with_los_look_mut<R>(f: impl FnOnce(&mut LosLookState) -> R) -> R {
    LOS_LOOK.with(|s| f(&mut s.borrow_mut()))
}

/// Test hook — reset file-static look state (dungeon_los.cpp globals).
#[doc(hidden)]
pub fn los_look_reset_for_test() {
    with_los_look_mut(|s| *s = LosLookState::default());
}

#[doc(hidden)]
pub fn los_look_set_frame(fxx: i32, fxy: i32, fyx: i32, fyy: i32) {
    with_los_look_mut(|s| {
        s.fxx = fxx;
        s.fxy = fxy;
        s.fyx = fyx;
        s.fyy = fyy;
    });
}

#[doc(hidden)]
pub fn los_look_set_hack_no_query(hack: bool) {
    with_los_look_mut(|s| s.hack_no_query = hack);
}

#[doc(hidden)]
pub fn los_look_set_rocks_and_objects(v: i32) {
    with_los_look_mut(|s| s.rocks_and_objects = v);
}

fn c_abs_i32(v: i32) -> i32 {
    (v as i64).abs() as i32
}

fn feature_blocks_los(y: i32, x: i32) -> bool {
    with_state(|s| s.dg.floor[y as usize][x as usize].feature_id >= MIN_CLOSED_SPACE)
}

fn snprintf_obj_desc(buf: &mut Obj_desc_t, formatted: &str) {
    let bytes = formatted.as_bytes();
    let n = bytes.len().min(MORIA_OBJ_DESC_SIZE_LEN - 1);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf[n] = 0;
}

/// C++ dungeon_los.cpp lines 25–176 — Joseph Hall integer LOS.
pub fn los(from: Coord_t, to: Coord_t) -> bool {
    let mut from = from;
    let mut to = to;
    let delta_x = to.x - from.x;
    let delta_y = to.y - from.y;

    // Adjacent? (lines 29–32)
    if delta_x < 2 && delta_x > -2 && delta_y < 2 && delta_y > -2 {
        return true;
    }

    if delta_x == 0 {
        if delta_y < 0 {
            std::mem::swap(&mut from.y, &mut to.y);
        }
        for yy in (from.y + 1)..to.y {
            if feature_blocks_los(yy, from.x) {
                return false;
            }
        }
        return true;
    }

    if delta_y == 0 {
        if delta_x < 0 {
            std::mem::swap(&mut from.x, &mut to.x);
        }
        for xx in (from.x + 1)..to.x {
            if feature_blocks_los(from.y, xx) {
                return false;
            }
        }
        return true;
    }

    // lines 79–82: scale = abs(delta_x * delta_y * 2)
    let delta_multiply = delta_x * delta_y;
    let scale_half = c_abs_i32(delta_multiply);
    let scale = scale_half << 1;
    let x_sign = if delta_x < 0 { -1 } else { 1 };
    let y_sign = if delta_y < 0 { -1 } else { 1 };
    let abs_delta_x = c_abs_i32(delta_x);
    let abs_delta_y = c_abs_i32(delta_y);

    if abs_delta_x >= abs_delta_y {
        // Major axis x (lines 90–135)
        let dy_init = delta_y * delta_y;
        let slope = dy_init << 1;
        let mut dy = dy_init;
        let mut xx = from.x + x_sign;
        let mut yy = if dy_init == scale_half {
            dy -= scale;
            from.y + y_sign
        } else {
            from.y
        };

        while to.x - xx != 0 {
            if feature_blocks_los(yy, xx) {
                return false;
            }
            dy += slope;
            if dy < scale_half {
                xx += x_sign;
            } else if dy > scale_half {
                yy += y_sign;
                if feature_blocks_los(yy, xx) {
                    return false;
                }
                xx += x_sign;
                dy -= scale;
            } else {
                // dy == scale_half — corner tile (lines 128–132)
                xx += x_sign;
                yy += y_sign;
                dy -= scale;
            }
        }
        return true;
    }

    // Major axis y (lines 138–174)
    let dx_init = delta_x * delta_x;
    let slope = dx_init << 1;
    let mut dx = dx_init;
    let mut yy = from.y + y_sign;
    let mut xx = if dx_init == scale_half {
        dx -= scale;
        from.x + x_sign
    } else {
        from.x
    };

    while to.y - yy != 0 {
        if feature_blocks_los(yy, xx) {
            return false;
        }
        dx += slope;
        if dx < scale_half {
            yy += y_sign;
        } else if dx > scale_half {
            xx += x_sign;
            if feature_blocks_los(yy, xx) {
                return false;
            }
            yy += y_sign;
            dx -= scale;
        } else {
            xx += x_sign;
            yy += y_sign;
            dx -= scale;
        }
    }
    true
}

/// C++ dungeon_los.cpp lines 258–357.
pub fn look() {
    if with_state(|s| s.py.flags.blind > 0) {
        terminal::print_message(Some("You can't see a damn thing!"));
        return;
    }

    if with_state(|s| s.py.flags.image > 0) {
        terminal::print_message(Some(
            "You can't believe what you are seeing! It's like a dream!",
        ));
        return;
    }

    let mut dir = 0i32;
    if !get_all_directions("Look which direction?", &mut dir) {
        return;
    }

    with_los_look_mut(|s| {
        s.num_places_seen = 0;
        s.rocks_and_objects = 0;
        s.hack_no_query = false;
    });

    let mut dummy = false;
    if look_see(Coord_t { y: 0, x: 0 }, &mut dummy) {
        return;
    }

    let mut abort;
    loop {
        abort = false;
        if dir == 5 {
            for i in 1..=4 {
                with_los_look_mut(|s| {
                    s.fxx = LOS_DIR_SET_FXX[i];
                    s.fyx = LOS_DIR_SET_FYX[i];
                    s.fxy = LOS_DIR_SET_FXY[i];
                    s.fyy = LOS_DIR_SET_FYY[i];
                });
                if look_ray(0, 2 * GRADF - 1, 1) {
                    abort = true;
                    break;
                }
                with_los_look_mut(|s| {
                    s.fxy = -s.fxy;
                    s.fyy = -s.fyy;
                });
                if look_ray(0, 2 * GRADF, 2) {
                    abort = true;
                    break;
                }
            }
        } else if (dir & 1) == 0 {
            let i = dir >> 1;
            with_los_look_mut(|s| {
                s.fxx = LOS_DIR_SET_FXX[i as usize];
                s.fyx = LOS_DIR_SET_FYX[i as usize];
                s.fxy = LOS_DIR_SET_FXY[i as usize];
                s.fyy = LOS_DIR_SET_FYY[i as usize];
            });
            if look_ray(0, GRADF, 1) {
                abort = true;
            } else {
                with_los_look_mut(|s| {
                    s.fxy = -s.fxy;
                    s.fyy = -s.fyy;
                });
                abort = look_ray(0, GRADF, 2);
            }
        } else {
            let i = LOS_MAP_DIAGONALS1[(dir >> 1) as usize];
            with_los_look_mut(|s| {
                s.fxx = LOS_DIR_SET_FXX[i as usize];
                s.fyx = LOS_DIR_SET_FYX[i as usize];
                s.fxy = -LOS_DIR_SET_FXY[i as usize];
                s.fyy = -LOS_DIR_SET_FYY[i as usize];
            });
            if look_ray(1, 2 * GRADF, GRADF) {
                abort = true;
            } else {
                let i = LOS_MAP_DIAGONALS2[(dir >> 1) as usize];
                with_los_look_mut(|s| {
                    s.fxx = LOS_DIR_SET_FXX[i as usize];
                    s.fyx = LOS_DIR_SET_FYX[i as usize];
                    s.fxy = LOS_DIR_SET_FXY[i as usize];
                    s.fyy = LOS_DIR_SET_FYY[i as usize];
                });
                abort = look_ray(1, 2 * GRADF - 1, GRADF);
            }
        }

        with_los_look_mut(|s| s.rocks_and_objects += 1);

        let highlight = with_state(|s| s.options.highlight_seams);
        let rocks = with_los_look(|s| s.rocks_and_objects);
        if abort || !highlight || rocks >= 2 {
            break;
        }
    }

    if abort {
        terminal::print_message(Some("--Aborting look--"));
        return;
    }

    let places_seen = with_los_look(|s| s.num_places_seen);
    if places_seen != 0 {
        if dir == 5 {
            terminal::print_message(Some("That's all you see."));
        } else {
            terminal::print_message(Some("That's all you see in that direction."));
        }
    } else if dir == 5 {
        terminal::print_message(Some("You see nothing of interest."));
    } else {
        terminal::print_message(Some("You see nothing of interest in that direction."));
    }
}

/// C++ dungeon_los.cpp lines 375–463.
pub fn look_ray(y: i32, mut from: i32, to: i32) -> bool {
    if from <= to || y > i32::from(MON_MAX_SIGHT) {
        return false;
    }

    // lines 386–389
    let mut x = ((i64::from(GRADF) * i64::from(2 * y - 1)) / i64::from(from) + 1) as i32;
    if x <= 0 {
        x = 1;
    }

    let mut max_x = (((i64::from(GRADF) * i64::from(2 * y + 1)) - 1) / i64::from(to)) as i32;
    if max_x > i32::from(MON_MAX_SIGHT) {
        max_x = i32::from(MON_MAX_SIGHT);
    }
    if max_x < x {
        return false;
    }

    with_los_look_mut(|s| {
        s.hack_no_query = (y == 0 && to > 1) || (y == x && from < GRADF * 2);
    });

    let mut transparent = false;
    if look_see(Coord_t { y, x }, &mut transparent) {
        return true;
    }

    if y == x {
        with_los_look_mut(|s| s.hack_no_query = false);
    }

    // C++ `goto init_transparent` when the first cell is already transparent:
    // skip the recursive lookRay / find-next-window and jump to window extension.
    let mut skip_to_extend_window = transparent;

    loop {
        if !skip_to_extend_window {
            // Look down the window we've found.
            if look_ray(
                y + 1,
                from,
                ((i64::from(2 * y + 1) * i64::from(GRADF)) / i64::from(x)) as i32,
            ) {
                return true;
            }

            // Find the start of next window.
            loop {
                if x == max_x {
                    return false;
                }

                from = ((i64::from(2 * y - 1) * i64::from(GRADF)) / i64::from(x)) as i32;
                if from <= to {
                    return false;
                }

                x += 1;
                if look_see(Coord_t { y, x }, &mut transparent) {
                    return true;
                }
                if transparent {
                    break;
                }
            }
        }
        skip_to_extend_window = false;

        // init_transparent: find the end of this window of visibility.
        loop {
            if x == max_x {
                return look_ray(y + 1, from, to);
            }

            x += 1;
            if look_see(Coord_t { y, x }, &mut transparent) {
                return true;
            }
            if !transparent {
                break;
            }
        }
    }
}

/// C++ dungeon_los.cpp lines 465–575.
pub fn look_see(mut coord: Coord_t, transparent: &mut bool) -> bool {
    if coord.x < 0 || coord.y < 0 || coord.y > coord.x {
        let msg = format!("Illegal call to lookSee({}, {})", coord.y, coord.x);
        terminal::print_message(Some(&msg));
    }

    let mut description = if coord.x == 0 && coord.y == 0 {
        "You are on"
    } else {
        "You see"
    };

    let j = with_state(|s| s.py.pos.x + with_los_look(|los| los.fxx * coord.x + los.fxy * coord.y));
    coord.y =
        with_state(|s| s.py.pos.y + with_los_look(|los| los.fyx * coord.x + los.fyy * coord.y));
    coord.x = j;

    if !coord_inside_panel(coord) {
        *transparent = false;
        return false;
    }

    let tile: Tile = with_state(|s| s.dg.floor[coord.y as usize][coord.x as usize]);
    *transparent = tile.feature_id <= MAX_OPEN_SPACE;

    if with_los_look(|s| s.hack_no_query) {
        return false;
    }

    let mut key = ESCAPE;
    let mut msg = [0u8; MORIA_OBJ_DESC_SIZE_LEN];

    let rocks_and_objects = with_los_look(|s| s.rocks_and_objects);
    if rocks_and_objects == 0 && tile.creature_id > 1 {
        let lit = with_state(|s| s.monsters[tile.creature_id as usize].lit);
        if lit {
            let creature_id =
                with_state(|s| s.monsters[tile.creature_id as usize].creature_id as usize);
            let article = if is_vowel(CREATURES_LIST[creature_id].name.as_bytes()[0]) {
                "an"
            } else {
                "a"
            };
            snprintf_obj_desc(
                &mut msg,
                &format!(
                    "{description} {article} {}. [(r)ecall]",
                    CREATURES_LIST[creature_id].name
                ),
            );
            description = "It is on";
            terminal::put_string_clear_to_eol(c_str_from_obj_desc(&msg), Coord { y: 0, x: 0 });
            terminal::panel_move_cursor(Coord {
                y: coord.y,
                x: coord.x,
            });
            key = terminal::get_key_input();

            if key == b'r' || key == b'R' {
                terminal::terminal_save_screen();
                key = memory_recall(creature_id as i32);
                terminal::terminal_restore_screen();
            }
        }
    }

    if tile.temporary_light || tile.permanent_light || tile.field_mark {
        let mut wall_description: Option<&str> = None;

        if tile.treasure_id != 0 {
            let category_id =
                with_state(|s| s.game.treasure.list[tile.treasure_id as usize].category_id);
            if category_id == TV_SECRET_DOOR {
                // C++ goto granite (lines 518–519, 537–545)
                wall_description = if msg[0] != 0 {
                    Some("a granite wall")
                } else {
                    None
                };
            } else if rocks_and_objects == 0 && category_id != TV_INVIS_TRAP {
                let mut obj_string = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
                with_state_mut(|s| {
                    item_description(
                        &mut obj_string,
                        s.game.treasure.list[tile.treasure_id as usize],
                        true,
                    );
                });
                snprintf_obj_desc(
                    &mut msg,
                    &format!(
                        "{description} {} ---pause---",
                        c_str_from_obj_desc(&obj_string)
                    ),
                );
                description = "It is in";
                terminal::put_string_clear_to_eol(c_str_from_obj_desc(&msg), Coord { y: 0, x: 0 });
                terminal::panel_move_cursor(Coord {
                    y: coord.y,
                    x: coord.x,
                });
                key = terminal::get_key_input();
            }
        }

        if wall_description.is_none()
            && (rocks_and_objects != 0 || msg[0] != 0)
            && tile.feature_id >= MIN_CLOSED_SPACE
        {
            wall_description = match tile.feature_id {
                TILE_BOUNDARY_WALL | TILE_GRANITE_WALL => {
                    if msg[0] != 0 {
                        Some("a granite wall")
                    } else {
                        None
                    }
                }
                TILE_MAGMA_WALL => Some("some dark rock"),
                TILE_QUARTZ_WALL => Some("a quartz vein"),
                _ => None,
            };
        }

        if let Some(wall_desc) = wall_description {
            snprintf_obj_desc(&mut msg, &format!("{description} {wall_desc} ---pause---"));
            terminal::put_string_clear_to_eol(c_str_from_obj_desc(&msg), Coord { y: 0, x: 0 });
            terminal::panel_move_cursor(Coord {
                y: coord.y,
                x: coord.x,
            });
            key = terminal::get_key_input();
        }
    }

    if msg[0] != 0 {
        with_los_look_mut(|s| s.num_places_seen += 1);
        if key == ESCAPE {
            return true;
        }
    }

    false
}

fn c_str_from_obj_desc(buf: &[u8]) -> &str {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    std::str::from_utf8(&buf[..end]).unwrap_or("")
}
