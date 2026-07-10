//! Port of `src/player_traps.cpp` — see `phase_4.4.10`.

use crate::config::treasure::chests::{
    CH_EXPLODE, CH_LOCKED, CH_LOSE_STR, CH_PARALYSED, CH_POISON, CH_SUMMON, CH_TRAPPED,
};
use crate::data_player::CLASS_LEVEL_ADJ;
use crate::dice::{dice_roll, Dice};
use crate::dungeon::dungeon_delete_object;
use crate::game::{random_number, with_state, with_state_mut};
use crate::identification::{
    object_blocked_by_monster, spell_item_identified,
    spell_item_identify_and_remove_random_inscription_for_state, SpecialNameIds,
};
use crate::monster_manager::monster_summon;
use crate::player::{player_no_light, player_takes_hit, PlayerAttr, PlayerClassLevelAdj};
use crate::player_move::{player_move, player_move_position};
use crate::player_stats::{
    player_disarm_adjustment, player_stat_adjustment_wisdom_intelligence,
    player_stat_random_decrease,
};
use crate::treasure::{TV_CHEST, TV_VIS_TRAP};
use crate::types::{Coord_t, Vtype_t, MORIA_MESSAGE_SIZE};
use crate::ui::display_character_experience;
use crate::ui_io::{get_direction_with_memory, terminal};

fn trap_hit_label(text: &str) -> Vtype_t {
    let mut buf = [0u8; MORIA_MESSAGE_SIZE];
    let bytes = text.as_bytes();
    let len = bytes.len().min(MORIA_MESSAGE_SIZE - 1);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf
}

/// C++ `player_traps.cpp` lines 10–30.
fn player_trap_disarm_ability() -> i32 {
    let (mut ability, blind, confused, image, class_id, level, int_stat) = with_state(|state| {
        (
            i32::from(state.py.misc.disarm),
            state.py.flags.blind,
            state.py.flags.confused,
            state.py.flags.image,
            state.py.misc.class_id,
            state.py.misc.level,
            state.py.stats.used[PlayerAttr::A_INT as usize],
        )
    });

    ability += 2;
    ability *= player_disarm_adjustment();
    ability += player_stat_adjustment_wisdom_intelligence(PlayerAttr::A_INT);
    ability += i32::from(CLASS_LEVEL_ADJ[class_id as usize][PlayerClassLevelAdj::DISARM as usize])
        * i32::from(level)
        / 3;

    if blind > 0 || player_no_light() {
        ability /= 10;
    }

    if confused > 0 {
        ability /= 10;
    }

    if image > 0 {
        ability /= 10;
    }

    let _ = int_stat;
    ability
}

/// C++ player_traps.cpp lines 32–61.
#[doc(hidden)]
pub fn player_disarm_floor_trap(coord: Coord_t, total: i32, level: i32, dir: i32, misc_use: i16) {
    let confused = with_state(|state| state.py.flags.confused);

    if total + 100 - level > random_number(100) {
        terminal::print_message(Some("You have disarmed the trap."));
        with_state_mut(|state| {
            state.py.misc.exp += i32::from(misc_use);
        });
        let _ = dungeon_delete_object(coord);

        with_state_mut(|state| {
            state.py.flags.confused = 0;
        });
        player_move(dir, false);
        with_state_mut(|state| {
            state.py.flags.confused = confused;
        });

        display_character_experience();
        return;
    }

    if total > 5 && random_number(total) > 5 {
        terminal::print_message_no_command_interrupt("You failed to disarm the trap.");
        return;
    }

    terminal::print_message(Some("You set the trap off!"));

    with_state_mut(|state| {
        state.py.flags.confused = 0;
    });
    player_move(dir, false);
    with_state_mut(|state| {
        state.py.flags.confused += confused;
    });
}

/// C++ player_traps.cpp lines 63–101.
#[doc(hidden)]
pub fn player_disarm_chest_trap(coord: Coord_t, total: i32, treasure_id: u8) {
    let identified =
        with_state(|state| spell_item_identified(state.game.treasure.list[treasure_id as usize]));
    if !identified {
        with_state_mut(|state| {
            state.game.player_free_turn = true;
        });
        terminal::print_message(Some("I don't see a trap."));
        return;
    }

    let trapped = with_state(|state| {
        (state.game.treasure.list[treasure_id as usize].flags & CH_TRAPPED) != 0
    });
    if trapped {
        let level = with_state(|state| {
            i32::from(state.game.treasure.list[treasure_id as usize].depth_first_found)
        });

        if total - level > random_number(100) {
            with_state_mut(|state| {
                let item = &mut state.game.treasure.list[treasure_id as usize];
                item.flags &= !CH_TRAPPED;
                if (item.flags & CH_LOCKED) != 0 {
                    item.special_name_id = SpecialNameIds::SN_LOCKED as u8;
                } else {
                    item.special_name_id = SpecialNameIds::SN_DISARMED as u8;
                }
            });

            terminal::print_message(Some("You have disarmed the chest."));

            with_state_mut(|state| {
                spell_item_identify_and_remove_random_inscription_for_state(
                    state,
                    treasure_id as usize,
                );
                state.py.misc.exp += level;
            });

            display_character_experience();
        } else if total > 5 && random_number(total) > 5 {
            terminal::print_message_no_command_interrupt("You failed to disarm the chest.");
        } else {
            terminal::print_message(Some("You set a trap off!"));
            with_state_mut(|state| {
                spell_item_identify_and_remove_random_inscription_for_state(
                    state,
                    treasure_id as usize,
                );
            });
            chest_trap(coord);
        }
        return;
    }

    terminal::print_message(Some("The chest was not trapped."));
    with_state_mut(|state| {
        state.game.player_free_turn = true;
    });
}

/// C++ `player_traps.cpp` lines 104–140.
pub fn player_disarm_trap() {
    let mut dir = 0i32;
    if !get_direction_with_memory(None, &mut dir) {
        return;
    }

    let mut coord = with_state(|state| state.py.pos);
    let _ = player_move_position(dir, &mut coord);

    let (creature_id, treasure_id, category_id) = with_state(|state| {
        let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
        let category_id = if tile.treasure_id != 0 {
            state.game.treasure.list[tile.treasure_id as usize].category_id
        } else {
            0
        };
        (tile.creature_id, tile.treasure_id, category_id)
    });

    let mut no_disarm = false;

    if creature_id > 1
        && treasure_id != 0
        && (category_id == TV_VIS_TRAP || category_id == TV_CHEST)
    {
        object_blocked_by_monster(i32::from(creature_id));
    } else if treasure_id != 0 {
        let disarm_ability = player_trap_disarm_ability();

        if category_id == TV_VIS_TRAP {
            let (level, misc_use) = with_state(|state| {
                let item = &state.game.treasure.list[treasure_id as usize];
                (i32::from(item.depth_first_found), item.misc_use)
            });
            player_disarm_floor_trap(coord, disarm_ability, level, dir, misc_use);
        } else if category_id == TV_CHEST {
            player_disarm_chest_trap(coord, disarm_ability, treasure_id);
        } else {
            no_disarm = true;
        }
    } else {
        no_disarm = true;
    }

    if no_disarm {
        terminal::print_message(Some("I do not see anything to disarm there."));
        with_state_mut(|state| {
            state.game.player_free_turn = true;
        });
    }
}

/// C++ `player_traps.cpp` lines 142–155.
fn chest_loose_strength() {
    terminal::print_message(Some("A small needle has pricked you!"));

    let sustain = with_state(|state| state.py.flags.sustain_str);
    if sustain {
        terminal::print_message(Some("You are unaffected."));
        return;
    }

    let _ = player_stat_random_decrease(PlayerAttr::A_STR);

    player_takes_hit(
        dice_roll(Dice { dice: 1, sides: 4 }),
        &trap_hit_label("a poison needle"),
    );

    terminal::print_message(Some("You feel weakened!"));
}

/// C++ `player_traps.cpp` lines 157–163.
fn chest_poison() {
    terminal::print_message(Some("A small needle has pricked you!"));

    player_takes_hit(
        dice_roll(Dice { dice: 1, sides: 6 }),
        &trap_hit_label("a poison needle"),
    );

    let poison = 10 + random_number(20);
    with_state_mut(|state| {
        state.py.flags.poisoned += poison as i16;
    });
}

/// C++ `player_traps.cpp` lines 165–175.
fn chest_paralysed() {
    terminal::print_message(Some("A puff of yellow gas surrounds you!"));

    let free_action = with_state(|state| state.py.flags.free_action);
    if free_action {
        terminal::print_message(Some("You are unaffected."));
        return;
    }

    terminal::print_message(Some("You choke and pass out."));
    let paralysis = (10 + random_number(20)) as i16;
    with_state_mut(|state| {
        state.py.flags.paralysis = paralysis;
    });
}

/// C++ `player_traps.cpp` lines 177–185.
fn chest_summon_monster(coord: Coord_t) {
    let mut position = Coord_t { y: 0, x: 0 };

    for _ in 0..3 {
        position.y = coord.y;
        position.x = coord.x;
        let _ = monster_summon(&mut position, false);
    }
}

/// C++ `player_traps.cpp` lines 187–193.
fn chest_explode(coord: Coord_t) {
    terminal::print_message(Some("There is a sudden explosion!"));

    let _ = dungeon_delete_object(coord);

    player_takes_hit(
        dice_roll(Dice { dice: 5, sides: 8 }),
        &trap_hit_label("an exploding chest"),
    );
}

/// C++ `player_traps.cpp` lines 197–219.
pub fn chest_trap(coord: Coord_t) {
    let flags = with_state(|state| {
        let treasure_id = state.dg.floor[coord.y as usize][coord.x as usize].treasure_id;
        state.game.treasure.list[treasure_id as usize].flags
    });

    if (flags & CH_LOSE_STR) != 0 {
        chest_loose_strength();
    }

    if (flags & CH_POISON) != 0 {
        chest_poison();
    }

    if (flags & CH_PARALYSED) != 0 {
        chest_paralysed();
    }

    if (flags & CH_SUMMON) != 0 {
        chest_summon_monster(coord);
    }

    if (flags & CH_EXPLODE) != 0 {
        chest_explode(coord);
    }
}
