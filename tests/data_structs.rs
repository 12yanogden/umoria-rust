//! Shared data-bearing struct definitions & container declarations.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use core::mem::{align_of, size_of};
use std::os::raw::c_char;

use umoria::character::{Background, Class, Race};
use umoria::data_creatures::{CREATURES_LIST, MONSTER_ATTACKS};
use umoria::data_player::{
    BLOWS_TABLE, CHARACTER_BACKGROUNDS, CHARACTER_RACES, CLASSES, CLASS_BASE_PROVISIONS,
    CLASS_LEVEL_ADJ, CLASS_RANK_TITLES, MAGIC_SPELLS, SPELL_NAMES,
};
use umoria::data_recall::{
    RECALL_DESCRIPTION_ATTACK_METHOD, RECALL_DESCRIPTION_ATTACK_TYPE, RECALL_DESCRIPTION_BREATH,
    RECALL_DESCRIPTION_HOW_MUCH, RECALL_DESCRIPTION_MOVE, RECALL_DESCRIPTION_SPELL,
    RECALL_DESCRIPTION_WEAKNESS,
};
use umoria::data_store_owners::STORE_OWNERS;
use umoria::data_stores::{
    RACE_GOLD_ADJUSTMENTS, SPEECH_BUYING_HAGGLE, SPEECH_BUYING_HAGGLE_FINAL,
    SPEECH_GET_OUT_OF_MY_STORE, SPEECH_HAGGLING_TRY_AGAIN, SPEECH_INSULTED_HAGGLING_DONE,
    SPEECH_SALE_ACCEPTED, SPEECH_SELLING_HAGGLE, SPEECH_SELLING_HAGGLE_FINAL, SPEECH_SORRY, STORES,
    STORE_CHOICES,
};
use umoria::data_treasure::{
    AMULETS, COLORS, GAME_OBJECTS, METALS, MUSHROOMS, ROCKS, SPECIAL_ITEM_NAMES, SYLLABLES, WOODS,
};
use umoria::dice::Dice;
use umoria::dungeon::{Dungeon, DungeonObject};
use umoria::dungeon_tile::{
    Tile, MAX_CAVE_FLOOR, MAX_CAVE_ROOM, MAX_OPEN_SPACE, MIN_CAVE_WALL, MIN_CLOSED_SPACE,
    TILE_BLOCKED_FLOOR, TILE_BOUNDARY_WALL, TILE_CORR_FLOOR, TILE_DARK_FLOOR, TILE_GRANITE_WALL,
    TILE_LIGHT_FLOOR, TILE_MAGMA_WALL, TILE_NULL_WALL, TILE_QUARTZ_WALL, TMP1_WALL, TMP2_WALL,
};
use umoria::identification::{
    SpecialNameIds, MAX_AMULETS, MAX_COLORS, MAX_METALS, MAX_MUSHROOMS, MAX_ROCKS, MAX_SYLLABLES,
    MAX_TITLES, MAX_WOODS,
};
use umoria::inventory::{
    Inventory, PlayerEquipment, INSCRIP_SIZE, ITEM_GROUP_MAX, ITEM_GROUP_MIN, ITEM_NEVER_STACK_MAX,
    ITEM_NEVER_STACK_MIN, ITEM_SINGLE_STACK_MAX, ITEM_SINGLE_STACK_MIN, PLAYER_INVENTORY_SIZE,
};
use umoria::monster::{
    Creature, Monster, MonsterAttack, MON_ATTACK_TYPES, MON_MAX_ATTACKS, MON_MAX_CREATURES,
    MON_MAX_LEVELS, MON_TOTAL_ALLOCATIONS,
};
use umoria::player::{
    ClassRankTitle, Player, PlayerAttr, PlayerClassLevelAdj, BTH_PER_PLUS_TO_HIT_ADJUST,
    CLASS_MAX_LEVEL_ADJUST, CLASS_MISC_HIT, PLAYER_MAX_BACKGROUNDS, PLAYER_MAX_CLASSES,
    PLAYER_MAX_LEVEL, PLAYER_MAX_RACES, PLAYER_NAME_SIZE,
};
use umoria::recall::Recall;
use umoria::spells::{MagicSpellFlags, Spell};
use umoria::store::{
    InventoryRecord, Owner, Store, COST_ADJUSTMENT, MAX_OWNERS, MAX_STORES,
    SPEECH_BUYING_HAGGLE as SPEECH_BUYING_HAGGLE_N,
    SPEECH_BUYING_HAGGLE_FINAL as SPEECH_BUYING_HAGGLE_FINAL_N,
    SPEECH_GET_OUT_OF_MY_STORE as SPEECH_GET_OUT_OF_MY_STORE_N,
    SPEECH_HAGGLING_TRY_AGAIN as SPEECH_HAGGLING_TRY_AGAIN_N,
    SPEECH_INSULTED_HAGGLING_DONE as SPEECH_INSULTED_HAGGLING_DONE_N,
    SPEECH_SALE_ACCEPTED as SPEECH_SALE_ACCEPTED_N,
    SPEECH_SELLING_HAGGLE as SPEECH_SELLING_HAGGLE_N,
    SPEECH_SELLING_HAGGLE_FINAL as SPEECH_SELLING_HAGGLE_FINAL_N, SPEECH_SORRY as SPEECH_SORRY_N,
    STORE_MAX_DISCRETE_ITEMS, STORE_MAX_ITEM_TYPES,
};
use umoria::treasure::{
    TV_AMULET, TV_ARROW, TV_BOLT, TV_BOOTS, TV_BOW, TV_CHEST, TV_CLOAK, TV_CLOSED_DOOR, TV_DIGGING,
    TV_DOWN_STAIR, TV_FLASK, TV_FOOD, TV_GLOVES, TV_GOLD, TV_HAFTED, TV_HARD_ARMOR, TV_HELM,
    TV_INVIS_TRAP, TV_LIGHT, TV_MAGIC_BOOK, TV_MAX_ENCHANT, TV_MAX_OBJECT, TV_MAX_PICK_UP,
    TV_MAX_VISIBLE, TV_MAX_WEAR, TV_MIN_DOORS, TV_MIN_ENCHANT, TV_MIN_VISIBLE, TV_MIN_WEAR,
    TV_MISC, TV_NEVER, TV_NOTHING, TV_OPEN_DOOR, TV_POLEARM, TV_POTION1, TV_POTION2,
    TV_PRAYER_BOOK, TV_RING, TV_RUBBLE, TV_SCROLL1, TV_SCROLL2, TV_SECRET_DOOR, TV_SHIELD,
    TV_SLING_AMMO, TV_SOFT_ARMOR, TV_SPIKE, TV_STAFF, TV_STORE_DOOR, TV_SWORD, TV_UP_STAIR,
    TV_VIS_TRAP, TV_WAND,
};
use umoria::types::{
    Coord_t, Screen, LEVEL_MAX_OBJECTS, MAX_DUNGEON_OBJECTS, MAX_HEIGHT, MAX_OBJECTS_IN_GAME,
    MAX_WIDTH, NORMAL_TABLE_SIZE, OBJECT_IDENT_SIZE, SN_ARRAY_SIZE, TREASURE_MAX_LEVELS,
};
use umoria::ui::Panel;

// --------------------------------------------------------------------------
// 1. Size / layout constants (compile-time)
// --------------------------------------------------------------------------
const _: () = {
    assert!(MON_MAX_CREATURES == 279);
    assert!(MON_ATTACK_TYPES == 215);
    assert!(MON_TOTAL_ALLOCATIONS == 125);
    assert!(MON_MAX_LEVELS == 40);
    assert!(MON_MAX_ATTACKS == 4);

    assert!(PLAYER_MAX_LEVEL == 40);
    assert!(PLAYER_MAX_CLASSES == 6);
    assert!(PLAYER_MAX_RACES == 8);
    assert!(PLAYER_MAX_BACKGROUNDS == 128);
    assert!(CLASS_MAX_LEVEL_ADJUST == 5);
    assert!(CLASS_MISC_HIT == 4);
    assert!(BTH_PER_PLUS_TO_HIT_ADJUST == 3);
    assert!(PLAYER_NAME_SIZE == 27);

    assert!(MAX_OWNERS == 18);
    assert!(MAX_STORES == 6);
    assert!(STORE_MAX_DISCRETE_ITEMS == 24);
    assert!(STORE_MAX_ITEM_TYPES == 26);
    assert!(COST_ADJUSTMENT == 100);

    assert!(PLAYER_INVENTORY_SIZE == 34);
    assert!(INSCRIP_SIZE == 13);
    assert!(ITEM_NEVER_STACK_MIN == 0);
    assert!(ITEM_NEVER_STACK_MAX == 63);
    assert!(ITEM_SINGLE_STACK_MIN == 64);
    assert!(ITEM_SINGLE_STACK_MAX == 192);
    assert!(ITEM_GROUP_MIN == 192);
    assert!(ITEM_GROUP_MAX == 255);

    assert!(MAX_COLORS == 49);
    assert!(MAX_MUSHROOMS == 22);
    assert!(MAX_WOODS == 25);
    assert!(MAX_METALS == 25);
    assert!(MAX_ROCKS == 32);
    assert!(MAX_AMULETS == 11);
    assert!(MAX_TITLES == 45);
    assert!(MAX_SYLLABLES == 153);

    assert!(MAX_OBJECTS_IN_GAME == 420);
    assert!(MAX_DUNGEON_OBJECTS == 344);
    assert!(OBJECT_IDENT_SIZE == 448);
    assert!(LEVEL_MAX_OBJECTS == 175);
    assert!(TREASURE_MAX_LEVELS == 50);

    assert!(TILE_NULL_WALL == 0);
    assert!(TILE_DARK_FLOOR == 1);
    assert!(TILE_LIGHT_FLOOR == 2);
    assert!(MAX_CAVE_ROOM == 2);
    assert!(TILE_CORR_FLOOR == 3);
    assert!(TILE_BLOCKED_FLOOR == 4);
    assert!(MAX_CAVE_FLOOR == 4);
    assert!(MAX_OPEN_SPACE == 3);
    assert!(MIN_CLOSED_SPACE == 4);
    assert!(TMP1_WALL == 8);
    assert!(TMP2_WALL == 9);
    assert!(MIN_CAVE_WALL == 12);
    assert!(TILE_GRANITE_WALL == 12);
    assert!(TILE_MAGMA_WALL == 13);
    assert!(TILE_QUARTZ_WALL == 14);
    assert!(TILE_BOUNDARY_WALL == 15);

    assert!(MAX_HEIGHT == 66);
    assert!(MAX_WIDTH == 198);

    assert!(SPEECH_SALE_ACCEPTED_N == 14);
    assert!(SPEECH_SELLING_HAGGLE_FINAL_N == 3);
    assert!(SPEECH_SELLING_HAGGLE_N == 16);
    assert!(SPEECH_BUYING_HAGGLE_FINAL_N == 3);
    assert!(SPEECH_BUYING_HAGGLE_N == 15);
    assert!(SPEECH_INSULTED_HAGGLING_DONE_N == 5);
    assert!(SPEECH_GET_OUT_OF_MY_STORE_N == 5);
    assert!(SPEECH_HAGGLING_TRY_AGAIN_N == 10);
    assert!(SPEECH_SORRY_N == 5);

    assert!(NORMAL_TABLE_SIZE == 256);
    assert!(TV_NEVER == -1);
};

#[test]
fn constant_rust_types_match_expected() {
    const _: u16 = MON_MAX_CREATURES;
    const _: u8 = MON_ATTACK_TYPES;
    const _: u8 = MON_TOTAL_ALLOCATIONS;
    const _: u16 = MAX_OBJECTS_IN_GAME;
    const _: u16 = MAX_DUNGEON_OBJECTS;
    const _: u16 = OBJECT_IDENT_SIZE;
    const _: u8 = TILE_NULL_WALL;
    const _: i8 = TV_NEVER;
}

// --------------------------------------------------------------------------
// 2. Struct field shapes and size_of sanity checks
// --------------------------------------------------------------------------
#[test]
fn creature_t_layout() {
    let c = Creature {
        name: "test",
        movement: 1,
        spells: 2,
        defenses: 3,
        kill_exp_value: 4,
        sleep_counter: 5,
        area_affect_radius: 6,
        ac: 7,
        speed: 8,
        sprite: b'x',
        hit_die: Dice { dice: 1, sides: 6 },
        damage: [1, 2, 3, 4],
        level: 9,
    };
    let _ = c.name;
    let _ = c.movement;
    let _ = c.spells;
    let _ = c.defenses;
    let _ = c.kill_exp_value;
    let _ = c.sleep_counter;
    let _ = c.area_affect_radius;
    let _ = c.ac;
    let _ = c.speed;
    let _ = c.sprite;
    let _ = c.hit_die;
    let _ = c.damage;
    let _ = c.level;
    assert!(size_of::<Creature>() >= size_of::<Dice>() + 4);
}

#[test]
fn monster_t_layout() {
    let m = Monster {
        hp: 1,
        sleep_count: 2,
        speed: 3,
        creature_id: 4,
        pos: Coord_t { y: 5, x: 6 },
        distance_from_player: 7,
        lit: true,
        stunned_amount: 8,
        confused_amount: 9,
    };
    let _ = (
        m.hp,
        m.sleep_count,
        m.speed,
        m.creature_id,
        m.pos,
        m.distance_from_player,
        m.lit,
        m.stunned_amount,
        m.confused_amount,
    );
    assert_eq!(align_of::<Monster>(), align_of::<Coord_t>());
}

#[test]
fn monster_attack_t_layout() {
    let a = MonsterAttack {
        type_id: 1,
        description_id: 2,
        dice: Dice { dice: 2, sides: 4 },
    };
    let _ = (a.type_id, a.description_id, a.dice);
    assert_eq!(
        size_of::<MonsterAttack>(),
        size_of::<u8>() * 2 + size_of::<Dice>()
    );
}

#[test]
fn race_t_layout() {
    let r = Race {
        name: "Human",
        str_adjustment: 0,
        int_adjustment: 0,
        wis_adjustment: 0,
        dex_adjustment: 0,
        con_adjustment: 0,
        chr_adjustment: 0,
        base_age: 0,
        max_age: 0,
        male_height_base: 0,
        male_height_mod: 0,
        male_weight_base: 0,
        male_weight_mod: 0,
        female_height_base: 0,
        female_height_mod: 0,
        female_weight_base: 0,
        female_weight_mod: 0,
        disarm_chance_base: 0,
        search_chance_base: 0,
        stealth: 0,
        fos: 0,
        base_to_hit: 0,
        base_to_hit_bows: 0,
        saving_throw_base: 0,
        hit_points_base: 0,
        infra_vision: 0,
        exp_factor_base: 0,
        classes_bit_field: 0,
    };
    let _ = r.name;
    assert!(size_of::<Race>() > 20);
}

#[test]
fn class_t_layout() {
    let c = Class {
        title: "Warrior",
        hit_points: 0,
        disarm_traps: 0,
        searching: 0,
        stealth: 0,
        fos: 0,
        base_to_hit: 0,
        base_to_hit_with_bows: 0,
        saving_throw: 0,
        strength: 0,
        intelligence: 0,
        wisdom: 0,
        dexterity: 0,
        constitution: 0,
        charisma: 0,
        class_to_use_mage_spells: 0,
        experience_factor: 0,
        min_level_for_spell_casting: 0,
    };
    let _ = c.title;
    assert!(size_of::<Class>() > 10);
}

#[test]
fn background_t_layout() {
    let b = Background {
        info: "test",
        roll: 1,
        chart: 2,
        next: 3,
        bonus: 4,
    };
    let _ = (b.info, b.roll, b.chart, b.next, b.bonus);
}

#[test]
fn spell_t_layout() {
    let s = Spell {
        level_required: 1,
        mana_required: 2,
        failure_chance: 3,
        exp_gain_for_learning: 4,
    };
    let _ = (
        s.level_required,
        s.mana_required,
        s.failure_chance,
        s.exp_gain_for_learning,
    );
    assert_eq!(size_of::<Spell>(), 4);
}

#[test]
fn owner_t_layout() {
    let o = Owner {
        name: "Bob",
        max_cost: 100,
        max_inflate: 1,
        min_inflate: 2,
        haggles_per: 3,
        race: 4,
        max_insults: 5,
    };
    let _ = (
        o.name,
        o.max_cost,
        o.max_inflate,
        o.min_inflate,
        o.haggles_per,
        o.race,
        o.max_insults,
    );
}

#[test]
fn inventory_record_t_layout() {
    let r = InventoryRecord {
        cost: 100,
        item: Inventory::default(),
    };
    let _ = (r.cost, r.item);
}

#[test]
fn store_t_layout() {
    let s = Store::default();
    let _ = (
        s.turns_left_before_closing,
        s.insults_counter,
        s.owner_id,
        s.unique_items_counter,
        s.good_purchases,
        s.bad_purchases,
        s.inventory.len(),
    );
    assert_eq!(s.inventory.len(), STORE_MAX_DISCRETE_ITEMS as usize);
}

#[test]
fn recall_t_layout() {
    let r = Recall {
        movement: 0,
        spells: 0,
        kills: 0,
        deaths: 0,
        defenses: 0,
        wake: 0,
        ignore: 0,
        attacks: [0; MON_MAX_ATTACKS as usize],
    };
    let _ = (
        r.movement, r.spells, r.kills, r.deaths, r.defenses, r.wake, r.ignore, r.attacks,
    );
    assert_eq!(r.attacks.len(), MON_MAX_ATTACKS as usize);
}

#[test]
fn inventory_t_layout() {
    let i = Inventory::default();
    let _ = (
        i.id,
        i.special_name_id,
        i.inscription,
        i.flags,
        i.category_id,
        i.sprite,
        i.misc_use,
        i.cost,
        i.sub_category_id,
        i.items_count,
        i.weight,
        i.to_hit,
        i.to_damage,
        i.ac,
        i.to_ac,
        i.damage,
        i.depth_first_found,
        i.identification,
    );
    assert_eq!(i.inscription.len(), INSCRIP_SIZE as usize);
    assert_eq!(
        size_of::<[c_char; INSCRIP_SIZE as usize]>(),
        INSCRIP_SIZE as usize
    );
}

#[test]
fn dungeon_object_t_layout() {
    let o = DungeonObject::default();
    let _ = (
        o.name,
        o.flags,
        o.category_id,
        o.sprite,
        o.misc_use,
        o.cost,
        o.sub_category_id,
        o.items_count,
        o.weight,
        o.to_hit,
        o.to_damage,
        o.ac,
        o.to_ac,
        o.damage,
        o.depth_first_found,
    );
}

#[test]
fn tile_t_layout() {
    let t = Tile::default();
    let _ = (
        t.creature_id,
        t.treasure_id,
        t.feature_id,
        t.perma_lit_room,
        t.field_mark,
        t.permanent_light,
        t.temporary_light,
    );
}

#[test]
fn panel_t_layout() {
    let p = Panel::default();
    let _ = (
        p.row, p.col, p.top, p.bottom, p.left, p.right, p.col_prt, p.row_prt, p.max_rows,
        p.max_cols,
    );
}

#[test]
fn player_t_layout() {
    let p = Player::default();
    let _ = (
        p.misc.name,
        p.misc.gender,
        p.misc.date_of_birth,
        p.misc.au,
        p.misc.max_exp,
        p.misc.exp,
        p.misc.exp_fraction,
        p.misc.age,
        p.misc.height,
        p.misc.weight,
        p.misc.level,
        p.misc.max_dungeon_depth,
        p.misc.chance_in_search,
        p.misc.fos,
        p.misc.bth,
        p.misc.bth_with_bows,
        p.misc.mana,
        p.misc.max_hp,
        p.misc.plusses_to_hit,
        p.misc.plusses_to_damage,
        p.misc.ac,
        p.misc.magical_ac,
        p.misc.display_to_hit,
        p.misc.display_to_damage,
        p.misc.display_ac,
        p.misc.display_to_ac,
        p.misc.disarm,
        p.misc.saving_throw,
        p.misc.social_class,
        p.misc.stealth_factor,
        p.misc.class_id,
        p.misc.race_id,
        p.misc.hit_die,
        p.misc.experience_factor,
        p.misc.current_mana,
        p.misc.current_mana_fraction,
        p.misc.current_hp,
        p.misc.current_hp_fraction,
        p.misc.history,
        p.stats.max,
        p.stats.current,
        p.stats.modified,
        p.stats.used,
        p.flags.status,
        p.flags.rest,
        p.flags.blind,
        p.flags.paralysis,
        p.flags.confused,
        p.flags.food,
        p.flags.food_digested,
        p.flags.protection,
        p.flags.speed,
        p.flags.fast,
        p.flags.slow,
        p.flags.afraid,
        p.flags.poisoned,
        p.flags.image,
        p.flags.protect_evil,
        p.flags.invulnerability,
        p.flags.heroism,
        p.flags.super_heroism,
        p.flags.blessed,
        p.flags.heat_resistance,
        p.flags.cold_resistance,
        p.flags.detect_invisible,
        p.flags.word_of_recall,
        p.flags.see_infra,
        p.flags.timed_infra,
        p.flags.see_invisible,
        p.flags.teleport,
        p.flags.free_action,
        p.flags.slow_digest,
        p.flags.aggravate,
        p.flags.resistant_to_fire,
        p.flags.resistant_to_cold,
        p.flags.resistant_to_acid,
        p.flags.regenerate_hp,
        p.flags.resistant_to_light,
        p.flags.free_fall,
        p.flags.sustain_str,
        p.flags.sustain_int,
        p.flags.sustain_wis,
        p.flags.sustain_con,
        p.flags.sustain_dex,
        p.flags.sustain_chr,
        p.flags.confuse_monster,
        p.flags.new_spells_to_learn,
        p.flags.spells_learnt,
        p.flags.spells_worked,
        p.flags.spells_forgotten,
        p.flags.spells_learned_order,
        p.pos,
        p.prev_dir,
        p.base_hp_levels.len(),
        p.base_exp_levels.len(),
        p.running_tracker,
        p.temporary_light_only,
        p.max_score,
        p.pack.unique_items,
        p.pack.weight,
        p.pack.heaviness,
        p.inventory.len(),
        p.equipment_count,
        p.weapon_is_heavy,
        p.carrying_light,
    );
    assert_eq!(p.misc.name.len(), PLAYER_NAME_SIZE as usize);
    assert_eq!(p.misc.history.len(), 4);
    assert_eq!(p.misc.history[0].len(), 60);
    assert_eq!(p.base_hp_levels.len(), PLAYER_MAX_LEVEL as usize);
    assert_eq!(p.base_exp_levels.len(), PLAYER_MAX_LEVEL as usize);
    assert_eq!(p.inventory.len(), PLAYER_INVENTORY_SIZE as usize);
    assert_eq!(p.prev_dir, b' ');
}

#[test]
fn dungeon_t_layout() {
    let d = Dungeon::default();
    let _ = (
        d.height,
        d.width,
        d.panel,
        d.game_turn,
        d.current_level,
        d.generate_new_level,
        d.floor.len(),
    );
    assert_eq!(d.game_turn, -1);
    assert!(d.generate_new_level);
    assert_eq!(d.floor.len(), MAX_HEIGHT as usize);
    assert_eq!(d.floor[0].len(), MAX_WIDTH as usize);
}

// --------------------------------------------------------------------------
// 3. Enum discriminants
// --------------------------------------------------------------------------
#[test]
fn special_name_ids_discriminants() {
    assert_eq!(SpecialNameIds::SN_NULL as u8, 0);
    assert_eq!(SpecialNameIds::SN_R as u8, 1);
    assert_eq!(SpecialNameIds::SN_HA as u8, 6);
    assert_eq!(SpecialNameIds::SN_FREE_ACTION as u8, 14);
    assert_eq!(SpecialNameIds::SN_SLAY_ANIMAL as u8, 55);
    assert_eq!(SpecialNameIds::SN_ARRAY_SIZE as u8, 56);
}

#[test]
fn player_equipment_discriminants() {
    assert_eq!(PlayerEquipment::Wield as u8, 22);
    assert_eq!(PlayerEquipment::Head as u8, 23);
    assert_eq!(PlayerEquipment::Neck as u8, 24);
    assert_eq!(PlayerEquipment::Body as u8, 25);
    assert_eq!(PlayerEquipment::Arm as u8, 26);
    assert_eq!(PlayerEquipment::Hands as u8, 27);
    assert_eq!(PlayerEquipment::Right as u8, 28);
    assert_eq!(PlayerEquipment::Left as u8, 29);
    assert_eq!(PlayerEquipment::Feet as u8, 30);
    assert_eq!(PlayerEquipment::Outer as u8, 31);
    assert_eq!(PlayerEquipment::Light as u8, 32);
    assert_eq!(PlayerEquipment::Auxiliary as u8, 33);
}

#[test]
fn player_class_level_adj_discriminants() {
    assert_eq!(PlayerClassLevelAdj::BTH as u8, 0);
    assert_eq!(PlayerClassLevelAdj::BTHB as u8, 1);
    assert_eq!(PlayerClassLevelAdj::DEVICE as u8, 2);
    assert_eq!(PlayerClassLevelAdj::DISARM as u8, 3);
    assert_eq!(PlayerClassLevelAdj::SAVE as u8, 4);
}

#[test]
fn player_attr_discriminants() {
    assert_eq!(PlayerAttr::A_STR as u8, 0);
    assert_eq!(PlayerAttr::A_INT as u8, 1);
    assert_eq!(PlayerAttr::A_WIS as u8, 2);
    assert_eq!(PlayerAttr::A_DEX as u8, 3);
    assert_eq!(PlayerAttr::A_CON as u8, 4);
    assert_eq!(PlayerAttr::A_CHR as u8, 5);
}

#[test]
fn magic_spell_flags_discriminants() {
    assert_eq!(MagicSpellFlags::MagicMissile as u8, 0);
    assert_eq!(MagicSpellFlags::Lightning as u8, 1);
    assert_eq!(MagicSpellFlags::PoisonGas as u8, 2);
    assert_eq!(MagicSpellFlags::Acid as u8, 3);
    assert_eq!(MagicSpellFlags::Frost as u8, 4);
    assert_eq!(MagicSpellFlags::Fire as u8, 5);
    assert_eq!(MagicSpellFlags::HolyOrb as u8, 6);
}

#[test]
fn screen_enum_discriminants() {
    assert_eq!(Screen::Blank as u8, 0);
    assert_eq!(Screen::Equipment as u8, 1);
    assert_eq!(Screen::Inventory as u8, 2);
    assert_eq!(Screen::Wear as u8, 3);
    assert_eq!(Screen::Help as u8, 4);
    assert_eq!(Screen::Wrong as u8, 5);
}

// --------------------------------------------------------------------------
// 4. TV_* constants
// --------------------------------------------------------------------------
#[test]
fn tv_constants_match_expected() {
    assert_eq!(TV_NEVER, -1);
    assert_eq!(TV_NOTHING, 0);
    assert_eq!(TV_MISC, 1);
    assert_eq!(TV_CHEST, 2);
    assert_eq!(TV_MIN_WEAR, 10);
    assert_eq!(TV_MIN_ENCHANT, 10);
    assert_eq!(TV_SLING_AMMO, 10);
    assert_eq!(TV_BOLT, 11);
    assert_eq!(TV_ARROW, 12);
    assert_eq!(TV_SPIKE, 13);
    assert_eq!(TV_LIGHT, 15);
    assert_eq!(TV_BOW, 20);
    assert_eq!(TV_HAFTED, 21);
    assert_eq!(TV_POLEARM, 22);
    assert_eq!(TV_SWORD, 23);
    assert_eq!(TV_DIGGING, 25);
    assert_eq!(TV_BOOTS, 30);
    assert_eq!(TV_GLOVES, 31);
    assert_eq!(TV_CLOAK, 32);
    assert_eq!(TV_HELM, 33);
    assert_eq!(TV_SHIELD, 34);
    assert_eq!(TV_HARD_ARMOR, 35);
    assert_eq!(TV_SOFT_ARMOR, 36);
    assert_eq!(TV_MAX_ENCHANT, 39);
    assert_eq!(TV_AMULET, 40);
    assert_eq!(TV_RING, 45);
    assert_eq!(TV_MAX_WEAR, 50);
    assert_eq!(TV_STAFF, 55);
    assert_eq!(TV_WAND, 65);
    assert_eq!(TV_SCROLL1, 70);
    assert_eq!(TV_SCROLL2, 71);
    assert_eq!(TV_POTION1, 75);
    assert_eq!(TV_POTION2, 76);
    assert_eq!(TV_FLASK, 77);
    assert_eq!(TV_FOOD, 80);
    assert_eq!(TV_MAGIC_BOOK, 90);
    assert_eq!(TV_PRAYER_BOOK, 91);
    assert_eq!(TV_MAX_OBJECT, 99);
    assert_eq!(TV_GOLD, 100);
    assert_eq!(TV_MAX_PICK_UP, 100);
    assert_eq!(TV_INVIS_TRAP, 101);
    assert_eq!(TV_MIN_VISIBLE, 102);
    assert_eq!(TV_VIS_TRAP, 102);
    assert_eq!(TV_RUBBLE, 103);
    assert_eq!(TV_MIN_DOORS, 104);
    assert_eq!(TV_OPEN_DOOR, 104);
    assert_eq!(TV_CLOSED_DOOR, 105);
    assert_eq!(TV_UP_STAIR, 107);
    assert_eq!(TV_DOWN_STAIR, 108);
    assert_eq!(TV_SECRET_DOOR, 109);
    assert_eq!(TV_STORE_DOOR, 110);
    assert_eq!(TV_MAX_VISIBLE, 110);
}

// --------------------------------------------------------------------------
// 5. magic_spells and spell_names dimensions
// --------------------------------------------------------------------------
#[test]
fn magic_spells_array_dimensions() {
    const CLASSES_MINUS_ONE: usize = PLAYER_MAX_CLASSES as usize - 1;
    let spells = &*MAGIC_SPELLS;
    assert_eq!(spells.len(), CLASSES_MINUS_ONE);
    assert_eq!(spells[0].len(), 31);
    assert_eq!(SPELL_NAMES.len(), 62);
}

// --------------------------------------------------------------------------
// 6. Static-table & container declarations
// --------------------------------------------------------------------------
#[test]
fn static_table_shapes() {
    assert_eq!(CREATURES_LIST.len(), MON_MAX_CREATURES as usize);
    assert_eq!(MONSTER_ATTACKS.len(), MON_ATTACK_TYPES as usize);
    assert_eq!(GAME_OBJECTS.len(), MAX_OBJECTS_IN_GAME as usize);
    assert_eq!(SPECIAL_ITEM_NAMES.len(), SN_ARRAY_SIZE as usize);
    assert_eq!(CLASS_RANK_TITLES.len(), PLAYER_MAX_CLASSES as usize);
    assert_eq!(CLASS_RANK_TITLES[0].len(), PLAYER_MAX_LEVEL as usize);
    assert_eq!(CHARACTER_RACES.len(), PLAYER_MAX_RACES as usize);
    assert_eq!(CHARACTER_BACKGROUNDS.len(), PLAYER_MAX_BACKGROUNDS as usize);
    assert_eq!(CLASSES.len(), PLAYER_MAX_CLASSES as usize);
    assert_eq!(CLASS_LEVEL_ADJ.len(), PLAYER_MAX_CLASSES as usize);
    assert_eq!(CLASS_LEVEL_ADJ[0].len(), CLASS_MAX_LEVEL_ADJUST as usize);
    assert_eq!(CLASS_BASE_PROVISIONS.len(), PLAYER_MAX_CLASSES as usize);
    assert_eq!(CLASS_BASE_PROVISIONS[0].len(), 5);
    assert_eq!(BLOWS_TABLE.len(), 7);
    assert_eq!(BLOWS_TABLE[0].len(), 6);
    assert_eq!(RACE_GOLD_ADJUSTMENTS.len(), PLAYER_MAX_RACES as usize);
    assert_eq!(RACE_GOLD_ADJUSTMENTS[0].len(), PLAYER_MAX_RACES as usize);
    assert_eq!(STORE_OWNERS.len(), MAX_OWNERS as usize);
    assert_eq!(STORE_CHOICES.len(), MAX_STORES as usize);
    assert_eq!(STORE_CHOICES[0].len(), STORE_MAX_ITEM_TYPES as usize);
    assert_eq!(STORES.len(), MAX_STORES as usize);

    assert_eq!(RECALL_DESCRIPTION_ATTACK_TYPE.len(), 25);
    assert_eq!(RECALL_DESCRIPTION_ATTACK_METHOD.len(), 20);
    assert_eq!(RECALL_DESCRIPTION_HOW_MUCH.len(), 8);
    assert_eq!(RECALL_DESCRIPTION_MOVE.len(), 6);
    assert_eq!(RECALL_DESCRIPTION_SPELL.len(), 15);
    assert_eq!(RECALL_DESCRIPTION_BREATH.len(), 5);
    assert_eq!(RECALL_DESCRIPTION_WEAKNESS.len(), 6);

    assert_eq!(COLORS.len(), MAX_COLORS as usize);
    assert_eq!(MUSHROOMS.len(), MAX_MUSHROOMS as usize);
    assert_eq!(WOODS.len(), MAX_WOODS as usize);
    assert_eq!(METALS.len(), MAX_METALS as usize);
    assert_eq!(ROCKS.len(), MAX_ROCKS as usize);
    assert_eq!(AMULETS.len(), MAX_AMULETS as usize);
    assert_eq!(SYLLABLES.len(), MAX_SYLLABLES as usize);

    assert_eq!(SPEECH_SALE_ACCEPTED.len(), SPEECH_SALE_ACCEPTED_N as usize);
    assert_eq!(
        SPEECH_SELLING_HAGGLE_FINAL.len(),
        SPEECH_SELLING_HAGGLE_FINAL_N as usize
    );
    assert_eq!(
        SPEECH_SELLING_HAGGLE.len(),
        SPEECH_SELLING_HAGGLE_N as usize
    );
    assert_eq!(
        SPEECH_BUYING_HAGGLE_FINAL.len(),
        SPEECH_BUYING_HAGGLE_FINAL_N as usize
    );
    assert_eq!(SPEECH_BUYING_HAGGLE.len(), SPEECH_BUYING_HAGGLE_N as usize);
    assert_eq!(
        SPEECH_INSULTED_HAGGLING_DONE.len(),
        SPEECH_INSULTED_HAGGLING_DONE_N as usize
    );
    assert_eq!(
        SPEECH_GET_OUT_OF_MY_STORE.len(),
        SPEECH_GET_OUT_OF_MY_STORE_N as usize
    );
    assert_eq!(
        SPEECH_HAGGLING_TRY_AGAIN.len(),
        SPEECH_HAGGLING_TRY_AGAIN_N as usize
    );
    assert_eq!(SPEECH_SORRY.len(), SPEECH_SORRY_N as usize);
}

#[test]
fn class_rank_title_is_static_str() {
    let _: ClassRankTitle = "";
}
