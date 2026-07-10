//! Phase 4.4.6 — player_throw.cpp parity.
#![allow(clippy::int_plus_one)]

use umoria::config::dungeon::objects::OBJ_NOTHING;
use umoria::config::monsters::defense::CD_EVIL;
use umoria::config::player::status::PY_STR_WGT;
use umoria::config::treasure::flags::{TR_EGO_WEAPON, TR_SLAY_EVIL};
use umoria::data_creatures::CREATURES_LIST;
use umoria::data_player::CLASS_LEVEL_ADJ;
use umoria::dice::{dice_roll, Dice};
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, TILE_CORR_FLOOR, TILE_GRANITE_WALL, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::inventory::{
    inventory_item_copy_to, Inventory, PlayerEquipment, PLAYER_INVENTORY_SIZE,
};
use umoria::monster::{Monster, MON_TOTAL_ALLOCATIONS};
use umoria::player::{player_test_being_hit, PlayerAttr, PlayerClassLevelAdj};
use umoria::player_move::player_move_position;
use umoria::player_throw::{
    inventory_drop_or_throw_item, player_throw_item, weapon_missile_facts,
};
use umoria::treasure::{TV_ARROW, TV_BOW, TV_SPIKE};
use umoria::types::{Coord_t, MESSAGE_HISTORY_SIZE};
use umoria::ui::panel_bounds_fields;
use umoria::ui_io::{test_push_getch_keys, test_set_direction, test_set_ncurses_stub, ESCAPE};

const POS: Coord_t = Coord_t { y: 10, x: 10 };
const STREET_URCHIN_ID: u16 = 0;
const MON_SLOT: i32 = 2;

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
        s.py.misc.level = 10;
        s.py.misc.class_id = 0;
        s.py.misc.bth_with_bows = 20;
        s.py.misc.plusses_to_hit = 5;
        s.py.stats.used[PlayerAttr::A_STR as usize] = 18;
        s.py.flags.blind = 0;
        s.py.flags.confused = 0;
        s.py.pack.weight = 0;
        s.py.pack.unique_items = 0;
        s.dg.panel.row = 0;
        s.dg.panel.col = 0;
        s.dg.panel.top = bounds.top;
        s.dg.panel.bottom = bounds.bottom;
        s.dg.panel.left = bounds.left;
        s.dg.panel.right = bounds.right;
        s.dg.panel.row_prt = bounds.row_prt;
        s.dg.panel.col_prt = bounds.col_prt;
        for i in 0..PLAYER_INVENTORY_SIZE as usize {
            inventory_item_copy_to(OBJ_NOTHING as i16, &mut s.py.inventory[i]);
        }
        s.next_free_monster_id =
            i16::from(umoria::config::monsters::MON_MIN_INDEX_ID) + 2;
        s.monsters = [Monster::default(); MON_TOTAL_ALLOCATIONS as usize];
    });
}

fn set_tile(coord: Coord_t, tile: Tile) {
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize] = tile;
    });
}

fn tile_at(coord: Coord_t) -> Tile {
    with_state(|s| s.dg.floor[coord.y as usize][coord.x as usize])
}

fn fill_corridor(start: Coord_t, direction: i32, length: i32) {
    let mut coord = start;
    for _ in 0..length {
        set_tile(
            coord,
            Tile {
                feature_id: TILE_CORR_FLOOR,
                temporary_light: true,
                ..Tile::default()
            },
        );
        let _ = player_move_position(direction, &mut coord);
    }
}

fn pack_item(slot: i32, item: Inventory) {
    with_state_mut(|s| {
        s.py.inventory[slot as usize] = item;
        if slot >= s.py.pack.unique_items as i32 {
            s.py.pack.unique_items = (slot + 1) as i16;
        }
    });
}

fn throwable_item(weight: u16, damage: Dice, to_damage: i16) -> Inventory {
    Inventory {
        category_id: TV_SPIKE,
        sub_category_id: 64,
        sprite: b'|',
        items_count: 1,
        weight,
        to_damage,
        damage,
        ..Inventory::default()
    }
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

fn reference_flight_path(start: Coord_t, direction: i32, max_distance: i32) -> Vec<Coord_t> {
    let mut coord = start;
    let mut current_distance = 0;
    let mut path = Vec::new();
    let mut stopped = false;

    while !stopped {
        let _ = player_move_position(direction, &mut coord);
        current_distance += 1;

        if current_distance > max_distance {
            stopped = true;
        }

        let tile = tile_at(coord);
        if tile.feature_id <= umoria::dungeon_tile::MAX_OPEN_SPACE && !stopped {
            if tile.creature_id > 1 {
                stopped = true;
            } else {
                path.push(coord);
            }
        } else {
            stopped = true;
        }
    }

    path
}

// ---------------------------------------------------------------------------
// 1. Selection / cancel — zero RNG
// ---------------------------------------------------------------------------

#[test]
fn player_throw_empty_pack_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(40, 40);
            setup_player_panel(POS);
        },
        player_throw_item,
    );
    assert_eq!(last_message_text(), "But you are not carrying anything.");
    assert!(with_state(|s| s.game.player_free_turn));
}

#[test]
fn player_throw_escape_item_selection_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(40, 40);
            setup_player_panel(POS);
            pack_item(0, throwable_item(10, Dice { dice: 1, sides: 6 }, 0));
            test_push_getch_keys(&[i32::from(ESCAPE)]);
        },
        player_throw_item,
    );
    assert!(with_state(|s| s.game.player_free_turn));
    assert_eq!(with_state(|s| s.py.pack.unique_items), 1);
}

#[test]
fn player_throw_escape_direction_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(40, 40);
            setup_player_panel(POS);
            pack_item(0, throwable_item(10, Dice { dice: 1, sides: 6 }, 0));
            test_push_getch_keys(&[b'a' as i32, i32::from(ESCAPE)]);
        },
        player_throw_item,
    );
    assert_eq!(with_state(|s| s.py.pack.unique_items), 1);
}

// ---------------------------------------------------------------------------
// 2. RNG-order golden — damage, break gate, scatter
// ---------------------------------------------------------------------------

#[test]
fn player_throw_wall_hit_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player_panel(POS);
    fill_corridor(POS, 6, 3);
    set_tile(
        Coord_t { y: 10, x: 14 },
        Tile {
            feature_id: TILE_GRANITE_WALL,
            ..Tile::default()
        },
    );
    pack_item(
        0,
        throwable_item(10, Dice { dice: 1, sides: 6 }, 2),
    );
    test_push_getch_keys(&[b'a' as i32]);
    test_set_direction(Some(6));
    player_throw_item();
    let post = random_number(100);

    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player_panel(POS);
    fill_corridor(POS, 6, 3);
    set_tile(
        Coord_t { y: 10, x: 14 },
        Tile {
            feature_id: TILE_GRANITE_WALL,
            ..Tile::default()
        },
    );
    pack_item(
        0,
        throwable_item(10, Dice { dice: 1, sides: 6 }, 2),
    );
    // weaponMissileFacts: diceRoll(1d6); landing: randomNumber(10) break gate only (tile valid).
    random_number(6);
    random_number(10);
    let expected_post = random_number(100);
    assert_eq!(post, expected_post);
}

#[test]
fn inventory_drop_or_throw_item_break_gate_only_on_landing_branch() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player_panel(POS);
    let item = throwable_item(10, Dice { dice: 1, sides: 6 }, 0);
    inventory_drop_or_throw_item(POS, item);
    let post_fail_probe = random_number(100);

    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player_panel(POS);
    random_number(10);
    let expected_post_fail_probe = random_number(100);
    assert_eq!(post_fail_probe, expected_post_fail_probe);

    // Block the landing tile so scatter rolls fire after break gate passes.
    reset_for_new_game(Some(43));
    setup_dungeon(40, 40);
    setup_player_panel(POS);
    set_tile(
        POS,
        Tile {
            feature_id: TILE_CORR_FLOOR,
            treasure_id: 1,
            ..Tile::default()
        },
    );
    let item = throwable_item(10, Dice { dice: 1, sides: 6 }, 0);
    inventory_drop_or_throw_item(POS, item);
    let post_scatter = random_number(100);

    reset_for_new_game(Some(43));
    setup_dungeon(40, 40);
    setup_player_panel(POS);
    set_tile(
        POS,
        Tile {
            feature_id: TILE_CORR_FLOOR,
            treasure_id: 1,
            ..Tile::default()
        },
    );
    random_number(10);
    random_number(3);
    random_number(3);
    let expected_post_scatter = random_number(100);
    assert_eq!(post_scatter, expected_post_scatter);
}

// ---------------------------------------------------------------------------
// 3. Flight-path parity
// ---------------------------------------------------------------------------

#[test]
fn player_throw_flight_path_matches_reference_corridor() {
    reset_for_new_game(Some(1));
    setup_dungeon(40, 40);
    setup_player_panel(POS);
    fill_corridor(POS, 6, 8);
    set_tile(
        Coord_t { y: 10, x: 19 },
        Tile {
            feature_id: TILE_GRANITE_WALL,
            ..Tile::default()
        },
    );
    pack_item(0, throwable_item(10, Dice { dice: 1, sides: 4 }, 0));
    test_push_getch_keys(&[b'a' as i32]);
    test_set_direction(Some(6));

    let expected = reference_flight_path(POS, 6, 10);
    player_throw_item();

    assert_eq!(expected.len(), 8);
    assert_eq!(expected.last().copied(), Some(Coord_t { y: 10, x: 18 }));
}

#[test]
fn player_throw_stops_at_wall_and_drops_on_previous_tile() {
    reset_for_new_game(Some(1));
    setup_dungeon(40, 40);
    setup_player_panel(POS);
    fill_corridor(POS, 6, 2);
    set_tile(
        Coord_t { y: 10, x: 13 },
        Tile {
            feature_id: TILE_GRANITE_WALL,
            ..Tile::default()
        },
    );
    pack_item(0, throwable_item(10, Dice { dice: 1, sides: 4 }, 0));
    test_push_getch_keys(&[b'a' as i32]);
    test_set_direction(Some(6));

    player_throw_item();

    assert_ne!(tile_at(Coord_t { y: 10, x: 12 }).treasure_id, 0);
}

#[test]
fn player_throw_stops_at_max_range() {
    reset_for_new_game(Some(1));
    setup_dungeon(40, 40);
    setup_player_panel(POS);
    fill_corridor(POS, 6, 20);
    pack_item(0, throwable_item(10, Dice { dice: 1, sides: 4 }, 0));
    test_push_getch_keys(&[b'a' as i32]);
    test_set_direction(Some(6));

    player_throw_item();

    assert_ne!(tile_at(Coord_t { y: 10, x: 20 }).treasure_id, 0);
    assert_eq!(tile_at(Coord_t { y: 10, x: 21 }).treasure_id, 0);
}

// ---------------------------------------------------------------------------
// 4. Hit + damage math
// ---------------------------------------------------------------------------

fn place_monster(id: i32, creature_id: u16, hp: i16, coord: Coord_t, lit: bool) {
    with_state_mut(|s| {
        s.monsters[id as usize] = Monster {
            hp,
            creature_id,
            pos: coord,
            lit,
            ..Default::default()
        };
        s.dg.floor[coord.y as usize][coord.x as usize].creature_id = id as u8;
    });
}

#[test]
fn player_throw_hit_applies_ego_slay_multiplier_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player_panel(POS);
    fill_corridor(POS, 6, 1);
    place_monster(MON_SLOT, STREET_URCHIN_ID, 500, Coord_t { y: 10, x: 11 }, true);

    with_state_mut(|s| {
        s.py.misc.bth_with_bows = 100;
        s.py.misc.plusses_to_hit = 50;
    });

    pack_item(
        0,
        Inventory {
            category_id: TV_ARROW,
            sub_category_id: 64,
            sprite: b'|',
            items_count: 1,
            weight: 10,
            to_damage: 0,
            damage: Dice { dice: 1, sides: 1 },
            flags: TR_EGO_WEAPON | TR_SLAY_EVIL,
            ..Inventory::default()
        },
    );
    test_push_getch_keys(&[b'a' as i32]);
    test_set_direction(Some(6));

    player_throw_item();

    assert!(last_message_text().contains("hits the"));
    with_state(|s| {
        assert!(s.monsters[MON_SLOT as usize].hp < 500);
        assert_eq!(
            s.creature_recall[STREET_URCHIN_ID as usize].defenses & CD_EVIL,
            CD_EVIL
        );
    });
}

#[test]
fn player_throw_miss_drops_item_without_extra_hit_rng() {
    reset_for_new_game(Some(777));
    setup_dungeon(40, 40);
    setup_player_panel(POS);
    fill_corridor(POS, 6, 1);
    place_monster(MON_SLOT, STREET_URCHIN_ID, 500, Coord_t { y: 10, x: 11 }, true);

    with_state_mut(|s| {
        s.py.misc.bth_with_bows = 0;
        s.py.misc.plusses_to_hit = -50;
        s.py.misc.level = 1;
    });

    pack_item(0, throwable_item(10, Dice { dice: 1, sides: 1 }, 0));
    test_push_getch_keys(&[b'a' as i32]);
    test_set_direction(Some(6));

    player_throw_item();

    assert_ne!(tile_at(POS).treasure_id, 0);
    assert_eq!(with_state(|s| s.monsters[MON_SLOT as usize].hp), 500);
}

// ---------------------------------------------------------------------------
// 5. Item disposition & inventory
// ---------------------------------------------------------------------------

#[test]
fn player_throw_stack_decrements_one_and_updates_weight() {
    reset_for_new_game(Some(1));
    setup_dungeon(40, 40);
    setup_player_panel(POS);
    fill_corridor(POS, 6, 1);
    set_tile(
        Coord_t { y: 10, x: 12 },
        Tile {
            feature_id: TILE_GRANITE_WALL,
            ..Tile::default()
        },
    );

    let mut item = throwable_item(10, Dice { dice: 1, sides: 4 }, 0);
    item.items_count = 3;
    pack_item(0, item);
    with_state_mut(|s| s.py.pack.weight = 30);

    test_push_getch_keys(&[b'a' as i32]);
    test_set_direction(Some(6));
    player_throw_item();

    with_state(|s| {
        assert_eq!(s.py.inventory[0].items_count, 2);
        assert_eq!(s.py.pack.weight, 20);
        assert_ne!(s.py.flags.status & PY_STR_WGT, 0);
        assert_eq!(s.py.pack.unique_items, 1);
    });
}

#[test]
fn player_throw_single_item_removed_from_pack() {
    reset_for_new_game(Some(1));
    setup_dungeon(40, 40);
    setup_player_panel(POS);
    fill_corridor(POS, 6, 1);
    set_tile(
        Coord_t { y: 10, x: 12 },
        Tile {
            feature_id: TILE_GRANITE_WALL,
            ..Tile::default()
        },
    );
    pack_item(0, throwable_item(10, Dice { dice: 1, sides: 4 }, 0));
    test_push_getch_keys(&[b'a' as i32]);
    test_set_direction(Some(6));

    player_throw_item();

    assert_eq!(with_state(|s| s.py.pack.unique_items), 0);
}

// ---------------------------------------------------------------------------
// 6. weaponMissileFacts — bow/arrow combos
// ---------------------------------------------------------------------------

#[test]
fn weapon_missile_facts_short_bow_arrow_doubles_damage_and_extends_range() {
    reset_for_new_game(Some(1));
    setup_player_panel(POS);
    with_state_mut(|s| {
        s.py.misc.bth_with_bows = 40;
        s.py.misc.plusses_to_hit = 3;
        s.py.stats.used[PlayerAttr::A_STR as usize] = 18;
        s.py.inventory[PlayerEquipment::Wield as usize] = Inventory {
            category_id: TV_BOW,
            misc_use: 2,
            to_hit: 4,
            to_damage: 6,
            ..Inventory::default()
        };
    });

    let arrow = Inventory {
        category_id: TV_ARROW,
        weight: 5,
        to_hit: 2,
        to_damage: 1,
        damage: Dice { dice: 1, sides: 6 },
        ..Inventory::default()
    };

    let mut bth = 0;
    let mut pth = 0;
    let mut dam = 0;
    let mut dis = 0;
    weapon_missile_facts(arrow, &mut bth, &mut pth, &mut dam, &mut dis);

    assert_eq!(bth, 40);
    assert_eq!(pth, 3 + 2 - 4 + 2 * 4);
    assert_eq!(dis, 25);

    let dice_only = dice_roll(Dice { dice: 1, sides: 6 });
    reset_for_new_game(Some(1));
    setup_player_panel(POS);
    with_state_mut(|s| {
        s.py.misc.bth_with_bows = 40;
        s.py.misc.plusses_to_hit = 3;
        s.py.stats.used[PlayerAttr::A_STR as usize] = 18;
        s.py.inventory[PlayerEquipment::Wield as usize] = Inventory {
            category_id: TV_BOW,
            misc_use: 2,
            to_hit: 4,
            to_damage: 6,
            ..Inventory::default()
        };
    });
    let mut bth2 = 0;
    let mut pth2 = 0;
    let mut dam2 = 0;
    let mut dis2 = 0;
    weapon_missile_facts(arrow, &mut bth2, &mut pth2, &mut dam2, &mut dis2);
    assert_eq!(dam2, (dice_only + 1 + 6) * 2);
}

#[test]
fn weapon_missile_facts_plain_throw_caps_distance_at_ten() {
    reset_for_new_game(Some(1));
    setup_player_panel(POS);
    with_state_mut(|s| {
        s.py.stats.used[PlayerAttr::A_STR as usize] = 18;
    });
    let item = throwable_item(1, Dice { dice: 1, sides: 4 }, 0);
    let mut bth = 0;
    let mut pth = 0;
    let mut dam = 0;
    let mut dis = 0;
    weapon_missile_facts(item, &mut bth, &mut pth, &mut dam, &mut dis);
    assert_eq!(dis, 10);
}

// ---------------------------------------------------------------------------
// 7. Hit formula integration — unlit monster penalty branch
// ---------------------------------------------------------------------------

#[test]
fn player_throw_unlit_monster_applies_distance_and_bonus_penalties() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40);
    setup_player_panel(POS);
    fill_corridor(POS, 6, 1);
    place_monster(
        MON_SLOT,
        STREET_URCHIN_ID,
        500,
        Coord_t { y: 10, x: 11 },
        false,
    );

    let item = throwable_item(10, Dice { dice: 1, sides: 6 }, 0);
    let mut tbth = 0;
    let mut tpth = 0;
    let mut tdam = 0;
    let mut tdis = 0;
    weapon_missile_facts(item, &mut tbth, &mut tpth, &mut tdam, &mut tdis);
    let current_distance = 1;
    tbth -= current_distance;
    tbth /= current_distance + 2;
    tbth -= with_state(|s| {
        i32::from(s.py.misc.level)
            * i32::from(
                CLASS_LEVEL_ADJ[s.py.misc.class_id as usize][PlayerClassLevelAdj::BTHB as usize],
            )
            / 2
    });
    tbth -= tpth * (i32::from(umoria::player::BTH_PER_PLUS_TO_HIT_ADJUST) - 1);

    let ac = i32::from(CREATURES_LIST[STREET_URCHIN_ID as usize].ac);
    let _hit = player_test_being_hit(
        tbth,
        10,
        tpth,
        ac,
        PlayerClassLevelAdj::BTHB as u8,
    );
}
