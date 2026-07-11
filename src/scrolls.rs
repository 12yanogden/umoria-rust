//! Scroll reading and scroll-specific effects

use crate::config::monsters::defense::CD_UNDEAD;
use crate::config::treasure::flags::TR_CURSED;
use crate::dice::max_dice_roll;
use crate::game::{random_number, with_state, with_state_mut};
use crate::helpers::get_and_clear_first_bit;
use crate::identification::{
    item_description, item_identify, item_remove_magic_naming, item_set_as_tried,
    item_set_colorless_as_identified, item_type_remaining_count_description,
};
use crate::inventory::{
    inventory_destroy_item, inventory_find_range, inventory_item_remove_curse, Inventory,
    PlayerEquipment,
};
use crate::monster::monster_sleep;
use crate::monster_manager::{monster_summon, monster_summon_undead};
use crate::player::{
    player_adjust_bonuses_for_item, player_bless, player_no_light, player_protect_evil,
    player_recalculate_bonuses, player_teleport, player_worn_item_is_cursed,
};
use crate::spells::{
    spell_aggravate_monsters, spell_create_food, spell_darken_area,
    spell_destroy_adjacent_doors_traps, spell_destroy_area,
    spell_detect_invisible_creatures_within_vicinity, spell_detect_objects_within_vicinity,
    spell_detect_secret_doors_within_vicinity, spell_detect_traps_within_vicinity,
    spell_detect_treasure_within_vicinity, spell_dispel_creature, spell_enchant_item,
    spell_genocide, spell_identify_item, spell_light_area, spell_map_current_area,
    spell_mass_genocide, spell_recharge_item, spell_remove_curse_from_all_worn_items,
    spell_surround_player_with_doors, spell_surround_player_with_traps, spell_warding_glyph,
};
use crate::treasure::{TV_DIGGING, TV_HAFTED, TV_NOTHING, TV_SCROLL1, TV_SCROLL2};
use crate::types::MORIA_OBJ_DESC_SIZE_LEN;
use crate::ui::display_character_experience;
use crate::ui_inventory::inventory_get_input_for_item_id;
use crate::ui_io::terminal;

/// 37
#[must_use]
pub fn player_can_read_scroll(item_pos_start: &mut i32, item_pos_end: &mut i32) -> bool {
    if with_state(|state| state.py.flags.blind > 0) {
        terminal::print_message(Some("You can't see to read the scroll."));
        return false;
    }

    if player_no_light() {
        terminal::print_message(Some("You have no light to read by."));
        return false;
    }

    if with_state(|state| state.py.flags.confused > 0) {
        terminal::print_message(Some("You are too confused to read a scroll."));
        return false;
    }

    if with_state(|state| state.py.pack.unique_items == 0) {
        terminal::print_message(Some("You are not carrying anything!"));
        return false;
    }

    if !inventory_find_range(
        i32::from(TV_SCROLL1),
        i32::from(TV_SCROLL2),
        item_pos_start,
        item_pos_end,
    ) {
        terminal::print_message(Some("You are not carrying any scrolls!"));
        return false;
    }

    true
}

/// 90
pub fn inventory_item_id_of_cursed_equipment() -> i32 {
    let mut item_count = 0;
    let mut items = [0i32; 6];

    let slots = [
        PlayerEquipment::Body,
        PlayerEquipment::Arm,
        PlayerEquipment::Outer,
        PlayerEquipment::Hands,
        PlayerEquipment::Head,
        PlayerEquipment::Feet,
    ];

    for (idx, slot) in slots.iter().enumerate() {
        if with_state(|state| state.py.inventory[*slot as usize].category_id != TV_NOTHING) {
            items[item_count] = *slot as i32;
            item_count += 1;
            let _ = idx;
        }
    }

    let mut item_id = 0;

    if item_count > 0 {
        item_id = items[(random_number(item_count as i32) - 1) as usize];
    }

    if player_worn_item_is_cursed(PlayerEquipment::Body) {
        item_id = PlayerEquipment::Body as i32;
    } else if player_worn_item_is_cursed(PlayerEquipment::Arm) {
        item_id = PlayerEquipment::Arm as i32;
    } else if player_worn_item_is_cursed(PlayerEquipment::Outer) {
        item_id = PlayerEquipment::Outer as i32;
    } else if player_worn_item_is_cursed(PlayerEquipment::Head) {
        item_id = PlayerEquipment::Head as i32;
    } else if player_worn_item_is_cursed(PlayerEquipment::Hands) {
        item_id = PlayerEquipment::Hands as i32;
    } else if player_worn_item_is_cursed(PlayerEquipment::Feet) {
        item_id = PlayerEquipment::Feet as i32;
    }

    item_id
}

fn enchant_item_field(item_id: i32, field: fn(&mut Inventory) -> &mut i16, max_bonus: i16) -> bool {
    let mut value = with_state_mut(|state| *field(&mut state.py.inventory[item_id as usize]));
    let enchanted = spell_enchant_item(&mut value, max_bonus);
    if enchanted {
        with_state_mut(|state| *field(&mut state.py.inventory[item_id as usize]) = value);
    }
    enchanted
}

fn wielded_weapon_scroll_type() -> i16 {
    with_state(|state| {
        let item = &state.py.inventory[PlayerEquipment::Wield as usize];
        if item.category_id >= TV_HAFTED && item.category_id <= TV_DIGGING {
            max_dice_roll(item.damage) as i16
        } else {
            10
        }
    })
}

fn slot_has_equipment(slot: PlayerEquipment) -> bool {
    with_state(|state| state.py.inventory[slot as usize].category_id != TV_NOTHING)
}

/// 114
#[must_use]
pub fn scroll_enchant_weapon_to_hit() -> bool {
    if with_state(|state| state.py.inventory[PlayerEquipment::Wield as usize].category_id)
        == TV_NOTHING
    {
        return false;
    }

    let desc = weapon_glow_message(" faintly!");
    terminal::print_message(Some(&desc));

    let enchanted = enchant_item_field(PlayerEquipment::Wield as i32, |item| &mut item.to_hit, 10);

    if enchanted {
        with_state_mut(|state| {
            inventory_item_remove_curse(&mut state.py.inventory[PlayerEquipment::Wield as usize]);
        });
        player_recalculate_bonuses();
    } else {
        terminal::print_message(Some("The enchantment fails."));
    }

    true
}

/// 148
#[must_use]
pub fn scroll_enchant_weapon_to_damage() -> bool {
    if with_state(|state| state.py.inventory[PlayerEquipment::Wield as usize].category_id)
        == TV_NOTHING
    {
        return false;
    }

    let desc = weapon_glow_message(" faintly!");
    terminal::print_message(Some(&desc));

    let scroll_type = wielded_weapon_scroll_type();

    let enchanted = enchant_item_field(
        PlayerEquipment::Wield as i32,
        |item| &mut item.to_damage,
        scroll_type,
    );

    if enchanted {
        with_state_mut(|state| {
            inventory_item_remove_curse(&mut state.py.inventory[PlayerEquipment::Wield as usize]);
        });
        player_recalculate_bonuses();
    } else {
        terminal::print_message(Some("The enchantment fails."));
    }

    true
}

/// 174
#[must_use]
pub fn scroll_enchant_item_to_ac() -> bool {
    let item_id = inventory_item_id_of_cursed_equipment();

    if item_id <= 0 {
        return false;
    }

    let desc = armor_glow_message(item_id, " faintly!");
    terminal::print_message(Some(&desc));

    let enchanted = enchant_item_field(item_id, |item| &mut item.to_ac, 10);

    if enchanted {
        with_state_mut(|state| {
            inventory_item_remove_curse(&mut state.py.inventory[item_id as usize]);
        });
        player_recalculate_bonuses();
    } else {
        terminal::print_message(Some("The enchantment fails."));
    }

    true
}

/// 192
pub fn scroll_identify_item(item_id: i32, is_used_up: &mut bool) -> i32 {
    terminal::print_message(Some("This is an identify scroll."));

    *is_used_up = spell_identify_item();

    let mut current_id = item_id;
    loop {
        let (category_id, flags) = with_state(|state| {
            let item = &state.py.inventory[current_id as usize];
            (item.category_id, item.flags)
        });

        if current_id > 0 && (category_id != TV_SCROLL1 || flags != 0x0000_0008) {
            current_id -= 1;
        } else {
            break;
        }
    }

    current_id
}

/// 200
#[must_use]
pub fn scroll_remove_curse() -> bool {
    if spell_remove_curse_from_all_worn_items() {
        terminal::print_message(Some("You feel as if someone is watching over you."));
        return true;
    }
    false
}

/// 213
#[must_use]
pub fn scroll_summon_monster() -> bool {
    let mut identified = false;

    let count = random_number(3);
    for _ in 0..count {
        let mut coord = with_state(|state| state.py.pos);
        identified |= monster_summon(&mut coord, false);
    }

    identified
}

/// 221
pub fn scroll_teleport_level() {
    let roll = random_number(2);
    with_state_mut(|state| {
        state.dg.current_level += ((-3) + 2 * roll) as i16;
        if state.dg.current_level < 1 {
            state.dg.current_level = 1;
        }
        state.dg.generate_new_level = true;
    });
}

/// 230
#[must_use]
pub fn scroll_confuse_monster() -> bool {
    if with_state(|state| !state.py.flags.confuse_monster) {
        terminal::print_message(Some("Your hands begin to glow."));
        with_state_mut(|state| state.py.flags.confuse_monster = true);
        return true;
    }
    false
}

/// 278
#[must_use]
pub fn scroll_enchant_weapon() -> bool {
    if with_state(|state| state.py.inventory[PlayerEquipment::Wield as usize].category_id)
        == TV_NOTHING
    {
        return false;
    }

    let desc = weapon_glow_message(" brightly!");
    terminal::print_message(Some(&desc));

    let mut enchanted = false;

    let to_hit_loops = random_number(2);
    for _ in 0..to_hit_loops {
        if enchant_item_field(PlayerEquipment::Wield as i32, |item| &mut item.to_hit, 10) {
            enchanted = true;
        }
    }

    let scroll_type = wielded_weapon_scroll_type();

    let to_dam_loops = random_number(2);
    for _ in 0..to_dam_loops {
        if enchant_item_field(
            PlayerEquipment::Wield as i32,
            |item| &mut item.to_damage,
            scroll_type,
        ) {
            enchanted = true;
        }
    }

    if enchanted {
        with_state_mut(|state| {
            inventory_item_remove_curse(&mut state.py.inventory[PlayerEquipment::Wield as usize]);
        });
        player_recalculate_bonuses();
    } else {
        terminal::print_message(Some("The enchantment fails."));
    }

    true
}

/// 308
#[must_use]
pub fn scroll_curse_weapon() -> bool {
    if with_state(|state| state.py.inventory[PlayerEquipment::Wield as usize].category_id)
        == TV_NOTHING
    {
        return false;
    }

    let desc = weapon_glow_message(" black, fades.");
    terminal::print_message(Some(&desc));

    let to_hit = (-random_number(5) - random_number(5)) as i16;
    let to_damage = (-random_number(5) - random_number(5)) as i16;

    let copy = with_state_mut(|state| {
        let item = &mut state.py.inventory[PlayerEquipment::Wield as usize];
        item_remove_magic_naming(item);
        item.to_hit = to_hit;
        item.to_damage = to_damage;
        item.to_ac = 0;
        *item
    });
    player_adjust_bonuses_for_item(copy, -1);
    with_state_mut(|state| {
        state.py.inventory[PlayerEquipment::Wield as usize].flags = TR_CURSED;
    });

    player_recalculate_bonuses();
    true
}

/// 342
#[must_use]
pub fn scroll_enchant_armor() -> bool {
    let item_id = inventory_item_id_of_cursed_equipment();

    if item_id <= 0 {
        return false;
    }

    let desc = armor_glow_message(item_id, " brightly!");
    terminal::print_message(Some(&desc));

    let mut enchanted = false;
    let loops = random_number(2) + 1;
    for _ in 0..loops {
        if enchant_item_field(item_id, |item| &mut item.to_ac, 10) {
            enchanted = true;
        }
    }

    if enchanted {
        with_state_mut(|state| {
            inventory_item_remove_curse(&mut state.py.inventory[item_id as usize]);
        });
        player_recalculate_bonuses();
    } else {
        terminal::print_message(Some("The enchantment fails."));
    }

    true
}

/// 398
#[must_use]
pub fn scroll_curse_armor() -> bool {
    let item_id = if slot_has_equipment(PlayerEquipment::Body) && random_number(4) == 1 {
        PlayerEquipment::Body as i32
    } else if slot_has_equipment(PlayerEquipment::Arm) && random_number(3) == 1 {
        PlayerEquipment::Arm as i32
    } else if slot_has_equipment(PlayerEquipment::Outer) && random_number(3) == 1 {
        PlayerEquipment::Outer as i32
    } else if slot_has_equipment(PlayerEquipment::Head) && random_number(3) == 1 {
        PlayerEquipment::Head as i32
    } else if slot_has_equipment(PlayerEquipment::Hands) && random_number(3) == 1 {
        PlayerEquipment::Hands as i32
    } else if slot_has_equipment(PlayerEquipment::Feet) && random_number(3) == 1 {
        PlayerEquipment::Feet as i32
    } else if slot_has_equipment(PlayerEquipment::Body) {
        PlayerEquipment::Body as i32
    } else if slot_has_equipment(PlayerEquipment::Arm) {
        PlayerEquipment::Arm as i32
    } else if slot_has_equipment(PlayerEquipment::Outer) {
        PlayerEquipment::Outer as i32
    } else if slot_has_equipment(PlayerEquipment::Head) {
        PlayerEquipment::Head as i32
    } else if slot_has_equipment(PlayerEquipment::Hands) {
        PlayerEquipment::Hands as i32
    } else if slot_has_equipment(PlayerEquipment::Feet) {
        PlayerEquipment::Feet as i32
    } else {
        0
    };

    if item_id <= 0 {
        return false;
    }

    let desc = armor_glow_message(item_id, " black, fades.");
    terminal::print_message(Some(&desc));

    let to_ac = (-random_number(5) - random_number(5)) as i16;
    with_state_mut(|state| {
        let item = &mut state.py.inventory[item_id as usize];
        item_remove_magic_naming(item);
        item.flags = TR_CURSED;
        item.to_hit = 0;
        item.to_damage = 0;
        item.to_ac = to_ac;
    });

    player_recalculate_bonuses();
    true
}

/// 411
#[must_use]
pub fn scroll_summon_undead() -> bool {
    let mut identified = false;

    let count = random_number(3);
    for _ in 0..count {
        let mut coord = with_state(|state| state.py.pos);
        identified |= monster_summon_undead(&mut coord);
    }

    identified
}

/// 418
pub fn scroll_word_of_recall() {
    let recall_roll = random_number(30);
    with_state_mut(|state| {
        if state.py.flags.word_of_recall == 0 {
            state.py.flags.word_of_recall = (25 + recall_roll) as i16;
        }
    });
    terminal::print_message(Some("The air about you becomes charged."));
}

fn weapon_glow_message(suffix: &str) -> String {
    let mut description = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
    let item = with_state(|state| state.py.inventory[PlayerEquipment::Wield as usize]);
    item_description(&mut description, item, false);
    format!(
        "Your {} glows{suffix}!",
        String::from_utf8_lossy(&description)
            .trim_end_matches('\0')
            .trim_end()
    )
}

fn armor_glow_message(item_id: i32, suffix: &str) -> String {
    let mut description = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
    let item = with_state(|state| state.py.inventory[item_id as usize]);
    item_description(&mut description, item, false);
    format!(
        "Your {} glows{suffix}!",
        String::from_utf8_lossy(&description)
            .trim_end_matches('\0')
            .trim_end()
    )
}

/// 615
pub fn scroll_read() {
    with_state_mut(|state| state.game.player_free_turn = true);

    let mut item_pos_start = 0;
    let mut item_pos_end = 0;
    if !player_can_read_scroll(&mut item_pos_start, &mut item_pos_end) {
        return;
    }

    let mut item_id = 0;
    if !inventory_get_input_for_item_id(
        &mut item_id,
        "Read which scroll?",
        item_pos_start,
        item_pos_end,
        None,
        None,
    ) {
        return;
    }

    with_state_mut(|state| state.game.player_free_turn = false);

    let mut used_up = true;
    let mut identified = false;

    let mut item_flags = with_state(|state| state.py.inventory[item_id as usize].flags);
    let category_id = with_state(|state| state.py.inventory[item_id as usize].category_id);

    while item_flags != 0 {
        let mut scroll_type = get_and_clear_first_bit(&mut item_flags) + 1;

        if category_id == TV_SCROLL2 {
            scroll_type += 32;
        }

        identified = match scroll_type {
            1 => scroll_enchant_weapon_to_hit(),
            2 => scroll_enchant_weapon_to_damage(),
            3 => scroll_enchant_item_to_ac(),
            4 => {
                item_id = scroll_identify_item(item_id, &mut used_up);
                true
            }
            5 => scroll_remove_curse(),
            6 => spell_light_area(with_state(|state| state.py.pos)),
            7 => scroll_summon_monster(),
            8 => {
                player_teleport(10);
                true
            }
            9 => {
                player_teleport(100);
                true
            }
            10 => {
                scroll_teleport_level();
                true
            }
            11 => scroll_confuse_monster(),
            12 => {
                spell_map_current_area();
                true
            }
            13 => monster_sleep(with_state(|state| state.py.pos)),
            14 => {
                spell_warding_glyph();
                true
            }
            15 => spell_detect_treasure_within_vicinity(),
            16 => spell_detect_objects_within_vicinity(),
            17 => spell_detect_traps_within_vicinity(),
            18 => spell_detect_secret_doors_within_vicinity(),
            19 => {
                terminal::print_message(Some("This is a mass genocide scroll."));
                let _ = spell_mass_genocide();
                true
            }
            20 => spell_detect_invisible_creatures_within_vicinity(),
            21 => {
                terminal::print_message(Some("There is a high pitched humming noise."));
                let _ = spell_aggravate_monsters(20);
                true
            }
            22 => spell_surround_player_with_traps(),
            23 => spell_destroy_adjacent_doors_traps(),
            24 => spell_surround_player_with_doors(),
            25 => {
                terminal::print_message(Some("This is a Recharge-Item scroll."));
                used_up = spell_recharge_item(60);
                true
            }
            26 => {
                terminal::print_message(Some("This is a genocide scroll."));
                let _ = spell_genocide();
                true
            }
            27 => spell_darken_area(with_state(|state| state.py.pos)),
            28 => player_protect_evil(),
            29 => {
                spell_create_food();
                true
            }
            30 => spell_dispel_creature(i32::from(CD_UNDEAD), 60),
            33 => scroll_enchant_weapon(),
            34 => scroll_curse_weapon(),
            35 => scroll_enchant_armor(),
            36 => scroll_curse_armor(),
            37 => scroll_summon_undead(),
            38 => {
                player_bless(random_number(12) + 6);
                true
            }
            39 => {
                player_bless(random_number(24) + 12);
                true
            }
            40 => {
                player_bless(random_number(48) + 24);
                true
            }
            41 => {
                scroll_word_of_recall();
                true
            }
            42 => {
                spell_destroy_area(with_state(|state| state.py.pos));
                true
            }
            _ => {
                terminal::print_message(Some("Internal error in scroll()"));
                identified
            }
        };
    }

    let (cat, sub, ident) = with_state(|state| {
        let item = &state.py.inventory[item_id as usize];
        (item.category_id, item.sub_category_id, item.identification)
    });

    if identified {
        if !item_set_colorless_as_identified(cat, sub, ident) {
            with_state_mut(|state| {
                let item = &state.py.inventory[item_id as usize];
                state.py.misc.exp += (i32::from(item.depth_first_found)
                    + (i32::from(state.py.misc.level) >> 1))
                    / i32::from(state.py.misc.level);
            });
            display_character_experience();
            item_identify(&mut item_id);
        }
    } else if !item_set_colorless_as_identified(cat, sub, ident) {
        let item = with_state(|state| state.py.inventory[item_id as usize]);
        item_set_as_tried(item);
    }

    if used_up {
        item_type_remaining_count_description(item_id);
        inventory_destroy_item(item_id);
    }
}
