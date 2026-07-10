//! Phase 4.8.1 — recall.cpp monster memory text parity (no RNG).

use umoria::config::monsters::move_flags::{
    CM_20_RANDOM, CM_40_RANDOM, CM_ATTACK_ONLY, CM_CARRY_GOLD, CM_CARRY_OBJ, CM_MOVE_NORMAL,
    CM_ONLY_MAGIC, CM_SMALL_OBJ, CM_TR_SHIFT,
};
use umoria::config::monsters::spells::{CS_BR_FIRE, CS_FREQ, CS_TEL_SHORT};
use umoria::data_creatures::CREATURES_LIST;
use umoria::game::{reset_for_new_game, with_state, with_state_mut};
use umoria::recall::{
    memory_kill_points_math, memory_monster_known, memory_wizard_mode_init,
    recall_monster_attributes, test_begin_memory_print_capture, test_feed_memory_print,
    test_finish_memory_print_capture, test_memory_print, test_memory_recall_lines, Recall,
};
use umoria::ui_io::{test_clear_getch_keys, test_push_getch_keys, test_set_ncurses_stub, ESCAPE};

const STREET_URCHIN_ID: i32 = 0;
const GREY_MUSHROOM_ID: i32 = 8;
const FLOATING_EYE_ID: i32 = 18;
const QUYLTHULG_ID: i32 = 174;
const FIRE_SPIRIT_ID: i32 = 164;
const BALROG_ID: i32 = 278;

fn set_memory(monster_id: i32, memory: Recall) {
    with_state_mut(|s| s.creature_recall[monster_id as usize] = memory);
}

fn get_memory(monster_id: i32) -> Recall {
    with_state(|s| s.creature_recall[monster_id as usize])
}

fn line_texts(lines: &[(i32, String)]) -> Vec<String> {
    lines.iter().map(|(_, s)| s.clone()).collect()
}

fn recall_body(lines: &[String]) -> String {
    lines.join(" ")
}

// ---------------------------------------------------------------------------
// 1. memoryPrint word-wrap engine
// ---------------------------------------------------------------------------

#[test]
fn memory_print_short_line_no_wrap() {
    let lines = test_memory_print("hello world");
    assert_eq!(lines, vec![]);
}

#[test]
fn memory_print_explicit_newline_emits_line() {
    let lines = test_memory_print("first line\n");
    assert_eq!(line_texts(&lines), vec!["first line"]);
}

#[test]
fn memory_print_wraps_at_buffer_boundary_on_space() {
    let prefix = "A".repeat(70);
    let input = format!("{prefix} word {}", "B".repeat(20));
    let lines = test_memory_print(&input);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].0, 0);
    assert_eq!(lines[0].1, format!("{prefix} word"));
}

#[test]
fn memory_print_carries_remainder_after_wrap() {
    let prefix = "A".repeat(70);
    let tail = "B".repeat(5);
    test_begin_memory_print_capture();
    test_feed_memory_print(&format!("{prefix} word {tail}"));
    test_feed_memory_print("\n");
    let lines = test_finish_memory_print_capture();
    assert_eq!(line_texts(&lines)[1], tail);
}

#[test]
fn memory_print_multiple_newlines_increment_row() {
    let lines = test_memory_print("one\n two\n three\n");
    assert_eq!(
        lines,
        vec![
            (0, "one".to_string()),
            (1, " two".to_string()),
            (2, " three".to_string()),
        ]
    );
}

// ---------------------------------------------------------------------------
// 2. Golden recall text per archetype
// ---------------------------------------------------------------------------

#[test]
fn recall_town_monster_no_battles() {
    reset_for_new_game(None);
    set_memory(
        STREET_URCHIN_ID,
        Recall {
            movement: 1,
            ..Default::default()
        },
    );
    let lines = test_memory_recall_lines(STREET_URCHIN_ID);
    assert_eq!(lines[0], "The Filthy Street Urchin:");
    assert!(lines[1].contains("No known battles to the death are recalled."));
    assert!(lines[1].contains("It lives in the town"));
    assert_eq!(lines.last().map(String::as_str), Some("--pause--"));
}

#[test]
fn recall_conflict_deaths_only() {
    reset_for_new_game(None);
    set_memory(
        GREY_MUSHROOM_ID,
        Recall {
            deaths: 1,
            movement: 1,
            ..Default::default()
        },
    );
    let lines = test_memory_recall_lines(GREY_MUSHROOM_ID);
    let body = recall_body(&lines);
    assert!(body.contains("1 of the contributors to your monster memory has"));
    assert!(body.contains("it is not ever known to have been defeated."));
}

#[test]
fn recall_conflict_kills_plural() {
    reset_for_new_game(None);
    set_memory(
        GREY_MUSHROOM_ID,
        Recall {
            kills: 3,
            movement: 1,
            ..Default::default()
        },
    );
    let lines = test_memory_recall_lines(GREY_MUSHROOM_ID);
    let body = recall_body(&lines);
    assert!(body.contains("At least 3 of these creatures have"));
}

#[test]
fn recall_balrog_depth_clamped_to_endgame() {
    reset_for_new_game(None);
    set_memory(
        BALROG_ID,
        Recall {
            kills: 1,
            movement: 1,
            ..Default::default()
        },
    );
    let lines = test_memory_recall_lines(BALROG_ID);
    let body = recall_body(&lines);
    assert!(body.contains("depths of 2500 feet"));
}

#[test]
fn recall_erratic_movement_how_much_index() {
    reset_for_new_game(None);
    set_memory(
        GREY_MUSHROOM_ID,
        Recall {
            movement: CM_20_RANDOM | CM_40_RANDOM | CM_MOVE_NORMAL,
            ..Default::default()
        },
    );
    let lines = test_memory_recall_lines(GREY_MUSHROOM_ID);
    let body = recall_body(&lines);
    assert!(body.contains(" quite erratically"));
}

#[test]
fn recall_attack_only_and_only_magic_punctuation() {
    reset_for_new_game(None);
    set_memory(
        QUYLTHULG_ID,
        Recall {
            movement: CM_ONLY_MAGIC | CM_ATTACK_ONLY,
            spells: CS_TEL_SHORT,
            ..Default::default()
        },
    );
    let lines = test_memory_recall_lines(QUYLTHULG_ID);
    let body = recall_body(&lines);
    assert!(body.contains("does not deign to chase intruders"));
    assert!(body.contains("always moves and attacks by using magic"));
}

#[test]
fn recall_kill_points_balrog_level1_overflow() {
    reset_for_new_game(None);
    with_state_mut(|s| s.py.misc.level = 1);
    set_memory(BALROG_ID, Recall { kills: 1, ..Default::default() });
    let lines = test_memory_recall_lines(BALROG_ID);
    let body = recall_body(&lines);
    assert!(body.contains("5500000.00 points"));
    assert!(body.contains("for a 1st level character."));
}

#[test]
fn recall_kill_points_singular_point_and_an_article() {
    reset_for_new_game(None);
    with_state_mut(|s| s.py.misc.level = 10);
    set_memory(
        GREY_MUSHROOM_ID,
        Recall {
            kills: 1,
            ..Default::default()
        },
    );
    let lines = test_memory_recall_lines(GREY_MUSHROOM_ID);
    let body = recall_body(&lines);
    assert!(body.contains("0.10 points"));
    assert!(body.contains("for a 10th level character."));
}

#[test]
fn recall_magic_breath_and_spell_frequency() {
    reset_for_new_game(None);
    set_memory(
        BALROG_ID,
        Recall {
            spells: CS_BR_FIRE | CS_FREQ,
            ..Default::default()
        },
    );
    let balrog = recall_body(&test_memory_recall_lines(BALROG_ID));
    assert!(balrog.contains("It can breathe fire"));
    assert!(balrog.contains("; 1 time in 3"));

    reset_for_new_game(None);
    set_memory(
        QUYLTHULG_ID,
        Recall {
            spells: CS_TEL_SHORT | CS_FREQ,
            ..Default::default()
        },
    );
    let quyl = recall_body(&test_memory_recall_lines(QUYLTHULG_ID));
    assert!(quyl.contains("magical, casting spells which teleport short distances"));
}

#[test]
fn recall_kill_difficulty_armor_and_max_hp() {
    reset_for_new_game(None);
    set_memory(BALROG_ID, Recall { kills: 100, ..Default::default() });
    let lines = test_memory_recall_lines(BALROG_ID);
    let body = recall_body(&lines);
    assert!(body.contains("armor rating of 125"));
    assert!(body.contains("life rating of 75d40."));
}

#[test]
fn recall_weaknesses_and_infra_no_sleep() {
    reset_for_new_game(None);
    set_memory(
        FIRE_SPIRIT_ID,
        Recall {
            defenses: 0x3010,
            movement: 1,
            ..Default::default()
        },
    );
    let lines = test_memory_recall_lines(FIRE_SPIRIT_ID);
    let body = recall_body(&lines);
    assert!(body.contains("susceptible to frost"));
    assert!(body.contains("warm blooded"));
    assert!(body.contains("cannot be charmed or slept"));
}

#[test]
fn recall_awareness_wake_ignore_gate() {
    reset_for_new_game(None);
    set_memory(
        FLOATING_EYE_ID,
        Recall {
            wake: 20,
            ..Default::default()
        },
    );
    let lines = test_memory_recall_lines(FLOATING_EYE_ID);
    let body = recall_body(&lines);
    assert!(body.contains("is fairly observant of"));
    assert!(body.contains("notice from 20 feet."));
}

#[test]
fn recall_loot_carrying_chance_wording() {
    reset_for_new_game(None);
    set_memory(
        GREY_MUSHROOM_ID,
        Recall {
            movement: CM_CARRY_OBJ | CM_CARRY_GOLD | CM_SMALL_OBJ | (1 << CM_TR_SHIFT),
            ..Default::default()
        },
    );
    let lines = test_memory_recall_lines(GREY_MUSHROOM_ID);
    let body = recall_body(&lines);
    assert!(body.contains("may often carry a small object or treasure."));
}

#[test]
fn recall_attacks_known_damage() {
    reset_for_new_game(None);
    set_memory(
        BALROG_ID,
        Recall {
            attacks: [15, 10, 0, 0],
            ..Default::default()
        },
    );
    let lines = test_memory_recall_lines(BALROG_ID);
    let body = recall_body(&lines);
    assert!(body.contains("It can hit to shoot flames"));
}

#[test]
fn recall_win_monster_tail_line() {
    reset_for_new_game(None);
    set_memory(BALROG_ID, Recall { movement: 1, ..Default::default() });
    let lines = test_memory_recall_lines(BALROG_ID);
    let body = recall_body(&lines);
    assert!(body.contains("Killing one of these wins the game!"));
    assert_eq!(lines.last().map(String::as_str), Some("--pause--"));
}

// ---------------------------------------------------------------------------
// 3. memoryMonsterKnown truth table
// ---------------------------------------------------------------------------

#[test]
fn memory_monster_known_all_zero_false() {
    assert!(!memory_monster_known(&Recall::default()));
}

#[test]
fn memory_monster_known_any_field_true() {
    assert!(memory_monster_known(&Recall {
        movement: 1,
        ..Default::default()
    }));
    assert!(memory_monster_known(&Recall {
        defenses: 1,
        ..Default::default()
    }));
    assert!(memory_monster_known(&Recall {
        kills: 1,
        ..Default::default()
    }));
    assert!(memory_monster_known(&Recall {
        spells: 1,
        ..Default::default()
    }));
    assert!(memory_monster_known(&Recall {
        deaths: 1,
        ..Default::default()
    }));
    assert!(memory_monster_known(&Recall {
        attacks: [1, 0, 0, 0],
        ..Default::default()
    }));
}

#[test]
fn memory_monster_known_wizard_mode_short_circuit() {
    reset_for_new_game(None);
    with_state_mut(|s| s.game.wizard_mode = true);
    assert!(memory_monster_known(&Recall::default()));
}

// ---------------------------------------------------------------------------
// 4. Wizard-mode init + save/restore
// ---------------------------------------------------------------------------

#[test]
fn memory_wizard_mode_init_maximal_recall() {
    let creature = &CREATURES_LIST[QUYLTHULG_ID as usize];
    let mut memory = Recall::default();
    memory_wizard_mode_init(&mut memory, creature);
    assert_eq!(memory.kills, i16::MAX as u16);
    assert_eq!(memory.wake, u8::MAX);
    assert_eq!(memory.ignore, u8::MAX);
    assert_eq!(memory.attacks[0], u8::MAX);
    assert_ne!(memory.spells & CS_FREQ, 0);
}

#[test]
fn memory_recall_wizard_mode_restores_memory() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.game.wizard_mode = true;
        s.creature_recall[QUYLTHULG_ID as usize] = Recall {
            kills: 2,
            ..Default::default()
        };
    });
    let before = get_memory(QUYLTHULG_ID);
    let _ = test_memory_recall_lines(QUYLTHULG_ID);
    assert_eq!(get_memory(QUYLTHULG_ID), before);
}

// ---------------------------------------------------------------------------
// 5. recallMonsterAttributes control flow
// ---------------------------------------------------------------------------

#[test]
fn recall_monster_attributes_prompt_abort_breaks() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[b'n' as i32]);

    set_memory(
        STREET_URCHIN_ID,
        Recall {
            movement: 1,
            ..Default::default()
        },
    );

    recall_monster_attributes(b'p');
    assert_eq!(get_memory(STREET_URCHIN_ID).movement, 1);
}

#[test]
fn recall_monster_attributes_escape_after_recall_breaks() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[b'y' as i32, ESCAPE as i32]);

    set_memory(
        STREET_URCHIN_ID,
        Recall {
            movement: 1,
            ..Default::default()
        },
    );

    recall_monster_attributes(b'p');
    assert_eq!(get_memory(STREET_URCHIN_ID).movement, 1);
}

// ---------------------------------------------------------------------------
// 6. Integer/overflow fidelity — memoryKillPoints math
// ---------------------------------------------------------------------------

#[test]
fn memory_kill_points_math_quotient_remainder() {
    assert_eq!(memory_kill_points_math(55000, 100, 1), (5_500_000, 0, 's'));
    assert_eq!(memory_kill_points_math(100, 10, 7), (142, 86, 's'));
    assert_eq!(memory_kill_points_math(50, 5, 10), (25, 0, 's'));
}

#[test]
fn memory_kill_points_math_balrog_l1_matches_cpp_long_arithmetic() {
    let (q, r, p) = memory_kill_points_math(55000, 100, 1);
    assert_eq!(q, 5_500_000);
    assert_eq!(r, 0);
    assert_eq!(p, 's');
}
