//! Staff & wand use parity (`staves`).
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
use umoria::config::identification::{ID_EMPTY, ID_KNOWN2};
use umoria::config::monsters::MON_ENDGAME_MONSTERS;
use umoria::data_creatures::CREATURES_LIST;
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_tile::{Tile, TILE_LIGHT_FLOOR};
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::identification::spell_item_identified;
use umoria::inventory::{inventory_item_copy_to, Inventory, PLAYER_INVENTORY_SIZE};
use umoria::monster::{MON_MAX_CREATURES, MON_MAX_LEVELS};
use umoria::player::PlayerAttr;
use umoria::staves::{staff_use, wand_aim};
use umoria::treasure::{TV_SCROLL1, TV_STAFF, TV_WAND};
use umoria::types::{Coord_t, MESSAGE_HISTORY_SIZE};
use umoria::ui::panel_bounds_fields;
use umoria::ui_io::{test_push_getch_keys, test_set_direction, test_set_ncurses_stub, ESCAPE};

const MAGE_CLASS_ID: u8 = 2;
const POS: Coord_t = Coord_t { y: 10, x: 10 };

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

fn init_monster_levels(state: &mut umoria::game::State) {
    state.monster_levels = [0; MON_MAX_LEVELS as usize + 1];
    let endgame = MON_ENDGAME_MONSTERS as usize;
    for i in 0..MON_MAX_CREATURES as usize - endgame {
        let level = CREATURES_LIST[i].level as usize;
        state.monster_levels[level] += 1;
    }
    for i in 1..=MON_MAX_LEVELS as usize {
        state.monster_levels[i] += state.monster_levels[i - 1];
    }
}

fn setup_dungeon(height: i16, width: i16, pos: Coord_t) {
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Tile::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                s.dg.floor[y as usize][x as usize].feature_id = TILE_LIGHT_FLOOR;
            }
        }
        s.dg.floor[pos.y as usize][pos.x as usize].temporary_light = true;
        s.game.treasure.current_id = 1;
    });
}

fn setup_player(level: u16, int_stat: u8, saving_throw: i16) {
    test_set_ncurses_stub(true);
    let bounds = panel_bounds_fields(0, 0);
    with_state_mut(|s| {
        s.py.pos = POS;
        s.py.misc.class_id = MAGE_CLASS_ID;
        s.py.misc.level = level;
        s.py.misc.saving_throw = saving_throw;
        s.py.stats.used[PlayerAttr::A_INT as usize] = int_stat;
        s.py.flags.blind = 0;
        s.py.flags.confused = 0;
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
        init_monster_levels(s);
    });
}

fn pack_device(slot: i32, category_id: u8, flags: u32, misc_use: i16, depth: u8) {
    with_state_mut(|s| {
        s.py.inventory[slot as usize] = Inventory {
            category_id,
            sub_category_id: 64,
            flags,
            misc_use,
            depth_first_found: depth,
            items_count: 1,
            weight: 10,
            ..Inventory::default()
        };
        if slot >= s.py.pack.unique_items as i32 {
            s.py.pack.unique_items = (slot + 1) as i16;
        }
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

// --------------------------------------------------------------------------
// 1. Empty / cancel paths — zero effect RNG
// --------------------------------------------------------------------------

#[test]
fn staff_use_empty_pack_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(40, 40, POS);
            setup_player(10, 18, 18);
        },
        staff_use,
    );
    assert_eq!(last_message_text(), "But you are not carrying anything.");
}

#[test]
fn staff_use_no_staff_in_pack_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(40, 40, POS);
            setup_player(10, 18, 18);
            pack_device(0, TV_SCROLL1, 1, 1, 5);
        },
        staff_use,
    );
    assert_eq!(last_message_text(), "You are not carrying any staffs.");
}

#[test]
fn staff_use_escape_before_use_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(40, 40, POS);
            setup_player(10, 18, 18);
            pack_device(0, TV_STAFF, 0x1, 5, 5);
            test_push_getch_keys(&[i32::from(ESCAPE)]);
        },
        staff_use,
    );
    assert!(with_state(|s| s.game.player_free_turn));
    assert_eq!(with_state(|s| s.py.inventory[0].misc_use), 5);
}

#[test]
fn wand_aim_empty_pack_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(40, 40, POS);
            setup_player(10, 18, 18);
        },
        wand_aim,
    );
}

#[test]
fn wand_aim_no_wand_consumes_no_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(40, 40, POS);
            setup_player(10, 18, 18);
            pack_device(0, TV_STAFF, 0x1, 5, 5);
        },
        wand_aim,
    );
}

#[test]
fn wand_aim_escape_on_direction_consumes_no_device_rng() {
    assert_rng_unchanged_after(
        || {
            setup_dungeon(40, 40, POS);
            setup_player(10, 18, 18);
            pack_device(0, TV_WAND, 0x1, 5, 5);
            test_push_getch_keys(&[b'a' as i32, i32::from(ESCAPE)]);
        },
        wand_aim,
    );
    assert_eq!(with_state(|s| s.py.inventory[0].misc_use), 5);
}

// --------------------------------------------------------------------------
// 2. Charge handling
// --------------------------------------------------------------------------

#[test]
fn staff_use_no_charges_marks_empty_without_discharge_rng() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40, POS);
    setup_player(40, 18, 100);
    pack_device(0, TV_STAFF, 0x1, 0, 5);
    test_push_getch_keys(&[b'a' as i32]);

    staff_use();

    assert_eq!(last_message_text(), "The staff has no charges left.");
    assert_eq!(with_state(|s| s.py.inventory[0].misc_use), 0);
    with_state(|s| assert_ne!(s.py.inventory[0].identification & ID_EMPTY, 0));
}

#[test]
fn wand_aim_no_charges_marks_empty() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40, POS);
    setup_player(40, 18, 100);
    pack_device(0, TV_WAND, 0x1, 0, 5);
    test_push_getch_keys(&[b'a' as i32]);
    test_set_direction(Some(2));

    wand_aim();

    assert_eq!(last_message_text(), "The wand has no charges left.");
    assert_eq!(with_state(|s| s.py.inventory[0].misc_use), 0);
    with_state(|s| assert_ne!(s.py.inventory[0].identification & ID_EMPTY, 0));
}

#[test]
fn staff_use_light_staff_decrements_charge() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40, POS);
    setup_player(40, 18, 100);
    pack_device(0, TV_STAFF, 0x1, 3, 0);
    test_push_getch_keys(&[b'a' as i32]);

    staff_use();

    assert_eq!(with_state(|s| s.py.inventory[0].misc_use), 2);
    assert!(!with_state(|s| s.game.player_free_turn));
}

#[test]
fn wand_aim_light_wand_decrements_charge() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40, POS);
    setup_player(40, 18, 100);
    pack_device(0, TV_WAND, 0x1, 4, 0);
    test_push_getch_keys(&[b'a' as i32]);
    test_set_direction(Some(2));

    wand_aim();

    assert_eq!(with_state(|s| s.py.inventory[0].misc_use), 3);
}

// --------------------------------------------------------------------------
// 3. Device failure RNG order
// --------------------------------------------------------------------------

#[test]
fn staff_use_device_fail_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40, POS);
    setup_player(1, 7, -8);
    pack_device(0, TV_STAFF, 0x1, 5, 50);
    test_push_getch_keys(&[b'a' as i32]);

    staff_use();

    assert_eq!(last_message_text(), "You failed to use the staff properly.");
    assert_eq!(with_state(|s| s.py.inventory[0].misc_use), 5);
    assert_eq!(next_random_pair(100), (100, 36));
}

#[test]
fn wand_aim_device_fail_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40, POS);
    setup_player(1, 7, -8);
    pack_device(0, TV_WAND, 0x1, 5, 50);
    test_push_getch_keys(&[b'a' as i32]);
    test_set_direction(Some(2));

    wand_aim();

    assert_eq!(last_message_text(), "You failed to use the wand properly.");
    assert_eq!(with_state(|s| s.py.inventory[0].misc_use), 5);
    assert_eq!(next_random_pair(100), (100, 36));
}

// --------------------------------------------------------------------------
// 4. Successful light staff/wand — RNG order after device rolls
// --------------------------------------------------------------------------

#[test]
fn staff_use_light_staff_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40, POS);
    setup_player(40, 18, 100);
    pack_device(0, TV_STAFF, 0x1, 5, 0);
    test_push_getch_keys(&[b'a' as i32]);

    staff_use();

    assert_eq!(with_state(|s| s.py.inventory[0].misc_use), 4);
    assert_eq!(next_random_pair(100), (100, 73));
}

#[test]
fn wand_aim_light_wand_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40, POS);
    setup_player(40, 18, 100);
    pack_device(0, TV_WAND, 0x1, 5, 0);
    test_push_getch_keys(&[b'a' as i32]);
    test_set_direction(Some(2));

    wand_aim();

    assert_eq!(with_state(|s| s.py.inventory[0].misc_use), 4);
    assert_eq!(next_random_pair(100), (100, 73));
}

// --------------------------------------------------------------------------
// 5. Summoning staff consumes randomNumber(4) loop RNG
// --------------------------------------------------------------------------

#[test]
fn staff_use_summoning_rng_order_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40, POS);
    setup_player(40, 18, 100);
    pack_device(0, TV_STAFF, 0x80, 5, 0);
    test_push_getch_keys(&[b'a' as i32]);

    staff_use();

    assert_eq!(with_state(|s| s.py.inventory[0].misc_use), 4);
    assert_eq!(next_random_pair(4), (4, 2));
}

// --------------------------------------------------------------------------
// 6. Confused wand aim rerolls direction
// --------------------------------------------------------------------------

#[test]
fn wand_aim_confused_consumes_random_direction_rng() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40, POS);
    setup_player(40, 18, 100);
    with_state_mut(|s| s.py.flags.confused = 5);
    pack_device(0, TV_WAND, 0x1, 5, 0);
    test_push_getch_keys(&[b'a' as i32]);
    test_set_direction(Some(2));

    wand_aim();

    assert!(last_message_text().contains("You are confused."));
    assert_eq!(with_state(|s| s.py.inventory[0].misc_use), 4);
    assert_eq!(next_random_pair(9), (9, 4));
}

// --------------------------------------------------------------------------
// 7. Identification / tried state
// --------------------------------------------------------------------------

#[test]
fn staff_use_unidentified_summoning_does_not_set_identified() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40, POS);
    setup_player(40, 18, 100);
    pack_device(0, TV_STAFF, 0x80, 5, 10);
    test_push_getch_keys(&[b'a' as i32]);

    staff_use();

    assert!(!spell_item_identified(with_state(|s| s.py.inventory[0])));
}

#[test]
fn staff_use_identified_light_staff_stays_identified() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40, POS);
    setup_player(40, 18, 100);
    pack_device(0, TV_STAFF, 0x1, 5, 0);
    with_state_mut(|s| s.py.inventory[0].identification |= ID_KNOWN2);
    test_push_getch_keys(&[b'a' as i32]);

    staff_use();

    assert!(spell_item_identified(with_state(|s| s.py.inventory[0])));
}

// --------------------------------------------------------------------------
// 8. misc_use i16 decrement semantics
// --------------------------------------------------------------------------

#[test]
fn staff_use_misc_use_i16_decrement_from_one() {
    reset_for_new_game(Some(42));
    setup_dungeon(40, 40, POS);
    setup_player(40, 18, 100);
    pack_device(0, TV_STAFF, 0x1, 1, 0);
    test_push_getch_keys(&[b'a' as i32]);

    staff_use();

    assert_eq!(with_state(|s| s.py.inventory[0].misc_use), 0);
}
