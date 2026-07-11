//! Scroll reading (`scrolls`) tests.
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

use umoria::config::dungeon::objects::OBJ_NOTHING;
use umoria::config::monsters::MON_ENDGAME_MONSTERS;
use umoria::config::treasure::flags::TR_CURSED;
use umoria::data_creatures::CREATURES_LIST;
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::inventory::{
    inventory_item_copy_to, inventory_item_is_cursed, Inventory, PlayerEquipment,
    PLAYER_INVENTORY_SIZE,
};
use umoria::monster::{MON_MAX_CREATURES, MON_MAX_LEVELS};
use umoria::player::PLAYER_MAX_LEVEL;
use umoria::scrolls::{
    inventory_item_id_of_cursed_equipment, player_can_read_scroll, scroll_confuse_monster,
    scroll_curse_armor, scroll_curse_weapon, scroll_enchant_armor, scroll_enchant_item_to_ac,
    scroll_enchant_weapon, scroll_enchant_weapon_to_damage, scroll_enchant_weapon_to_hit,
    scroll_read, scroll_remove_curse, scroll_summon_monster, scroll_summon_undead,
    scroll_teleport_level, scroll_word_of_recall,
};
use umoria::spells::spell_enchant_item;
use umoria::treasure::{TV_HARD_ARMOR, TV_SCROLL1, TV_SCROLL2, TV_SWORD};
use umoria::types::Coord_t;
use umoria::ui::panel_bounds_fields;
use umoria::ui_io::{test_push_getch_keys, test_set_ncurses_stub, ESCAPE};

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
        s.dg.current_level = 10;
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
        s.py.flags.blind = 0;
        s.py.flags.confused = 0;
        s.py.flags.confuse_monster = false;
        s.py.flags.word_of_recall = 0;
        s.py.misc.level = u16::from(PLAYER_MAX_LEVEL);
        s.py.misc.exp = 999_999;
        s.py.misc.max_exp = 999_999;
        s.py.pack.unique_items = 0;
        s.py.pack.weight = 0;
        for i in 0..PLAYER_INVENTORY_SIZE as usize {
            inventory_item_copy_to(OBJ_NOTHING as i16, &mut s.py.inventory[i]);
        }
        s.dg.floor[pos.y as usize][pos.x as usize].temporary_light = true;
    });
}

fn init_monster_levels() {
    with_state_mut(|state| {
        state.monster_levels = [0; MON_MAX_LEVELS as usize + 1];
        let endgame = MON_ENDGAME_MONSTERS as usize;
        for i in 0..MON_MAX_CREATURES as usize - endgame {
            let level = CREATURES_LIST[i].level as usize;
            state.monster_levels[level] += 1;
        }
        for i in 1..=MON_MAX_LEVELS as usize {
            state.monster_levels[i] += state.monster_levels[i - 1];
        }
    });
}

fn make_scroll(flags: u32, category_id: u8) -> Inventory {
    Inventory {
        category_id,
        sub_category_id: 64,
        flags,
        items_count: 1,
        weight: 5,
        ..Default::default()
    }
}

fn pack_scroll(slot: i32, flags: u32, category_id: u8) {
    with_state_mut(|s| {
        s.py.inventory[slot as usize] = make_scroll(flags, category_id);
        if slot >= s.py.pack.unique_items as i32 {
            s.py.pack.unique_items = (slot + 1) as i16;
        }
    });
}

fn wield_item(category_id: u8) {
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Wield as usize].category_id = category_id;
        s.py.inventory[PlayerEquipment::Wield as usize].sub_category_id = 64;
        s.py.inventory[PlayerEquipment::Wield as usize].damage.dice = 2;
        s.py.inventory[PlayerEquipment::Wield as usize].damage.sides = 6;
    });
}

fn wear_armor(slot: PlayerEquipment) {
    with_state_mut(|s| {
        s.py.inventory[slot as usize].category_id = TV_HARD_ARMOR;
        s.py.inventory[slot as usize].sub_category_id = 64;
    });
}

// --------------------------------------------------------------------------
// 1. playerCanReadScroll gating — zero RNG on failure
// --------------------------------------------------------------------------

#[test]
fn player_can_read_scroll_false_when_blind_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(20, 20);
            setup_player(Coord_t { y: 10, x: 10 });
            pack_scroll(0, 0x1, TV_SCROLL1);
            with_state_mut(|s| s.py.flags.blind = 1);
        },
        || {
            let mut start = 0;
            let mut end = 0;
            assert!(!player_can_read_scroll(&mut start, &mut end));
        },
    );
}

#[test]
fn player_can_read_scroll_false_when_no_light_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(20, 20);
            setup_player(Coord_t { y: 10, x: 10 });
            pack_scroll(0, 0x1, TV_SCROLL1);
            with_state_mut(|s| {
                s.dg.floor[10][10].temporary_light = false;
                s.dg.floor[10][10].permanent_light = false;
            });
        },
        || {
            let mut start = 0;
            let mut end = 0;
            assert!(!player_can_read_scroll(&mut start, &mut end));
        },
    );
}

#[test]
fn player_can_read_scroll_false_when_confused_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(20, 20);
            setup_player(Coord_t { y: 10, x: 10 });
            pack_scroll(0, 0x1, TV_SCROLL1);
            with_state_mut(|s| s.py.flags.confused = 1);
        },
        || {
            let mut start = 0;
            let mut end = 0;
            assert!(!player_can_read_scroll(&mut start, &mut end));
        },
    );
}

#[test]
fn player_can_read_scroll_finds_scroll_range() {
    reset_for_new_game(Some(1));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    pack_scroll(0, 0x1, TV_SCROLL1);
    pack_scroll(1, 0x2, TV_SCROLL1);

    let mut start = 0;
    let mut end = 0;
    assert!(player_can_read_scroll(&mut start, &mut end));
    assert_eq!(start, 0);
    assert_eq!(end, 1);
}

// --------------------------------------------------------------------------
// 2. Zero-RNG scroll paths
// --------------------------------------------------------------------------

#[test]
fn scroll_remove_curse_no_rng_when_nothing_cursed() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(20, 20);
            setup_player(Coord_t { y: 10, x: 10 });
        },
        || assert!(!scroll_remove_curse()),
    );
}

#[test]
fn scroll_confuse_monster_second_use_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(20, 20);
            setup_player(Coord_t { y: 10, x: 10 });
            with_state_mut(|s| s.py.flags.confuse_monster = true);
        },
        || assert!(!scroll_confuse_monster()),
    );
}

#[test]
fn scroll_confuse_monster_first_use_zero_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(20, 20);
            setup_player(Coord_t { y: 10, x: 10 });
        },
        || {
            assert!(scroll_confuse_monster());
            assert!(with_state(|s| s.py.flags.confuse_monster));
        },
    );
}

#[test]
fn scroll_word_of_recall_second_use_no_extra_rng() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    scroll_word_of_recall();
    let first = with_state(|s| s.py.flags.word_of_recall);
    assert!(first > 0);

    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    with_state_mut(|s| s.py.flags.word_of_recall = first);
    scroll_word_of_recall();
    let baseline = random_number(100);

    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    with_state_mut(|s| s.py.flags.word_of_recall = first);
    scroll_word_of_recall();
    assert_eq!(random_number(100), baseline);
}

// --------------------------------------------------------------------------
// 3. RNG-order golden — enchant / curse / summon / teleport
// --------------------------------------------------------------------------

#[test]
fn scroll_enchant_weapon_to_hit_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    wield_item(TV_SWORD);

    let (m1, r1) = next_random_pair(10);
    assert!(scroll_enchant_weapon_to_hit());
    let (m2, r2) = next_random_pair(100);
    assert_eq!((m1, r1), (10, 2));
    assert_eq!((m2, r2), (100, 36));
    assert_eq!(
        with_state(|s| s.py.inventory[PlayerEquipment::Wield as usize].to_hit),
        1
    );
}

#[test]
fn scroll_enchant_weapon_to_damage_melee_limit_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    wield_item(TV_SWORD);

    let (m1, r1) = next_random_pair(12);
    assert!(scroll_enchant_weapon_to_damage());
    let (m2, r2) = next_random_pair(100);
    assert_eq!((m1, r1), (12, 2));
    assert_eq!((m2, r2), (100, 36));
    assert_eq!(
        with_state(|s| s.py.inventory[PlayerEquipment::Wield as usize].to_damage),
        1
    );
}

#[test]
fn scroll_enchant_weapon_rng_order_seed777() {
    reset_for_new_game(Some(777));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    wield_item(TV_SWORD);

    let pairs = [
        next_random_pair(2),
        next_random_pair(10),
        next_random_pair(2),
        next_random_pair(12),
    ];
    assert!(scroll_enchant_weapon());
    let after = next_random_pair(100);
    assert_eq!(pairs[0], (2, 1));
    assert_eq!(pairs[1], (10, 9));
    assert_eq!(pairs[2], (2, 2));
    assert_eq!(after, (100, 79));
}

#[test]
fn scroll_curse_weapon_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    wield_item(TV_SWORD);

    let pairs = [
        next_random_pair(5),
        next_random_pair(5),
        next_random_pair(5),
        next_random_pair(5),
    ];
    assert!(scroll_curse_weapon());
    let after = next_random_pair(100);
    assert_eq!(pairs[0], (5, 2));
    assert_eq!(pairs[1], (5, 3));
    assert_eq!(pairs[2], (5, 1));
    assert_eq!(pairs[3], (5, 2));
    assert_eq!(after, (100, 27));
    with_state(|s| {
        let item = &s.py.inventory[PlayerEquipment::Wield as usize];
        assert_eq!(item.to_hit, -6);
        assert_eq!(item.to_damage, -6);
        assert_eq!(item.to_ac, 0);
        assert!(inventory_item_is_cursed(*item));
    });
}

#[test]
fn scroll_teleport_level_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });

    let (m, r) = next_random_pair(2);
    scroll_teleport_level();
    let after = next_random_pair(100);
    assert_eq!((m, r), (2, 2));
    assert_eq!(after, (100, 36));
    with_state(|s| {
        assert_eq!(s.dg.current_level, 9);
        assert!(s.dg.generate_new_level);
    });
}

#[test]
fn scroll_summon_monster_rng_count_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(30, 30);
    setup_player(Coord_t { y: 15, x: 15 });
    init_monster_levels();

    let (m, r) = next_random_pair(3);
    let _ = scroll_summon_monster();
    assert_eq!((m, r), (3, 2));
}

#[test]
fn scroll_summon_undead_rng_count_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(30, 30);
    setup_player(Coord_t { y: 15, x: 15 });
    init_monster_levels();

    let (m, r) = next_random_pair(3);
    let _ = scroll_summon_undead();
    assert_eq!((m, r), (3, 2));
}

// --------------------------------------------------------------------------
// 4. Enchant/curse math — worn items, plusses clamping
// --------------------------------------------------------------------------

#[test]
fn scroll_enchant_item_to_ac_targets_worn_armor_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    wear_armor(PlayerEquipment::Body);

    let (m, r) = next_random_pair(6);
    assert!(scroll_enchant_item_to_ac());
    let (m2, r2) = next_random_pair(10);
    assert_eq!((m, r), (6, 2));
    assert_eq!((m2, r2), (10, 2));
    assert_eq!(
        with_state(|s| s.py.inventory[PlayerEquipment::Body as usize].to_ac),
        1
    );
}

#[test]
fn scroll_enchant_armor_multiple_rolls_seed777() {
    reset_for_new_game(Some(777));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    wear_armor(PlayerEquipment::Body);

    let (pick_max, pick) = next_random_pair(6);
    let (loop_max, loop_count) = next_random_pair(2);
    let loops = loop_count + 1;
    let mut pairs = Vec::new();
    for _ in 0..loops {
        pairs.push(next_random_pair(10));
    }
    assert!(scroll_enchant_armor());
    assert_eq!((pick_max, pick), (6, 5));
    assert_eq!((loop_max, loops), (2, loop_count + 1));
}

#[test]
fn scroll_curse_armor_applies_negative_ac_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    wear_armor(PlayerEquipment::Body);

    let pairs = [
        next_random_pair(4),
        next_random_pair(3),
        next_random_pair(3),
        next_random_pair(3),
        next_random_pair(3),
        next_random_pair(3),
        next_random_pair(5),
        next_random_pair(5),
    ];
    assert!(scroll_curse_armor());
    let after = next_random_pair(100);
    assert_eq!(pairs[0], (4, 2));
    assert_eq!(pairs[6], (5, 4));
    assert_eq!(pairs[7], (5, 2));
    assert_eq!(after, (100, 62));
    with_state(|s| {
        let item = &s.py.inventory[PlayerEquipment::Body as usize];
        assert_eq!(item.to_ac, -7);
        assert!(inventory_item_is_cursed(*item));
    });
}

#[test]
fn inventory_item_id_of_cursed_equipment_prefers_cursed_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    wear_armor(PlayerEquipment::Body);
    wear_armor(PlayerEquipment::Arm);
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Arm as usize].flags = TR_CURSED;
    });

    let (m, r) = next_random_pair(6);
    let id = inventory_item_id_of_cursed_equipment();
    let after = next_random_pair(100);
    assert_eq!((m, r), (6, 2));
    assert_eq!(id, PlayerEquipment::Arm as i32);
    assert_eq!(after, (100, 36));
}

#[test]
fn spell_enchant_item_zero_limit_never_calls_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(20, 20);
            setup_player(Coord_t { y: 10, x: 10 });
        },
        || {
            let mut plusses = 0i16;
            assert!(!spell_enchant_item(&mut plusses, 0));
            assert_eq!(plusses, 0);
        },
    );
}

// --------------------------------------------------------------------------
// 5. scrollRead dispatch — consumption, free turn, routing
// --------------------------------------------------------------------------

#[test]
fn scroll_read_consumes_scroll_and_identifies_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    pack_scroll(0, 0x0000_0400, TV_SCROLL1); // monster confusion

    test_push_getch_keys(&[b'a' as i32]);
    scroll_read();

    with_state(|s| {
        assert_eq!(s.py.pack.unique_items, 0);
        assert!(s.py.flags.confuse_monster);
        assert!(!s.game.player_free_turn);
    });
}

#[test]
fn scroll_read_escape_leaves_scroll_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(20, 20);
            setup_player(Coord_t { y: 10, x: 10 });
            pack_scroll(0, 0x1, TV_SCROLL1);
            test_push_getch_keys(&[i32::from(ESCAPE)]);
        },
        scroll_read,
    );
    assert_eq!(with_state(|s| s.py.pack.unique_items), 1);
}

#[test]
fn scroll_read_light_scroll_routes_to_spell_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(66, 66);
    setup_player(Coord_t { y: 15, x: 20 });
    pack_scroll(0, 0x0000_0020, TV_SCROLL1); // light

    test_push_getch_keys(&[b'a' as i32]);
    scroll_read();

    assert_eq!(with_state(|s| s.py.pack.unique_items), 0);
}

#[test]
fn scroll_read_scroll2_enchant_weapon_type_seed777() {
    reset_for_new_game(Some(777));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    wield_item(TV_SWORD);
    pack_scroll(0, 0x1, TV_SCROLL2); // *Enchant Weapon* => type 33

    test_push_getch_keys(&[b'a' as i32]);
    scroll_read();

    assert_eq!(with_state(|s| s.py.pack.unique_items), 0);
    assert!(with_state(|s| s.py.inventory
        [PlayerEquipment::Wield as usize]
        .to_hit
        > 0
        || s.py.inventory[PlayerEquipment::Wield as usize].to_damage
            > 0));
}

#[test]
fn scroll_read_teleport_level_updates_dungeon_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    pack_scroll(0, 0x0000_0200, TV_SCROLL1); // teleport level

    test_push_getch_keys(&[b'a' as i32]);
    scroll_read();

    with_state(|s| {
        assert_eq!(s.dg.current_level, 11);
        assert!(s.dg.generate_new_level);
        assert_eq!(s.py.pack.unique_items, 0);
    });
}

#[test]
fn scroll_enchant_weapon_to_hit_false_without_weapon_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(20, 20);
            setup_player(Coord_t { y: 10, x: 10 });
        },
        || assert!(!scroll_enchant_weapon_to_hit()),
    );
}
