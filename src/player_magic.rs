//! Port of src/player_magic.cpp — player magic functions.

use crate::config::monsters::defense::{
    CD_ANIMAL, CD_DRAGON, CD_EVIL, CD_FIRE, CD_FROST, CD_UNDEAD,
};
use crate::config::treasure::flags::{
    TR_EGO_WEAPON, TR_FLAME_TONGUE, TR_FROST_BRAND, TR_SLAY_ANIMAL, TR_SLAY_DRAGON,
    TR_SLAY_EVIL, TR_SLAY_UNDEAD,
};
use crate::data_creatures::CREATURES_LIST;
use crate::game::{random_number_state, with_state, with_state_mut};
use crate::inventory::Inventory;
use crate::treasure::{TV_ARROW, TV_FLASK, TV_HAFTED, TV_SLING_AMMO, TV_SWORD};

/// C++ player_magic.cpp lines 11–17.
pub fn player_cure_confusion() -> bool {
    if with_state(|state| state.py.flags.confused) > 1 {
        with_state_mut(|state| state.py.flags.confused = 1);
        return true;
    }
    false
}

/// C++ player_magic.cpp lines 20–26.
pub fn player_cure_blindness() -> bool {
    if with_state(|state| state.py.flags.blind) > 1 {
        with_state_mut(|state| state.py.flags.blind = 1);
        return true;
    }
    false
}

/// C++ player_magic.cpp lines 29–35.
pub fn player_cure_poison() -> bool {
    if with_state(|state| state.py.flags.poisoned) > 1 {
        with_state_mut(|state| state.py.flags.poisoned = 1);
        return true;
    }
    false
}

/// C++ player_magic.cpp lines 38–44.
pub fn player_remove_fear() -> bool {
    if with_state(|state| state.py.flags.afraid) > 1 {
        with_state_mut(|state| state.py.flags.afraid = 1);
        return true;
    }
    false
}

/// C++ player_magic.cpp lines 47–53.
#[must_use]
pub fn player_protect_evil() -> bool {
    with_state_mut(|state| {
        let is_protected = state.py.flags.protect_evil == 0;
        state.py.flags.protect_evil += (random_number_state(state, 25)
            + 3 * i32::from(state.py.misc.level)) as i16;
        is_protected
    })
}

/// C++ player_magic.cpp lines 56–58.
pub fn player_bless(adjustment: i32) {
    with_state_mut(|state| {
        state.py.flags.blessed += adjustment as i16;
    });
}

/// C++ player_magic.cpp lines 61–63.
pub fn player_detect_invisible(adjustment: i32) {
    with_state_mut(|state| {
        state.py.flags.detect_invisible += adjustment as i16;
    });
}

/// C++ player_magic.cpp lines 66–114.
pub fn item_magic_ability_damage(item: Inventory, total_damage: i32, monster_id: i32) -> i32 {
    let is_ego_weapon = (item.flags & TR_EGO_WEAPON) != 0;
    let is_projectile = item.category_id >= TV_SLING_AMMO && item.category_id <= TV_ARROW;
    let is_hafted_sword = item.category_id >= TV_HAFTED && item.category_id <= TV_SWORD;
    let is_flask = item.category_id == TV_FLASK;

    if !is_ego_weapon || !(is_projectile || is_hafted_sword || is_flask) {
        return total_damage;
    }

    with_state_mut(|state| {
        let creature = &CREATURES_LIST[monster_id as usize];
        let memory = &mut state.creature_recall[monster_id as usize];

        if (creature.defenses & CD_DRAGON) != 0 && (item.flags & TR_SLAY_DRAGON) != 0 {
            memory.defenses |= CD_DRAGON;
            return total_damage * 4;
        }
        if (creature.defenses & CD_UNDEAD) != 0 && (item.flags & TR_SLAY_UNDEAD) != 0 {
            memory.defenses |= CD_UNDEAD;
            return total_damage * 3;
        }
        if (creature.defenses & CD_ANIMAL) != 0 && (item.flags & TR_SLAY_ANIMAL) != 0 {
            memory.defenses |= CD_ANIMAL;
            return total_damage * 2;
        }
        if (creature.defenses & CD_EVIL) != 0 && (item.flags & TR_SLAY_EVIL) != 0 {
            memory.defenses |= CD_EVIL;
            return total_damage * 2;
        }
        if (creature.defenses & CD_FROST) != 0 && (item.flags & TR_FROST_BRAND) != 0 {
            memory.defenses |= CD_FROST;
            return total_damage * 3 / 2;
        }
        if (creature.defenses & CD_FIRE) != 0 && (item.flags & TR_FLAME_TONGUE) != 0 {
            memory.defenses |= CD_FIRE;
            return total_damage * 3 / 2;
        }

        total_damage
    })
}
