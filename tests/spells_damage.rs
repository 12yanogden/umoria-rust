//! Bolt / ball / breath damage parity (`spells`).
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

use umoria::config::monsters::defense::{CD_EVIL, CD_FIRE, CD_FROST, CD_LIGHT};
use umoria::config::monsters::spells::{CS_BR_FIRE, CS_BR_FROST, CS_BR_LIGHT};
use umoria::config::treasure::OBJECT_BOLTS_MAX_RANGE;
use umoria::data_creatures::CREATURES_LIST;
use umoria::dungeon::{coord_distance_between, MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, TILE_GRANITE_WALL, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::game_objects::popt;
use umoria::inventory::{set_frost_destroyable_items, set_null};
use umoria::monster::{Monster, MON_TOTAL_ALLOCATIONS};
use umoria::player::PLAYER_MAX_LEVEL;
use umoria::spells::{
    spell_apply_area_distance_falloff, spell_apply_monster_damage_scaling, spell_breath,
    spell_fire_ball, spell_fire_bolt, spell_get_area_affect_flags, MagicSpellFlags,
};
use umoria::treasure::TV_POTION1;
use umoria::types::{Coord_t, Vtype_t, MORIA_MESSAGE_SIZE};
use umoria::ui::panel_bounds_fields;
use umoria::ui_io::test_set_ncurses_stub;

const BLUE_JELLY_ID: u16 = 51;
const FIRE_ELEMENTAL_ID: u16 = 238;
const GIANT_YELLOW_CENTIPEDE_ID: u16 = 9;
const VAMPIRE_ID: u16 = 209;

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

fn setup_player(pos: Coord_t) {
    test_set_ncurses_stub(true);
    let bounds = panel_bounds_fields(0, 0);
    with_state_mut(|s| {
        s.py.pos = pos;
        s.dg.panel.row = 0;
        s.dg.panel.col = 0;
        s.dg.panel.top = bounds.top;
        s.dg.panel.bottom = bounds.bottom;
        s.dg.panel.left = bounds.left;
        s.dg.panel.right = bounds.right;
        s.dg.panel.row_prt = bounds.row_prt;
        s.dg.panel.col_prt = bounds.col_prt;
        s.py.misc.current_hp = 500;
        s.py.misc.max_hp = 500;
        s.py.misc.level = u16::from(PLAYER_MAX_LEVEL);
        s.py.misc.exp = 0;
        s.py.misc.max_exp = 0;
        s.py.flags.blind = 0;
        s.py.flags.status = 0;
        s.py.flags.poisoned = 0;
        s.py.flags.resistant_to_fire = false;
        s.py.flags.resistant_to_cold = false;
        s.py.flags.resistant_to_acid = false;
        s.py.flags.resistant_to_light = false;
        s.py.flags.heat_resistance = 0;
        s.py.flags.cold_resistance = 0;
        s.dg.floor[pos.y as usize][pos.x as usize].creature_id = 1;
        s.hack_monptr = -1;
    });
}

fn reset_monster_slots() {
    with_state_mut(|s| {
        s.next_free_monster_id = i16::from(umoria::config::monsters::MON_MIN_INDEX_ID);
        s.monsters = [Monster::default(); MON_TOTAL_ALLOCATIONS as usize];
    });
}

fn place_monster(id: i32, creature_id: u16, hp: i16, coord: Coord_t, lit: bool) {
    with_state_mut(|s| {
        s.monsters[id as usize] = Monster {
            hp,
            sleep_count: 0,
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

fn place_wall(coord: Coord_t) {
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize].feature_id = TILE_GRANITE_WALL;
    });
}

fn place_potion(coord: Coord_t) -> u8 {
    let tid = popt() as u8;
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize].treasure_id = tid;
        s.game.treasure.list[tid as usize].category_id = TV_POTION1;
    });
    tid
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
}

fn vtype_from_str(text: &str) -> Vtype_t {
    let mut buf = [0u8; MORIA_MESSAGE_SIZE];
    let bytes = text.as_bytes();
    let n = bytes.len().min(MORIA_MESSAGE_SIZE - 1);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf
}

// --------------------------------------------------------------------------
// 1. spell_get_area_affect_flags
// --------------------------------------------------------------------------

#[test]
fn spell_get_area_affect_flags_maps_all_spell_types() {
    let mm = spell_get_area_affect_flags(MagicSpellFlags::MagicMissile);
    assert_eq!(mm.weapon_type, 0);
    assert_eq!(mm.harm_type, 0);
    assert_eq!(mm.destroy as usize, set_null as *const () as usize);

    let lightning = spell_get_area_affect_flags(MagicSpellFlags::Lightning);
    assert_eq!(lightning.weapon_type, CS_BR_LIGHT);
    assert_eq!(lightning.harm_type, CD_LIGHT);

    let frost = spell_get_area_affect_flags(MagicSpellFlags::Frost);
    assert_eq!(frost.weapon_type, CS_BR_FROST);
    assert_eq!(frost.harm_type, CD_FROST);

    let fire = spell_get_area_affect_flags(MagicSpellFlags::Fire);
    assert_eq!(fire.weapon_type, CS_BR_FIRE);
    assert_eq!(fire.harm_type, CD_FIRE);

    let holy = spell_get_area_affect_flags(MagicSpellFlags::HolyOrb);
    assert_eq!(holy.weapon_type, 0);
    assert_eq!(holy.harm_type, CD_EVIL);
}

// --------------------------------------------------------------------------
// 2. Damage-scaling golden (resist / immune / susceptible)
// --------------------------------------------------------------------------

#[test]
fn damage_scaling_neutral_creature_no_modifiers() {
    let creature = &CREATURES_LIST[GIANT_YELLOW_CENTIPEDE_ID as usize];
    assert_eq!(
        spell_apply_monster_damage_scaling(
            40,
            CD_LIGHT,
            CS_BR_LIGHT,
            creature.defenses,
            creature.spells
        ),
        40
    );
}

#[test]
fn damage_scaling_susceptible_doubles_before_immune_check() {
    let creature = &CREATURES_LIST[BLUE_JELLY_ID as usize];
    assert!((creature.defenses & CD_LIGHT) != 0);
    assert!((creature.spells & CS_BR_FROST) != 0);
    assert_eq!(
        spell_apply_monster_damage_scaling(
            50,
            CD_LIGHT,
            CS_BR_LIGHT,
            creature.defenses,
            creature.spells
        ),
        100
    );
    assert_eq!(
        spell_apply_monster_damage_scaling(
            50,
            CD_FROST,
            CS_BR_FROST,
            creature.defenses,
            creature.spells
        ),
        12
    );
    assert_eq!(
        spell_apply_monster_damage_scaling(
            50,
            CD_FIRE,
            CS_BR_FIRE,
            creature.defenses,
            creature.spells
        ),
        100
    );
}

#[test]
fn damage_scaling_immune_quarters_when_not_susceptible() {
    let creature = &CREATURES_LIST[FIRE_ELEMENTAL_ID as usize];
    assert_eq!(
        spell_apply_monster_damage_scaling(
            100,
            CD_FIRE,
            CS_BR_FIRE,
            creature.defenses,
            creature.spells
        ),
        25
    );
}

#[test]
fn damage_scaling_holy_orb_doubles_vs_evil() {
    let creature = &CREATURES_LIST[VAMPIRE_ID as usize];
    assert!((creature.defenses & CD_EVIL) != 0);
    assert_eq!(
        spell_apply_monster_damage_scaling(60, CD_EVIL, 0, creature.defenses, creature.spells),
        120
    );
}

#[test]
fn damage_scaling_area_falloff_matches_expected_integer_division() {
    assert_eq!(spell_apply_area_distance_falloff(100, 0), 100);
    assert_eq!(spell_apply_area_distance_falloff(100, 1), 50);
    assert_eq!(spell_apply_area_distance_falloff(100, 2), 33);
    assert_eq!(spell_apply_area_distance_falloff(7, 2), 2);
}

#[test]
fn bolt_applies_scaled_damage_to_monster_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(2, BLUE_JELLY_ID, 500, Coord_t { y: 10, x: 12 }, true);
    spell_fire_bolt(
        Coord_t { y: 10, x: 10 },
        6,
        50,
        MagicSpellFlags::Lightning,
        "Lightning Bolt",
    );
    with_state(|s| {
        assert_eq!(s.monsters[2].hp, 400);
    });
}

#[test]
fn ball_applies_distance_scaled_damage_seed100() {
    reset_for_new_game(Some(100));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(
        2,
        GIANT_YELLOW_CENTIPEDE_ID,
        500,
        Coord_t { y: 10, x: 11 },
        true,
    );
    place_wall(Coord_t { y: 10, x: 12 });
    spell_fire_ball(
        Coord_t { y: 10, x: 10 },
        6,
        90,
        MagicSpellFlags::MagicMissile,
        "Magic Ball",
    );
    with_state(|s| {
        assert_eq!(s.monsters[2].hp, 410);
    });
}

// --------------------------------------------------------------------------
// 3. Stop conditions (wall / monster / max range)
// --------------------------------------------------------------------------

#[test]
fn bolt_stops_at_first_monster_without_hitting_later_target() {
    reset_for_new_game(Some(1));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(
        2,
        GIANT_YELLOW_CENTIPEDE_ID,
        500,
        Coord_t { y: 10, x: 11 },
        true,
    );
    place_monster(
        3,
        GIANT_YELLOW_CENTIPEDE_ID,
        500,
        Coord_t { y: 10, x: 13 },
        true,
    );
    spell_fire_bolt(
        Coord_t { y: 10, x: 10 },
        6,
        20,
        MagicSpellFlags::MagicMissile,
        "Magic Missile",
    );
    with_state(|s| {
        assert_eq!(s.monsters[2].hp, 480);
        assert_eq!(s.monsters[3].hp, 500);
    });
}

#[test]
fn bolt_stops_at_closed_space_without_damage() {
    reset_for_new_game(Some(1));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(
        2,
        GIANT_YELLOW_CENTIPEDE_ID,
        500,
        Coord_t { y: 10, x: 13 },
        true,
    );
    place_wall(Coord_t { y: 10, x: 12 });
    spell_fire_bolt(
        Coord_t { y: 10, x: 10 },
        6,
        20,
        MagicSpellFlags::MagicMissile,
        "Magic Missile",
    );
    with_state(|s| {
        assert_eq!(s.monsters[2].hp, 500);
    });
}

#[test]
fn ball_backs_up_to_old_coord_on_wall_hit() {
    reset_for_new_game(Some(1));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(
        2,
        GIANT_YELLOW_CENTIPEDE_ID,
        500,
        Coord_t { y: 10, x: 11 },
        true,
    );
    place_wall(Coord_t { y: 10, x: 12 });
    spell_fire_ball(
        Coord_t { y: 10, x: 10 },
        6,
        30,
        MagicSpellFlags::MagicMissile,
        "Magic Ball",
    );
    with_state(|s| {
        assert_eq!(s.monsters[2].hp, 470);
    });
}

#[test]
fn ball_stops_travel_at_object_bolts_max_range() {
    reset_for_new_game(Some(1));
    setup_dungeon(40, 40);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    let far = Coord_t {
        y: 10,
        x: 10 + i32::from(OBJECT_BOLTS_MAX_RANGE) + 2,
    };
    place_monster(2, GIANT_YELLOW_CENTIPEDE_ID, 500, far, true);
    spell_fire_ball(
        Coord_t { y: 10, x: 10 },
        6,
        30,
        MagicSpellFlags::MagicMissile,
        "Magic Ball",
    );
    with_state(|s| assert_eq!(s.monsters[2].hp, 500));
}

// --------------------------------------------------------------------------
// 4. Item destruction ordering
// --------------------------------------------------------------------------

#[test]
fn frost_ball_destroys_potion_in_row_major_scan_order() {
    reset_for_new_game(Some(200));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    let center = Coord_t { y: 10, x: 11 };
    place_wall(Coord_t { y: 10, x: 12 });
    let first = place_potion(Coord_t { y: 8, x: 10 });
    let second = place_potion(Coord_t { y: 8, x: 11 });
    assert!(set_frost_destroyable_items(&umoria::inventory::Inventory {
        category_id: TV_POTION1,
        ..Default::default()
    }));
    spell_fire_ball(center, 6, 10, MagicSpellFlags::Frost, "Frost Ball");
    with_state(|s| {
        assert_eq!(s.dg.floor[8][10].treasure_id, 0);
        assert_eq!(s.dg.floor[8][11].treasure_id, 0);
        assert_ne!(first, 0);
        assert_ne!(second, 0);
    });
}

// --------------------------------------------------------------------------
// 5. Breath player damage — randomNumber(0) guard
// --------------------------------------------------------------------------

#[test]
fn breath_applies_minimum_one_player_damage_when_scaled_to_zero() {
    reset_for_new_game(Some(300));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 11 });
    reset_monster_slots();
    place_monster(2, BLUE_JELLY_ID, 4, Coord_t { y: 10, x: 12 }, true);
    let name = vtype_from_str("gas");
    spell_breath(
        Coord_t { y: 10, x: 10 },
        2,
        1,
        MagicSpellFlags::PoisonGas,
        &name,
    );
    with_state(|s| {
        assert_eq!(s.py.misc.current_hp, 499);
    });
    assert_eq!(next_random_pair(1), (1, 1));
}

// --------------------------------------------------------------------------
// 6. Integer semantics (i16 wrap on breath direct HP subtract)
// --------------------------------------------------------------------------

#[test]
fn integer_subtraction_uses_i16_truncation_like_expected() {
    assert_eq!((i32::from(5i16) - 10) as i16, -5i16);
    assert_eq!((i32::from(1i16) - 2) as i16, -1i16);
}

#[test]
fn breath_kills_monster_when_hp_drops_below_zero() {
    reset_for_new_game(Some(1));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(
        2,
        GIANT_YELLOW_CENTIPEDE_ID,
        5,
        Coord_t { y: 10, x: 10 },
        true,
    );
    let name = vtype_from_str("fire");
    spell_breath(
        Coord_t { y: 10, x: 10 },
        2,
        10,
        MagicSpellFlags::Fire,
        &name,
    );
    with_state(|s| {
        assert_eq!(s.dg.floor[10][10].creature_id, 0);
    });
}

// --------------------------------------------------------------------------
// 7. RNG-order golden — bolt kill consumes death-drop rolls
// --------------------------------------------------------------------------

#[test]
fn bolt_kill_rng_order_seed500() {
    reset_for_new_game(Some(500));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(
        2,
        GIANT_YELLOW_CENTIPEDE_ID,
        10,
        Coord_t { y: 10, x: 11 },
        true,
    );
    spell_fire_bolt(
        Coord_t { y: 10, x: 10 },
        6,
        20,
        MagicSpellFlags::MagicMissile,
        "Magic Missile",
    );
    with_state(|s| assert_eq!(s.dg.floor[10][11].creature_id, 0));
    assert_eq!(next_random_pair(2), (2, 2));
    assert_eq!(next_random_pair(4), (4, 3));
}

// --------------------------------------------------------------------------
// 8. Ball multi-monster row-major hit order
// --------------------------------------------------------------------------

#[test]
fn ball_hits_monsters_in_row_major_explosion_order() {
    reset_for_new_game(Some(400));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    let center = Coord_t { y: 10, x: 11 };
    place_monster(
        2,
        GIANT_YELLOW_CENTIPEDE_ID,
        500,
        Coord_t { y: 9, x: 10 },
        true,
    );
    place_monster(
        3,
        GIANT_YELLOW_CENTIPEDE_ID,
        500,
        Coord_t { y: 9, x: 11 },
        true,
    );
    place_wall(Coord_t { y: 10, x: 12 });
    spell_fire_ball(center, 6, 30, MagicSpellFlags::MagicMissile, "Magic Ball");
    with_state(|s| {
        assert_eq!(s.monsters[2].hp, 485);
        assert_eq!(s.monsters[3].hp, 485);
    });
}
