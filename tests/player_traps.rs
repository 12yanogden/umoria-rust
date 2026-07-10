//! `player_traps` parity.
#![allow(
    clippy::int_plus_one,
    reason = "test assertions mirror C++ inclusive bound comparisons"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::config::identification::ID_KNOWN2;
use umoria::config::monsters::MON_ENDGAME_MONSTERS;
use umoria::config::treasure::chests::{
    CH_EXPLODE, CH_LOCKED, CH_LOSE_STR, CH_PARALYSED, CH_POISON, CH_SUMMON, CH_TRAPPED,
};
use umoria::data_creatures::CREATURES_LIST;
use umoria::dungeon::{dungeon_set_trap, MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::identification::SpecialNameIds;
use umoria::inventory::Inventory;
use umoria::monster::{Monster, MON_MAX_CREATURES, MON_MAX_LEVELS, MON_TOTAL_ALLOCATIONS};
use umoria::player::PlayerAttr;
use umoria::player_move::player_move;
use umoria::player_stats::player_initialize_base_experience_levels;
use umoria::player_traps::{
    chest_trap, player_disarm_chest_trap, player_disarm_floor_trap, player_disarm_trap,
};
use umoria::treasure::TV_CHEST;
use umoria::types::{Coord_t, MESSAGE_HISTORY_SIZE};
use umoria::ui::panel_bounds_fields;
use umoria::ui_io::{test_set_direction, test_set_ncurses_stub};

const POS: Coord_t = Coord_t { y: 10, x: 10 };
const TARGET: Coord_t = Coord_t { y: 9, x: 10 };
const NORTH: i32 = 8;

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

fn message_text(id: i16) -> String {
    with_state(|s| {
        let idx = id.rem_euclid(MESSAGE_HISTORY_SIZE as i16) as usize;
        let msg = &s.messages[idx];
        let end = msg.iter().position(|&b| b == 0).unwrap_or(msg.len());
        String::from_utf8_lossy(&msg[..end]).into_owned()
    })
}

fn last_message_text() -> String {
    with_state(|s| message_text(s.last_message_id))
}

fn init_monster_levels() {
    with_state_mut(|s| {
        s.monster_levels = [0; MON_MAX_LEVELS as usize + 1];
        let endgame = MON_ENDGAME_MONSTERS as usize;
        for i in 0..MON_MAX_CREATURES as usize - endgame {
            let level = CREATURES_LIST[i].level as usize;
            s.monster_levels[level] += 1;
        }
        for i in 1..=MON_MAX_LEVELS as usize {
            s.monster_levels[i] += s.monster_levels[i - 1];
        }
        s.dg.current_level = 10;
    });
}

fn setup_dungeon(height: i16, width: i16) {
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                s.dg.floor[y as usize][x as usize].feature_id = TILE_LIGHT_FLOOR;
            }
        }
        s.game.treasure.current_id = 1;
    });
}

fn setup_player() {
    test_set_ncurses_stub(true);
    player_initialize_base_experience_levels();
    let bounds = panel_bounds_fields(0, 0);
    with_state_mut(|s| {
        s.py.pos = POS;
        s.py.misc.class_id = 0;
        s.py.misc.level = 10;
        s.py.misc.disarm = 0;
        s.py.misc.current_hp = 500;
        s.py.misc.max_hp = 500;
        s.py.misc.fos = 100;
        s.py.misc.exp = 0;
        s.py.stats.used[PlayerAttr::A_DEX as usize] = 18;
        s.py.stats.used[PlayerAttr::A_INT as usize] = 10;
        s.py.flags.blind = 0;
        s.py.flags.confused = 0;
        s.py.flags.image = 0;
        s.py.flags.paralysis = 0;
        s.py.flags.poisoned = 0;
        s.py.flags.sustain_str = false;
        s.py.flags.free_action = false;
        s.py.carrying_light = true;
        s.dg.panel.row = 0;
        s.dg.panel.col = 0;
        s.dg.panel.top = bounds.top;
        s.dg.panel.bottom = bounds.bottom;
        s.dg.panel.left = bounds.left;
        s.dg.panel.right = bounds.right;
        s.dg.panel.row_prt = bounds.row_prt;
        s.dg.panel.col_prt = bounds.col_prt;
        s.dg.floor[POS.y as usize][POS.x as usize].creature_id = 1;
        s.dg.floor[POS.y as usize][POS.x as usize].temporary_light = true;
        s.dg.floor[TARGET.y as usize][TARGET.x as usize].feature_id = TILE_LIGHT_FLOOR;
    });
    init_monster_levels();
}

fn place_vis_trap(coord: Coord_t, depth: u8, misc_use: i16) -> u8 {
    dungeon_set_trap(coord, 0);
    with_state_mut(|s| {
        let treasure_id = s.dg.floor[coord.y as usize][coord.x as usize].treasure_id;
        s.game.treasure.list[treasure_id as usize].depth_first_found = depth;
        s.game.treasure.list[treasure_id as usize].misc_use = misc_use;
        s.game.treasure.list[treasure_id as usize].identification |= ID_KNOWN2;
        treasure_id
    })
}

fn place_chest(coord: Coord_t, flags: u32, depth: u8) -> u8 {
    with_state_mut(|s| {
        let treasure_id = s.game.treasure.current_id as u8;
        s.game.treasure.current_id += 1;
        s.game.treasure.list[treasure_id as usize] = Inventory {
            category_id: TV_CHEST,
            flags,
            depth_first_found: depth,
            identification: ID_KNOWN2,
            ..Default::default()
        };
        s.dg.floor[coord.y as usize][coord.x as usize].treasure_id = treasure_id;
        treasure_id
    })
}

fn reset_monster_slots() {
    with_state_mut(|s| {
        s.next_free_monster_id = 2;
        s.monsters = [Monster::default(); MON_TOTAL_ALLOCATIONS as usize];
    });
}

// ---------------------------------------------------------------------------
// 1. Floor trap disarm — RNG gates
// ---------------------------------------------------------------------------

#[test]
fn floor_trap_disarm_success_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player();
    let _ = place_vis_trap(TARGET, 10, 0);

    player_disarm_floor_trap(TARGET, 50, 10, NORTH, 0);

    // After success, player_move always consumes randomNumber(4) (C++ playerRandomMovement).
    assert_eq!(next_random_pair(100), (100, 2));
    with_state(|s| {
        assert_eq!(
            s.dg.floor[TARGET.y as usize][TARGET.x as usize].treasure_id,
            0
        );
    });
}

#[test]
fn floor_trap_disarm_failure_rng_order_seed3() {
    reset_for_new_game(Some(3));
    setup_dungeon(20, 20);
    setup_player();
    let treasure_id = place_vis_trap(TARGET, 100, 0);

    player_disarm_floor_trap(TARGET, 20, 100, NORTH, 0);

    assert_eq!(last_message_text(), "You failed to disarm the trap.");
    assert_eq!(next_random_pair(100), (100, 52));
    with_state(|s| {
        assert_eq!(
            s.dg.floor[TARGET.y as usize][TARGET.x as usize].treasure_id,
            treasure_id
        );
    });
}

#[test]
fn floor_trap_disarm_trigger_rng_order_seed6() {
    reset_for_new_game(Some(6));
    setup_dungeon(20, 20);
    setup_player();
    let treasure_id = place_vis_trap(TARGET, 100, 0);

    player_disarm_floor_trap(TARGET, 20, 100, NORTH, 0);

    assert!(last_message_text().starts_with("You set the trap off!"));
    // Trigger path also calls player_move, which always draws randomNumber(4).
    assert_eq!(next_random_pair(100), (100, 9));
    with_state(|s| {
        assert_eq!(
            s.dg.floor[TARGET.y as usize][TARGET.x as usize].treasure_id,
            treasure_id
        );
    });
}

#[test]
fn floor_trap_disarm_skips_random_number_total_when_total_le_5() {
    reset_for_new_game(Some(7));
    setup_dungeon(20, 20);
    setup_player();
    let _ = place_vis_trap(TARGET, 100, 0);
    player_disarm_floor_trap(TARGET, 3, 100, NORTH, 0);
    let after_low_total = random_number(100);

    reset_for_new_game(Some(7));
    setup_dungeon(20, 20);
    setup_player();
    let _ = place_vis_trap(TARGET, 100, 0);
    let _ = random_number(100);
    with_state_mut(|s| s.py.flags.confused = 0);
    player_move(NORTH, false);
    assert_eq!(random_number(100), after_low_total);
}

// ---------------------------------------------------------------------------
// 2. Chest disarm — RNG gates
// ---------------------------------------------------------------------------

#[test]
fn chest_disarm_success_rng_order_seed12() {
    reset_for_new_game(Some(12));
    setup_dungeon(20, 20);
    setup_player();
    let treasure_id = place_chest(TARGET, CH_TRAPPED | CH_LOCKED, 5);

    player_disarm_chest_trap(TARGET, 100, treasure_id);

    assert_eq!(next_random_pair(100), (100, 91));
    with_state(|s| {
        let item = &s.game.treasure.list[treasure_id as usize];
        assert_eq!(item.flags & CH_TRAPPED, 0);
        assert_eq!(item.special_name_id, SpecialNameIds::SN_LOCKED as u8);
    });
}

#[test]
fn chest_disarm_failure_rng_order_seed7() {
    reset_for_new_game(Some(7));
    setup_dungeon(20, 20);
    setup_player();
    let treasure_id = place_chest(TARGET, CH_TRAPPED, 15);

    player_disarm_chest_trap(TARGET, 20, treasure_id);

    assert_eq!(last_message_text(), "You failed to disarm the chest.");
    assert_eq!(next_random_pair(100), (100, 3));
    with_state(|s| {
        assert!(s.game.treasure.list[treasure_id as usize].flags & CH_TRAPPED != 0);
    });
}

#[test]
fn chest_disarm_trigger_fires_chest_trap_seed6() {
    reset_for_new_game(Some(6));
    setup_dungeon(20, 20);
    setup_player();
    let treasure_id = place_chest(TARGET, CH_TRAPPED | CH_POISON, 15);

    player_disarm_chest_trap(TARGET, 20, treasure_id);

    assert_eq!(next_random_pair(100), (100, 94));
    with_state(|s| assert_eq!(s.py.flags.poisoned, 20));
    let _ = treasure_id;
}

#[test]
fn chest_disarm_skips_random_number_total_when_total_le_5() {
    reset_for_new_game(Some(7));
    setup_dungeon(20, 20);
    setup_player();
    let treasure_id = place_chest(TARGET, CH_TRAPPED, 50);
    player_disarm_chest_trap(TARGET, 3, treasure_id);
    let after_low_total = random_number(100);

    reset_for_new_game(Some(7));
    setup_dungeon(20, 20);
    setup_player();
    let treasure_id = place_chest(TARGET, CH_TRAPPED | CH_POISON, 50);
    let _ = random_number(100);
    chest_trap(TARGET);
    let _ = treasure_id;
    assert_eq!(random_number(100), after_low_total);
}

// ---------------------------------------------------------------------------
// 3. chestTrap — per-flag RNG order
// ---------------------------------------------------------------------------

#[test]
fn chest_trap_lose_str_rng_order_seed9() {
    reset_for_new_game(Some(9));
    setup_dungeon(20, 20);
    setup_player();
    let _ = place_chest(TARGET, CH_LOSE_STR, 1);

    chest_trap(TARGET);

    assert_eq!(next_random_pair(4), (4, 4));
}

#[test]
fn chest_trap_poison_rng_order_seed11() {
    reset_for_new_game(Some(11));
    setup_dungeon(20, 20);
    setup_player();
    let _ = place_chest(TARGET, CH_POISON, 1);

    chest_trap(TARGET);

    assert_eq!(next_random_pair(6), (6, 4));
    assert_eq!(next_random_pair(20), (20, 2));
    with_state(|s| assert_eq!(s.py.flags.poisoned, 12));
}

#[test]
fn chest_trap_paralysed_int16_cast_seed6() {
    reset_for_new_game(Some(6));
    setup_dungeon(20, 20);
    setup_player();
    let _ = place_chest(TARGET, CH_PARALYSED, 1);

    chest_trap(TARGET);

    assert_eq!(next_random_pair(20), (20, 4));
    with_state(|s| assert_eq!(s.py.flags.paralysis, 20));
}

#[test]
fn chest_trap_explode_rng_order_seed8() {
    reset_for_new_game(Some(8));
    setup_dungeon(20, 20);
    setup_player();
    let treasure_id = place_chest(TARGET, CH_EXPLODE, 1);

    chest_trap(TARGET);

    assert_eq!(next_random_pair(8), (8, 2));
    with_state(|s| {
        assert_eq!(
            s.dg.floor[TARGET.y as usize][TARGET.x as usize].treasure_id,
            0
        );
        assert!(s.py.misc.current_hp < 500);
    });
    let _ = treasure_id;
}

#[test]
fn chest_trap_summon_calls_monster_summon_three_times() {
    reset_for_new_game(Some(5));
    setup_dungeon(20, 20);
    setup_player();
    reset_monster_slots();
    let _ = place_chest(TARGET, CH_SUMMON, 1);

    chest_trap(TARGET);

    with_state(|s| assert!(i32::from(s.next_free_monster_id) > 2));
}

#[test]
fn chest_trap_multi_flag_order_seed15() {
    reset_for_new_game(Some(15));
    setup_dungeon(20, 20);
    setup_player();
    let _ = place_chest(TARGET, CH_LOSE_STR | CH_POISON | CH_PARALYSED, 1);

    chest_trap(TARGET);

    assert_eq!(next_random_pair(4), (4, 1));
    assert_eq!(next_random_pair(6), (6, 6));
    assert_eq!(next_random_pair(20), (20, 5));
    with_state(|s| {
        assert!(s.py.flags.poisoned > 0);
        assert!(s.py.flags.paralysis > 0);
    });
}

#[test]
fn chest_trap_sustain_str_skips_damage_rng() {
    reset_for_new_game(Some(9));
    setup_dungeon(20, 20);
    setup_player();
    with_state_mut(|s| s.py.flags.sustain_str = true);
    let _ = place_chest(TARGET, CH_LOSE_STR, 1);

    chest_trap(TARGET);

    assert_eq!(next_random_pair(100), (100, 71));
}

#[test]
fn chest_trap_free_action_skips_paralysis_rng() {
    reset_for_new_game(Some(6));
    setup_dungeon(20, 20);
    setup_player();
    with_state_mut(|s| s.py.flags.free_action = true);
    let _ = place_chest(TARGET, CH_PARALYSED, 1);

    chest_trap(TARGET);

    assert_eq!(next_random_pair(100), (100, 50));
    with_state(|s| assert_eq!(s.py.flags.paralysis, 0));
}

// ---------------------------------------------------------------------------
// 4. playerDisarmTrap dispatch + outcome messages
// ---------------------------------------------------------------------------

#[test]
fn player_disarm_trap_no_target_sets_free_turn() {
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    setup_player();
    test_set_direction(Some(NORTH));
    player_disarm_trap();
    with_state(|s| assert!(s.game.player_free_turn));
    assert_eq!(
        last_message_text(),
        "I do not see anything to disarm there."
    );
}

#[test]
fn chest_disarm_unidentified_trap_sets_free_turn() {
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    setup_player();
    let treasure_id = place_chest(TARGET, CH_TRAPPED, 5);
    with_state_mut(|s| {
        s.game.treasure.list[treasure_id as usize].identification = 0;
    });
    player_disarm_chest_trap(TARGET, 20, treasure_id);
    with_state(|s| assert!(s.game.player_free_turn));
    assert_eq!(last_message_text(), "I don't see a trap.");
}

#[test]
fn chest_disarm_not_trapped_message() {
    reset_for_new_game(None);
    setup_dungeon(20, 20);
    setup_player();
    let treasure_id = place_chest(TARGET, CH_LOCKED, 5);
    player_disarm_chest_trap(TARGET, 20, treasure_id);
    assert_eq!(last_message_text(), "The chest was not trapped.");
}

#[test]
fn chest_disarm_success_unlocked_sets_disarmed_name() {
    reset_for_new_game(Some(12));
    setup_dungeon(20, 20);
    setup_player();
    let treasure_id = place_chest(TARGET, CH_TRAPPED, 5);

    player_disarm_chest_trap(TARGET, 100, treasure_id);

    with_state(|s| {
        let item = &s.game.treasure.list[treasure_id as usize];
        assert_eq!(item.special_name_id, SpecialNameIds::SN_DISARMED as u8);
    });
}

#[test]
fn player_disarm_trap_floor_via_direction_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player();
    with_state_mut(|s| s.py.misc.disarm = 12);
    let _ = place_vis_trap(TARGET, 10, 0);
    test_set_direction(Some(NORTH));
    player_disarm_trap();
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn floor_trap_success_skips_accidental_roll() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player();
    let _ = place_vis_trap(TARGET, 10, 0);
    player_disarm_floor_trap(TARGET, 50, 10, NORTH, 0);
    let baseline = random_number(100);

    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player();
    let _ = place_vis_trap(TARGET, 10, 0);
    player_disarm_floor_trap(TARGET, 50, 10, NORTH, 0);
    assert_eq!(random_number(100), baseline);
}
