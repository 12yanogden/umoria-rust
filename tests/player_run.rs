//! `player_run` running/find state machine tests.
#![allow(
    clippy::int_plus_one,
    reason = "test assertions use inclusive bound comparisons"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{
    Tile, MAX_OPEN_SPACE, TILE_CORR_FLOOR, TILE_GRANITE_WALL, TILE_LIGHT_FLOOR,
};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::inventory::Inventory;
use umoria::monster::Monster;
use umoria::player::player_disturb;
use umoria::player_run::{
    player_area_affect, player_end_running, player_find_initialize, player_run_and_find,
    test_reset_run_find_state, test_run_find_state, test_set_run_find_state, RunFindState,
};
use umoria::treasure::{TV_GOLD, TV_OPEN_DOOR};
use umoria::types::Coord_t;
use umoria::ui_io::test_set_ncurses_stub;

fn setup_dungeon(height: i16, width: i16) {
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
        for y in 0..height {
            for x in 0..width {
                s.dg.floor[y as usize][x as usize] = lit_wall();
            }
        }
    });
}

fn set_tile(coord: Coord_t, tile: Tile) {
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize] = tile;
    });
}

fn lit_floor(feature_id: u8) -> Tile {
    Tile {
        feature_id,
        permanent_light: true,
        ..Default::default()
    }
}

fn lit_wall() -> Tile {
    Tile {
        feature_id: TILE_GRANITE_WALL,
        permanent_light: true,
        ..Default::default()
    }
}

fn place_player(coord: Coord_t) {
    with_state_mut(|s| {
        s.py.pos = coord;
        s.py.carrying_light = true;
        s.py.misc.fos = 100;
        s.dg.floor[coord.y as usize][coord.x as usize].creature_id = 1;
        s.dg.floor[coord.y as usize][coord.x as usize].feature_id = TILE_CORR_FLOOR;
        s.dg.floor[coord.y as usize][coord.x as usize].permanent_light = true;
    });
}

fn assert_rng_unchanged_after(setup: impl Fn(), action: impl FnOnce()) {
    reset_for_new_game(Some(7));
    setup();
    let baseline = random_number(100);
    reset_for_new_game(Some(7));
    setup();
    action();
    assert_eq!(random_number(100), baseline);
}

fn run_until_stopped(max_steps: usize) -> Vec<Coord_t> {
    let mut path = Vec::new();
    for _ in 0..max_steps {
        let running = with_state(|s| s.py.running_tracker);
        if running == 0 {
            break;
        }
        let pos = with_state(|s| s.py.pos);
        path.push(pos);
        player_run_and_find();
    }
    path.push(with_state(|s| s.py.pos));
    path
}

// --------------------------------------------------------------------------
// 1. playerEndRunning
// --------------------------------------------------------------------------

#[test]
fn player_end_running_noop_when_not_running() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    place_player(Coord_t { y: 5, x: 5 });
    player_end_running();
    with_state(|s| assert_eq!(s.py.running_tracker, 0));
}

#[test]
fn player_end_running_clears_tracker_once() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    place_player(Coord_t { y: 5, x: 5 });
    with_state_mut(|s| s.py.running_tracker = 4);
    player_end_running();
    with_state(|s| assert_eq!(s.py.running_tracker, 0));
    player_end_running();
    with_state(|s| assert_eq!(s.py.running_tracker, 0));
}

// --------------------------------------------------------------------------
// 2. playerFindInitialize
// --------------------------------------------------------------------------

#[test]
fn player_find_initialize_enclosed_corridor_sets_corridor_flags() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_dungeon(15, 15);
    for x in 3..=11 {
        set_tile(Coord_t { y: 7, x }, lit_floor(TILE_CORR_FLOOR));
        set_tile(Coord_t { y: 6, x }, lit_wall());
        set_tile(Coord_t { y: 8, x }, lit_wall());
    }
    place_player(Coord_t { y: 7, x: 5 });
    test_reset_run_find_state();

    player_find_initialize(6);

    with_state(|s| assert_eq!(s.py.running_tracker, 1));
    assert_eq!(
        test_run_find_state(),
        RunFindState {
            find_openarea: false,
            find_breakright: true,
            find_breakleft: true,
            find_prevdir: 6,
            find_direction: 6,
        }
    );
}

#[test]
fn player_find_initialize_open_area_sets_open_flag() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_dungeon(15, 15);
    for y in 4..=10 {
        for x in 4..=10 {
            set_tile(Coord_t { y, x }, lit_floor(TILE_LIGHT_FLOOR));
        }
    }
    place_player(Coord_t { y: 7, x: 7 });
    test_reset_run_find_state();

    player_find_initialize(6);

    with_state(|s| assert_eq!(s.py.running_tracker, 1));
    assert!(test_run_find_state().find_openarea);
    assert!(!test_run_find_state().find_breakleft);
    assert!(!test_run_find_state().find_breakright);
}

#[test]
fn player_find_initialize_invalid_direction_clears_running() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_dungeon(5, 5);
    for y in 0..5 {
        for x in 0..5 {
            set_tile(Coord_t { y, x }, lit_floor(TILE_CORR_FLOOR));
        }
    }
    place_player(Coord_t { y: 0, x: 0 });
    player_find_initialize(4);
    with_state(|s| {
        assert_eq!(s.py.running_tracker, 0);
        assert_eq!(s.game.command_count, 0);
    });
}

// --------------------------------------------------------------------------
// 3. playerAreaAffect — table-driven geometry
// --------------------------------------------------------------------------

#[test]
fn player_area_affect_blind_is_noop() {
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    place_player(Coord_t { y: 5, x: 5 });
    with_state_mut(|s| {
        s.py.flags.blind = 1;
        s.py.running_tracker = 2;
    });
    test_set_run_find_state(RunFindState {
        find_openarea: false,
        find_breakright: true,
        find_breakleft: true,
        find_prevdir: 6,
        find_direction: 6,
    });
    player_area_affect(6, Coord_t { y: 5, x: 6 });
    assert_eq!(test_run_find_state().find_direction, 6);
    with_state(|s| assert_eq!(s.py.running_tracker, 2));
}

#[test]
fn player_area_affect_open_area_break_right_stops_run() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_dungeon(12, 12);
    for y in 4..=8 {
        for x in 4..=8 {
            set_tile(Coord_t { y, x }, lit_floor(TILE_LIGHT_FLOOR));
        }
    }
    set_tile(Coord_t { y: 5, x: 9 }, lit_floor(TILE_LIGHT_FLOOR));
    place_player(Coord_t { y: 5, x: 5 });
    with_state_mut(|s| s.py.running_tracker = 2);
    test_set_run_find_state(RunFindState {
        find_openarea: true,
        find_breakright: true,
        find_breakleft: false,
        find_prevdir: 6,
        find_direction: 6,
    });

    player_area_affect(6, Coord_t { y: 5, x: 6 });

    with_state(|s| assert_eq!(s.py.running_tracker, 0));
}

#[test]
fn player_area_affect_corridor_single_exit_sets_direction() {
    reset_for_new_game(None);
    setup_dungeon(12, 12);
    for x in 3..=9 {
        set_tile(Coord_t { y: 6, x }, lit_floor(TILE_CORR_FLOOR));
        set_tile(Coord_t { y: 5, x }, lit_wall());
        set_tile(Coord_t { y: 7, x }, lit_wall());
    }
    set_tile(Coord_t { y: 6, x: 9 }, lit_floor(TILE_CORR_FLOOR));
    place_player(Coord_t { y: 6, x: 5 });
    with_state_mut(|s| s.py.running_tracker = 2);
    test_set_run_find_state(RunFindState {
        find_openarea: false,
        find_breakright: true,
        find_breakleft: true,
        find_prevdir: 6,
        find_direction: 6,
    });

    player_area_affect(6, Coord_t { y: 6, x: 6 });

    assert_eq!(test_run_find_state().find_direction, 6);
    with_state(|s| assert_ne!(s.py.running_tracker, 0));
}

#[test]
fn player_area_affect_visible_treasure_stops_run() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    for y in 4..=6 {
        for x in 4..=6 {
            set_tile(Coord_t { y, x }, lit_floor(TILE_CORR_FLOOR));
        }
    }
    with_state_mut(|s| {
        s.game.treasure.current_id = 1;
        s.game.treasure.list[1] = Inventory {
            category_id: TV_GOLD,
            ..Default::default()
        };
    });
    set_tile(
        Coord_t { y: 5, x: 7 },
        Tile {
            feature_id: TILE_CORR_FLOOR,
            permanent_light: true,
            treasure_id: 1,
            ..Default::default()
        },
    );
    place_player(Coord_t { y: 5, x: 5 });
    with_state_mut(|s| s.py.running_tracker = 2);
    test_set_run_find_state(RunFindState {
        find_openarea: false,
        find_breakright: true,
        find_breakleft: true,
        find_prevdir: 6,
        find_direction: 6,
    });

    player_area_affect(6, Coord_t { y: 5, x: 6 });

    with_state(|s| assert_eq!(s.py.running_tracker, 0));
}

#[test]
fn player_area_affect_visible_monster_stops_run() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    for y in 4..=6 {
        for x in 4..=8 {
            set_tile(Coord_t { y, x }, lit_floor(TILE_CORR_FLOOR));
        }
    }
    set_tile(
        Coord_t { y: 5, x: 7 },
        Tile {
            feature_id: TILE_CORR_FLOOR,
            permanent_light: true,
            creature_id: 2,
            ..Default::default()
        },
    );
    with_state_mut(|s| {
        s.monsters[2] = Monster {
            lit: true,
            pos: Coord_t { y: 5, x: 7 },
            ..Default::default()
        };
    });
    place_player(Coord_t { y: 5, x: 5 });
    with_state_mut(|s| s.py.running_tracker = 2);
    test_set_run_find_state(RunFindState {
        find_openarea: false,
        find_breakright: true,
        find_breakleft: true,
        find_prevdir: 6,
        find_direction: 6,
    });

    player_area_affect(6, Coord_t { y: 5, x: 6 });

    with_state(|s| assert_eq!(s.py.running_tracker, 0));
}

#[test]
fn player_area_affect_open_door_respected_with_ignore_doors_off() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_dungeon(10, 10);
    for y in 4..=6 {
        for x in 4..=8 {
            set_tile(Coord_t { y, x }, lit_floor(TILE_CORR_FLOOR));
        }
    }
    with_state_mut(|s| {
        s.game.treasure.current_id = 1;
        s.game.treasure.list[1] = Inventory {
            category_id: TV_OPEN_DOOR,
            ..Default::default()
        };
        s.options.run_ignore_doors = false;
    });
    set_tile(
        Coord_t { y: 5, x: 7 },
        Tile {
            feature_id: TILE_CORR_FLOOR,
            permanent_light: true,
            treasure_id: 1,
            ..Default::default()
        },
    );
    place_player(Coord_t { y: 5, x: 5 });
    with_state_mut(|s| s.py.running_tracker = 2);
    test_set_run_find_state(RunFindState {
        find_openarea: false,
        find_breakright: true,
        find_breakleft: true,
        find_prevdir: 6,
        find_direction: 6,
    });

    player_area_affect(6, Coord_t { y: 5, x: 6 });

    with_state(|s| assert_eq!(s.py.running_tracker, 0));
}

// --------------------------------------------------------------------------
// 4. Corridor-run scripted layouts
// --------------------------------------------------------------------------

#[test]
fn run_straight_corridor_east_stops_at_end() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    for x in 2..=12 {
        set_tile(Coord_t { y: 10, x }, lit_floor(TILE_CORR_FLOOR));
        set_tile(Coord_t { y: 9, x }, lit_wall());
        set_tile(Coord_t { y: 11, x }, lit_wall());
    }
    set_tile(Coord_t { y: 10, x: 13 }, lit_wall());
    place_player(Coord_t { y: 10, x: 3 });
    test_reset_run_find_state();

    player_find_initialize(6);
    let path = run_until_stopped(20);

    assert_eq!(path.last(), Some(&Coord_t { y: 10, x: 12 }));
    with_state(|s| assert_eq!(s.py.running_tracker, 0));
}

#[test]
fn run_l_bend_turns_corner() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    for x in 5..=10 {
        set_tile(Coord_t { y: 10, x }, lit_floor(TILE_CORR_FLOOR));
        set_tile(Coord_t { y: 11, x }, lit_wall());
        if x != 10 {
            set_tile(Coord_t { y: 9, x }, lit_wall());
        }
    }
    set_tile(Coord_t { y: 9, x: 10 }, lit_floor(TILE_CORR_FLOOR));
    for y in 7..=9 {
        set_tile(Coord_t { y, x: 10 }, lit_floor(TILE_CORR_FLOOR));
        set_tile(Coord_t { y, x: 9 }, lit_wall());
        set_tile(Coord_t { y, x: 11 }, lit_wall());
    }
    place_player(Coord_t { y: 10, x: 5 });
    test_reset_run_find_state();

    player_find_initialize(6);
    let path = run_until_stopped(20);

    assert!(path.contains(&Coord_t { y: 10, x: 9 }));
    assert!(path.contains(&Coord_t { y: 9, x: 10 }));
    assert_eq!(path.last(), Some(&Coord_t { y: 7, x: 10 }));
}

#[test]
fn run_t_junction_stops_before_branch() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    for x in 4..=12 {
        set_tile(Coord_t { y: 10, x }, lit_floor(TILE_CORR_FLOOR));
        set_tile(Coord_t { y: 9, x }, lit_wall());
        set_tile(Coord_t { y: 11, x }, lit_wall());
    }
    set_tile(Coord_t { y: 9, x: 10 }, lit_floor(TILE_CORR_FLOOR));
    set_tile(Coord_t { y: 11, x: 10 }, lit_floor(TILE_CORR_FLOOR));
    place_player(Coord_t { y: 10, x: 4 });
    test_reset_run_find_state();

    player_find_initialize(6);
    let path = run_until_stopped(20);

    assert_eq!(path.last(), Some(&Coord_t { y: 10, x: 9 }));
    with_state(|s| assert_eq!(s.py.running_tracker, 0));
}

#[test]
fn run_dead_end_stops_one_before_wall() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    for x in 5..=9 {
        set_tile(Coord_t { y: 8, x }, lit_floor(TILE_CORR_FLOOR));
        set_tile(Coord_t { y: 7, x }, lit_wall());
        set_tile(Coord_t { y: 9, x }, lit_wall());
    }
    set_tile(Coord_t { y: 8, x: 10 }, lit_wall());
    place_player(Coord_t { y: 8, x: 5 });
    test_reset_run_find_state();

    player_find_initialize(6);
    let path = run_until_stopped(10);

    assert_eq!(path.last(), Some(&Coord_t { y: 8, x: 9 }));
    with_state(|s| assert_eq!(s.py.running_tracker, 0));
}

#[test]
fn run_open_area_stops_before_enclosed_space() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_dungeon(25, 25);
    for y in 8..=12 {
        for x in 8..=16 {
            set_tile(Coord_t { y, x }, lit_floor(TILE_LIGHT_FLOOR));
        }
    }
    for x in 13..=16 {
        set_tile(Coord_t { y: 10, x }, lit_floor(TILE_CORR_FLOOR));
        set_tile(Coord_t { y: 9, x }, lit_wall());
        set_tile(Coord_t { y: 11, x }, lit_wall());
    }
    place_player(Coord_t { y: 10, x: 8 });
    test_reset_run_find_state();

    player_find_initialize(6);
    let path = run_until_stopped(20);

    assert_eq!(path.last(), Some(&Coord_t { y: 10, x: 12 }));
    with_state(|s| assert_eq!(s.py.running_tracker, 0));
}

// --------------------------------------------------------------------------
// 5. Disturbance interruption
// --------------------------------------------------------------------------

#[test]
fn player_disturb_stops_active_run() {
    test_set_ncurses_stub(true);
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    for x in 2..=15 {
        set_tile(Coord_t { y: 10, x }, lit_floor(TILE_CORR_FLOOR));
        set_tile(Coord_t { y: 9, x }, lit_wall());
        set_tile(Coord_t { y: 11, x }, lit_wall());
    }
    place_player(Coord_t { y: 10, x: 3 });
    player_find_initialize(6);
    let before = with_state(|s| s.py.pos);
    player_disturb(1, 1);
    with_state(|s| {
        assert_eq!(s.py.running_tracker, 0);
        assert_eq!(s.py.pos, before);
    });
}

// --------------------------------------------------------------------------
// 6. No RNG in pure find helpers
// --------------------------------------------------------------------------

#[test]
fn player_area_affect_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            test_set_ncurses_stub(true);
            setup_dungeon(12, 12);
            for x in 3..=9 {
                set_tile(Coord_t { y: 6, x }, lit_floor(TILE_CORR_FLOOR));
                set_tile(Coord_t { y: 5, x }, lit_wall());
                set_tile(Coord_t { y: 7, x }, lit_wall());
            }
            place_player(Coord_t { y: 6, x: 5 });
            with_state_mut(|s| s.py.running_tracker = 2);
            test_set_run_find_state(RunFindState {
                find_openarea: false,
                find_breakright: true,
                find_breakleft: true,
                find_prevdir: 6,
                find_direction: 6,
            });
        },
        || player_area_affect(6, Coord_t { y: 6, x: 6 }),
    );
}

#[test]
fn player_end_running_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            test_set_ncurses_stub(true);
            setup_dungeon(10, 10);
            place_player(Coord_t { y: 5, x: 5 });
            with_state_mut(|s| s.py.running_tracker = 3);
        },
        player_end_running,
    );
}

#[test]
fn find_running_break_via_initialize_consumes_no_rng_when_blind() {
    assert_rng_unchanged_after(
        || {
            test_set_ncurses_stub(true);
            setup_dungeon(15, 15);
            for x in 3..=11 {
                set_tile(Coord_t { y: 7, x }, lit_floor(TILE_CORR_FLOOR));
                set_tile(Coord_t { y: 6, x }, lit_wall());
                set_tile(Coord_t { y: 8, x }, lit_wall());
            }
            place_player(Coord_t { y: 7, x: 5 });
            with_state_mut(|s| s.py.flags.blind = 1);
            test_reset_run_find_state();
        },
        || {
            let mut coord = with_state(|s| s.py.pos);
            umoria::player_move::player_move_position(6, &mut coord);
            umoria::player_run::test_set_run_find_state(RunFindState {
                find_openarea: false,
                find_breakright: false,
                find_breakleft: false,
                find_prevdir: 6,
                find_direction: 6,
            });
        },
    );
}

#[test]
fn max_open_space_constant_matches_expected() {
    assert_eq!(MAX_OPEN_SPACE, 3);
}
