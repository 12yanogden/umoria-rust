//! Port of src/game_death.cpp — see phase_5.4.

use std::cell::{Cell, RefCell};

use crate::config::files::{death_royal, death_tomb};
use crate::data_player::CLASSES;
use crate::game::{exit_program, with_state, with_state_mut};
use crate::game_files::{display_death_file, output_player_character_to_file};
use crate::game_save::save_game;
use crate::helpers::human_date_string;
use crate::identification::{item_set_as_identified, spell_item_identify_and_remove_random_inscription};
use crate::player::{
    player_is_male, player_rank_title, player_recalculate_bonuses, PLAYER_MAX_LEVEL,
};
use crate::scores::{record_new_high_score, show_scores_screen};
use crate::spells::spell_restore_player_levels;
use crate::types::CNIL;
use crate::ui::print_character;
use crate::ui_inventory::{display_equipment, display_inventory_items};
use crate::ui_io::ESCAPE;
use crate::ui_io::terminal::{self, Coord};

thread_local! {
    static TEST_IDENTIFY_COUNT: Cell<u32> = const { Cell::new(0) };
    static TEST_RECALC_COUNT: Cell<u32> = const { Cell::new(0) };
    static TEST_DISPLAY_EQUIPMENT: Cell<bool> = const { Cell::new(false) };
    static TEST_DISPLAY_INVENTORY: RefCell<Option<(i32, i32)>> = const { RefCell::new(None) };
    static TEST_KINGLY_CALLED: Cell<bool> = const { Cell::new(false) };
    static TEST_PRINT_TOMB_CALLED: Cell<bool> = const { Cell::new(false) };
    static TEST_PRINT_CROWN_CALLED: Cell<bool> = const { Cell::new(false) };
}

#[doc(hidden)]
pub fn test_reset_death_hooks() {
    TEST_IDENTIFY_COUNT.with(|c| c.set(0));
    TEST_RECALC_COUNT.with(|c| c.set(0));
    TEST_DISPLAY_EQUIPMENT.with(|c| c.set(false));
    TEST_DISPLAY_INVENTORY.with(|slot| *slot.borrow_mut() = None);
    TEST_KINGLY_CALLED.with(|c| c.set(false));
    TEST_PRINT_TOMB_CALLED.with(|c| c.set(false));
    TEST_PRINT_CROWN_CALLED.with(|c| c.set(false));
}

#[doc(hidden)]
pub fn test_identify_side_effect_count() -> u32 {
    TEST_IDENTIFY_COUNT.with(|c| c.get())
}

#[doc(hidden)]
pub fn test_recalculate_bonuses_count() -> u32 {
    TEST_RECALC_COUNT.with(|c| c.get())
}

#[doc(hidden)]
pub fn test_display_equipment_called() -> bool {
    TEST_DISPLAY_EQUIPMENT.with(|c| c.get())
}

#[doc(hidden)]
pub fn test_display_inventory_range() -> Option<(i32, i32)> {
    TEST_DISPLAY_INVENTORY.with(|slot| *slot.borrow())
}

#[doc(hidden)]
pub fn test_kingly_called() -> bool {
    TEST_KINGLY_CALLED.with(|c| c.get())
}

#[doc(hidden)]
pub fn test_print_tomb_called() -> bool {
    TEST_PRINT_TOMB_CALLED.with(|c| c.get())
}

#[doc(hidden)]
pub fn test_print_crown_called() -> bool {
    TEST_PRINT_CROWN_CALLED.with(|c| c.get())
}

/// C++ `(int)(26 - text.length() / 2)` with unsigned `length()` arithmetic.
#[must_use]
pub fn tomb_center_col(text_len: usize) -> i32 {
    (26usize.wrapping_sub(text_len / 2)) as i32
}

fn c_strcpy(dst: &mut [u8], src: &str) {
    let bytes = src.as_bytes();
    let n = bytes.len().min(dst.len().saturating_sub(1));
    dst[..n].copy_from_slice(&bytes[..n]);
    if !dst.is_empty() {
        dst[n.min(dst.len() - 1)] = 0;
    }
}

fn print_crown() {
    TEST_PRINT_CROWN_CALLED.with(|c| c.set(true));
    display_death_file(death_royal);
    if player_is_male() {
        terminal::put_string("King!", Coord { y: 17, x: 45 });
    } else {
        terminal::put_string("Queen!", Coord { y: 17, x: 45 });
    }
    terminal::flush_input_buffer();
    terminal::wait_for_continue_key(23);
}

fn kingly() {
    TEST_KINGLY_CALLED.with(|c| c.set(true));
    with_state_mut(|state| {
        state.dg.current_level = 0;
        c_strcpy(&mut state.game.character_died_from, "Ripe Old Age");
    });

    let _ = spell_restore_player_levels();

    with_state_mut(|state| {
        state.py.misc.level = state
            .py
            .misc
            .level
            .wrapping_add(u16::from(PLAYER_MAX_LEVEL));
        state.py.misc.au = state.py.misc.au.wrapping_add(250_000);
        state.py.misc.max_exp = state.py.misc.max_exp.wrapping_add(5_000_000);
        state.py.misc.exp = state.py.misc.max_exp;
    });

    print_crown();
}

fn identify_inventory_and_recalculate() {
    with_state_mut(|state| {
        for item in &mut state.py.inventory {
            item_set_as_identified(item.category_id, item.sub_category_id);
            spell_item_identify_and_remove_random_inscription(item);
            TEST_IDENTIFY_COUNT.with(|c| c.set(c.get().wrapping_add(1)));
        }
    });
    player_recalculate_bonuses();
    TEST_RECALC_COUNT.with(|c| c.set(c.get().wrapping_add(1)));
}

fn print_tomb() {
    TEST_PRINT_TOMB_CALLED.with(|c| c.set(true));
    display_death_file(death_tomb);

    let snapshot = with_state(|state| {
        (
            c_string(&state.py.misc.name),
            state.game.total_winner,
            state.py.misc.class_id,
            state.py.misc.level,
            state.py.misc.exp,
            state.py.misc.au,
            state.dg.current_level,
            c_string(&state.game.character_died_from),
        )
    });

    let (name, total_winner, class_id, level, exp, au, current_level, died_from) = snapshot;

    terminal::put_string(&name, Coord { y: 6, x: tomb_center_col(name.len()) });

    let rank_text = if total_winner {
        "Magnificent".to_string()
    } else {
        player_rank_title().to_string()
    };
    terminal::put_string(
        &rank_text,
        Coord {
            y: 8,
            x: tomb_center_col(rank_text.len()),
        },
    );

    let class_text = if total_winner {
        if player_is_male() {
            "*King*".to_string()
        } else {
            "*Queen*".to_string()
        }
    } else {
        CLASSES
            .get(class_id as usize)
            .map(|class| class.title.to_string())
            .unwrap_or_default()
    };
    terminal::put_string(
        &class_text,
        Coord {
            y: 10,
            x: tomb_center_col(class_text.len()),
        },
    );

    let level_text = level.to_string();
    terminal::put_string(&level_text, Coord { y: 11, x: 30 });

    let exp_text = format!("{exp} Exp");
    terminal::put_string(
        &exp_text,
        Coord {
            y: 12,
            x: tomb_center_col(exp_text.len()),
        },
    );

    let au_text = format!("{au} Au");
    terminal::put_string(
        &au_text,
        Coord {
            y: 13,
            x: tomb_center_col(au_text.len()),
        },
    );

    let depth_text = current_level.to_string();
    terminal::put_string(&depth_text, Coord { y: 14, x: 34 });

    terminal::put_string(
        &died_from,
        Coord {
            y: 16,
            x: tomb_center_col(died_from.len()),
        },
    );

    let mut day = [0u8; 11];
    human_date_string(&mut day);
    let date_text = c_string(&day);
    terminal::put_string(
        &date_text,
        Coord {
            y: 17,
            x: tomb_center_col(date_text.len()),
        },
    );

    loop {
        terminal::flush_input_buffer();

        terminal::put_string(
            "(ESC to abort, return to print on screen, or file name)",
            Coord { y: 23, x: 0 },
        );
        terminal::put_string("Character record?", Coord { y: 22, x: 0 });

        let mut str_buf = [0u8; 80];
        if terminal::get_string_input(&mut str_buf, Coord { y: 22, x: 18 }, 60) {
            identify_inventory_and_recalculate();

            if str_buf[0] != 0 {
                let path = c_string(&str_buf);
                if !output_player_character_to_file(&path) {
                    continue;
                }
            } else {
                terminal::clear_screen();
                print_character();
                terminal::put_string(
                    "Type ESC to skip the inventory:",
                    Coord { y: 23, x: 0 },
                );
                if terminal::get_key_input() != ESCAPE {
                    terminal::clear_screen();
                    terminal::print_message(Some("You are using:"));
                    TEST_DISPLAY_EQUIPMENT.with(|c| c.set(true));
                    let _ = display_equipment(true, 0);
                    terminal::print_message(CNIL);
                    terminal::print_message(Some("You are carrying:"));
                    terminal::clear_to_bottom(1);
                    let end = i32::from(with_state(|state| state.py.pack.unique_items - 1));
                    TEST_DISPLAY_INVENTORY.with(|slot| *slot.borrow_mut() = Some((0, end)));
                    let _ = display_inventory_items(0, end, true, 0, None);
                    terminal::print_message(CNIL);
                }
            }
        }
        break;
    }
}

fn c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// Port of `endGame` in game_death.cpp lines 122–154.
pub fn end_game() {
    terminal::print_message(CNIL);
    terminal::flush_input_buffer();

    if with_state(|state| state.dg.game_turn >= 0) {
        if with_state(|state| state.game.total_winner) {
            kingly();
        }
        print_tomb();
    }

    if with_state(|state| state.game.character_generated && !state.game.character_saved) {
        let _ = save_game();
    }

    if with_state(|state| state.game.character_generated) {
        with_state_mut(|state| state.game.character_saved = false);
        record_new_high_score();
        show_scores_screen();
    }

    terminal::erase_line(Coord { y: 23, x: 0 });
    exit_program();
}

#[doc(hidden)]
pub fn test_print_tomb() {
    print_tomb();
}

#[doc(hidden)]
pub fn test_print_crown() {
    print_crown();
}

#[doc(hidden)]
pub fn test_kingly() {
    kingly();
}
