//! Monster allocation and creature list management.

use crate::config::monsters::defense::{CD_MAX_HP, CD_UNDEAD};
use crate::config::monsters::move_flags::CM_WIN;
use crate::config::monsters::{
    self, MON_ENDGAME_MONSTERS, MON_MAX_SIGHT, MON_SUMMONED_LEVEL_ADJUST,
};
use crate::data_creatures::CREATURES_LIST;
use crate::dice::{max_dice_roll, Dice};
use crate::dungeon::{
    coord_distance_between, coord_in_bounds, dungeon_delete_monster,
    dungeon_remove_monster_from_level,
};
use crate::dungeon_tile::{MAX_OPEN_SPACE, MIN_CLOSED_SPACE};
use crate::game::{
    random_number, random_number_normal_distribution_state, random_number_state, with_state,
    with_state_mut, State,
};
use crate::monster::MON_MAX_LEVELS;
use crate::types::Coord_t;
use crate::ui_io::terminal;

/// 28
fn popm() -> i32 {
    let needs_compact = with_state(|state| {
        state.next_free_monster_id == i16::from(crate::monster::MON_TOTAL_ALLOCATIONS)
    });
    if needs_compact && !compact_monsters() {
        return -1;
    }
    with_state_mut(|state| {
        let id = i32::from(state.next_free_monster_id);
        state.next_free_monster_id += 1;
        id
    })
}

fn dice_roll_state(state: &mut State, dice: Dice) -> i32 {
    let mut sum = 0;
    for _ in 0..dice.dice {
        sum += random_number_state(state, dice.sides as i32);
    }
    sum
}

fn monster_place_new_state(
    state: &mut State,
    coord: Coord_t,
    creature_id: i32,
    sleeping: bool,
    monster_id: i32,
) {
    let creature = CREATURES_LIST[creature_id as usize];
    let hp = if (creature.defenses & CD_MAX_HP) != 0 {
        max_dice_roll(creature.hit_die) as i16
    } else {
        dice_roll_state(state, creature.hit_die) as i16
    };
    let speed = (i32::from(creature.speed) - 10 + i32::from(state.py.flags.speed)) as i16;
    let distance_from_player = coord_distance_between(state.py.pos, coord) as u8;
    let sleep_count = if sleeping {
        if creature.sleep_counter == 0 {
            0
        } else {
            let sleep_counter = i32::from(creature.sleep_counter);
            (sleep_counter * 2 + random_number_state(state, sleep_counter * 10)) as i16
        }
    } else {
        0
    };

    let monster = &mut state.monsters[monster_id as usize];
    monster.pos.y = coord.y;
    monster.pos.x = coord.x;
    monster.creature_id = creature_id as u16;
    monster.hp = hp;
    monster.speed = speed;
    monster.stunned_amount = 0;
    monster.distance_from_player = distance_from_player;
    monster.lit = false;
    monster.sleep_count = sleep_count;

    state.dg.floor[coord.y as usize][coord.x as usize].creature_id = monster_id as u8;
}

/// 70
pub fn monster_place_new(coord: Coord_t, creature_id: i32, sleeping: bool) -> bool {
    let monster_id = popm();
    if monster_id == -1 {
        return false;
    }

    with_state_mut(|state| {
        monster_place_new_state(state, coord, creature_id, sleeping, monster_id);
    });

    true
}

/// 124
pub fn monster_place_winning() {
    if crate::game::game(|g| g.total_winner) {
        return;
    }

    let coord = with_state_mut(|state| {
        let mut coord = Coord_t { y: 0, x: 0 };
        loop {
            coord.y = random_number_state(state, i32::from(state.dg.height) - 2);
            coord.x = random_number_state(state, i32::from(state.dg.width) - 2);
            let tile = &state.dg.floor[coord.y as usize][coord.x as usize];
            if tile.feature_id >= MIN_CLOSED_SPACE
                || tile.creature_id != 0
                || tile.treasure_id != 0
                || coord_distance_between(coord, state.py.pos) <= i32::from(MON_MAX_SIGHT)
            {
                continue;
            }
            break;
        }
        coord
    });

    let creature_id = with_state_mut(|state| {
        random_number_state(state, i32::from(MON_ENDGAME_MONSTERS)) - 1
            + i32::from(state.monster_levels[MON_MAX_LEVELS as usize])
    });

    let monster_id = popm();
    if monster_id == -1 {
        std::process::abort();
    }

    with_state_mut(|state| {
        let creature = CREATURES_LIST[creature_id as usize];
        let hp = if (creature.defenses & CD_MAX_HP) != 0 {
            max_dice_roll(creature.hit_die) as i16
        } else {
            dice_roll_state(state, creature.hit_die) as i16
        };
        let speed = (i32::from(creature.speed) - 10 + i32::from(state.py.flags.speed)) as i16;
        let distance_from_player = coord_distance_between(state.py.pos, coord) as u8;

        let monster = &mut state.monsters[monster_id as usize];
        monster.pos.y = coord.y;
        monster.pos.x = coord.x;
        monster.creature_id = creature_id as u16;
        monster.hp = hp;
        monster.speed = speed;
        monster.stunned_amount = 0;
        monster.distance_from_player = distance_from_player;
        monster.sleep_count = 0;

        state.dg.floor[coord.y as usize][coord.x as usize].creature_id = monster_id as u8;
    });
}

/// 160
#[doc(hidden)]
pub fn monster_get_one_suitable_for_level(level: i32) -> i32 {
    with_state_mut(|state| monster_get_one_suitable_for_level_state(state, level))
}

fn monster_get_one_suitable_for_level_state(state: &mut State, mut level: i32) -> i32 {
    if level == 0 {
        return random_number_state(state, i32::from(state.monster_levels[0])) - 1;
    }

    if level > i32::from(MON_MAX_LEVELS) {
        level = i32::from(MON_MAX_LEVELS);
    }

    if random_number_state(state, i32::from(monsters::MON_CHANCE_OF_NASTY)) == 1 {
        let abs_distribution = random_number_normal_distribution_state(state, 0, 4).abs();
        level += abs_distribution + 1;
        if level > i32::from(MON_MAX_LEVELS) {
            level = i32::from(MON_MAX_LEVELS);
        }
    } else {
        let num =
            i32::from(state.monster_levels[level as usize]) - i32::from(state.monster_levels[0]);
        let mut i = random_number_state(state, num) - 1;
        let j = random_number_state(state, num) - 1;
        if j > i {
            i = j;
        }
        level = i32::from(CREATURES_LIST[(i + i32::from(state.monster_levels[0])) as usize].level);
    }

    random_number_state(
        state,
        i32::from(state.monster_levels[level as usize])
            - i32::from(state.monster_levels[(level - 1) as usize]),
    ) - 1
        + i32::from(state.monster_levels[(level - 1) as usize])
}

/// 187
pub fn monster_place_new_within_distance(
    number: i32,
    distance_from_source: i32,
    mut sleeping: bool,
) {
    for _ in 0..number {
        let position = with_state_mut(|state| {
            let mut position = Coord_t { y: 0, x: 0 };
            loop {
                position.y = random_number_state(state, i32::from(state.dg.height) - 2);
                position.x = random_number_state(state, i32::from(state.dg.width) - 2);
                let tile = &state.dg.floor[position.y as usize][position.x as usize];
                if tile.feature_id >= MIN_CLOSED_SPACE
                    || tile.creature_id != 0
                    || coord_distance_between(position, state.py.pos) <= distance_from_source
                {
                    continue;
                }
                break;
            }
            position
        });

        let level = with_state(|state| i32::from(state.dg.current_level));
        let l = monster_get_one_suitable_for_level(level);

        if CREATURES_LIST[l as usize].sprite == b'd' || CREATURES_LIST[l as usize].sprite == b'D' {
            sleeping = true;
        }

        let _ = monster_place_new(position, l, sleeping);
    }
}

/// 215
fn place_monster_adjacent_to(monster_id: i32, coord: &mut Coord_t, slp: bool) -> bool {
    let mut placed = false;

    // `for (i = 0; i <= 9; i++)` with `i = 9` on success then increments to 10
    // and exits — equivalent to break (unlike dungeonPlaceRandomObjectNear's i<=10).
    for _ in 0..=9 {
        let position = with_state_mut(|state| {
            let mut position = Coord_t { y: 0, x: 0 };
            position.y = coord.y - 2 + random_number_state(state, 3);
            position.x = coord.x - 2 + random_number_state(state, 3);
            position
        });

        if coord_in_bounds(position) {
            let open = with_state(|state| {
                let tile = &state.dg.floor[position.y as usize][position.x as usize];
                tile.feature_id <= MAX_OPEN_SPACE && tile.creature_id == 0
            });
            if open {
                if !monster_place_new(position, monster_id, slp) {
                    return false;
                }
                coord.y = position.y;
                coord.x = position.x;
                placed = true;
                break;
            }
        }
    }

    placed
}

/// 221
pub fn monster_summon(coord: &mut Coord_t, sleeping: bool) -> bool {
    let level = with_state(|state| {
        i32::from(state.dg.current_level) + i32::from(MON_SUMMONED_LEVEL_ADJUST)
    });
    let monster_id = monster_get_one_suitable_for_level(level);
    place_monster_adjacent_to(monster_id, coord, sleeping)
}

/// 246
pub fn monster_summon_undead(coord: &mut Coord_t) -> bool {
    let monster_id = with_state_mut(|state| {
        let mut max_levels = i32::from(state.monster_levels[MON_MAX_LEVELS as usize]);
        let mut creature_id;

        loop {
            creature_id = random_number_state(state, max_levels) - 1;
            let mut i = 0;
            while i <= 19 {
                if (CREATURES_LIST[creature_id as usize].defenses & CD_UNDEAD) != 0 {
                    i = 20;
                    max_levels = 0;
                } else {
                    creature_id += 1;
                    if creature_id > max_levels {
                        i = 20;
                    } else {
                        i += 1;
                    }
                }
            }
            if max_levels == 0 {
                break;
            }
        }

        creature_id
    });

    place_monster_adjacent_to(monster_id, coord, false)
}

/// 285
pub fn compact_monsters() -> bool {
    terminal::print_message(Some("Compacting monsters..."));

    let mut cur_dis = 66;
    loop {
        let mut delete_any = false;
        let start_id = with_state(|state| i32::from(state.next_free_monster_id));

        for i in (i32::from(monsters::MON_MIN_INDEX_ID)..start_id).rev() {
            let distance =
                with_state(|state| i32::from(state.monsters[i as usize].distance_from_player));
            if cur_dis >= distance {
                continue;
            }
            if random_number(3) != 1 {
                continue;
            }

            let action = with_state(|state| {
                let creature_id = state.monsters[i as usize].creature_id as usize;
                if (CREATURES_LIST[creature_id].movement & CM_WIN) != 0 {
                    return 0;
                }
                if state.hack_monptr < i {
                    1
                } else {
                    2
                }
            });

            match action {
                0 => {}
                1 => {
                    dungeon_delete_monster(i);
                    delete_any = true;
                }
                2 => {
                    dungeon_remove_monster_from_level(i);
                }
                _ => {}
            }
        }

        if delete_any {
            return true;
        }

        cur_dis -= 6;
        if cur_dis < 0 {
            return false;
        }
    }
}
