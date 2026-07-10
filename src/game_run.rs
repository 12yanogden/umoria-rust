//! Port of `src/game_run.cpp` — boot path (`phase_5.6.1`) and command dispatch (`phase_5.6.3`).

use std::cell::{Cell, RefCell};
use std::path::Path;

use crate::character::character_create;
use crate::config::dungeon::objects::OBJ_NOTHING;
use crate::config::files;
use crate::config::identification::ID_MAGIK;
use crate::config::identification::ID_SHOW_HIT_DAM;
use crate::config::monsters::{MON_CHANCE_OF_NEW, MON_ENDGAME_MONSTERS, MON_MAX_SIGHT};
use crate::config::player::status::{
    PY_ARMOR, PY_BLESSED, PY_BLIND, PY_CONFUSED, PY_DET_INV, PY_FAST, PY_FEAR, PY_HERO, PY_HP,
    PY_HUNGRY, PY_INVULN, PY_MANA, PY_PARALYSED, PY_POISONED, PY_REPEAT, PY_SEARCH, PY_SHERO,
    PY_SLOW, PY_SPEED, PY_STATS, PY_STR, PY_STR_WGT, PY_STUDY, PY_TIM_INFRA, PY_WEAK,
};
use crate::config::player::{
    PLAYER_FOOD_ALERT, PLAYER_FOOD_FAINT, PLAYER_FOOD_WEAK, PLAYER_REGEN_FAINT,
    PLAYER_REGEN_HPBASE, PLAYER_REGEN_MNBASE, PLAYER_REGEN_NORMAL, PLAYER_REGEN_WEAK,
};
use crate::config::spells::{SPELL_TYPE_MAGE, SPELL_TYPE_PRIEST};
use crate::config::treasure::OBJECT_LAMP_MAX_CAPACITY;
use crate::data_creatures::CREATURES_LIST;
use crate::data_player::{CLASSES, CLASS_BASE_PROVISIONS, MAGIC_SPELLS};
use crate::data_treasure::GAME_OBJECTS;
use crate::dungeon::dungeon_display_map;
use crate::dungeon::{SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::dungeon_generate::generate_cave;
use crate::dungeon_los::look;
use crate::game::{
    current_unix_time, get_direction_with_memory, random_number, seeds_initialize,
    set_game_options, with_state, with_state_mut,
};
use crate::game_death::end_game;
use crate::game_files::display_splash_screen;
use crate::game_files::{display_text_help_file, output_random_level_objects_to_file};
use crate::game_save::{load_game, save_game};
use crate::helpers::get_and_clear_first_bit;
use crate::identification::{
    identify_game_object, item_append_to_inscription, item_identify_as_store_bought, item_inscribe,
    item_type_remaining_count_description, magic_initialize_item_names, spell_item_identified,
};
use crate::inventory::{
    inventory_carry_item, inventory_destroy_item, inventory_find_range, inventory_item_copy_to,
    inventory_item_is_cursed, Inventory, PlayerEquipment, PLAYER_INVENTORY_SIZE,
};
use crate::mage_spells::get_and_cast_magic_spell;
use crate::monster::{update_monsters, MON_TOTAL_ALLOCATIONS};
use crate::monster_manager::{compact_monsters, monster_place_new_within_distance};
use crate::player::{player_calculate_allowed_spells_count, player_gain_mana, PlayerAttr};
use crate::player::{
    player_change_speed, player_close_door, player_disturb, player_gain_spells, player_no_light,
    player_open_closed_object, player_recalculate_bonuses, player_rest_off, player_rest_on,
    player_search, player_search_off, player_search_on, player_strength, player_takes_hit,
    player_teleport,
};
use crate::player_bash::player_bash;
use crate::player_eat::player_eat;
use crate::player_move::player_move;
use crate::player_move::player_move_position;
use crate::player_pray::pray;
use crate::player_quaff::quaff;
use crate::player_run::{player_end_running, player_find_initialize, player_run_and_find};
use crate::player_stats::player_initialize_base_experience_levels;
use crate::player_stats::player_stat_adjustment_constitution;
use crate::player_throw::player_throw_item;
use crate::player_traps::player_disarm_trap;
use crate::player_tunnel::player_tunnel;
use crate::scores::show_scores_screen;
use crate::scrolls::scroll_read;
use crate::spells::{spell_identify_item, spell_map_current_area, spell_mass_genocide};
use crate::staves::{staff_use, wand_aim};
use crate::store::store_maintenance;
use crate::store::{store_initialize_owners, COST_ADJUSTMENT};
use crate::treasure::TV_SWORD;
use crate::treasure::{
    TV_CLOSED_DOOR, TV_DOWN_STAIR, TV_FLASK, TV_MAGIC_BOOK, TV_MAX_ENCHANT, TV_MIN_ENCHANT,
    TV_NEVER, TV_NOTHING, TV_OPEN_DOOR, TV_PRAYER_BOOK, TV_SPIKE, TV_UP_STAIR,
};
use crate::types::{Coord_t, Vtype_t, MESSAGE_HISTORY_SIZE, MORIA_MESSAGE_SIZE};
use crate::types::{MAX_DUNGEON_OBJECTS, MON_MAX_CREATURES, MON_MAX_LEVELS, TREASURE_MAX_LEVELS};
use crate::ui::print_character_stats_block;
use crate::ui::{
    change_character_name, coord_outside_panel, display_character_stats, display_spells_list,
    draw_dungeon_panel, dungeon_reset_view, print_character_blind_status,
    print_character_confused_state, print_character_current_armor_class,
    print_character_current_depth, print_character_current_hit_points,
    print_character_current_mana, print_character_fear_state, print_character_hunger_status,
    print_character_max_hit_points, print_character_movement_state, print_character_poisoned_state,
    print_character_speed, print_character_study_instruction, print_character_winner,
};
use crate::ui_inventory::inventory_get_input_for_item_id;
use crate::ui_inventory::{inventory_execute_command, player_item_wearing_description};
use crate::ui_io::{self, ctrl_key, terminal, DELETE, ESCAPE};
use crate::wizard::{
    enter_wizard_mode, wizard_character_adjustment, wizard_create_objects, wizard_cure_all,
    wizard_drop_random_items, wizard_gain_experience, wizard_generate_object, wizard_jump_level,
    wizard_light_up_dungeon, wizard_summon_monster,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootEvent {
    SetRoguelikeKeys,
    PriceAdjust,
    DisplaySplashScreen,
    SeedsInitialize,
    InitializeMonsterLevels,
    InitializeTreasureLevels,
    StoreInitializeOwners,
    PlayerInitializeBaseExperienceLevels,
    ZeroSpellCounters,
    LoadGame,
    EnterWizardMode,
    EndGame,
    ChangeCharacterName,
    CharacterCreate,
    SetDateOfBirth,
    InitializeCharacterInventory,
    SetFoodDefaults,
    MageManaBranch,
    PriestManaBranch,
    SetDefaultPlayerFields,
    SetCharacterGenerated,
    MagicInitializeItemNames,
    BeginGameDisplay,
    GenerateCave,
    PlayDungeon,
    EofSave,
    SaveGame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PlayDungeonScript {
    #[default]
    MarkDead,
    ContinueAlive,
    SetEof,
    ContinueThenDead(u32),
}

#[derive(Clone, Copy, Debug, Default)]
struct LoadGameHook {
    enabled: bool,
    result: bool,
    generate: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayDungeonTrace {
    InitPlayerLight,
    UpdateMaxDepth,
    ResetDungeonFlags,
    PanelReset,
    ResetView,
    SearchOff,
    UpdateMonstersFalse,
    PrintDepth,
    TurnBegin,
    StoreMaintenance,
    MonsterPlaceNew,
    UpdateLightStatus,
    UpdateHeroStatus,
    FoodConsumption,
    UpdateRegeneration,
    UpdateBlindness,
    UpdateConfusion,
    UpdateFearState,
    UpdatePoisonedState,
    UpdateSpeed,
    UpdateRestingState,
    InterruptCheck,
    UpdateHallucination,
    UpdateParalysis,
    UpdateEvilProtection,
    UpdateInvulnerability,
    UpdateBlessedness,
    UpdateHeatResistance,
    UpdateColdResistance,
    UpdateDetectInvisible,
    UpdateInfraVision,
    UpdateWordOfRecall,
    RandomTeleport,
    PlayerStrength,
    PrintStudyInstruction,
    UpdateStatusFlags,
    DetectEnchantment,
    CompactMonsters,
    ExecuteInputCommands,
    PanelMoveCursor,
    TeleportPlayer,
    UpdateMonstersTrue,
}

thread_local! {
    static DISPATCH_LOG: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static TEST_REGEN_HP_AMOUNTS: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
    static TEST_REGEN_MANA_AMOUNTS: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
    static TEST_END_RUNNING_COUNT: Cell<u32> = const { Cell::new(0) };
    static TEST_BOOT_EVENTS: RefCell<Vec<BootEvent>> = const { RefCell::new(Vec::new()) };
    static TEST_SKIP_CHARACTER_CREATE: Cell<bool> = const { Cell::new(false) };
    static TEST_SKIP_GENERATE_CAVE: Cell<bool> = const { Cell::new(false) };
    static TEST_SKIP_END_GAME: Cell<bool> = const { Cell::new(false) };
    static TEST_PLAY_DUNGEON_SCRIPT: Cell<PlayDungeonScript> = const { Cell::new(PlayDungeonScript::MarkDead) };
    static TEST_PLAY_DUNGEON_OVERRIDE: Cell<bool> = const { Cell::new(false) };
    static TEST_PLAY_DUNGEON_CALLS: Cell<u32> = const { Cell::new(0) };
    static TEST_PLAY_DUNGEON_MAX_TURNS: Cell<u32> = const { Cell::new(0) };
    static TEST_PLAY_DUNGEON_TRACE: RefCell<Vec<PlayDungeonTrace>> = const { RefCell::new(Vec::new()) };
    static TEST_SKIP_INPUT_COMMAND_LOOP: Cell<bool> = const { Cell::new(false) };
    static TEST_LOAD_GAME_HOOK: Cell<LoadGameHook> = const { Cell::new(LoadGameHook { enabled: false, result: false, generate: false }) };
    static TEST_SKIP_CHANGE_CHARACTER_NAME: Cell<bool> = const { Cell::new(false) };
    static TEST_BOOT_STOP_AFTER: Cell<Option<BootEvent>> = const { Cell::new(None) };
}

const SHRT_MAX: i32 = 32_767;

fn trace_play_dungeon(event: PlayDungeonTrace) {
    TEST_PLAY_DUNGEON_TRACE.with(|trace| trace.borrow_mut().push(event));
}

#[doc(hidden)]
pub fn test_reset_play_dungeon_trace() {
    TEST_PLAY_DUNGEON_TRACE.with(|trace| trace.borrow_mut().clear());
}

#[doc(hidden)]
pub fn test_play_dungeon_trace() -> Vec<PlayDungeonTrace> {
    TEST_PLAY_DUNGEON_TRACE.with(|trace| trace.borrow().clone())
}

#[doc(hidden)]
pub fn test_set_skip_input_command_loop(skip: bool) {
    TEST_SKIP_INPUT_COMMAND_LOOP.with(|c| c.set(skip));
}

#[doc(hidden)]
pub fn test_set_play_dungeon_max_turns(max: u32) {
    TEST_PLAY_DUNGEON_MAX_TURNS.with(|c| c.set(max));
}

/// C++ `examineBook` (`game_run.cpp:2083–2146`).
pub fn examine_book() {
    let mut item_pos_start = 0i32;
    let mut item_pos_end = 0i32;
    if !inventory_find_range(
        i32::from(TV_MAGIC_BOOK),
        i32::from(TV_PRAYER_BOOK),
        &mut item_pos_start,
        &mut item_pos_end,
    ) {
        terminal::print_message(Some("You are not carrying any books."));
        return;
    }

    if with_state(|state| state.py.flags.blind > 0) {
        terminal::print_message(Some("You can't see to read your spell book!"));
        return;
    }

    if player_no_light() {
        terminal::print_message(Some("You have no light to read by."));
        return;
    }

    if with_state(|state| state.py.flags.confused > 0) {
        terminal::print_message(Some("You are too confused."));
        return;
    }

    let mut item_id = 0i32;
    if inventory_get_input_for_item_id(
        &mut item_id,
        "Which Book?",
        item_pos_start,
        item_pos_end,
        None,
        None,
    ) {
        let (treasure_type, class_id) = with_state(|state| {
            (
                state.py.inventory[item_id as usize].category_id,
                state.py.misc.class_id,
            )
        });

        let can_read = if CLASSES[class_id as usize].class_to_use_mage_spells == SPELL_TYPE_MAGE {
            treasure_type == TV_MAGIC_BOOK
        } else if CLASSES[class_id as usize].class_to_use_mage_spells == SPELL_TYPE_PRIEST {
            treasure_type == TV_PRAYER_BOOK
        } else {
            false
        };

        if !can_read {
            terminal::print_message(Some("You do not understand the language."));
            return;
        }

        let mut item_flags = with_state(|state| state.py.inventory[item_id as usize].flags);
        let mut spell_index = [0i32; 31];
        let mut spell_id = 0usize;

        while item_flags != 0 {
            let bit = get_and_clear_first_bit(&mut item_flags);
            if MAGIC_SPELLS[(class_id - 1) as usize][bit as usize].level_required < 99 {
                spell_index[spell_id] = bit;
                spell_id += 1;
            }
        }

        terminal::terminal_save_screen();
        display_spells_list(&spell_index[..spell_id], spell_id as i32, true, -1);
        terminal::wait_for_continue_key(0);
        terminal::terminal_restore_screen();
    }
}

/// C++ `dungeonGoUpLevel` (`game_run.cpp:2149–2163`).
pub fn dungeon_go_up_level() {
    let (y, x) = with_state(|state| (state.py.pos.y as usize, state.py.pos.x as usize));
    let tile_id = with_state(|state| state.dg.floor[y][x].treasure_id);

    if tile_id != 0
        && with_state(|state| state.game.treasure.list[tile_id as usize].category_id) == TV_UP_STAIR
    {
        with_state_mut(|state| state.dg.current_level -= 1);
        terminal::print_message(Some("You enter a maze of up staircases."));
        terminal::print_message(Some("You pass through a one-way door."));
        with_state_mut(|state| state.dg.generate_new_level = true);
    } else {
        terminal::print_message(Some("I see no up staircase here."));
        with_state_mut(|state| state.game.player_free_turn = true);
    }
}

/// C++ `dungeonGoDownLevel` (`game_run.cpp:2166–2180`).
pub fn dungeon_go_down_level() {
    let (y, x) = with_state(|state| (state.py.pos.y as usize, state.py.pos.x as usize));
    let tile_id = with_state(|state| state.dg.floor[y][x].treasure_id);

    if tile_id != 0
        && with_state(|state| state.game.treasure.list[tile_id as usize].category_id)
            == TV_DOWN_STAIR
    {
        with_state_mut(|state| state.dg.current_level += 1);
        terminal::print_message(Some("You enter a maze of down staircases."));
        terminal::print_message(Some("You pass through a one-way door."));
        with_state_mut(|state| state.dg.generate_new_level = true);
    } else {
        terminal::print_message(Some("I see no down staircase here."));
        with_state_mut(|state| state.game.player_free_turn = true);
    }
}

/// C++ `dungeonJamDoor` (`game_run.cpp:2183–2248`).
pub fn dungeon_jam_door() {
    with_state_mut(|state| state.game.player_free_turn = true);

    let mut coord = with_state(|state| state.py.pos);
    let mut direction = 0i32;
    if !get_direction_with_memory(None, &mut direction) {
        return;
    }
    let _ = player_move_position(direction, &mut coord);

    let (treasure_id, creature_id) = with_state(|state| {
        let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
        (tile.treasure_id, tile.creature_id)
    });

    if treasure_id == 0 {
        terminal::print_message(Some("That isn't a door!"));
        return;
    }

    let item_id = with_state(|state| state.game.treasure.list[treasure_id as usize].category_id);
    if item_id != TV_CLOSED_DOOR && item_id != TV_OPEN_DOOR {
        terminal::print_message(Some("That isn't a door!"));
        return;
    }

    if item_id == TV_OPEN_DOOR {
        terminal::print_message(Some("The door must be closed first."));
        return;
    }

    if creature_id == 0 {
        let mut item_pos_start = 0i32;
        let mut item_pos_end = 0i32;
        if inventory_find_range(
            i32::from(TV_SPIKE),
            i32::from(TV_NEVER),
            &mut item_pos_start,
            &mut item_pos_end,
        ) {
            with_state_mut(|state| state.game.player_free_turn = false);
            terminal::print_message_no_command_interrupt("You jam the door with a spike.");

            with_state_mut(|state| {
                let item = &mut state.game.treasure.list[treasure_id as usize];
                if item.misc_use > 0 {
                    item.misc_use = -item.misc_use;
                }
                item.misc_use -= 1 + 190 / (10 - item.misc_use);
            });

            let (items_count, weight) = with_state(|state| {
                let spike = &state.py.inventory[item_pos_start as usize];
                (spike.items_count, spike.weight)
            });
            if items_count > 1 {
                with_state_mut(|state| {
                    state.py.inventory[item_pos_start as usize].items_count -= 1;
                    state.py.pack.weight -= weight as i16;
                });
            } else {
                inventory_destroy_item(item_pos_start);
            }
        } else {
            terminal::print_message(Some("But you have no spikes."));
        }
    } else {
        with_state_mut(|state| state.game.player_free_turn = false);
        let creature_name = with_state(|state| {
            let monster_creature_id = state.monsters[creature_id as usize].creature_id as usize;
            CREATURES_LIST[monster_creature_id].name
        });
        let mut msg = [0u8; MORIA_MESSAGE_SIZE];
        snprintf_vtype(&mut msg, &format!("The {creature_name} is in your way!"));
        let msg_len = msg.iter().position(|&b| b == 0).unwrap_or(msg.len());
        terminal::print_message(Some(std::str::from_utf8(&msg[..msg_len]).unwrap_or("")));
    }
}

/// C++ `inventoryRefillLamp` (`game_run.cpp:2251–2284`).
pub fn inventory_refill_lamp() {
    with_state_mut(|state| state.game.player_free_turn = true);

    if with_state(|state| state.py.inventory[PlayerEquipment::Light as usize].sub_category_id != 0)
    {
        terminal::print_message(Some("But you are not using a lamp."));
        return;
    }

    let mut item_pos_start = 0i32;
    let mut item_pos_end = 0i32;
    if !inventory_find_range(
        i32::from(TV_FLASK),
        i32::from(TV_NEVER),
        &mut item_pos_start,
        &mut item_pos_end,
    ) {
        terminal::print_message(Some("You have no oil."));
        return;
    }

    with_state_mut(|state| state.game.player_free_turn = false);

    with_state_mut(|state| {
        let oil = state.py.inventory[item_pos_start as usize].misc_use;
        let lamp = &mut state.py.inventory[PlayerEquipment::Light as usize];
        lamp.misc_use += oil;
    });

    let max_capacity = i32::from(OBJECT_LAMP_MAX_CAPACITY);
    let half_capacity = max_capacity / 2;
    let new_misc = with_state(|state| state.py.inventory[PlayerEquipment::Light as usize].misc_use);

    if i32::from(new_misc) > max_capacity {
        with_state_mut(|state| {
            state.py.inventory[PlayerEquipment::Light as usize].misc_use =
                OBJECT_LAMP_MAX_CAPACITY as i16;
        });
        terminal::print_message(Some("Your lamp overflows, spilling oil on the ground."));
        terminal::print_message(Some("Your lamp is full."));
    } else if i32::from(new_misc) > half_capacity {
        terminal::print_message(Some("Your lamp is more than half full."));
    } else if i32::from(new_misc) == half_capacity {
        terminal::print_message(Some("Your lamp is half full."));
    } else {
        terminal::print_message(Some("Your lamp is less than half full."));
    }

    item_type_remaining_count_description(item_pos_start);
    inventory_destroy_item(item_pos_start);
}

fn log_dispatch(command: u8) {
    DISPATCH_LOG.with(|log| log.borrow_mut().push(command));
}

#[doc(hidden)]
pub fn test_dispatch_log() -> Vec<u8> {
    DISPATCH_LOG.with(|log| std::mem::take(&mut *log.borrow_mut()))
}

#[doc(hidden)]
pub fn test_clear_dispatch_log() {
    DISPATCH_LOG.with(|log| log.borrow_mut().clear());
}

fn copy_cstr(dest: &mut [u8], src: &str) {
    let bytes = src.as_bytes();
    let len = bytes.len().min(dest.len().saturating_sub(1));
    dest[..len].copy_from_slice(&bytes[..len]);
    dest[len] = 0;
}

fn vtype_as_str(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

/// C++ `getCommandRepeatCount` (`game_run.cpp:951–993`).
pub fn get_command_repeat_count(last_input_command: &mut u8) -> i32 {
    terminal::put_string_clear_to_eol("Repeat count:", terminal::Coord { y: 0, x: 0 });

    if *last_input_command == b'#' {
        *last_input_command = b'0';
    }

    let mut repeat_count: u32 = 0;

    loop {
        if *last_input_command == DELETE || *last_input_command == ctrl_key(b'H') {
            repeat_count /= 10;
            let text = format!("{:07}", repeat_count as i16);
            terminal::put_string_clear_to_eol(&text, terminal::Coord { y: 0, x: 14 });
        } else if last_input_command.is_ascii_digit() {
            if repeat_count > 99 {
                let _ = terminal::terminal_bell_sound();
            } else {
                repeat_count = repeat_count * 10 + u32::from(*last_input_command - b'0');
                let text = format!("{repeat_count:07}");
                terminal::put_string_clear_to_eol(&text, terminal::Coord { y: 0, x: 14 });
            }
        } else {
            break;
        }
        *last_input_command = terminal::get_key_input();
    }

    if repeat_count == 0 {
        repeat_count = 99;
        let text = format!("{repeat_count}");
        terminal::put_string_clear_to_eol(&text, terminal::Coord { y: 0, x: 14 });
    }

    if *last_input_command == b' ' {
        terminal::put_string_clear_to_eol("Command:", terminal::Coord { y: 0, x: 20 });
        *last_input_command = terminal::get_key_input();
    }

    repeat_count as i32
}

/// C++ `parseAlternateCtrlInput` (`game_run.cpp:995–1014`).
pub fn parse_alternate_ctrl_input(mut last_input_command: u8) -> u8 {
    if with_state(|state| state.game.command_count > 0) {
        print_character_movement_state();
    }

    if terminal::get_command("Control-", &mut last_input_command) {
        if last_input_command.is_ascii_uppercase() {
            last_input_command = last_input_command.wrapping_sub(b'A' - 1);
        } else if last_input_command.is_ascii_lowercase() {
            last_input_command = last_input_command.wrapping_sub(b'a' - 1);
        } else {
            last_input_command = b' ';
            terminal::print_message(Some("Type ^ <letter> for a control char"));
        }
    } else {
        last_input_command = b' ';
    }

    last_input_command
}

fn direction_to_move_key(direction: i32) -> u8 {
    match direction {
        1 => b'b',
        2 => b'j',
        3 => b'n',
        4 => b'h',
        6 => b'l',
        7 => b'y',
        8 => b'k',
        9 => b'u',
        _ => b'~',
    }
}

fn direction_to_run_key(direction: i32) -> u8 {
    match direction {
        1 => b'B',
        2 => b'J',
        3 => b'N',
        4 => b'H',
        6 => b'L',
        7 => b'Y',
        8 => b'K',
        9 => b'U',
        _ => b' ',
    }
}

fn direction_to_tunnel_key(direction: i32) -> u8 {
    match direction {
        1 => ctrl_key(b'B'),
        2 => ctrl_key(b'J'),
        3 => ctrl_key(b'N'),
        4 => ctrl_key(b'H'),
        6 => ctrl_key(b'L'),
        7 => ctrl_key(b'Y'),
        8 => ctrl_key(b'K'),
        9 => ctrl_key(b'U'),
        _ => b' ',
    }
}

/// C++ `originalCommands` (`game_run.cpp:1109–1331`).
pub fn original_commands(mut command: u8) -> u8 {
    let mut direction = 0i32;

    match command {
        c if c == ctrl_key(b'K') => command = b'Q',
        c if c == ctrl_key(b'J') || c == ctrl_key(b'M') => command = b'+',
        c if c == ctrl_key(b'P')
            || c == ctrl_key(b'W')
            || c == ctrl_key(b'X')
            || c == ctrl_key(b'V')
            || c == b' '
            || c == b'!'
            || c == b'$' => {}
        b'.' => {
            if get_direction_with_memory(None, &mut direction) {
                command = direction_to_run_key(direction);
            } else {
                command = b' ';
            }
        }
        b'/' | b'<' | b'>' | b'-' | b'=' | b'{' | b'?' | b'A' => {}
        b'1' => command = b'b',
        b'2' => command = b'j',
        b'3' => command = b'n',
        b'4' => command = b'h',
        b'5' => command = b'.',
        b'6' => command = b'l',
        b'7' => command = b'y',
        b'8' => command = b'k',
        b'9' => command = b'u',
        b'B' => command = b'f',
        b'C' | b'D' | b'E' | b'F' | b'G' => {}
        b'L' => command = b'W',
        b'M' | b'R' => {}
        b'S' => command = b'#',
        b'T' => {
            if get_direction_with_memory(None, &mut direction) {
                command = direction_to_tunnel_key(direction);
            } else {
                command = b' ';
            }
        }
        b'V' => {}
        b'a' => command = b'z',
        b'b' => command = b'P',
        b'c' | b'd' | b'e' => {}
        b'f' => command = b't',
        b'h' => command = b'?',
        b'i' => {}
        b'j' => command = b'S',
        b'l' => command = b'x',
        b'm' | b'o' | b'p' | b'q' | b'r' | b's' => {}
        b't' => command = b'T',
        b'u' => command = b'Z',
        b'v' | b'w' => {}
        b'x' => command = b'X',
        c if c == ctrl_key(b'A') => {}
        c if c == ctrl_key(b'B') => command = ctrl_key(b'O'),
        c if c == ctrl_key(b'D') => {}
        c if c == ctrl_key(b'H') => command = b'\\',
        c if c == ctrl_key(b'I') => {}
        c if c == ctrl_key(b'L') => command = b'*',
        b':' => {}
        c if c == ctrl_key(b'T')
            || c == ctrl_key(b'E')
            || c == ctrl_key(b'F')
            || c == ctrl_key(b'G') => {}
        b'@' | b'+' => {}
        c if c == ctrl_key(b'U') => command = b'&',
        _ => command = b'~',
    }

    command
}

/// C++ `moveWithoutPickup` (`game_run.cpp:1333–1386`).
pub fn move_without_pickup(command: &mut u8) -> bool {
    let cmd = *command;
    if cmd != b'-' {
        return true;
    }

    let mut direction = 0i32;
    let count_save = with_state(|state| state.game.command_count);

    if get_direction_with_memory(None, &mut direction) {
        with_state_mut(|state| state.game.command_count = count_save);
        *command = direction_to_move_key(direction);
    } else {
        *command = b' ';
    }

    false
}

/// C++ `commandQuit` (`game_run.cpp:1388–1397`).
pub fn command_quit() {
    terminal::flush_input_buffer();

    if terminal::get_input_confirmation("Do you really want to quit?") {
        with_state_mut(|state| {
            state.game.character_is_dead = true;
            state.dg.generate_new_level = true;
            copy_cstr(&mut state.game.character_died_from, "Quitting");
        });
    }
}

/// C++ `calculateMaxMessageCount` (`game_run.cpp:1399–1412`).
pub fn calculate_max_message_count() -> u8 {
    let mut max_messages = MESSAGE_HISTORY_SIZE as u8;

    if with_state(|state| state.game.command_count > 0) {
        let count = with_state(|state| state.game.command_count);
        if count < MESSAGE_HISTORY_SIZE as u32 {
            max_messages = count as u8;
        }
        with_state_mut(|state| state.game.command_count = 0);
    } else if with_state(|state| state.game.last_command != ctrl_key(b'P')) {
        max_messages = 1;
    }

    max_messages
}

/// C++ `commandPreviousMessage` (`game_run.cpp:1414–1444`).
pub fn command_previous_message() {
    let max_messages = calculate_max_message_count();

    if max_messages <= 1 {
        terminal::put_string(">", terminal::Coord { y: 0, x: 0 });
        let text =
            with_state(|state| vtype_as_str(&state.messages[state.last_message_id as usize]));
        terminal::put_string_clear_to_eol(&text, terminal::Coord { y: 0, x: 1 });
        return;
    }

    terminal::terminal_save_screen();

    let line_number = max_messages;
    let mut msg_id = with_state(|state| state.last_message_id);

    let mut remaining = max_messages;
    while remaining > 0 {
        remaining -= 1;
        let row = remaining;
        let text = with_state(|state| vtype_as_str(&state.messages[msg_id as usize]));
        terminal::put_string_clear_to_eol(
            &text,
            terminal::Coord {
                y: i32::from(row),
                x: 0,
            },
        );

        if msg_id == 0 {
            msg_id = (MESSAGE_HISTORY_SIZE - 1) as i16;
        } else {
            msg_id -= 1;
        }
    }

    terminal::erase_line(terminal::Coord {
        y: i32::from(line_number),
        x: 0,
    });
    terminal::wait_for_continue_key(i32::from(line_number));
    terminal::terminal_restore_screen();
}

/// C++ `commandFlipWizardMode` (`game_run.cpp:1446–1455`).
pub fn command_flip_wizard_mode() {
    if with_state(|state| state.game.wizard_mode) {
        with_state_mut(|state| state.game.wizard_mode = false);
        terminal::print_message(Some("Wizard mode off."));
    } else if enter_wizard_mode() {
        terminal::print_message(Some("Wizard mode on."));
    }

    print_character_winner();
}

/// C++ `commandSaveAndExit` (`game_run.cpp:1457–1476`).
pub fn command_save_and_exit() {
    if with_state(|state| state.game.total_winner) {
        terminal::print_message(Some(
            "You are a Total Winner,  your character must be retired.",
        ));

        if with_state(|state| state.options.use_roguelike_keys) {
            terminal::print_message(Some("Use 'Q' to when you are ready to quit."));
        } else {
            terminal::print_message(Some("Use <Control>-K when you are ready to quit."));
        }
    } else {
        with_state_mut(|state| copy_cstr(&mut state.game.character_died_from, "(saved)"));
        terminal::print_message(Some("Saving game..."));

        if save_game() {
            end_game();
        }

        with_state_mut(|state| copy_cstr(&mut state.game.character_died_from, "(alive and well)"));
    }
}

/// C++ `commandLocateOnMap` (`game_run.cpp:1478–1548`).
pub fn command_locate_on_map() {
    if with_state(|state| state.py.flags.blind > 0) || player_no_light() {
        terminal::print_message(Some("You can't see your map."));
        return;
    }

    let mut player_coord = with_state(|state| state.py.pos);
    if coord_outside_panel(player_coord, true) {
        draw_dungeon_panel();
    }

    let old_panel = with_state(|state| Coord_t {
        y: state.dg.panel.row,
        x: state.dg.panel.col,
    });

    loop {
        let panel = with_state(|state| Coord_t {
            y: state.dg.panel.row,
            x: state.dg.panel.col,
        });

        let tmp_str = if panel.y == old_panel.y && panel.x == old_panel.x {
            String::new()
        } else {
            let ns = match panel.y.cmp(&old_panel.y) {
                std::cmp::Ordering::Less => " North",
                std::cmp::Ordering::Greater => " South",
                std::cmp::Ordering::Equal => "",
            };
            let ew = match panel.x.cmp(&old_panel.x) {
                std::cmp::Ordering::Less => " West",
                std::cmp::Ordering::Greater => " East",
                std::cmp::Ordering::Equal => "",
            };
            format!("{ns}{ew} of")
        };

        let prompt = format!(
            "Map sector [{},{}], which is{} your sector. Look which direction?",
            panel.y, panel.x, tmp_str
        );

        let mut dir_val = 0i32;
        if !get_direction_with_memory(Some(&prompt), &mut dir_val) {
            break;
        }

        loop {
            player_coord.x += ((dir_val - 1) % 3 - 1) * i32::from(SCREEN_WIDTH / 2);
            player_coord.y -= ((dir_val - 1) / 3 - 1) * i32::from(SCREEN_HEIGHT / 2);

            let width = i32::from(with_state(|state| state.dg.width));
            if player_coord.x < 0
                || player_coord.y < 0
                || player_coord.x >= width
                || player_coord.y >= width
            {
                terminal::print_message(Some("You've gone past the end of your map."));

                player_coord.x -= ((dir_val - 1) % 3 - 1) * i32::from(SCREEN_WIDTH / 2);
                player_coord.y += ((dir_val - 1) / 3 - 1) * i32::from(SCREEN_HEIGHT / 2);
                break;
            }

            if coord_outside_panel(player_coord, true) {
                draw_dungeon_panel();
                break;
            }
        }
    }

    if coord_outside_panel(with_state(|state| state.py.pos), false) {
        draw_dungeon_panel();
    }
}

/// C++ `commandToggleSearch` (`game_run.cpp:1550–1556`).
pub fn command_toggle_search() {
    if with_state(|state| state.py.flags.status & PY_SEARCH != 0) {
        player_search_off();
    } else {
        player_search_on();
    }
}

/// C++ `doWizardCommands` (`game_run.cpp:1558–1634`).
pub fn do_wizard_commands(command: u8) {
    match command {
        c if c == ctrl_key(b'A') => wizard_cure_all(),
        c if c == ctrl_key(b'E') => {
            wizard_character_adjustment();
            terminal::message_line_clear();
        }
        c if c == ctrl_key(b'F') => {
            let _ = spell_mass_genocide();
        }
        c if c == ctrl_key(b'G') => wizard_drop_random_items(),
        c if c == ctrl_key(b'D') => wizard_jump_level(),
        c if c == ctrl_key(b'O') => output_random_level_objects_to_file(),
        b'\\' => {
            if with_state(|state| state.options.use_roguelike_keys) {
                display_text_help_file(files::help_roguelike_wizard);
            } else {
                display_text_help_file(files::help_wizard);
            }
        }
        c if c == ctrl_key(b'I') => {
            let _ = spell_identify_item();
        }
        b'*' => wizard_light_up_dungeon(),
        b':' => spell_map_current_area(),
        c if c == ctrl_key(b'T') => player_teleport(100),
        b'%' => {
            wizard_generate_object();
            draw_dungeon_panel();
        }
        b'+' => wizard_gain_experience(),
        b'&' => wizard_summon_monster(),
        b'@' => wizard_create_objects(),
        _ => {
            if with_state(|state| state.options.use_roguelike_keys) {
                terminal::put_string_clear_to_eol(
                    "Type '?' or '\\' for help.",
                    terminal::Coord { y: 0, x: 0 },
                );
            } else {
                terminal::put_string_clear_to_eol(
                    "Type '?' or ^H for help.",
                    terminal::Coord { y: 0, x: 0 },
                );
            }
        }
    }
}

pub const VALID_COUNT_FALSE: &[u8] = &[
    b'Q',
    ctrl_key(b'W'),
    ctrl_key(b'X'),
    b'=',
    b'{',
    b'/',
    b'<',
    b'>',
    b'?',
    b'C',
    b'E',
    b'F',
    b'G',
    b'V',
    b'#',
    b'z',
    b'P',
    b'c',
    b'd',
    b'e',
    b't',
    b'i',
    b'x',
    b'm',
    b'p',
    b'q',
    b'r',
    b'T',
    b'Z',
    b'v',
    b'w',
    b'W',
    b'X',
    ctrl_key(b'A'),
    b'\\',
    ctrl_key(b'I'),
    b'*',
    b':',
    ctrl_key(b'T'),
    ctrl_key(b'E'),
    ctrl_key(b'F'),
    ctrl_key(b'S'),
    ctrl_key(b'Q'),
];

pub const VALID_COUNT_TRUE: &[u8] = &[
    ctrl_key(b'P'),
    ESCAPE,
    b' ',
    b'-',
    b'b',
    b'f',
    b'j',
    b'n',
    b'h',
    b'l',
    b'y',
    b'k',
    b'u',
    b'.',
    b'B',
    b'J',
    b'N',
    b'H',
    b'L',
    b'Y',
    b'K',
    b'U',
    b'D',
    b'R',
    ctrl_key(b'Y'),
    ctrl_key(b'K'),
    ctrl_key(b'U'),
    ctrl_key(b'L'),
    ctrl_key(b'N'),
    ctrl_key(b'J'),
    ctrl_key(b'B'),
    ctrl_key(b'H'),
    b'S',
    b'o',
    b's',
    ctrl_key(b'D'),
    ctrl_key(b'G'),
    b'+',
];

/// C++ `validCountCommand` (`game_run.cpp:1898–1986`).
pub fn valid_count_command(command: u8) -> bool {
    if VALID_COUNT_FALSE.contains(&command) {
        return false;
    }
    if VALID_COUNT_TRUE.contains(&command) {
        return true;
    }
    false
}

/// C++ `doCommand` (`game_run.cpp:1640–1895`).
pub fn do_command(mut command: u8) {
    let do_pickup = move_without_pickup(&mut command);

    match command {
        b'Q' => {
            command_quit();
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        c if c == ctrl_key(b'P') => {
            command_previous_message();
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        c if c == ctrl_key(b'V') => {
            display_text_help_file(files::license);
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        c if c == ctrl_key(b'W') => {
            command_flip_wizard_mode();
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        c if c == ctrl_key(b'X') => {
            command_save_and_exit();
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        b'=' => {
            terminal::terminal_save_screen();
            set_game_options();
            terminal::terminal_restore_screen();
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        b'{' => {
            item_inscribe();
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        b'!' | b'$' | ESCAPE | b' ' => {
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        b'b' => player_move(1, do_pickup),
        b'j' => player_move(2, do_pickup),
        b'n' => player_move(3, do_pickup),
        b'h' => player_move(4, do_pickup),
        b'l' => player_move(6, do_pickup),
        b'y' => player_move(7, do_pickup),
        b'k' => player_move(8, do_pickup),
        b'u' => player_move(9, do_pickup),
        b'B' => player_find_initialize(1),
        b'J' => player_find_initialize(2),
        b'N' => player_find_initialize(3),
        b'H' => player_find_initialize(4),
        b'L' => player_find_initialize(6),
        b'Y' => player_find_initialize(7),
        b'K' => player_find_initialize(8),
        b'U' => player_find_initialize(9),
        b'/' => {
            identify_game_object();
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        b'.' => {
            player_move(5, do_pickup);
            if with_state(|state| state.game.command_count > 1) {
                with_state_mut(|state| state.game.command_count -= 1);
                player_rest_on();
            }
        }
        b'<' => {
            log_dispatch(command);
            dungeon_go_up_level();
        }
        b'>' => {
            log_dispatch(command);
            dungeon_go_down_level();
        }
        b'?' => {
            if with_state(|state| state.options.use_roguelike_keys) {
                display_text_help_file(files::help_roguelike);
            } else {
                display_text_help_file(files::help);
            }
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        b'f' => player_bash(),
        b'C' => {
            terminal::terminal_save_screen();
            change_character_name();
            terminal::terminal_restore_screen();
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        b'D' => player_disarm_trap(),
        b'E' => player_eat(),
        b'F' => {
            log_dispatch(command);
            inventory_refill_lamp();
        }
        b'G' => player_gain_spells(),
        b'V' => {
            terminal::terminal_save_screen();
            show_scores_screen();
            terminal::terminal_restore_screen();
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        b'W' => {
            command_locate_on_map();
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        b'R' => player_rest_on(),
        b'#' => {
            command_toggle_search();
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        c if c == ctrl_key(b'B') => player_tunnel(1),
        c if c == ctrl_key(b'M') || c == ctrl_key(b'J') => player_tunnel(2),
        c if c == ctrl_key(b'N') => player_tunnel(3),
        c if c == ctrl_key(b'H') => player_tunnel(4),
        c if c == ctrl_key(b'L') => player_tunnel(6),
        c if c == ctrl_key(b'Y') => player_tunnel(7),
        c if c == ctrl_key(b'K') => player_tunnel(8),
        c if c == ctrl_key(b'U') => player_tunnel(9),
        b'z' => wand_aim(),
        b'M' => {
            dungeon_display_map();
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        b'P' => {
            log_dispatch(command);
            examine_book();
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        b'c' => player_close_door(),
        b'd' => inventory_execute_command(b'd'),
        b'e' => inventory_execute_command(b'e'),
        b't' => player_throw_item(),
        b'i' => inventory_execute_command(b'i'),
        b'S' => {
            log_dispatch(command);
            dungeon_jam_door();
        }
        b'x' => {
            look();
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        b'm' => get_and_cast_magic_spell(),
        b'o' => player_open_closed_object(),
        b'p' => pray(),
        b'q' => quaff(),
        b'r' => scroll_read(),
        b's' => {
            let (pos, chance) =
                with_state(|state| (state.py.pos, i32::from(state.py.misc.chance_in_search)));
            player_search(pos, chance);
        }
        b'T' => inventory_execute_command(b't'),
        b'Z' => staff_use(),
        b'v' => {
            display_text_help_file(files::versions_history);
            with_state_mut(|state| state.game.player_free_turn = true);
        }
        b'w' => inventory_execute_command(b'w'),
        b'X' => inventory_execute_command(b'x'),
        _ => {
            with_state_mut(|state| state.game.player_free_turn = true);
            if with_state(|state| state.game.wizard_mode) {
                do_wizard_commands(command);
            } else {
                terminal::put_string_clear_to_eol(
                    "Type '?' for help.",
                    terminal::Coord { y: 0, x: 0 },
                );
            }
        }
    }

    with_state_mut(|state| state.game.last_command = command);
}

/// C++ `executeInputCommands` (`game_run.cpp:1017–1107`).
pub fn execute_input_commands(command: &mut u8, find_count: &mut i32) {
    let mut last_input_command = *command;

    loop {
        if with_state(|state| state.py.flags.status & PY_REPEAT != 0) {
            print_character_movement_state();
        }

        with_state_mut(|state| {
            state.game.use_last_direction = false;
            state.game.player_free_turn = false;
        });

        if with_state(|state| state.py.running_tracker != 0) {
            player_run_and_find();
            *find_count -= 1;

            if *find_count == 0 {
                player_end_running();
            }

            terminal::put_qio();
            if !should_continue_input_loop() {
                break;
            }
            continue;
        }

        if with_state(|state| state.game.doing_inventory_command != 0) {
            let inv_cmd = with_state(|state| state.game.doing_inventory_command);
            inventory_execute_command(inv_cmd);
            if !should_continue_input_loop() {
                break;
            }
            continue;
        }

        let pos = with_state(|state| state.py.pos);
        terminal::panel_move_cursor(terminal::Coord { y: pos.y, x: pos.x });

        with_state_mut(|state| state.message_ready_to_print = false);

        if with_state(|state| state.game.command_count > 0) {
            with_state_mut(|state| state.game.use_last_direction = true);
        } else {
            last_input_command = terminal::get_key_input();

            let mut repeat_count = 0i32;
            let use_roguelike = with_state(|state| state.options.use_roguelike_keys);
            if (use_roguelike && last_input_command.is_ascii_digit())
                || (!use_roguelike && last_input_command == b'#')
            {
                repeat_count = get_command_repeat_count(&mut last_input_command);
            }

            if last_input_command == b'^' {
                last_input_command = parse_alternate_ctrl_input(last_input_command);
            }

            let pos = with_state(|state| state.py.pos);
            terminal::panel_move_cursor(terminal::Coord { y: pos.y, x: pos.x });

            if !use_roguelike {
                last_input_command = original_commands(last_input_command);
            }

            if repeat_count > 0 {
                if valid_count_command(last_input_command) {
                    with_state_mut(|state| state.game.command_count = repeat_count as u32);
                    print_character_movement_state();
                } else {
                    with_state_mut(|state| state.game.player_free_turn = true);
                    last_input_command = b' ';
                    terminal::print_message(Some("Invalid command with a count."));
                }
            }
        }

        terminal::message_line_clear();
        let pos = with_state(|state| state.py.pos);
        terminal::panel_move_cursor(terminal::Coord { y: pos.y, x: pos.x });
        terminal::put_qio();

        do_command(last_input_command);

        if with_state(|state| state.py.running_tracker != 0) {
            let count = with_state(|state| state.game.command_count);
            *find_count = count as i32 - 1;
            with_state_mut(|state| state.game.command_count = 0);
        } else if with_state(|state| state.game.player_free_turn) {
            with_state_mut(|state| state.game.command_count = 0);
        } else if with_state(|state| state.game.command_count != 0) {
            with_state_mut(|state| state.game.command_count -= 1);
        }

        if !should_continue_input_loop() {
            break;
        }
    }

    *command = last_input_command;
}

fn should_continue_input_loop() -> bool {
    with_state(|state| {
        state.game.player_free_turn && !state.dg.generate_new_level && ui_io::eof_flag() == 0
    })
}

#[doc(hidden)]
pub fn test_set_message(id: i16, text: &str) {
    with_state_mut(|state| {
        copy_cstr(&mut state.messages[id as usize], text);
        state.last_message_id = id;
    });
}

#[doc(hidden)]
pub fn test_free_turn_after_command(command: u8) -> bool {
    setup_command_test_state();
    do_command(command);
    with_state(|state| state.game.player_free_turn)
}

#[doc(hidden)]
pub fn test_last_command_after(command: u8) -> u8 {
    setup_command_test_state();
    do_command(command);
    with_state(|state| state.game.last_command)
}

fn setup_command_test_state() {
    with_state_mut(|state| {
        state.game.player_free_turn = false;
        state.game.command_count = 0;
        state.game.wizard_mode = false;
    });
}

// ---------------------------------------------------------------------------
// Phase 5.6.2 — per-turn player status/upkeep updates
// ---------------------------------------------------------------------------

#[doc(hidden)]
pub fn test_reset_game_run_hooks() {
    TEST_REGEN_HP_AMOUNTS.with(|v| v.borrow_mut().clear());
    TEST_REGEN_MANA_AMOUNTS.with(|v| v.borrow_mut().clear());
    TEST_END_RUNNING_COUNT.with(|c| c.set(0));
    test_reset_play_dungeon_trace();
    TEST_PLAY_DUNGEON_MAX_TURNS.with(|c| c.set(0));
    TEST_SKIP_INPUT_COMMAND_LOOP.with(|c| c.set(false));
}

#[doc(hidden)]
pub fn test_regenerate_hp_amounts() -> Vec<i32> {
    TEST_REGEN_HP_AMOUNTS.with(|v| v.borrow().clone())
}

#[doc(hidden)]
pub fn test_regenerate_mana_amounts() -> Vec<i32> {
    TEST_REGEN_MANA_AMOUNTS.with(|v| v.borrow().clone())
}

#[doc(hidden)]
pub fn test_end_running_count() -> u32 {
    TEST_END_RUNNING_COUNT.with(std::cell::Cell::get)
}

fn vtype_label(text: &str) -> Vtype_t {
    let mut buf = [0u8; MORIA_MESSAGE_SIZE];
    let bytes = text.as_bytes();
    let n = bytes.len().min(MORIA_MESSAGE_SIZE - 1);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf
}

fn snprintf_vtype(buf: &mut Vtype_t, formatted: &str) {
    let max = MORIA_MESSAGE_SIZE;
    let bytes = formatted.as_bytes();
    let n = bytes.len().min(max - 1);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf[n] = 0;
}

pub fn player_update_max_dungeon_depth() {
    with_state_mut(|state| {
        if state.dg.current_level > state.py.misc.max_dungeon_depth as i16 {
            state.py.misc.max_dungeon_depth = state.dg.current_level as u16;
        }
    });
}

enum LightStatusAction {
    None,
    GoneOut,
    CheckGrowingFaint,
    UnlightEmpty,
    Relight,
}

pub fn player_update_light_status() {
    let action = with_state_mut(|state| {
        let item = &mut state.py.inventory[PlayerEquipment::Light as usize];
        if state.py.carrying_light {
            if item.misc_use > 0 {
                item.misc_use -= 1;
                if item.misc_use == 0 {
                    state.py.carrying_light = false;
                    LightStatusAction::GoneOut
                } else if item.misc_use < 40 && state.py.flags.blind < 1 {
                    LightStatusAction::CheckGrowingFaint
                } else {
                    LightStatusAction::None
                }
            } else {
                state.py.carrying_light = false;
                LightStatusAction::UnlightEmpty
            }
        } else if item.misc_use > 0 {
            item.misc_use -= 1;
            state.py.carrying_light = true;
            LightStatusAction::Relight
        } else {
            LightStatusAction::None
        }
    });

    match action {
        LightStatusAction::GoneOut => {
            terminal::print_message(Some("Your light has gone out!"));
            player_disturb(0, 1);
            update_monsters(false);
        }
        LightStatusAction::CheckGrowingFaint if random_number(5) == 1 => {
            player_disturb(0, 0);
            terminal::print_message(Some("Your light is growing faint."));
        }
        LightStatusAction::UnlightEmpty => {
            player_disturb(0, 1);
            update_monsters(false);
        }
        LightStatusAction::Relight => {
            player_disturb(0, 1);
            update_monsters(false);
        }
        LightStatusAction::None | LightStatusAction::CheckGrowingFaint => {}
    }
}

pub fn player_activate_heroism() {
    with_state_mut(|state| state.py.flags.status |= PY_HERO);
    player_disturb(0, 0);
    with_state_mut(|state| {
        state.py.misc.max_hp += 10;
        state.py.misc.current_hp += 10;
        state.py.misc.bth += 12;
        state.py.misc.bth_with_bows += 12;
    });
    terminal::print_message(Some("You feel like a HERO!"));
    print_character_max_hit_points();
    print_character_current_hit_points();
}

pub fn player_disable_heroism() {
    with_state_mut(|state| state.py.flags.status &= !PY_HERO);
    player_disturb(0, 0);
    let clamp = with_state(|state| state.py.misc.current_hp > state.py.misc.max_hp);
    with_state_mut(|state| {
        state.py.misc.max_hp -= 10;
        if clamp {
            state.py.misc.current_hp = state.py.misc.max_hp;
            state.py.misc.current_hp_fraction = 0;
        }
        state.py.misc.bth -= 12;
        state.py.misc.bth_with_bows -= 12;
    });
    if clamp {
        print_character_current_hit_points();
    }
    terminal::print_message(Some("The heroism wears off."));
    print_character_max_hit_points();
}

pub fn player_activate_super_heroism() {
    with_state_mut(|state| state.py.flags.status |= PY_SHERO);
    player_disturb(0, 0);
    with_state_mut(|state| {
        state.py.misc.max_hp += 20;
        state.py.misc.current_hp += 20;
        state.py.misc.bth += 24;
        state.py.misc.bth_with_bows += 24;
    });
    terminal::print_message(Some("You feel like a SUPER HERO!"));
    print_character_max_hit_points();
    print_character_current_hit_points();
}

pub fn player_disable_super_heroism() {
    with_state_mut(|state| state.py.flags.status &= !PY_SHERO);
    player_disturb(0, 0);
    let clamp = with_state(|state| state.py.misc.current_hp > state.py.misc.max_hp);
    with_state_mut(|state| {
        state.py.misc.max_hp -= 20;
        if clamp {
            state.py.misc.current_hp = state.py.misc.max_hp;
            state.py.misc.current_hp_fraction = 0;
        }
        state.py.misc.bth -= 24;
        state.py.misc.bth_with_bows -= 24;
    });
    if clamp {
        print_character_current_hit_points();
    }
    terminal::print_message(Some("The super heroism wears off."));
    print_character_max_hit_points();
}

pub fn player_update_hero_status() {
    if with_state(|state| state.py.flags.heroism) > 0 {
        if with_state(|state| state.py.flags.status & PY_HERO) == 0 {
            player_activate_heroism();
        }
        with_state_mut(|state| state.py.flags.heroism -= 1);
        if with_state(|state| state.py.flags.heroism) == 0 {
            player_disable_heroism();
        }
    }
    if with_state(|state| state.py.flags.super_heroism) > 0 {
        if with_state(|state| state.py.flags.status & PY_SHERO) == 0 {
            player_activate_super_heroism();
        }
        with_state_mut(|state| state.py.flags.super_heroism -= 1);
        if with_state(|state| state.py.flags.super_heroism) == 0 {
            player_disable_super_heroism();
        }
    }
}

pub fn player_food_consumption() -> i32 {
    let mut regen_amount = i32::from(PLAYER_REGEN_NORMAL);
    let (food, need_weak_msg, need_hungry_msg, need_faint_rng) = with_state(|state| {
        let food = state.py.flags.food;
        let status = state.py.flags.status;
        let below_alert = food < PLAYER_FOOD_ALERT as i16;
        let below_weak = food < PLAYER_FOOD_WEAK as i16;
        (
            food,
            below_alert && below_weak && (status & PY_WEAK) == 0,
            below_alert && !below_weak && (status & PY_HUNGRY) == 0,
            below_alert && below_weak && food < PLAYER_FOOD_FAINT as i16,
        )
    });

    if food < PLAYER_FOOD_ALERT as i16 {
        if food < PLAYER_FOOD_WEAK as i16 {
            if food < 0 {
                regen_amount = 0;
            } else if food < PLAYER_FOOD_FAINT as i16 {
                regen_amount = i32::from(PLAYER_REGEN_FAINT);
            } else if food < PLAYER_FOOD_WEAK as i16 {
                regen_amount = i32::from(PLAYER_REGEN_WEAK);
            }
            if need_weak_msg {
                with_state_mut(|state| state.py.flags.status |= PY_WEAK);
                terminal::print_message(Some("You are getting weak from hunger."));
                player_disturb(0, 0);
                print_character_hunger_status();
            }
            if need_faint_rng && random_number(8) == 1 {
                let paralysis_add = random_number(5);
                with_state_mut(|state| state.py.flags.paralysis += paralysis_add as i16);
                terminal::print_message(Some("You faint from the lack of food."));
                player_disturb(1, 0);
            }
        } else if need_hungry_msg {
            with_state_mut(|state| state.py.flags.status |= PY_HUNGRY);
            terminal::print_message(Some("You are getting hungry."));
            player_disturb(0, 0);
            print_character_hunger_status();
        }
    }

    with_state_mut(|state| {
        if state.py.flags.speed < 0 {
            state.py.flags.food -= state.py.flags.speed * state.py.flags.speed;
        }
        state.py.flags.food -= state.py.flags.food_digested;
    });

    let starve_damage = with_state(|state| {
        if state.py.flags.food < 0 {
            -state.py.flags.food / 16
        } else {
            0
        }
    });
    if starve_damage != 0 {
        player_takes_hit(i32::from(starve_damage), &vtype_label("starvation"));
        player_disturb(1, 0);
    }
    regen_amount
}

pub fn player_regenerate_hit_points(percent: i32) {
    TEST_REGEN_HP_AMOUNTS.with(|v| v.borrow_mut().push(percent));
    let old_chp = with_state(|state| state.py.misc.current_hp);
    let (max_hp, fraction) = with_state(|state| {
        (
            i32::from(state.py.misc.max_hp),
            i32::from(state.py.misc.current_hp_fraction),
        )
    });
    let new_chp = max_hp * percent + i32::from(PLAYER_REGEN_HPBASE);
    with_state_mut(|state| {
        state.py.misc.current_hp += (new_chp >> 16) as i16;
        if state.py.misc.current_hp < 0 && old_chp > 0 {
            state.py.misc.current_hp = SHRT_MAX as i16;
        }
        let new_chp_fraction = (new_chp & 0xFFFF) + fraction;
        if new_chp_fraction >= 0x1_0000 {
            state.py.misc.current_hp_fraction = (new_chp_fraction - 0x1_0000) as u16;
            state.py.misc.current_hp += 1;
        } else {
            state.py.misc.current_hp_fraction = new_chp_fraction as u16;
        }
        if state.py.misc.current_hp >= state.py.misc.max_hp {
            state.py.misc.current_hp = state.py.misc.max_hp;
            state.py.misc.current_hp_fraction = 0;
        }
    });
    if old_chp != with_state(|state| state.py.misc.current_hp) {
        print_character_current_hit_points();
    }
}

pub fn player_regenerate_mana(percent: i32) {
    TEST_REGEN_MANA_AMOUNTS.with(|v| v.borrow_mut().push(percent));
    let old_cmana = with_state(|state| state.py.misc.current_mana);
    let (mana, fraction) = with_state(|state| {
        (
            i32::from(state.py.misc.mana),
            i32::from(state.py.misc.current_mana_fraction),
        )
    });
    let new_mana = mana * percent + i32::from(PLAYER_REGEN_MNBASE);
    with_state_mut(|state| {
        state.py.misc.current_mana += (new_mana >> 16) as i16;
        if state.py.misc.current_mana < 0 && old_cmana > 0 {
            state.py.misc.current_mana = SHRT_MAX as i16;
        }
        let new_mana_fraction = (new_mana & 0xFFFF) + fraction;
        if new_mana_fraction >= 0x1_0000 {
            state.py.misc.current_mana_fraction = (new_mana_fraction - 0x1_0000) as u16;
            state.py.misc.current_mana += 1;
        } else {
            state.py.misc.current_mana_fraction = new_mana_fraction as u16;
        }
        if state.py.misc.current_mana >= state.py.misc.mana {
            state.py.misc.current_mana = state.py.misc.mana;
            state.py.misc.current_mana_fraction = 0;
        }
    });
    if old_cmana != with_state(|state| state.py.misc.current_mana) {
        print_character_current_mana();
    }
}

pub fn player_update_regeneration(amount: i32) {
    let mut amount = amount;
    if with_state(|state| state.py.flags.regenerate_hp) {
        amount = amount * 3 / 2;
    }
    if with_state(|state| (state.py.flags.status & PY_SEARCH) != 0 || state.py.flags.rest != 0) {
        amount *= 2;
    }
    if with_state(|state| {
        state.py.flags.poisoned < 1 && state.py.misc.current_hp < state.py.misc.max_hp
    }) {
        player_regenerate_hit_points(amount);
    }
    if with_state(|state| state.py.misc.current_mana < state.py.misc.mana) {
        player_regenerate_mana(amount);
    }
}

pub fn player_update_blindness() {
    if with_state(|state| state.py.flags.blind) <= 0 {
        return;
    }
    if with_state(|state| (state.py.flags.status & PY_BLIND) == 0) {
        with_state_mut(|state| state.py.flags.status |= PY_BLIND);
        draw_dungeon_panel();
        print_character_blind_status();
        player_disturb(0, 1);
        update_monsters(false);
    }
    with_state_mut(|state| state.py.flags.blind -= 1);
    if with_state(|state| state.py.flags.blind) == 0 {
        with_state_mut(|state| state.py.flags.status &= !PY_BLIND);
        print_character_blind_status();
        draw_dungeon_panel();
        player_disturb(0, 1);
        update_monsters(false);
        terminal::print_message(Some("The veil of darkness lifts."));
    }
}

pub fn player_update_confusion() {
    if with_state(|state| state.py.flags.confused) <= 0 {
        return;
    }
    if with_state(|state| (state.py.flags.status & PY_CONFUSED) == 0) {
        with_state_mut(|state| state.py.flags.status |= PY_CONFUSED);
        print_character_confused_state();
    }
    with_state_mut(|state| state.py.flags.confused -= 1);
    if with_state(|state| state.py.flags.confused) == 0 {
        with_state_mut(|state| state.py.flags.status &= !PY_CONFUSED);
        print_character_confused_state();
        terminal::print_message(Some("You feel less confused now."));
        if with_state(|state| state.py.flags.rest) != 0 {
            player_rest_off();
        }
    }
}

pub fn player_update_fear_state() {
    if with_state(|state| state.py.flags.afraid) <= 0 {
        return;
    }
    if with_state(|state| (state.py.flags.status & PY_FEAR) == 0) {
        if with_state(|state| state.py.flags.super_heroism + state.py.flags.heroism) > 0 {
            with_state_mut(|state| state.py.flags.afraid = 0);
        } else {
            with_state_mut(|state| state.py.flags.status |= PY_FEAR);
            print_character_fear_state();
        }
    } else if with_state(|state| state.py.flags.super_heroism + state.py.flags.heroism) > 0 {
        with_state_mut(|state| state.py.flags.afraid = 1);
    }
    with_state_mut(|state| state.py.flags.afraid -= 1);
    if with_state(|state| state.py.flags.afraid) == 0 {
        with_state_mut(|state| state.py.flags.status &= !PY_FEAR);
        print_character_fear_state();
        terminal::print_message(Some("You feel bolder now."));
        player_disturb(0, 0);
    }
}

pub fn player_update_poisoned_state() {
    if with_state(|state| state.py.flags.poisoned) <= 0 {
        return;
    }
    if with_state(|state| (state.py.flags.status & PY_POISONED) == 0) {
        with_state_mut(|state| state.py.flags.status |= PY_POISONED);
        print_character_poisoned_state();
    }
    with_state_mut(|state| state.py.flags.poisoned -= 1);
    if with_state(|state| state.py.flags.poisoned) == 0 {
        with_state_mut(|state| state.py.flags.status &= !PY_POISONED);
        print_character_poisoned_state();
        terminal::print_message(Some("You feel better."));
        player_disturb(0, 0);
        return;
    }
    let game_turn = with_state(|state| state.dg.game_turn);
    let damage = match player_stat_adjustment_constitution() {
        -4 => 4,
        -3 | -2 => 3,
        -1 => 2,
        0 => 1,
        1..=3 => i32::from(game_turn % 2 == 0),
        4 | 5 => i32::from(game_turn % 3 == 0),
        6 => i32::from(game_turn % 4 == 0),
        _ => 0,
    };
    player_takes_hit(damage, &vtype_label("poison"));
    player_disturb(1, 0);
}

pub fn player_update_fastness() {
    if with_state(|state| state.py.flags.fast) <= 0 {
        return;
    }
    if with_state(|state| (state.py.flags.status & PY_FAST) == 0) {
        with_state_mut(|state| state.py.flags.status |= PY_FAST);
        player_change_speed(-1);
        terminal::print_message(Some("You feel yourself moving faster."));
        player_disturb(0, 0);
    }
    with_state_mut(|state| state.py.flags.fast -= 1);
    if with_state(|state| state.py.flags.fast) == 0 {
        with_state_mut(|state| state.py.flags.status &= !PY_FAST);
        player_change_speed(1);
        terminal::print_message(Some("You feel yourself slow down."));
        player_disturb(0, 0);
    }
}

pub fn player_update_slowness() {
    if with_state(|state| state.py.flags.slow) <= 0 {
        return;
    }
    if with_state(|state| (state.py.flags.status & PY_SLOW) == 0) {
        with_state_mut(|state| state.py.flags.status |= PY_SLOW);
        player_change_speed(1);
        terminal::print_message(Some("You feel yourself moving slower."));
        player_disturb(0, 0);
    }
    with_state_mut(|state| state.py.flags.slow -= 1);
    if with_state(|state| state.py.flags.slow) == 0 {
        with_state_mut(|state| state.py.flags.status &= !PY_SLOW);
        player_change_speed(-1);
        terminal::print_message(Some("You feel yourself speed up."));
        player_disturb(0, 0);
    }
}

pub fn player_update_speed() {
    player_update_fastness();
    player_update_slowness();
}

pub fn player_update_resting_state() {
    let rest = with_state(|state| state.py.flags.rest);
    if rest > 0 {
        with_state_mut(|state| state.py.flags.rest -= 1);
        if with_state(|state| state.py.flags.rest) == 0 {
            player_rest_off();
        }
    } else if rest < 0 {
        with_state_mut(|state| state.py.flags.rest += 1);
        let (hp, max_hp, mana, max_mana, rest_after) = with_state(|state| {
            (
                state.py.misc.current_hp,
                state.py.misc.max_hp,
                state.py.misc.current_mana,
                state.py.misc.mana,
                state.py.flags.rest,
            )
        });
        if (hp == max_hp && mana == max_mana) || rest_after == 0 {
            player_rest_off();
        }
    }
}

pub fn player_update_hallucination() {
    if with_state(|state| state.py.flags.image) <= 0 {
        return;
    }
    player_end_running();
    TEST_END_RUNNING_COUNT.with(|c| c.set(c.get().wrapping_add(1)));
    with_state_mut(|state| state.py.flags.image -= 1);
    if with_state(|state| state.py.flags.image) == 0 {
        draw_dungeon_panel();
    }
}

pub fn player_update_paralysis() {
    if with_state(|state| state.py.flags.paralysis) <= 0 {
        return;
    }
    with_state_mut(|state| state.py.flags.paralysis -= 1);
    player_disturb(1, 0);
}

pub fn player_update_evil_protection() {
    if with_state(|state| state.py.flags.protect_evil) <= 0 {
        return;
    }
    with_state_mut(|state| state.py.flags.protect_evil -= 1);
    if with_state(|state| state.py.flags.protect_evil) == 0 {
        terminal::print_message(Some("You no longer feel safe from evil."));
    }
}

pub fn player_update_invulnerability() {
    if with_state(|state| state.py.flags.invulnerability) <= 0 {
        return;
    }
    if with_state(|state| (state.py.flags.status & PY_INVULN) == 0) {
        with_state_mut(|state| state.py.flags.status |= PY_INVULN);
        player_disturb(0, 0);
        with_state_mut(|state| {
            state.py.misc.ac += 100;
            state.py.misc.display_ac += 100;
        });
        print_character_current_armor_class();
        terminal::print_message(Some("Your skin turns into steel!"));
    }
    with_state_mut(|state| state.py.flags.invulnerability -= 1);
    if with_state(|state| state.py.flags.invulnerability) == 0 {
        with_state_mut(|state| state.py.flags.status &= !PY_INVULN);
        player_disturb(0, 0);
        with_state_mut(|state| {
            state.py.misc.ac -= 100;
            state.py.misc.display_ac -= 100;
        });
        print_character_current_armor_class();
        terminal::print_message(Some("Your skin returns to normal."));
    }
}

pub fn player_update_blessedness() {
    if with_state(|state| state.py.flags.blessed) <= 0 {
        return;
    }
    if with_state(|state| (state.py.flags.status & PY_BLESSED) == 0) {
        with_state_mut(|state| state.py.flags.status |= PY_BLESSED);
        player_disturb(0, 0);
        with_state_mut(|state| {
            state.py.misc.bth += 5;
            state.py.misc.bth_with_bows += 5;
            state.py.misc.ac += 2;
            state.py.misc.display_ac += 2;
        });
        terminal::print_message(Some("You feel righteous!"));
        print_character_current_armor_class();
    }
    with_state_mut(|state| state.py.flags.blessed -= 1);
    if with_state(|state| state.py.flags.blessed) == 0 {
        with_state_mut(|state| state.py.flags.status &= !PY_BLESSED);
        player_disturb(0, 0);
        with_state_mut(|state| {
            state.py.misc.bth -= 5;
            state.py.misc.bth_with_bows -= 5;
            state.py.misc.ac -= 2;
            state.py.misc.display_ac -= 2;
        });
        terminal::print_message(Some("The prayer has expired."));
        print_character_current_armor_class();
    }
}

pub fn player_update_heat_resistance() {
    if with_state(|state| state.py.flags.heat_resistance) <= 0 {
        return;
    }
    with_state_mut(|state| state.py.flags.heat_resistance -= 1);
    if with_state(|state| state.py.flags.heat_resistance) == 0 {
        terminal::print_message(Some("You no longer feel safe from flame."));
    }
}

pub fn player_update_cold_resistance() {
    if with_state(|state| state.py.flags.cold_resistance) <= 0 {
        return;
    }
    with_state_mut(|state| state.py.flags.cold_resistance -= 1);
    if with_state(|state| state.py.flags.cold_resistance) == 0 {
        terminal::print_message(Some("You no longer feel safe from cold."));
    }
}

pub fn player_update_detect_invisible() {
    if with_state(|state| state.py.flags.detect_invisible) <= 0 {
        return;
    }
    if with_state(|state| (state.py.flags.status & PY_DET_INV) == 0) {
        with_state_mut(|state| {
            state.py.flags.status |= PY_DET_INV;
            state.py.flags.see_invisible = true;
        });
        update_monsters(false);
    }
    with_state_mut(|state| state.py.flags.detect_invisible -= 1);
    if with_state(|state| state.py.flags.detect_invisible) == 0 {
        with_state_mut(|state| state.py.flags.status &= !PY_DET_INV);
        player_recalculate_bonuses();
        update_monsters(false);
    }
}

pub fn player_update_infra_vision() {
    if with_state(|state| state.py.flags.timed_infra) <= 0 {
        return;
    }
    if with_state(|state| (state.py.flags.status & PY_TIM_INFRA) == 0) {
        with_state_mut(|state| {
            state.py.flags.status |= PY_TIM_INFRA;
            state.py.flags.see_infra += 1;
        });
        update_monsters(false);
    }
    with_state_mut(|state| state.py.flags.timed_infra -= 1);
    if with_state(|state| state.py.flags.timed_infra) == 0 {
        with_state_mut(|state| {
            state.py.flags.status &= !PY_TIM_INFRA;
            state.py.flags.see_infra -= 1;
        });
        update_monsters(false);
    }
}

pub fn player_update_word_of_recall() {
    if with_state(|state| state.py.flags.word_of_recall) <= 0 {
        return;
    }
    if with_state(|state| state.py.flags.word_of_recall) == 1 {
        let yank = with_state_mut(|state| {
            state.dg.generate_new_level = true;
            state.py.flags.paralysis += 1;
            state.py.flags.word_of_recall = 0;
            if state.dg.current_level > 0 {
                state.dg.current_level = 0;
                Some("You feel yourself yanked upwards!")
            } else if state.py.misc.max_dungeon_depth != 0 {
                state.dg.current_level = state.py.misc.max_dungeon_depth as i16;
                Some("You feel yourself yanked downwards!")
            } else {
                None
            }
        });
        if let Some(msg) = yank {
            terminal::print_message(Some(msg));
        }
    } else {
        with_state_mut(|state| state.py.flags.word_of_recall -= 1);
    }
}

pub fn player_update_status_flags() {
    if with_state(|state| (state.py.flags.status & PY_SPEED) != 0) {
        with_state_mut(|state| state.py.flags.status &= !PY_SPEED);
        print_character_speed();
    }
    let (paralysed_bit, paralysis, rest) = with_state(|state| {
        (
            (state.py.flags.status & PY_PARALYSED) != 0,
            state.py.flags.paralysis,
            state.py.flags.rest,
        )
    });
    if paralysed_bit && paralysis < 1 {
        print_character_movement_state();
        with_state_mut(|state| state.py.flags.status &= !PY_PARALYSED);
    } else if paralysis > 0 {
        print_character_movement_state();
        with_state_mut(|state| state.py.flags.status |= PY_PARALYSED);
    } else if rest != 0 {
        print_character_movement_state();
    }
    if with_state(|state| (state.py.flags.status & PY_ARMOR) != 0) {
        print_character_current_armor_class();
        with_state_mut(|state| state.py.flags.status &= !PY_ARMOR);
    }
    if with_state(|state| (state.py.flags.status & PY_STATS) != 0) {
        let status = with_state(|state| state.py.flags.status);
        for n in 0..6 {
            if ((PY_STR << n) & status) != 0 {
                display_character_stats(n);
            }
        }
        with_state_mut(|state| state.py.flags.status &= !PY_STATS);
    }
    if with_state(|state| (state.py.flags.status & PY_HP) != 0) {
        print_character_max_hit_points();
        print_character_current_hit_points();
        with_state_mut(|state| state.py.flags.status &= !PY_HP);
    }
    if with_state(|state| (state.py.flags.status & PY_MANA) != 0) {
        print_character_current_mana();
        with_state_mut(|state| state.py.flags.status &= !PY_MANA);
    }
}

pub fn item_enchanted(item: Inventory) -> bool {
    if item.category_id < TV_MIN_ENCHANT
        || item.category_id > TV_MAX_ENCHANT
        || inventory_item_is_cursed(item)
        || spell_item_identified(item)
        || (item.identification & ID_MAGIK) != 0
    {
        return false;
    }
    if item.to_hit > 0 || item.to_damage > 0 || item.to_ac > 0 {
        return true;
    }
    if (0x4000_107f & item.flags) != 0 && item.misc_use > 0 {
        return true;
    }
    (0x07ff_e980 & item.flags) != 0
}

pub fn player_detect_enchantment() {
    let unique_items = with_state(|state| state.py.pack.unique_items);
    let mut i = 0i32;
    while i < i32::from(PLAYER_INVENTORY_SIZE) {
        if i == i32::from(unique_items) {
            i = 22;
        }
        let (category_id, enchanted) = with_state(|state| {
            let item = state.py.inventory[i as usize];
            (item.category_id, item_enchanted(item))
        });
        let chance = if i < 22 { 50 } else { 10 };
        if category_id != TV_NOTHING && enchanted && random_number(chance) == 1 {
            let desc = player_item_wearing_description(i as u8);
            let mut tmp_str = [0u8; MORIA_MESSAGE_SIZE];
            snprintf_vtype(
                &mut tmp_str,
                &format!("There's something about what you are {desc}..."),
            );
            player_disturb(0, 0);
            let msg_len = tmp_str
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(tmp_str.len());
            terminal::print_message(Some(std::str::from_utf8(&tmp_str[..msg_len]).unwrap_or("")));
            with_state_mut(|state| {
                item_append_to_inscription(&mut state.py.inventory[i as usize], ID_MAGIK);
            });
        }
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Phase 5.6.1 — startMoria boot path and init helpers
// ---------------------------------------------------------------------------

/// Round-half-up cost adjustment (C++ `priceAdjust` formula).
#[must_use]
pub fn price_adjust_cost(cost: i32, adjustment: i32) -> i32 {
    ((cost * adjustment) + 50) / 100
}

/// C++ `game_run.cpp` lines 236–243.
pub fn price_adjust() {
    if COST_ADJUSTMENT != 100 {
        // C++ mutates the global `game_objects` table at boot. Rust keeps immutable
        // `GAME_OBJECTS`; when COST_ADJUSTMENT != 100 the port would need mutable storage.
    }
}

/// C++ `game_run.cpp` lines 189–201.
pub fn initialize_monster_levels() {
    with_state_mut(|state| {
        for level in &mut state.monster_levels {
            *level = 0;
        }

        for i in 0..(MON_MAX_CREATURES - u16::from(MON_ENDGAME_MONSTERS)) as usize {
            let level = CREATURES_LIST[i].level as usize;
            state.monster_levels[level] += 1;
        }

        for i in 1..=MON_MAX_LEVELS as usize {
            state.monster_levels[i] += state.monster_levels[i - 1];
        }
    });
}

/// C++ `game_run.cpp` lines 204–233.
pub fn initialize_treasure_levels() {
    with_state_mut(|state| {
        for level in &mut state.treasure_levels {
            *level = 0;
        }

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

/// C++ `game_run.cpp` lines 160–186.
pub fn initialize_character_inventory() {
    with_state_mut(|state| {
        for entry in &mut state.py.inventory {
            inventory_item_copy_to(OBJ_NOTHING as i16, entry);
        }
        for id in &mut state.py.flags.spells_learned_order {
            *id = 99;
        }
    });

    let class_id = with_state(|state| state.py.misc.class_id as usize);
    for &item_id in &CLASS_BASE_PROVISIONS[class_id] {
        let mut item = Inventory::default();
        inventory_item_copy_to(item_id as i16, &mut item);
        item_identify_as_store_bought(&mut item);

        if item.category_id == TV_SWORD {
            item.identification |= ID_SHOW_HIT_DAM;
        }

        let _ = inventory_carry_item(item);
    }
}

/// C++ `game_run.cpp` lines 252–259.
pub fn reset_dungeon_flags() {
    with_state_mut(|state| {
        state.game.command_count = 0;
        state.dg.generate_new_level = false;
        state.py.running_tracker = 0;
        state.game.teleport_player = false;
        state.monster_multiply_total = 0;
        let y = state.py.pos.y as usize;
        let x = state.py.pos.x as usize;
        state.dg.floor[y][x].creature_id = 1;
    });
}

/// C++ `game_run.cpp` lines 262–264.
pub fn player_initialize_player_light() {
    with_state_mut(|state| {
        state.py.carrying_light = state.py.inventory[PlayerEquipment::Light as usize].misc_use > 0;
    });
}

fn record_boot_event(event: BootEvent) -> bool {
    TEST_BOOT_EVENTS.with(|events| events.borrow_mut().push(event));
    boot_stop_after(event)
}

#[doc(hidden)]
pub fn test_reset_boot_hooks() {
    TEST_BOOT_EVENTS.with(|events| events.borrow_mut().clear());
    TEST_SKIP_CHARACTER_CREATE.with(|c| c.set(false));
    TEST_SKIP_GENERATE_CAVE.with(|c| c.set(false));
    TEST_SKIP_END_GAME.with(|c| c.set(false));
    TEST_PLAY_DUNGEON_SCRIPT.with(|c| c.set(PlayDungeonScript::MarkDead));
    TEST_PLAY_DUNGEON_OVERRIDE.with(|c| c.set(true));
    TEST_PLAY_DUNGEON_CALLS.with(|c| c.set(0));
    TEST_PLAY_DUNGEON_MAX_TURNS.with(|c| c.set(0));
    test_reset_play_dungeon_trace();
    TEST_LOAD_GAME_HOOK.with(|c| c.set(LoadGameHook::default()));
    TEST_SKIP_CHANGE_CHARACTER_NAME.with(|c| c.set(false));
    TEST_BOOT_STOP_AFTER.with(|c| c.set(None));
}

#[doc(hidden)]
pub fn test_boot_events() -> Vec<BootEvent> {
    TEST_BOOT_EVENTS.with(|events| events.borrow().clone())
}

#[doc(hidden)]
pub fn test_set_skip_character_create(skip: bool) {
    TEST_SKIP_CHARACTER_CREATE.with(|c| c.set(skip));
}

#[doc(hidden)]
pub fn test_set_skip_generate_cave(skip: bool) {
    TEST_SKIP_GENERATE_CAVE.with(|c| c.set(skip));
}

#[doc(hidden)]
pub fn test_set_skip_end_game(skip: bool) {
    TEST_SKIP_END_GAME.with(|c| c.set(skip));
}

#[doc(hidden)]
pub fn test_set_play_dungeon_script(script: PlayDungeonScript) {
    TEST_PLAY_DUNGEON_SCRIPT.with(|c| c.set(script));
    TEST_PLAY_DUNGEON_OVERRIDE.with(|c| c.set(true));
}

#[doc(hidden)]
pub fn test_play_dungeon_call_count() -> u32 {
    TEST_PLAY_DUNGEON_CALLS.with(std::cell::Cell::get)
}

#[doc(hidden)]
pub fn test_set_load_game_hook(result: bool, generate: bool) {
    TEST_LOAD_GAME_HOOK.with(|c| {
        c.set(LoadGameHook {
            enabled: true,
            result,
            generate,
        });
    });
}

#[doc(hidden)]
pub fn test_set_skip_change_character_name(skip: bool) {
    TEST_SKIP_CHANGE_CHARACTER_NAME.with(|c| c.set(skip));
}

#[doc(hidden)]
pub fn test_set_boot_stop_after(event: Option<BootEvent>) {
    TEST_BOOT_STOP_AFTER.with(|c| c.set(event));
}

fn boot_stop_after(event: BootEvent) -> bool {
    TEST_BOOT_STOP_AFTER.with(std::cell::Cell::get) == Some(event)
}

fn boot_load_game(generate: &mut bool) -> bool {
    record_boot_event(BootEvent::LoadGame);
    let hook = TEST_LOAD_GAME_HOOK.with(std::cell::Cell::get);
    if hook.enabled {
        *generate = hook.generate;
        return hook.result;
    }

    let save_path = with_state(|state| state.config_save_game.clone());
    if Path::new(&save_path).exists() {
        load_game(generate)
    } else {
        false
    }
}

fn boot_generate_cave() {
    record_boot_event(BootEvent::GenerateCave);
    if !TEST_SKIP_GENERATE_CAVE.with(std::cell::Cell::get) {
        generate_cave();
    }
}

fn boot_end_game() {
    record_boot_event(BootEvent::EndGame);
    if !TEST_SKIP_END_GAME.with(std::cell::Cell::get) {
        end_game();
    }
}

fn apply_play_dungeon_test_script(call: u32) {
    if !TEST_PLAY_DUNGEON_OVERRIDE.with(std::cell::Cell::get) {
        return;
    }

    match TEST_PLAY_DUNGEON_SCRIPT.with(std::cell::Cell::get) {
        PlayDungeonScript::MarkDead => {
            with_state_mut(|state| state.game.character_is_dead = true);
        }
        PlayDungeonScript::ContinueAlive => {}
        PlayDungeonScript::SetEof => {
            ui_io::test_set_eof_flag(1);
        }
        PlayDungeonScript::ContinueThenDead(dead_on) if call >= dead_on => {
            with_state_mut(|state| state.game.character_is_dead = true);
        }
        PlayDungeonScript::ContinueThenDead(_) => {}
    }
}

fn play_dungeon_prologue() {
    trace_play_dungeon(PlayDungeonTrace::InitPlayerLight);
    player_initialize_player_light();
    trace_play_dungeon(PlayDungeonTrace::UpdateMaxDepth);
    player_update_max_dungeon_depth();
    trace_play_dungeon(PlayDungeonTrace::ResetDungeonFlags);
    reset_dungeon_flags();

    with_state_mut(|state| {
        state.dg.panel.row = -1;
        state.dg.panel.col = -1;
    });
    trace_play_dungeon(PlayDungeonTrace::PanelReset);

    trace_play_dungeon(PlayDungeonTrace::ResetView);
    dungeon_reset_view();

    if with_state(|state| state.py.flags.status & PY_SEARCH != 0) {
        trace_play_dungeon(PlayDungeonTrace::SearchOff);
        player_search_off();
    }

    trace_play_dungeon(PlayDungeonTrace::UpdateMonstersFalse);
    update_monsters(false);

    trace_play_dungeon(PlayDungeonTrace::PrintDepth);
    print_character_current_depth();
}

fn play_dungeon_turn_body(last_input_command: &mut u8, find_count: &mut i32, turn: u32) {
    trace_play_dungeon(PlayDungeonTrace::TurnBegin);

    with_state_mut(|state| state.dg.game_turn += 1);

    let (current_level, game_turn) =
        with_state(|state| (state.dg.current_level, state.dg.game_turn));
    if current_level != 0 && game_turn % 1000 == 0 {
        trace_play_dungeon(PlayDungeonTrace::StoreMaintenance);
        store_maintenance();
    }

    if random_number(i32::from(MON_CHANCE_OF_NEW)) == 1 {
        trace_play_dungeon(PlayDungeonTrace::MonsterPlaceNew);
        monster_place_new_within_distance(1, i32::from(MON_MAX_SIGHT), false);
    }

    trace_play_dungeon(PlayDungeonTrace::UpdateLightStatus);
    player_update_light_status();

    trace_play_dungeon(PlayDungeonTrace::UpdateHeroStatus);
    player_update_hero_status();

    trace_play_dungeon(PlayDungeonTrace::FoodConsumption);
    let regen_amount = player_food_consumption();
    trace_play_dungeon(PlayDungeonTrace::UpdateRegeneration);
    player_update_regeneration(regen_amount);

    trace_play_dungeon(PlayDungeonTrace::UpdateBlindness);
    player_update_blindness();
    trace_play_dungeon(PlayDungeonTrace::UpdateConfusion);
    player_update_confusion();
    trace_play_dungeon(PlayDungeonTrace::UpdateFearState);
    player_update_fear_state();
    trace_play_dungeon(PlayDungeonTrace::UpdatePoisonedState);
    player_update_poisoned_state();
    trace_play_dungeon(PlayDungeonTrace::UpdateSpeed);
    player_update_speed();
    trace_play_dungeon(PlayDungeonTrace::UpdateRestingState);
    player_update_resting_state();

    let (command_count, running, rest, microseconds) = with_state(|state| {
        (
            state.game.command_count,
            state.py.running_tracker,
            state.py.flags.rest,
            if state.py.running_tracker != 0 {
                0
            } else {
                10_000
            },
        )
    });
    if (command_count > 0 || running != 0 || rest != 0)
        && terminal::check_for_non_blocking_key_press(microseconds)
    {
        trace_play_dungeon(PlayDungeonTrace::InterruptCheck);
        player_disturb(0, 0);
    }

    trace_play_dungeon(PlayDungeonTrace::UpdateHallucination);
    player_update_hallucination();
    trace_play_dungeon(PlayDungeonTrace::UpdateParalysis);
    player_update_paralysis();
    trace_play_dungeon(PlayDungeonTrace::UpdateEvilProtection);
    player_update_evil_protection();
    trace_play_dungeon(PlayDungeonTrace::UpdateInvulnerability);
    player_update_invulnerability();
    trace_play_dungeon(PlayDungeonTrace::UpdateBlessedness);
    player_update_blessedness();
    trace_play_dungeon(PlayDungeonTrace::UpdateHeatResistance);
    player_update_heat_resistance();
    trace_play_dungeon(PlayDungeonTrace::UpdateColdResistance);
    player_update_cold_resistance();
    trace_play_dungeon(PlayDungeonTrace::UpdateDetectInvisible);
    player_update_detect_invisible();
    trace_play_dungeon(PlayDungeonTrace::UpdateInfraVision);
    player_update_infra_vision();
    trace_play_dungeon(PlayDungeonTrace::UpdateWordOfRecall);
    player_update_word_of_recall();

    if with_state(|state| state.py.flags.teleport) && random_number(100) == 1 {
        trace_play_dungeon(PlayDungeonTrace::RandomTeleport);
        player_disturb(0, 0);
        player_teleport(40);
    }

    if with_state(|state| state.py.flags.status & PY_STR_WGT != 0) {
        trace_play_dungeon(PlayDungeonTrace::PlayerStrength);
        player_strength();
    }

    if with_state(|state| state.py.flags.status & PY_STUDY != 0) {
        trace_play_dungeon(PlayDungeonTrace::PrintStudyInstruction);
        print_character_study_instruction();
    }

    trace_play_dungeon(PlayDungeonTrace::UpdateStatusFlags);
    player_update_status_flags();

    let (level, confused) = with_state(|state| (state.py.misc.level, state.py.flags.confused));
    let chance = 10 + 750 / (5 + i32::from(level));
    if game_turn.trailing_zeros() >= 4 && confused == 0 && random_number(chance) == 1 {
        trace_play_dungeon(PlayDungeonTrace::DetectEnchantment);
        player_detect_enchantment();
    }

    let free_slots = with_state(|state| {
        i32::from(MON_TOTAL_ALLOCATIONS) - i32::from(state.next_free_monster_id)
    });
    if free_slots < 10 {
        trace_play_dungeon(PlayDungeonTrace::CompactMonsters);
        let _ = compact_monsters();
    }

    let (paralysis, rest, character_is_dead) = with_state(|state| {
        (
            state.py.flags.paralysis,
            state.py.flags.rest,
            state.game.character_is_dead,
        )
    });
    if paralysis < 1 && rest == 0 && !character_is_dead {
        trace_play_dungeon(PlayDungeonTrace::ExecuteInputCommands);
        if TEST_SKIP_INPUT_COMMAND_LOOP.with(std::cell::Cell::get) {
            with_state_mut(|state| state.game.player_free_turn = false);
        } else {
            execute_input_commands(last_input_command, find_count);
        }
    } else {
        trace_play_dungeon(PlayDungeonTrace::PanelMoveCursor);
        let pos = with_state(|state| state.py.pos);
        terminal::panel_move_cursor(terminal::Coord { y: pos.y, x: pos.x });
        terminal::put_qio();
    }

    if with_state(|state| state.game.teleport_player) {
        trace_play_dungeon(PlayDungeonTrace::TeleportPlayer);
        player_teleport(100);
    }

    if !with_state(|state| state.dg.generate_new_level) {
        trace_play_dungeon(PlayDungeonTrace::UpdateMonstersTrue);
        update_monsters(true);
    }

    let max_turns = TEST_PLAY_DUNGEON_MAX_TURNS.with(std::cell::Cell::get);
    if max_turns != 0 && turn + 1 >= max_turns {
        with_state_mut(|state| state.dg.generate_new_level = true);
    }
}

/// C++ `playDungeon` (`game_run.cpp:2287–2425`).
pub fn play_dungeon() {
    record_boot_event(BootEvent::PlayDungeon);
    let call = TEST_PLAY_DUNGEON_CALLS.with(|c| {
        c.set(c.get() + 1);
        c.get()
    });

    play_dungeon_prologue();

    let test_override = TEST_PLAY_DUNGEON_OVERRIDE.with(std::cell::Cell::get);
    if !test_override {
        let mut find_count = 0i32;
        let mut last_input_command = 0u8;
        let mut turn = 0u32;

        while !with_state(|state| state.dg.generate_new_level) && ui_io::eof_flag() == 0 {
            play_dungeon_turn_body(&mut last_input_command, &mut find_count, turn);
            turn += 1;
        }
    }

    apply_play_dungeon_test_script(call);
}

/// C++ `game_run.cpp` lines 28–157.
pub fn start_moria(seed: u32, start_new_game: bool, roguelike_keys: bool) {
    with_state_mut(|state| state.options.use_roguelike_keys = roguelike_keys);
    record_boot_event(BootEvent::SetRoguelikeKeys);

    record_boot_event(BootEvent::PriceAdjust);
    price_adjust();

    record_boot_event(BootEvent::DisplaySplashScreen);
    display_splash_screen();

    record_boot_event(BootEvent::SeedsInitialize);
    seeds_initialize(seed);
    if boot_stop_after(BootEvent::SeedsInitialize) {
        return;
    }

    record_boot_event(BootEvent::InitializeMonsterLevels);
    initialize_monster_levels();

    record_boot_event(BootEvent::InitializeTreasureLevels);
    initialize_treasure_levels();

    record_boot_event(BootEvent::StoreInitializeOwners);
    store_initialize_owners();

    record_boot_event(BootEvent::PlayerInitializeBaseExperienceLevels);
    player_initialize_base_experience_levels();

    record_boot_event(BootEvent::ZeroSpellCounters);
    with_state_mut(|state| {
        state.py.flags.spells_learnt = 0;
        state.py.flags.spells_worked = 0;
        state.py.flags.spells_forgotten = 0;
    });

    let mut result = false;
    let mut generate = false;

    if !start_new_game && boot_load_game(&mut generate) {
        result = true;
    }

    if with_state(|state| state.game.to_be_wizard) {
        record_boot_event(BootEvent::EnterWizardMode);
        if !enter_wizard_mode() {
            boot_end_game();
            return;
        }
    }

    if result {
        record_boot_event(BootEvent::ChangeCharacterName);
        if !TEST_SKIP_CHANGE_CHARACTER_NAME.with(std::cell::Cell::get) {
            change_character_name();
        }

        with_state_mut(|state| {
            if state.py.misc.current_hp < 0 {
                state.game.character_is_dead = true;
            }
        });
    } else {
        record_boot_event(BootEvent::CharacterCreate);
        if !TEST_SKIP_CHARACTER_CREATE.with(std::cell::Cell::get) {
            character_create();
        }

        record_boot_event(BootEvent::SetDateOfBirth);
        with_state_mut(|state| {
            state.py.misc.date_of_birth = current_unix_time() as i32;
        });

        record_boot_event(BootEvent::InitializeCharacterInventory);
        initialize_character_inventory();

        record_boot_event(BootEvent::SetFoodDefaults);
        with_state_mut(|state| {
            state.py.flags.food = 7500;
            state.py.flags.food_digested = 2;
        });

        let class_id = with_state(|state| state.py.misc.class_id as usize);
        if CLASSES[class_id].class_to_use_mage_spells == SPELL_TYPE_MAGE {
            record_boot_event(BootEvent::MageManaBranch);
            terminal::clear_screen();
            player_calculate_allowed_spells_count(PlayerAttr::A_INT);
            player_gain_mana(PlayerAttr::A_INT);
        } else if CLASSES[class_id].class_to_use_mage_spells == SPELL_TYPE_PRIEST {
            record_boot_event(BootEvent::PriestManaBranch);
            player_calculate_allowed_spells_count(PlayerAttr::A_WIS);
            terminal::clear_screen();
            player_gain_mana(PlayerAttr::A_WIS);
        }

        record_boot_event(BootEvent::SetDefaultPlayerFields);
        with_state_mut(|state| {
            state.py.temporary_light_only = false;
            state.py.weapon_is_heavy = false;
            state.py.pack.heaviness = 0;
        });

        record_boot_event(BootEvent::SetCharacterGenerated);
        with_state_mut(|state| state.game.character_generated = true);
        generate = true;
    }

    record_boot_event(BootEvent::MagicInitializeItemNames);
    magic_initialize_item_names();

    record_boot_event(BootEvent::BeginGameDisplay);
    terminal::clear_screen();
    terminal::put_string("Press ? for help", terminal::Coord { y: 0, x: 63 });
    print_character_stats_block();

    if generate {
        boot_generate_cave();
    }

    while !with_state(|state| state.game.character_is_dead) {
        play_dungeon();

        if ui_io::eof_flag() != 0 {
            record_boot_event(BootEvent::EofSave);
            with_state_mut(|state| {
                copy_cstr(&mut state.game.character_died_from, "(end of input: saved)");
            });
            record_boot_event(BootEvent::SaveGame);
            if !save_game() {
                with_state_mut(|state| {
                    copy_cstr(&mut state.game.character_died_from, "unexpected eof");
                });
            }
            with_state_mut(|state| state.game.character_is_dead = true);
        }

        if !with_state(|state| state.game.character_is_dead) {
            boot_generate_cave();
        }
    }

    boot_end_game();
}
