//! Phase 4.5.4.1 — identification state, flavor init & identify logic parity.
#![allow(clippy::int_plus_one)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use umoria::config::identification::{
    ID_DAMD, ID_EMPTY, ID_KNOWN2, ID_MAGIK, ID_STORE_BOUGHT, OD_KNOWN1, OD_TRIED,
};
use umoria::config::treasure::flags::TR_CURSED;
use umoria::game::{reset_for_new_game, with_state, with_state_mut, State};
use umoria::identification::{
    identify_game_object, item_identification_clear_empty, item_identify,
    item_identify_as_store_bought, item_remove_magic_naming, item_set_as_identified,
    item_set_as_tried, item_set_colorless_as_identified, magic_initialize_item_names,
    object_description, object_position_offset, spell_item_identified,
    spell_item_identify_and_remove_random_inscription, spell_item_remove_identification,
    FlavorTables, SpecialNameIds, MAX_AMULETS, MAX_COLORS, MAX_METALS, MAX_MUSHROOMS, MAX_ROCKS,
    MAX_TITLES, MAX_WOODS,
};
use umoria::inventory::{inventory_item_single_stackable, Inventory, ITEM_SINGLE_STACK_MIN};
use umoria::rng::get_seed;
use umoria::treasure::{
    TV_AMULET, TV_FOOD, TV_HARD_ARMOR, TV_POTION1, TV_POTION2, TV_RING, TV_SCROLL1, TV_SCROLL2,
    TV_STAFF, TV_SWORD, TV_WAND,
};
use umoria::ui_io::{test_clear_getch_keys, test_push_getch_keys, test_set_ncurses_stub};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

struct MagicInitGolden {
    main_seed_before: u32,
    main_seed_after: u32,
    sections: HashMap<String, Vec<String>>,
}

fn parse_magic_init_golden(path: &Path) -> MagicInitGolden {
    let content = fs::read_to_string(path).unwrap();
    let mut main_seed_before = 0u32;
    let mut main_seed_after = 0u32;
    let mut sections: HashMap<String, Vec<String>> = HashMap::new();
    let mut current: Option<String> = None;

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("main_seed_before=") {
            main_seed_before = rest.parse().unwrap();
        } else if let Some(rest) = line.strip_prefix("main_seed_after=") {
            main_seed_after = rest.parse().unwrap();
        } else if line.starts_with('[') && line.ends_with(']') {
            current = Some(line[1..line.len() - 1].to_string());
            sections.entry(current.clone().unwrap()).or_default();
        } else if let Some(section) = &current {
            if !line.is_empty() || sections[section].is_empty() {
                sections.get_mut(section).unwrap().push(line.to_string());
            }
        }
    }

    MagicInitGolden {
        main_seed_before,
        main_seed_after,
        sections,
    }
}

fn setup_main_rng(seed: u32) {
    reset_for_new_game(Some(seed));
    with_state_mut(|s| {
        s.flavor = FlavorTables::from_static_defaults();
        s.objects_identified.fill(0);
    });
}

type FlavorSnapshot = (
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
);

fn flavor_snapshot(state: &State) -> FlavorSnapshot {
    let colors: Vec<_> = (0..MAX_COLORS as usize)
        .map(|i| state.flavor.color_name(i).to_string())
        .collect();
    let woods: Vec<_> = (0..MAX_WOODS as usize)
        .map(|i| state.flavor.wood_name(i).to_string())
        .collect();
    let metals: Vec<_> = (0..MAX_METALS as usize)
        .map(|i| state.flavor.metal_name(i).to_string())
        .collect();
    let rocks: Vec<_> = (0..MAX_ROCKS as usize)
        .map(|i| state.flavor.rock_name(i).to_string())
        .collect();
    let amulets: Vec<_> = (0..MAX_AMULETS as usize)
        .map(|i| state.flavor.amulet_name(i).to_string())
        .collect();
    let mushrooms: Vec<_> = (0..MAX_MUSHROOMS as usize)
        .map(|i| state.flavor.mushroom_name(i).to_string())
        .collect();
    let titles: Vec<_> = (0..MAX_TITLES as usize)
        .map(|i| state.flavor.magic_item_title(i).to_string())
        .collect();
    (colors, woods, metals, rocks, amulets, mushrooms, titles)
}

fn ident_flag(category_id: u8, sub_category_id: u8) -> u8 {
    with_state(|state| {
        let offset = object_position_offset(category_id, sub_category_id);
        assert!(offset >= 0);
        let id = (offset as usize) << 6;
        let id = id + usize::from(sub_category_id & (ITEM_SINGLE_STACK_MIN - 1));
        state.objects_identified[id]
    })
}

fn make_item(category_id: u8, sub_category_id: u8, items_count: u8) -> Inventory {
    Inventory {
        category_id,
        sub_category_id,
        items_count,
        ..Inventory::default()
    }
}

#[test]
fn magic_initialize_item_names_matches_cpp_goldens() {
    const MAIN_SEED: u32 = 12345;
    const MAGIC_SEEDS: [u32; 4] = [1, 42, 12345, 1_700_000_000];

    for magic_seed in MAGIC_SEEDS {
        let path = repo_root().join(format!(
            "tests/golden/identification/magic_init_seed{magic_seed}.txt"
        ));
        let golden = parse_magic_init_golden(&path);

        setup_main_rng(MAIN_SEED);
        assert_eq!(get_seed(), golden.main_seed_before);

        with_state_mut(|s| s.game.magic_seed = magic_seed);
        magic_initialize_item_names();

        assert_eq!(get_seed(), golden.main_seed_after);

        let snap = with_state(flavor_snapshot);
        assert_eq!(&snap.0, golden.sections.get("colors").unwrap());
        assert_eq!(&snap.1, golden.sections.get("woods").unwrap());
        assert_eq!(&snap.2, golden.sections.get("metals").unwrap());
        assert_eq!(&snap.3, golden.sections.get("rocks").unwrap());
        assert_eq!(&snap.4, golden.sections.get("amulets").unwrap());
        assert_eq!(&snap.5, golden.sections.get("mushrooms").unwrap());
        assert_eq!(&snap.6, golden.sections.get("magic_item_titles").unwrap());
    }
}

#[test]
fn magic_initialize_item_names_restore_matches_cpp_seed_set() {
    setup_main_rng(999);
    let before = get_seed();
    with_state_mut(|s| s.game.magic_seed = 42);
    magic_initialize_item_names();
    // C++ seedResetToOldSeed() calls setRandomSeed(old_seed), which re-normalizes:
    // restored = (old_seed % (RNG_M-1)) + 1 — one past the saved stream position.
    assert_eq!(get_seed(), before.wrapping_add(1));
}

#[test]
fn object_position_offset_exhaustive() {
    let cases: &[(u8, u8, i16)] = &[
        (TV_AMULET, 64, 0),
        (TV_RING, 70, 1),
        (TV_STAFF, 64, 2),
        (TV_WAND, 64, 3),
        (TV_SCROLL1, 64, 4),
        (TV_SCROLL2, 65, 4),
        (TV_POTION1, 64, 5),
        (TV_POTION2, 80, 5),
        (TV_FOOD, 64, 6),
        (TV_FOOD, 64 + MAX_MUSHROOMS - 1, 6),
        (TV_FOOD, 64 + MAX_MUSHROOMS, -1),
        (TV_SWORD, 64, -1),
        (TV_HARD_ARMOR, 64, -1),
    ];
    for &(cat, sub, expected) in cases {
        assert_eq!(
            object_position_offset(cat, sub),
            expected,
            "tval {cat} sub {sub}"
        );
    }
}

#[test]
fn item_set_as_tried_sets_od_tried_bit() {
    reset_for_new_game(None);
    let item = make_item(TV_POTION1, 64 + 3, 1);
    assert_eq!(ident_flag(TV_POTION1, 64 + 3), 0);
    item_set_as_tried(item);
    assert_eq!(ident_flag(TV_POTION1, 64 + 3), OD_TRIED);
}

#[test]
fn item_set_as_identified_sets_known_clears_tried() {
    reset_for_new_game(None);
    let item = make_item(TV_WAND, 64 + 5, 1);
    item_set_as_tried(item);
    assert_eq!(ident_flag(TV_WAND, 64 + 5), OD_TRIED);
    item_set_as_identified(TV_WAND, 64 + 5);
    assert_eq!(ident_flag(TV_WAND, 64 + 5), OD_KNOWN1);
}

#[test]
fn item_set_colorless_as_identified_for_non_flavor_types() {
    reset_for_new_game(None);
    assert!(item_set_colorless_as_identified(TV_SWORD, 64, 0));
}

#[test]
fn item_set_colorless_as_identified_for_store_bought() {
    reset_for_new_game(None);
    assert!(item_set_colorless_as_identified(
        TV_POTION1,
        64 + 1,
        ID_STORE_BOUGHT
    ));
}

#[test]
fn item_set_colorless_as_identified_reflects_known_state() {
    reset_for_new_game(None);
    assert!(!item_set_colorless_as_identified(TV_SCROLL1, 64 + 2, 0));
    item_set_as_identified(TV_SCROLL1, 64 + 2);
    assert!(item_set_colorless_as_identified(TV_SCROLL1, 64 + 2, 0));
}

#[test]
fn item_identify_marks_potion_known() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.pack.unique_items = 1;
        s.py.inventory[0] = make_item(TV_POTION1, 64 + 4, 2);
    });
    let mut slot = 0i32;
    item_identify(&mut slot);
    assert_eq!(slot, 0);
    assert_eq!(ident_flag(TV_POTION1, 64 + 4), OD_KNOWN1);
}

#[test]
fn item_identify_merges_stackable_duplicates() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.pack.unique_items = 2;
        s.py.inventory[0] = make_item(TV_SCROLL1, 64 + 1, 3);
        s.py.inventory[1] = make_item(TV_SCROLL1, 64 + 1, 2);
    });
    let mut slot = 1i32;
    item_identify(&mut slot);
    assert_eq!(slot, 0);
    with_state(|s| {
        assert_eq!(s.py.pack.unique_items, 1);
        assert_eq!(s.py.inventory[0].items_count, 5);
    });
}

#[test]
fn item_identify_skips_merge_for_non_stackable() {
    reset_for_new_game(None);
    let item = make_item(TV_SWORD, 63, 1);
    assert!(!inventory_item_single_stackable(item));
    with_state_mut(|s| {
        s.py.pack.unique_items = 2;
        s.py.inventory[0] = item;
        s.py.inventory[1] = item;
    });
    let mut slot = 1i32;
    item_identify(&mut slot);
    with_state(|s| assert_eq!(s.py.pack.unique_items, 2));
}

#[test]
fn item_identify_appends_damned_to_cursed_item() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.pack.unique_items = 1;
        let mut item = make_item(TV_RING, 64 + 2, 1);
        item.flags |= TR_CURSED;
        s.py.inventory[0] = item;
    });
    let mut slot = 0i32;
    item_identify(&mut slot);
    with_state(|s| {
        assert_ne!(s.py.inventory[0].identification & ID_DAMD, 0);
    });
}

#[test]
fn spell_item_identified_and_remove_identification() {
    let mut item = make_item(TV_STAFF, 64, 1);
    assert!(!spell_item_identified(item));
    spell_item_identify_and_remove_random_inscription(&mut item);
    assert!(spell_item_identified(item));
    spell_item_remove_identification(&mut item);
    assert!(!spell_item_identified(item));
}

#[test]
fn unsample_clears_magik_empty_and_tried() {
    reset_for_new_game(None);
    let mut item = make_item(TV_POTION1, 64 + 6, 1);
    item.identification = ID_MAGIK | ID_EMPTY;
    item_set_as_tried(item);
    assert_eq!(ident_flag(TV_POTION1, 64 + 6), OD_TRIED);

    spell_item_identify_and_remove_random_inscription(&mut item);
    assert_eq!(item.identification & (ID_MAGIK | ID_EMPTY), 0);
    assert_ne!(item.identification & ID_KNOWN2, 0);
    assert_eq!(ident_flag(TV_POTION1, 64 + 6), 0);
}

#[test]
fn test_item_identification_clear_empty() {
    let mut item = make_item(TV_WAND, 64, 1);
    item.identification = ID_EMPTY;
    item_identification_clear_empty(&mut item);
    assert_eq!(item.identification & ID_EMPTY, 0);
}

#[test]
fn test_item_identify_as_store_bought() {
    let mut item = make_item(TV_SCROLL1, 64 + 3, 1);
    item.identification = ID_MAGIK;
    item_identify_as_store_bought(&mut item);
    assert_ne!(item.identification & ID_STORE_BOUGHT, 0);
    assert!(spell_item_identified(item));
    assert_eq!(item.identification & ID_MAGIK, 0);
}

#[test]
fn item_remove_magic_naming_clears_special_name() {
    let mut item = make_item(TV_AMULET, 64, 1);
    item.special_name_id = SpecialNameIds::SN_MAGI as u8;
    item_remove_magic_naming(&mut item);
    assert_eq!(item.special_name_id, SpecialNameIds::SN_NULL as u8);
}

#[test]
fn object_description_known_tile_chars() {
    reset_for_new_game(None);
    assert_eq!(object_description(b'!'), "! - A potion.");
    assert_eq!(object_description(b'?'), "? - A scroll.");
    assert_eq!(object_description(b'|'), "| - A sword or dagger.");
    assert_eq!(object_description(b'Z'), "Not Used.");
}

#[test]
fn object_description_player_tile_uses_name() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.misc.name[..5].copy_from_slice(b"Gand\0");
    });
    assert_eq!(object_description(b'@'), "Gand");
}

#[test]
fn object_description_highlight_seams_option() {
    reset_for_new_game(None);
    with_state_mut(|s| s.options.highlight_seams = false);
    assert_eq!(object_description(b'%'), "% - Not used.");
    with_state_mut(|s| s.options.highlight_seams = true);
    assert_eq!(object_description(b'%'), "% - A magma or quartz vein.");
}

#[test]
fn identify_game_object_prints_description() {
    test_set_ncurses_stub(true);
    test_clear_getch_keys();
    test_push_getch_keys(&[b'?' as i32]);
    reset_for_new_game(None);

    identify_game_object();

    test_set_ncurses_stub(false);
    test_clear_getch_keys();
}

#[test]
fn object_ident_index_uses_u8_subcategory_mask() {
    reset_for_new_game(None);
    item_set_as_identified(TV_POTION1, 192);
    assert_eq!(ident_flag(TV_POTION1, 192), OD_KNOWN1);
}

#[test]
fn magic_item_title_truncates_at_nine_chars() {
    setup_main_rng(777);
    with_state_mut(|s| s.game.magic_seed = 1);
    magic_initialize_item_names();
    with_state(|s| {
        for title in &s.flavor.magic_item_titles {
            let end = title.iter().position(|&b| b == 0).unwrap_or(10);
            assert!(end <= 9, "title longer than C++ vtype_t allows");
        }
    });
}

#[test]
fn flavor_tables_reset_on_new_game() {
    setup_main_rng(100);
    with_state_mut(|s| s.game.magic_seed = 42);
    magic_initialize_item_names();
    reset_for_new_game(Some(100));
    with_state(|s| {
        assert_eq!(s.flavor.color_order[0], 0);
        assert_eq!(s.flavor.magic_item_titles[0], [0; 10]);
    });
}

#[test]
fn item_identify_early_return_when_already_known() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.pack.unique_items = 2;
        s.py.inventory[0] = make_item(TV_POTION1, 64 + 1, 1);
        s.py.inventory[1] = make_item(TV_POTION1, 64 + 1, 1);
    });
    item_set_as_identified(TV_POTION1, 64 + 1);
    let mut slot = 1i32;
    item_identify(&mut slot);
    with_state(|s| assert_eq!(s.py.pack.unique_items, 2));
}
