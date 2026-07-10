//! Port of `src/player_eat.cpp` — see `phase_2`.

use crate::config::player::status::{PY_HUNGRY, PY_WEAK};
use crate::config::player::{PLAYER_FOOD_FULL, PLAYER_FOOD_MAX};
use crate::dice::{dice_roll, Dice};
use crate::game::{random_number, with_state, with_state_mut};
use crate::helpers::get_and_clear_first_bit;
use crate::identification::{
    item_identify, item_set_as_tried, item_set_colorless_as_identified,
    item_type_remaining_count_description,
};
use crate::inventory::{inventory_destroy_item, inventory_find_range, Inventory};
use crate::player::{player_takes_hit, PlayerAttr};
use crate::player_magic::{player_cure_blindness, player_cure_confusion, player_cure_poison};
use crate::player_stats::player_stat_restore;
use crate::spells::{spell_change_player_hit_points, spell_lose_con, spell_lose_str};
use crate::treasure::{TV_FOOD, TV_NEVER};
use crate::types::{Vtype_t, MORIA_MESSAGE_SIZE};
use crate::ui::{display_character_experience, draw_cave_panel, print_character_hunger_status};
use crate::ui_inventory::inventory_get_input_for_item_id;
use crate::ui_io::terminal;

fn vtype_label(text: &str) -> Vtype_t {
    let mut buf = [0u8; MORIA_MESSAGE_SIZE];
    let bytes = text.as_bytes();
    let len = bytes.len().min(MORIA_MESSAGE_SIZE - 1);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf
}

fn apply_food_effect(item: &Inventory, item_flags: &mut u32) -> bool {
    let depth = i32::from(item.depth_first_found);

    match get_and_clear_first_bit(item_flags) + 1 {
        1 => {
            let roll = random_number(10);
            with_state_mut(|state| {
                state.py.flags.poisoned += (roll + depth) as i16;
            });
            true
        }
        2 => {
            let roll = random_number(250);
            with_state_mut(|state| {
                state.py.flags.blind += (roll + 10 * depth + 100) as i16;
            });
            draw_cave_panel();
            terminal::print_message(Some("A veil of darkness surrounds you."));
            true
        }
        3 => {
            let roll = random_number(10);
            with_state_mut(|state| {
                state.py.flags.afraid += (roll + depth) as i16;
            });
            terminal::print_message(Some("You feel terrified!"));
            true
        }
        4 => {
            let roll = random_number(10);
            with_state_mut(|state| {
                state.py.flags.confused += (roll + depth) as i16;
            });
            terminal::print_message(Some("You feel drugged."));
            true
        }
        5 => {
            let roll = random_number(200);
            with_state_mut(|state| {
                state.py.flags.image += (roll + 25 * depth + 200) as i16;
            });
            terminal::print_message(Some("You feel drugged."));
            true
        }
        6 => player_cure_poison(),
        7 => player_cure_blindness(),
        8 => {
            if with_state(|state| state.py.flags.afraid) > 1 {
                with_state_mut(|state| state.py.flags.afraid = 1);
                true
            } else {
                false
            }
        }
        9 => player_cure_confusion(),
        10 => {
            spell_lose_str();
            true
        }
        11 => {
            spell_lose_con();
            true
        }
        16 => {
            if player_stat_restore(PlayerAttr::A_STR) {
                terminal::print_message(Some("You feel your strength returning."));
                true
            } else {
                false
            }
        }
        17 => {
            if player_stat_restore(PlayerAttr::A_CON) {
                terminal::print_message(Some("You feel your health returning."));
                true
            } else {
                false
            }
        }
        18 => {
            if player_stat_restore(PlayerAttr::A_INT) {
                terminal::print_message(Some("Your head spins a moment."));
                true
            } else {
                false
            }
        }
        19 => {
            if player_stat_restore(PlayerAttr::A_WIS) {
                terminal::print_message(Some("You feel your wisdom returning."));
                true
            } else {
                false
            }
        }
        20 => {
            if player_stat_restore(PlayerAttr::A_DEX) {
                terminal::print_message(Some("You feel more dexterous."));
                true
            } else {
                false
            }
        }
        21 => {
            if player_stat_restore(PlayerAttr::A_CHR) {
                terminal::print_message(Some("Your skin stops itching."));
                true
            } else {
                false
            }
        }
        22 => spell_change_player_hit_points(random_number(6)),
        23 => spell_change_player_hit_points(random_number(12)),
        24 => spell_change_player_hit_points(random_number(18)),
        26 => spell_change_player_hit_points(dice_roll(Dice { dice: 3, sides: 12 })),
        27 => {
            player_takes_hit(random_number(18), &vtype_label("poisonous food."));
            true
        }
        _ => {
            terminal::print_message(Some("Internal error in playerEat()"));
            false
        }
    }
}

/// C++ `player_eat.cpp` lines 38–233.
pub fn player_eat() {
    with_state_mut(|state| state.game.player_free_turn = true);

    if with_state(|state| state.py.pack.unique_items) == 0 {
        terminal::print_message(Some("But you are not carrying anything."));
        return;
    }

    let mut item_pos_start = 0;
    let mut item_pos_end = 0;
    if !inventory_find_range(
        i32::from(TV_FOOD),
        i32::from(TV_NEVER),
        &mut item_pos_start,
        &mut item_pos_end,
    ) {
        terminal::print_message(Some("You are not carrying any food."));
        return;
    }

    let mut item_id = 0;
    if !inventory_get_input_for_item_id(
        &mut item_id,
        "Eat what?",
        item_pos_start,
        item_pos_end,
        None,
        None,
    ) {
        return;
    }

    with_state_mut(|state| state.game.player_free_turn = false);

    let mut identified = false;
    let mut item_flags = with_state(|state| state.py.inventory[item_id as usize].flags);

    while item_flags != 0 {
        let item = with_state(|state| state.py.inventory[item_id as usize]);
        if apply_food_effect(&item, &mut item_flags) {
            identified = true;
        }
    }

    let (category_id, sub_category_id, identification, misc_use) = with_state(|state| {
        let item = state.py.inventory[item_id as usize];
        (
            item.category_id,
            item.sub_category_id,
            item.identification,
            item.misc_use,
        )
    });

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
        let item = with_state(|state| state.py.inventory[item_id as usize]);
        item_set_as_tried(item);
    }

    player_ingest_food(i32::from(misc_use));

    with_state_mut(|state| {
        state.py.flags.status &= !(PY_WEAK | PY_HUNGRY);
    });
    print_character_hunger_status();

    item_type_remaining_count_description(item_id);
    inventory_destroy_item(item_id);
}

/// C++ `player_eat.cpp` lines 236–264.
pub fn player_ingest_food(amount: i32) {
    let message = with_state_mut(|state| {
        if state.py.flags.food < 0 {
            state.py.flags.food = 0;
        }

        state.py.flags.food += amount as i16;

        if state.py.flags.food > PLAYER_FOOD_MAX as i16 {
            let mut extra = i32::from(state.py.flags.food) - i32::from(PLAYER_FOOD_MAX);
            if extra > amount {
                extra = amount;
            }
            let penalty = extra / 50;

            state.py.flags.slow += penalty as i16;

            if extra == amount {
                state.py.flags.food = (i32::from(state.py.flags.food) - amount + penalty) as i16;
            } else {
                state.py.flags.food = (i32::from(PLAYER_FOOD_MAX) + penalty) as i16;
            }

            Some("You are bloated from overeating.")
        } else if state.py.flags.food > PLAYER_FOOD_FULL as i16 {
            Some("You are full.")
        } else {
            None
        }
    });

    if let Some(msg) = message {
        terminal::print_message(Some(msg));
    }
}
