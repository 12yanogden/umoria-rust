//! Phase 4.2.5 — turn engine & visibility parity.
#![allow(clippy::int_plus_one)]

mod common;

use umoria::config::monsters::defense::CD_INFRA;
use umoria::config::monsters::move_flags::{CM_INVISIBLE, CM_MOVE_NORMAL};
use umoria::config::monsters::{self, MON_MAX_SIGHT};
use umoria::dungeon::{coord_distance_between, MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::TILE_LIGHT_FLOOR;
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::monster::{
    memory_update_recall, monster_attacking_update, monster_is_visible, monster_movement_rate,
    monster_name_description, monster_sleep, monster_update_visibility, print_monster_action_text,
    update_monsters, Monster, MON_TOTAL_ALLOCATIONS,
};
use umoria::types::Coord_t;
use umoria::ui::panel_bounds_fields;
use umoria::ui_io::test_set_ncurses_stub;

const FLOATING_EYE_ID: u16 = 18;
const GREY_MUSHROOM_ID: u16 = 8;
const GIANT_WHITE_RAT_ID: u16 = 53;
const INVISIBLE_STALKER_ID: u16 = 232;

fn setup_dungeon(height: i16, width: i16) {
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Default::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                s.dg.floor[y as usize][x as usize].feature_id = TILE_LIGHT_FLOOR;
            }
        }
    });
}

fn reset_monster_slots() {
    with_state_mut(|s| {
        s.next_free_monster_id = i16::from(monsters::MON_MIN_INDEX_ID);
        s.monsters = [Monster::default(); MON_TOTAL_ALLOCATIONS as usize];
        s.hack_monptr = -1;
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
        s.py.flags.rest = 0;
        s.py.flags.paralysis = 0;
        s.py.flags.aggravate = false;
        s.py.flags.blind = 0;
        s.py.flags.status = 0;
        s.py.flags.see_invisible = false;
        s.py.flags.see_infra = 0;
        s.py.carrying_light = false;
        s.py.running_tracker = 0;
        s.game.wizard_mode = false;
        s.game.character_is_dead = false;
        s.game.command_count = 5;
        s.screen_has_changed = false;
    });
}

fn light_monster_tile(coord: Coord_t) {
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize].permanent_light = true;
    });
}

fn place_monster(id: i32, creature_id: u16, hp: i16, coord: Coord_t, lit: bool) {
    with_state_mut(|s| {
        s.monsters[id as usize] = Monster {
            hp,
            creature_id,
            pos: coord,
            distance_from_player: coord_distance_between(s.py.pos, coord) as u8,
            lit,
            ..Default::default()
        };
        s.dg.floor[coord.y as usize][coord.x as usize].creature_id = id as u8;
        if id + 1 >= i32::from(s.next_free_monster_id) {
            s.next_free_monster_id = (id + 1) as i16;
        }
    });
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

fn message_text(id: i16) -> String {
    with_state(|s| {
        let idx = id.rem_euclid(umoria::types::MESSAGE_HISTORY_SIZE as i16) as usize;
        let msg = &s.messages[idx];
        let end = msg.iter().position(|&b| b == 0).unwrap_or(msg.len());
        String::from_utf8_lossy(&msg[..end]).into_owned()
    })
}

// ---------------------------------------------------------------------------
// 1. monsterMovementRate parity
// ---------------------------------------------------------------------------
#[test]
fn monster_movement_rate_positive_speed_matches_rest_gate() {
    reset_for_new_game(None);
    with_state_mut(|s| s.py.flags.rest = 0);
    assert_eq!(monster_movement_rate(5), 5);
    with_state_mut(|s| s.py.flags.rest = 1);
    assert_eq!(monster_movement_rate(5), 1);
    assert_eq!(next_random_pair(100), (100, 48));
}

#[test]
fn monster_movement_rate_negative_speed_uses_game_turn_mod_seed42() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| s.dg.game_turn = 0);
    assert_eq!(monster_movement_rate(-1), 1);
    with_state_mut(|s| s.dg.game_turn = 1);
    assert_eq!(monster_movement_rate(-1), 0);
    with_state_mut(|s| s.dg.game_turn = 4);
    assert_eq!(monster_movement_rate(-2), 1);
    assert_eq!(next_random_pair(100), (100, 2));
}

// ---------------------------------------------------------------------------
// 2. monsterIsVisible / monsterUpdateVisibility / monsterMakeVisible parity
// ---------------------------------------------------------------------------
#[test]
fn monster_is_visible_lit_floor_without_invisible_flag() {
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    let coord = Coord_t { y: 10, x: 12 };
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize].permanent_light = true;
        s.monsters[2] = Monster {
            creature_id: GIANT_WHITE_RAT_ID,
            pos: coord,
            distance_from_player: 2,
            ..Default::default()
        };
    });
    assert!(monster_is_visible(&with_state(|s| s.monsters[2])));
}

#[test]
fn monster_is_visible_see_invisible_sets_recall_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    let coord = Coord_t { y: 10, x: 12 };
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize].permanent_light = true;
        s.py.flags.see_invisible = true;
        s.monsters[2] = Monster {
            creature_id: INVISIBLE_STALKER_ID,
            pos: coord,
            distance_from_player: 2,
            ..Default::default()
        };
    });
    assert!(monster_is_visible(&with_state(|s| s.monsters[2])));
    with_state(|s| {
        assert_eq!(
            s.creature_recall[INVISIBLE_STALKER_ID as usize].movement & CM_INVISIBLE,
            CM_INVISIBLE
        );
    });
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn monster_is_visible_infra_branch_sets_recall() {
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    let coord = Coord_t { y: 10, x: 12 };
    with_state_mut(|s| {
        s.py.flags.see_infra = 3;
        s.monsters[2] = Monster {
            creature_id: FLOATING_EYE_ID,
            pos: coord,
            distance_from_player: 2,
            ..Default::default()
        };
    });
    assert!(monster_is_visible(&with_state(|s| s.monsters[2])));
    with_state(|s| {
        assert_eq!(
            s.creature_recall[FLOATING_EYE_ID as usize].defenses & CD_INFRA,
            CD_INFRA
        );
    });
}

#[test]
fn monster_update_visibility_wizard_sight_lights_monster() {
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    test_set_ncurses_stub(true);
    place_monster(2, GIANT_WHITE_RAT_ID, 10, Coord_t { y: 10, x: 12 }, false);
    with_state_mut(|s| {
        s.game.wizard_mode = true;
        s.monsters[2].distance_from_player = 2;
    });

    monster_update_visibility(2);

    with_state(|s| {
        assert!(s.monsters[2].lit);
        assert!(s.screen_has_changed);
        assert_eq!(s.game.command_count, 0);
    });
}

#[test]
fn monster_update_visibility_unlights_when_out_of_sight() {
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    test_set_ncurses_stub(true);
    place_monster(2, GIANT_WHITE_RAT_ID, 10, Coord_t { y: 10, x: 12 }, true);
    with_state_mut(|s| {
        s.monsters[2].distance_from_player = i32::from(MON_MAX_SIGHT) as u8 + 1;
    });

    monster_update_visibility(2);

    with_state(|s| {
        assert!(!s.monsters[2].lit);
        assert!(s.screen_has_changed);
    });
}

// ---------------------------------------------------------------------------
// 3. memoryUpdateRecall parity
// ---------------------------------------------------------------------------
#[test]
fn memory_update_recall_wake_ignore_and_rcmove_bits() {
    reset_for_new_game(None);
    let monster = Monster {
        creature_id: GREY_MUSHROOM_ID,
        lit: true,
        ..Default::default()
    };

    memory_update_recall(&monster, true, false, CM_MOVE_NORMAL);
    with_state(|s| {
        let memory = &s.creature_recall[GREY_MUSHROOM_ID as usize];
        assert_eq!(memory.wake, 1);
        assert_eq!(memory.ignore, 0);
        assert_eq!(memory.movement & CM_MOVE_NORMAL, CM_MOVE_NORMAL);
    });

    memory_update_recall(&monster, false, true, 0);
    with_state(|s| {
        let memory = &s.creature_recall[GREY_MUSHROOM_ID as usize];
        assert_eq!(memory.wake, 1);
        assert_eq!(memory.ignore, 1);
    });

    let unlit = Monster {
        creature_id: GREY_MUSHROOM_ID,
        lit: false,
        ..Default::default()
    };
    memory_update_recall(&unlit, true, true, CM_MOVE_NORMAL);
    with_state(|s| assert_eq!(s.creature_recall[GREY_MUSHROOM_ID as usize].wake, 1));
}

// ---------------------------------------------------------------------------
// 4. monsterAttackingUpdate RNG-order parity
// ---------------------------------------------------------------------------
#[test]
fn monster_attacking_update_aggravate_skips_sleep_rng_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    test_set_ncurses_stub(true);
    place_monster(2, GREY_MUSHROOM_ID, 10, Coord_t { y: 10, x: 11 }, true);
    light_monster_tile(Coord_t { y: 10, x: 11 });
    with_state_mut(|s| {
        s.monsters[2].sleep_count = 500;
        s.monsters[2].speed = 1;
        s.monsters[2].stunned_amount = 5;
        s.py.flags.aggravate = true;
        s.creature_recall[GREY_MUSHROOM_ID as usize].movement = 0;
    });

    monster_attacking_update(2, 1);

    with_state(|s| {
        assert_eq!(s.monsters[2].sleep_count, 0);
        assert!(s.monsters[2].lit);
    });
    assert_eq!(next_random_pair(5000), (5000, 2473));
}

#[test]
fn monster_attacking_update_sleep_notice_rng_order_seed1000() {
    reset_for_new_game(Some(1000));
    setup_dungeon(20, 20);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    test_set_ncurses_stub(true);
    place_monster(2, GREY_MUSHROOM_ID, 10, Coord_t { y: 10, x: 11 }, true);
    light_monster_tile(Coord_t { y: 10, x: 11 });
    with_state_mut(|s| {
        s.monsters[2].sleep_count = 500;
        s.monsters[2].speed = 1;
        s.py.misc.stealth_factor = 0;
    });

    monster_attacking_update(2, 1);

    with_state(|s| {
        assert_eq!(s.monsters[2].sleep_count, 400);
        assert_eq!(s.creature_recall[GREY_MUSHROOM_ID as usize].ignore, 1);
    });
    assert_eq!(next_random_pair(1024), (1024, 221));
}

// ---------------------------------------------------------------------------
// 5. updateMonsters iteration parity
// ---------------------------------------------------------------------------
#[test]
fn update_monsters_iterates_downward_and_recomputes_distance() {
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    test_set_ncurses_stub(true);
    reset_monster_slots();
    place_monster(2, GREY_MUSHROOM_ID, 10, Coord_t { y: 10, x: 11 }, false);
    place_monster(3, GIANT_WHITE_RAT_ID, 10, Coord_t { y: 10, x: 13 }, false);
    with_state_mut(|s| {
        s.monsters[2].distance_from_player = 99;
        s.monsters[3].distance_from_player = 99;
        s.monsters[2].speed = 0;
        s.monsters[3].speed = 0;
        s.dg.game_turn = 1;
    });

    update_monsters(true);

    with_state(|s| {
        assert_eq!(s.monsters[2].distance_from_player, 1);
        assert_eq!(s.monsters[3].distance_from_player, 3);
    });
}

#[test]
fn update_monsters_deletes_negative_hp_before_and_after() {
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(2, GREY_MUSHROOM_ID, -1, Coord_t { y: 10, x: 11 }, false);
    place_monster(3, GIANT_WHITE_RAT_ID, 10, Coord_t { y: 10, x: 12 }, false);
    with_state_mut(|s| s.monsters[3].speed = 0);

    update_monsters(false);

    with_state(|s| {
        assert_eq!(s.next_free_monster_id, 3);
        assert_eq!(s.monsters[2].creature_id, GIANT_WHITE_RAT_ID);
    });
}

// ---------------------------------------------------------------------------
// 6. monsterSleep parity
// ---------------------------------------------------------------------------
#[test]
fn monster_sleep_rng_order_and_messages_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player_panel(0, 0, Coord_t { y: 10, x: 10 });
    test_set_ncurses_stub(true);
    place_monster(2, GIANT_WHITE_RAT_ID, 10, Coord_t { y: 10, x: 11 }, true);

    assert!(monster_sleep(Coord_t { y: 10, x: 10 }));

    with_state(|s| assert_eq!(s.monsters[2].sleep_count, 500));
    with_state(|s| assert!(message_text(s.last_message_id).contains("falls asleep")));
    assert_eq!(next_random_pair(40), (40, 33));
}

// ---------------------------------------------------------------------------
// 7. printMonsterActionText / monsterNameDescription parity
// ---------------------------------------------------------------------------
#[test]
fn monster_name_description_lit_vs_unlit() {
    assert_eq!(
        monster_name_description("giant white rat", true),
        "The giant white rat"
    );
    assert_eq!(monster_name_description("giant white rat", false), "It");
}

#[test]
fn print_monster_action_text_exact_message() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    print_monster_action_text("The rat", "falls asleep.");
    with_state(|s| {
        assert!(message_text(s.last_message_id).contains("The rat falls asleep."));
    });
}
