//! Port of `src/player_quaff.cpp` — potion quaffing.

use crate::config::player::PLAYER_MAX_EXP;
use crate::dice::{dice_roll, Dice};
use crate::game::{random_number_state, with_state, with_state_mut};
use crate::helpers::get_and_clear_first_bit;
use crate::identification::{
    item_identify, item_set_as_tried, item_set_colorless_as_identified,
    item_type_remaining_count_description,
};
use crate::inventory::{inventory_destroy_item, inventory_find_range};
use crate::player::PlayerAttr;
use crate::player_eat::player_ingest_food;
use crate::player_magic::{
    player_cure_blindness, player_cure_confusion, player_cure_poison, player_detect_invisible,
    player_remove_fear,
};
use crate::player_stats::{player_stat_random_increase, player_stat_restore};
use crate::spells::{
    spell_change_player_hit_points, spell_lose_chr, spell_lose_exp, spell_lose_int, spell_lose_str,
    spell_lose_wis, spell_restore_player_levels, spell_slow_poison,
};
use crate::treasure::{TV_POTION1, TV_POTION2};
use crate::ui::{display_character_experience, print_character_current_mana};
use crate::ui_inventory::inventory_get_input_for_item_id;
use crate::ui_io::terminal;

const SHRT_MAX: i32 = 32_767;

/// C++ `player_quaff.cpp` lines 60–334.
pub fn player_drink_potion(flags: u32, item_type: u8) -> bool {
    let mut identified = false;
    let mut flags = flags;

    while flags != 0 {
        let mut potion_id = get_and_clear_first_bit(&mut flags) + 1;

        if item_type == TV_POTION2 {
            potion_id += 32;
        }

        identified = match potion_id {
            1 => {
                if player_stat_random_increase(PlayerAttr::A_STR) {
                    terminal::print_message(Some("Wow!  What bulging muscles!"));
                    true
                } else {
                    identified
                }
            }
            2 => {
                spell_lose_str();
                true
            }
            3 => {
                if player_stat_restore(PlayerAttr::A_STR) {
                    terminal::print_message(Some("You feel warm all over."));
                    true
                } else {
                    identified
                }
            }
            4 => {
                if player_stat_random_increase(PlayerAttr::A_INT) {
                    terminal::print_message(Some("Aren't you brilliant!"));
                    true
                } else {
                    identified
                }
            }
            5 => {
                spell_lose_int();
                true
            }
            6 => {
                if player_stat_restore(PlayerAttr::A_INT) {
                    terminal::print_message(Some("You have have a warm feeling."));
                    true
                } else {
                    identified
                }
            }
            7 => {
                if player_stat_random_increase(PlayerAttr::A_WIS) {
                    terminal::print_message(Some("You suddenly have a profound thought!"));
                    true
                } else {
                    identified
                }
            }
            8 => {
                spell_lose_wis();
                true
            }
            9 => {
                if player_stat_restore(PlayerAttr::A_WIS) {
                    terminal::print_message(Some("You feel your wisdom returning."));
                    true
                } else {
                    identified
                }
            }
            10 => {
                if player_stat_random_increase(PlayerAttr::A_CHR) {
                    terminal::print_message(Some("Gee, ain't you cute!"));
                    true
                } else {
                    identified
                }
            }
            11 => {
                spell_lose_chr();
                true
            }
            12 => {
                if player_stat_restore(PlayerAttr::A_CHR) {
                    terminal::print_message(Some("You feel your looks returning."));
                    true
                } else {
                    identified
                }
            }
            13 => spell_change_player_hit_points(dice_roll(Dice { dice: 2, sides: 7 })),
            14 => spell_change_player_hit_points(dice_roll(Dice { dice: 4, sides: 7 })),
            15 => spell_change_player_hit_points(dice_roll(Dice { dice: 6, sides: 7 })),
            16 => spell_change_player_hit_points(1000),
            17 => {
                if player_stat_random_increase(PlayerAttr::A_CON) {
                    terminal::print_message(Some("You feel tingly for a moment."));
                    true
                } else {
                    identified
                }
            }
            18 => {
                if with_state(|state| state.py.misc.exp) < PLAYER_MAX_EXP {
                    let mut exp = with_state(|state| state.py.misc.exp / 2 + 10);
                    if exp > 100_000 {
                        exp = 100_000;
                    }
                    with_state_mut(|state| state.py.misc.exp += exp);
                    terminal::print_message(Some("You feel more experienced."));
                    display_character_experience();
                    true
                } else {
                    identified
                }
            }
            19 => {
                if with_state(|state| !state.py.flags.free_action) {
                    terminal::print_message(Some("You fall asleep."));
                    with_state_mut(|state| {
                        state.py.flags.paralysis = state
                            .py
                            .flags
                            .paralysis
                            .wrapping_add((random_number_state(state, 4) + 4) as i16);
                    });
                    true
                } else {
                    identified
                }
            }
            20 => {
                if with_state(|state| state.py.flags.blind == 0) {
                    terminal::print_message(Some("You are covered by a veil of darkness."));
                    identified = true;
                }
                with_state_mut(|state| {
                    state.py.flags.blind = state
                        .py
                        .flags
                        .blind
                        .wrapping_add((random_number_state(state, 100) + 100) as i16);
                });
                identified
            }
            21 => {
                if with_state(|state| state.py.flags.confused == 0) {
                    terminal::print_message(Some("Hey!  This is good stuff!  * Hick! *"));
                    identified = true;
                }
                with_state_mut(|state| {
                    state.py.flags.confused = state
                        .py
                        .flags
                        .confused
                        .wrapping_add((random_number_state(state, 20) + 12) as i16);
                });
                identified
            }
            22 => {
                if with_state(|state| state.py.flags.poisoned == 0) {
                    terminal::print_message(Some("You feel very sick."));
                    identified = true;
                }
                with_state_mut(|state| {
                    state.py.flags.poisoned = state
                        .py
                        .flags
                        .poisoned
                        .wrapping_add((random_number_state(state, 15) + 10) as i16);
                });
                identified
            }
            23 => {
                if with_state(|state| state.py.flags.fast == 0) {
                    identified = true;
                }
                with_state_mut(|state| {
                    state.py.flags.fast = state
                        .py
                        .flags
                        .fast
                        .wrapping_add((random_number_state(state, 25) + 15) as i16);
                });
                identified
            }
            24 => {
                if with_state(|state| state.py.flags.slow == 0) {
                    identified = true;
                }
                with_state_mut(|state| {
                    state.py.flags.slow = state
                        .py
                        .flags
                        .slow
                        .wrapping_add((random_number_state(state, 25) + 15) as i16);
                });
                identified
            }
            26 => {
                if player_stat_random_increase(PlayerAttr::A_DEX) {
                    terminal::print_message(Some("You feel more limber!"));
                    true
                } else {
                    identified
                }
            }
            27 => {
                if player_stat_restore(PlayerAttr::A_DEX) {
                    terminal::print_message(Some("You feel less clumsy."));
                    true
                } else {
                    identified
                }
            }
            28 => {
                if player_stat_restore(PlayerAttr::A_CON) {
                    terminal::print_message(Some("You feel your health returning!"));
                    true
                } else {
                    identified
                }
            }
            29 => player_cure_blindness(),
            30 => player_cure_confusion(),
            31 => player_cure_poison(),
            34 => {
                if with_state(|state| state.py.misc.exp > 0) {
                    terminal::print_message(Some("You feel your memories fade."));

                    let exp = with_state_mut(|state| {
                        let mut exp = state.py.misc.exp / 5;
                        if state.py.misc.exp > SHRT_MAX {
                            let scale = i32::MAX / state.py.misc.exp;
                            exp += (random_number_state(state, scale) * state.py.misc.exp)
                                / (scale * 5);
                        } else {
                            exp += random_number_state(state, state.py.misc.exp) / 5;
                        }
                        exp
                    });
                    spell_lose_exp(exp);
                    true
                } else {
                    identified
                }
            }
            35 => {
                let _ = player_cure_poison();
                with_state_mut(|state| {
                    if state.py.flags.food > 150 {
                        state.py.flags.food = 150;
                    }
                    state.py.flags.paralysis = 4;
                });
                terminal::print_message(Some("The potion makes you vomit!"));
                true
            }
            36 => {
                if with_state(|state| state.py.flags.invulnerability == 0) {
                    identified = true;
                }
                with_state_mut(|state| {
                    state.py.flags.invulnerability = state
                        .py
                        .flags
                        .invulnerability
                        .wrapping_add((random_number_state(state, 10) + 10) as i16);
                });
                identified
            }
            37 => {
                if with_state(|state| state.py.flags.heroism == 0) {
                    identified = true;
                }
                with_state_mut(|state| {
                    state.py.flags.heroism = state
                        .py
                        .flags
                        .heroism
                        .wrapping_add((random_number_state(state, 25) + 25) as i16);
                });
                identified
            }
            38 => {
                if with_state(|state| state.py.flags.super_heroism == 0) {
                    identified = true;
                }
                with_state_mut(|state| {
                    state.py.flags.super_heroism = state
                        .py
                        .flags
                        .super_heroism
                        .wrapping_add((random_number_state(state, 25) + 25) as i16);
                });
                identified
            }
            39 => player_remove_fear(),
            40 => spell_restore_player_levels(),
            41 => {
                if with_state(|state| state.py.flags.heat_resistance == 0) {
                    identified = true;
                }
                with_state_mut(|state| {
                    state.py.flags.heat_resistance = state
                        .py
                        .flags
                        .heat_resistance
                        .wrapping_add((random_number_state(state, 10) + 10) as i16);
                });
                identified
            }
            42 => {
                if with_state(|state| state.py.flags.cold_resistance == 0) {
                    identified = true;
                }
                with_state_mut(|state| {
                    state.py.flags.cold_resistance = state
                        .py
                        .flags
                        .cold_resistance
                        .wrapping_add((random_number_state(state, 10) + 10) as i16);
                });
                identified
            }
            43 => {
                if with_state(|state| state.py.flags.detect_invisible == 0) {
                    identified = true;
                }
                let adjustment = with_state_mut(|state| random_number_state(state, 12) + 12);
                player_detect_invisible(adjustment);
                identified
            }
            44 => spell_slow_poison(),
            45 => player_cure_poison(),
            46 => {
                let needs_restore =
                    with_state(|state| state.py.misc.current_mana < state.py.misc.mana);
                if needs_restore {
                    with_state_mut(|state| state.py.misc.current_mana = state.py.misc.mana);
                    terminal::print_message(Some("Your feel your head clear."));
                    print_character_current_mana();
                    true
                } else {
                    identified
                }
            }
            47 => {
                if with_state(|state| state.py.flags.timed_infra == 0) {
                    terminal::print_message(Some("Your eyes begin to tingle."));
                    identified = true;
                }
                with_state_mut(|state| {
                    state.py.flags.timed_infra = state
                        .py
                        .flags
                        .timed_infra
                        .wrapping_add((100 + random_number_state(state, 100)) as i16);
                });
                identified
            }
            _ => {
                terminal::print_message(Some("Internal error in potion()"));
                identified
            }
        };
    }

    identified
}

/// C++ `player_quaff.cpp` lines 337–384.
pub fn quaff() {
    with_state_mut(|state| state.game.player_free_turn = true);

    if with_state(|state| state.py.pack.unique_items) == 0 {
        terminal::print_message(Some("But you are not carrying anything."));
        return;
    }

    let mut item_pos_begin = 0;
    let mut item_pos_end = 0;
    if !inventory_find_range(
        i32::from(TV_POTION1),
        i32::from(TV_POTION2),
        &mut item_pos_begin,
        &mut item_pos_end,
    ) {
        terminal::print_message(Some("You are not carrying any potions."));
        return;
    }

    let mut item_id = 0;
    if !inventory_get_input_for_item_id(
        &mut item_id,
        "Quaff which potion?",
        item_pos_begin,
        item_pos_end,
        None,
        None,
    ) {
        return;
    }

    with_state_mut(|state| state.game.player_free_turn = false);

    let (flags, category_id, sub_category_id, identification, misc_use) = with_state(|state| {
        let item = state.py.inventory[item_id as usize];
        (
            item.flags,
            item.category_id,
            item.sub_category_id,
            item.identification,
            item.misc_use,
        )
    });

    let identified = if flags == 0 {
        terminal::print_message(Some("You feel less thirsty."));
        true
    } else {
        player_drink_potion(flags, category_id)
    };

    if identified {
        if !item_set_colorless_as_identified(category_id, sub_category_id, identification) {
            with_state_mut(|state| {
                state.py.misc.exp +=
                    (i32::from(state.py.inventory[item_id as usize].depth_first_found)
                        + (i32::from(state.py.misc.level) >> 1))
                        / i32::from(state.py.misc.level);
            });
            display_character_experience();
            item_identify(&mut item_id);
        }
    } else if !item_set_colorless_as_identified(category_id, sub_category_id, identification) {
        with_state(|state| item_set_as_tried(state.py.inventory[item_id as usize]));
    }

    player_ingest_food(i32::from(misc_use));
    item_type_remaining_count_description(item_id);
    inventory_destroy_item(item_id);
}
