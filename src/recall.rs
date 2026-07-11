//! Monster memory recall text and knowledge store

use std::cell::{Cell, RefCell};

use crate::config::monsters::defense::{
    CD_ANIMAL, CD_EVIL, CD_FROST, CD_INFRA, CD_MAX_HP, CD_NO_SLEEP, CD_UNDEAD, CD_WEAKNESS,
};
use crate::config::monsters::move_flags::{
    CM_1D2_OBJ, CM_2D2_OBJ, CM_4D2_OBJ, CM_60_RANDOM, CM_90_RANDOM, CM_ALL_MV_FLAGS,
    CM_ATTACK_ONLY, CM_CARRY_GOLD, CM_CARRY_OBJ, CM_INVISIBLE, CM_ONLY_MAGIC, CM_RANDOM_MOVE,
    CM_SMALL_OBJ, CM_SPECIAL, CM_TREASURE, CM_TR_SHIFT, CM_WIN,
};
use crate::config::monsters::spells::{CS_BREATHE, CS_BR_LIGHT, CS_FREQ, CS_SPELLS, CS_TEL_SHORT};
use crate::config::monsters::MON_ENDGAME_LEVEL;
use crate::data_creatures::{CREATURES_LIST, MONSTER_ATTACKS};
use crate::data_recall::{
    RECALL_DESCRIPTION_ATTACK_METHOD, RECALL_DESCRIPTION_ATTACK_TYPE, RECALL_DESCRIPTION_BREATH,
    RECALL_DESCRIPTION_HOW_MUCH, RECALL_DESCRIPTION_MOVE, RECALL_DESCRIPTION_SPELL,
    RECALL_DESCRIPTION_WEAKNESS,
};
use crate::game::{with_state, with_state_mut};
use crate::monster::{Creature, MON_MAX_ATTACKS};
use crate::types::{Vtype_t, MORIA_MESSAGE_SIZE};
use crate::ui_io::terminal;
use crate::ui_io::{self, ESCAPE};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Recall {
    pub movement: u32,
    pub spells: u32,
    pub kills: u16,
    pub deaths: u16,
    pub defenses: u16,
    pub wake: u8,
    pub ignore: u8,
    pub attacks: [u8; MON_MAX_ATTACKS as usize],
}

const SHRT_MAX: u16 = i16::MAX as u16;
const UCHAR_MAX: u8 = u8::MAX;

thread_local! {
    static ROFF_BUFFER: RefCell<Vtype_t> = const { RefCell::new([0; MORIA_MESSAGE_SIZE]) };
    static ROFF_BUFFER_POINTER: Cell<usize> = const { Cell::new(0) };
    static ROFF_PRINT_LINE: Cell<i32> = const { Cell::new(0) };
    static TEST_CAPTURE_LINES: RefCell<Vec<(i32, String)>> = const { RefCell::new(Vec::new()) };
    static TEST_CAPTURE_ENABLED: Cell<bool> = const { Cell::new(false) };
}

#[inline]
fn plural<'a>(count: u16, singular: &'a str, plural_form: &'a str) -> &'a str {
    if count == 1 {
        singular
    } else {
        plural_form
    }
}

#[inline]
fn knowdamage(level: u8, attacks: u8, damage: u8) -> bool {
    (4 + u32::from(level)) * u32::from(attacks) > 80 * u32::from(damage)
}

fn vtype_clear(buf: &mut Vtype_t) {
    buf.fill(0);
}

fn vtype_set_cstr(buf: &mut Vtype_t, text: &str) {
    vtype_clear(buf);
    let bytes = text.as_bytes();
    let n = bytes.len().min(MORIA_MESSAGE_SIZE - 1);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf[n] = 0;
}

fn vtype_cstr(buf: &Vtype_t) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

fn emit_recall_line(line: i32, text: &str) {
    if TEST_CAPTURE_ENABLED.with(std::cell::Cell::get) {
        TEST_CAPTURE_LINES.with(|lines| lines.borrow_mut().push((line, text.to_owned())));
    } else {
        terminal::put_string_clear_to_eol(text, terminal::Coord { y: line, x: 0 });
    }
}

/// 53
fn memory_print(p: &str) {
    for ch in p.chars() {
        ROFF_BUFFER.with(|buf_cell| {
            let mut buf = buf_cell.borrow_mut();
            let ptr = ROFF_BUFFER_POINTER.get();

            if ptr < MORIA_MESSAGE_SIZE {
                buf[ptr] = ch as u8;
            }

            if ch == '\n' || ptr >= MORIA_MESSAGE_SIZE - 1 {
                let mut q = ptr;
                if ch != '\n' {
                    while buf[q] != b' ' {
                        q = q.wrapping_sub(1);
                    }
                }
                buf[q] = 0;

                let line = ROFF_PRINT_LINE.get();
                emit_recall_line(line, &vtype_cstr(&buf));
                ROFF_PRINT_LINE.set(line + 1);

                let mut r = 0usize;
                let mut qq = q;
                while qq < ptr {
                    qq += 1;
                    buf[r] = buf[qq];
                    r += 1;
                }
                ROFF_BUFFER_POINTER.set(r);
            } else {
                ROFF_BUFFER_POINTER.set(ptr + 1);
            }
        });
    }
}

fn reset_recall_print_state() {
    ROFF_BUFFER.with(|buf| vtype_clear(&mut buf.borrow_mut()));
    ROFF_BUFFER_POINTER.set(0);
    ROFF_PRINT_LINE.set(0);
}

/// 72
#[must_use]
pub fn memory_monster_known(memory: &Recall) -> bool {
    if with_state(|s| s.game.wizard_mode) {
        return true;
    }

    if memory.movement != 0
        || memory.defenses != 0
        || memory.kills != 0
        || memory.spells != 0
        || memory.deaths != 0
    {
        return true;
    }

    memory.attacks.iter().any(|&attack| attack != 0)
}

/// 103
pub fn memory_wizard_mode_init(memory: &mut Recall, creature: &Creature) {
    memory.kills = SHRT_MAX;
    memory.wake = UCHAR_MAX;
    memory.ignore = UCHAR_MAX;

    let mut mv = u32::from((creature.movement & CM_4D2_OBJ) != 0) * 8;
    mv += u32::from((creature.movement & CM_2D2_OBJ) != 0) * 4;
    mv += u32::from((creature.movement & CM_1D2_OBJ) != 0) * 2;
    mv += u32::from((creature.movement & CM_90_RANDOM) != 0);
    mv += u32::from((creature.movement & CM_60_RANDOM) != 0);

    memory.movement = (creature.movement & !CM_TREASURE) | (mv << CM_TR_SHIFT);
    memory.defenses = creature.defenses;

    if (creature.spells & CS_FREQ) != 0 {
        memory.spells = creature.spells | CS_FREQ;
    } else {
        memory.spells = creature.spells;
    }

    for i in 0..MON_MAX_ATTACKS as usize {
        if creature.damage[i] == 0 {
            break;
        }
        memory.attacks[i] = UCHAR_MAX;
    }

    if (memory.movement & CM_ONLY_MAGIC) != 0 {
        memory.attacks[0] = UCHAR_MAX;
    }
}

/// 126
fn memory_conflict_history(deaths: u16, kills: u16) {
    let mut desc = [0u8; MORIA_MESSAGE_SIZE];

    if deaths != 0 {
        vtype_set_cstr(
            &mut desc,
            &format!(
                "{} of the contributors to your monster memory {}",
                deaths,
                plural(deaths, "has", "have")
            ),
        );
        memory_print(&vtype_cstr(&desc));
        memory_print(" been killed by this creature, and ");
        if kills == 0 {
            memory_print("it is not ever known to have been defeated.");
        } else {
            vtype_set_cstr(
                &mut desc,
                &format!(
                    "at least {} of the beasts {} been exterminated.",
                    kills,
                    plural(kills, "has", "have")
                ),
            );
            memory_print(&vtype_cstr(&desc));
        }
    } else if kills != 0 {
        vtype_set_cstr(
            &mut desc,
            &format!(
                "At least {} of these creatures {}",
                kills,
                plural(kills, "has", "have")
            ),
        );
        memory_print(&vtype_cstr(&desc));
        memory_print(" been killed by contributors to your monster memory.");
    } else {
        memory_print("No known battles to the death are recalled.");
    }
}

/// 149
fn memory_depth_found_at(level: u8, kills: u16) -> bool {
    let mut known = false;

    if level == 0 {
        known = true;
        memory_print(" It lives in the town");
    } else if kills != 0 {
        known = true;
        let mut lvl = level;
        if lvl > MON_ENDGAME_LEVEL {
            lvl = MON_ENDGAME_LEVEL;
        }
        let mut desc = [0u8; MORIA_MESSAGE_SIZE];
        vtype_set_cstr(
            &mut desc,
            &format!(
                " It is normally found at depths of {} feet",
                u32::from(lvl) * 50
            ),
        );
        memory_print(&vtype_cstr(&desc));
    }

    known
}

/// 218
fn memory_movement(rc_move: u32, monster_speed: u8, mut is_known: bool) -> bool {
    let monster_speed = i32::from(monster_speed) - 10;

    if (rc_move & CM_ALL_MV_FLAGS) != 0 {
        if is_known {
            memory_print(", and");
        } else {
            memory_print(" It");
            is_known = true;
        }

        memory_print(" moves");

        if (rc_move & CM_RANDOM_MOVE) != 0 {
            let idx = ((rc_move & CM_RANDOM_MOVE) >> 3) as usize;
            memory_print(RECALL_DESCRIPTION_HOW_MUCH[idx]);
            memory_print(" erratically");
        }

        if monster_speed == 1 {
            memory_print(" at normal speed");
        } else {
            if (rc_move & CM_RANDOM_MOVE) != 0 {
                memory_print(", and");
            }

            if monster_speed <= 0 {
                if monster_speed == -1 {
                    memory_print(" very");
                } else if monster_speed < -1 {
                    memory_print(" incredibly");
                }
                memory_print(" slowly");
            } else {
                if monster_speed == 3 {
                    memory_print(" very");
                } else if monster_speed > 3 {
                    memory_print(" unbelievably");
                }
                memory_print(" quickly");
            }
        }
    }

    if (rc_move & CM_ATTACK_ONLY) != 0 {
        if is_known {
            memory_print(", but");
        } else {
            memory_print(" It");
            is_known = true;
        }
        memory_print(" does not deign to chase intruders");
    }

    if (rc_move & CM_ONLY_MAGIC) != 0 {
        if is_known {
            memory_print(", but");
        } else {
            memory_print(" It");
            is_known = true;
        }
        memory_print(" always moves and attacks by using magic");
    }

    is_known
}

/// returns (quotient, remainder, `plural_char`)
#[must_use]
pub fn memory_kill_points_math(
    monster_exp: u16,
    creature_level: u8,
    player_level: u16,
) -> (i32, i32, char) {
    let player_level_i = i32::from(player_level);
    let quotient = i32::from(monster_exp) * i32::from(creature_level) / player_level_i;
    let remainder = ((i32::from(monster_exp) * i32::from(creature_level) % player_level_i) * 1000
        / player_level_i
        + 5)
        / 10;
    let plural_ch = if quotient == 1 && remainder == 0 {
        '\0'
    } else {
        's'
    };
    (quotient, remainder, plural_ch)
}

/// 279
fn memory_kill_points(creature_defense: u16, monster_exp: u16, level: u8) {
    memory_print(" A kill of this");

    if (creature_defense & CD_ANIMAL) != 0 {
        memory_print(" natural");
    }
    if (creature_defense & CD_EVIL) != 0 {
        memory_print(" evil");
    }
    if (creature_defense & CD_UNDEAD) != 0 {
        memory_print(" undead");
    }

    let player_level = with_state(|s| s.py.misc.level);
    let (quotient, remainder, plural_ch) =
        memory_kill_points_math(monster_exp, level, player_level);

    let mut desc = [0u8; MORIA_MESSAGE_SIZE];
    vtype_set_cstr(
        &mut desc,
        &format!(" creature is worth {quotient}.{remainder:02} point{plural_ch}"),
    );
    memory_print(&vtype_cstr(&desc));

    let ord_suffix = if player_level / 10 == 1 {
        "th"
    } else {
        match player_level % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    };

    let article_n = if player_level == 8 || player_level == 11 || player_level == 18 {
        "n"
    } else {
        ""
    };

    vtype_set_cstr(
        &mut desc,
        &format!(" for a{article_n} {player_level}{ord_suffix} level character."),
    );
    memory_print(&vtype_cstr(&desc));
}

/// 340
fn memory_magic_skills(
    memory_spell_flags: u32,
    monster_spell_flags: u32,
    creature_spell_flags: u32,
) {
    let mut known = true;
    let mut spell_flags = memory_spell_flags;

    let mut i = 0;
    while (spell_flags & CS_BREATHE) != 0 {
        if (spell_flags & (CS_BR_LIGHT << i)) != 0 {
            spell_flags &= !(CS_BR_LIGHT << i);

            if known {
                if (monster_spell_flags & CS_FREQ) != 0 {
                    memory_print(" It can breathe ");
                } else {
                    memory_print(" It is resistant to ");
                }
                known = false;
            } else if (spell_flags & CS_BREATHE) != 0 {
                memory_print(", ");
            } else {
                memory_print(" and ");
            }
            memory_print(RECALL_DESCRIPTION_BREATH[i as usize]);
        }
        i += 1;
    }

    known = true;
    i = 0;
    while (spell_flags & CS_SPELLS) != 0 {
        if (spell_flags & (CS_TEL_SHORT << i)) != 0 {
            spell_flags &= !(CS_TEL_SHORT << i);

            if known {
                if (memory_spell_flags & CS_BREATHE) != 0 {
                    memory_print(", and is also");
                } else {
                    memory_print(" It is");
                }
                memory_print(" magical, casting spells which ");
                known = false;
            } else if (spell_flags & CS_SPELLS) != 0 {
                memory_print(", ");
            } else {
                memory_print(" or ");
            }
            memory_print(RECALL_DESCRIPTION_SPELL[i as usize]);
        }
        i += 1;
    }

    if (memory_spell_flags & (CS_BREATHE | CS_SPELLS)) != 0 {
        if (monster_spell_flags & CS_FREQ) > 5 {
            let temp = format!("; 1 time in {}", creature_spell_flags & CS_FREQ);
            memory_print(&temp);
        }
        memory_print(".");
    }
}

/// 362
fn memory_kill_difficulty(creature: &Creature, monster_kills: u32) {
    if monster_kills <= 304 / (4 + u32::from(creature.level)) {
        return;
    }

    let mut description = [0u8; MORIA_MESSAGE_SIZE];
    vtype_set_cstr(
        &mut description,
        &format!(" It has an armor rating of {}", creature.ac),
    );
    memory_print(&vtype_cstr(&description));

    let maximized = if (creature.defenses & CD_MAX_HP) != 0 {
        " maximized"
    } else {
        ""
    };
    vtype_set_cstr(
        &mut description,
        &format!(
            " and a{} life rating of {}d{}.",
            maximized, creature.hit_die.dice, creature.hit_die.sides
        ),
    );
    memory_print(&vtype_cstr(&description));
}

/// 387
fn memory_special_abilities(mut mv: u32) {
    let mut known = true;
    let mut i = 0;

    while (mv & CM_SPECIAL) != 0 {
        if (mv & (CM_INVISIBLE << i)) != 0 {
            mv &= !(CM_INVISIBLE << i);

            if known {
                memory_print(" It can ");
                known = false;
            } else if (mv & CM_SPECIAL) != 0 {
                memory_print(", ");
            } else {
                memory_print(" and ");
            }
            memory_print(RECALL_DESCRIPTION_MOVE[i as usize]);
        }
        i += 1;
    }

    if !known {
        memory_print(".");
    }
}

/// 411
fn memory_weaknesses(mut defense: u32) {
    let mut known = true;
    let mut i = 0;

    while (defense & u32::from(CD_WEAKNESS)) != 0 {
        if (defense & (u32::from(CD_FROST) << i)) != 0 {
            defense &= !(u32::from(CD_FROST) << i);

            if known {
                memory_print(" It is susceptible to ");
                known = false;
            } else if (defense & u32::from(CD_WEAKNESS)) != 0 {
                memory_print(", ");
            } else {
                memory_print(" and ");
            }
            memory_print(RECALL_DESCRIPTION_WEAKNESS[i as usize]);
        }
        i += 1;
    }

    if !known {
        memory_print(".");
    }
}

/// 446
fn memory_awareness(creature: &Creature, memory: &Recall) {
    if u32::from(memory.wake) * u32::from(memory.wake) > u32::from(creature.sleep_counter)
        || memory.ignore == UCHAR_MAX
        || (creature.sleep_counter == 0 && memory.kills >= 10)
    {
        memory_print(" It ");

        match creature.sleep_counter {
            n if n > 200 => memory_print("prefers to ignore"),
            n if n > 95 => memory_print("pays very little attention to"),
            n if n > 75 => memory_print("pays little attention to"),
            n if n > 45 => memory_print("tends to overlook"),
            n if n > 25 => memory_print("takes quite a while to see"),
            n if n > 10 => memory_print("takes a while to see"),
            n if n > 5 => memory_print("is fairly observant of"),
            n if n > 3 => memory_print("is observant of"),
            n if n > 1 => memory_print("is very observant of"),
            0 => memory_print("is ever vigilant for"),
            _ => memory_print("is vigilant for"),
        }

        let mut text = [0u8; MORIA_MESSAGE_SIZE];
        vtype_set_cstr(
            &mut text,
            &format!(
                " intruders, which it may notice from {} feet.",
                10 * u32::from(creature.area_affect_radius)
            ),
        );
        memory_print(&vtype_cstr(&text));
    }
}

/// 506
fn memory_loot_carried(creature_move: u32, memory_move: u32) {
    if (memory_move & (CM_CARRY_OBJ | CM_CARRY_GOLD)) == 0 {
        return;
    }

    memory_print(" It may");

    let carrying_chance = (memory_move & CM_TREASURE) >> CM_TR_SHIFT;

    if carrying_chance == 1 {
        if (creature_move & CM_TREASURE) == CM_60_RANDOM {
            memory_print(" sometimes");
        } else {
            memory_print(" often");
        }
    } else if carrying_chance == 2 && (creature_move & CM_TREASURE) == (CM_60_RANDOM | CM_90_RANDOM)
    {
        memory_print(" often");
    }

    memory_print(" carry");

    let mut p: &str = if (memory_move & CM_SMALL_OBJ) != 0 {
        " small objects"
    } else {
        " objects"
    };

    if carrying_chance == 1 {
        p = if (memory_move & CM_SMALL_OBJ) != 0 {
            " a small object"
        } else {
            " an object"
        };
    } else if carrying_chance == 2 {
        memory_print(" one or two");
    } else {
        let msg = format!(" up to {carrying_chance}");
        memory_print(&msg);
    }

    if (memory_move & CM_CARRY_OBJ) != 0 {
        memory_print(p);
        if (memory_move & CM_CARRY_GOLD) != 0 {
            memory_print(" or treasure");
            if carrying_chance > 1 {
                memory_print("s");
            }
        }
        memory_print(".");
    } else if carrying_chance != 1 {
        memory_print(" treasures.");
    } else {
        memory_print(" treasure.");
    }
}

/// 585
fn memory_attack_number_and_damage(memory: &Recall, creature: &Creature) {
    let known_attacks = memory.attacks.iter().filter(|&&a| a != 0).count();

    let mut attack_count = 0;
    for i in 0..MON_MAX_ATTACKS as usize {
        let attack_id = creature.damage[i];
        if attack_id == 0 {
            break;
        }

        if memory.attacks[i] == 0 {
            continue;
        }

        let attack = &MONSTER_ATTACKS[attack_id as usize];
        let mut attack_type = attack.type_id;
        let mut attack_description_id = attack.description_id;
        let dice = attack.dice;

        attack_count += 1;

        if attack_count == 1 {
            memory_print(" It can ");
        } else if attack_count == known_attacks {
            memory_print(", and ");
        } else {
            memory_print(", ");
        }

        if attack_description_id > 19 {
            attack_description_id = 0;
        }

        memory_print(RECALL_DESCRIPTION_ATTACK_METHOD[attack_description_id as usize]);

        if attack_type != 1 || (dice.dice > 0 && dice.sides > 0) {
            memory_print(" to ");

            if attack_type > 24 {
                attack_type = 0;
            }

            memory_print(RECALL_DESCRIPTION_ATTACK_TYPE[attack_type as usize]);

            if dice.dice != 0
                && dice.sides != 0
                && knowdamage(creature.level, memory.attacks[i], dice.dice * dice.sides)
            {
                if attack_type == 19 {
                    memory_print(" by");
                } else {
                    memory_print(" with damage");
                }
                let msg = format!(" {}d{}", dice.dice, dice.sides);
                memory_print(&msg);
            }
        }
    }

    if attack_count != 0 {
        memory_print(".");
    } else if known_attacks > 0 && memory.attacks[0] >= 10 {
        memory_print(" It has no physical attacks.");
    } else {
        memory_print(" Nothing is known about its attack.");
    }
}

pub fn memory_recall(monster_id: i32) -> u8 {
    reset_recall_print_state();

    let (saved_memory, wizard_mode) = with_state_mut(|state| {
        let wizard = state.game.wizard_mode;
        let saved = if wizard {
            Some(state.creature_recall[monster_id as usize])
        } else {
            None
        };
        if wizard {
            let creature = CREATURES_LIST[monster_id as usize];
            memory_wizard_mode_init(&mut state.creature_recall[monster_id as usize], &creature);
        }
        (saved, wizard)
    });

    // Snapshot recall data — memory_print / terminal I/O re-enter game state.
    let memory = with_state(|state| state.creature_recall[monster_id as usize]);
    let creature = &CREATURES_LIST[monster_id as usize];

    let spells = memory.spells & creature.spells & !CS_FREQ;
    let mv = memory.movement | (creature.movement & CM_WIN);
    let defense = memory.defenses & creature.defenses;

    let mut msg = [0u8; MORIA_MESSAGE_SIZE];
    vtype_set_cstr(&mut msg, &format!("The {}:\n", creature.name));
    memory_print(&vtype_cstr(&msg));

    memory_conflict_history(memory.deaths, memory.kills);
    let mut known = memory_depth_found_at(creature.level, memory.kills);
    known = memory_movement(mv, creature.speed, known);

    if known {
        memory_print(".");
    }

    if memory.kills != 0 {
        memory_kill_points(creature.defenses, creature.kill_exp_value, creature.level);
    }

    memory_magic_skills(spells, memory.spells, creature.spells);
    memory_kill_difficulty(creature, u32::from(memory.kills));
    memory_special_abilities(mv);
    memory_weaknesses(u32::from(defense));

    if (defense & CD_INFRA) != 0 {
        memory_print(" It is warm blooded");
    }

    if (defense & CD_NO_SLEEP) != 0 {
        if (defense & CD_INFRA) != 0 {
            memory_print(", and");
        } else {
            memory_print(" It");
        }
        memory_print(" cannot be charmed or slept");
    }

    if (defense & (CD_NO_SLEEP | CD_INFRA)) != 0 {
        memory_print(".");
    }

    memory_awareness(creature, &memory);
    memory_loot_carried(creature.movement, mv);
    memory_attack_number_and_damage(&memory, creature);

    if (creature.movement & CM_WIN) != 0 {
        memory_print(" Killing one of these wins the game!");
    }

    memory_print("\n");

    let pause_line = ROFF_PRINT_LINE.get();
    emit_recall_line(pause_line, "--pause--");

    if wizard_mode {
        if let Some(saved) = saved_memory {
            with_state_mut(|state| {
                state.creature_recall[monster_id as usize] = saved;
            });
        }
    }

    ui_io::terminal::get_key_input()
}

/// 699
pub fn recall_monster_attributes(command: u8) {
    let mut n = 0i32;

    for i in (0..crate::monster::MON_MAX_CREATURES).rev() {
        let i = i as usize;
        let (sprite, known) = with_state(|state| {
            (
                CREATURES_LIST[i].sprite,
                memory_monster_known(&state.creature_recall[i]),
            )
        });

        if sprite == command && known {
            if n == 0 {
                let confirmed = ui_io::terminal::get_input_confirmation_with_abort(
                    40,
                    "You recall those details?",
                );
                if confirmed != 1 {
                    break;
                }
                terminal::erase_line(terminal::Coord { y: 0, x: 40 });
                ui_io::terminal::terminal_save_screen();
            }
            n += 1;

            let query = memory_recall(i as i32);
            terminal::terminal_restore_screen();
            if query == ESCAPE {
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Test hooks
// ---------------------------------------------------------------------------

#[doc(hidden)]
pub fn test_set_capture_enabled(enabled: bool) {
    TEST_CAPTURE_ENABLED.with(|c| c.set(enabled));
    if enabled {
        TEST_CAPTURE_LINES.with(|lines| lines.borrow_mut().clear());
    }
}

#[doc(hidden)]
pub fn test_take_captured_lines() -> Vec<(i32, String)> {
    TEST_CAPTURE_LINES.with(|lines| std::mem::take(&mut *lines.borrow_mut()))
}

#[doc(hidden)]
pub fn test_begin_memory_print_capture() {
    test_set_capture_enabled(true);
    reset_recall_print_state();
}

#[doc(hidden)]
pub fn test_feed_memory_print(input: &str) {
    memory_print(input);
}

#[doc(hidden)]
pub fn test_finish_memory_print_capture() -> Vec<(i32, String)> {
    let lines = test_take_captured_lines();
    test_set_capture_enabled(false);
    lines
}

#[doc(hidden)]
pub fn test_memory_print(input: &str) -> Vec<(i32, String)> {
    test_begin_memory_print_capture();
    test_feed_memory_print(input);
    test_finish_memory_print_capture()
}

#[doc(hidden)]
pub fn test_memory_recall_lines(monster_id: i32) -> Vec<String> {
    ui_io::test_set_ncurses_stub(true);
    ui_io::test_clear_getch_keys();
    with_state_mut(|s| {
        if s.py.misc.level == 0 {
            s.py.misc.level = 10;
        }
    });
    test_set_capture_enabled(true);
    let _ = memory_recall(monster_id);
    let lines: Vec<String> = test_take_captured_lines()
        .into_iter()
        .map(|(_, text)| text)
        .collect();
    test_set_capture_enabled(false);
    lines
}
