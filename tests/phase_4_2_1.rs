//! Phase 4.2.1 — monster_manager.cpp parity.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

mod common;

use umoria::config::monsters::defense::CD_UNDEAD;
use umoria::config::monsters::{self};
use umoria::data_creatures::CREATURES_LIST;
use umoria::dungeon::{coord_distance_between, MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::monster::{Monster, MON_MAX_CREATURES, MON_MAX_LEVELS, MON_TOTAL_ALLOCATIONS};
use umoria::monster_manager::{
    compact_monsters, monster_get_one_suitable_for_level, monster_place_new,
    monster_place_new_within_distance, monster_place_winning, monster_summon,
    monster_summon_undead,
};
use umoria::types::Coord_t;
use umoria::ui_io::test_set_ncurses_stub;

fn init_monster_levels() {
    with_state_mut(|state| {
        state.monster_levels = [0; MON_MAX_LEVELS as usize + 1];
        let endgame = monsters::MON_ENDGAME_MONSTERS as usize;
        for i in 0..MON_MAX_CREATURES as usize - endgame {
            let level = CREATURES_LIST[i].level as usize;
            state.monster_levels[level] += 1;
        }
        for i in 1..=MON_MAX_LEVELS as usize {
            state.monster_levels[i] += state.monster_levels[i - 1];
        }
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
    });
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

fn reset_monster_slots() {
    with_state_mut(|s| {
        s.next_free_monster_id = i16::from(monsters::MON_MIN_INDEX_ID);
        s.monsters = [Monster::default(); MON_TOTAL_ALLOCATIONS as usize];
        s.hack_monptr = -1;
    });
}

// ---------------------------------------------------------------------------
// 1. monsterGetOneSuitableForLevel RNG-order/value parity
// ---------------------------------------------------------------------------
#[test]
fn monster_get_one_suitable_for_level_zero_seed42() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    let id = monster_get_one_suitable_for_level(0);
    assert_eq!(id, 5);
    assert_eq!(
        next_random_pair(i32::from(with_state(|s| s.monster_levels[0]))),
        (8, 1)
    );
}

#[test]
fn monster_get_one_suitable_for_level_nasty_branch_seed777() {
    reset_for_new_game(Some(777));
    init_monster_levels();
    let id = monster_get_one_suitable_for_level(10);
    assert_eq!(id, 84);
}

#[test]
fn monster_get_one_suitable_for_level_normal_branch_seed42() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    let id = monster_get_one_suitable_for_level(10);
    assert_eq!(id, 85);
    assert_eq!(next_random_pair(50), (50, 7));
}

// ---------------------------------------------------------------------------
// 2. monsterPlaceNew parity
// ---------------------------------------------------------------------------
#[test]
fn monster_place_new_max_hp_and_fields_seed42() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    reset_monster_slots();
    setup_dungeon(20, 20);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.py.flags.speed = 0;
    });

    let coord = Coord_t { y: 5, x: 5 };
    let balrog_creature_id = i32::from(MON_MAX_CREATURES) - 1;
    assert!(monster_place_new(coord, balrog_creature_id, false));

    with_state(|s| {
        let m = &s.monsters[2];
        assert_eq!(m.creature_id, balrog_creature_id as u16);
        assert_eq!(m.hp, 3000);
        assert_eq!(m.speed, 3);
        assert_eq!(m.distance_from_player, 7);
        assert_eq!(m.sleep_count, 0);
        assert!(!m.lit);
        assert_eq!(
            s.dg.floor[coord.y as usize][coord.x as usize].creature_id,
            2
        );
        assert_eq!(s.next_free_monster_id, 3);
    });
    assert_eq!(next_random_pair(12), (12, 2));
}

#[test]
fn monster_place_new_dice_hp_sleep_counter_seed1() {
    reset_for_new_game(Some(1));
    init_monster_levels();
    reset_monster_slots();
    setup_dungeon(20, 20);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.py.flags.speed = 5;
    });

    let coord = Coord_t { y: 8, x: 8 };
    assert!(monster_place_new(coord, 0, true));

    with_state(|s| {
        let m = &s.monsters[2];
        assert_eq!(m.hp, 3);
        assert_eq!(m.speed, 6);
        assert_eq!(m.distance_from_player, 3);
        assert_eq!(m.sleep_count, 179);
    });
    assert_eq!(next_random_pair(400), (400, 100));
}

#[test]
fn monster_place_new_returns_false_when_popm_fails() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    setup_dungeon(20, 20);
    with_state_mut(|s| {
        s.next_free_monster_id = i16::from(MON_TOTAL_ALLOCATIONS);
        s.hack_monptr = -1;
        for i in i16::from(monsters::MON_MIN_INDEX_ID)..i16::from(MON_TOTAL_ALLOCATIONS) {
            s.monsters[i as usize].distance_from_player = 0;
            s.monsters[i as usize].creature_id = 0;
        }
    });
    test_set_ncurses_stub(true);
    assert!(!monster_place_new(Coord_t { y: 5, x: 5 }, 0, false));
}

// ---------------------------------------------------------------------------
// 3. popm / compactMonsters parity
// ---------------------------------------------------------------------------
#[test]
fn popm_increments_next_free_monster_id() {
    reset_for_new_game(Some(42));
    reset_monster_slots();
    setup_dungeon(20, 20);
    assert!(monster_place_new(Coord_t { y: 5, x: 5 }, 0, false));
    with_state(|s| assert_eq!(s.next_free_monster_id, 3));
}

#[test]
fn compact_monsters_deletes_far_monster_seed42() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    reset_monster_slots();
    setup_dungeon(20, 20);
    with_state_mut(|s| {
        s.monsters[2].distance_from_player = 80;
        s.monsters[2].creature_id = 0;
        s.monsters[2].pos = Coord_t { y: 5, x: 5 };
        s.dg.floor[5][5].creature_id = 2;
        s.next_free_monster_id = 3;
        s.hack_monptr = -1;
    });
    test_set_ncurses_stub(true);
    assert!(compact_monsters());
    with_state(|s| assert_eq!(s.next_free_monster_id, 2));
    assert_eq!(next_random_pair(3), (3, 2));
}

#[test]
fn compact_monsters_skips_cm_win() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    reset_monster_slots();
    setup_dungeon(20, 20);
    let balrog = MON_MAX_CREATURES - 1;
    with_state_mut(|s| {
        s.monsters[2].distance_from_player = 80;
        s.monsters[2].creature_id = balrog;
        s.monsters[2].pos = Coord_t { y: 5, x: 5 };
        s.dg.floor[5][5].creature_id = 2;
        s.next_free_monster_id = 3;
        s.hack_monptr = -1;
    });
    test_set_ncurses_stub(true);
    assert!(!compact_monsters());
    with_state(|s| assert_eq!(s.next_free_monster_id, 3));
}

#[test]
fn compact_monsters_hack_monptr_remove_only() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    reset_monster_slots();
    setup_dungeon(20, 20);
    with_state_mut(|s| {
        s.monsters[2].distance_from_player = 80;
        s.monsters[2].creature_id = 0;
        s.monsters[2].pos = Coord_t { y: 5, x: 5 };
        s.dg.floor[5][5].creature_id = 2;
        s.next_free_monster_id = 3;
        s.hack_monptr = 5;
    });
    test_set_ncurses_stub(true);
    assert!(!compact_monsters());
    with_state(|s| {
        assert_eq!(s.next_free_monster_id, 3);
        assert_eq!(s.monsters[2].hp, -1);
    });
}

#[test]
fn compact_monsters_fails_when_cur_dis_below_zero() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    reset_monster_slots();
    with_state_mut(|s| {
        s.monsters[2].distance_from_player = 0;
        s.next_free_monster_id = 3;
        s.hack_monptr = -1;
    });
    test_set_ncurses_stub(true);
    assert!(!compact_monsters());
}

// ---------------------------------------------------------------------------
// 4. monsterPlaceWinning parity
// ---------------------------------------------------------------------------
#[test]
fn monster_place_winning_noop_when_total_winner() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    reset_monster_slots();
    setup_dungeon(30, 30);
    with_state_mut(|s| s.game.total_winner = true);
    monster_place_winning();
    with_state(|s| assert_eq!(s.next_free_monster_id, 2));
}

#[test]
fn monster_place_winning_places_endgame_monster_seed42() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    reset_monster_slots();
    setup_dungeon(30, 30);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 15, x: 15 };
        s.py.flags.speed = 0;
    });
    monster_place_winning();
    with_state(|s| {
        let m = &s.monsters[2];
        assert!(m.creature_id >= s.monster_levels[MON_MAX_LEVELS as usize] as u16);
        assert_eq!(m.sleep_count, 0);
        assert_eq!(s.next_free_monster_id, 3);
    });
    assert_eq!(next_random_pair(12), (12, 9));
}

// ---------------------------------------------------------------------------
// 5. monsterPlaceNewWithinDistance parity
// ---------------------------------------------------------------------------
#[test]
fn monster_place_new_within_distance_one_monster_seed42() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    reset_monster_slots();
    setup_dungeon(30, 30);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 15, x: 15 };
        s.dg.current_level = 5;
    });
    monster_place_new_within_distance(1, 3, false);
    with_state(|s| assert_eq!(s.next_free_monster_id, 3));
    assert_eq!(next_random_pair(50), (50, 8));
}

#[test]
fn monster_place_new_within_distance_dragon_forces_sleeping() {
    reset_for_new_game(Some(9999));
    init_monster_levels();
    reset_monster_slots();
    setup_dungeon(30, 30);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 15, x: 15 };
        s.dg.current_level = 20;
    });
    monster_place_new_within_distance(1, 3, false);
    with_state(|s| {
        let cid = s.monsters[2].creature_id as usize;
        if CREATURES_LIST[cid].sprite == b'd' || CREATURES_LIST[cid].sprite == b'D' {
            assert!(s.monsters[2].sleep_count > 0);
        }
    });
}

// ---------------------------------------------------------------------------
// 6. monsterSummon / monsterSummonUndead / placeMonsterAdjacentTo
// ---------------------------------------------------------------------------
#[test]
fn monster_summon_adjacent_seed42() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    reset_monster_slots();
    setup_dungeon(30, 30);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 15, x: 15 };
        s.dg.current_level = 5;
    });
    let mut coord = Coord_t { y: 10, x: 10 };
    assert!(monster_summon(&mut coord, false));
    with_state(|s| {
        assert_eq!(
            s.dg.floor[coord.y as usize][coord.x as usize].creature_id,
            2
        );
        assert_ne!(coord.y, 10);
    });
}

#[test]
fn monster_summon_undead_adjacent_seed42() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    reset_monster_slots();
    setup_dungeon(30, 30);
    with_state_mut(|s| s.py.pos = Coord_t { y: 15, x: 15 });
    let mut coord = Coord_t { y: 10, x: 10 };
    assert!(monster_summon_undead(&mut coord));
    with_state(|s| {
        let cid = s.monsters[2].creature_id as usize;
        assert_ne!(CREATURES_LIST[cid].defenses & CD_UNDEAD, 0);
    });
}

// ---------------------------------------------------------------------------
// 7. Integer-semantics tests
// ---------------------------------------------------------------------------
#[test]
fn monster_place_new_uint8_distance_wrap() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    reset_monster_slots();
    setup_dungeon(66, 66);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 1, x: 1 };
        s.py.flags.speed = 0;
    });
    let coord = Coord_t { y: 60, x: 60 };
    assert!(monster_place_new(coord, 0, false));
    with_state(|s| {
        let dist = coord_distance_between(s.py.pos, coord);
        assert_eq!(s.monsters[2].distance_from_player, dist as u8);
    });
}

#[test]
fn monster_place_new_speed_int16_cast() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    reset_monster_slots();
    setup_dungeon(20, 20);
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.py.flags.speed = 300;
    });
    assert!(monster_place_new(Coord_t { y: 5, x: 5 }, 0, false));
    with_state(|s| {
        let creature_speed = CREATURES_LIST[0].speed;
        let expected = (i32::from(creature_speed) - 10 + 300) as i16;
        assert_eq!(s.monsters[2].speed, expected);
    });
}
