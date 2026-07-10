//! Phase 2.5 — creatures_list + monster_attacks parity with data_creatures.cpp.

use umoria::config::monsters::defense as cd;
use umoria::config::monsters::move_flags as cm;
use umoria::config::monsters::spells as cs;
use umoria::data_creatures::{CREATURES_LIST, MONSTER_ATTACKS};
use umoria::dice::Dice;
use umoria::monster::{MON_ATTACK_TYPES, MON_MAX_CREATURES};

fn assert_creature(index: usize, expected: umoria::monster::Creature) {
    let actual = CREATURES_LIST[index];
    assert_eq!(actual.name, expected.name, "name at {index}");
    assert_eq!(actual.movement, expected.movement, "movement at {index}");
    assert_eq!(actual.spells, expected.spells, "spells at {index}");
    assert_eq!(actual.defenses, expected.defenses, "defenses at {index}");
    assert_eq!(
        actual.kill_exp_value, expected.kill_exp_value,
        "kill_exp_value at {index}"
    );
    assert_eq!(
        actual.sleep_counter, expected.sleep_counter,
        "sleep_counter at {index}"
    );
    assert_eq!(
        actual.area_affect_radius, expected.area_affect_radius,
        "area_affect_radius at {index}"
    );
    assert_eq!(actual.ac, expected.ac, "ac at {index}");
    assert_eq!(actual.speed, expected.speed, "speed at {index}");
    assert_eq!(actual.sprite, expected.sprite, "sprite at {index}");
    assert_eq!(actual.hit_die, expected.hit_die, "hit_die at {index}");
    assert_eq!(actual.damage, expected.damage, "damage at {index}");
    assert_eq!(actual.level, expected.level, "level at {index}");
}

fn assert_attack(index: usize, expected: umoria::monster::MonsterAttack) {
    let actual = MONSTER_ATTACKS[index];
    assert_eq!(actual.type_id, expected.type_id, "type_id at {index}");
    assert_eq!(
        actual.description_id, expected.description_id,
        "description_id at {index}"
    );
    assert_eq!(actual.dice, expected.dice, "dice at {index}");
}

#[test]
fn creatures_list_length() {
    assert_eq!(CREATURES_LIST.len(), 279);
    assert_eq!(CREATURES_LIST.len(), MON_MAX_CREATURES as usize);
}

#[test]
fn monster_attacks_length() {
    assert_eq!(MONSTER_ATTACKS.len(), 215);
    assert_eq!(MONSTER_ATTACKS.len(), MON_ATTACK_TYPES as usize);
}

#[test]
fn creatures_list_first_entry() {
    assert_creature(
        0,
        umoria::monster::Creature {
            name: "Filthy Street Urchin",
            movement: 0x0012_000A,
            spells: 0x0000_0000,
            defenses: 0x2034,
            kill_exp_value: 0,
            sleep_counter: 40,
            area_affect_radius: 4,
            ac: 1,
            speed: 11,
            sprite: b'p',
            hit_die: Dice { dice: 1, sides: 4 },
            damage: [72, 148, 0, 0],
            level: 0,
        },
    );
}

#[test]
fn creatures_list_floating_eye() {
    assert_creature(
        18,
        umoria::monster::Creature {
            name: "Floating Eye",
            movement: 0x0000_0001,
            spells: 0x0001_000D,
            defenses: 0x2100,
            kill_exp_value: 1,
            sleep_counter: 10,
            area_affect_radius: 2,
            ac: 6,
            speed: 11,
            sprite: b'e',
            hit_die: Dice { dice: 3, sides: 6 },
            damage: [146, 0, 0, 0],
            level: 1,
        },
    );
}

#[test]
fn creatures_list_ancient_multi_hued_dragon() {
    assert_creature(
        276,
        umoria::monster::Creature {
            name: "Ancient Multi-Hued Dragon",
            movement: 0x7F00_0002,
            spells: 0x00F8_9E05,
            defenses: 0x6005,
            kill_exp_value: 12000,
            sleep_counter: 70,
            area_affect_radius: 20,
            ac: 100,
            speed: 12,
            sprite: b'D',
            hit_die: Dice {
                dice: 52,
                sides: 40,
            },
            damage: [57, 57, 42, 0],
            level: 40,
        },
    );
}

#[test]
fn creatures_list_evil_iggy() {
    assert_creature(
        277,
        umoria::monster::Creature {
            name: "Evil Iggy",
            movement: 0x7F13_0002,
            spells: 0x0001_D713,
            defenses: 0x5004,
            kill_exp_value: 18000,
            sleep_counter: 0,
            area_affect_radius: 30,
            ac: 80,
            speed: 12,
            sprite: b'p',
            hit_die: Dice {
                dice: 60,
                sides: 40,
            },
            damage: [81, 150, 0, 0],
            level: 50,
        },
    );
}

#[test]
fn creatures_list_balrog() {
    assert_creature(
        278,
        umoria::monster::Creature {
            name: "Balrog",
            movement: 0xFF1F_0002,
            spells: 0x0081_C743,
            defenses: 0x5004,
            kill_exp_value: 55000,
            sleep_counter: 0,
            area_affect_radius: 40,
            ac: 125,
            speed: 13,
            sprite: b'B',
            hit_die: Dice {
                dice: 75,
                sides: 40,
            },
            damage: [104, 78, 214, 0],
            level: 100,
        },
    );
}

#[test]
fn monster_attacks_spot_checks() {
    assert_attack(
        0,
        umoria::monster::MonsterAttack {
            type_id: 0,
            description_id: 0,
            dice: Dice { dice: 0, sides: 0 },
        },
    );
    assert_attack(
        1,
        umoria::monster::MonsterAttack {
            type_id: 1,
            description_id: 1,
            dice: Dice { dice: 1, sides: 2 },
        },
    );
    assert_attack(
        212,
        umoria::monster::MonsterAttack {
            type_id: 23,
            description_id: 1,
            dice: Dice { dice: 1, sides: 1 },
        },
    );
    assert_attack(
        214,
        umoria::monster::MonsterAttack {
            type_id: 24,
            description_id: 5,
            dice: Dice { dice: 0, sides: 0 },
        },
    );
}

#[test]
fn no_accidental_all_zero_creature_entries() {
    for (i, creature) in CREATURES_LIST.iter().enumerate() {
        let all_zero = creature.movement == 0
            && creature.spells == 0
            && creature.defenses == 0
            && creature.kill_exp_value == 0
            && creature.sleep_counter == 0
            && creature.area_affect_radius == 0
            && creature.ac == 0
            && creature.speed == 0
            && creature.sprite == 0
            && creature.hit_die == Dice::default()
            && creature.damage == [0, 0, 0, 0]
            && creature.level == 0
            && creature.name.is_empty();
        assert!(
            !all_zero,
            "creatures_list[{i}] looks like an accidental default entry"
        );
    }
}

#[test]
fn flag_equivalence_spot_checks() {
    let urchin = &CREATURES_LIST[0];
    assert_eq!(
        urchin.movement,
        cm::CM_PICKS_UP | cm::CM_OPEN_DOOR | cm::CM_20_RANDOM | cm::CM_MOVE_NORMAL
    );
    assert_eq!(
        urchin.defenses,
        cd::CD_INFRA | cd::CD_FIRE | cd::CD_FROST | cd::CD_EVIL
    );

    let eye = &CREATURES_LIST[18];
    assert_eq!(eye.movement, cm::CM_ATTACK_ONLY);
    assert_eq!(eye.spells, cs::CS_DRAIN_MANA | 0x0000_000D);
    assert_eq!(eye.defenses, cd::CD_INFRA | cd::CD_LIGHT);

    let balrog = &CREATURES_LIST[278];
    assert_ne!(balrog.movement & cm::CM_WIN, 0);
}

#[test]
fn full_table_no_duplicate_default_creature_blocks() {
    let mut names: Vec<&str> = CREATURES_LIST.iter().map(|c| c.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(
        names.len(),
        CREATURES_LIST.len(),
        "creature names should be unique"
    );
}
