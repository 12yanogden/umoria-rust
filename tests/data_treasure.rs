//! `data_treasure` (`game_objects` + `special_item_names`).
//! Phase 2.8 (partial) — descriptive string arrays from data_tables.cpp
#![allow(
    clippy::assertions_on_constants,
    reason = "constant assertions document table sizes from C++ headers"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::data_treasure::{
    AMULETS, COLORS, GAME_OBJECTS, METALS, MUSHROOMS, ROCKS, SPECIAL_ITEM_NAMES, SYLLABLES, WOODS,
};
use umoria::dice::Dice;
use umoria::dungeon::DungeonObject;
use umoria::identification::{
    SpecialNameIds, MAX_AMULETS, MAX_COLORS, MAX_METALS, MAX_MUSHROOMS, MAX_ROCKS, MAX_SYLLABLES,
    MAX_WOODS,
};
use umoria::treasure::{
    TV_CHEST, TV_CLOSED_DOOR, TV_DOWN_STAIR, TV_FOOD, TV_NOTHING, TV_OPEN_DOOR, TV_POTION1,
    TV_SCROLL1, TV_SECRET_DOOR, TV_STORE_DOOR, TV_SWORD, TV_UP_STAIR,
};
use umoria::types::MAX_OBJECTS_IN_GAME;

fn assert_object(index: usize, expected: DungeonObject) {
    let actual = &GAME_OBJECTS[index];
    assert_eq!(actual.name, expected.name, "index {index} name");
    assert_eq!(actual.flags, expected.flags, "index {index} flags");
    assert_eq!(
        actual.category_id, expected.category_id,
        "index {index} category_id"
    );
    assert_eq!(actual.sprite, expected.sprite, "index {index} sprite");
    assert_eq!(actual.misc_use, expected.misc_use, "index {index} misc_use");
    assert_eq!(actual.cost, expected.cost, "index {index} cost");
    assert_eq!(
        actual.sub_category_id, expected.sub_category_id,
        "index {index} sub_category_id"
    );
    assert_eq!(
        actual.items_count, expected.items_count,
        "index {index} items_count"
    );
    assert_eq!(actual.weight, expected.weight, "index {index} weight");
    assert_eq!(actual.to_hit, expected.to_hit, "index {index} to_hit");
    assert_eq!(
        actual.to_damage, expected.to_damage,
        "index {index} to_damage"
    );
    assert_eq!(actual.ac, expected.ac, "index {index} ac");
    assert_eq!(actual.to_ac, expected.to_ac, "index {index} to_ac");
    assert_eq!(actual.damage, expected.damage, "index {index} damage");
    assert_eq!(
        actual.depth_first_found, expected.depth_first_found,
        "index {index} depth_first_found"
    );
}

#[allow(
    clippy::too_many_arguments,
    reason = "test helper mirrors multi-arg C++ setup"
)]
fn obj(
    name: &'static str,
    flags: u32,
    category_id: u8,
    sprite: u8,
    misc_use: i16,
    cost: i32,
    sub_category_id: u8,
    items_count: u8,
    weight: u16,
    to_hit: i16,
    to_damage: i16,
    ac: i16,
    to_ac: i16,
    damage: Dice,
    depth_first_found: u8,
) -> DungeonObject {
    DungeonObject {
        name,
        flags,
        category_id,
        sprite,
        misc_use,
        cost,
        sub_category_id,
        items_count,
        weight,
        to_hit,
        to_damage,
        ac,
        to_ac,
        damage,
        depth_first_found,
    }
}

// ---------------------------------------------------------------------------
// Length gates
// ---------------------------------------------------------------------------

#[test]
fn game_objects_length() {
    assert_eq!(GAME_OBJECTS.len(), 420);
    assert_eq!(GAME_OBJECTS.len(), MAX_OBJECTS_IN_GAME as usize);
}

#[test]
fn special_item_names_length() {
    assert_eq!(
        SPECIAL_ITEM_NAMES.len(),
        SpecialNameIds::SN_ARRAY_SIZE as usize
    );
    assert_eq!(SPECIAL_ITEM_NAMES.len(), 56);
}

#[test]
fn descriptive_array_lengths() {
    assert_eq!(COLORS.len(), MAX_COLORS as usize);
    assert_eq!(COLORS.len(), 49);
    assert_eq!(MUSHROOMS.len(), MAX_MUSHROOMS as usize);
    assert_eq!(MUSHROOMS.len(), 22);
    assert_eq!(WOODS.len(), MAX_WOODS as usize);
    assert_eq!(WOODS.len(), 25);
    assert_eq!(METALS.len(), MAX_METALS as usize);
    assert_eq!(METALS.len(), 25);
    assert_eq!(ROCKS.len(), MAX_ROCKS as usize);
    assert_eq!(ROCKS.len(), 32);
    assert_eq!(AMULETS.len(), MAX_AMULETS as usize);
    assert_eq!(AMULETS.len(), 11);
    assert_eq!(SYLLABLES.len(), MAX_SYLLABLES as usize);
    assert_eq!(SYLLABLES.len(), 153);
}

// ---------------------------------------------------------------------------
// game_objects spot checks (field-for-field vs C++ data_treasure.cpp)
// ---------------------------------------------------------------------------

#[test]
fn game_objects_spot_checks() {
    assert_object(
        0,
        obj(
            "Poison",
            0x0000_0001,
            TV_FOOD,
            b',',
            500,
            0,
            64,
            1,
            1,
            0,
            0,
            0,
            0,
            Dice { dice: 0, sides: 0 },
            7,
        ),
    );

    assert_object(
        32,
        obj(
            "& Broken Dagger",
            0,
            TV_SWORD,
            b'|',
            0,
            0,
            5,
            1,
            15,
            -2,
            -2,
            0,
            0,
            Dice { dice: 1, sides: 1 },
            0,
        ),
    );

    assert_object(
        34,
        obj(
            "& Bastard Sword",
            0,
            TV_SWORD,
            b'|',
            0,
            350,
            7,
            1,
            140,
            0,
            0,
            0,
            0,
            Dice { dice: 3, sides: 4 },
            14,
        ),
    );

    assert_object(
        113,
        obj(
            "Rusty Chain Mail",
            0,
            umoria::treasure::TV_HARD_ARMOR,
            b'[',
            0,
            0,
            3,
            1,
            220,
            -5,
            0,
            14,
            -8,
            Dice { dice: 1, sides: 4 },
            26,
        ),
    );

    assert_object(
        145,
        obj(
            "Weakness",
            0x8000_0001,
            umoria::treasure::TV_RING,
            b'=',
            -5,
            0,
            13,
            1,
            2,
            0,
            0,
            0,
            0,
            Dice { dice: 0, sides: 0 },
            7,
        ),
    );

    assert_object(
        176,
        obj(
            "Identify",
            0x0000_0008,
            TV_SCROLL1,
            b'?',
            0,
            50,
            67,
            1,
            5,
            0,
            0,
            0,
            0,
            Dice { dice: 0, sides: 0 },
            1,
        ),
    );

    assert_object(
        224,
        obj(
            "Water",
            0,
            TV_POTION1,
            b'!',
            200,
            0,
            66,
            1,
            4,
            0,
            0,
            0,
            0,
            Dice { dice: 1, sides: 1 },
            0,
        ),
    );

    assert_object(
        367,
        obj(
            "& open door",
            0,
            TV_OPEN_DOOR,
            b'\'',
            0,
            0,
            1,
            1,
            0,
            0,
            0,
            0,
            0,
            Dice { dice: 1, sides: 1 },
            0,
        ),
    );

    assert_object(
        368,
        obj(
            "& closed door",
            0,
            TV_CLOSED_DOOR,
            b'+',
            0,
            0,
            19,
            1,
            0,
            0,
            0,
            0,
            0,
            Dice { dice: 1, sides: 1 },
            0,
        ),
    );

    assert_object(
        369,
        obj(
            "& secret door",
            0,
            TV_SECRET_DOOR,
            b'#',
            0,
            0,
            19,
            1,
            0,
            0,
            0,
            0,
            0,
            Dice { dice: 1, sides: 1 },
            0,
        ),
    );

    assert_object(
        370,
        obj(
            "an up staircase",
            0,
            TV_UP_STAIR,
            b'<',
            0,
            0,
            1,
            1,
            0,
            0,
            0,
            0,
            0,
            Dice { dice: 1, sides: 1 },
            0,
        ),
    );

    assert_object(
        371,
        obj(
            "a down staircase",
            0,
            TV_DOWN_STAIR,
            b'>',
            0,
            0,
            1,
            1,
            0,
            0,
            0,
            0,
            0,
            Dice { dice: 1, sides: 1 },
            0,
        ),
    );

    assert_object(
        372,
        obj(
            "General Store",
            0,
            TV_STORE_DOOR,
            b'1',
            0,
            0,
            101,
            1,
            0,
            0,
            0,
            0,
            0,
            Dice { dice: 0, sides: 0 },
            0,
        ),
    );

    assert_object(
        417,
        obj(
            "nothing",
            0,
            TV_NOTHING,
            b' ',
            0,
            0,
            64,
            0,
            0,
            0,
            0,
            0,
            0,
            Dice { dice: 0, sides: 0 },
            0,
        ),
    );

    assert_object(
        418,
        obj(
            "& ruined chest",
            0,
            TV_CHEST,
            b'&',
            0,
            0,
            0,
            1,
            250,
            0,
            0,
            0,
            0,
            Dice { dice: 0, sides: 0 },
            0,
        ),
    );

    assert_object(
        419,
        obj(
            "",
            0,
            TV_NOTHING,
            b' ',
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            Dice { dice: 0, sides: 0 },
            0,
        ),
    );
}

// ---------------------------------------------------------------------------
// special_item_names spot checks
// ---------------------------------------------------------------------------

#[test]
fn special_item_names_spot_checks() {
    // C++ CNIL → "" in Rust &str table
    assert_eq!(SPECIAL_ITEM_NAMES[SpecialNameIds::SN_NULL as usize], "");
    assert_eq!(
        SPECIAL_ITEM_NAMES[SpecialNameIds::SN_FREE_ACTION as usize],
        "of Free Action"
    );
    assert_eq!(
        SPECIAL_ITEM_NAMES[SpecialNameIds::SN_MAGI as usize],
        "of the Magi"
    );
    assert_eq!(
        SPECIAL_ITEM_NAMES[SpecialNameIds::SN_SLAY_ANIMAL as usize],
        "of Slay Animal"
    );
}

// ---------------------------------------------------------------------------
// Descriptive string array spot checks (data_tables.cpp)
// ---------------------------------------------------------------------------

#[test]
fn descriptive_arrays_spot_checks() {
    assert_eq!(COLORS[0], "Icky Green");
    assert_eq!(COLORS[1], "Light Brown");
    assert_eq!(COLORS[2], "Clear");
    assert_eq!(COLORS[48], "Yellow");

    assert_eq!(MUSHROOMS[0], "Blue");
    assert_eq!(MUSHROOMS[21], "Yellow");

    assert_eq!(WOODS[0], "Aspen");
    assert_eq!(WOODS[24], "Walnut");

    assert_eq!(METALS[0], "Aluminum");
    assert_eq!(METALS[24], "Zinc-Plated");

    assert_eq!(ROCKS[0], "Alexandrite");
    assert_eq!(ROCKS[31], "Zircon");

    assert_eq!(AMULETS[0], "Amber");
    assert_eq!(AMULETS[10], "Tortoise Shell");

    assert_eq!(SYLLABLES[0], "a");
    assert_eq!(SYLLABLES[152], "zun");
}
