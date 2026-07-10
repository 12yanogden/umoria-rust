//! Phase 4.2.4 — monster spellcasting parity.
#![allow(clippy::int_plus_one)]

mod common;

use umoria::config::monsters::spells::CS_FREQ;
use umoria::config::monsters::{self, MON_MAX_SPELL_CAST_DISTANCE};
use umoria::data_creatures::CREATURES_LIST;
use umoria::dungeon::{MAX_HEIGHT, MAX_WIDTH};
use umoria::dungeon_los::los;
use umoria::dungeon_tile::TILE_LIGHT_FLOOR;
use umoria::game::{random_number, reset_for_new_game, with_state, with_state_mut};
use umoria::helpers::get_and_clear_first_bit;
use umoria::monster::{
    monster_can_cast_spells, monster_cast_spell, monster_execute_casting_of_spell, Monster,
    MON_TOTAL_ALLOCATIONS,
};
use umoria::player::PlayerAttr;
use umoria::types::{Coord_t, Vtype_t, MORIA_MESSAGE_SIZE};
use umoria::ui::panel_bounds_fields;
use umoria::ui_io::test_set_ncurses_stub;

const FLOATING_EYE_ID: u16 = 18;
const NOVICE_PRIEST_ID: u16 = 24;
const POLTERGEIST_ID: u16 = 34;
const BLUE_JELLY_ID: u16 = 51;
const LOST_SOUL_ID: u16 = 87;
const PRIEST_ID: u16 = 121;
const ORC_SHAMAN_ID: u16 = 102;

fn setup_dungeon(height: i16, width: i16) {
    with_state_mut(|s| {
        s.dg.height = height;
        s.dg.width = width;
        s.dg.floor = [[Default::default(); MAX_WIDTH as usize]; MAX_HEIGHT as usize];
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

fn setup_player(pos: Coord_t) {
    test_set_ncurses_stub(true);
    let bounds = panel_bounds_fields(0, 0);
    with_state_mut(|s| {
        s.py.pos = pos;
        // Breath / area effects hit the player only when the floor tile is marked.
        s.dg.floor[pos.y as usize][pos.x as usize].creature_id = 1;
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
        s.py.misc.current_mana = 100;
        s.py.misc.current_mana_fraction = 0;
        s.py.misc.saving_throw = -50;
        s.py.misc.level = 10;
        s.py.misc.class_id = 0;
        s.py.stats.used[PlayerAttr::A_WIS as usize] = 10;
        s.py.flags.paralysis = 0;
        s.py.flags.blind = 0;
        s.py.flags.confused = 0;
        s.py.flags.afraid = 0;
        s.py.flags.slow = 0;
        s.py.flags.free_action = false;
        s.game.character_is_dead = false;
        s.game.command_count = 5;
        s.message_ready_to_print = false;
    });
}

fn place_monster(id: i32, creature_id: u16, hp: i16, coord: Coord_t, lit: bool, distance: u8) {
    with_state_mut(|s| {
        s.monsters[id as usize] = Monster {
            hp,
            sleep_count: 99,
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

fn message_text(id: i16) -> String {
    with_state(|s| {
        let idx = id.rem_euclid(umoria::types::MESSAGE_HISTORY_SIZE as i16) as usize;
        let msg = &s.messages[idx];
        let end = msg.iter().position(|&b| b == 0).unwrap_or(msg.len());
        String::from_utf8_lossy(&msg[..end]).into_owned()
    })
}

fn vtype_from_str(text: &str) -> Vtype_t {
    let mut buf = [0u8; MORIA_MESSAGE_SIZE];
    let bytes = text.as_bytes();
    let n = bytes.len().min(MORIA_MESSAGE_SIZE - 1);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf
}

fn death_description_for(creature_id: u16) -> Vtype_t {
    let creature = &CREATURES_LIST[creature_id as usize];
    let mut desc = [0u8; MORIA_MESSAGE_SIZE];
    umoria::player::player_died_from_string(&mut desc, creature.name, creature.movement);
    desc
}

// ---------------------------------------------------------------------------
// 1. monsterCanCastSpells gate parity
// ---------------------------------------------------------------------------
#[test]
fn monster_can_cast_spells_freq_gate_fails_without_extra_rng_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    let monster = Monster {
        pos: Coord_t { y: 10, x: 12 },
        distance_from_player: 2,
        ..Default::default()
    };
    let spells = CREATURES_LIST[FLOATING_EYE_ID as usize].spells;
    assert_eq!(spells & CS_FREQ, 13);
    assert!(!monster_can_cast_spells(&monster, spells));
    assert_eq!(next_random_pair(13), (13, 6));
}

#[test]
fn monster_can_cast_spells_freq_gate_passes_with_los_seed1000() {
    reset_for_new_game(Some(1000));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    let monster = Monster {
        pos: Coord_t { y: 10, x: 12 },
        distance_from_player: 2,
        ..Default::default()
    };
    let spells = CREATURES_LIST[FLOATING_EYE_ID as usize].spells;
    assert!(monster_can_cast_spells(&monster, spells));
    assert!(los(Coord_t { y: 10, x: 10 }, monster.pos));
    assert_eq!(next_random_pair(13), (13, 4));
}

#[test]
fn monster_can_cast_spells_out_of_range_fails_after_freq_roll_seed1000() {
    reset_for_new_game(Some(1000));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    let monster = Monster {
        pos: Coord_t { y: 10, x: 12 },
        distance_from_player: MON_MAX_SPELL_CAST_DISTANCE + 1,
        ..Default::default()
    };
    let spells = CREATURES_LIST[FLOATING_EYE_ID as usize].spells;
    assert!(!monster_can_cast_spells(&monster, spells));
    assert_eq!(next_random_pair(13), (13, 4));
}

// ---------------------------------------------------------------------------
// 2. Spell selection parity
// ---------------------------------------------------------------------------
#[test]
fn spell_choice_bit_order_matches_get_and_clear_first_bit() {
    let mut flags = CREATURES_LIST[LOST_SOUL_ID as usize].spells & !CS_FREQ;
    let mut spell_choice = [0i32; 30];
    let mut id = 0;
    while flags != 0 {
        spell_choice[id] = get_and_clear_first_bit(&mut flags);
        id += 1;
    }
    assert_eq!(id, 2);
    assert_eq!(spell_choice[0], 5);
    assert_eq!(spell_choice[1], 16);
}

#[test]
fn monster_cast_spell_lost_soul_freq_and_selection_seed14() {
    reset_for_new_game(Some(14));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(2, LOST_SOUL_ID, 20, Coord_t { y: 10, x: 11 }, true, 1);
    assert!(monster_cast_spell(2));
    // seed14: freq=1, selection=1 → Teleport Long; teleport consumes placement rolls,
    // then the next draws are rn(15)=1 and rn(2)=1 (C++-matching stream).
    assert_eq!(next_random_pair(15), (15, 1));
    assert_eq!(next_random_pair(2), (2, 1));
}

// ---------------------------------------------------------------------------
// 3. monsterExecuteCastingOfSpell per-spell parity
// ---------------------------------------------------------------------------
#[test]
fn execute_spell_light_wound_dice_after_failed_save_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(2, NOVICE_PRIEST_ID, 20, Coord_t { y: 10, x: 11 }, true, 1);
    let creature = &CREATURES_LIST[NOVICE_PRIEST_ID as usize];
    let mut name = vtype_from_str("The Novice Priest ");
    let death = death_description_for(NOVICE_PRIEST_ID);
    monster_execute_casting_of_spell(2, 8, creature.level, &mut name, &death);
    with_state(|s| assert_eq!(s.py.misc.current_hp, 493));
    assert_eq!(next_random_pair(100), (100, 57));
}

#[test]
fn execute_spell_serious_wound_resists_on_save_seed777() {
    reset_for_new_game(Some(777));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    with_state_mut(|s| s.py.misc.saving_throw = 100);
    reset_monster_slots();
    place_monster(2, NOVICE_PRIEST_ID, 20, Coord_t { y: 10, x: 11 }, true, 1);
    let creature = &CREATURES_LIST[NOVICE_PRIEST_ID as usize];
    let mut name = vtype_from_str("The Novice Priest ");
    let death = death_description_for(NOVICE_PRIEST_ID);
    monster_execute_casting_of_spell(2, 9, creature.level, &mut name, &death);
    assert_eq!(next_random_pair(100), (100, 29));
    with_state(|s| {
        assert_eq!(
            message_text(s.last_message_id),
            "You resist the effects of the spell."
        );
        assert_eq!(s.py.misc.current_hp, 500);
    });
}

#[test]
fn execute_spell_paralyze_duration_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(2, NOVICE_PRIEST_ID, 20, Coord_t { y: 10, x: 11 }, true, 1);
    let creature = &CREATURES_LIST[NOVICE_PRIEST_ID as usize];
    let mut name = vtype_from_str("It ");
    let death = death_description_for(NOVICE_PRIEST_ID);
    monster_execute_casting_of_spell(2, 10, creature.level, &mut name, &death);
    with_state(|s| assert_eq!(s.py.flags.paralysis, 7));
    assert_eq!(next_random_pair(100), (100, 36));
    assert_eq!(next_random_pair(5), (5, 2));
}

#[test]
fn execute_spell_blind_duration_seed100() {
    reset_for_new_game(Some(100));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(2, NOVICE_PRIEST_ID, 20, Coord_t { y: 10, x: 11 }, true, 1);
    let creature = &CREATURES_LIST[NOVICE_PRIEST_ID as usize];
    let mut name = vtype_from_str("It ");
    let death = death_description_for(NOVICE_PRIEST_ID);
    monster_execute_casting_of_spell(2, 11, creature.level, &mut name, &death);
    with_state(|s| assert_eq!(s.py.flags.blind, 14));
    assert_eq!(next_random_pair(100), (100, 2));
    assert_eq!(next_random_pair(3), (3, 1));
}

#[test]
fn execute_spell_confuse_duration_seed200() {
    reset_for_new_game(Some(200));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(2, NOVICE_PRIEST_ID, 20, Coord_t { y: 10, x: 11 }, true, 1);
    let creature = &CREATURES_LIST[NOVICE_PRIEST_ID as usize];
    let mut name = vtype_from_str("It ");
    let death = death_description_for(NOVICE_PRIEST_ID);
    monster_execute_casting_of_spell(2, 12, creature.level, &mut name, &death);
    with_state(|s| assert_eq!(s.py.flags.confused, 6));
    assert_eq!(next_random_pair(100), (100, 77));
    assert_eq!(next_random_pair(5), (5, 5));
}

#[test]
fn execute_spell_fear_duration_seed300() {
    reset_for_new_game(Some(300));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(2, NOVICE_PRIEST_ID, 20, Coord_t { y: 10, x: 11 }, true, 1);
    let creature = &CREATURES_LIST[NOVICE_PRIEST_ID as usize];
    let mut name = vtype_from_str("It ");
    let death = death_description_for(NOVICE_PRIEST_ID);
    monster_execute_casting_of_spell(2, 13, creature.level, &mut name, &death);
    with_state(|s| assert_eq!(s.py.flags.afraid, 5));
    assert_eq!(next_random_pair(100), (100, 5));
    assert_eq!(next_random_pair(5), (5, 3));
}

#[test]
fn execute_spell_slow_duration_seed400() {
    reset_for_new_game(Some(400));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(2, NOVICE_PRIEST_ID, 20, Coord_t { y: 10, x: 11 }, true, 1);
    let creature = &CREATURES_LIST[NOVICE_PRIEST_ID as usize];
    let mut name = vtype_from_str("It ");
    let death = death_description_for(NOVICE_PRIEST_ID);
    monster_execute_casting_of_spell(2, 16, creature.level, &mut name, &death);
    with_state(|s| assert_eq!(s.py.flags.slow, 4));
    assert_eq!(next_random_pair(100), (100, 80));
    assert_eq!(next_random_pair(5), (5, 3));
}

#[test]
fn execute_spell_drain_mana_seed500() {
    reset_for_new_game(Some(500));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(2, FLOATING_EYE_ID, 30, Coord_t { y: 10, x: 11 }, true, 1);
    let creature = &CREATURES_LIST[FLOATING_EYE_ID as usize];
    let mut name = vtype_from_str("The Floating Eye ");
    let death = death_description_for(FLOATING_EYE_ID);
    monster_execute_casting_of_spell(2, 17, creature.level, &mut name, &death);
    assert_eq!(next_random_pair(1), (1, 1));
    with_state(|s| {
        assert_eq!(s.py.misc.current_mana, 99);
        assert_eq!(s.monsters[2].hp, 36);
        assert_eq!(s.game.command_count, 0);
        assert_eq!(
            message_text(s.last_message_id),
            "The Floating Eye appears healthier."
        );
    });
}

#[test]
fn execute_spell_breath_light_scaling_seed600() {
    reset_for_new_game(Some(600));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(2, BLUE_JELLY_ID, 120, Coord_t { y: 10, x: 11 }, true, 1);
    let creature = &CREATURES_LIST[BLUE_JELLY_ID as usize];
    let mut name = vtype_from_str("The Blue Jelly ");
    let death = death_description_for(BLUE_JELLY_ID);
    monster_execute_casting_of_spell(2, 20, creature.level, &mut name, &death);
    with_state(|s| {
        assert_eq!(
            message_text(s.last_message_id),
            "The Blue Jelly breathes lightning."
        );
        // Breath damage is monster.hp / 4 = 30 at distance 0 → full 30 HP.
        assert_eq!(s.py.misc.current_hp, 470);
    });
}

#[test]
fn execute_spell_teleport_to_player_seed700() {
    reset_for_new_game(Some(700));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(2, ORC_SHAMAN_ID, 40, Coord_t { y: 12, x: 14 }, true, 4);
    let creature = &CREATURES_LIST[ORC_SHAMAN_ID as usize];
    let mut name = vtype_from_str("The Orc Shaman ");
    let death = death_description_for(ORC_SHAMAN_ID);
    let before = with_state(|s| s.py.pos);
    monster_execute_casting_of_spell(2, 7, creature.level, &mut name, &death);
    with_state(|s| {
        // Teleport-to places the player near the monster at (12,14).
        let dy = (s.py.pos.y - 12).abs();
        let dx = (s.py.pos.x - 14).abs();
        assert!(
            dy <= 2 && dx <= 2,
            "player should land near monster, got {:?} (was {:?})",
            s.py.pos,
            before
        );
        assert_ne!(s.py.pos, before);
    });
}

#[test]
fn execute_spell_summon_monster_seed800() {
    reset_for_new_game(Some(800));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    init_monster_levels_for_summon();
    place_monster(2, LOST_SOUL_ID, 20, Coord_t { y: 10, x: 11 }, true, 1);
    let creature = &CREATURES_LIST[LOST_SOUL_ID as usize];
    let mut name = vtype_from_str("The Lost Soul ");
    let death = death_description_for(LOST_SOUL_ID);
    monster_execute_casting_of_spell(2, 14, creature.level, &mut name, &death);
    with_state(|s| {
        assert_eq!(
            message_text(s.last_message_id),
            "The Lost Soul magically summons a monster!"
        );
    });
}

#[test]
fn execute_spell_summon_undead_seed900() {
    reset_for_new_game(Some(900));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    init_monster_levels_for_summon();
    place_monster(2, LOST_SOUL_ID, 20, Coord_t { y: 10, x: 11 }, true, 1);
    let creature = &CREATURES_LIST[LOST_SOUL_ID as usize];
    let mut name = vtype_from_str("The Lost Soul ");
    let death = death_description_for(LOST_SOUL_ID);
    monster_execute_casting_of_spell(2, 15, creature.level, &mut name, &death);
}

fn init_monster_levels_for_summon() {
    use umoria::monster::{MON_MAX_CREATURES, MON_MAX_LEVELS};
    use umoria::monster_manager::monster_get_one_suitable_for_level;
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
    let _ = monster_get_one_suitable_for_level(0);
}

// ---------------------------------------------------------------------------
// 4. Disturb / message parity via monster_cast_spell
// ---------------------------------------------------------------------------
#[test]
fn monster_cast_spell_disturbs_for_hold_person_seed4() {
    reset_for_new_game(Some(4));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(2, PRIEST_ID, 20, Coord_t { y: 10, x: 11 }, true, 1);
    assert!(monster_cast_spell(2));
    with_state(|s| {
        assert_eq!(s.game.command_count, 0);
        assert_eq!(message_text(s.last_message_id), "The Priest casts a spell.");
    });
}

#[test]
fn monster_cast_spell_it_prefix_when_unlit_seed4() {
    reset_for_new_game(Some(4));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(2, PRIEST_ID, 20, Coord_t { y: 10, x: 11 }, false, 1);
    assert!(monster_cast_spell(2));
    with_state(|s| {
        assert!(!s.monsters[2].lit);
        assert_eq!(message_text(s.last_message_id), "It casts a spell.");
    });
}

#[test]
fn monster_cast_spell_teleport_short_no_disturb_seed14() {
    // seed14: freq gate rn(15)==1 so Poltergeist casts; only spell is Teleport Short.
    reset_for_new_game(Some(14));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    place_monster(2, POLTERGEIST_ID, 10, Coord_t { y: 10, x: 11 }, true, 1);
    with_state_mut(|s| s.game.command_count = 5);
    assert!(monster_cast_spell(2));
    with_state(|s| {
        // Teleport Short (spell id 5) must not disturb (command_count stays 5).
        assert_eq!(s.game.command_count, 5);
        // Monster should have moved away from (10,11).
        assert_ne!(s.monsters[2].pos, Coord_t { y: 10, x: 11 });
    });
}

// ---------------------------------------------------------------------------
// 5. Recall bookkeeping parity
// ---------------------------------------------------------------------------
#[test]
fn monster_cast_spell_recall_spell_bit_and_freq_seed12() {
    reset_for_new_game(Some(12));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    with_state_mut(|s| {
        s.creature_recall[FLOATING_EYE_ID as usize].spells = CS_FREQ;
    });
    place_monster(2, FLOATING_EYE_ID, 30, Coord_t { y: 10, x: 11 }, true, 1);
    assert!(monster_cast_spell(2));
    with_state(|s| {
        let memory = &s.creature_recall[FLOATING_EYE_ID as usize];
        assert_ne!(memory.spells & (1 << 16), 0);
        assert_eq!(memory.spells & CS_FREQ, CS_FREQ);
    });
}

// ---------------------------------------------------------------------------
// 6. Dead-player short-circuit
// ---------------------------------------------------------------------------
#[test]
fn monster_cast_spell_dead_player_no_rng_seed42() {
    reset_for_new_game(Some(42));
    setup_dungeon(20, 20);
    setup_player(Coord_t { y: 10, x: 10 });
    reset_monster_slots();
    with_state_mut(|s| s.game.character_is_dead = true);
    place_monster(2, FLOATING_EYE_ID, 30, Coord_t { y: 10, x: 11 }, true, 1);
    assert!(!monster_cast_spell(2));
    assert_eq!(next_random_pair(13), (13, 6));
}
