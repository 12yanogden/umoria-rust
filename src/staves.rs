//! Port of src/staves.cpp — staff and wand use.

use crate::config::identification::ID_EMPTY;
use crate::config::monsters::defense::CD_EVIL;
use crate::config::player::PLAYER_USE_DEVICE_DIFFICULTY;
use crate::data_player::{CLASS_LEVEL_ADJ, SPELL_NAMES};
use crate::dice::{dice_roll, Dice};
use crate::game::{get_random_direction, random_number, with_state, with_state_mut};
use crate::helpers::get_and_clear_first_bit;
use crate::identification::{
    item_append_to_inscription, item_charges_remaining_description, item_identify,
    item_set_as_tried, item_set_colorless_as_identified, spell_item_identified,
};
use crate::inventory::inventory_find_range;
use crate::monster_manager::monster_summon;
use crate::player::{player_stat_adjustment_wisdom_intelligence, player_teleport, PlayerAttr, PlayerClassLevelAdj};
use crate::player_magic::{player_cure_blindness, player_cure_confusion, player_cure_poison};
use crate::spells::{
    spell_build_wall, spell_change_monster_hit_points, spell_change_player_hit_points,
    spell_clone_monster, spell_confuse_monster, spell_darken_area, spell_destroy_area,
    spell_destroy_doors_traps_in_direction, spell_detect_evil,
    spell_detect_invisible_creatures_within_vicinity, spell_detect_objects_within_vicinity,
    spell_detect_secret_doors_within_vicinity, spell_detect_traps_within_vicinity,
    spell_detect_treasure_within_vicinity, spell_dispel_creature, spell_disarm_all_in_direction,
    spell_drain_life_from_monster, spell_earthquake, spell_fire_ball, spell_fire_bolt,
    spell_light_area, spell_light_line, spell_mass_polymorph, spell_polymorph_monster,
    spell_remove_curse_from_all_worn_items, spell_sleep_all_monsters, spell_sleep_monster,
    spell_speed_all_monsters, spell_speed_monster, spell_starlite,
    spell_teleport_away_monster_in_direction, spell_wall_to_mud, MagicSpellFlags,
};
use crate::treasure::{TV_NEVER, TV_STAFF, TV_WAND};
use crate::types::Coord_t;
use crate::ui::display_character_experience;
use crate::ui_inventory::inventory_get_input_for_item_id;
use crate::ui_io::{get_direction_with_memory, terminal};

fn staff_player_is_carrying(item_pos_start: &mut i32, item_pos_end: &mut i32) -> bool {
    if with_state(|state| state.py.pack.unique_items) == 0 {
        terminal::print_message(Some("But you are not carrying anything."));
        return false;
    }

    if !inventory_find_range(
        i32::from(TV_STAFF),
        i32::from(TV_NEVER),
        item_pos_start,
        item_pos_end,
    ) {
        terminal::print_message(Some("You are not carrying any staffs."));
        return false;
    }

    true
}

fn staff_player_can_use(item_id: i32) -> bool {
    let item = with_state(|state| state.py.inventory[item_id as usize]);
    let mut chance = with_state(|state| i32::from(state.py.misc.saving_throw));
    chance += player_stat_adjustment_wisdom_intelligence(PlayerAttr::A_INT);
    chance -= i32::from(item.depth_first_found) - 5;
    chance += i32::from(
        CLASS_LEVEL_ADJ[with_state(|state| state.py.misc.class_id as usize)]
            [PlayerClassLevelAdj::DEVICE as usize],
    ) * i32::from(with_state(|state| state.py.misc.level))
        / 3;

    if with_state(|state| state.py.flags.confused) > 0 {
        chance /= 2;
    }

    if chance < i32::from(PLAYER_USE_DEVICE_DIFFICULTY)
        && random_number(i32::from(PLAYER_USE_DEVICE_DIFFICULTY) - chance + 1) == 1
    {
        chance = i32::from(PLAYER_USE_DEVICE_DIFFICULTY);
    }

    if chance < 1 {
        chance = 1;
    }

    if random_number(chance) < i32::from(PLAYER_USE_DEVICE_DIFFICULTY) {
        terminal::print_message(Some("You failed to use the staff properly."));
        return false;
    }

    if item.misc_use < 1 {
        terminal::print_message(Some("The staff has no charges left."));
        if !spell_item_identified(item) {
            with_state_mut(|state| {
                item_append_to_inscription(&mut state.py.inventory[item_id as usize], ID_EMPTY);
            });
        }
        return false;
    }

    true
}

fn staff_discharge(item_id: i32) -> bool {
    let mut identified = false;

    with_state_mut(|state| state.py.inventory[item_id as usize].misc_use -= 1);

    let mut flags = with_state(|state| state.py.inventory[item_id as usize].flags);
    let player_pos = with_state(|state| state.py.pos);

    while flags != 0 {
        let spell_type = get_and_clear_first_bit(&mut flags) + 1;

        match spell_type {
            1 => identified = spell_light_area(player_pos),
            2 => identified = spell_detect_secret_doors_within_vicinity(),
            3 => identified = spell_detect_traps_within_vicinity(),
            4 => identified = spell_detect_treasure_within_vicinity(),
            5 => identified = spell_detect_objects_within_vicinity(),
            6 => {
                player_teleport(100);
                identified = true;
            }
            7 => {
                identified = true;
                spell_earthquake();
            }
            8 => {
                identified = false;
                for _ in 0..random_number(4) {
                    let mut coord = player_pos;
                    identified |= monster_summon(&mut coord, false);
                }
            }
            10 => {
                identified = true;
                spell_destroy_area(player_pos);
            }
            11 => {
                identified = true;
                spell_starlite(player_pos);
            }
            12 => identified = spell_speed_all_monsters(1),
            13 => identified = spell_speed_all_monsters(-1),
            14 => identified = spell_sleep_all_monsters(),
            15 => identified = spell_change_player_hit_points(random_number(8)),
            16 => identified = spell_detect_invisible_creatures_within_vicinity(),
            17 => {
                if with_state(|state| state.py.flags.fast) == 0 {
                    identified = true;
                }
                with_state_mut(|state| {
                    state.py.flags.fast += (random_number(30) + 15) as i16;
                });
            }
            18 => {
                if with_state(|state| state.py.flags.slow) == 0 {
                    identified = true;
                }
                with_state_mut(|state| {
                    state.py.flags.slow += (random_number(30) + 15) as i16;
                });
            }
            19 => identified = spell_mass_polymorph(),
            20 => {
                if spell_remove_curse_from_all_worn_items() {
                    if with_state(|state| state.py.flags.blind) < 1 {
                        terminal::print_message(Some("The staff glows blue for a moment.."));
                    }
                    identified = true;
                }
            }
            21 => identified = spell_detect_evil(),
            22 => {
                if player_cure_blindness() || player_cure_poison() || player_cure_confusion() {
                    identified = true;
                }
            }
            23 => identified = spell_dispel_creature(i32::from(CD_EVIL), 60),
            25 => identified = spell_darken_area(player_pos),
            32 => {}
            _ => terminal::print_message(Some("Internal error in staffs()")),
        }
    }

    identified
}

/// C++ staves.cpp lines 200–238.
pub fn staff_use() {
    with_state_mut(|state| state.game.player_free_turn = true);

    let mut item_pos_start = -1;
    let mut item_pos_end = -1;
    if !staff_player_is_carrying(&mut item_pos_start, &mut item_pos_end) {
        return;
    }

    let mut item_id = 0;
    if !inventory_get_input_for_item_id(
        &mut item_id,
        "Use which staff?",
        item_pos_start,
        item_pos_end,
        None,
        None,
    ) {
        return;
    }

    with_state_mut(|state| state.game.player_free_turn = false);

    if !staff_player_can_use(item_id) {
        return;
    }

    let identified = staff_discharge(item_id);

    let (category_id, sub_category_id, identification) = with_state(|state| {
        let item = state.py.inventory[item_id as usize];
        (item.category_id, item.sub_category_id, item.identification)
    });

    if identified {
        if !item_set_colorless_as_identified(category_id, sub_category_id, identification) {
            with_state_mut(|state| {
                state.py.misc.exp += (i32::from(
                    state.py.inventory[item_id as usize].depth_first_found,
                ) + (i32::from(state.py.misc.level) >> 1))
                    / i32::from(state.py.misc.level);
            });
            display_character_experience();
            item_identify(&mut item_id);
        }
    } else if !item_set_colorless_as_identified(category_id, sub_category_id, identification) {
        with_state(|state| item_set_as_tried(state.py.inventory[item_id as usize]));
    }

    item_charges_remaining_description(item_id);
}

fn wand_discharge(item_id: i32, direction: i32) -> bool {
    with_state_mut(|state| state.py.inventory[item_id as usize].misc_use -= 1);

    let mut identified = false;
    let mut flags = with_state(|state| state.py.inventory[item_id as usize].flags);
    let mut coord = Coord_t { y: 0, x: 0 };

    while flags != 0 {
        coord.y = with_state(|state| state.py.pos.y);
        coord.x = with_state(|state| state.py.pos.x);

        match get_and_clear_first_bit(&mut flags) + 1 {
            1 => {
                terminal::print_message(Some("A line of blue shimmering light appears."));
                spell_light_line(coord, direction);
                identified = true;
            }
            2 => {
                spell_fire_bolt(
                    coord,
                    direction,
                    dice_roll(Dice { dice: 4, sides: 8 }),
                    MagicSpellFlags::Lightning,
                    SPELL_NAMES[8],
                );
                identified = true;
            }
            3 => {
                spell_fire_bolt(
                    coord,
                    direction,
                    dice_roll(Dice { dice: 6, sides: 8 }),
                    MagicSpellFlags::Frost,
                    SPELL_NAMES[14],
                );
                identified = true;
            }
            4 => {
                spell_fire_bolt(
                    coord,
                    direction,
                    dice_roll(Dice { dice: 9, sides: 8 }),
                    MagicSpellFlags::Fire,
                    SPELL_NAMES[22],
                );
                identified = true;
            }
            5 => identified = spell_wall_to_mud(coord, direction),
            6 => identified = spell_polymorph_monster(coord, direction),
            7 => {
                identified = spell_change_monster_hit_points(
                    coord,
                    direction,
                    -dice_roll(Dice { dice: 4, sides: 6 }),
                );
            }
            8 => identified = spell_speed_monster(coord, direction, 1),
            9 => identified = spell_speed_monster(coord, direction, -1),
            10 => identified = spell_confuse_monster(coord, direction),
            11 => identified = spell_sleep_monster(coord, direction),
            12 => identified = spell_drain_life_from_monster(coord, direction),
            13 => identified = spell_destroy_doors_traps_in_direction(coord, direction),
            14 => {
                spell_fire_bolt(
                    coord,
                    direction,
                    dice_roll(Dice { dice: 2, sides: 6 }),
                    MagicSpellFlags::MagicMissile,
                    SPELL_NAMES[0],
                );
                identified = true;
            }
            15 => identified = spell_build_wall(coord, direction),
            16 => identified = spell_clone_monster(coord, direction),
            17 => identified = spell_teleport_away_monster_in_direction(coord, direction),
            18 => identified = spell_disarm_all_in_direction(coord, direction),
            19 => {
                spell_fire_ball(
                    coord,
                    direction,
                    32,
                    MagicSpellFlags::Lightning,
                    "Lightning Ball",
                );
                identified = true;
            }
            20 => {
                spell_fire_ball(coord, direction, 48, MagicSpellFlags::Frost, "Cold Ball");
                identified = true;
            }
            21 => {
                spell_fire_ball(
                    coord,
                    direction,
                    72,
                    MagicSpellFlags::Fire,
                    SPELL_NAMES[28],
                );
                identified = true;
            }
            22 => {
                spell_fire_ball(
                    coord,
                    direction,
                    12,
                    MagicSpellFlags::PoisonGas,
                    SPELL_NAMES[6],
                );
                identified = true;
            }
            23 => {
                spell_fire_ball(coord, direction, 60, MagicSpellFlags::Acid, "Acid Ball");
                identified = true;
            }
            24 => flags = 1u32 << (random_number(23) - 1),
            _ => terminal::print_message(Some("Internal error in wands()")),
        }
    }

    identified
}

/// C++ staves.cpp lines 376–452.
pub fn wand_aim() {
    with_state_mut(|state| state.game.player_free_turn = true);

    if with_state(|state| state.py.pack.unique_items) == 0 {
        terminal::print_message(Some("But you are not carrying anything."));
        return;
    }

    let mut item_pos_start = -1;
    let mut item_pos_end = -1;
    if !inventory_find_range(
        i32::from(TV_WAND),
        i32::from(TV_NEVER),
        &mut item_pos_start,
        &mut item_pos_end,
    ) {
        terminal::print_message(Some("You are not carrying any wands."));
        return;
    }

    let mut item_id = 0;
    if !inventory_get_input_for_item_id(
        &mut item_id,
        "Aim which wand?",
        item_pos_start,
        item_pos_end,
        None,
        None,
    ) {
        return;
    }

    with_state_mut(|state| state.game.player_free_turn = false);

    let mut direction = 0;
    if !get_direction_with_memory(None, &mut direction) {
        return;
    }

    if with_state(|state| state.py.flags.confused) > 0 {
        terminal::print_message(Some("You are confused."));
        direction = get_random_direction();
    }

    let player_class_lev_adj = with_state(|state| {
        i32::from(CLASS_LEVEL_ADJ[state.py.misc.class_id as usize]
            [PlayerClassLevelAdj::DEVICE as usize])
            * i32::from(state.py.misc.level)
            / 3
    });

    let item = with_state(|state| state.py.inventory[item_id as usize]);
    let mut chance = with_state(|state| i32::from(state.py.misc.saving_throw))
        + player_stat_adjustment_wisdom_intelligence(PlayerAttr::A_INT)
        - i32::from(item.depth_first_found)
        + player_class_lev_adj;

    if with_state(|state| state.py.flags.confused) > 0 {
        chance /= 2;
    }

    if chance < i32::from(PLAYER_USE_DEVICE_DIFFICULTY)
        && random_number(i32::from(PLAYER_USE_DEVICE_DIFFICULTY) - chance + 1) == 1
    {
        chance = i32::from(PLAYER_USE_DEVICE_DIFFICULTY);
    }

    if chance <= 0 {
        chance = 1;
    }

    if random_number(chance) < i32::from(PLAYER_USE_DEVICE_DIFFICULTY) {
        terminal::print_message(Some("You failed to use the wand properly."));
        return;
    }

    if item.misc_use < 1 {
        terminal::print_message(Some("The wand has no charges left."));
        if !spell_item_identified(item) {
            with_state_mut(|state| {
                item_append_to_inscription(&mut state.py.inventory[item_id as usize], ID_EMPTY);
            });
        }
        return;
    }

    let identified = wand_discharge(item_id, direction);

    let (category_id, sub_category_id, identification) = with_state(|state| {
        let item = state.py.inventory[item_id as usize];
        (item.category_id, item.sub_category_id, item.identification)
    });

    if identified {
        if !item_set_colorless_as_identified(category_id, sub_category_id, identification) {
            with_state_mut(|state| {
                state.py.misc.exp += (i32::from(
                    state.py.inventory[item_id as usize].depth_first_found,
                ) + (i32::from(state.py.misc.level) >> 1))
                    / i32::from(state.py.misc.level);
            });
            display_character_experience();
            item_identify(&mut item_id);
        }
    } else if !item_set_colorless_as_identified(category_id, sub_category_id, identification) {
        with_state(|state| item_set_as_tried(state.py.inventory[item_id as usize]));
    }

    item_charges_remaining_description(item_id);
}
