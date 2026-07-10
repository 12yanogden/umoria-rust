//! Phase 4.7.1 — spells.cpp selection & detection parity.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::config::dungeon::objects::{MAX_TRAPS, OBJ_CLOSED_DOOR};
use umoria::config::monsters::defense::CD_EVIL;
use umoria::config::monsters::move_flags::CM_INVISIBLE;
use umoria::config::spells::{NAME_OFFSET_SPELLS, SPELL_TYPE_MAGE};
use umoria::config::treasure::chests::{CH_LOCKED, CH_TRAPPED};
use umoria::data_creatures::CREATURES_LIST;
use umoria::data_player::{CLASSES, MAGIC_SPELLS};
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, TILE_BLOCKED_FLOOR, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::game_objects::popt;
use umoria::helpers::get_and_clear_first_bit;
use umoria::identification::SpecialNameIds;
use umoria::monster::Monster;
use umoria::player::PlayerAttr;
use umoria::spells::{
    build_castable_spell_list, cast_spell_get_id, spell_aggravate_monsters,
    spell_chance_of_success, spell_destroy_adjacent_doors_traps, spell_detect_evil,
    spell_detect_invisible_creatures_within_vicinity, spell_detect_monsters,
    spell_detect_objects_within_vicinity, spell_detect_secret_doors_within_vicinity,
    spell_detect_traps_within_vicinity, spell_detect_treasure_within_vicinity,
    spell_map_current_area, spell_surround_player_with_doors, spell_surround_player_with_traps,
};
use umoria::treasure::{
    TV_CHEST, TV_GOLD, TV_INVIS_TRAP, TV_SECRET_DOOR, TV_SWORD, TV_UP_STAIR, TV_VIS_TRAP,
};
use umoria::types::Coord_t;
use umoria::ui::panel_bounds_fields;
use umoria::ui_io::{test_clear_getch_keys, test_push_getch_keys, test_set_ncurses_stub, ESCAPE};

const MAGE_CLASS_ID: u8 = 2;
const FLOATING_EYE_ID: u16 = 18;
const LOST_SOUL_ID: u16 = 87;
const ORC_ID: u16 = 77;

fn assert_rng_unchanged_after(setup: impl Fn(), action: impl FnOnce()) {
    reset_for_new_game(Some(7));
    setup();
    let baseline = random_number(100);
    reset_for_new_game(Some(7));
    setup();
    action();
    assert_eq!(random_number(100), baseline);
}

fn next_random_pair(max: i32) -> (i32, i32) {
    (max, random_number(max))
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

fn setup_player_panel(pos: Coord_t) {
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
        s.py.misc.class_id = MAGE_CLASS_ID;
        s.py.misc.level = 10;
        s.py.misc.current_mana = 100;
        s.py.stats.used[PlayerAttr::A_INT as usize] = 18;
        s.py.flags.spells_learnt = 0;
        s.message_ready_to_print = true;
    });
}

fn place_treasure(coord: Coord_t, category_id: u8, flags: u32) -> u8 {
    let treasure_id = popt();
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize].treasure_id = treasure_id as u8;
        let item = &mut s.game.treasure.list[treasure_id as usize];
        item.category_id = category_id;
        item.flags = flags;
    });
    treasure_id as u8
}

fn place_monster(id: i32, creature_id: u16, coord: Coord_t) {
    with_state_mut(|s| {
        s.monsters[id as usize] = Monster {
            hp: 10,
            creature_id,
            pos: coord,
            distance_from_player: 1,
            ..Default::default()
        };
        s.dg.floor[coord.y as usize][coord.x as usize].creature_id = id as u8;
        if id + 1 >= i32::from(s.next_free_monster_id) {
            s.next_free_monster_id = (id + 1) as i16;
        }
    });
}

fn tile_at(coord: Coord_t) -> umoria::dungeon_tile::Tile {
    with_state(|s| s.dg.floor[coord.y as usize][coord.x as usize])
}

// ---------------------------------------------------------------------------
// 1. RNG-order golden — spellMapCurrentArea
// ---------------------------------------------------------------------------

#[test]
fn spell_map_current_area_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(66, 66);
    setup_player_panel(Coord_t { y: 15, x: 20 });
    let fifth = {
        random_number(10);
        random_number(10);
        random_number(20);
        random_number(20);
        random_number(10)
    };

    reset_for_new_game(Some(42));
    setup_dungeon(66, 66);
    setup_player_panel(Coord_t { y: 15, x: 20 });
    spell_map_current_area();

    assert_eq!(next_random_pair(10), (10, fifth));
}

// ---------------------------------------------------------------------------
// 2. RNG-order golden — spellSurroundPlayerWithTraps
// ---------------------------------------------------------------------------

#[test]
fn spell_surround_player_with_traps_rng_count_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(30, 30);
    setup_player_panel(Coord_t { y: 15, x: 15 });
    let ninth = {
        for _ in 0..8 {
            random_number(i32::from(MAX_TRAPS));
        }
        random_number(10)
    };

    reset_for_new_game(Some(42));
    setup_dungeon(30, 30);
    setup_player_panel(Coord_t { y: 15, x: 15 });
    assert!(spell_surround_player_with_traps());

    assert_eq!(next_random_pair(10), (10, ninth));
}

// ---------------------------------------------------------------------------
// 3. build_castable_spell_list / spellGetId filtering
// ---------------------------------------------------------------------------

#[test]
fn build_castable_spell_list_filters_by_level_and_learnt() {
    reset_for_new_game(Some(1));
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.misc.class_id = MAGE_CLASS_ID;
        s.py.misc.level = 3;
        s.py.flags.spells_learnt = 0b1_1111;
        s.py.inventory[0].flags = 0b1_0010; // spells 1 and 4 in book
    });

    let (first_spell, spell_ids) = build_castable_spell_list(0).expect("some spells");
    assert_eq!(first_spell, 1);
    assert_eq!(spell_ids, vec![1, 4]);

    with_state_mut(|s| {
        s.py.misc.level = 1;
    });
    let (first_spell, spell_ids) = build_castable_spell_list(0).expect("some spells");
    assert_eq!(first_spell, 1);
    assert_eq!(spell_ids, vec![1]);
}

#[test]
fn spell_get_id_selection_consumes_no_rng() {
    reset_for_new_game(Some(99));
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    let baseline = random_number(100);
    reset_for_new_game(Some(99));
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[b'a' as i32]);

    with_state_mut(|s| {
        s.py.misc.class_id = MAGE_CLASS_ID;
        s.py.misc.level = 10;
        s.py.stats.used[PlayerAttr::A_INT as usize] = 18;
        s.py.misc.current_mana = 100;
        s.py.flags.spells_learnt = 1 << 0;
        s.py.inventory[0].flags = 1;
    });

    let mut spell_id = -1;
    let mut spell_chance = 0;
    let result = cast_spell_get_id("Cast which?", 0, &mut spell_id, &mut spell_chance);
    assert_eq!(result, 1);
    assert_eq!(spell_id, 0);
    assert_eq!(spell_chance, spell_chance_of_success(spell_id));
    assert_eq!(random_number(100), baseline);
}

#[test]
fn spell_chance_clamps_match_cpp() {
    reset_for_new_game(Some(1));
    with_state_mut(|s| {
        s.py.misc.class_id = MAGE_CLASS_ID;
        s.py.misc.level = 40;
        s.py.stats.used[PlayerAttr::A_INT as usize] = 18;
        s.py.misc.current_mana = 255;
    });
    assert_eq!(spell_chance_of_success(0), 5);

    with_state_mut(|s| {
        s.py.misc.level = 1;
        s.py.misc.current_mana = 0;
        s.py.stats.used[PlayerAttr::A_INT as usize] = 18;
    });
    assert_eq!(spell_chance_of_success(22), 95);
}

// ---------------------------------------------------------------------------
// 4. castSpellGetId return contract
// ---------------------------------------------------------------------------

#[test]
fn cast_spell_get_id_no_known_spells_returns_minus_one() {
    reset_for_new_game(Some(1));
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.misc.class_id = MAGE_CLASS_ID;
        s.py.misc.level = 10;
        s.py.flags.spells_learnt = 0;
        s.py.inventory[0].flags = 0b11;
    });

    let mut spell_id = 0;
    let mut spell_chance = 0;
    assert_eq!(
        cast_spell_get_id("Cast which?", 0, &mut spell_id, &mut spell_chance),
        -1
    );
}

#[test]
fn cast_spell_get_id_escape_returns_zero() {
    reset_for_new_game(Some(1));
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(ESCAPE)]);

    with_state_mut(|s| {
        s.py.misc.class_id = MAGE_CLASS_ID;
        s.py.misc.level = 10;
        s.py.flags.spells_learnt = 1;
        s.py.inventory[0].flags = 1;
    });

    let mut spell_id = 0;
    let mut spell_chance = 0;
    assert_eq!(
        cast_spell_get_id("Cast which?", 0, &mut spell_id, &mut spell_chance),
        0
    );
}

#[test]
fn cast_spell_get_id_low_mana_confirm_abort_returns_zero() {
    reset_for_new_game(Some(1));
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[b'n' as i32, b'a' as i32]);

    with_state_mut(|s| {
        s.py.misc.class_id = MAGE_CLASS_ID;
        s.py.misc.level = 10;
        s.py.flags.spells_learnt = 1;
        s.py.inventory[0].flags = 1;
        s.py.misc.current_mana = 0;
    });

    let mut spell_id = 0;
    let mut spell_chance = 0;
    assert_eq!(
        cast_spell_get_id("Cast which?", 0, &mut spell_id, &mut spell_chance),
        0
    );
}

// ---------------------------------------------------------------------------
// 5. Detection extents/flags (zero RNG)
// ---------------------------------------------------------------------------

#[test]
fn spell_detect_treasure_marks_hidden_gold_only() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(66, 66);
            setup_player_panel(Coord_t { y: 15, x: 20 });
            place_treasure(Coord_t { y: 15, x: 21 }, TV_GOLD, 0);
            place_treasure(Coord_t { y: 16, x: 21 }, TV_SWORD, 0);
        },
        || {
            assert!(spell_detect_treasure_within_vicinity());
        },
    );
    assert!(tile_at(Coord_t { y: 15, x: 21 }).field_mark);
    assert!(!tile_at(Coord_t { y: 16, x: 21 }).field_mark);
}

#[test]
fn spell_detect_objects_marks_non_gold_objects() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(66, 66);
            setup_player_panel(Coord_t { y: 15, x: 20 });
            place_treasure(Coord_t { y: 15, x: 21 }, TV_SWORD, 0);
        },
        || {
            assert!(spell_detect_objects_within_vicinity());
            assert!(tile_at(Coord_t { y: 15, x: 21 }).field_mark);
        },
    );
}

#[test]
fn spell_detect_traps_reveals_invis_trap() {
    reset_for_new_game(Some(7));
    setup_dungeon(66, 66);
    setup_player_panel(Coord_t { y: 15, x: 20 });
    let trap_coord = Coord_t { y: 14, x: 20 };
    place_treasure(trap_coord, TV_INVIS_TRAP, 0);
    let baseline = random_number(100);
    reset_for_new_game(Some(7));
    setup_dungeon(66, 66);
    setup_player_panel(Coord_t { y: 15, x: 20 });
    let treasure_id = place_treasure(trap_coord, TV_INVIS_TRAP, 0);
    assert!(spell_detect_traps_within_vicinity());
    with_state(|s| {
        assert_eq!(
            s.game.treasure.list[treasure_id as usize].category_id,
            TV_VIS_TRAP
        );
    });
    assert_eq!(random_number(100), baseline);
}

#[test]
fn spell_detect_secret_doors_reveals_secret_and_stairs() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(66, 66);
            setup_player_panel(Coord_t { y: 15, x: 20 });
            place_treasure(Coord_t { y: 14, x: 21 }, TV_SECRET_DOOR, 0);
            place_treasure(Coord_t { y: 16, x: 21 }, TV_UP_STAIR, 0);
        },
        || {
            assert!(spell_detect_secret_doors_within_vicinity());
            assert!(tile_at(Coord_t { y: 14, x: 21 }).field_mark);
            assert!(tile_at(Coord_t { y: 16, x: 21 }).field_mark);
        },
    );
}

#[test]
fn spell_detect_invisible_creatures_lights_invisible_on_panel() {
    reset_for_new_game(Some(7));
    setup_dungeon(66, 66);
    setup_player_panel(Coord_t { y: 15, x: 20 });
    assert!(CREATURES_LIST[LOST_SOUL_ID as usize].movement & CM_INVISIBLE != 0);
    place_monster(2, LOST_SOUL_ID, Coord_t { y: 15, x: 22 });
    assert!(spell_detect_invisible_creatures_within_vicinity());
}

#[test]
fn spell_detect_monsters_lights_visible_on_panel() {
    reset_for_new_game(Some(7));
    setup_dungeon(66, 66);
    setup_player_panel(Coord_t { y: 15, x: 20 });
    assert!(CREATURES_LIST[FLOATING_EYE_ID as usize].movement & CM_INVISIBLE == 0);
    place_monster(2, FLOATING_EYE_ID, Coord_t { y: 15, x: 22 });
    assert!(spell_detect_monsters());
}

#[test]
fn spell_detect_evil_lights_evil_on_panel() {
    reset_for_new_game(Some(7));
    setup_dungeon(66, 66);
    setup_player_panel(Coord_t { y: 15, x: 20 });
    assert!(CREATURES_LIST[ORC_ID as usize].defenses & CD_EVIL != 0);
    place_monster(2, ORC_ID, Coord_t { y: 15, x: 22 });
    assert!(spell_detect_evil());
}

// ---------------------------------------------------------------------------
// 6. Utility spells
// ---------------------------------------------------------------------------

#[test]
fn spell_aggravate_monsters_speeds_nearby_sleepers() {
    reset_for_new_game(Some(1));
    setup_dungeon(30, 30);
    setup_player_panel(Coord_t { y: 15, x: 15 });
    place_monster(2, ORC_ID, Coord_t { y: 15, x: 16 });
    with_state_mut(|s| {
        s.monsters[2].sleep_count = 5;
        s.monsters[2].speed = 0;
        s.monsters[2].distance_from_player = 1;
    });

    assert!(spell_aggravate_monsters(10));
    with_state(|s| {
        assert_eq!(s.monsters[2].sleep_count, 0);
        assert_eq!(s.monsters[2].speed, 1);
    });
}

#[test]
fn spell_surround_player_with_doors_places_adjacent_doors() {
    reset_for_new_game(Some(1));
    setup_dungeon(30, 30);
    setup_player_panel(Coord_t { y: 15, x: 15 });

    assert!(spell_surround_player_with_doors());

    let north = tile_at(Coord_t { y: 14, x: 15 });
    assert_eq!(north.feature_id, TILE_BLOCKED_FLOOR);
    assert_ne!(north.treasure_id, 0);
    with_state(|s| {
        let item = &s.game.treasure.list[north.treasure_id as usize];
        assert_eq!(item.id, OBJ_CLOSED_DOOR);
    });
}

#[test]
fn spell_destroy_adjacent_doors_traps_disarms_chest() {
    reset_for_new_game(Some(1));
    setup_dungeon(30, 30);
    setup_player_panel(Coord_t { y: 15, x: 15 });

    let chest_coord = Coord_t { y: 15, x: 16 };
    let treasure_id = place_treasure(chest_coord, TV_CHEST, CH_TRAPPED | CH_LOCKED);
    with_state_mut(|s| {
        s.game.treasure.list[treasure_id as usize].special_name_id =
            SpecialNameIds::SN_LOCKED as u8;
    });

    assert!(spell_destroy_adjacent_doors_traps());
    with_state(|s| {
        let item = &s.game.treasure.list[treasure_id as usize];
        assert_eq!(item.flags & (CH_TRAPPED | CH_LOCKED), 0);
        assert_eq!(item.special_name_id, SpecialNameIds::SN_UNLOCKED as u8);
    });
}

#[test]
fn cast_spell_list_bit_order_matches_get_and_clear_first_bit() {
    let mut flags = 0b1010u32;
    let first = get_and_clear_first_bit(&mut flags);
    flags = 0b1010 & with_state(|_| 0b1111); // spells_learnt mask simulated
    let mut list = Vec::new();
    while flags != 0 {
        list.push(get_and_clear_first_bit(&mut flags));
    }
    assert_eq!(first, 1);
    assert_eq!(list, vec![1, 3]);
}

#[test]
fn mage_spell_menu_offset_is_spells_not_prayers() {
    assert_eq!(
        CLASSES[(MAGE_CLASS_ID - 1) as usize].class_to_use_mage_spells,
        SPELL_TYPE_MAGE
    );
    assert_eq!(NAME_OFFSET_SPELLS, 0);
    assert!(MAGIC_SPELLS[MAGE_CLASS_ID as usize - 1][0].level_required <= 10);
}
