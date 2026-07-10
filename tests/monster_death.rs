//! Monster death, hit resolution & loot parity.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use umoria::config::monsters;
use umoria::config::monsters::move_flags::{
    CM_1D2_OBJ, CM_2D2_OBJ, CM_4D2_OBJ, CM_60_RANDOM, CM_90_RANDOM, CM_CARRY_GOLD, CM_CARRY_OBJ,
    CM_SMALL_OBJ, CM_TREASURE, CM_TR_SHIFT, CM_WIN,
};
use umoria::data_treasure::GAME_OBJECTS;
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_generate::treasure_linker;
use umoria::dungeon_tile::{Tile, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::monster::{
    monster_death, monster_death_item_drop_count, monster_death_item_drop_type, monster_take_hit,
    Monster, MON_TOTAL_ALLOCATIONS,
};
use umoria::types::Coord_t;
use umoria::types::{MAX_DUNGEON_OBJECTS, TREASURE_MAX_LEVELS};
use umoria::ui_io::test_set_ncurses_stub;

const GREY_MUSHROOM_ID: u16 = 8;
const BALROG_ID: u16 = 278;

fn setup_dungeon(height: i16, width: i16) {
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                s.dg.floor[y as usize][x as usize].feature_id = TILE_LIGHT_FLOOR;
                s.dg.floor[y as usize][x as usize].permanent_light = true;
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

fn init_treasure_levels() {
    with_state_mut(|state| {
        state.treasure_levels = [0; TREASURE_MAX_LEVELS as usize + 1];
        for i in 0..MAX_DUNGEON_OBJECTS as usize {
            let level = GAME_OBJECTS[i].depth_first_found as usize;
            state.treasure_levels[level] += 1;
        }
        for i in 1..=TREASURE_MAX_LEVELS as usize {
            state.treasure_levels[i] += state.treasure_levels[i - 1];
        }

        let mut indexes = [1i16; TREASURE_MAX_LEVELS as usize + 1];
        for i in 0..MAX_DUNGEON_OBJECTS as usize {
            let level = GAME_OBJECTS[i].depth_first_found as usize;
            let object_id = state.treasure_levels[level] - indexes[level];
            state.sorted_objects[object_id as usize] = i as i16;
            indexes[level] += 1;
        }
    });
}

fn prepare_object_tables() {
    treasure_linker();
    init_treasure_levels();
}

fn place_monster(id: i32, creature_id: u16, hp: i16, coord: Coord_t, lit: bool) {
    with_state_mut(|s| {
        s.monsters[id as usize] = Monster {
            hp,
            sleep_count: 99,
            creature_id,
            pos: coord,
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
        let msg = &s.messages[id as usize];
        let end = msg.iter().position(|&b| b == 0).unwrap_or(msg.len());
        String::from_utf8_lossy(&msg[..end]).into_owned()
    })
}

fn flags_from_bits(bits: u32) -> u32 {
    let mut flags = 0u32;
    if bits & 1 != 0 {
        flags |= CM_60_RANDOM;
    }
    if bits & 2 != 0 {
        flags |= CM_90_RANDOM;
    }
    if bits & 4 != 0 {
        flags |= CM_1D2_OBJ;
    }
    if bits & 8 != 0 {
        flags |= CM_2D2_OBJ;
    }
    if bits & 16 != 0 {
        flags |= CM_4D2_OBJ;
    }
    flags
}

// ---------------------------------------------------------------------------
// 1. monster_take_hit parity
// ---------------------------------------------------------------------------
#[test]
fn monster_take_hit_survival_returns_minus_one_and_no_rng() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    let coord = Coord_t { y: 10, x: 10 };
    place_monster(2, GREY_MUSHROOM_ID, 10, coord, true);

    assert_eq!(monster_take_hit(2, 3), -1);

    with_state(|s| {
        assert_eq!(s.monsters[2].hp, 7);
        assert_eq!(s.monsters[2].sleep_count, 0);
        assert_eq!(
            s.dg.floor[coord.y as usize][coord.x as usize].creature_id,
            2
        );
    });
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn monster_take_hit_death_grants_exp_and_recall_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    with_state_mut(|s| {
        s.py.misc.level = 1;
        s.py.misc.exp = 0;
        s.py.misc.exp_fraction = 0;
        s.py.flags.blind = 0;
    });
    let coord = Coord_t { y: 10, x: 10 };
    place_monster(2, GREY_MUSHROOM_ID, 1, coord, true);

    assert_eq!(monster_take_hit(2, 2), i32::from(GREY_MUSHROOM_ID));

    with_state(|s| {
        assert_eq!(s.py.misc.exp, 1);
        assert_eq!(s.creature_recall[GREY_MUSHROOM_ID as usize].kills, 1);
        assert_eq!(s.next_free_monster_id, 2);
        assert_eq!(
            s.dg.floor[coord.y as usize][coord.x as usize].creature_id,
            0
        );
    });
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn monster_take_hit_hack_monptr_remove_vs_delete() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    with_state_mut(|s| s.py.misc.level = 1);
    let coord = Coord_t { y: 10, x: 10 };
    place_monster(2, GREY_MUSHROOM_ID, 1, coord, false);

    with_state_mut(|s| s.hack_monptr = 5);
    assert_eq!(monster_take_hit(2, 2), i32::from(GREY_MUSHROOM_ID));
    with_state(|s| {
        assert_eq!(s.monsters[2].hp, -1);
        assert_eq!(s.next_free_monster_id, 3);
    });

    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    with_state_mut(|s| s.py.misc.level = 1);
    place_monster(2, GREY_MUSHROOM_ID, 1, coord, false);
    with_state_mut(|s| s.hack_monptr = -1);
    assert_eq!(monster_take_hit(2, 2), i32::from(GREY_MUSHROOM_ID));
    with_state(|s| assert_eq!(s.next_free_monster_id, 2));
}

#[test]
fn monster_take_hit_win_creature_updates_recall_when_unlit() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    test_set_ncurses_stub(true);
    prepare_object_tables();
    with_state_mut(|s| {
        s.py.misc.level = 10;
        s.py.flags.blind = 0;
        s.dg.current_level = 50;
        s.creature_recall[BALROG_ID as usize].movement = CM_CARRY_OBJ | (3 << CM_TR_SHIFT);
    });
    let coord = Coord_t { y: 10, x: 10 };
    place_monster(2, BALROG_ID, 1, coord, false);

    assert_eq!(monster_take_hit(2, 2), i32::from(BALROG_ID));

    with_state(|s| {
        assert!(s.game.total_winner);
        assert_eq!(s.creature_recall[BALROG_ID as usize].kills, 1);
        assert_eq!(
            s.creature_recall[BALROG_ID as usize].movement,
            CM_CARRY_OBJ | CM_CARRY_GOLD | (13 << CM_TR_SHIFT)
        );
    });
}

// ---------------------------------------------------------------------------
// 2. monster_death_item_drop_type parity
// ---------------------------------------------------------------------------
#[test]
fn monster_death_item_drop_type_bit_composition() {
    assert_eq!(monster_death_item_drop_type(0), 0);
    assert_eq!(monster_death_item_drop_type(CM_CARRY_OBJ), 1);
    assert_eq!(monster_death_item_drop_type(CM_CARRY_GOLD), 2);
    assert_eq!(monster_death_item_drop_type(CM_SMALL_OBJ), 4);
    assert_eq!(
        monster_death_item_drop_type(CM_CARRY_OBJ | CM_CARRY_GOLD),
        3
    );
    assert_eq!(
        monster_death_item_drop_type(CM_CARRY_OBJ | CM_CARRY_GOLD | CM_SMALL_OBJ),
        7
    );
}

// ---------------------------------------------------------------------------
// 3. monster_death_item_drop_count RNG-order parity
// ---------------------------------------------------------------------------
#[test]
fn monster_death_item_drop_count_isolated_flags_seed42() {
    let cases = [
        (0u32, 0),
        (CM_60_RANDOM, 1),
        (CM_90_RANDOM, 1),
        (CM_1D2_OBJ, 2),
        (CM_2D2_OBJ, 3),
        (CM_4D2_OBJ, 7),
        (
            CM_60_RANDOM | CM_90_RANDOM | CM_1D2_OBJ | CM_2D2_OBJ | CM_4D2_OBJ,
            13,
        ),
    ];
    for (flags, expected) in cases {
        reset_for_new_game(Some(42));
        assert_eq!(
            monster_death_item_drop_count(flags),
            expected,
            "flags={flags:#x}"
        );
    }
}

#[test]
fn monster_death_item_drop_count_all_flag_combos_rng_order_seed42() {
    const EXPECTED: [i32; 32] = [
        0, 1, 1, 2, 1, 1, 1, 4, 4, 5, 3, 5, 4, 6, 7, 6, 5, 8, 8, 8, 9, 7, 9, 9, 8, 9, 11, 11, 9,
        10, 13, 12,
    ];

    reset_for_new_game(Some(42));
    for bits in 0..32 {
        let flags = flags_from_bits(bits);
        assert_eq!(
            monster_death_item_drop_count(flags),
            EXPECTED[bits as usize],
            "bits={bits}"
        );
    }
    assert_eq!(next_random_pair(100), (100, 15));
}

// ---------------------------------------------------------------------------
// 4. monster_death parity
// ---------------------------------------------------------------------------
#[test]
fn monster_death_winner_path_sets_total_winner_and_messages() {
    reset_for_new_game(Some(42));
    test_set_ncurses_stub(true);
    with_state_mut(|s| s.game.character_is_dead = false);

    let coord = Coord_t { y: 10, x: 10 };
    assert_eq!(monster_death(coord, CM_WIN), 0);

    with_state(|s| assert!(s.game.total_winner));
    assert!(message_text(1).contains("CONGRATULATIONS"));
    assert!(message_text(2).contains("cannot save this game"));
}

#[test]
fn monster_death_no_drop_returns_zero() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    assert_eq!(monster_death(Coord_t { y: 10, x: 10 }, 0), 0);
    assert_eq!(next_random_pair(100), (100, 2));
}

#[test]
fn monster_death_summon_and_return_mask_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    prepare_object_tables();
    with_state_mut(|s| s.dg.current_level = 5);
    let coord = Coord_t { y: 10, x: 10 };
    let flags = CM_CARRY_OBJ | CM_1D2_OBJ;
    let mask = monster_death(coord, flags);
    assert_eq!(mask, CM_CARRY_OBJ | (2 << CM_TR_SHIFT));
}

// ---------------------------------------------------------------------------
// 5. Integer-semantics tests
// ---------------------------------------------------------------------------
#[test]
fn monster_take_hit_i16_hp_wrap_triggers_death() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    with_state_mut(|s| {
        s.py.misc.level = 1;
        s.hack_monptr = 5;
    });
    let coord = Coord_t { y: 5, x: 5 };
    place_monster(2, GREY_MUSHROOM_ID, 0, coord, false);

    assert_eq!(monster_take_hit(2, 1), i32::from(GREY_MUSHROOM_ID));
    with_state(|s| assert_eq!(s.monsters[2].hp, -1));
}

#[test]
fn monster_death_return_mask_integer_arithmetic() {
    let item_type = 5; // CM_SMALL_OBJ in low bits
    let dropped_item_id = 261u32; // 256 + 5
    let mut return_flags = 0u32;
    if (dropped_item_id & 255) != 0 {
        return_flags |= CM_CARRY_OBJ;
        if (item_type & 0x04) != 0 {
            return_flags |= CM_SMALL_OBJ;
        }
    }
    if dropped_item_id >= 256 {
        return_flags |= CM_CARRY_GOLD;
    }
    let number_of_items = ((dropped_item_id % 256) + (dropped_item_id / 256)) << CM_TR_SHIFT;
    let mask = return_flags | number_of_items;
    assert_eq!(
        mask,
        CM_CARRY_OBJ | CM_CARRY_GOLD | CM_SMALL_OBJ | (6 << CM_TR_SHIFT)
    );
}

#[test]
fn monster_take_hit_treasure_recall_merge_prefers_higher_shifted_count() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    with_state_mut(|s| {
        s.py.misc.level = 1;
        s.py.flags.blind = 0;
        s.creature_recall[GREY_MUSHROOM_ID as usize].movement = CM_CARRY_GOLD | (9 << CM_TR_SHIFT);
    });
    let coord = Coord_t { y: 8, x: 8 };
    place_monster(2, GREY_MUSHROOM_ID, 1, coord, true);

    monster_take_hit(2, 2);

    with_state(|s| {
        let mem = &s.creature_recall[GREY_MUSHROOM_ID as usize];
        assert_eq!((mem.movement & CM_TREASURE) >> CM_TR_SHIFT, 9);
        assert_eq!(mem.movement & CM_CARRY_GOLD, CM_CARRY_GOLD);
    });
}
