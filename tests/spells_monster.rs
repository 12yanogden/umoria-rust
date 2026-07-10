//! Monster-affecting spells (`spells`) parity.
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

mod common;

use umoria::config::monsters::defense::{CD_NO_SLEEP, CD_UNDEAD};
use umoria::config::monsters::{self, MON_MAX_SIGHT};
use umoria::config::treasure::OBJECT_BOLTS_MAX_RANGE;
use umoria::data_creatures::CREATURES_LIST;
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, MIN_CLOSED_SPACE, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::monster::{Monster, MON_MAX_CREATURES, MON_MAX_LEVELS, MON_TOTAL_ALLOCATIONS};
use umoria::spells::{
    spell_change_monster_hit_points, spell_clone_monster, spell_confuse_monster,
    spell_dispel_creature, spell_drain_life_from_monster, spell_genocide, spell_mass_genocide,
    spell_mass_polymorph, spell_polymorph_monster, spell_sleep_all_monsters, spell_sleep_monster,
    spell_speed_all_monsters, spell_speed_monster, spell_teleport_away_monster,
    spell_teleport_away_monster_in_direction, spell_turn_undead,
};
use umoria::types::Coord_t;
use umoria::ui::panel_bounds_fields;
use umoria::ui_io::{test_push_getch_keys, test_set_ncurses_stub};

const URCHIN_ID: u16 = 0;
const CENTIPEDE_ID: u16 = 9;
const COPPER_COINS_ID: u16 = 52;
const ZOMBIE_KOBOLD_ID: u16 = 86;
const BALROG_ID: u16 = MON_MAX_CREATURES - 1;

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

fn reset_monster_slots() {
    with_state_mut(|s| {
        s.next_free_monster_id = i16::from(monsters::MON_MIN_INDEX_ID);
        s.monsters = [Monster::default(); MON_TOTAL_ALLOCATIONS as usize];
        s.hack_monptr = -1;
    });
}

fn setup_player(pos: Coord_t, level: u16) {
    test_set_ncurses_stub(true);
    let bounds = panel_bounds_fields(0, 0);
    with_state_mut(|s| {
        s.py.pos = pos;
        s.py.misc.level = level;
        s.dg.panel.row = 0;
        s.dg.panel.col = 0;
        s.dg.panel.top = bounds.top;
        s.dg.panel.bottom = bounds.bottom;
        s.dg.panel.left = bounds.left;
        s.dg.panel.right = bounds.right;
        s.dg.panel.row_prt = bounds.row_prt;
        s.dg.panel.col_prt = bounds.col_prt;
        s.py.flags.blind = 0;
    });
}

fn place_monster(
    id: i32,
    creature_id: u16,
    hp: i16,
    coord: Coord_t,
    lit: bool,
    distance: u8,
    speed: i16,
) {
    with_state_mut(|s| {
        s.monsters[id as usize] = Monster {
            hp,
            sleep_count: 99,
            speed,
            creature_id,
            pos: coord,
            distance_from_player: distance,
            lit,
            ..Default::default()
        };
        s.dg.floor[coord.y as usize][coord.x as usize].creature_id = id as u8;
        if lit {
            s.dg.floor[coord.y as usize][coord.x as usize].permanent_light = true;
        }
        if id + 1 >= i32::from(s.next_free_monster_id) {
            s.next_free_monster_id = (id + 1) as i16;
        }
    });
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

// ---------------------------------------------------------------------------
// 1. Early-out: blocked direction consumes zero RNG
// ---------------------------------------------------------------------------
#[test]
fn early_out_wall_consumes_no_rng_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);

    with_state_mut(|s| {
        s.dg.floor[10][11].feature_id = MIN_CLOSED_SPACE;
    });

    let before = with_state(|s| s.rng.old_seed);
    let changed = spell_change_monster_hit_points(Coord_t { y: 10, x: 10 }, 6, 50);
    let after = with_state(|s| s.rng.old_seed);

    assert!(!changed);
    assert_eq!(before, after);
}

#[test]
fn early_out_empty_ray_consumes_no_rng_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);

    let before = with_state(|s| s.rng.old_seed);
    let confused = spell_confuse_monster(Coord_t { y: 10, x: 10 }, 6);
    let after = with_state(|s| s.rng.old_seed);

    assert!(!confused);
    assert_eq!(before, after);
}

// ---------------------------------------------------------------------------
// 2. Directed HP / drain life
// ---------------------------------------------------------------------------
#[test]
fn change_monster_hp_damages_and_returns_true_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(2, URCHIN_ID, 20, Coord_t { y: 10, x: 11 }, true, 1, 11);

    assert!(spell_change_monster_hit_points(
        Coord_t { y: 10, x: 10 },
        6,
        5
    ));
    with_state(|s| {
        assert_eq!(s.monsters[2].hp, 15);
        assert_eq!(s.monsters[2].sleep_count, 0);
    });
}

#[test]
fn drain_life_skips_undead_sets_recall_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(
        2,
        ZOMBIE_KOBOLD_ID,
        100,
        Coord_t { y: 10, x: 11 },
        true,
        1,
        11,
    );

    assert!(!spell_drain_life_from_monster(Coord_t { y: 10, x: 10 }, 6));
    with_state(|s| {
        assert_eq!(s.monsters[2].hp, 100);
        assert_ne!(
            s.creature_recall[ZOMBIE_KOBOLD_ID as usize].defenses & CD_UNDEAD,
            0
        );
    });
}

#[test]
fn drain_life_hits_living_seed777() {
    reset_for_new_game(Some(777));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(2, URCHIN_ID, 100, Coord_t { y: 10, x: 11 }, true, 1, 11);

    assert!(spell_drain_life_from_monster(Coord_t { y: 10, x: 10 }, 6));
    with_state(|s| assert_eq!(s.monsters[2].hp, 25));
}

// ---------------------------------------------------------------------------
// 3. Speed / confuse / sleep resist rolls (RNG order)
// ---------------------------------------------------------------------------
#[test]
fn speed_monster_haste_no_resist_roll_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(2, URCHIN_ID, 10, Coord_t { y: 10, x: 11 }, true, 1, 11);

    let before = with_state(|s| s.rng.old_seed);
    assert!(spell_speed_monster(Coord_t { y: 10, x: 10 }, 6, 2));
    let after = with_state(|s| s.rng.old_seed);
    assert_eq!(before, after);
    with_state(|s| assert_eq!(s.monsters[2].speed, 13));
}

#[test]
fn speed_monster_slow_resist_roll_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(2, CENTIPEDE_ID, 10, Coord_t { y: 10, x: 11 }, true, 1, 11);

    assert!(spell_speed_monster(Coord_t { y: 10, x: 10 }, 6, -2));
    assert_eq!(next_random_pair(i32::from(MON_MAX_LEVELS)), (40, 33));
    with_state(|s| assert_eq!(s.monsters[2].speed, 9));
}

#[test]
fn confuse_monster_success_duration_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(2, URCHIN_ID, 10, Coord_t { y: 10, x: 11 }, true, 1, 11);

    assert!(spell_confuse_monster(Coord_t { y: 10, x: 10 }, 6));
    with_state(|s| {
        assert!(s.monsters[2].confused_amount >= 2);
        assert_eq!(s.monsters[2].sleep_count, 0);
    });
}

#[test]
fn confuse_monster_cd_no_sleep_short_circuit_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(
        2,
        COPPER_COINS_ID,
        50,
        Coord_t { y: 10, x: 11 },
        true,
        1,
        10,
    );

    let before = with_state(|s| s.rng.old_seed);
    assert!(!spell_confuse_monster(Coord_t { y: 10, x: 10 }, 6));
    let after = with_state(|s| s.rng.old_seed);
    assert_eq!(before, after);
    with_state(|s| {
        assert_ne!(
            s.creature_recall[COPPER_COINS_ID as usize].defenses & CD_NO_SLEEP,
            0
        );
    });
}

#[test]
fn sleep_monster_success_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(2, URCHIN_ID, 10, Coord_t { y: 10, x: 11 }, true, 1, 11);

    assert!(spell_sleep_monster(Coord_t { y: 10, x: 10 }, 6));
    assert_eq!(next_random_pair(i32::from(MON_MAX_LEVELS)), (40, 33));
    with_state(|s| assert_eq!(s.monsters[2].sleep_count, 500));
}

// ---------------------------------------------------------------------------
// 4. Polymorph / clone
// ---------------------------------------------------------------------------
#[test]
fn polymorph_monster_replaces_creature_seed42() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(2, URCHIN_ID, 10, Coord_t { y: 10, x: 11 }, true, 1, 11);

    assert!(spell_polymorph_monster(Coord_t { y: 10, x: 10 }, 6));
    with_state(|s| {
        assert_ne!(s.monsters[2].creature_id, URCHIN_ID);
        assert_eq!(s.dg.floor[10][11].creature_id, 2);
    });
}

#[test]
fn clone_monster_attempts_multiply_seed42() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(2, URCHIN_ID, 10, Coord_t { y: 10, x: 11 }, true, 1, 11);

    let _ = spell_clone_monster(Coord_t { y: 10, x: 10 }, 6);
    with_state(|s| assert_eq!(s.monsters[2].sleep_count, 0));
}

// ---------------------------------------------------------------------------
// 5. Teleport away
// ---------------------------------------------------------------------------
#[test]
fn teleport_away_monster_moves_off_tile_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(2, URCHIN_ID, 10, Coord_t { y: 10, x: 11 }, true, 1, 11);

    spell_teleport_away_monster(2, i32::from(MON_MAX_SIGHT));
    with_state(|s| {
        assert_ne!(s.monsters[2].pos, Coord_t { y: 10, x: 11 });
        assert_eq!(s.dg.floor[10][11].creature_id, 0);
        assert!(!s.monsters[2].lit);
    });
}

#[test]
fn teleport_away_in_direction_wakes_and_moves_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(2, URCHIN_ID, 10, Coord_t { y: 10, x: 11 }, true, 1, 11);

    assert!(spell_teleport_away_monster_in_direction(
        Coord_t { y: 10, x: 10 },
        6
    ));
    with_state(|s| {
        assert_eq!(s.monsters[2].sleep_count, 0);
        assert_ne!(s.monsters[2].pos, Coord_t { y: 10, x: 11 });
    });
}

// ---------------------------------------------------------------------------
// 6. Genocide / mass genocide — zero RNG
// ---------------------------------------------------------------------------
#[test]
fn mass_genocide_removes_in_sight_non_winners_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(2, URCHIN_ID, 10, Coord_t { y: 10, x: 11 }, true, 1, 11);
    place_monster(3, BALROG_ID, 3000, Coord_t { y: 10, x: 12 }, true, 2, 13);

    let before = with_state(|s| s.rng.old_seed);
    assert!(spell_mass_genocide());
    let after = with_state(|s| s.rng.old_seed);
    assert_eq!(before, after);
    with_state(|s| {
        assert_eq!(s.next_free_monster_id, 3);
        assert_eq!(s.monsters[2].creature_id, BALROG_ID);
        assert!(s.monsters[2].hp > 0);
    });
}

#[test]
fn genocide_aborted_consumes_no_rng() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    test_push_getch_keys(&[ESCAPE_KEY]);

    let before = with_state(|s| s.rng.old_seed);
    assert!(!spell_genocide());
    let after = with_state(|s| s.rng.old_seed);
    assert_eq!(before, after);
}

#[test]
fn genocide_kills_matching_sprite_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(2, URCHIN_ID, 10, Coord_t { y: 10, x: 11 }, true, 1, 11);
    test_push_getch_keys(&[b'p' as i32]);

    let before = with_state(|s| s.rng.old_seed);
    assert!(spell_genocide());
    let after = with_state(|s| s.rng.old_seed);
    assert_eq!(before, after);
    with_state(|s| assert_eq!(s.next_free_monster_id, 2));
}

const ESCAPE_KEY: i32 = 27;

// ---------------------------------------------------------------------------
// 7. Area spells — speed/sleep all, mass polymorph, dispel, turn undead
// ---------------------------------------------------------------------------
#[test]
fn speed_all_monsters_haste_lit_only_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(2, URCHIN_ID, 10, Coord_t { y: 10, x: 11 }, true, 1, 11);
    place_monster(3, URCHIN_ID, 10, Coord_t { y: 15, x: 15 }, false, 25, 11);

    assert!(spell_speed_all_monsters(3));
    with_state(|s| {
        assert_eq!(s.monsters[2].speed, 14);
        assert_eq!(s.monsters[3].speed, 11);
    });
}

#[test]
fn sleep_all_resist_roll_per_eligible_monster_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(2, URCHIN_ID, 10, Coord_t { y: 10, x: 11 }, true, 1, 11);

    assert!(spell_sleep_all_monsters());
    assert_eq!(next_random_pair(i32::from(MON_MAX_LEVELS)), (40, 33));
    with_state(|s| assert_eq!(s.monsters[2].sleep_count, 500));
}

#[test]
fn mass_polymorph_replaces_in_sight_seed42() {
    reset_for_new_game(Some(42));
    init_monster_levels();
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(2, URCHIN_ID, 10, Coord_t { y: 10, x: 11 }, true, 1, 11);

    assert!(spell_mass_polymorph());
    with_state(|s| {
        assert_ne!(s.monsters[2].creature_id, URCHIN_ID);
    });
}

#[test]
fn dispel_creature_rolls_damage_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(
        2,
        ZOMBIE_KOBOLD_ID,
        200,
        Coord_t { y: 10, x: 11 },
        true,
        1,
        11,
    );

    assert!(spell_dispel_creature(i32::from(CD_UNDEAD), 100));
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn turn_undead_confusion_amount_truncates_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    reset_monster_slots();
    setup_player(Coord_t { y: 10, x: 10 }, 10);
    place_monster(
        2,
        ZOMBIE_KOBOLD_ID,
        200,
        Coord_t { y: 10, x: 11 },
        true,
        1,
        11,
    );

    assert!(spell_turn_undead());
    with_state(|s| assert_eq!(s.monsters[2].confused_amount, 10));
}

#[test]
fn bolt_max_range_is_eighteen() {
    assert_eq!(OBJECT_BOLTS_MAX_RANGE, 18);
}
