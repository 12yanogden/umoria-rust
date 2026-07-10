//! Phase 4.5.4.2 — itemDescription, inscriptions & description helpers parity.
#![allow(clippy::int_plus_one)]

use std::fs;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};

use umoria::config::identification::{
    ID_EMPTY, ID_KNOWN2, ID_MAGIK, ID_STORE_BOUGHT, OD_TRIED,
};
use umoria::config::treasure::flags::{TR_STR};
use umoria::data_creatures::CREATURES_LIST;
use umoria::game::{reset_for_new_game, with_state, with_state_mut};
use umoria::identification::{
    bow_damage_value, item_append_to_inscription, item_charges_remaining_description,
    item_description, item_inscribe, item_replace_inscription, item_set_as_identified,
    item_type_remaining_count_description, magic_initialize_item_names,
    object_blocked_by_monster, spell_item_identified, FlavorTables, SpecialNameIds,
};
use umoria::inventory::{inventory_item_copy_to, Inventory, INSCRIP_SIZE};
use umoria::monster::Monster;
use umoria::treasure::{TV_FOOD, TV_POTION1};
use umoria::types::MORIA_OBJ_DESC_SIZE_LEN;
use umoria::ui_io::test_set_ncurses_stub;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn setup_flavor_seed42() {
    reset_for_new_game(Some(42));
    with_state_mut(|s| {
        s.game.magic_seed = 42;
        s.flavor = FlavorTables::from_static_defaults();
        s.objects_identified.fill(0);
    });
    magic_initialize_item_names();
}

fn make_item(id: u16, sub_category_id: u8) -> Inventory {
    let mut item = Inventory::default();
    inventory_item_copy_to(id as i16, &mut item);
    item.sub_category_id = sub_category_id;
    item.items_count = 1;
    item
}

fn desc_string(item: Inventory, add_prefix: bool) -> String {
    let mut out = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
    item_description(&mut out, item, add_prefix);
    c_str_to_string(&out)
}

fn c_str_to_string(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

fn message_text(id: i16) -> String {
    with_state(|s| {
        let msg = &s.messages[id as usize];
        c_str_to_string(msg)
    })
}

fn load_cpp_golden() -> Vec<(String, bool, String)> {
    let path = repo_root().join("tests/golden/item_desc_capture.txt");
    let content = fs::read_to_string(path).expect("golden capture");
    content
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(4, '\t');
            let kind = parts.next()?;
            if kind != "DESC" {
                return None;
            }
            let name = parts.next()?.to_string();
            let add_prefix = parts.next()? == "1";
            let desc = parts.next()?.to_string();
            Some((name, add_prefix, desc))
        })
        .collect()
}

fn case_item(name: &str) -> Inventory {
    match name {
        "misc" | "misc_no_prefix" => make_item(332, 64),
        "arrow_2d6" => {
            let mut item = make_item(81, 64);
            item.damage.dice = 2;
            item.damage.sides = 6;
            item
        }
        "bow_x3" => {
            let mut item = make_item(75, 64);
            item.misc_use = 3;
            item
        }
        "sword_hit_dam" => {
            let mut item = make_item(34, 64);
            item.damage.dice = 3;
            item.damage.sides = 8;
            item.to_hit = 2;
            item.to_damage = 5;
            item.identification = ID_KNOWN2 | umoria::config::identification::ID_SHOW_HIT_DAM;
            item
        }
        "sword_cursed_str" => {
            let mut item = make_item(34, 64);
            item.damage.dice = 2;
            item.damage.sides = 6;
            item.misc_use = -2;
            item.flags = TR_STR;
            item.identification = ID_KNOWN2;
            item
        }
        "helm_ac_bracket" => {
            let mut item = make_item(98, 64);
            item.ac = 0;
            item.to_ac = 3;
            item.identification = ID_KNOWN2;
            item
        }
        "shield_ac5" => {
            let mut item = make_item(130, 64);
            item.ac = 5;
            item.identification = ID_KNOWN2;
            item
        }
        "light_turns" => {
            let mut item = make_item(87, 64);
            item.misc_use = 750;
            item
        }
        "potion_unknown" => make_item(243, 64),
        "potion_known" => make_item(243, 64),
        "scroll_unknown" => make_item(177, 64),
        "staff_charges" => {
            let mut item = make_item(293, 64);
            item.misc_use = 5;
            item.identification = ID_KNOWN2;
            item
        }
        "ring_plus2" => {
            let mut item = make_item(132, 64);
            item.misc_use = 2;
            item.identification = ID_KNOWN2;
            item
        }
        "amulet_plus1" => {
            let mut item = make_item(163, 64);
            item.misc_use = 1;
            item.identification = ID_KNOWN2;
            item
        }
        "mushroom_unknown" => {
            let mut item = make_item(243, 64);
            item.category_id = TV_FOOD;
            item.sub_category_id = 64;
            item
        }
        "mushroom_plural" => {
            let mut item = make_item(243, 64);
            item.category_id = TV_FOOD;
            item.sub_category_id = 64;
            item.items_count = 3;
            item
        }
        "gold" => make_item(408, 64),
        "store_door" => make_item(373, 64),
        "special_name" => {
            let mut item = make_item(34, 64);
            item.damage.dice = 2;
            item.damage.sides = 6;
            item.special_name_id = SpecialNameIds::SN_SLAYING as u8;
            item.identification = ID_KNOWN2;
            item
        }
        "inscription_flags" => {
            let mut item = make_item(293, 64);
            item.misc_use = 3;
            item.identification = ID_KNOWN2 | ID_MAGIK | ID_EMPTY;
            item.inscription[..3].copy_from_slice(&[b'a' as c_char, b'b' as c_char, b'c' as c_char]);
            item.inscription[3] = 0;
            item
        }
        "tried_potion" => make_item(243, 65),
        "store_bought_potion" => {
            let mut item = make_item(243, 64);
            item.identification = ID_STORE_BOUGHT;
            item
        }
        "digging_zplusses" => {
            let mut item = make_item(88, 64);
            item.damage.dice = 2;
            item.damage.sides = 8;
            item.misc_use = 3;
            item.identification = ID_KNOWN2;
            item
        }
        "no_more_potion" => {
            let mut item = make_item(243, 64);
            item.items_count = 0;
            item
        }
        "magic_book" => make_item(319, 64),
        "prayer_book" => make_item(324, 64),
        other => panic!("unknown golden case {other}"),
    }
}

#[test]
fn capture_sequence_wood0_matches_cpp() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.game.magic_seed = 42;
        s.flavor = FlavorTables::from_static_defaults();
    });
    magic_initialize_item_names();
    with_state_mut(|s| s.objects_identified.fill(0));
    magic_initialize_item_names();
    with_state(|s| assert_eq!(s.flavor.wood_name(0), "Ironwood"));
}

// ---------------------------------------------------------------------------
// 1. itemDescription golden capture (C++ reference output)
// ---------------------------------------------------------------------------
#[test]
fn item_description_golden_capture_matches_cpp() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.game.magic_seed = 42;
        s.flavor = FlavorTables::from_static_defaults();
        s.objects_identified.fill(0);
    });
    magic_initialize_item_names();

    for (name, add_prefix, expected) in load_cpp_golden() {
        match name.as_str() {
            "potion_known" => item_set_as_identified(TV_POTION1, 64),
            "scroll_unknown" => {
                with_state_mut(|s| s.objects_identified.fill(0));
                magic_initialize_item_names();
            }
            "tried_potion" => {
                with_state_mut(|s| {
                    let id = (5usize << 6) + 1;
                    s.objects_identified[id] |= OD_TRIED;
                });
            }
            _ => {}
        }
        let item = case_item(&name);
        let actual = desc_string(item, add_prefix);
        assert_eq!(actual, expected, "case {name} add_prefix={add_prefix}");
    }
}

// ---------------------------------------------------------------------------
// 2. bowDamageValue exhaustive
// ---------------------------------------------------------------------------
#[test]
fn bow_damage_value_matches_cpp() {
    for misc in -5..=10 {
        let expected = match misc {
            1 | 2 => 2,
            3 | 5 => 3,
            4 | 6 => 4,
            _ => -1,
        };
        assert_eq!(bow_damage_value(misc), expected, "misc_use={misc}");
    }
}

// ---------------------------------------------------------------------------
// 3. charge / count remaining messages
// ---------------------------------------------------------------------------
#[test]
fn item_charges_remaining_description_message() {
    setup_flavor_seed42();
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.inventory[0] = make_item(293, 64);
        s.py.inventory[0].misc_use = 7;
        s.py.inventory[0].identification = ID_KNOWN2;
        s.py.pack.unique_items = 1;
    });
    item_charges_remaining_description(0);
    assert_eq!(message_text(with_state(|s| s.last_message_id)), "You have 7 charges remaining.");
    test_set_ncurses_stub(false);
}

#[test]
fn item_charges_remaining_description_unidentified_silent() {
    setup_flavor_seed42();
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.inventory[0] = make_item(293, 64);
        s.py.inventory[0].misc_use = 7;
        s.py.pack.unique_items = 1;
    });
    let before = with_state(|s| s.last_message_id);
    item_charges_remaining_description(0);
    assert_eq!(with_state(|s| s.last_message_id), before);
    test_set_ncurses_stub(false);
}

#[test]
fn item_type_remaining_count_description_message() {
    setup_flavor_seed42();
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.inventory[0] = make_item(243, 64);
        s.py.inventory[0].items_count = 2;
        s.py.pack.unique_items = 1;
    });
    item_type_remaining_count_description(0);
    assert_eq!(
        message_text(with_state(|s| s.last_message_id)),
        "You have an Icky Green Potion."
    );
    test_set_ncurses_stub(false);
}

// ---------------------------------------------------------------------------
// 4. inscription editing
// ---------------------------------------------------------------------------
#[test]
fn item_append_to_inscription_sets_ident_flags() {
    let mut item = Inventory::default();
    item_append_to_inscription(&mut item, ID_MAGIK);
    assert_eq!(item.identification & ID_MAGIK, ID_MAGIK);
}

#[test]
fn item_replace_inscription_truncates_to_inscrip_size() {
    let mut item = Inventory::default();
    let long = vec![b'x'; 20];
    item_replace_inscription(&mut item, &long);
    let end = item
        .inscription
        .iter()
        .position(|&ch| ch == 0)
        .unwrap_or(INSCRIP_SIZE as usize);
    assert_eq!(end, INSCRIP_SIZE as usize - 1);
    assert!(item.inscription[..end].iter().all(|&ch| ch == b'x' as c_char));
}

#[test]
fn item_inscribe_empty_pack_message() {
    setup_flavor_seed42();
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.pack.unique_items = 0;
        s.py.equipment_count = 0;
    });
    item_inscribe();
    assert_eq!(
        message_text(with_state(|s| s.last_message_id)),
        "You are not carrying anything to inscribe."
    );
    test_set_ncurses_stub(false);
}

// ---------------------------------------------------------------------------
// 5. objectBlockedByMonster
// ---------------------------------------------------------------------------
#[test]
fn object_blocked_by_monster_lit() {
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.monsters[3] = Monster {
            lit: true,
            creature_id: 1,
            ..Monster::default()
        };
    });
    object_blocked_by_monster(3);
    assert_eq!(
        message_text(with_state(|s| s.last_message_id)),
        format!(
            "The {} is in your way!",
            CREATURES_LIST[1].name
        )
    );
    test_set_ncurses_stub(false);
}

#[test]
fn object_blocked_by_monster_unlit() {
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.monsters[3] = Monster {
            lit: false,
            creature_id: 1,
            ..Monster::default()
        };
    });
    object_blocked_by_monster(3);
    assert_eq!(
        message_text(with_state(|s| s.last_message_id)),
        "Something is in your way!"
    );
    test_set_ncurses_stub(false);
}

// ---------------------------------------------------------------------------
// 6. buffer / prefix behavior
// ---------------------------------------------------------------------------
#[test]
fn item_description_no_prefix_strips_article() {
    setup_flavor_seed42();
    let item = make_item(332, 64);
    assert_eq!(desc_string(item, false), "Rat Skeleton");
}

#[test]
fn spell_item_identified_gate() {
    let mut item = make_item(293, 64);
    item.misc_use = 5;
    assert!(!spell_item_identified(item));
    item.identification = ID_KNOWN2;
    assert!(spell_item_identified(item));
}
