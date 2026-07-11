//! Misc actions, level transitions & `playDungeon` loop.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::config::identification::ID_MAGIK;
use umoria::config::player::status::{PY_SEARCH, PY_STR_WGT, PY_STUDY};
use umoria::config::player::PLAYER_REGEN_HPBASE;
use umoria::config::treasure::OBJECT_LAMP_MAX_CAPACITY;
use umoria::data_creatures::CREATURES_LIST;
use umoria::data_player::MAGIC_SPELLS;
use umoria::dungeon_tile::TILE_LIGHT_FLOOR;
use umoria::game::{
    random_number, reset_for_new_game, test_set_direction, with_state, with_state_mut,
};
use umoria::game_objects::popt;
use umoria::game_run::{
    dungeon_go_down_level, dungeon_go_up_level, dungeon_jam_door, examine_book,
    inventory_refill_lamp, item_enchanted, play_dungeon, player_regenerate_hit_points,
    player_regenerate_mana, test_play_dungeon_trace, test_reset_game_run_hooks,
    test_reset_play_dungeon_trace, test_set_play_dungeon_max_turns,
    test_set_skip_input_command_loop, PlayDungeonTrace,
};
use umoria::helpers::get_and_clear_first_bit;
use umoria::inventory::{Inventory, PlayerEquipment};
use umoria::monster::{Monster, MON_TOTAL_ALLOCATIONS};
use umoria::treasure::{
    TV_CLOSED_DOOR, TV_DOWN_STAIR, TV_FLASK, TV_MAGIC_BOOK, TV_MAX_ENCHANT, TV_MIN_ENCHANT,
    TV_OPEN_DOOR, TV_PRAYER_BOOK, TV_SPIKE, TV_SWORD, TV_UP_STAIR,
};
use umoria::types::Coord_t;
use umoria::ui_io::{
    register_game_ui_hooks, test_clear_getch_keys, test_push_getch_keys, test_set_ncurses_stub,
    test_set_select_ready, test_set_ui_capture, test_ui_messages_peek,
};

const MAGE_CLASS_ID: u8 = 1;
const ORC_ID: u16 = 77;
const SHRT_MAX: i16 = 32_767;

fn setup() {
    test_set_ncurses_stub(true);
    register_game_ui_hooks();
    test_set_ui_capture(true);
    test_reset_game_run_hooks();
    test_reset_play_dungeon_trace();
    test_set_skip_input_command_loop(true);
    test_set_select_ready(None);
    test_clear_getch_keys();
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.dg.height = 20;
        s.dg.width = 20;
        for y in 1..19 {
            for x in 1..19 {
                s.dg.floor[y][x].feature_id = TILE_LIGHT_FLOOR;
            }
        }
        s.game.treasure.current_id = 1;
    });
}

fn peek_ui_messages() -> Vec<String> {
    test_ui_messages_peek()
}

fn messages_contain(needle: &str) -> bool {
    peek_ui_messages().iter().any(|m| m.contains(needle))
}

fn play_one_dungeon_turn() {
    test_set_play_dungeon_max_turns(1);
    play_dungeon();
}

fn place_treasure(coord: Coord_t, category_id: u8) -> u8 {
    let treasure_id = popt();
    with_state_mut(|s| {
        s.dg.floor[coord.y as usize][coord.x as usize].treasure_id = treasure_id as u8;
        s.game.treasure.list[treasure_id as usize].category_id = category_id;
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

fn jam_door_misc_use_after(start: i16) -> i16 {
    let mut misc_use = start;
    if misc_use > 0 {
        misc_use = -misc_use;
    }
    misc_use -= 1 + 190 / (10 - misc_use);
    misc_use
}

// --------------------------------------------------------------------------
// 1. playerRegenerateHitPoints
// --------------------------------------------------------------------------

#[test]
fn regenerate_hp_fixed_point_carry_overflow_and_clamp() {
    setup();
    with_state_mut(|s| {
        s.py.misc.max_hp = 100;
        s.py.misc.current_hp = 99;
        s.py.misc.current_hp_fraction = 0xFFFF;
    });

    let percent = 0x0001_0000;
    player_regenerate_hit_points(percent);
    assert_eq!(with_state(|s| s.py.misc.current_hp), 100);
    assert_eq!(with_state(|s| s.py.misc.current_hp_fraction), 0);

    setup();
    with_state_mut(|s| {
        s.py.misc.max_hp = 100;
        s.py.misc.current_hp = 100;
        s.py.misc.current_hp_fraction = 0x8000;
    });
    player_regenerate_hit_points(percent);
    assert_eq!(with_state(|s| s.py.misc.current_hp), 100);
    assert_eq!(with_state(|s| s.py.misc.current_hp_fraction), 0);

    setup();
    with_state_mut(|s| {
        s.py.misc.max_hp = 10;
        s.py.misc.current_hp = 5;
        s.py.misc.current_hp_fraction = 0;
    });
    let max_hp = i32::from(with_state(|s| s.py.misc.max_hp));
    let new_chp = max_hp * (-0x0002_0000) + i32::from(PLAYER_REGEN_HPBASE);
    let old = with_state(|s| s.py.misc.current_hp);
    player_regenerate_hit_points(-0x0002_0000);
    let current = with_state(|s| s.py.misc.current_hp);
    if (new_chp >> 16) < 0 && current < 0 && old > 0 {
        assert_eq!(current, SHRT_MAX);
    }
}

// --------------------------------------------------------------------------
// 2. playerRegenerateMana
// --------------------------------------------------------------------------

#[test]
fn regenerate_mana_fixed_point_boundaries() {
    setup();
    with_state_mut(|s| {
        s.py.misc.mana = 50;
        s.py.misc.current_mana = 49;
        s.py.misc.current_mana_fraction = 0xFFFF;
    });
    player_regenerate_mana(0x0001_0000);
    assert_eq!(with_state(|s| s.py.misc.current_mana), 50);
    assert_eq!(with_state(|s| s.py.misc.current_mana_fraction), 0);

    setup();
    with_state_mut(|s| {
        s.py.misc.mana = 50;
        s.py.misc.current_mana = 50;
        s.py.misc.current_mana_fraction = 1;
    });
    player_regenerate_mana(0x0001_0000);
    assert_eq!(with_state(|s| s.py.misc.current_mana), 50);
    assert_eq!(with_state(|s| s.py.misc.current_mana_fraction), 0);
}

// --------------------------------------------------------------------------
// 3. itemEnchanted
// --------------------------------------------------------------------------

#[test]
fn item_enchanted_full_truth_table() {
    let mut item = Inventory::default();
    assert!(!item_enchanted(item));

    item.category_id = TV_MIN_ENCHANT.wrapping_sub(1);
    assert!(!item_enchanted(item));

    item.category_id = TV_MAX_ENCHANT + 1;
    assert!(!item_enchanted(item));

    item.category_id = TV_SWORD;
    item.flags = 0x0000_0008; // TR_CURSED
    assert!(!item_enchanted(item));

    item.flags = 0;
    item.to_hit = 1;
    assert!(item_enchanted(item));

    item.to_hit = 0;
    item.to_damage = 1;
    assert!(item_enchanted(item));

    item.to_damage = 0;
    item.to_ac = 1;
    assert!(item_enchanted(item));

    item.to_ac = 0;
    item.identification = ID_MAGIK;
    assert!(!item_enchanted(item));

    item.identification = 0;
    item.flags = 0x4000_107f;
    item.misc_use = 1;
    assert!(item_enchanted(item));

    item.misc_use = 0;
    assert!(!item_enchanted(item));

    item.flags = 0x07ff_e980;
    assert!(item_enchanted(item));
}

// --------------------------------------------------------------------------
// 4. examineBook
// --------------------------------------------------------------------------

#[test]
fn examine_book_guard_messages() {
    setup();
    examine_book();
    assert!(messages_contain("You are not carrying any books."));

    setup();
    with_state_mut(|s| {
        s.py.pack.unique_items = 1;
        s.py.inventory[0].category_id = TV_MAGIC_BOOK;
    });
    with_state_mut(|s| s.py.flags.blind = 1);
    examine_book();
    assert!(messages_contain("can't see"));

    setup();
    with_state_mut(|s| {
        s.py.pack.unique_items = 1;
        s.py.inventory[0].category_id = TV_MAGIC_BOOK;
        s.py.flags.blind = 0;
        s.py.carrying_light = false;
        s.py.inventory[PlayerEquipment::Light as usize].misc_use = 0;
    });
    examine_book();
    assert!(messages_contain("no light"));

    setup();
    with_state_mut(|s| {
        s.py.pack.unique_items = 1;
        s.py.inventory[0].category_id = TV_MAGIC_BOOK;
        s.py.flags.confused = 1;
        let y = s.py.pos.y as usize;
        let x = s.py.pos.x as usize;
        s.dg.floor[y][x].temporary_light = true;
    });
    examine_book();
    assert!(messages_contain("too confused"));
}

#[test]
fn examine_book_wrong_language_and_spell_index_order() {
    setup();
    with_state_mut(|s| {
        s.py.misc.class_id = MAGE_CLASS_ID;
        s.py.pack.unique_items = 1;
        s.py.inventory[0].category_id = TV_PRAYER_BOOK;
        let y = s.py.pos.y as usize;
        let x = s.py.pos.x as usize;
        s.dg.floor[y][x].temporary_light = true;
    });
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'a')]);
    examine_book();
    assert!(
        messages_contain("do not understand"),
        "messages: {:?}",
        peek_ui_messages()
    );

    setup();
    with_state_mut(|s| {
        s.py.misc.class_id = MAGE_CLASS_ID;
        s.py.pack.unique_items = 1;
        s.py.inventory[0].category_id = TV_MAGIC_BOOK;
        s.py.inventory[0].flags = 0b1010;
        let y = s.py.pos.y as usize;
        let x = s.py.pos.x as usize;
        s.dg.floor[y][x].temporary_light = true;
    });
    test_clear_getch_keys();
    test_push_getch_keys(&[i32::from(b'a'), i32::from(b' ')]);
    examine_book();

    let mut flags = 0b1010u32;
    let mut expected = Vec::new();
    while flags != 0 {
        let bit = get_and_clear_first_bit(&mut flags);
        if MAGIC_SPELLS[(MAGE_CLASS_ID - 1) as usize][bit as usize].level_required < 99 {
            expected.push(bit);
        }
    }
    assert_eq!(expected, vec![1, 3]);
}

// --------------------------------------------------------------------------
// 5. dungeonGoUpLevel / dungeonGoDownLevel
// --------------------------------------------------------------------------

#[test]
fn dungeon_staircase_transitions() {
    setup();
    let pos = Coord_t { y: 10, x: 10 };
    with_state_mut(|s| s.py.pos = pos);
    place_treasure(pos, TV_UP_STAIR);
    with_state_mut(|s| s.dg.current_level = 5);
    dungeon_go_up_level();
    assert_eq!(with_state(|s| s.dg.current_level), 4);
    assert!(with_state(|s| s.dg.generate_new_level));
    assert!(messages_contain("up staircases"));
    assert!(messages_contain("one-way door"));

    setup();
    with_state_mut(|s| s.py.pos = pos);
    dungeon_go_up_level();
    assert!(messages_contain("I see no up staircase here."));
    assert!(with_state(|s| s.game.player_free_turn));

    setup();
    with_state_mut(|s| {
        s.py.pos = pos;
        s.dg.current_level = 3;
    });
    place_treasure(pos, TV_DOWN_STAIR);
    dungeon_go_down_level();
    assert_eq!(with_state(|s| s.dg.current_level), 4);
    assert!(with_state(|s| s.dg.generate_new_level));

    setup();
    with_state_mut(|s| s.py.pos = pos);
    dungeon_go_down_level();
    assert!(messages_contain("I see no down staircase here."));
    assert!(with_state(|s| s.game.player_free_turn));
}

// --------------------------------------------------------------------------
// 6. dungeonJamDoor
// --------------------------------------------------------------------------

#[test]
fn dungeon_jam_door_messages_and_spike_series() {
    setup();
    dungeon_jam_door();
    assert!(with_state(|s| s.game.player_free_turn));

    setup();
    test_set_direction(Some(6));
    let door = Coord_t { y: 10, x: 11 };
    place_treasure(door, TV_OPEN_DOOR);
    dungeon_jam_door();
    assert!(messages_contain("must be closed first"));

    setup();
    test_set_direction(Some(6));
    let door = Coord_t { y: 10, x: 11 };
    place_treasure(door, TV_CLOSED_DOOR);
    dungeon_jam_door();
    assert!(messages_contain("no spikes"));

    setup();
    with_state_mut(|s| {
        s.py.pack.unique_items = 1;
        s.py.inventory[0].category_id = TV_SPIKE;
        s.py.inventory[0].items_count = 1;
        s.py.inventory[0].weight = 5;
    });
    test_set_direction(Some(6));
    let door = Coord_t { y: 10, x: 11 };
    let tid = place_treasure(door, TV_CLOSED_DOOR);
    with_state_mut(|s| s.game.treasure.list[tid as usize].misc_use = 0);
    dungeon_jam_door();
    assert!(messages_contain("jam the door"));
    assert!(!with_state(|s| s.game.player_free_turn));
    assert_eq!(
        with_state(|s| s.game.treasure.list[tid as usize].misc_use),
        jam_door_misc_use_after(0)
    );

    let mut misc = 0i16;
    assert_eq!(misc, 0);
    misc = jam_door_misc_use_after(misc);
    assert_eq!(-misc, 20);
    assert_eq!(10 - misc, 30);
    misc = jam_door_misc_use_after(misc);
    assert_eq!(10 - misc, 37);
    misc = jam_door_misc_use_after(misc);
    assert_eq!(10 - misc, 43);

    setup();
    with_state_mut(|s| {
        s.py.pack.unique_items = 1;
        s.py.inventory[0].category_id = TV_SPIKE;
        s.py.inventory[0].items_count = 1;
    });
    test_set_direction(Some(6));
    let door = Coord_t { y: 10, x: 11 };
    place_treasure(door, TV_CLOSED_DOOR);
    place_monster(1, ORC_ID, door);
    dungeon_jam_door();
    let expected = format!(
        "The {} is in your way!",
        CREATURES_LIST[ORC_ID as usize].name
    );
    assert!(messages_contain(&expected));
}

// --------------------------------------------------------------------------
// 7. inventoryRefillLamp
// --------------------------------------------------------------------------

#[test]
fn inventory_refill_lamp_tiers() {
    setup();
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Light as usize].sub_category_id = 1;
    });
    inventory_refill_lamp();
    assert!(messages_contain("not using a lamp"));

    setup();
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Light as usize].sub_category_id = 0;
        s.py.inventory[PlayerEquipment::Light as usize].misc_use = 100;
    });
    inventory_refill_lamp();
    assert!(messages_contain("You have no oil."));

    setup();
    with_state_mut(|s| {
        s.py.pack.unique_items = 1;
        s.py.inventory[0].category_id = TV_FLASK;
        s.py.inventory[0].items_count = 1;
        s.py.inventory[0].misc_use = 100;
        s.py.inventory[PlayerEquipment::Light as usize].sub_category_id = 0;
        s.py.inventory[PlayerEquipment::Light as usize].misc_use = 100;
    });
    inventory_refill_lamp();
    assert!(messages_contain("less than half full"));

    setup();
    with_state_mut(|s| {
        s.py.pack.unique_items = 1;
        s.py.inventory[0].category_id = TV_FLASK;
        s.py.inventory[0].items_count = 1;
        s.py.inventory[0].misc_use = (i32::from(OBJECT_LAMP_MAX_CAPACITY) / 4) as i16;
        s.py.inventory[PlayerEquipment::Light as usize].sub_category_id = 0;
        s.py.inventory[PlayerEquipment::Light as usize].misc_use =
            (i32::from(OBJECT_LAMP_MAX_CAPACITY) / 4) as i16;
    });
    inventory_refill_lamp();
    assert!(messages_contain("half full"));

    setup();
    with_state_mut(|s| {
        s.py.pack.unique_items = 1;
        s.py.inventory[0].category_id = TV_FLASK;
        s.py.inventory[0].items_count = 1;
        s.py.inventory[0].misc_use = 5000;
        s.py.inventory[PlayerEquipment::Light as usize].sub_category_id = 0;
        s.py.inventory[PlayerEquipment::Light as usize].misc_use = 9000;
    });
    inventory_refill_lamp();
    assert!(messages_contain("more than half full"));

    setup();
    with_state_mut(|s| {
        s.py.pack.unique_items = 1;
        s.py.inventory[0].category_id = TV_FLASK;
        s.py.inventory[0].items_count = 1;
        s.py.inventory[0].misc_use = 5000;
        s.py.inventory[PlayerEquipment::Light as usize].sub_category_id = 0;
        s.py.inventory[PlayerEquipment::Light as usize].misc_use =
            (i32::from(OBJECT_LAMP_MAX_CAPACITY) - 1000) as i16;
    });
    inventory_refill_lamp();
    assert!(messages_contain("overflows"));
    assert!(messages_contain("full"));
    assert_eq!(
        with_state(|s| s.py.inventory[PlayerEquipment::Light as usize].misc_use),
        OBJECT_LAMP_MAX_CAPACITY as i16
    );
}

// --------------------------------------------------------------------------
// 8. playDungeon prologue
// --------------------------------------------------------------------------

#[test]
fn play_dungeon_prologue_order() {
    setup();
    with_state_mut(|s| s.py.flags.status |= PY_SEARCH);
    test_set_play_dungeon_max_turns(1);
    play_one_dungeon_turn();

    let trace = test_play_dungeon_trace();
    let expected_prefix = [
        PlayDungeonTrace::InitPlayerLight,
        PlayDungeonTrace::UpdateMaxDepth,
        PlayDungeonTrace::ResetDungeonFlags,
        PlayDungeonTrace::PanelReset,
        PlayDungeonTrace::ResetView,
        PlayDungeonTrace::SearchOff,
        PlayDungeonTrace::UpdateMonstersFalse,
        PlayDungeonTrace::PrintDepth,
    ];
    assert_eq!(&trace[..expected_prefix.len()], expected_prefix);
}

// --------------------------------------------------------------------------
// 9. playDungeon per-turn body order
// --------------------------------------------------------------------------

#[test]
fn play_dungeon_turn_body_order() {
    setup();
    with_state_mut(|s| {
        s.py.flags.status |= PY_STR_WGT | PY_STUDY;
        s.py.flags.teleport = true;
        s.dg.game_turn = 0xF;
        s.py.misc.level = 1;
    });
    test_set_play_dungeon_max_turns(1);
    test_set_select_ready(Some(false));
    play_one_dungeon_turn();

    let trace = test_play_dungeon_trace();
    let turn_start = trace
        .iter()
        .position(|e| *e == PlayDungeonTrace::TurnBegin)
        .expect("turn begin");
    let body = &trace[turn_start..];
    let expected = [
        PlayDungeonTrace::TurnBegin,
        PlayDungeonTrace::UpdateLightStatus,
        PlayDungeonTrace::UpdateHeroStatus,
        PlayDungeonTrace::FoodConsumption,
        PlayDungeonTrace::UpdateRegeneration,
        PlayDungeonTrace::UpdateBlindness,
        PlayDungeonTrace::UpdateConfusion,
        PlayDungeonTrace::UpdateFearState,
        PlayDungeonTrace::UpdatePoisonedState,
        PlayDungeonTrace::UpdateSpeed,
        PlayDungeonTrace::UpdateRestingState,
        PlayDungeonTrace::UpdateHallucination,
        PlayDungeonTrace::UpdateParalysis,
        PlayDungeonTrace::UpdateEvilProtection,
        PlayDungeonTrace::UpdateInvulnerability,
        PlayDungeonTrace::UpdateBlessedness,
        PlayDungeonTrace::UpdateHeatResistance,
        PlayDungeonTrace::UpdateColdResistance,
        PlayDungeonTrace::UpdateDetectInvisible,
        PlayDungeonTrace::UpdateInfraVision,
        PlayDungeonTrace::UpdateWordOfRecall,
        PlayDungeonTrace::PlayerStrength,
        PlayDungeonTrace::PrintStudyInstruction,
        PlayDungeonTrace::UpdateStatusFlags,
        PlayDungeonTrace::ExecuteInputCommands,
        PlayDungeonTrace::UpdateMonstersTrue,
    ];
    for (idx, event) in expected.iter().enumerate() {
        assert_eq!(body[idx], *event, "body step {idx}");
    }
}

// --------------------------------------------------------------------------
// 10. Turn-boundary behaviors
// --------------------------------------------------------------------------

#[test]
fn play_dungeon_store_maintenance_on_1000_turn_boundary() {
    setup();
    with_state_mut(|s| {
        s.dg.current_level = 5;
        s.dg.game_turn = 999;
    });
    play_one_dungeon_turn();

    assert!(test_play_dungeon_trace().contains(&PlayDungeonTrace::StoreMaintenance));
    assert_eq!(with_state(|s| s.dg.game_turn), 1000);
}

#[test]
fn play_dungeon_interrupt_check_guarded_by_select_ready() {
    setup();
    with_state_mut(|s| {
        s.py.flags.rest = 5;
        s.py.flags.food = 10_000;
        s.game.command_count = 0;
        s.py.running_tracker = 0;
    });
    test_set_play_dungeon_max_turns(1);
    test_set_select_ready(Some(false));
    play_one_dungeon_turn();
    assert!(!test_play_dungeon_trace().contains(&PlayDungeonTrace::InterruptCheck));

    setup();
    with_state_mut(|s| {
        s.py.flags.rest = 5;
        s.py.flags.food = 10_000;
        s.game.command_count = 0;
        s.py.running_tracker = 0;
    });
    test_set_play_dungeon_max_turns(1);
    test_set_select_ready(Some(true));
    play_one_dungeon_turn();
    assert!(test_play_dungeon_trace().contains(&PlayDungeonTrace::InterruptCheck));
}

#[test]
fn play_dungeon_loop_exits_on_generate_new_level() {
    setup();
    play_one_dungeon_turn();
    assert!(with_state(|s| s.dg.generate_new_level));
    assert_eq!(
        test_play_dungeon_trace()
            .iter()
            .filter(|e| **e == PlayDungeonTrace::TurnBegin)
            .count(),
        1
    );
}

#[test]
fn play_dungeon_compact_monsters_when_nearly_full() {
    setup();
    with_state_mut(|s| {
        s.next_free_monster_id = i16::from(MON_TOTAL_ALLOCATIONS) - 5;
    });
    play_one_dungeon_turn();
    assert!(test_play_dungeon_trace().contains(&PlayDungeonTrace::CompactMonsters));
}

// --------------------------------------------------------------------------
// 11. Full playthrough parity (single-turn scripted harness)
// --------------------------------------------------------------------------

#[test]
fn play_dungeon_single_turn_state_and_rng_consumption() {
    setup();
    with_state_mut(|s| {
        s.dg.game_turn = 0;
        s.dg.current_level = 1;
        s.py.misc.level = 10;
    });

    let baseline_turn = with_state(|s| s.dg.game_turn);
    let rng_before = random_number(100);

    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.py.pos = Coord_t { y: 10, x: 10 };
        s.dg.height = 20;
        s.dg.width = 20;
        s.dg.game_turn = 0;
        s.dg.current_level = 1;
        s.py.misc.level = 10;
    });
    play_one_dungeon_turn();

    assert_eq!(with_state(|s| s.dg.game_turn), baseline_turn + 1);
    let rng_after = random_number(100);
    assert_eq!(rng_after, rng_before);
    assert!(with_state(|s| s.dg.generate_new_level));
}
