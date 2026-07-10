//! Port of src/mage_spells.cpp — mage spellcasting driver.

use crate::config::spells::SPELL_TYPE_MAGE;
use crate::data_player::{CLASSES, MAGIC_SPELLS, SPELL_NAMES};
use crate::dice::{dice_roll, Dice};
use crate::game::{random_number, with_state, with_state_mut};
use crate::inventory::{inventory_find_range, inventory_item_remove_curse, PLAYER_INVENTORY_SIZE};
use crate::monster::monster_sleep;
use crate::player::{player_teleport, PlayerAttr};
use crate::player_magic::player_cure_poison;
use crate::player_stats::player_stat_random_decrease;
use crate::spells::{
    cast_spell_get_id, spell_change_player_hit_points,
    spell_confuse_monster, spell_create_food, spell_destroy_adjacent_doors_traps,
    spell_destroy_area, spell_detect_monsters, spell_detect_secret_doors_within_vicinity,
    spell_detect_traps_within_vicinity, spell_fire_ball, spell_fire_bolt, spell_genocide,
    spell_identify_item, spell_light_area, spell_polymorph_monster, spell_recharge_item,
    spell_sleep_all_monsters, spell_sleep_monster, spell_speed_monster,
    spell_teleport_away_monster_in_direction, spell_wall_to_mud, MagicSpellFlags,
};
use crate::treasure::{TV_MAGIC_BOOK, TV_NEVER};
use crate::ui::{display_character_experience, print_character_current_mana};
use crate::ui_inventory::inventory_get_input_for_item_id;
use crate::ui_io::get_direction_with_memory;
use crate::ui_io::terminal;

pub use crate::spells::spell_chance_of_success;

/// C++ mage_spells.cpp lines 11–43.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
enum MageSpellId {
    MagicMissile = 1,
    DetectMonsters,
    PhaseDoor,
    LightArea,
    CureLightWounds,
    FindHiddenTrapsDoors,
    StinkingCloud,
    Confusion,
    LightningBolt,
    TrapDoorDestruction,
    Sleep1,
    CurePoison,
    TeleportSelf,
    RemoveCurse,
    FrostBolt,
    WallToMud,
    CreateFood,
    RechargeItem1,
    Sleep2,
    PolymorphOther,
    IdentifyItem,
    Sleep3,
    FireBolt,
    SpeedMonster,
    FrostBall,
    RechargeItem2,
    TeleportOther,
    HasteSelf,
    FireBall,
    WordOfDestruction,
    Genocide,
}

/// C++ mage_spells.cpp lines 45–67.
#[must_use]
pub fn can_read_spells() -> bool {
    let block_reason = with_state(|state| {
        if state.py.flags.blind > 0 {
            Some("You can't see to read your spell book!")
        } else {
            let tile = &state.dg.floor[state.py.pos.y as usize][state.py.pos.x as usize];
            if !tile.temporary_light && !tile.permanent_light {
                Some("You have no light to read by.")
            } else if state.py.flags.confused > 0 {
                Some("You are too confused.")
            } else if CLASSES[state.py.misc.class_id as usize].class_to_use_mage_spells
                != SPELL_TYPE_MAGE
            {
                Some("You can't cast spells!")
            } else {
                None
            }
        }
    });

    if let Some(msg) = block_reason {
        terminal::print_message(Some(msg));
        false
    } else {
        true
    }
}

/// C++ mage_spells.cpp lines 69–199.
pub fn cast_spell(spell_id: i32) {
    let (pos, level) = with_state(|state| (state.py.pos, state.py.misc.level));
    let mut dir = 0;

    match spell_id {
            id if id == MageSpellId::MagicMissile as i32 => {
                if get_direction_with_memory(None, &mut dir) {
                    spell_fire_bolt(
                        pos,
                        dir,
                        dice_roll(Dice { dice: 2, sides: 6 }),
                        MagicSpellFlags::MagicMissile,
                        SPELL_NAMES[0],
                    );
                }
            }
            id if id == MageSpellId::DetectMonsters as i32 => {
                let _ = spell_detect_monsters();
            }
            id if id == MageSpellId::PhaseDoor as i32 => {
                player_teleport(10);
            }
            id if id == MageSpellId::LightArea as i32 => {
                let _ = spell_light_area(pos);
            }
            id if id == MageSpellId::CureLightWounds as i32 => {
                let _ = spell_change_player_hit_points(dice_roll(Dice { dice: 4, sides: 4 }));
            }
            id if id == MageSpellId::FindHiddenTrapsDoors as i32 => {
                let _ = spell_detect_secret_doors_within_vicinity();
                let _ = spell_detect_traps_within_vicinity();
            }
            id if id == MageSpellId::StinkingCloud as i32 => {
                if get_direction_with_memory(None, &mut dir) {
                    spell_fire_ball(
                        pos,
                        dir,
                        12,
                        MagicSpellFlags::PoisonGas,
                        SPELL_NAMES[6],
                    );
                }
            }
            id if id == MageSpellId::Confusion as i32 => {
                if get_direction_with_memory(None, &mut dir) {
                    let _ = spell_confuse_monster(pos, dir);
                }
            }
            id if id == MageSpellId::LightningBolt as i32 => {
                if get_direction_with_memory(None, &mut dir) {
                    spell_fire_bolt(
                        pos,
                        dir,
                        dice_roll(Dice { dice: 4, sides: 8 }),
                        MagicSpellFlags::Lightning,
                        SPELL_NAMES[8],
                    );
                }
            }
            id if id == MageSpellId::TrapDoorDestruction as i32 => {
                let _ = spell_destroy_adjacent_doors_traps();
            }
            id if id == MageSpellId::Sleep1 as i32 => {
                if get_direction_with_memory(None, &mut dir) {
                    let _ = spell_sleep_monster(pos, dir);
                }
            }
            id if id == MageSpellId::CurePoison as i32 => {
                let _ = player_cure_poison();
            }
            id if id == MageSpellId::TeleportSelf as i32 => {
                player_teleport(i32::from(level) * 5);
            }
            id if id == MageSpellId::RemoveCurse as i32 => {
                with_state_mut(|state| {
                    for slot in 22..PLAYER_INVENTORY_SIZE as i32 {
                        inventory_item_remove_curse(&mut state.py.inventory[slot as usize]);
                    }
                });
            }
            id if id == MageSpellId::FrostBolt as i32 => {
                if get_direction_with_memory(None, &mut dir) {
                    spell_fire_bolt(
                        pos,
                        dir,
                        dice_roll(Dice { dice: 6, sides: 8 }),
                        MagicSpellFlags::Frost,
                        SPELL_NAMES[14],
                    );
                }
            }
            id if id == MageSpellId::WallToMud as i32 => {
                if get_direction_with_memory(None, &mut dir) {
                    let _ = spell_wall_to_mud(pos, dir);
                }
            }
            id if id == MageSpellId::CreateFood as i32 => {
                spell_create_food();
            }
            id if id == MageSpellId::RechargeItem1 as i32 => {
                let _ = spell_recharge_item(20);
            }
            id if id == MageSpellId::Sleep2 as i32 => {
                let _ = monster_sleep(pos);
            }
            id if id == MageSpellId::PolymorphOther as i32 => {
                if get_direction_with_memory(None, &mut dir) {
                    let _ = spell_polymorph_monster(pos, dir);
                }
            }
            id if id == MageSpellId::IdentifyItem as i32 => {
                let _ = spell_identify_item();
            }
            id if id == MageSpellId::Sleep3 as i32 => {
                let _ = spell_sleep_all_monsters();
            }
            id if id == MageSpellId::FireBolt as i32 => {
                if get_direction_with_memory(None, &mut dir) {
                    spell_fire_bolt(
                        pos,
                        dir,
                        dice_roll(Dice { dice: 9, sides: 8 }),
                        MagicSpellFlags::Fire,
                        SPELL_NAMES[22],
                    );
                }
            }
            id if id == MageSpellId::SpeedMonster as i32 => {
                if get_direction_with_memory(None, &mut dir) {
                    let _ = spell_speed_monster(pos, dir, -1);
                }
            }
            id if id == MageSpellId::FrostBall as i32 => {
                if get_direction_with_memory(None, &mut dir) {
                    spell_fire_ball(
                        pos,
                        dir,
                        48,
                        MagicSpellFlags::Frost,
                        SPELL_NAMES[24],
                    );
                }
            }
            id if id == MageSpellId::RechargeItem2 as i32 => {
                let _ = spell_recharge_item(60);
            }
            id if id == MageSpellId::TeleportOther as i32 => {
                if get_direction_with_memory(None, &mut dir) {
                    let _ = spell_teleport_away_monster_in_direction(pos, dir);
                }
            }
            id if id == MageSpellId::HasteSelf as i32 => {
                with_state_mut(|state| {
                    use crate::game::random_number_state;
                    state.py.flags.fast += (random_number_state(state, 20)
                        + i32::from(state.py.misc.level)) as i16;
                });
            }
            id if id == MageSpellId::FireBall as i32 => {
                if get_direction_with_memory(None, &mut dir) {
                    spell_fire_ball(
                        pos,
                        dir,
                        72,
                        MagicSpellFlags::Fire,
                        SPELL_NAMES[28],
                    );
                }
            }
            id if id == MageSpellId::WordOfDestruction as i32 => {
                spell_destroy_area(pos);
            }
            id if id == MageSpellId::Genocide as i32 => {
                let _ = spell_genocide();
            }
            _ => {}
    }
}

/// C++ mage_spells.cpp lines 202–267.
pub fn get_and_cast_magic_spell() {
    with_state_mut(|state| state.game.player_free_turn = true);

    if !can_read_spells() {
        return;
    }

    let mut item_pos_start = 0;
    let mut item_pos_end = 0;
    if !inventory_find_range(
        i32::from(TV_MAGIC_BOOK),
        i32::from(TV_NEVER),
        &mut item_pos_start,
        &mut item_pos_end,
    ) {
        terminal::print_message(Some("But you are not carrying any spell-books!"));
        return;
    }

    let mut item_val = 0;
    if !inventory_get_input_for_item_id(
        &mut item_val,
        "Use which spell-book?",
        item_pos_start,
        item_pos_end,
        None,
        None,
    ) {
        return;
    }

    let mut choice = 0;
    let mut chance = 0;
    let result = cast_spell_get_id("Cast which spell?", item_val, &mut choice, &mut chance);
    if result < 0 {
        terminal::print_message(Some("You don't know any spells in that book."));
        return;
    }
    if result == 0 {
        return;
    }

    with_state_mut(|state| state.game.player_free_turn = false);

    let (exp_gain_for_learning, mana_required, current_mana) = with_state(|state| {
        let class_id = state.py.misc.class_id as usize;
        let spell = &MAGIC_SPELLS[class_id - 1][choice as usize];
        (
            i32::from(spell.exp_gain_for_learning),
            i32::from(spell.mana_required),
            i32::from(state.py.misc.current_mana),
        )
    });

    if random_number(100) < chance {
        terminal::print_message(Some("You failed to get the spell off!"));
    } else {
        cast_spell(choice + 1);

        let should_award_exp = with_state(|state| {
            !state.game.player_free_turn
                && (state.py.flags.spells_worked & (1u32 << choice)) == 0
        });
        if should_award_exp {
            with_state_mut(|state| {
                state.py.misc.exp = state
                    .py
                    .misc
                    .exp
                    .wrapping_add(exp_gain_for_learning << 2);
                state.py.flags.spells_worked |= 1u32 << choice;
            });
            display_character_experience();
        }
    }

    if with_state(|state| state.game.player_free_turn) {
        return;
    }

    if mana_required > current_mana {
        terminal::print_message(Some("You faint from the effort!"));

        let paralysis_roll = random_number(5 * (mana_required - current_mana));
        with_state_mut(|state| {
            state.py.flags.paralysis = paralysis_roll as i16;
            state.py.misc.current_mana = 0;
            state.py.misc.current_mana_fraction = 0;
        });

        if random_number(3) == 1 {
            terminal::print_message(Some("You have damaged your health!"));
            let _ = player_stat_random_decrease(PlayerAttr::A_CON);
        }
    } else {
        with_state_mut(|state| {
            state.py.misc.current_mana -= mana_required as i16;
        });
    }

    print_character_current_mana();
}
