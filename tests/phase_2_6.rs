//! Phase 2.6 — data_player.cpp static tables (+ blows_table from data_tables.cpp).

use umoria::config::spells::{SPELL_TYPE_MAGE, SPELL_TYPE_NONE};
use umoria::data_player::{
    BLOWS_TABLE, CHARACTER_BACKGROUNDS, CHARACTER_RACES, CLASSES, CLASS_BASE_PROVISIONS,
    CLASS_LEVEL_ADJ, CLASS_RANK_TITLES, MAGIC_SPELLS, SPELL_NAMES,
};
use umoria::player::{
    CLASS_MAX_LEVEL_ADJUST, PLAYER_MAX_BACKGROUNDS, PLAYER_MAX_CLASSES, PLAYER_MAX_LEVEL,
    PLAYER_MAX_RACES,
};

// ---------------------------------------------------------------------------
// 1. Dimension checks
// ---------------------------------------------------------------------------
#[test]
fn table_dimensions() {
    assert_eq!(CHARACTER_RACES.len(), PLAYER_MAX_RACES as usize);
    assert_eq!(CHARACTER_BACKGROUNDS.len(), PLAYER_MAX_BACKGROUNDS as usize);
    assert_eq!(CLASSES.len(), PLAYER_MAX_CLASSES as usize);
    assert_eq!(CLASS_RANK_TITLES.len(), PLAYER_MAX_CLASSES as usize);
    for row in CLASS_RANK_TITLES.iter() {
        assert_eq!(row.len(), PLAYER_MAX_LEVEL as usize);
    }
    assert_eq!(CLASS_LEVEL_ADJ.len(), PLAYER_MAX_CLASSES as usize);
    for row in CLASS_LEVEL_ADJ.iter() {
        assert_eq!(row.len(), CLASS_MAX_LEVEL_ADJUST as usize);
    }
    assert_eq!(CLASS_BASE_PROVISIONS.len(), PLAYER_MAX_CLASSES as usize);
    for row in CLASS_BASE_PROVISIONS.iter() {
        assert_eq!(row.len(), 5);
    }
    assert_eq!(MAGIC_SPELLS.len(), PLAYER_MAX_CLASSES as usize - 1);
    for row in MAGIC_SPELLS.iter() {
        assert_eq!(row.len(), 31);
    }
    assert_eq!(SPELL_NAMES.len(), 62);
    assert_eq!(BLOWS_TABLE.len(), 7);
    for row in BLOWS_TABLE.iter() {
        assert_eq!(row.len(), 6);
    }
}

// ---------------------------------------------------------------------------
// 2. Spot-checks vs C++ source
// ---------------------------------------------------------------------------
#[test]
fn class_rank_titles_spot_checks() {
    let titles = &*CLASS_RANK_TITLES;
    assert_eq!(titles[0][0], "Rookie");
    assert_eq!(titles[0][39], "Lord Noble");
    assert_eq!(titles[1][0], "Novice");
    assert_eq!(titles[5][39], "High Lord");
}

#[test]
fn character_races_spot_checks() {
    let races = &*CHARACTER_RACES;
    let human = &races[0];
    assert_eq!(human.name, "Human");
    assert_eq!(human.str_adjustment, 0);
    assert_eq!(human.int_adjustment, 0);
    assert_eq!(human.wis_adjustment, 0);
    assert_eq!(human.dex_adjustment, 0);
    assert_eq!(human.con_adjustment, 0);
    assert_eq!(human.chr_adjustment, 0);
    assert_eq!(human.base_age, 14);
    assert_eq!(human.hit_points_base, 10);
    assert_eq!(human.exp_factor_base, 100);
    assert_eq!(human.classes_bit_field, 0x3F);

    let half_troll = &races[7];
    assert_eq!(half_troll.name, "Half-Troll");
    assert_eq!(half_troll.str_adjustment, 4);
    assert_eq!(half_troll.chr_adjustment, -6);
    assert_eq!(half_troll.classes_bit_field, 0x05);
}

#[test]
fn classes_spot_checks() {
    let classes = &*CLASSES;
    let warrior = &classes[0];
    assert_eq!(warrior.title, "Warrior");
    assert_eq!(warrior.hit_points, 9);
    assert_eq!(warrior.disarm_traps, 25);
    assert_eq!(warrior.saving_throw, 18);
    assert_eq!(warrior.class_to_use_mage_spells, SPELL_TYPE_NONE);
    assert_eq!(warrior.experience_factor, 0);
    assert_eq!(warrior.min_level_for_spell_casting, 0);

    let mage = &classes[1];
    assert_eq!(mage.title, "Mage");
    assert_eq!(mage.class_to_use_mage_spells, SPELL_TYPE_MAGE);
    assert_eq!(mage.experience_factor, 30);
}

#[test]
fn magic_spells_spot_checks() {
    let spells = &*MAGIC_SPELLS;
    let magic_missile = spells[0][0];
    assert_eq!(magic_missile.level_required, 1);
    assert_eq!(magic_missile.mana_required, 1);
    assert_eq!(magic_missile.failure_chance, 22);
    assert_eq!(magic_missile.exp_gain_for_learning, 1);

    let rogue_sentinel = spells[2][0];
    assert_eq!(rogue_sentinel.level_required, 99);
    assert_eq!(rogue_sentinel.mana_required, 99);
    assert_eq!(rogue_sentinel.failure_chance, 0);
    assert_eq!(rogue_sentinel.exp_gain_for_learning, 0);
}

#[test]
fn class_level_adj_and_provisions_spot_checks() {
    assert_eq!(CLASS_LEVEL_ADJ[0], [4, 4, 2, 2, 3]);
    assert_eq!(CLASS_BASE_PROVISIONS[0], [344, 365, 123, 30, 103]);
}

#[test]
fn spell_names_spot_checks() {
    assert_eq!(SPELL_NAMES[0], "Magic Missile");
    assert_eq!(SPELL_NAMES[30], "Genocide");
    assert_eq!(SPELL_NAMES[31], "Detect Evil");
    assert_eq!(SPELL_NAMES[61], "Holy Word");
}

#[test]
fn character_backgrounds_spot_checks() {
    let backgrounds = &*CHARACTER_BACKGROUNDS;
    let first = &backgrounds[0];
    assert_eq!(
        first.info,
        "You are the illegitimate and unacknowledged child "
    );
    assert_eq!(first.roll, 10);
    assert_eq!(first.chart, 1);
    assert_eq!(first.next, 2);
    assert_eq!(first.bonus, 25);

    let last = &backgrounds[127];
    assert_eq!(last.info, "leprous skin.");
    assert_eq!(last.roll, 100);
    assert_eq!(last.chart, 66);
    assert_eq!(last.next, 0);
    assert_eq!(last.bonus, 50);
}

#[test]
fn blows_table_spot_checks() {
    assert_eq!(BLOWS_TABLE[0], [1, 1, 1, 1, 1, 1]);
    assert_eq!(BLOWS_TABLE[6], [2, 2, 3, 3, 4, 4]);
}
