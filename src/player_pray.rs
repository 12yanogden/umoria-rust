//! Priest prayer driver

use crate::config::monsters::defense::{CD_EVIL, CD_UNDEAD};
use crate::config::spells::SPELL_TYPE_PRIEST;
use crate::data_player::{CLASSES, MAGIC_SPELLS};
use crate::dice::{dice_roll, Dice};
use crate::game::{random_number, with_state, with_state_mut};
use crate::inventory::{inventory_find_range, inventory_item_remove_curse};
use crate::monster::monster_sleep;
use crate::player::{player_no_light, player_teleport, PlayerAttr};
use crate::player_magic::{
    player_bless, player_cure_poison, player_detect_invisible, player_protect_evil,
    player_remove_fear,
};
use crate::player_stats::{player_stat_random_decrease, player_stat_restore};
use crate::spells::{
    cast_spell_get_id, spell_change_player_hit_points, spell_confuse_monster, spell_create_food,
    spell_detect_evil, spell_detect_secret_doors_within_vicinity,
    spell_detect_traps_within_vicinity, spell_dispel_creature, spell_earthquake, spell_fire_ball,
    spell_light_area, spell_map_current_area, spell_slow_poison, spell_turn_undead,
    spell_warding_glyph, MagicSpellFlags,
};
use crate::treasure::{TV_MAX_WEAR, TV_MIN_WEAR, TV_NEVER, TV_PRAYER_BOOK};
use crate::ui::{display_character_experience, print_character_current_mana};
use crate::ui_inventory::inventory_get_input_for_item_id;
use crate::ui_io::get_direction_with_memory;
use crate::ui_io::terminal;

/// 77
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
enum PriestSpellId {
    DetectEvil = 1,
    CureLightWounds,
    Bless,
    RemoveFear,
    CallLight,
    FindTraps,
    DetectDoorsStairs,
    SlowPoison,
    BlindCreature,
    Portal,
    CureMediumWounds,
    Chant,
    Sanctuary,
    CreateFood,
    RemoveCurse,
    ResistHeadCold,
    NeutralizePoison,
    OrbOfDraining,
    CureSeriousWounds,
    SenseInvisible,
    ProtectFromEvil,
    Earthquake,
    SenseSurroundings,
    CureCriticalWounds,
    TurnUndead,
    Prayer,
    DispelUndead,
    Heal,
    DispelEvil,
    GlyphOfWarding,
    HolyWord,
}

/// 42
#[must_use]
pub fn player_can_pray(item_pos_begin: &mut i32, item_pos_end: &mut i32) -> bool {
    let block_reason = with_state(|state| {
        if state.py.flags.blind > 0 {
            Some("You can't see to read your prayer!")
        } else if player_no_light() {
            Some("You have no light to read by.")
        } else if state.py.flags.confused > 0 {
            Some("You are too confused.")
        } else if CLASSES[state.py.misc.class_id as usize].class_to_use_mage_spells
            != SPELL_TYPE_PRIEST
        {
            Some("Pray hard enough and your prayers may be answered.")
        } else if state.py.pack.unique_items == 0 {
            Some("But you are not carrying anything!")
        } else if !inventory_find_range(
            i32::from(TV_PRAYER_BOOK),
            i32::from(TV_NEVER),
            item_pos_begin,
            item_pos_end,
        ) {
            Some("You are not carrying any Holy Books!")
        } else {
            None
        }
    });

    if let Some(msg) = block_reason {
        terminal::print_message(Some(msg));
        false
    } else {
        true
    }
}

/// 206
pub fn player_recite_prayer(prayer_type: i32) {
    let (pos, level) = with_state(|state| (state.py.pos, state.py.misc.level));
    let mut dir = 0;

    match prayer_type + 1 {
        id if id == PriestSpellId::DetectEvil as i32 => {
            let _ = spell_detect_evil();
        }
        id if id == PriestSpellId::CureLightWounds as i32 => {
            let _ = spell_change_player_hit_points(dice_roll(Dice { dice: 3, sides: 3 }));
        }
        id if id == PriestSpellId::Bless as i32 => {
            player_bless(random_number(12) + 12);
        }
        id if id == PriestSpellId::RemoveFear as i32 => {
            let _ = player_remove_fear();
        }
        id if id == PriestSpellId::CallLight as i32 => {
            let _ = spell_light_area(pos);
        }
        id if id == PriestSpellId::FindTraps as i32 => {
            let _ = spell_detect_traps_within_vicinity();
        }
        id if id == PriestSpellId::DetectDoorsStairs as i32 => {
            let _ = spell_detect_secret_doors_within_vicinity();
        }
        id if id == PriestSpellId::SlowPoison as i32 => {
            let _ = spell_slow_poison();
        }
        id if id == PriestSpellId::BlindCreature as i32
            && get_direction_with_memory(None, &mut dir) =>
        {
            let _ = spell_confuse_monster(pos, dir);
        }
        id if id == PriestSpellId::Portal as i32 => {
            player_teleport(i32::from(level) * 3);
        }
        id if id == PriestSpellId::CureMediumWounds as i32 => {
            let _ = spell_change_player_hit_points(dice_roll(Dice { dice: 4, sides: 4 }));
        }
        id if id == PriestSpellId::Chant as i32 => {
            player_bless(random_number(24) + 24);
        }
        id if id == PriestSpellId::Sanctuary as i32 => {
            let _ = monster_sleep(pos);
        }
        id if id == PriestSpellId::CreateFood as i32 => {
            spell_create_food();
        }
        id if id == PriestSpellId::RemoveCurse as i32 => {
            with_state_mut(|state| {
                for entry in &mut state.py.inventory {
                    if entry.category_id >= TV_MIN_WEAR && entry.category_id <= TV_MAX_WEAR {
                        inventory_item_remove_curse(entry);
                    }
                }
            });
        }
        id if id == PriestSpellId::ResistHeadCold as i32 => {
            let heat = random_number(10) + 10;
            let cold = random_number(10) + 10;
            with_state_mut(|state| {
                state.py.flags.heat_resistance += heat as i16;
                state.py.flags.cold_resistance += cold as i16;
            });
        }
        id if id == PriestSpellId::NeutralizePoison as i32 => {
            let _ = player_cure_poison();
        }
        id if id == PriestSpellId::OrbOfDraining as i32
            && get_direction_with_memory(None, &mut dir) =>
        {
            spell_fire_ball(
                pos,
                dir,
                dice_roll(Dice { dice: 3, sides: 6 }) + i32::from(level),
                MagicSpellFlags::HolyOrb,
                "Black Sphere",
            );
        }
        id if id == PriestSpellId::CureSeriousWounds as i32 => {
            let _ = spell_change_player_hit_points(dice_roll(Dice { dice: 8, sides: 4 }));
        }
        id if id == PriestSpellId::SenseInvisible as i32 => {
            player_detect_invisible(random_number(24) + 24);
        }
        id if id == PriestSpellId::ProtectFromEvil as i32 => {
            let _ = player_protect_evil();
        }
        id if id == PriestSpellId::Earthquake as i32 => {
            spell_earthquake();
        }
        id if id == PriestSpellId::SenseSurroundings as i32 => {
            spell_map_current_area();
        }
        id if id == PriestSpellId::CureCriticalWounds as i32 => {
            let _ = spell_change_player_hit_points(dice_roll(Dice { dice: 16, sides: 4 }));
        }
        id if id == PriestSpellId::TurnUndead as i32 => {
            let _ = spell_turn_undead();
        }
        id if id == PriestSpellId::Prayer as i32 => {
            player_bless(random_number(48) + 48);
        }
        id if id == PriestSpellId::DispelUndead as i32 => {
            let _ = spell_dispel_creature(i32::from(CD_UNDEAD), 3 * i32::from(level));
        }
        id if id == PriestSpellId::Heal as i32 => {
            let _ = spell_change_player_hit_points(200);
        }
        id if id == PriestSpellId::DispelEvil as i32 => {
            let _ = spell_dispel_creature(i32::from(CD_EVIL), 3 * i32::from(level));
        }
        id if id == PriestSpellId::GlyphOfWarding as i32 => {
            spell_warding_glyph();
        }
        id if id == PriestSpellId::HolyWord as i32 => {
            let _ = player_remove_fear();
            let _ = player_cure_poison();
            let _ = spell_change_player_hit_points(1000);

            for stat in [
                PlayerAttr::A_STR,
                PlayerAttr::A_INT,
                PlayerAttr::A_WIS,
                PlayerAttr::A_DEX,
                PlayerAttr::A_CON,
                PlayerAttr::A_CHR,
            ] {
                let _ = player_stat_restore(stat);
            }

            let _ = spell_dispel_creature(i32::from(CD_EVIL), 4 * i32::from(level));
            let _ = spell_turn_undead();

            with_state_mut(|state| {
                if state.py.flags.invulnerability < 3 {
                    state.py.flags.invulnerability = 3;
                } else {
                    state.py.flags.invulnerability += 1;
                }
            });
        }
        _ => {}
    }
}

/// 272
pub fn pray() {
    with_state_mut(|state| state.game.player_free_turn = true);

    let mut item_pos_begin = 0;
    let mut item_pos_end = 0;
    if !player_can_pray(&mut item_pos_begin, &mut item_pos_end) {
        return;
    }

    let mut item_id = 0;
    if !inventory_get_input_for_item_id(
        &mut item_id,
        "Use which Holy Book?",
        item_pos_begin,
        item_pos_end,
        None,
        None,
    ) {
        return;
    }

    let mut choice = 0;
    let mut chance = 0;
    let result = cast_spell_get_id("Recite which prayer?", item_id, &mut choice, &mut chance);
    if result < 0 {
        terminal::print_message(Some("You don't know any prayers in that book."));
        return;
    }
    if result == 0 {
        return;
    }

    with_state_mut(|state| state.game.player_free_turn = false);

    let (exp_gain_for_learning, mana_required) = with_state(|state| {
        let class_id = state.py.misc.class_id as usize;
        let spell = &MAGIC_SPELLS[class_id - 1][choice as usize];
        (
            i32::from(spell.exp_gain_for_learning),
            i32::from(spell.mana_required),
        )
    });

    if random_number(100) < chance {
        terminal::print_message(Some("You lost your concentration!"));
    } else {
        player_recite_prayer(choice);

        let should_award_exp = with_state(|state| {
            !state.game.player_free_turn && (state.py.flags.spells_worked & (1u32 << choice)) == 0
        });
        if should_award_exp {
            with_state_mut(|state| {
                state.py.misc.exp = state.py.misc.exp.wrapping_add(exp_gain_for_learning << 2);
                state.py.flags.spells_worked |= 1u32 << choice;
            });
            display_character_experience();
        }
    }

    if with_state(|state| state.game.player_free_turn) {
        return;
    }

    // read live current_mana after recite
    let current_mana = with_state(|state| i32::from(state.py.misc.current_mana));
    if mana_required > current_mana {
        terminal::print_message(Some("You faint from fatigue!"));

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
