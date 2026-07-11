//! Player bashing of doors, chests, and creatures.

use crate::config::dungeon::objects::{OBJ_OPEN_DOOR, OBJ_RUINED_CHEST};
use crate::config::monsters::defense::CD_MAX_HP;
use crate::config::treasure::chests::CH_LOCKED;
use crate::data_creatures::CREATURES_LIST;
use crate::data_player::CLASS_LEVEL_ADJ;
use crate::dice::{dice_roll, max_dice_roll};
use crate::dungeon::dungeon_lite_spot;
use crate::dungeon_tile::{MIN_CAVE_WALL, TILE_CORR_FLOOR};
use crate::game::{get_random_direction, random_number, with_state, with_state_mut};
use crate::inventory::{inventory_item_copy_to, PlayerEquipment};
use crate::monster::monster_take_hit;
use crate::player::{
    player_test_being_hit, player_weapon_critical_blow, PlayerAttr, PlayerClassLevelAdj,
    BTH_PER_PLUS_TO_HIT_ADJUST,
};
use crate::player_move::{player_move, player_move_position};
use crate::treasure::{TV_CHEST, TV_CLOSED_DOOR};
use crate::types::Coord_t;
use crate::ui::display_character_experience;
use crate::ui_io::{get_direction_with_memory, terminal};

/// 78
pub fn player_bash() {
    let mut dir = 0i32;
    if !get_direction_with_memory(None, &mut dir) {
        return;
    }

    if with_state(|state| state.py.flags.confused > 0) {
        terminal::print_message(Some("You are confused."));
        dir = get_random_direction();
    }

    let mut coord = with_state(|state| state.py.pos);
    let _ = player_move_position(dir, &mut coord);

    let (creature_id, treasure_id, category_id, feature_id) = with_state(|state| {
        let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
        let category_id = if tile.treasure_id != 0 {
            state.game.treasure.list[tile.treasure_id as usize].category_id
        } else {
            0
        };
        (
            tile.creature_id,
            tile.treasure_id,
            category_id,
            tile.feature_id,
        )
    });

    if creature_id > 1 {
        player_bash_position(coord);
        return;
    }

    if treasure_id != 0 {
        if category_id == TV_CLOSED_DOOR {
            player_bash_closed_door(coord, dir);
        } else if category_id == TV_CHEST {
            player_bash_closed_chest(treasure_id);
        } else {
            terminal::print_message(Some("You bash it, but nothing interesting happens."));
        }
        return;
    }

    if i32::from(feature_id) < i32::from(MIN_CAVE_WALL) {
        terminal::print_message(Some("You bash at empty space."));
        return;
    }

    terminal::print_message(Some("You bash it, but nothing interesting happens."));
}

/// 161
#[doc(hidden)]
pub fn player_bash_attack(coord: Coord_t) {
    let monster_id =
        with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].creature_id as i32);

    with_state_mut(|state| {
        state.monsters[monster_id as usize].sleep_count = 0;
    });

    let (lit, monster_creature_id) = with_state(|state| {
        let monster = &state.monsters[monster_id as usize];
        (monster.lit, monster.creature_id)
    });

    let name = if lit {
        format!("the {}", CREATURES_LIST[monster_creature_id as usize].name)
    } else {
        "it".to_string()
    };

    let (str_stat, arm_weight, misc_weight, dex, level, class_id) = with_state(|state| {
        (
            i32::from(state.py.stats.used[PlayerAttr::A_STR as usize]),
            i32::from(state.py.inventory[PlayerEquipment::Arm as usize].weight),
            i32::from(state.py.misc.weight),
            i32::from(state.py.stats.used[PlayerAttr::A_DEX as usize]),
            i32::from(state.py.misc.level),
            state.py.misc.class_id,
        )
    });

    let mut base_to_hit = str_stat + arm_weight / 2 + misc_weight / 10;

    if !lit {
        base_to_hit /= 2;
        base_to_hit -= dex * (i32::from(BTH_PER_PLUS_TO_HIT_ADJUST) - 1);
        base_to_hit -= level
            * i32::from(CLASS_LEVEL_ADJ[class_id as usize][PlayerClassLevelAdj::BTH as usize])
            / 2;
    }

    let creature_ac = i32::from(CREATURES_LIST[monster_creature_id as usize].ac);

    if player_test_being_hit(
        base_to_hit,
        level,
        dex,
        creature_ac,
        PlayerClassLevelAdj::BTH as u8,
    ) {
        terminal::print_message(Some(&format!("You hit {name}.")));

        let arm_damage =
            with_state(|state| state.py.inventory[PlayerEquipment::Arm as usize].damage);
        let mut damage = dice_roll(arm_damage);
        damage = player_weapon_critical_blow(
            arm_weight / 4 + str_stat,
            0,
            damage,
            PlayerClassLevelAdj::BTH as u8,
        );
        damage += misc_weight / 60;
        damage += 3;

        if damage < 0 {
            damage = 0;
        }

        if monster_take_hit(monster_id, damage) >= 0 {
            terminal::print_message(Some(&format!("You have slain {name}.")));
            display_character_experience();
        } else {
            let mut display_name = name.clone();
            if let Some(first) = display_name.get_mut(0..1) {
                first.make_ascii_uppercase();
            }

            let creature = &CREATURES_LIST[monster_creature_id as usize];
            let avg_max_hp = if (creature.defenses & CD_MAX_HP) != 0 {
                max_dice_roll(creature.hit_die)
            } else {
                (i32::from(creature.hit_die.dice) * i32::from(creature.hit_die.sides + 1)) >> 1
            };

            let monster_hp = with_state(|state| i32::from(state.monsters[monster_id as usize].hp));

            if 100 + random_number(400) + random_number(400) > monster_hp + avg_max_hp {
                let stun_amount = (random_number(3) + 1) as u8;
                with_state_mut(|state| {
                    let monster = &mut state.monsters[monster_id as usize];
                    monster.stunned_amount += stun_amount;
                    if monster.stunned_amount > 24 {
                        monster.stunned_amount = 24;
                    }
                });
                terminal::print_message(Some(&format!("{display_name} appears stunned!")));
            } else {
                terminal::print_message(Some(&format!("{display_name} ignores your bash!")));
            }
        }
    } else {
        terminal::print_message(Some(&format!("You miss {name}.")));
    }

    if random_number(150) > dex {
        terminal::print_message(Some("You are off balance."));
        let paralysis = (1 + random_number(2)) as i16;
        with_state_mut(|state| {
            state.py.flags.paralysis = paralysis;
        });
    }
}

fn player_bash_position(coord: Coord_t) {
    if with_state(|state| state.py.flags.afraid > 0) {
        terminal::print_message(Some("You are afraid!"));
        return;
    }

    player_bash_attack(coord);
}

/// 208
#[doc(hidden)]
pub fn player_bash_closed_door(coord: Coord_t, dir: i32) {
    terminal::print_message_no_command_interrupt("You smash into the door!");

    let (chance, dex, confused) = with_state(|state| {
        (
            i32::from(state.py.stats.used[PlayerAttr::A_STR as usize])
                + i32::from(state.py.misc.weight) / 2,
            i32::from(state.py.stats.used[PlayerAttr::A_DEX as usize]),
            state.py.flags.confused,
        )
    });

    let treasure_id =
        with_state(|state| state.dg.floor[coord.y as usize][coord.x as usize].treasure_id);

    let abs_misc_use = with_state(|state| {
        state.game.treasure.list[treasure_id as usize]
            .misc_use
            .unsigned_abs() as i32
    });

    if random_number(chance * (20 + abs_misc_use)) < 10 * (chance - abs_misc_use) {
        terminal::print_message(Some("The door crashes open!"));

        let misc_use = (1 - random_number(2)) as i16;
        with_state_mut(|state| {
            inventory_item_copy_to(
                OBJ_OPEN_DOOR as i16,
                &mut state.game.treasure.list[treasure_id as usize],
            );
            state.game.treasure.list[treasure_id as usize].misc_use = misc_use;
            state.dg.floor[coord.y as usize][coord.x as usize].feature_id = TILE_CORR_FLOOR;
        });

        if confused == 0 {
            player_move(dir, false);
        } else {
            dungeon_lite_spot(coord);
        }
        return;
    }

    if random_number(150) > dex {
        terminal::print_message(Some("You are off-balance."));
        let paralysis = (1 + random_number(2)) as i16;
        with_state_mut(|state| {
            state.py.flags.paralysis = paralysis;
        });
        return;
    }

    if with_state(|state| state.game.command_count == 0) {
        terminal::print_message(Some("The door holds firm."));
    }
}

/// 230
#[doc(hidden)]
pub fn player_bash_closed_chest(treasure_id: u8) {
    if random_number(10) == 1 {
        terminal::print_message(Some("You have destroyed the chest."));
        terminal::print_message(Some("and its contents!"));

        with_state_mut(|state| {
            let item = &mut state.game.treasure.list[treasure_id as usize];
            item.id = OBJ_RUINED_CHEST;
            item.flags = 0;
        });
        return;
    }

    let locked =
        with_state(|state| (state.game.treasure.list[treasure_id as usize].flags & CH_LOCKED) != 0);

    if locked && random_number(10) == 1 {
        terminal::print_message(Some("The lock breaks open!"));
        with_state_mut(|state| {
            state.game.treasure.list[treasure_id as usize].flags &= !CH_LOCKED;
        });
        return;
    }

    terminal::print_message_no_command_interrupt("The chest holds firm.");
}
