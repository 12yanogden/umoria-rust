//! Pack/equipment bookkeeping and elemental item damage

use std::os::raw::c_char;

use crate::config::dungeon::objects::OBJ_NOTHING;
use crate::config::identification::ID_EMPTY;
use crate::config::player::status::PY_STR_WGT;
use crate::config::treasure::flags::{TR_CURSED, TR_RES_ACID, TR_RES_FIRE};
use crate::data_treasure::GAME_OBJECTS;
use crate::dice::Dice;
use crate::dungeon::dungeon_delete_object;
use crate::game::{random_number, random_number_state, with_state, with_state_mut};
use crate::game_objects::popt;
use crate::identification::{
    item_append_to_inscription, item_description, item_set_colorless_as_identified_for_state,
    object_position_offset, spell_item_identified, SpecialNameIds,
};
use crate::player::{player_recalculate_bonuses, player_take_off, player_takes_hit, PlayerAttr};
use crate::treasure::{
    TV_ARROW, TV_BOLT, TV_BOOTS, TV_BOW, TV_CHEST, TV_CLOAK, TV_CLOSED_DOOR, TV_FLASK, TV_FOOD,
    TV_GLOVES, TV_HAFTED, TV_HARD_ARMOR, TV_HELM, TV_MISC, TV_NOTHING, TV_OPEN_DOOR, TV_POLEARM,
    TV_POTION1, TV_POTION2, TV_RING, TV_SCROLL1, TV_SCROLL2, TV_SHIELD, TV_SOFT_ARMOR, TV_SPIKE,
    TV_STAFF, TV_SWORD, TV_WAND,
};
use crate::types::{Vtype_t, MORIA_OBJ_DESC_SIZE};
use crate::ui_io::terminal::print_message;

pub const PLAYER_INVENTORY_SIZE: u8 = 34;

pub const ITEM_NEVER_STACK_MIN: u8 = 0;
pub const ITEM_NEVER_STACK_MAX: u8 = 63;
pub const ITEM_SINGLE_STACK_MIN: u8 = 64;
pub const ITEM_SINGLE_STACK_MAX: u8 = 192;
pub const ITEM_GROUP_MIN: u8 = 192;
pub const ITEM_GROUP_MAX: u8 = 255;

pub const INSCRIP_SIZE: u8 = 13;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PlayerEquipment {
    Wield = 22,
    Head,
    Neck,
    Body,
    Arm,
    Hands,
    Right,
    Left,
    Feet,
    Outer,
    Light,
    Auxiliary,
}

#[derive(Clone, Copy, Debug)]
pub struct Inventory {
    pub id: u16,
    pub special_name_id: u8,
    pub inscription: [c_char; INSCRIP_SIZE as usize],
    pub flags: u32,
    pub category_id: u8,
    pub sprite: u8,
    pub misc_use: i16,
    pub cost: i32,
    pub sub_category_id: u8,
    pub items_count: u8,
    pub weight: u16,
    pub to_hit: i16,
    pub to_damage: i16,
    pub ac: i16,
    pub to_ac: i16,
    pub damage: Dice,
    pub depth_first_found: u8,
    pub identification: u8,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            id: 0,
            special_name_id: 0,
            inscription: [0; INSCRIP_SIZE as usize],
            flags: 0,
            category_id: 0,
            sprite: 0,
            misc_use: 0,
            cost: 0,
            sub_category_id: 0,
            items_count: 0,
            weight: 0,
            to_hit: 0,
            to_damage: 0,
            ac: 0,
            to_ac: 0,
            damage: Dice { dice: 0, sides: 0 },
            depth_first_found: 0,
            identification: 0,
        }
    }
}

/// 17
pub fn inventory_collect_all_item_flags() -> u32 {
    with_state(|state| {
        let mut flags = 0u32;
        for i in PlayerEquipment::Wield as usize..PlayerEquipment::Light as usize {
            flags |= state.py.inventory[i].flags;
        }
        flags
    })
}

/// 358
pub fn inventory_item_copy_to(from_item_id: i16, to_item: &mut Inventory) {
    let from = &GAME_OBJECTS[from_item_id as usize];
    to_item.id = from_item_id as u16;
    to_item.special_name_id = SpecialNameIds::SN_NULL as u8;
    to_item.inscription[0] = 0;
    to_item.flags = from.flags;
    to_item.category_id = from.category_id;
    to_item.sprite = from.sprite;
    to_item.misc_use = from.misc_use;
    to_item.cost = from.cost;
    to_item.sub_category_id = from.sub_category_id;
    to_item.items_count = from.items_count;
    to_item.weight = from.weight;
    to_item.to_hit = from.to_hit;
    to_item.to_damage = from.to_damage;
    to_item.ac = from.ac;
    to_item.to_ac = from.to_ac;
    to_item.damage = from.damage;
    to_item.depth_first_found = from.depth_first_found;
    to_item.identification = 0;
}

/// 38
pub fn inventory_destroy_item(item_id: i32) {
    with_state_mut(|state| {
        let item = &mut state.py.inventory[item_id as usize];

        if item.items_count > 1 && item.sub_category_id <= ITEM_SINGLE_STACK_MAX {
            item.items_count -= 1;
            state.py.pack.weight -= item.weight as i16;
        } else {
            state.py.pack.weight = (i32::from(state.py.pack.weight)
                - (u32::from(item.weight) * u32::from(item.items_count)) as i32)
                as i16;

            let unique = state.py.pack.unique_items as i32;
            for i in item_id..unique - 1 {
                state.py.inventory[i as usize] = state.py.inventory[i as usize + 1];
            }

            inventory_item_copy_to(
                OBJ_NOTHING as i16,
                &mut state.py.inventory[(unique - 1) as usize],
            );
            state.py.pack.unique_items -= 1;
        }

        state.py.flags.status |= PY_STR_WGT;
    });
}

/// 48
pub fn inventory_take_one_item(to_item: &mut Inventory, from_item: &Inventory) {
    *to_item = *from_item;
    if to_item.items_count > 1 && inventory_item_single_stackable(*to_item) {
        to_item.items_count = 1;
    }
}

/// 90
pub fn inventory_drop_item(item_id: i32, drop_all: bool) {
    let needs_delete = with_state(|state| {
        state.dg.floor[state.py.pos.y as usize][state.py.pos.x as usize].treasure_id != 0
    });
    if needs_delete {
        let _ = dungeon_delete_object(with_state(|state| state.py.pos));
    }

    let treasure_id = popt();

    with_state_mut(|state| {
        let item = state.py.inventory[item_id as usize];
        state.game.treasure.list[treasure_id as usize] = item;
        state.dg.floor[state.py.pos.y as usize][state.py.pos.x as usize].treasure_id =
            treasure_id as u8;
    });

    if item_id >= PlayerEquipment::Wield as i32 {
        player_take_off(item_id, -1);
        return;
    }

    let dropped = with_state_mut(|state| {
        let item = &mut state.py.inventory[item_id as usize];

        if drop_all || item.items_count == 1 {
            state.py.pack.weight = (i32::from(state.py.pack.weight)
                - (u32::from(item.weight) * u32::from(item.items_count)) as i32)
                as i16;
            state.py.pack.unique_items -= 1;

            let mut slot = item_id;
            while slot < state.py.pack.unique_items as i32 {
                state.py.inventory[slot as usize] = state.py.inventory[slot as usize + 1];
                slot += 1;
            }

            inventory_item_copy_to(
                OBJ_NOTHING as i16,
                &mut state.py.inventory[state.py.pack.unique_items as usize],
            );
        } else {
            state.game.treasure.list[treasure_id as usize].items_count = 1;
            state.py.pack.weight -= item.weight as i16;
            item.items_count -= 1;
        }

        state.py.flags.status |= PY_STR_WGT;
        state.game.treasure.list[treasure_id as usize]
    });

    // item_description borrows game state; must not run under with_state_mut.
    let mut prt1 = [0u8; MORIA_OBJ_DESC_SIZE as usize];
    item_description(&mut prt1, dropped, true);
    let desc_end = prt1.iter().position(|&b| b == 0).unwrap_or(prt1.len());
    let desc = String::from_utf8_lossy(&prt1[..desc_end]);
    print_message(Some(&format!("Dropped {desc}")));
}

/// 104
pub fn inventory_damage_item(item_type: fn(&Inventory) -> bool, chance_percentage: i32) -> i32 {
    let mut damage = 0;
    let mut i = 0;

    while i < with_state(|state| state.py.pack.unique_items) {
        let matches = with_state(|state| item_type(&state.py.inventory[i as usize]));
        if matches && random_number(100) < chance_percentage {
            inventory_destroy_item(i as i32);
            damage += 1;
        }
        i += 1;
    }

    damage
}

/// 126
pub fn inventory_diminish_light_attack(noticed: bool) -> bool {
    let mut noticed = noticed;
    let mut dim_message = None;
    with_state_mut(|state| {
        let light_idx = PlayerEquipment::Light as usize;
        if state.py.inventory[light_idx].misc_use > 0 {
            let roll = random_number_state(state, 250);
            let item = &mut state.py.inventory[light_idx];
            item.misc_use -= (250 + roll) as i16;

            if item.misc_use < 1 {
                item.misc_use = 1;
            }

            if state.py.flags.blind < 1 {
                dim_message = Some("Your light dims.".to_string());
            } else {
                noticed = false;
            }
        } else {
            noticed = false;
        }
    });
    if let Some(msg) = dim_message {
        print_message(Some(&msg));
    }
    noticed
}

/// 145
pub fn inventory_diminish_charges_attack(
    creature_level: u8,
    monster_hp: &mut i16,
    noticed: bool,
) -> bool {
    let mut noticed = noticed;
    let mut drain_message = None;
    with_state_mut(|state| {
        let item_index = random_number_state(state, i32::from(state.py.pack.unique_items)) - 1;
        let item = &mut state.py.inventory[item_index as usize];

        let has_charges = item.category_id == TV_STAFF || item.category_id == TV_WAND;

        if has_charges && item.misc_use > 0 {
            *monster_hp += i16::from(creature_level) * item.misc_use;
            item.misc_use = 0;
            if !spell_item_identified(*item) {
                item_append_to_inscription(item, ID_EMPTY);
            }
            drain_message = Some("Energy drains from your pack!".to_string());
        } else {
            noticed = false;
        }
    });
    if let Some(msg) = drain_message {
        print_message(Some(&msg));
    }
    noticed
}

/// 208
pub fn execute_disenchant_attack() -> bool {
    let item_id = match random_number(7) {
        1 => PlayerEquipment::Wield as i32,
        2 => PlayerEquipment::Body as i32,
        3 => PlayerEquipment::Arm as i32,
        4 => PlayerEquipment::Outer as i32,
        5 => PlayerEquipment::Hands as i32,
        6 => PlayerEquipment::Head as i32,
        7 => PlayerEquipment::Feet as i32,
        _ => return false,
    };

    let mut success = false;
    with_state_mut(|state| {
        let idx = item_id as usize;
        if state.py.inventory[idx].to_hit > 0 {
            let roll = random_number_state(state, 2) as i16;
            let item = &mut state.py.inventory[idx];
            item.to_hit -= roll;
            if item.to_hit < 0 {
                item.to_hit = 0;
            }
            success = true;
        }
        if state.py.inventory[idx].to_damage > 0 {
            let roll = random_number_state(state, 2) as i16;
            let item = &mut state.py.inventory[idx];
            item.to_damage -= roll;
            if item.to_damage < 0 {
                item.to_damage = 0;
            }
            success = true;
        }
        if state.py.inventory[idx].to_ac > 0 {
            let roll = random_number_state(state, 2) as i16;
            let item = &mut state.py.inventory[idx];
            item.to_ac -= roll;
            if item.to_ac < 0 {
                item.to_ac = 0;
            }
            success = true;
        }
    });
    success
}

/// 243
pub fn inventory_can_carry_item_count(item: Inventory) -> bool {
    with_state(|state| {
        if state.py.pack.unique_items < PlayerEquipment::Wield as i16 {
            return true;
        }

        if !inventory_item_stackable(item) {
            return false;
        }

        for i in 0..state.py.pack.unique_items {
            let inv = &state.py.inventory[i as usize];
            let same_character = inv.category_id == item.category_id;
            let same_category = inv.sub_category_id == item.sub_category_id;
            let same_number = u16::from(inv.items_count) + u16::from(item.items_count) < 256;
            let same_group = item.sub_category_id < ITEM_GROUP_MIN || inv.misc_use == item.misc_use;
            let inventory_item_is_colorless = item_set_colorless_as_identified_for_state(
                state,
                inv.category_id,
                inv.sub_category_id,
                inv.identification,
            );
            let item_is_colorless = item_set_colorless_as_identified_for_state(
                state,
                item.category_id,
                item.sub_category_id,
                item.identification,
            );
            let identification = inventory_item_is_colorless == item_is_colorless;

            if same_character && same_category && same_number && same_group && identification {
                return true;
            }
        }

        false
    })
}

/// 257
pub fn inventory_can_carry_item(item: Inventory) -> bool {
    with_state(|state| {
        let mut limit = {
            let mut weight_cap = i32::from(state.py.stats.used[PlayerAttr::A_STR as usize])
                * i32::from(crate::config::player::PLAYER_WEIGHT_CAP)
                + i32::from(state.py.misc.weight);
            if weight_cap > 3000 {
                weight_cap = 3000;
            }
            weight_cap
        };
        let new_weight =
            i32::from(item.items_count) * i32::from(item.weight) + i32::from(state.py.pack.weight);

        if limit < new_weight {
            limit = new_weight / (limit + 1);
        } else {
            limit = 0;
        }

        i32::from(state.py.pack.heaviness) == limit
    })
}

/// 303
pub fn inventory_carry_item(new_item: Inventory) -> i32 {
    with_state_mut(|state| {
        let is_known = item_set_colorless_as_identified_for_state(
            state,
            new_item.category_id,
            new_item.sub_category_id,
            new_item.identification,
        );
        let is_always_known =
            object_position_offset(new_item.category_id, new_item.sub_category_id) == -1;

        let mut slot_id = 0;

        for slot in 0..PLAYER_INVENTORY_SIZE as i32 {
            slot_id = slot;
            let (item_cat, item_sub, item_ident, item_misc) = {
                let item = &state.py.inventory[slot as usize];
                (
                    item.category_id,
                    item.sub_category_id,
                    item.identification,
                    item.misc_use,
                )
            };

            let is_same_category = new_item.category_id == item_cat;
            let is_same_sub_category = new_item.sub_category_id == item_sub;
            let not_too_many_items = {
                let item = &state.py.inventory[slot as usize];
                i32::from(new_item.items_count) + i32::from(item.items_count) < 256
            };
            let same_known_status =
                item_set_colorless_as_identified_for_state(state, item_cat, item_sub, item_ident)
                    == is_known;
            let is_stackable = inventory_item_stackable(new_item);
            let is_same_group =
                new_item.sub_category_id < ITEM_GROUP_MIN || item_misc == new_item.misc_use;

            if is_same_category
                && is_same_sub_category
                && is_stackable
                && not_too_many_items
                && is_same_group
                && same_known_status
            {
                state.py.inventory[slot as usize].items_count += new_item.items_count;
                break;
            }

            if (is_same_category && new_item.sub_category_id < item_sub && is_always_known)
                || new_item.category_id > item_cat
            {
                for i in (slot..state.py.pack.unique_items as i32).rev() {
                    state.py.inventory[(i + 1) as usize] = state.py.inventory[i as usize];
                }
                state.py.inventory[slot as usize] = new_item;
                state.py.pack.unique_items += 1;
                break;
            }
        }

        state.py.pack.weight = (i32::from(state.py.pack.weight)
            + i32::from(new_item.items_count) * i32::from(new_item.weight))
            as i16;
        state.py.flags.status |= PY_STR_WGT;

        slot_id
    })
}

/// 333
pub fn inventory_find_range(
    item_id_start: i32,
    item_id_end: i32,
    item_pos_start: &mut i32,
    item_pos_end: &mut i32,
) -> bool {
    *item_pos_start = -1;
    *item_pos_end = -1;

    with_state(|state| {
        let mut at_end_of_range = false;

        for i in 0..state.py.pack.unique_items {
            let item_id = i32::from(state.py.inventory[i as usize].category_id);

            if !at_end_of_range {
                if item_id == item_id_start || item_id == item_id_end {
                    at_end_of_range = true;
                    *item_pos_start = i as i32;
                }
            } else if item_id != item_id_start && item_id != item_id_end {
                *item_pos_end = i as i32 - 1;
                break;
            }
        }

        if at_end_of_range && *item_pos_end == -1 {
            *item_pos_end = i32::from(state.py.pack.unique_items) - 1;
        }

        at_end_of_range
    })
}

/// 362
pub fn inventory_item_single_stackable(item: Inventory) -> bool {
    item.sub_category_id >= ITEM_SINGLE_STACK_MIN && item.sub_category_id <= ITEM_SINGLE_STACK_MAX
}

/// 367
pub fn inventory_item_stackable(item: Inventory) -> bool {
    item.sub_category_id >= ITEM_SINGLE_STACK_MIN
}

/// 372
pub fn inventory_item_is_cursed(item: Inventory) -> bool {
    (item.flags & TR_CURSED) != 0
}

/// 376
pub fn inventory_item_remove_curse(item: &mut Inventory) {
    item.flags &= !TR_CURSED;
}

/// 450
pub fn set_null(_item: &Inventory) -> bool {
    false
}

fn set_corrodable_items(item: &Inventory) -> bool {
    matches!(
        item.category_id,
        TV_SWORD | TV_HELM | TV_SHIELD | TV_HARD_ARMOR | TV_WAND
    )
}

fn set_flammable_items(item: &Inventory) -> bool {
    match item.category_id {
        TV_ARROW | TV_BOW | TV_HAFTED | TV_POLEARM | TV_BOOTS | TV_GLOVES | TV_CLOAK
        | TV_SOFT_ARMOR => (item.flags & TR_RES_FIRE) == 0,
        TV_STAFF | TV_SCROLL1 | TV_SCROLL2 => true,
        _ => false,
    }
}

fn set_acid_affected_items(item: &Inventory) -> bool {
    match item.category_id {
        TV_MISC | TV_CHEST => true,
        TV_BOLT | TV_ARROW | TV_BOW | TV_HAFTED | TV_POLEARM | TV_BOOTS | TV_GLOVES | TV_CLOAK
        | TV_SOFT_ARMOR => (item.flags & TR_RES_ACID) == 0,
        _ => false,
    }
}

/// 508
pub fn set_frost_destroyable_items(item: &Inventory) -> bool {
    item.category_id == TV_POTION1 || item.category_id == TV_POTION2 || item.category_id == TV_FLASK
}

/// 512
pub fn set_lightning_destroyable_items(item: &Inventory) -> bool {
    item.category_id == TV_RING || item.category_id == TV_WAND || item.category_id == TV_SPIKE
}

/// 538
pub fn set_acid_destroyable_items(item: &Inventory) -> bool {
    match item.category_id {
        TV_ARROW | TV_BOW | TV_HAFTED | TV_POLEARM | TV_BOOTS | TV_GLOVES | TV_CLOAK | TV_HELM
        | TV_SHIELD | TV_HARD_ARMOR | TV_SOFT_ARMOR => (item.flags & TR_RES_ACID) == 0,
        TV_STAFF | TV_SCROLL1 | TV_SCROLL2 | TV_FOOD | TV_OPEN_DOOR | TV_CLOSED_DOOR => true,
        _ => false,
    }
}

/// 564
pub fn set_fire_destroyable_items(item: &Inventory) -> bool {
    match item.category_id {
        TV_ARROW | TV_BOW | TV_HAFTED | TV_POLEARM | TV_BOOTS | TV_GLOVES | TV_CLOAK
        | TV_SOFT_ARMOR => (item.flags & TR_RES_FIRE) == 0,
        TV_STAFF | TV_SCROLL1 | TV_SCROLL2 | TV_POTION1 | TV_POTION2 | TV_FLASK | TV_FOOD
        | TV_OPEN_DOOR | TV_CLOSED_DOOR => true,
        _ => false,
    }
}

/// 444
pub fn damage_minus_ac(typ_dam: u32) -> bool {
    let mut items = [0u8; 6];
    let mut items_count = 0usize;

    with_state(|state| {
        if state.py.inventory[PlayerEquipment::Body as usize].category_id != TV_NOTHING {
            items[items_count] = PlayerEquipment::Body as u8;
            items_count += 1;
        }
        if state.py.inventory[PlayerEquipment::Arm as usize].category_id != TV_NOTHING {
            items[items_count] = PlayerEquipment::Arm as u8;
            items_count += 1;
        }
        if state.py.inventory[PlayerEquipment::Outer as usize].category_id != TV_NOTHING {
            items[items_count] = PlayerEquipment::Outer as u8;
            items_count += 1;
        }
        if state.py.inventory[PlayerEquipment::Hands as usize].category_id != TV_NOTHING {
            items[items_count] = PlayerEquipment::Hands as u8;
            items_count += 1;
        }
        if state.py.inventory[PlayerEquipment::Head as usize].category_id != TV_NOTHING {
            items[items_count] = PlayerEquipment::Head as u8;
            items_count += 1;
        }
        if state.py.inventory[PlayerEquipment::Feet as usize].category_id != TV_NOTHING {
            items[items_count] = PlayerEquipment::Feet as u8;
            items_count += 1;
        }
    });

    if items_count == 0 {
        return false;
    }

    let item_id = items[(random_number(items_count as i32) - 1) as usize];

    // Snapshot item for description outside the mutable borrow — item_description
    // itself calls with_state and must not nest under with_state_mut.
    let item_snapshot = with_state(|state| state.py.inventory[item_id as usize]);
    let resists = (item_snapshot.flags & typ_dam) != 0;
    let can_damage = item_snapshot.ac + item_snapshot.to_ac > 0;

    if !resists && !can_damage {
        return false;
    }

    let mut description = [0u8; MORIA_OBJ_DESC_SIZE as usize];
    item_description(&mut description, item_snapshot, false);
    let desc_end = description
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(description.len());
    let desc = String::from_utf8_lossy(&description[..desc_end]);

    if resists {
        print_message(Some(&format!("Your {desc} resists damage!")));
        return true;
    }

    with_state_mut(|state| {
        state.py.inventory[item_id as usize].to_ac -= 1;
    });
    print_message(Some(&format!("Your {desc} is damaged!")));
    player_recalculate_bonuses();
    true
}

/// 575
pub fn damage_corroding_gas(creature_name: &Vtype_t) {
    if !damage_minus_ac(TR_RES_ACID) {
        player_takes_hit(random_number(8), creature_name);
    }

    if inventory_damage_item(set_corrodable_items, 5) > 0 {
        print_message(Some("There is an acrid smell coming from your pack."));
    }
}

/// 582
pub fn damage_poisoned_gas(damage: i32, creature_name: &Vtype_t) {
    player_takes_hit(damage, creature_name);

    with_state_mut(|state| {
        let roll = random_number_state(state, damage);
        state.py.flags.poisoned += (12 + roll) as i16;
    });
}

/// 599
pub fn damage_fire(damage: i32, creature_name: &Vtype_t) {
    let mut damage = damage;
    with_state(|state| {
        if state.py.flags.resistant_to_fire {
            damage /= 3;
        }
        if state.py.flags.heat_resistance > 0 {
            damage /= 3;
        }
    });

    player_takes_hit(damage, creature_name);

    if inventory_damage_item(set_flammable_items, 3) > 0 {
        print_message(Some("There is smoke coming from your pack."));
    }
}

/// 616
pub fn damage_cold(damage: i32, creature_name: &Vtype_t) {
    let mut damage = damage;
    with_state(|state| {
        if state.py.flags.resistant_to_cold {
            damage /= 3;
        }
        if state.py.flags.cold_resistance > 0 {
            damage /= 3;
        }
    });

    player_takes_hit(damage, creature_name);

    if inventory_damage_item(set_frost_destroyable_items, 5) > 0 {
        print_message(Some("Something shatters inside your pack!"));
    }
}

/// 629
pub fn damage_lightning_bolt(damage: i32, creature_name: &Vtype_t) {
    let mut damage = damage;
    with_state(|state| {
        if state.py.flags.resistant_to_light {
            damage /= 3;
        }
    });

    player_takes_hit(damage, creature_name);

    if inventory_damage_item(set_lightning_destroyable_items, 3) > 0 {
        print_message(Some("There are sparks coming from your pack."));
    }
}

/// 648
pub fn damage_acid(damage: i32, creature_name: &Vtype_t) {
    let mut flag = 0;

    if damage_minus_ac(TR_RES_ACID) {
        flag = 1;
    }

    with_state(|state| {
        if state.py.flags.resistant_to_acid {
            flag += 2;
        }
    });

    player_takes_hit(damage / (flag + 1), creature_name);

    if inventory_damage_item(set_acid_affected_items, 3) > 0 {
        print_message(Some("There is an acrid smell coming from your pack."));
    }
}
