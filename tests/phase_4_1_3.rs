//! Phase 4.1.3 — dungeon_los.cpp (los, look, lookRay, lookSee).
#![allow(clippy::int_plus_one, clippy::too_many_lines)]

use umoria::config::monsters::MON_MAX_SIGHT;
use umoria::dungeon::MAX_HEIGHT;
use umoria::dungeon::MAX_WIDTH;
use umoria::dungeon_los::{
    look, look_ray, look_see, los, los_look_reset_for_test, los_look_set_frame,
    los_look_set_hack_no_query, los_look_set_rocks_and_objects,
};
use umoria::dungeon_tile::{
    Tile, MIN_CLOSED_SPACE, TILE_CORR_FLOOR, TILE_GRANITE_WALL, TILE_LIGHT_FLOOR, TILE_MAGMA_WALL,
};
use umoria::game::{reset_for_new_game, with_state, with_state_mut};
use umoria::types::Coord_t;
use umoria::ui::panel_bounds_fields;

fn setup_dungeon(height: i16, width: i16) {
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
    });
}

fn fill_feature(feature_id: u8) {
    with_state_mut(|s| {
        for y in 0..s.dg.height {
            for x in 0..s.dg.width {
                s.dg.floor[y as usize][x as usize].feature_id = feature_id;
            }
        }
    });
}

fn set_tile(coord: Coord_t, tile: Tile) {
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize] = tile;
    });
}

fn setup_player_panel(row: i32, col: i32, pos: Coord_t) {
    let bounds = panel_bounds_fields(row, col);
    with_state_mut(|s| {
        s.py.pos = pos;
        s.dg.panel.row = row;
        s.dg.panel.col = col;
        s.dg.panel.top = bounds.top;
        s.dg.panel.bottom = bounds.bottom;
        s.dg.panel.left = bounds.left;
        s.dg.panel.right = bounds.right;
        s.dg.panel.row_prt = bounds.row_prt;
        s.dg.panel.col_prt = bounds.col_prt;
    });
}

/// Independent C++-literal reference for los parity (dungeon_los.cpp lines 25–176).
fn reference_los(from: Coord_t, to: Coord_t) -> bool {
    let mut from = from;
    let mut to = to;
    let delta_x = to.x - from.x;
    let delta_y = to.y - from.y;

    if delta_x < 2 && delta_x > -2 && delta_y < 2 && delta_y > -2 {
        return true;
    }

    if delta_x == 0 {
        if delta_y < 0 {
            std::mem::swap(&mut from.y, &mut to.y);
        }
        for yy in (from.y + 1)..to.y {
            let blocked = with_state(|s| {
                s.dg.floor[yy as usize][from.x as usize].feature_id >= MIN_CLOSED_SPACE
            });
            if blocked {
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
            let blocked = with_state(|s| {
                s.dg.floor[from.y as usize][xx as usize].feature_id >= MIN_CLOSED_SPACE
            });
            if blocked {
                return false;
            }
        }
        return true;
    }

    let c_abs = |v: i32| (v as i64).abs() as i32;

    let delta_multiply = delta_x * delta_y;
    let scale_half = c_abs(delta_multiply);
    let scale = scale_half << 1;
    let x_sign = if delta_x < 0 { -1 } else { 1 };
    let y_sign = if delta_y < 0 { -1 } else { 1 };
    let abs_delta_x = c_abs(delta_x);
    let abs_delta_y = c_abs(delta_y);

    if abs_delta_x >= abs_delta_y {
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
            if with_state(|s| s.dg.floor[yy as usize][xx as usize].feature_id >= MIN_CLOSED_SPACE) {
                return false;
            }
            dy += slope;
            if dy < scale_half {
                xx += x_sign;
            } else if dy > scale_half {
                yy += y_sign;
                if with_state(|s| {
                    s.dg.floor[yy as usize][xx as usize].feature_id >= MIN_CLOSED_SPACE
                }) {
                    return false;
                }
                xx += x_sign;
                dy -= scale;
            } else {
                xx += x_sign;
                yy += y_sign;
                dy -= scale;
            }
        }
        return true;
    }

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
        if with_state(|s| s.dg.floor[yy as usize][xx as usize].feature_id >= MIN_CLOSED_SPACE) {
            return false;
        }
        dx += slope;
        if dx < scale_half {
            yy += y_sign;
        } else if dx > scale_half {
            xx += x_sign;
            if with_state(|s| s.dg.floor[yy as usize][xx as usize].feature_id >= MIN_CLOSED_SPACE) {
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

// ---------------------------------------------------------------------------
// 1. los exhaustive parity
// ---------------------------------------------------------------------------

#[test]
fn los_adjacent_always_true_even_through_walls() {
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    fill_feature(TILE_GRANITE_WALL);
    let from = Coord_t { y: 10, x: 10 };
    let to = Coord_t { y: 11, x: 11 };
    assert!(los(from, to));
    assert!(reference_los(from, to));
}

#[test]
fn los_horizontal_clear_and_blocked() {
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    fill_feature(TILE_LIGHT_FLOOR);
    set_tile(
        Coord_t { y: 10, x: 12 },
        Tile {
            feature_id: TILE_GRANITE_WALL,
            ..Default::default()
        },
    );
    let from = Coord_t { y: 10, x: 10 };
    let to = Coord_t { y: 10, x: 15 };
    assert!(!los(from, to));
    assert!(!reference_los(from, to));
    set_tile(
        Coord_t { y: 10, x: 12 },
        Tile {
            feature_id: TILE_LIGHT_FLOOR,
            ..Default::default()
        },
    );
    assert!(los(from, to));
}

#[test]
fn los_vertical_reverse_delta() {
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    fill_feature(TILE_CORR_FLOOR);
    set_tile(
        Coord_t { y: 8, x: 5 },
        Tile {
            feature_id: MIN_CLOSED_SPACE,
            ..Default::default()
        },
    );
    let from = Coord_t { y: 10, x: 5 };
    let to = Coord_t { y: 6, x: 5 };
    assert!(!los(from, to));
    assert_eq!(los(from, to), reference_los(from, to));
}

#[test]
fn los_diagonal_corner_slope_equality() {
    reset_for_new_game(None);
    setup_dungeon(30, 30);
    fill_feature(TILE_LIGHT_FLOOR);
    // dy == scale_half: delta (2,2) → first stepped cell is (11,11)
    set_tile(
        Coord_t { y: 11, x: 11 },
        Tile {
            feature_id: TILE_GRANITE_WALL,
            ..Default::default()
        },
    );
    let from = Coord_t { y: 10, x: 10 };
    let to = Coord_t { y: 12, x: 12 };
    assert!(!los(from, to));
    assert_eq!(los(from, to), reference_los(from, to));
}

#[test]
fn los_dense_grid_matches_reference() {
    reset_for_new_game(None);
    setup_dungeon(25, 25);
    fill_feature(TILE_LIGHT_FLOOR);
    for &(wy, wx) in &[(8, 8), (12, 14), (15, 10), (18, 18)] {
        set_tile(
            Coord_t { y: wy, x: wx },
            Tile {
                feature_id: TILE_GRANITE_WALL,
                ..Default::default()
            },
        );
    }
    for from_y in 5..20 {
        for from_x in 5..20 {
            for to_y in 5..20 {
                for to_x in 5..20 {
                    let from = Coord_t {
                        y: from_y,
                        x: from_x,
                    };
                    let to = Coord_t { y: to_y, x: to_x };
                    assert_eq!(
                        los(from, to),
                        reference_los(from, to),
                        "from {from:?} to {to:?}"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Integer-semantics probe at boundaries
// ---------------------------------------------------------------------------

#[test]
fn los_large_delta_within_cpp_comment_limit() {
    reset_for_new_game(None);
    setup_dungeon(MAX_HEIGHT as i16, MAX_WIDTH as i16);
    fill_feature(TILE_LIGHT_FLOOR);
    let from = Coord_t { y: 5, x: 5 };
    let to = Coord_t { y: 65, x: 65 };
    assert!(los(from, to));
    assert_eq!(los(from, to), reference_los(from, to));
}

#[test]
fn los_near_max_width_horizontal() {
    reset_for_new_game(None);
    setup_dungeon(MAX_HEIGHT as i16, MAX_WIDTH as i16);
    fill_feature(TILE_LIGHT_FLOOR);
    let from = Coord_t { y: 33, x: 2 };
    let to = Coord_t {
        y: 33,
        x: i32::from(MAX_WIDTH) - 2,
    };
    assert!(los(from, to));
    set_tile(
        Coord_t {
            y: 33,
            x: i32::from(MAX_WIDTH) / 2,
        },
        Tile {
            feature_id: TILE_MAGMA_WALL,
            ..Default::default()
        },
    );
    assert!(!los(from, to));
}

// ---------------------------------------------------------------------------
// 2. lookSee / lookRay unit parity
// ---------------------------------------------------------------------------

#[test]
fn look_see_open_floor_is_transparent() {
    reset_for_new_game(None);
    setup_dungeon(30, 30);
    fill_feature(TILE_LIGHT_FLOOR);
    setup_player_panel(1, 0, Coord_t { y: 15, x: 15 });
    los_look_reset_for_test();
    los_look_set_frame(0, 1, 1, 0);
    los_look_set_hack_no_query(true);

    let mut transparent = false;
    let abort = look_see(Coord_t { y: 1, x: 3 }, &mut transparent);
    assert!(!abort);
    assert!(transparent);
}

#[test]
fn look_see_wall_not_transparent() {
    reset_for_new_game(None);
    setup_dungeon(30, 30);
    fill_feature(TILE_LIGHT_FLOOR);
    set_tile(
        Coord_t { y: 18, x: 16 },
        Tile {
            feature_id: TILE_GRANITE_WALL,
            ..Default::default()
        },
    );
    setup_player_panel(1, 0, Coord_t { y: 15, x: 15 });
    los_look_reset_for_test();
    los_look_set_frame(0, 1, 1, 0);
    los_look_set_hack_no_query(true);

    let mut transparent = false;
    let abort = look_see(Coord_t { y: 1, x: 3 }, &mut transparent);
    assert!(!abort);
    assert!(!transparent);
}

#[test]
fn look_see_outside_panel_not_transparent() {
    reset_for_new_game(None);
    setup_dungeon(30, 30);
    fill_feature(TILE_LIGHT_FLOOR);
    setup_player_panel(0, 0, Coord_t { y: 5, x: 5 });
    los_look_reset_for_test();
    los_look_set_frame(0, 1, 1, 0);
    los_look_set_hack_no_query(true);

    let mut transparent = true;
    let abort = look_see(Coord_t { y: 50, x: 50 }, &mut transparent);
    assert!(!abort);
    assert!(!transparent);
}

#[test]
fn look_ray_returns_false_when_from_le_to() {
    reset_for_new_game(None);
    setup_dungeon(30, 30);
    fill_feature(TILE_LIGHT_FLOOR);
    setup_player_panel(1, 0, Coord_t { y: 15, x: 15 });
    los_look_reset_for_test();
    los_look_set_frame(0, 1, 1, 0);
    los_look_set_hack_no_query(true);
    los_look_set_rocks_and_objects(0);

    assert!(!look_ray(0, 100, 200));
}

#[test]
fn look_ray_y_exceeds_mon_max_sight() {
    reset_for_new_game(None);
    setup_dungeon(30, 30);
    fill_feature(TILE_LIGHT_FLOOR);
    setup_player_panel(1, 0, Coord_t { y: 15, x: 15 });
    los_look_reset_for_test();
    los_look_set_frame(0, 1, 1, 0);

    assert!(!look_ray(i32::from(MON_MAX_SIGHT) + 1, 20_000, 10_000));
}

#[test]
fn look_ray_open_floor_scan_no_abort_with_hack() {
    reset_for_new_game(None);
    setup_dungeon(30, 30);
    fill_feature(TILE_LIGHT_FLOOR);
    setup_player_panel(1, 0, Coord_t { y: 15, x: 15 });
    los_look_reset_for_test();
    los_look_set_frame(0, 1, 1, 0);
    los_look_set_hack_no_query(true);
    los_look_set_rocks_and_objects(0);

    assert!(!look_ray(0, 10_000, 1));
}

// ---------------------------------------------------------------------------
// 3. look behavioral / early exits
// ---------------------------------------------------------------------------

#[test]
fn look_blind_returns_early() {
    reset_for_new_game(None);
    umoria::ui_io::test_set_ncurses_stub(true);
    with_state_mut(|s| s.py.flags.blind = 1);
    look();
    umoria::ui_io::test_set_ncurses_stub(false);
}

#[test]
fn look_image_returns_early() {
    reset_for_new_game(None);
    umoria::ui_io::test_set_ncurses_stub(true);
    with_state_mut(|s| s.py.flags.image = 1);
    look();
    umoria::ui_io::test_set_ncurses_stub(false);
}

#[test]
#[ignore = "TODO(phase_1.5): capture"]
fn look_transcript_parity_newchar_seed42() {
    // Recorded-input look sessions vs C++ transcript — phase_1 harness.
}

#[test]
#[ignore = "TODO(phase_5.5): get_all_directions"]
fn look_directional_scan_order() {
    reset_for_new_game(None);
    setup_dungeon(30, 30);
    fill_feature(TILE_LIGHT_FLOOR);
    setup_player_panel(1, 0, Coord_t { y: 15, x: 15 });
    umoria::ui_io::test_set_ncurses_stub(true);
    look();
    umoria::ui_io::test_set_ncurses_stub(false);
}
