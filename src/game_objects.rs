//! Port of src/game_objects.cpp — object heap and random object selection.

use crate::config::dungeon::objects::OBJ_NOTHING;
use crate::config::treasure::TREASURE_CHANCE_OF_GREAT_ITEM;
use crate::data_treasure::GAME_OBJECTS;
use crate::dungeon::{coord_distance_between, dungeon_delete_object, DungeonObject};
use crate::game::{random_number, random_number_state, with_state, with_state_mut, State};
use crate::inventory::inventory_item_copy_to;
use crate::treasure::{
    TV_BOW, TV_CHEST, TV_CLOSED_DOOR, TV_DIGGING, TV_DOWN_STAIR, TV_HAFTED, TV_HARD_ARMOR,
    TV_INVIS_TRAP, TV_OPEN_DOOR, TV_POLEARM, TV_RUBBLE, TV_SECRET_DOOR, TV_SOFT_ARMOR, TV_STAFF,
    TV_STORE_DOOR, TV_SWORD, TV_UP_STAIR, TV_VIS_TRAP,
};
use crate::types::{Coord_t, LEVEL_MAX_OBJECTS, TREASURE_MAX_LEVELS};
use crate::ui::draw_dungeon_panel;
use crate::ui_io::terminal;

/// C++ game_objects.cpp lines 14–67.
fn compact_objects() {
    terminal::print_message(Some("Compacting objects..."));

    let mut counter = 0;
    let mut current_distance = 66;

    while counter <= 0 {
        let (height, width, player_pos) = with_state(|state| {
            (
                i32::from(state.dg.height),
                i32::from(state.dg.width),
                state.py.pos,
            )
        });

        for y in 0..height {
            for x in 0..width {
                let coord = Coord_t { y, x };
                let treasure_id = with_state(|state| {
                    state.dg.floor[y as usize][x as usize].treasure_id
                });
                if treasure_id == 0 {
                    continue;
                }
                if coord_distance_between(coord, player_pos) <= current_distance {
                    continue;
                }

                let chance = with_state(|state| {
                    match state.game.treasure.list[treasure_id as usize].category_id {
                        TV_VIS_TRAP => 15,
                        TV_INVIS_TRAP | TV_RUBBLE | TV_OPEN_DOOR | TV_CLOSED_DOOR => 5,
                        TV_UP_STAIR | TV_DOWN_STAIR | TV_STORE_DOOR => 0,
                        TV_SECRET_DOOR => 3,
                        _ => 10,
                    }
                });

                if random_number(100) <= chance {
                    let _ = dungeon_delete_object(coord);
                    counter += 1;
                }
            }
        }

        if counter == 0 {
            current_distance -= 6;
        }
    }

    if current_distance < 66 {
        draw_dungeon_panel();
    }
}

/// C++ game_objects.cpp lines 101–117.
#[doc(hidden)]
pub fn item_bigger_than_chest(obj: &DungeonObject) -> bool {
    match obj.category_id {
        TV_CHEST | TV_BOW | TV_POLEARM | TV_HARD_ARMOR | TV_SOFT_ARMOR | TV_STAFF => true,
        TV_HAFTED | TV_SWORD | TV_DIGGING => obj.weight > 150,
        _ => false,
    }
}

/// C++ game_objects.cpp lines 70–76.
pub fn popt() -> i32 {
    let needs_compact = with_state(|state| {
        state.game.treasure.current_id == i16::from(LEVEL_MAX_OBJECTS)
    });
    if needs_compact {
        compact_objects();
    }
    with_state_mut(|state| {
        let id = i32::from(state.game.treasure.current_id);
        state.game.treasure.current_id += 1;
        id
    })
}

/// C++ game_objects.cpp lines 81–97.
pub fn pusht_state(state: &mut State, treasure_id: u8) {
    let current = state.game.treasure.current_id;
    if i32::from(treasure_id) != i32::from(current) - 1 {
        let last = (current - 1) as usize;
        state.game.treasure.list[treasure_id as usize] = state.game.treasure.list[last];
        for y in 0..state.dg.height {
            for x in 0..state.dg.width {
                if state.dg.floor[y as usize][x as usize].treasure_id == (current - 1) as u8 {
                    state.dg.floor[y as usize][x as usize].treasure_id = treasure_id;
                }
            }
        }
    }
    state.game.treasure.current_id -= 1;
    inventory_item_copy_to(
        OBJ_NOTHING as i16,
        &mut state.game.treasure.list[state.game.treasure.current_id as usize],
    );
}

/// C++ game_objects.cpp lines 81–97.
pub fn pusht(treasure_id: u8) {
    with_state_mut(|state| pusht_state(state, treasure_id));
}

/// C++ game_objects.cpp lines 120–171.
pub fn item_get_random_object_id(level: i32, must_be_small: bool) -> i32 {
    with_state_mut(|state| {
        let mut level = level;
        if level == 0 {
            return random_number_state(state, i32::from(state.treasure_levels[0])) - 1;
        }

        if level >= i32::from(TREASURE_MAX_LEVELS) {
            level = i32::from(TREASURE_MAX_LEVELS);
        } else if random_number_state(state, i32::from(TREASURE_CHANCE_OF_GREAT_ITEM)) == 1 {
            level = level * i32::from(TREASURE_MAX_LEVELS)
                / random_number_state(state, i32::from(TREASURE_MAX_LEVELS))
                + 1;
            if level > i32::from(TREASURE_MAX_LEVELS) {
                level = i32::from(TREASURE_MAX_LEVELS);
            }
        }

        let level_usize = level as usize;
        loop {
            let object_id = if random_number_state(state, 2) == 1 {
                random_number_state(state, i32::from(state.treasure_levels[level_usize])) - 1
            } else {
                let mut object_id =
                    random_number_state(state, i32::from(state.treasure_levels[level_usize])) - 1;
                let mut j =
                    random_number_state(state, i32::from(state.treasure_levels[level_usize])) - 1;
                if object_id < j {
                    object_id = j;
                }
                j = random_number_state(state, i32::from(state.treasure_levels[level_usize])) - 1;
                if object_id < j {
                    object_id = j;
                }
                let found_level = GAME_OBJECTS[state.sorted_objects[object_id as usize] as usize]
                    .depth_first_found;
                if found_level == 0 {
                    random_number_state(state, i32::from(state.treasure_levels[0])) - 1
                } else {
                    let found = found_level as usize;
                    random_number_state(
                        state,
                        i32::from(state.treasure_levels[found])
                            - i32::from(state.treasure_levels[found - 1]),
                    ) - 1
                        + i32::from(state.treasure_levels[found - 1])
                }
            };

            if !must_be_small
                || !item_bigger_than_chest(
                    &GAME_OBJECTS[state.sorted_objects[object_id as usize] as usize],
                )
            {
                return object_id;
            }
        }
    })
}
