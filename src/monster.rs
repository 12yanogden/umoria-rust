//! Port of src/monster.h — creature/monster data types and constants.
//! Death/hit/loot logic from monster.cpp lines 1353–1493 (phase 4.2.6).
//! Melee attacks on player from monster.cpp lines 256–475, 1541–1745 (phase 4.2.3).

use crate::config::dungeon::objects::OBJ_OPEN_DOOR;
use crate::config::monsters::defense::{CD_EVIL, CD_INFRA, CD_NO_SLEEP, CD_UNDEAD};
use crate::config::monsters::move_flags::CM_INVISIBLE;
use crate::config::monsters::move_flags::{
    CM_1D2_OBJ, CM_20_RANDOM, CM_2D2_OBJ, CM_40_RANDOM, CM_4D2_OBJ, CM_60_RANDOM, CM_75_RANDOM,
    CM_90_RANDOM, CM_ATTACK_ONLY, CM_CARRY_GOLD, CM_CARRY_OBJ, CM_EATS_OTHER, CM_MOVE_NORMAL,
    CM_MULTIPLY, CM_ONLY_MAGIC, CM_OPEN_DOOR, CM_PHASE, CM_PICKS_UP, CM_SMALL_OBJ, CM_TREASURE,
    CM_TR_SHIFT, CM_WIN,
};
use crate::config::monsters::spells::CS_FREQ;
use crate::config::monsters::{
    self, MON_MAX_MULTIPLY_PER_LEVEL, MON_MULTIPLY_ADJUST, MON_PLAYER_EXP_DRAINED_PER_HIT,
};
use crate::config::player::status::PY_BLIND;
use crate::config::treasure::OBJECTS_RUNE_PROTECTION;
use crate::data_creatures::{CREATURES_LIST, MONSTER_ATTACKS};
use crate::dice::{dice_roll, Dice};
use crate::dungeon::{
    coord_distance_between, coord_in_bounds, dungeon_delete_monster, dungeon_delete_monster_record,
    dungeon_delete_object, dungeon_lite_spot, dungeon_move_creature_record,
    dungeon_remove_monster_from_level, dungeon_summon_object, MAX_HEIGHT, MAX_WIDTH,
};
use crate::dungeon_los::los;
use crate::dungeon_tile::{MAX_OPEN_SPACE, MIN_CAVE_WALL, TILE_BOUNDARY_WALL, TILE_CORR_FLOOR};
use crate::game::{random_number, random_number_state, with_state, with_state_mut};
use crate::helpers::get_and_clear_first_bit;
use crate::inventory::inventory_item_copy_to;
use crate::inventory::{
    inventory_destroy_item, inventory_diminish_charges_attack, inventory_diminish_light_attack,
    inventory_find_range,
};
use crate::monster_manager::monster_place_new;
use crate::monster_manager::{monster_summon, monster_summon_undead};
use crate::player::player_gain_kill_experience;
use crate::player::{
    player_died_from_string, player_disturb, player_recalculate_bonuses, player_saving_throw,
    player_stat_random_decrease, player_takes_hit, player_test_attack_hits, PlayerAttr,
};
use crate::player_move::player_move_position;
use crate::player_tunnel::player_tunnel_wall;
use crate::spells::{
    damage_acid, damage_cold, damage_corroding_gas, damage_fire, damage_lightning_bolt,
    execute_disenchant_attack, spell_aggravate_monsters, spell_breath, spell_lose_exp,
    spell_teleport_away_monster, spell_teleport_player_to, MagicSpellFlags,
};
use crate::treasure::{
    TV_CLOSED_DOOR, TV_FOOD, TV_MAX_OBJECT, TV_NEVER, TV_SECRET_DOOR, TV_VIS_TRAP,
};
use crate::types::Coord_t;
use crate::types::{Vtype_t, MORIA_MESSAGE_SIZE};
use crate::ui::display_character_experience;
use crate::ui::{
    coord_inside_panel_bounds, print_character_current_mana, print_character_gold_value,
    print_character_winner,
};
use crate::ui_io::terminal;

use std::cell::RefCell;

thread_local! {
    static TEST_UPDATE_MONSTERS_CALLS: RefCell<Vec<bool>> = const { RefCell::new(Vec::new()) };
}

#[doc(hidden)]
pub fn test_reset_update_monsters_hooks() {
    TEST_UPDATE_MONSTERS_CALLS.with(|c| c.borrow_mut().clear());
}

#[doc(hidden)]
pub fn test_update_monsters_calls() -> Vec<bool> {
    TEST_UPDATE_MONSTERS_CALLS.with(|c| c.borrow().clone())
}

#[doc(hidden)]
pub(crate) fn test_record_update_monsters(attack: bool) {
    TEST_UPDATE_MONSTERS_CALLS.with(|c| c.borrow_mut().push(attack));
}

pub const MON_MAX_CREATURES: u16 = 279;
pub const MON_ATTACK_TYPES: u8 = 215;
pub const MON_TOTAL_ALLOCATIONS: u8 = 125;
pub const MON_MAX_LEVELS: u8 = 40;
pub const MON_MAX_ATTACKS: u8 = 4;

/// Port of `Creature_t` in monster.h.
#[derive(Clone, Copy, Debug, Default)]
pub struct Creature {
    pub name: &'static str,
    pub movement: u32,
    pub spells: u32,
    pub defenses: u16,
    pub kill_exp_value: u16,
    pub sleep_counter: u8,
    pub area_affect_radius: u8,
    pub ac: u8,
    pub speed: u8,
    pub sprite: u8,
    pub hit_die: Dice,
    pub damage: [u8; 4],
    pub level: u8,
}

/// Port of `Monster_t` in monster.h.
#[derive(Clone, Copy, Debug, Default)]
pub struct Monster {
    pub hp: i16,
    pub sleep_count: i16,
    pub speed: i16,
    pub creature_id: u16,
    pub pos: Coord_t,
    pub distance_from_player: u8,
    pub lit: bool,
    pub stunned_amount: u8,
    pub confused_amount: u8,
}

/// Port of `MonsterAttack_t` in monster.h.
#[derive(Clone, Copy, Debug, Default)]
pub struct MonsterAttack {
    pub type_id: u8,
    pub description_id: u8,
    pub dice: Dice,
}

pub const BLANK_MONSTER: Monster = Monster {
    hp: 0,
    sleep_count: 0,
    speed: 0,
    creature_id: 0,
    pos: Coord_t { y: 0, x: 0 },
    distance_from_player: 0,
    lit: false,
    stunned_amount: 0,
    confused_amount: 0,
};

/// C++ monster.cpp lines 1353–1397.
pub fn monster_take_hit(monster_id: i32, damage: i32) -> i32 {
    let survives = with_state_mut(|state| {
        let monster = &mut state.monsters[monster_id as usize];
        monster.sleep_count = 0;
        monster.hp -= damage as i16;
        monster.hp >= 0
    });

    if survives {
        return -1;
    }

    let (coord, creature_movement, creature_id) = with_state(|state| {
        let monster = &state.monsters[monster_id as usize];
        let creature = &CREATURES_LIST[monster.creature_id as usize];
        (
            monster.pos,
            creature.movement,
            i32::from(monster.creature_id),
        )
    });

    let treasure_flags = monster_death(coord, creature_movement);

    with_state_mut(|state| {
        let monster = &state.monsters[monster_id as usize];
        let creature = &CREATURES_LIST[monster.creature_id as usize];
        let memory = &mut state.creature_recall[monster.creature_id as usize];

        if (state.py.flags.blind < 1 && monster.lit) || (creature.movement & CM_WIN) != 0 {
            let tmp = (memory.movement & CM_TREASURE) >> CM_TR_SHIFT;
            let mut tf = treasure_flags;
            if tmp > (tf & CM_TREASURE) >> CM_TR_SHIFT {
                tf = (tf & !CM_TREASURE) | (tmp << CM_TR_SHIFT);
            }
            memory.movement = (memory.movement & !CM_TREASURE) | tf;
            if memory.kills < i16::MAX as u16 {
                memory.kills += 1;
            }
        }
    });

    player_gain_kill_experience(&CREATURES_LIST[creature_id as usize]);

    if with_state(|state| state.hack_monptr) < monster_id {
        dungeon_delete_monster(monster_id);
    } else {
        dungeon_remove_monster_from_level(monster_id);
    }

    creature_id
}

/// C++ monster.cpp lines 1399–1417.
pub fn monster_death_item_drop_type(flags: u32) -> i32 {
    let mut object = if (flags & CM_CARRY_OBJ) != 0 { 1 } else { 0 };

    if (flags & CM_CARRY_GOLD) != 0 {
        object += 2;
    }

    if (flags & CM_SMALL_OBJ) != 0 {
        object += 4;
    }

    object
}

/// C++ monster.cpp lines 1419–1443.
pub fn monster_death_item_drop_count(flags: u32) -> i32 {
    let mut count = 0;

    if (flags & CM_60_RANDOM) != 0 && random_number(100) < 60 {
        count += 1;
    }

    if (flags & CM_90_RANDOM) != 0 && random_number(100) < 90 {
        count += 1;
    }

    if (flags & CM_1D2_OBJ) != 0 {
        count += random_number(2);
    }

    if (flags & CM_2D2_OBJ) != 0 {
        count += dice_roll(Dice { dice: 2, sides: 2 });
    }

    if (flags & CM_4D2_OBJ) != 0 {
        count += dice_roll(Dice { dice: 4, sides: 2 });
    }

    count
}

/// C++ monster.cpp lines 1451–1493.
pub fn monster_death(coord: Coord_t, flags: u32) -> u32 {
    let item_type = monster_death_item_drop_type(flags);
    let item_count = monster_death_item_drop_count(flags);

    let mut dropped_item_id = 0u32;

    if item_count > 0 {
        dropped_item_id = dungeon_summon_object(coord, item_count, item_type) as u32;
    }

    if (flags & CM_WIN) != 0 && !with_state(|state| state.game.character_is_dead) {
        with_state_mut(|state| state.game.total_winner = true);
        print_character_winner();
        terminal::print_message(Some("*** CONGRATULATIONS *** You have won the game."));
        terminal::print_message(Some(
            "You cannot save this game, but you may retire when ready.",
        ));
    }

    if dropped_item_id == 0 {
        return 0;
    }

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

    let mut number_of_items = (dropped_item_id % 256) + (dropped_item_id / 256);
    number_of_items <<= CM_TR_SHIFT;

    return_flags | number_of_items
}

fn vtype_clear(buf: &mut Vtype_t) {
    buf.fill(0);
}

fn vtype_copy(dst: &mut Vtype_t, src: &Vtype_t) {
    dst.copy_from_slice(src);
}

fn vtype_set_cstr(buf: &mut Vtype_t, text: &str) {
    vtype_clear(buf);
    vtype_append_cstr(buf, text);
}

fn vtype_append_cstr(buf: &mut Vtype_t, text: &str) {
    let base = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    let bytes = text.as_bytes();
    let copy_len = bytes.len().min(MORIA_MESSAGE_SIZE - 1 - base);
    buf[base..base + copy_len].copy_from_slice(&bytes[..copy_len]);
    buf[base + copy_len] = 0;
}

fn vtype_snprintf(buf: &mut Vtype_t, formatted: &str) {
    let bytes = formatted.as_bytes();
    let n = bytes.len().min(MORIA_MESSAGE_SIZE - 1);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf[n] = 0;
}

/// C++ monster.cpp lines 16–37.
#[must_use]
pub fn monster_is_visible(monster: &Monster) -> bool {
    let (visible, recall_movement, recall_defense) = with_state(|state| {
        let tile = &state.dg.floor[monster.pos.y as usize][monster.pos.x as usize];
        let creature = &CREATURES_LIST[monster.creature_id as usize];
        let mut visible = false;
        let mut recall_movement = None;
        let mut recall_defense = None;

        if tile.permanent_light
            || tile.temporary_light
            || (state.py.running_tracker != 0
                && monster.distance_from_player < 2
                && state.py.carrying_light)
        {
            if (creature.movement & CM_INVISIBLE) == 0 {
                visible = true;
            } else if state.py.flags.see_invisible {
                visible = true;
                recall_movement = Some(CM_INVISIBLE);
            }
        } else if state.py.flags.see_infra > 0
            && i32::from(monster.distance_from_player) <= i32::from(state.py.flags.see_infra)
            && (creature.defenses & CD_INFRA) != 0
        {
            visible = true;
            recall_defense = Some(CD_INFRA);
        }

        (visible, recall_movement, recall_defense)
    });

    if recall_movement.is_some() || recall_defense.is_some() {
        with_state_mut(|state| {
            let memory = &mut state.creature_recall[monster.creature_id as usize];
            if let Some(flag) = recall_movement {
                memory.movement |= flag;
            }
            if let Some(flag) = recall_defense {
                memory.defenses |= flag;
            }
        });
    }

    visible
}

/// C++ monster.cpp lines 40–72.
pub fn monster_update_visibility(monster_id: i32) {
    let (distance, pos, was_lit, player_pos, wizard, blind, in_panel) = with_state(|state| {
        let monster = &state.monsters[monster_id as usize];
        (
            monster.distance_from_player,
            monster.pos,
            monster.lit,
            state.py.pos,
            state.game.wizard_mode,
            (state.py.flags.status & PY_BLIND) != 0,
            coord_inside_panel_bounds(&state.dg.panel, monster.pos),
        )
    });

    let mut visible = false;
    if distance <= monsters::MON_MAX_SIGHT && !blind && in_panel {
        if wizard {
            visible = true;
        } else if los(player_pos, pos) {
            let monster = with_state(|state| state.monsters[monster_id as usize]);
            visible = monster_is_visible(&monster);
        }
    }

    if visible && !was_lit {
        with_state_mut(|state| {
            state.monsters[monster_id as usize].lit = true;
            state.screen_has_changed = true;
        });
        player_disturb(1, 0);
        dungeon_lite_spot(pos);
    } else if !visible && was_lit {
        with_state_mut(|state| {
            state.monsters[monster_id as usize].lit = false;
            state.screen_has_changed = true;
        });
        dungeon_lite_spot(pos);
    }
}

/// C++ monster.cpp lines 74–92.
#[must_use]
pub fn monster_movement_rate(speed: i16) -> i32 {
    if speed > 0 {
        if with_state(|state| state.py.flags.rest) != 0 {
            return 1;
        }
        return i32::from(speed);
    }

    with_state(|state| {
        if (state.dg.game_turn % (2 - i32::from(speed))) == 0 {
            1
        } else {
            0
        }
    })
}

/// C++ monster.cpp lines 1235–1253.
pub fn memory_update_recall(monster: &Monster, wake: bool, ignore: bool, rcmove: u32) {
    if !monster.lit {
        return;
    }

    with_state_mut(|state| {
        let memory = &mut state.creature_recall[monster.creature_id as usize];
        if wake {
            memory.wake = memory.wake.saturating_add(1);
        } else if ignore {
            memory.ignore = memory.ignore.saturating_add(1);
        }
        memory.movement |= rcmove;
    });
}

/// C++ monster.cpp lines 1255–1310.
pub fn monster_attacking_update(monster_id: i32, moves: i32) {
    for _ in 0..moves {
        let mut wake = false;
        let mut ignore = false;
        let mut rcmove = 0u32;

        let should_act = with_state(|state| {
            let monster = &state.monsters[monster_id as usize];
            let creature = &CREATURES_LIST[monster.creature_id as usize];
            monster.lit
                || monster.distance_from_player <= creature.area_affect_radius
                || ((creature.movement & CM_PHASE) == 0
                    && state.dg.floor[monster.pos.y as usize][monster.pos.x as usize].feature_id
                        >= MIN_CAVE_WALL)
        });

        if should_act {
            let recovery_message = with_state_mut(|state| {
                let creature_id = state.monsters[monster_id as usize].creature_id;
                let creature = &CREATURES_LIST[creature_id as usize];

                if state.monsters[monster_id as usize].sleep_count > 0 {
                    if state.py.flags.aggravate {
                        state.monsters[monster_id as usize].sleep_count = 0;
                    } else if (state.py.flags.rest == 0 && state.py.flags.paralysis < 1)
                        || random_number_state(state, 50) == 1
                    {
                        let notice = random_number_state(state, 1024);
                        let threshold = 1i64 << (29 - i64::from(state.py.misc.stealth_factor));
                        if i64::from(notice) * i64::from(notice) * i64::from(notice) <= threshold {
                            state.monsters[monster_id as usize].sleep_count -= 100
                                / i16::from(
                                    state.monsters[monster_id as usize].distance_from_player,
                                );
                            if state.monsters[monster_id as usize].sleep_count > 0 {
                                ignore = true;
                            } else {
                                wake = true;
                                state.monsters[monster_id as usize].sleep_count = 0;
                            }
                        }
                    }
                }

                let was_lit = state.monsters[monster_id as usize].lit;
                if state.monsters[monster_id as usize].stunned_amount != 0 {
                    let level = i32::from(creature.level);
                    if random_number_state(state, 5000) < level * level {
                        state.monsters[monster_id as usize].stunned_amount = 0;
                    } else {
                        state.monsters[monster_id as usize].stunned_amount -= 1;
                    }

                    if state.monsters[monster_id as usize].stunned_amount == 0 && was_lit {
                        return Some(format!("The {} recovers and glares at you.", creature.name));
                    }
                }
                None
            });

            if let Some(message) = recovery_message {
                terminal::print_message(Some(&message));
            }

            let move_now = with_state(|state| {
                let monster = &state.monsters[monster_id as usize];
                monster.sleep_count == 0 && monster.stunned_amount == 0
            });
            if move_now {
                monster_move(monster_id, &mut rcmove);
            }
        }

        monster_update_visibility(monster_id);

        let monster = with_state(|state| state.monsters[monster_id as usize]);
        memory_update_recall(&monster, wake, ignore, rcmove);
    }
}

/// C++ monster.cpp lines 1313–1349.
pub fn update_monsters(attack: bool) {
    test_record_update_monsters(attack);
    let start_id = with_state(|state| i32::from(state.next_free_monster_id - 1));
    let min_id = i32::from(monsters::MON_MIN_INDEX_ID);
    let mut id = start_id;

    while id >= min_id {
        if with_state(|state| state.game.character_is_dead) {
            break;
        }

        if with_state(|state| state.monsters[id as usize].hp) < 0 {
            dungeon_delete_monster_record(id);
            id -= 1;
            continue;
        }

        with_state_mut(|state| {
            let monster = &mut state.monsters[id as usize];
            monster.distance_from_player = coord_distance_between(state.py.pos, monster.pos) as u8;
        });

        if attack {
            let moves =
                with_state(|state| monster_movement_rate(state.monsters[id as usize].speed));
            if moves <= 0 {
                monster_update_visibility(id);
            } else {
                monster_attacking_update(id, moves);
            }
        } else {
            monster_update_visibility(id);
        }

        if with_state(|state| state.monsters[id as usize].hp) < 0 {
            dungeon_delete_monster_record(id);
        }

        id -= 1;
    }
}

/// C++ monster.cpp lines 1495–1497.
pub fn print_monster_action_text(name: &str, action: &str) {
    terminal::print_message(Some(&format!("{name} {action}")));
}

/// C++ monster.cpp lines 1499–1504.
#[must_use]
pub fn monster_name_description(real_name: &str, is_lit: bool) -> String {
    if is_lit {
        format!("The {real_name}")
    } else {
        "It".to_string()
    }
}

/// C++ monster.cpp lines 1507–1539.
#[must_use]
pub fn monster_sleep(coord: Coord_t) -> bool {
    let mut asleep = false;

    let mut y = coord.y - 1;
    while y <= coord.y + 1 && y < i32::from(MAX_HEIGHT) {
        let mut x = coord.x - 1;
        while x <= coord.x + 1 && x < i32::from(MAX_WIDTH) {
            let monster_id = with_state(|state| state.dg.floor[y as usize][x as usize].creature_id);
            if monster_id <= 1 {
                x += 1;
                continue;
            }

            let (creature_id, lit, creature_level, creature_defenses, creature_name) =
                with_state(|state| {
                    let monster = &state.monsters[monster_id as usize];
                    let creature = &CREATURES_LIST[monster.creature_id as usize];
                    (
                        monster.creature_id,
                        monster.lit,
                        creature.level,
                        creature.defenses,
                        creature.name,
                    )
                });

            let name = monster_name_description(creature_name, lit);

            if random_number(i32::from(MON_MAX_LEVELS)) < i32::from(creature_level)
                || (creature_defenses & CD_NO_SLEEP) != 0
            {
                if lit && (creature_defenses & CD_NO_SLEEP) != 0 {
                    with_state_mut(|state| {
                        state.creature_recall[creature_id as usize].defenses |= CD_NO_SLEEP;
                    });
                }
                print_monster_action_text(&name, "is unaffected.");
            } else {
                with_state_mut(|state| {
                    state.monsters[monster_id as usize].sleep_count = 500;
                });
                asleep = true;
                print_monster_action_text(&name, "falls asleep.");
            }

            x += 1;
        }
        y += 1;
    }

    asleep
}

/// C++ monster.cpp lines 256–355.
pub fn monster_print_attack_description(msg: &mut Vtype_t, attack_id: i32) {
    match attack_id {
        1 => {
            vtype_append_cstr(msg, "hits you.");
            terminal::print_message(Some(c_vtype_str(msg)));
        }
        2 => {
            vtype_append_cstr(msg, "bites you.");
            terminal::print_message(Some(c_vtype_str(msg)));
        }
        3 => {
            vtype_append_cstr(msg, "claws you.");
            terminal::print_message(Some(c_vtype_str(msg)));
        }
        4 => {
            vtype_append_cstr(msg, "stings you.");
            terminal::print_message(Some(c_vtype_str(msg)));
        }
        5 => {
            vtype_append_cstr(msg, "touches you.");
            terminal::print_message(Some(c_vtype_str(msg)));
        }
        7 => {
            vtype_append_cstr(msg, "gazes at you.");
            terminal::print_message(Some(c_vtype_str(msg)));
        }
        8 => {
            vtype_append_cstr(msg, "breathes on you.");
            terminal::print_message(Some(c_vtype_str(msg)));
        }
        9 => {
            vtype_append_cstr(msg, "spits on you.");
            terminal::print_message(Some(c_vtype_str(msg)));
        }
        10 => {
            vtype_append_cstr(msg, "makes a horrible wail.");
            terminal::print_message(Some(c_vtype_str(msg)));
        }
        12 => {
            vtype_append_cstr(msg, "crawls on you.");
            terminal::print_message(Some(c_vtype_str(msg)));
        }
        13 => {
            vtype_append_cstr(msg, "releases a cloud of spores.");
            terminal::print_message(Some(c_vtype_str(msg)));
        }
        14 => {
            vtype_append_cstr(msg, "begs you for money.");
            terminal::print_message(Some(c_vtype_str(msg)));
        }
        15 => terminal::print_message(Some("You've been slimed!")),
        16 => {
            vtype_append_cstr(msg, "crushes you.");
            terminal::print_message(Some(c_vtype_str(msg)));
        }
        17 => {
            vtype_append_cstr(msg, "tramples you.");
            terminal::print_message(Some(c_vtype_str(msg)));
        }
        18 => {
            vtype_append_cstr(msg, "drools on you.");
            terminal::print_message(Some(c_vtype_str(msg)));
        }
        19 => {
            let suffix = match random_number(9) {
                1 => "insults you!",
                2 => "insults your mother!",
                3 => "gives you the finger!",
                4 => "humiliates you!",
                5 => "wets on your leg!",
                6 => "defiles you!",
                7 => "dances around you!",
                8 => "makes obscene gestures!",
                9 => "moons you!!!",
                _ => "",
            };
            if !suffix.is_empty() {
                vtype_append_cstr(msg, suffix);
                terminal::print_message(Some(c_vtype_str(msg)));
            }
        }
        99 => {
            vtype_append_cstr(msg, "is repelled.");
            terminal::print_message(Some(c_vtype_str(msg)));
        }
        _ => {}
    }
}

fn c_vtype_str(buf: &Vtype_t) -> &str {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    std::str::from_utf8(&buf[..end]).unwrap_or("")
}

/// C++ monster.cpp lines 357–381.
pub fn monster_confuse_on_attack(
    creature: &Creature,
    confused_amount: &mut u8,
    attack_type: i32,
    monster_name: &Vtype_t,
    visible: bool,
    creature_id: u16,
) {
    let should_confuse = with_state(|state| state.py.flags.confuse_monster);
    if !should_confuse || attack_type == 99 {
        return;
    }

    terminal::print_message(Some("Your hands stop glowing."));
    with_state_mut(|state| {
        state.py.flags.confuse_monster = false;
    });

    let mut msg = [0u8; MORIA_MESSAGE_SIZE];
    let name = c_vtype_str(monster_name);
    if random_number(i32::from(MON_MAX_LEVELS)) < i32::from(creature.level)
        || (creature.defenses & CD_NO_SLEEP) != 0
    {
        vtype_snprintf(&mut msg, &format!("{name}is unaffected."));
    } else {
        vtype_snprintf(&mut msg, &format!("{name}appears confused."));
        if *confused_amount != 0 {
            *confused_amount += 3;
        } else {
            *confused_amount = (2 + random_number(16)) as u8;
        }
    }
    terminal::print_message(Some(c_vtype_str(&msg)));

    if visible && !with_state(|state| state.game.character_is_dead) && random_number(4) == 1 {
        with_state_mut(|state| {
            let memory = &mut state.creature_recall[creature_id as usize];
            memory.defenses |= creature.defenses & CD_NO_SLEEP;
        });
    }
}

/// C++ monster.cpp lines 1541–1745.
pub fn execute_attack_on_player(
    creature_level: u8,
    monster_hp: &mut i16,
    monster_id: i32,
    attack_type: i32,
    mut damage: i32,
    death_description: &Vtype_t,
    mut noticed: bool,
) -> bool {
    let mut item_pos_start = 0i32;
    let mut item_pos_end = 0i32;

    match attack_type {
        1 => {
            let (ac, magical_ac) = with_state(|state| {
                (
                    i32::from(state.py.misc.ac),
                    i32::from(state.py.misc.magical_ac),
                )
            });
            damage -= ((ac + magical_ac) * damage) / 200;
            player_takes_hit(damage, death_description);
        }
        2 => {
            player_takes_hit(damage, death_description);
            if with_state(|state| state.py.flags.sustain_str) {
                terminal::print_message(Some("You feel weaker for a moment, but it passes."));
            } else if random_number(2) == 1 {
                terminal::print_message(Some("You feel weaker."));
                let _ = player_stat_random_decrease(PlayerAttr::A_STR);
            } else {
                noticed = false;
            }
        }
        3 => {
            player_takes_hit(damage, death_description);
            if random_number(2) == 1 {
                let confused = with_state(|state| state.py.flags.confused);
                if confused < 1 {
                    terminal::print_message(Some("You feel confused."));
                    let addition = random_number(i32::from(creature_level));
                    with_state_mut(|state| {
                        state.py.flags.confused += addition as i16;
                    });
                } else {
                    noticed = false;
                }
                with_state_mut(|state| {
                    state.py.flags.confused += 3;
                });
            } else {
                noticed = false;
            }
        }
        4 => {
            player_takes_hit(damage, death_description);
            if player_saving_throw() {
                terminal::print_message(Some("You resist the effects!"));
            } else {
                let afraid = with_state(|state| state.py.flags.afraid);
                if afraid < 1 {
                    terminal::print_message(Some("You are suddenly afraid!"));
                    let addition = 3 + random_number(i32::from(creature_level));
                    with_state_mut(|state| {
                        state.py.flags.afraid += addition as i16;
                    });
                } else {
                    with_state_mut(|state| {
                        state.py.flags.afraid += 3;
                    });
                    noticed = false;
                }
            }
        }
        5 => {
            terminal::print_message(Some("You are enveloped in flames!"));
            damage_fire(damage, death_description);
        }
        6 => {
            terminal::print_message(Some("You are covered in acid!"));
            damage_acid(damage, death_description);
        }
        7 => {
            terminal::print_message(Some("You are covered with frost!"));
            damage_cold(damage, death_description);
        }
        8 => {
            terminal::print_message(Some("Lightning strikes you!"));
            damage_lightning_bolt(damage, death_description);
        }
        9 => {
            terminal::print_message(Some("A stinging red gas swirls about you."));
            damage_corroding_gas(death_description);
            player_takes_hit(damage, death_description);
        }
        10 => {
            player_takes_hit(damage, death_description);
            let blind = with_state(|state| state.py.flags.blind);
            if blind < 1 {
                let addition = 10 + random_number(i32::from(creature_level));
                with_state_mut(|state| {
                    state.py.flags.blind += addition as i16;
                });
                terminal::print_message(Some("Your eyes begin to sting."));
            } else {
                with_state_mut(|state| {
                    state.py.flags.blind += 5;
                });
                noticed = false;
            }
        }
        11 => {
            player_takes_hit(damage, death_description);
            if player_saving_throw() {
                terminal::print_message(Some("You resist the effects!"));
            } else {
                let paralysis = with_state(|state| state.py.flags.paralysis);
                if paralysis < 1 {
                    if with_state(|state| state.py.flags.free_action) {
                        terminal::print_message(Some("You are unaffected."));
                    } else {
                        let amount = (random_number(i32::from(creature_level)) + 3) as i16;
                        with_state_mut(|state| {
                            state.py.flags.paralysis = amount;
                        });
                        terminal::print_message(Some("You are paralyzed."));
                    }
                } else {
                    noticed = false;
                }
            }
        }
        12 => {
            let (paralysis, dex, au_before) = with_state(|state| {
                (
                    state.py.flags.paralysis,
                    i32::from(state.py.stats.used[PlayerAttr::A_DEX as usize]),
                    state.py.misc.au,
                )
            });
            if paralysis < 1 && random_number(124) < dex {
                terminal::print_message(Some("You quickly protect your money pouch!"));
            } else {
                let gold = (au_before / 10) + random_number(25);
                with_state_mut(|state| {
                    if gold > state.py.misc.au {
                        state.py.misc.au = 0;
                    } else {
                        state.py.misc.au -= gold;
                    }
                });
                terminal::print_message(Some("Your purse feels lighter."));
                print_character_gold_value();
            }
            if random_number(2) == 1 {
                terminal::print_message(Some("There is a puff of smoke!"));
                spell_teleport_away_monster(monster_id, i32::from(monsters::MON_MAX_SIGHT));
            }
        }
        13 => {
            let (paralysis, dex, unique_items) = with_state(|state| {
                (
                    state.py.flags.paralysis,
                    i32::from(state.py.stats.used[PlayerAttr::A_DEX as usize]),
                    state.py.pack.unique_items,
                )
            });
            if paralysis < 1 && random_number(124) < dex {
                terminal::print_message(Some("You grab hold of your backpack!"));
            } else {
                inventory_destroy_item(random_number(i32::from(unique_items)) - 1);
                terminal::print_message(Some("Your backpack feels lighter."));
            }
            if random_number(2) == 1 {
                terminal::print_message(Some("There is a puff of smoke!"));
                spell_teleport_away_monster(monster_id, i32::from(monsters::MON_MAX_SIGHT));
            }
        }
        14 => {
            player_takes_hit(damage, death_description);
            terminal::print_message(Some("You feel very sick."));
            let addition = random_number(i32::from(creature_level)) + 5;
            with_state_mut(|state| {
                state.py.flags.poisoned += addition as i16;
            });
        }
        15 => {
            player_takes_hit(damage, death_description);
            if with_state(|state| state.py.flags.sustain_dex) {
                terminal::print_message(Some("You feel clumsy for a moment, but it passes."));
            } else {
                terminal::print_message(Some("You feel more clumsy."));
                let _ = player_stat_random_decrease(PlayerAttr::A_DEX);
            }
        }
        16 => {
            player_takes_hit(damage, death_description);
            if with_state(|state| state.py.flags.sustain_con) {
                terminal::print_message(Some("Your body resists the effects of the disease."));
            } else {
                terminal::print_message(Some("Your health is damaged!"));
                let _ = player_stat_random_decrease(PlayerAttr::A_CON);
            }
        }
        17 => {
            player_takes_hit(damage, death_description);
            terminal::print_message(Some("You have trouble thinking clearly."));
            if with_state(|state| state.py.flags.sustain_int) {
                terminal::print_message(Some("But your mind quickly clears."));
            } else {
                let _ = player_stat_random_decrease(PlayerAttr::A_INT);
            }
        }
        18 => {
            player_takes_hit(damage, death_description);
            if with_state(|state| state.py.flags.sustain_wis) {
                terminal::print_message(Some("Your wisdom is sustained."));
            } else {
                terminal::print_message(Some("Your wisdom is drained."));
                let _ = player_stat_random_decrease(PlayerAttr::A_WIS);
            }
        }
        19 => {
            terminal::print_message(Some("You feel your life draining away!"));
            let exp_drain = with_state(|state| {
                damage + (state.py.misc.exp / 100) * i32::from(MON_PLAYER_EXP_DRAINED_PER_HIT)
            });
            spell_lose_exp(exp_drain);
        }
        20 => {
            let _ = spell_aggravate_monsters(20);
        }
        21 if execute_disenchant_attack() => {
            terminal::print_message(Some("There is a static feeling in the air."));
            player_recalculate_bonuses();
        }
        21 => {
            noticed = false;
        }
        22 if inventory_find_range(
            i32::from(TV_FOOD),
            i32::from(TV_NEVER),
            &mut item_pos_start,
            &mut item_pos_end,
        ) =>
        {
            inventory_destroy_item(item_pos_start);
            terminal::print_message(Some("It got at your rations!"));
        }
        22 => {
            noticed = false;
        }
        23 => {
            noticed = inventory_diminish_light_attack(noticed);
        }
        24 => {
            noticed = inventory_diminish_charges_attack(creature_level, monster_hp, noticed);
        }
        _ => {
            noticed = false;
        }
    }

    noticed
}

/// C++ monster.cpp lines 384–475.
pub fn monster_attack_player(monster_id: i32) {
    if with_state(|state| state.game.character_is_dead) {
        return;
    }

    let creature_id = with_state(|state| state.monsters[monster_id as usize].creature_id);
    let creature = CREATURES_LIST[creature_id as usize];
    let lit = with_state(|state| state.monsters[monster_id as usize].lit);

    let mut name = [0u8; MORIA_MESSAGE_SIZE];
    if !lit {
        vtype_set_cstr(&mut name, "It ");
    } else {
        vtype_snprintf(&mut name, &format!("The {} ", creature.name));
    }

    let mut death_description = [0u8; MORIA_MESSAGE_SIZE];
    player_died_from_string(&mut death_description, creature.name, creature.movement);

    let mut attack_counter = 0i32;
    for damage_type_id in creature.damage {
        if damage_type_id == 0 || with_state(|state| state.game.character_is_dead) {
            break;
        }

        let attack = &MONSTER_ATTACKS[damage_type_id as usize];
        let mut attack_type = i32::from(attack.type_id);
        let mut attack_desc = i32::from(attack.description_id);
        let dice = attack.dice;

        let repelled = with_state(|state| {
            state.py.flags.protect_evil > 0
                && (creature.defenses & CD_EVIL) != 0
                && i32::from(state.py.misc.level) + 1 > i32::from(creature.level)
        });
        if repelled {
            if lit {
                with_state_mut(|state| {
                    state.creature_recall[creature_id as usize].defenses |= CD_EVIL;
                });
            }
            attack_type = 99;
            attack_desc = 99;
        }

        if player_test_attack_hits(attack_type, creature.level) {
            player_disturb(1, 0);

            let mut description = [0u8; MORIA_MESSAGE_SIZE];
            vtype_copy(&mut description, &name);
            monster_print_attack_description(&mut description, attack_desc);

            let visible = lit;
            let notice = visible;

            let damage = dice_roll(dice);
            let mut monster_hp = with_state(|state| state.monsters[monster_id as usize].hp);
            let notice = execute_attack_on_player(
                creature.level,
                &mut monster_hp,
                monster_id,
                attack_type,
                damage,
                &death_description,
                notice,
            );
            with_state_mut(|state| {
                state.monsters[monster_id as usize].hp = monster_hp;
            });

            let mut confused_amount =
                with_state(|state| state.monsters[monster_id as usize].confused_amount);
            monster_confuse_on_attack(
                &creature,
                &mut confused_amount,
                attack_desc,
                &name,
                visible,
                creature_id,
            );
            with_state_mut(|state| {
                state.monsters[monster_id as usize].confused_amount = confused_amount;
            });

            with_state_mut(|state| {
                let memory = &mut state.creature_recall[creature_id as usize];
                let prior = memory.attacks[attack_counter as usize];
                if (notice || (visible && prior != 0 && attack_type != 99))
                    && memory.attacks[attack_counter as usize] < u8::MAX
                {
                    memory.attacks[attack_counter as usize] += 1;
                }

                if state.game.character_is_dead && memory.deaths < i16::MAX as u16 {
                    memory.deaths += 1;
                }
            });
        } else if (1..=3).contains(&attack_desc) || attack_desc == 6 {
            player_disturb(1, 0);
            let mut description = [0u8; MORIA_MESSAGE_SIZE];
            vtype_copy(&mut description, &name);
            vtype_append_cstr(&mut description, "misses you.");
            terminal::print_message(Some(c_vtype_str(&description)));
        }

        if attack_counter < i32::from(MON_MAX_ATTACKS) - 1 {
            attack_counter += 1;
        } else {
            break;
        }
    }
}

/// C++ monster.cpp lines 674–687.
pub fn monster_can_cast_spells(monster: &Monster, spells: u32) -> bool {
    if random_number((spells & CS_FREQ) as i32) != 1 {
        return false;
    }

    let within_range = monster.distance_from_player <= monsters::MON_MAX_SPELL_CAST_DISTANCE;
    let unobstructed = with_state(|state| los(state.py.pos, monster.pos));

    within_range && unobstructed
}

/// C++ monster.cpp lines 689–844.
pub fn monster_execute_casting_of_spell(
    monster_id: i32,
    spell_id: i32,
    level: u8,
    monster_name: &mut Vtype_t,
    death_description: &Vtype_t,
) {
    match spell_id {
        5 => spell_teleport_away_monster(monster_id, 5),
        6 => spell_teleport_away_monster(monster_id, i32::from(monsters::MON_MAX_SIGHT)),
        7 => {
            let target = Coord_t {
                y: with_state(|state| state.monsters[monster_id as usize].pos.y),
                x: with_state(|state| state.monsters[monster_id as usize].pos.x),
            };
            spell_teleport_player_to(target);
        }
        8 => {
            if player_saving_throw() {
                terminal::print_message(Some("You resist the effects of the spell."));
            } else {
                player_takes_hit(dice_roll(Dice { dice: 3, sides: 8 }), death_description);
            }
        }
        9 => {
            if player_saving_throw() {
                terminal::print_message(Some("You resist the effects of the spell."));
            } else {
                player_takes_hit(dice_roll(Dice { dice: 8, sides: 8 }), death_description);
            }
        }
        10 => {
            if with_state(|state| state.py.flags.free_action) {
                terminal::print_message(Some("You are unaffected."));
            } else if player_saving_throw() {
                terminal::print_message(Some("You resist the effects of the spell."));
            } else if with_state(|state| state.py.flags.paralysis > 0) {
                with_state_mut(|state| {
                    state.py.flags.paralysis += 2;
                });
            } else {
                let amount = (random_number(5) + 4) as i16;
                with_state_mut(|state| {
                    state.py.flags.paralysis = amount;
                });
            }
        }
        11 => {
            if player_saving_throw() {
                terminal::print_message(Some("You resist the effects of the spell."));
            } else if with_state(|state| state.py.flags.blind > 0) {
                with_state_mut(|state| {
                    state.py.flags.blind += 6;
                });
            } else {
                let addition = 12 + random_number(3);
                with_state_mut(|state| {
                    state.py.flags.blind += addition as i16;
                });
            }
        }
        12 => {
            if player_saving_throw() {
                terminal::print_message(Some("You resist the effects of the spell."));
            } else if with_state(|state| state.py.flags.confused > 0) {
                with_state_mut(|state| {
                    state.py.flags.confused += 2;
                });
            } else {
                let amount = (random_number(5) + 3) as i16;
                with_state_mut(|state| {
                    state.py.flags.confused = amount;
                });
            }
        }
        13 => {
            if player_saving_throw() {
                terminal::print_message(Some("You resist the effects of the spell."));
            } else if with_state(|state| state.py.flags.afraid > 0) {
                with_state_mut(|state| {
                    state.py.flags.afraid += 2;
                });
            } else {
                let amount = (random_number(5) + 3) as i16;
                with_state_mut(|state| {
                    state.py.flags.afraid = amount;
                });
            }
        }
        14 => {
            vtype_append_cstr(monster_name, "magically summons a monster!");
            terminal::print_message(Some(c_vtype_str(monster_name)));
            let mut coord = with_state(|state| state.py.pos);

            with_state_mut(|state| {
                state.hack_monptr = monster_id;
            });
            let _ = monster_summon(&mut coord, false);
            with_state_mut(|state| {
                state.hack_monptr = -1;
            });
            let summoned_id =
                with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].creature_id);
            monster_update_visibility(summoned_id as i32);
        }
        15 => {
            vtype_append_cstr(monster_name, "magically summons an undead!");
            terminal::print_message(Some(c_vtype_str(monster_name)));
            let mut coord = with_state(|state| state.py.pos);

            with_state_mut(|state| {
                state.hack_monptr = monster_id;
            });
            let _ = monster_summon_undead(&mut coord);
            with_state_mut(|state| {
                state.hack_monptr = -1;
            });
            let summoned_id =
                with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].creature_id);
            monster_update_visibility(summoned_id as i32);
        }
        16 => {
            if with_state(|state| state.py.flags.free_action) {
                terminal::print_message(Some("You are unaffected."));
            } else if player_saving_throw() {
                terminal::print_message(Some("You resist the effects of the spell."));
            } else if with_state(|state| state.py.flags.slow > 0) {
                with_state_mut(|state| {
                    state.py.flags.slow += 2;
                });
            } else {
                let amount = (random_number(5) + 3) as i16;
                with_state_mut(|state| {
                    state.py.flags.slow = amount;
                });
            }
        }
        17 => {
            let current_mana = with_state(|state| state.py.misc.current_mana);
            if current_mana > 0 {
                player_disturb(1, 0);

                let name = c_vtype_str(monster_name);
                let mut msg = [0u8; MORIA_MESSAGE_SIZE];
                vtype_snprintf(&mut msg, &format!("{name}draws psychic energy from you!"));
                terminal::print_message(Some(c_vtype_str(&msg)));

                let lit = with_state(|state| state.monsters[monster_id as usize].lit);
                if lit {
                    vtype_snprintf(&mut msg, &format!("{name}appears healthier."));
                    terminal::print_message(Some(c_vtype_str(&msg)));
                }

                let mut num = (random_number(i32::from(level)) >> 1) + 1;
                with_state_mut(|state| {
                    if num > i32::from(state.py.misc.current_mana) {
                        num = i32::from(state.py.misc.current_mana);
                        state.py.misc.current_mana = 0;
                        state.py.misc.current_mana_fraction = 0;
                    } else {
                        state.py.misc.current_mana -= num as i16;
                    }
                    state.monsters[monster_id as usize].hp += (6 * num) as i16;
                });
                print_character_current_mana();
            }
        }
        20 => {
            vtype_append_cstr(monster_name, "breathes lightning.");
            terminal::print_message(Some(c_vtype_str(monster_name)));
            let (target, hp) =
                with_state(|state| (state.py.pos, state.monsters[monster_id as usize].hp));
            spell_breath(
                target,
                monster_id,
                i32::from(hp) / 4,
                MagicSpellFlags::Lightning,
                death_description,
            );
        }
        21 => {
            vtype_append_cstr(monster_name, "breathes gas.");
            terminal::print_message(Some(c_vtype_str(monster_name)));
            let (target, hp) =
                with_state(|state| (state.py.pos, state.monsters[monster_id as usize].hp));
            spell_breath(
                target,
                monster_id,
                i32::from(hp) / 3,
                MagicSpellFlags::PoisonGas,
                death_description,
            );
        }
        22 => {
            vtype_append_cstr(monster_name, "breathes acid.");
            terminal::print_message(Some(c_vtype_str(monster_name)));
            let (target, hp) =
                with_state(|state| (state.py.pos, state.monsters[monster_id as usize].hp));
            spell_breath(
                target,
                monster_id,
                i32::from(hp) / 3,
                MagicSpellFlags::Acid,
                death_description,
            );
        }
        23 => {
            vtype_append_cstr(monster_name, "breathes frost.");
            terminal::print_message(Some(c_vtype_str(monster_name)));
            let (target, hp) =
                with_state(|state| (state.py.pos, state.monsters[monster_id as usize].hp));
            spell_breath(
                target,
                monster_id,
                i32::from(hp) / 3,
                MagicSpellFlags::Frost,
                death_description,
            );
        }
        24 => {
            vtype_append_cstr(monster_name, "breathes fire.");
            terminal::print_message(Some(c_vtype_str(monster_name)));
            let (target, hp) =
                with_state(|state| (state.py.pos, state.monsters[monster_id as usize].hp));
            spell_breath(
                target,
                monster_id,
                i32::from(hp) / 3,
                MagicSpellFlags::Fire,
                death_description,
            );
        }
        _ => {
            vtype_append_cstr(monster_name, "cast unknown spell.");
            terminal::print_message(Some(c_vtype_str(monster_name)));
        }
    }
}

/// C++ monster.cpp lines 849–915.
pub fn monster_cast_spell(monster_id: i32) -> bool {
    if with_state(|state| state.game.character_is_dead) {
        return false;
    }

    let (creature_id, creature_spells, creature_name, creature_level, creature_movement) =
        with_state(|state| {
            let monster = &state.monsters[monster_id as usize];
            let creature = &CREATURES_LIST[monster.creature_id as usize];
            (
                monster.creature_id,
                creature.spells,
                creature.name,
                creature.level,
                creature.movement,
            )
        });

    let monster_snapshot = with_state(|state| state.monsters[monster_id as usize]);
    if !monster_can_cast_spells(&monster_snapshot, creature_spells) {
        return false;
    }

    monster_update_visibility(monster_id);

    let mut name = [0u8; MORIA_MESSAGE_SIZE];
    let lit = with_state(|state| state.monsters[monster_id as usize].lit);
    if lit {
        vtype_snprintf(&mut name, &format!("The {creature_name} "));
    } else {
        vtype_set_cstr(&mut name, "It ");
    }

    let mut death_description = [0u8; MORIA_MESSAGE_SIZE];
    player_died_from_string(&mut death_description, creature_name, creature_movement);

    let mut spell_flags = creature_spells & !CS_FREQ;
    let mut spell_choice = [0i32; 30];
    let mut id = 0;
    while spell_flags != 0 {
        spell_choice[id] = get_and_clear_first_bit(&mut spell_flags);
        id += 1;
    }

    let thrown_spell = spell_choice[(random_number(id as i32) - 1) as usize] + 1;

    if thrown_spell > 6 && thrown_spell != 17 {
        player_disturb(1, 0);
    }

    if (thrown_spell < 14 && thrown_spell > 6) || thrown_spell == 16 {
        vtype_append_cstr(&mut name, "casts a spell.");
        terminal::print_message(Some(c_vtype_str(&name)));
    }

    monster_execute_casting_of_spell(
        monster_id,
        thrown_spell,
        creature_level,
        &mut name,
        &death_description,
    );

    if lit {
        with_state_mut(|state| {
            let memory = &mut state.creature_recall[creature_id as usize];
            memory.spells |= 1u32 << (thrown_spell - 1);
            if (memory.spells & CS_FREQ) != CS_FREQ {
                memory.spells += 1;
            }
            if state.game.character_is_dead && memory.deaths < i16::MAX as u16 {
                memory.deaths += 1;
            }
        });
    }

    true
}

/// C++ monster.cpp lines 95–103.
fn monster_make_visible(coord: Coord_t) -> bool {
    let monster_id = with_state(|state| {
        i32::from(state.dg.floor[coord.y as usize][coord.x as usize].creature_id)
    });
    if monster_id <= 1 {
        return false;
    }

    monster_update_visibility(monster_id);
    with_state(|state| state.monsters[monster_id as usize].lit)
}

/// C++ monster.cpp lines 106–254.
pub fn monster_get_move_direction(monster_id: i32, directions: &mut [i32; 9]) {
    let (monster_pos, player_pos) =
        with_state(|state| (state.monsters[monster_id as usize].pos, state.py.pos));

    let y = monster_pos.y - player_pos.y;
    let x = monster_pos.x - player_pos.x;

    let (mut movement, ay) = if y < 0 { (8, -y) } else { (0, y) };
    let ax = if x > 0 {
        movement += 4;
        x
    } else {
        -x
    };

    let movement = if ay > (ax << 1) {
        movement + 2
    } else if ax > (ay << 1) {
        movement + 1
    } else {
        movement
    };

    match movement {
        0 => {
            directions[0] = 9;
            if ay > ax {
                directions[1] = 8;
                directions[2] = 6;
                directions[3] = 7;
                directions[4] = 3;
            } else {
                directions[1] = 6;
                directions[2] = 8;
                directions[3] = 3;
                directions[4] = 7;
            }
        }
        1 | 9 => {
            directions[0] = 6;
            if y < 0 {
                directions[1] = 3;
                directions[2] = 9;
                directions[3] = 2;
                directions[4] = 8;
            } else {
                directions[1] = 9;
                directions[2] = 3;
                directions[3] = 8;
                directions[4] = 2;
            }
        }
        2 | 6 => {
            directions[0] = 8;
            if x < 0 {
                directions[1] = 9;
                directions[2] = 7;
                directions[3] = 6;
                directions[4] = 4;
            } else {
                directions[1] = 7;
                directions[2] = 9;
                directions[3] = 4;
                directions[4] = 6;
            }
        }
        4 => {
            directions[0] = 7;
            if ay > ax {
                directions[1] = 8;
                directions[2] = 4;
                directions[3] = 9;
                directions[4] = 1;
            } else {
                directions[1] = 4;
                directions[2] = 8;
                directions[3] = 1;
                directions[4] = 9;
            }
        }
        5 | 13 => {
            directions[0] = 4;
            if y < 0 {
                directions[1] = 1;
                directions[2] = 7;
                directions[3] = 2;
                directions[4] = 8;
            } else {
                directions[1] = 7;
                directions[2] = 1;
                directions[3] = 8;
                directions[4] = 2;
            }
        }
        8 => {
            directions[0] = 3;
            if ay > ax {
                directions[1] = 2;
                directions[2] = 6;
                directions[3] = 1;
                directions[4] = 9;
            } else {
                directions[1] = 6;
                directions[2] = 2;
                directions[3] = 9;
                directions[4] = 1;
            }
        }
        10 | 14 => {
            directions[0] = 2;
            if x < 0 {
                directions[1] = 3;
                directions[2] = 1;
                directions[3] = 6;
                directions[4] = 4;
            } else {
                directions[1] = 1;
                directions[2] = 3;
                directions[3] = 4;
                directions[4] = 6;
            }
        }
        12 => {
            directions[0] = 1;
            if ay > ax {
                directions[1] = 2;
                directions[2] = 4;
                directions[3] = 3;
                directions[4] = 7;
            } else {
                directions[1] = 4;
                directions[2] = 2;
                directions[3] = 7;
                directions[4] = 3;
            }
        }
        _ => {}
    }
}

/// C++ monster.cpp lines 477–540.
pub fn monster_open_door(
    coord: Coord_t,
    monster_hp: i16,
    move_bits: u32,
    do_turn: &mut bool,
    do_move: &mut bool,
    rcmove: &mut u32,
) {
    let mut lite_spot = false;
    let mut stuck_message = false;
    let mut bash_message = false;
    let mut disturb = false;

    with_state_mut(|state| {
        let treasure_id = state.dg.floor[coord.y as usize][coord.x as usize].treasure_id;
        let (category_id, misc_use) = {
            let item = &state.game.treasure.list[treasure_id as usize];
            (item.category_id, item.misc_use)
        };

        if (move_bits & CM_OPEN_DOOR) != 0 {
            let mut door_is_stuck = false;

            if category_id == TV_CLOSED_DOOR {
                *do_turn = true;

                if misc_use == 0 {
                    *do_move = true;
                } else if misc_use > 0 {
                    let lhs = (i32::from(monster_hp) + 1) * (50 + i32::from(misc_use));
                    let rhs = 40 * (i32::from(monster_hp) - 10 - i32::from(misc_use));
                    if random_number_state(state, lhs) < rhs {
                        state.game.treasure.list[treasure_id as usize].misc_use = 0;
                    }
                } else if misc_use < 0 {
                    let lhs = (i32::from(monster_hp) + 1) * (50 - i32::from(misc_use));
                    let rhs = 40 * (i32::from(monster_hp) - 10 + i32::from(misc_use));
                    if random_number_state(state, lhs) < rhs {
                        stuck_message = true;
                        disturb = true;
                        door_is_stuck = true;
                        *do_move = true;
                    }
                }
            } else if category_id == TV_SECRET_DOOR {
                *do_turn = true;
                *do_move = true;
            }

            if *do_move {
                inventory_item_copy_to(
                    OBJ_OPEN_DOOR as i16,
                    &mut state.game.treasure.list[treasure_id as usize],
                );

                if door_is_stuck {
                    state.game.treasure.list[treasure_id as usize].misc_use =
                        (1 - random_number_state(state, 2)) as i16;
                }
                state.dg.floor[coord.y as usize][coord.x as usize].feature_id = TILE_CORR_FLOOR;
                lite_spot = true;
                *rcmove |= CM_OPEN_DOOR;
                *do_move = false;
            }
        } else if category_id == TV_CLOSED_DOOR {
            *do_turn = true;

            let abs_misc_use = i32::from(misc_use.unsigned_abs());
            let lhs = (i32::from(monster_hp) + 1) * (80 + abs_misc_use);
            let rhs = 40 * (i32::from(monster_hp) - 20 - abs_misc_use);
            if random_number_state(state, lhs) < rhs {
                inventory_item_copy_to(
                    OBJ_OPEN_DOOR as i16,
                    &mut state.game.treasure.list[treasure_id as usize],
                );
                state.game.treasure.list[treasure_id as usize].misc_use =
                    (1 - random_number_state(state, 2)) as i16;
                state.dg.floor[coord.y as usize][coord.x as usize].feature_id = TILE_CORR_FLOOR;
                lite_spot = true;
                bash_message = true;
                disturb = true;
            }
        }
    });

    if stuck_message {
        terminal::print_message(Some("You hear a door burst open!"));
    }
    if bash_message {
        terminal::print_message(Some("You hear a door burst open!"));
    }
    if disturb {
        player_disturb(1, 0);
    }
    if lite_spot {
        dungeon_lite_spot(coord);
    }
}

/// C++ monster.cpp lines 542–558.
pub fn glyph_of_warding_protection(
    creature_id: u16,
    move_bits: u32,
    do_move: &mut bool,
    do_turn: &mut bool,
    coord: Coord_t,
) {
    if random_number(i32::from(OBJECTS_RUNE_PROTECTION))
        < i32::from(CREATURES_LIST[creature_id as usize].level)
    {
        let on_player = with_state(|state| coord.y == state.py.pos.y && coord.x == state.py.pos.x);
        if on_player {
            terminal::print_message(Some("The rune of protection is broken!"));
        }
        let _ = dungeon_delete_object(coord);
        return;
    }

    *do_move = false;

    if (move_bits & CM_ATTACK_ONLY) != 0 {
        *do_turn = true;
    }
}

/// C++ monster.cpp lines 560–594.
pub fn monster_moves_on_player(
    monster_id: i32,
    tile_creature_id: u8,
    move_bits: u32,
    do_move: &mut bool,
    do_turn: &mut bool,
    rcmove: &mut u32,
    coord: Coord_t,
) {
    if tile_creature_id == 1 {
        let lit = with_state(|state| state.monsters[monster_id as usize].lit);
        if !lit {
            monster_update_visibility(monster_id);
        }
        monster_attack_player(monster_id);
        *do_move = false;
        *do_turn = true;
    } else if i32::from(tile_creature_id) > 1 {
        let (monster_creature_id, monster_pos, other_lit) = with_state(|state| {
            let monster = &state.monsters[monster_id as usize];
            (
                monster.creature_id,
                monster.pos,
                state.monsters[tile_creature_id as usize].lit,
            )
        });

        if coord.y != monster_pos.y || coord.x != monster_pos.x {
            let eater = &CREATURES_LIST[monster_creature_id as usize];
            let eaten_creature_id =
                with_state(|state| state.monsters[tile_creature_id as usize].creature_id);
            let eaten = &CREATURES_LIST[eaten_creature_id as usize];

            if (move_bits & CM_EATS_OTHER) != 0 && eater.kill_exp_value >= eaten.kill_exp_value {
                if other_lit {
                    *rcmove |= CM_EATS_OTHER;
                }

                if monster_id < i32::from(tile_creature_id) {
                    dungeon_delete_monster(i32::from(tile_creature_id));
                } else {
                    dungeon_remove_monster_from_level(i32::from(tile_creature_id));
                }
            } else {
                *do_move = false;
            }
        }
    }
}

/// C++ monster.cpp lines 596–620.
pub fn monster_allowed_to_move(
    monster_id: i32,
    move_bits: u32,
    do_turn: &mut bool,
    rcmove: &mut u32,
    coord: Coord_t,
) {
    let old_pos = with_state(|state| state.monsters[monster_id as usize].pos);

    if (move_bits & CM_PICKS_UP) != 0 {
        let treasure_id =
            with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].treasure_id);
        let picks_up = with_state(|state| {
            treasure_id != 0
                && state.game.treasure.list[treasure_id as usize].category_id <= TV_MAX_OBJECT
        });
        if picks_up {
            *rcmove |= CM_PICKS_UP;
            let _ = dungeon_delete_object(coord);
        }
    }

    dungeon_move_creature_record(old_pos, coord);

    let was_lit = with_state_mut(|state| {
        let monster = &mut state.monsters[monster_id as usize];
        let lit = monster.lit;
        if lit {
            monster.lit = false;
        }
        lit
    });
    if was_lit {
        dungeon_lite_spot(old_pos);
    }

    with_state_mut(|state| {
        let monster = &mut state.monsters[monster_id as usize];
        monster.pos = coord;
        monster.distance_from_player = coord_distance_between(state.py.pos, coord) as u8;
    });

    *do_turn = true;
}

/// C++ monster.cpp lines 623–672.
pub fn make_move(monster_id: i32, directions: &[i32; 9], rcmove: &mut u32) {
    let move_bits = with_state(|state| {
        let creature_id = state.monsters[monster_id as usize].creature_id;
        CREATURES_LIST[creature_id as usize].movement
    });

    let mut do_turn = false;
    let mut do_move = false;
    let mut coord;

    for direction in &directions[..5] {
        if do_turn {
            break;
        }

        coord = with_state(|state| state.monsters[monster_id as usize].pos);

        let _ = player_move_position(*direction, &mut coord);

        let (feature_id, treasure_id, tile_creature_id) = with_state(|state| {
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            (tile.feature_id, tile.treasure_id, tile.creature_id)
        });

        if feature_id == TILE_BOUNDARY_WALL {
            continue;
        }

        if feature_id <= MAX_OPEN_SPACE {
            do_move = true;
        } else if (move_bits & CM_PHASE) != 0 {
            do_move = true;
            *rcmove |= CM_PHASE;
        } else if treasure_id != 0 {
            monster_open_door(
                coord,
                with_state(|state| state.monsters[monster_id as usize].hp),
                move_bits,
                &mut do_turn,
                &mut do_move,
                rcmove,
            );
        }

        if do_move {
            let is_glyph = with_state(|state| {
                treasure_id != 0
                    && state.game.treasure.list[treasure_id as usize].category_id == TV_VIS_TRAP
                    && state.game.treasure.list[treasure_id as usize].sub_category_id == 99
            });
            if is_glyph {
                let creature_id =
                    with_state(|state| state.monsters[monster_id as usize].creature_id);
                glyph_of_warding_protection(
                    creature_id,
                    move_bits,
                    &mut do_move,
                    &mut do_turn,
                    coord,
                );
            }
        }

        if do_move {
            monster_moves_on_player(
                monster_id,
                tile_creature_id,
                move_bits,
                &mut do_move,
                &mut do_turn,
                rcmove,
                coord,
            );
        }

        if do_move {
            monster_allowed_to_move(monster_id, move_bits, &mut do_turn, rcmove, coord);
        }
    }
}

/// C++ monster.cpp lines 919–983.
pub fn monster_multiply(coord: Coord_t, creature_id: i32, monster_id: i32) -> bool {
    let mut position = Coord_t { y: 0, x: 0 };

    for _ in 0..=18 {
        position.y = coord.y - 2 + random_number(3);
        position.x = coord.x - 2 + random_number(3);

        if coord_in_bounds(position) && (position.y != coord.y || position.x != coord.x) {
            let (feature_id, treasure_id, tile_creature_id) = with_state(|state| {
                let tile = &state.dg.floor[position.y as usize][position.x as usize];
                (tile.feature_id, tile.treasure_id, tile.creature_id)
            });

            if feature_id <= MAX_OPEN_SPACE && treasure_id == 0 && tile_creature_id != 1 {
                if tile_creature_id > 1 {
                    let cannibalistic =
                        (CREATURES_LIST[creature_id as usize].movement & CM_EATS_OTHER) != 0;
                    let eaten_creature_id =
                        with_state(|state| state.monsters[tile_creature_id as usize].creature_id);
                    let experienced = CREATURES_LIST[creature_id as usize].kill_exp_value
                        >= CREATURES_LIST[eaten_creature_id as usize].kill_exp_value;

                    if cannibalistic && experienced {
                        if monster_id < i32::from(tile_creature_id) {
                            dungeon_delete_monster(i32::from(tile_creature_id));
                        } else {
                            dungeon_remove_monster_from_level(i32::from(tile_creature_id));
                        }

                        with_state_mut(|state| state.hack_monptr = monster_id);
                        let result = monster_place_new(position, creature_id, false);
                        with_state_mut(|state| state.hack_monptr = -1);
                        if !result {
                            return false;
                        }

                        with_state_mut(|state| state.monster_multiply_total += 1);
                        return monster_make_visible(position);
                    }
                } else {
                    with_state_mut(|state| state.hack_monptr = monster_id);
                    let result = monster_place_new(position, creature_id, false);
                    with_state_mut(|state| state.hack_monptr = -1);
                    if !result {
                        return false;
                    }

                    with_state_mut(|state| state.monster_multiply_total += 1);
                    return monster_make_visible(position);
                }
            }
        }
    }

    false
}

/// C++ monster.cpp lines 985–1009.
pub fn monster_multiply_critter(monster_id: i32, rcmove: &mut u32) {
    let pos = with_state(|state| state.monsters[monster_id as usize].pos);
    let mut counter = 0;

    for y in pos.y - 1..=pos.y + 1 {
        for x in pos.x - 1..=pos.x + 1 {
            let at = Coord_t { y, x };
            if coord_in_bounds(at)
                && with_state(|state| state.dg.floor[y as usize][x as usize].creature_id) > 1
            {
                counter += 1;
            }
        }
    }

    if counter == 0 {
        counter += 1;
    }

    if counter < 4 && random_number(counter * i32::from(MON_MULTIPLY_ADJUST)) == 1 {
        let creature_id = with_state(|state| state.monsters[monster_id as usize].creature_id);
        if monster_multiply(pos, i32::from(creature_id), monster_id) {
            *rcmove |= CM_MULTIPLY;
        }
    }
}

/// C++ monster.cpp lines 1011–1067.
pub fn monster_move_out_of_wall(monster_id: i32, rcmove: &mut u32) {
    let hp = with_state(|state| state.monsters[monster_id as usize].hp);
    if hp < 0 {
        return;
    }

    let monster_pos = with_state(|state| state.monsters[monster_id as usize].pos);
    let mut id = 0usize;
    let mut dir = 1i32;
    let mut directions = [0i32; 9];

    for y in (monster_pos.y - 1..=monster_pos.y + 1).rev() {
        for x in monster_pos.x - 1..=monster_pos.x + 1 {
            let feature_id = with_state(|state| state.dg.floor[y as usize][x as usize].feature_id);
            let creature_id =
                with_state(|state| state.dg.floor[y as usize][x as usize].creature_id);
            if dir != 5 && feature_id <= MAX_OPEN_SPACE && creature_id != 1 {
                directions[id] = dir;
                id += 1;
            }
            dir += 1;
        }
    }

    if id != 0 {
        let pick = (random_number(id as i32) - 1) as usize;
        directions.swap(0, pick);
        make_move(monster_id, &directions, rcmove);
    }

    let still_in_wall = with_state(|state| {
        let pos = state.monsters[monster_id as usize].pos;
        state.dg.floor[pos.y as usize][pos.x as usize].feature_id >= MIN_CAVE_WALL
    });

    if still_in_wall {
        with_state_mut(|state| state.hack_monptr = monster_id);
        let result = monster_take_hit(monster_id, dice_roll(Dice { dice: 8, sides: 8 }));
        with_state_mut(|state| state.hack_monptr = -1);

        if result >= 0 {
            terminal::print_message(Some("You hear a scream muffled by rock!"));
            display_character_experience();
        } else {
            terminal::print_message(Some("A creature digs itself out from the rock!"));
            let pos = with_state(|state| state.monsters[monster_id as usize].pos);
            let _ = player_tunnel_wall(pos, 1, 0);
        }
    }
}

/// C++ monster.cpp lines 1070–1084.
pub fn monster_move_undead(creature: &Creature, monster_id: i32, rcmove: &mut u32) {
    let mut directions = [0i32; 9];
    monster_get_move_direction(monster_id, &mut directions);

    directions[0] = 10 - directions[0];
    directions[1] = 10 - directions[1];
    directions[2] = 10 - directions[2];
    directions[3] = random_number(9);
    directions[4] = random_number(9);

    if (creature.movement & CM_ATTACK_ONLY) == 0 {
        make_move(monster_id, &directions, rcmove);
    }
}

/// C++ monster.cpp lines 1086–1099.
pub fn monster_move_confused(creature: &Creature, monster_id: i32, rcmove: &mut u32) {
    let mut directions = [0i32; 9];

    directions[0] = random_number(9);
    directions[1] = random_number(9);
    directions[2] = random_number(9);
    directions[3] = random_number(9);
    directions[4] = random_number(9);

    if (creature.movement & CM_ATTACK_ONLY) == 0 {
        make_move(monster_id, &directions, rcmove);
    }
}

/// C++ monster.cpp lines 1101–1119.
pub fn monster_do_move(monster_id: i32, rcmove: &mut u32) -> bool {
    let (confused_amount, creature_id) = with_state(|state| {
        let monster = &state.monsters[monster_id as usize];
        (monster.confused_amount, monster.creature_id)
    });
    let creature = CREATURES_LIST[creature_id as usize];

    if confused_amount != 0 {
        if (creature.defenses & CD_UNDEAD) != 0 {
            monster_move_undead(&creature, monster_id, rcmove);
        } else {
            monster_move_confused(&creature, monster_id, rcmove);
        }
        with_state_mut(|state| {
            state.monsters[monster_id as usize].confused_amount -= 1;
        });
        return true;
    }

    if (creature.spells & CS_FREQ) != 0 {
        return monster_cast_spell(monster_id);
    }

    false
}

/// C++ monster.cpp lines 1121–1133.
pub fn monster_move_randomly(monster_id: i32, rcmove: &mut u32, randomness: u32) {
    let mut directions = [0i32; 9];

    directions[0] = random_number(9);
    directions[1] = random_number(9);
    directions[2] = random_number(9);
    directions[3] = random_number(9);
    directions[4] = random_number(9);

    *rcmove |= randomness;
    make_move(monster_id, &directions, rcmove);
}

/// C++ monster.cpp lines 1135–1151.
pub fn monster_move_normally(monster_id: i32, rcmove: &mut u32) {
    let mut directions = [0i32; 9];

    if random_number(200) == 1 {
        directions[0] = random_number(9);
        directions[1] = random_number(9);
        directions[2] = random_number(9);
        directions[3] = random_number(9);
        directions[4] = random_number(9);
    } else {
        monster_get_move_direction(monster_id, &mut directions);
    }

    *rcmove |= CM_MOVE_NORMAL;
    make_move(monster_id, &directions, rcmove);
}

/// C++ monster.cpp lines 1153–1164.
pub fn monster_attack_without_moving(monster_id: i32, rcmove: &mut u32, distance_from_player: u8) {
    let mut directions = [0i32; 9];

    if distance_from_player < 2 {
        monster_get_move_direction(monster_id, &mut directions);
        make_move(monster_id, &directions, rcmove);
    } else {
        *rcmove |= CM_ATTACK_ONLY;
    }
}

/// C++ monster.cpp lines 1167–1233.
pub fn monster_move(monster_id: i32, rcmove: &mut u32) {
    let (creature_id, distance_from_player, monster_pos) = with_state(|state| {
        let monster = &state.monsters[monster_id as usize];
        (
            monster.creature_id,
            monster.distance_from_player,
            monster.pos,
        )
    });
    let creature = CREATURES_LIST[creature_id as usize];

    let abs_rest_period = i32::from(with_state(|state| state.py.flags.rest).unsigned_abs());
    if (creature.movement & CM_MULTIPLY) != 0
        && i32::from(MON_MAX_MULTIPLY_PER_LEVEL)
            >= i32::from(with_state(|state| state.monster_multiply_total))
        && (abs_rest_period % i32::from(MON_MULTIPLY_ADJUST)) == 0
    {
        monster_multiply_critter(monster_id, rcmove);
    }

    if (creature.movement & CM_PHASE) == 0
        && with_state(|state| {
            state.dg.floor[monster_pos.y as usize][monster_pos.x as usize].feature_id
        }) >= MIN_CAVE_WALL
    {
        monster_move_out_of_wall(monster_id, rcmove);
        return;
    }

    if monster_do_move(monster_id, rcmove) {
        return;
    }

    if (creature.movement & CM_75_RANDOM) != 0 && random_number(100) < 75 {
        monster_move_randomly(monster_id, rcmove, CM_75_RANDOM);
        return;
    }

    if (creature.movement & CM_40_RANDOM) != 0 && random_number(100) < 40 {
        monster_move_randomly(monster_id, rcmove, CM_40_RANDOM);
        return;
    }

    if (creature.movement & CM_20_RANDOM) != 0 && random_number(100) < 20 {
        monster_move_randomly(monster_id, rcmove, CM_20_RANDOM);
        return;
    }

    if (creature.movement & CM_MOVE_NORMAL) != 0 {
        monster_move_normally(monster_id, rcmove);
        return;
    }

    if (creature.movement & CM_ATTACK_ONLY) != 0 {
        monster_attack_without_moving(monster_id, rcmove, distance_from_player);
        return;
    }

    if (creature.movement & CM_ONLY_MAGIC) != 0 && distance_from_player < 2 {
        with_state_mut(|state| {
            let memory = &mut state.creature_recall[creature_id as usize];
            memory.attacks[0] = memory.attacks[0].saturating_add(1);
            if memory.attacks[0] > 20 {
                memory.movement |= CM_ONLY_MAGIC;
            }
        });
    }
}
