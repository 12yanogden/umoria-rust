//! Item description formatting.

use super::{
    item_set_colorless_as_identified_for_state, object_position_offset, spell_item_identified,
    SpecialNameIds,
};
use crate::config::identification::{
    ID_DAMD, ID_EMPTY, ID_MAGIK, ID_NO_SHOW_P1, ID_SHOW_HIT_DAM, ID_SHOW_P1, OD_TRIED,
};
use crate::config::treasure::flags::{TR_STEALTH, TR_STR};
use crate::data_treasure::{GAME_OBJECTS, SPECIAL_ITEM_NAMES};
use crate::game::{with_state, with_state_mut, State};
use crate::helpers::{insert_string_into_string, is_vowel};
use crate::inventory::{Inventory, INSCRIP_SIZE, ITEM_SINGLE_STACK_MIN, PLAYER_INVENTORY_SIZE};
use crate::treasure::{
    TV_AMULET, TV_ARROW, TV_BOLT, TV_BOOTS, TV_BOW, TV_CHEST, TV_CLOAK, TV_CLOSED_DOOR, TV_DIGGING,
    TV_DOWN_STAIR, TV_FLASK, TV_FOOD, TV_GLOVES, TV_GOLD, TV_HAFTED, TV_HARD_ARMOR, TV_HELM,
    TV_INVIS_TRAP, TV_LIGHT, TV_MAGIC_BOOK, TV_MISC, TV_OPEN_DOOR, TV_POLEARM, TV_POTION1,
    TV_POTION2, TV_PRAYER_BOOK, TV_RING, TV_RUBBLE, TV_SCROLL1, TV_SCROLL2, TV_SECRET_DOOR,
    TV_SHIELD, TV_SLING_AMMO, TV_SOFT_ARMOR, TV_SPIKE, TV_STAFF, TV_STORE_DOOR, TV_SWORD,
    TV_UP_STAIR, TV_VIS_TRAP, TV_WAND,
};
use crate::types::{
    Obj_desc_t, Vtype_t, MORIA_MESSAGE_SIZE, MORIA_OBJ_DESC_SIZE, MORIA_OBJ_DESC_SIZE_LEN,
};
use crate::ui_inventory::inventory_get_input_for_item_id;
use crate::ui_io::terminal::{self, Coord};

#[derive(Clone, Copy, PartialEq, Eq)]
enum ItemMiscUse {
    Ignored,
    Charges,
    Plusses,
    Light,
    Flags,
    ZPlusses,
}

fn c_strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

pub(crate) fn c_str_from_buf(buf: &[u8]) -> &str {
    let end = c_strlen(buf);
    std::str::from_utf8(&buf[..end]).unwrap_or("")
}

fn c_strcpy_obj(dst: &mut Obj_desc_t, src: &[u8]) {
    let len = c_strlen(src).min(MORIA_OBJ_DESC_SIZE_LEN - 1);
    dst[..len].copy_from_slice(&src[..len]);
    dst[len] = 0;
}

fn c_strcpy_obj_str(dst: &mut Obj_desc_t, src: &str) {
    c_strcpy_obj(dst, src.as_bytes());
}

fn c_strcat_obj(dst: &mut Obj_desc_t, src: &[u8]) {
    let mut pos = c_strlen(dst);
    for &byte in src {
        if byte == 0 {
            break;
        }
        if pos >= MORIA_OBJ_DESC_SIZE_LEN - 1 {
            break;
        }
        dst[pos] = byte;
        pos += 1;
    }
    dst[pos] = 0;
}

fn c_strcat_obj_str(dst: &mut Obj_desc_t, src: &str) {
    c_strcat_obj(dst, src.as_bytes());
}

fn c_strcat_vtype_str(dst: &mut Vtype_t, src: &str) {
    let mut pos = c_strlen(dst);
    for byte in src.bytes() {
        if pos >= MORIA_MESSAGE_SIZE - 1 {
            break;
        }
        dst[pos] = byte;
        pos += 1;
    }
    dst[pos] = 0;
}

fn c_strcat_vtype(dst: &mut Vtype_t, src: &[u8]) {
    let mut pos = c_strlen(dst);
    for &byte in src {
        if byte == 0 {
            break;
        }
        if pos >= MORIA_MESSAGE_SIZE - 1 {
            break;
        }
        dst[pos] = byte;
        pos += 1;
    }
    dst[pos] = 0;
}

fn snprintf_vtype(dst: &mut Vtype_t, formatted: &str) {
    let bytes = formatted.as_bytes();
    let n = bytes.len().min(MORIA_MESSAGE_SIZE - 1);
    dst[..n].copy_from_slice(&bytes[..n]);
    dst[n] = 0;
}

fn snprintf_obj_desc_truncated(
    dst: &mut Obj_desc_t,
    prefix: &str,
    suffix: &[u8],
    max_suffix: usize,
) {
    let mut pos = 0usize;
    for byte in prefix.bytes() {
        if pos >= MORIA_OBJ_DESC_SIZE_LEN - 1 {
            break;
        }
        dst[pos] = byte;
        pos += 1;
    }
    let suffix_len = c_strlen(suffix).min(max_suffix);
    for &byte in &suffix[..suffix_len] {
        if pos >= MORIA_OBJ_DESC_SIZE_LEN - 1 {
            break;
        }
        dst[pos] = byte;
        pos += 1;
    }
    dst[pos] = 0;
}

fn insert_into_tmp_val(tmp_val: &mut Obj_desc_t, from: &[u8], insert: Option<&[u8]>) {
    // SAFETY: `Vtype_t` is the prefix of `Obj_desc_t` (`MORIA_MESSAGE_SIZE` bytes)
    let vtype: &mut Vtype_t = unsafe { &mut *(tmp_val.as_mut_ptr().cast::<Vtype_t>()) };
    insert_string_into_string(vtype, from, insert);
    tmp_val[MORIA_MESSAGE_SIZE - 1] = 0;
}

fn item_store_bought(identification: u8) -> bool {
    (identification & crate::config::identification::ID_STORE_BOUGHT) != 0
}

/// 504
pub fn bow_damage_value(misc_use: i16) -> i32 {
    if misc_use == 1 || misc_use == 2 {
        2
    } else if misc_use == 3 || misc_use == 5 {
        3
    } else if misc_use == 4 || misc_use == 6 {
        4
    } else {
        -1
    }
}

/// 861
pub fn item_description(out: &mut Obj_desc_t, item: Inventory, add_prefix: bool) {
    with_state(|state| item_description_for_state(out, item, add_prefix, state));
}

pub fn item_description_for_state(
    out: &mut Obj_desc_t,
    item: Inventory,
    add_prefix: bool,
    state: &State,
) {
    let mut indexx = usize::from(item.sub_category_id & (ITEM_SINGLE_STACK_MIN - 1));
    let object_name = GAME_OBJECTS[item.id as usize].name;
    let mut basenm = object_name;
    let mut modstr: Option<&str> = None;
    let mut damstr = [0u8; MORIA_MESSAGE_SIZE];
    let mut append_name = false;
    let modify = !item_set_colorless_as_identified_for_state(
        state,
        item.category_id,
        item.sub_category_id,
        item.identification,
    );
    let mut misc_type = ItemMiscUse::Ignored;

    match item.category_id {
        TV_MISC | TV_CHEST => {}
        TV_SLING_AMMO | TV_BOLT | TV_ARROW => {
            snprintf_vtype(
                &mut damstr,
                &format!(" ({}d{})", item.damage.dice, item.damage.sides),
            );
        }
        TV_LIGHT => misc_type = ItemMiscUse::Light,
        TV_SPIKE => {}
        TV_BOW => {
            snprintf_vtype(
                &mut damstr,
                &format!(" (x{})", bow_damage_value(item.misc_use)),
            );
        }
        TV_HAFTED | TV_POLEARM | TV_SWORD => {
            snprintf_vtype(
                &mut damstr,
                &format!(" ({}d{})", item.damage.dice, item.damage.sides),
            );
            misc_type = ItemMiscUse::Flags;
        }
        TV_DIGGING => {
            misc_type = ItemMiscUse::ZPlusses;
            snprintf_vtype(
                &mut damstr,
                &format!(" ({}d{})", item.damage.sides, item.damage.sides),
            );
        }
        TV_BOOTS | TV_GLOVES | TV_CLOAK | TV_HELM | TV_SHIELD | TV_HARD_ARMOR | TV_SOFT_ARMOR => {}
        TV_AMULET => {
            if modify {
                basenm = "& %s Amulet";
                modstr = Some(state.flavor.amulet_name(indexx));
            } else {
                basenm = "& Amulet";
                append_name = true;
            }
            misc_type = ItemMiscUse::Plusses;
        }
        TV_RING => {
            if modify {
                basenm = "& %s Ring";
                modstr = Some(state.flavor.rock_name(indexx));
            } else {
                basenm = "& Ring";
                append_name = true;
            }
            misc_type = ItemMiscUse::Plusses;
        }
        TV_STAFF => {
            if modify {
                basenm = "& %s Staff";
                modstr = Some(state.flavor.wood_name(indexx));
            } else {
                basenm = "& Staff";
                append_name = true;
            }
            misc_type = ItemMiscUse::Charges;
        }
        TV_WAND => {
            if modify {
                basenm = "& %s Wand";
                modstr = Some(state.flavor.metal_name(indexx));
            } else {
                basenm = "& Wand";
                append_name = true;
            }
            misc_type = ItemMiscUse::Charges;
        }
        TV_SCROLL1 | TV_SCROLL2 => {
            if modify {
                basenm = "& Scroll~ titled \"%s\"";
                modstr = Some(state.flavor.magic_item_title(indexx));
            } else {
                basenm = "& Scroll~";
                append_name = true;
            }
        }
        TV_POTION1 | TV_POTION2 => {
            if modify {
                basenm = "& %s Potion~";
                modstr = Some(state.flavor.color_name(indexx));
            } else {
                basenm = "& Potion~";
                append_name = true;
            }
        }
        TV_FLASK => {}
        TV_FOOD => {
            if modify {
                if indexx <= 15 {
                    basenm = "& %s Mushroom~";
                } else if indexx <= 20 {
                    basenm = "& Hairy %s Mold~";
                }
                if indexx <= 20 {
                    modstr = Some(state.flavor.mushroom_name(indexx));
                }
            } else {
                append_name = true;
                if indexx <= 15 {
                    basenm = "& Mushroom~";
                } else if indexx <= 20 {
                    basenm = "& Hairy Mold~";
                } else {
                    append_name = false;
                }
            }
        }
        TV_MAGIC_BOOK => {
            modstr = Some(basenm);
            basenm = "& Book~ of Magic Spells %s";
        }
        TV_PRAYER_BOOK => {
            modstr = Some(basenm);
            basenm = "& Holy Book~ of Prayers %s";
        }
        TV_OPEN_DOOR | TV_CLOSED_DOOR | TV_SECRET_DOOR | TV_RUBBLE => {}
        TV_GOLD | TV_INVIS_TRAP | TV_VIS_TRAP | TV_UP_STAIR | TV_DOWN_STAIR => {
            c_strcpy_obj_str(out, object_name);
            c_strcat_obj_str(out, ".");
            return;
        }
        TV_STORE_DOOR => {
            let formatted = format!("the entrance to the {object_name}.");
            c_strcpy_obj_str(out, &formatted);
            return;
        }
        _ => {
            c_strcpy_obj_str(out, "Error in objdes()");
            return;
        }
    }

    let mut tmp_val = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
    if let Some(modifier) = modstr {
        c_strcpy_obj_str(&mut tmp_val, &basenm.replacen("%s", modifier, 1));
    } else {
        c_strcpy_obj_str(&mut tmp_val, basenm);
    }

    if append_name {
        c_strcat_obj_str(&mut tmp_val, " of ");
        c_strcat_obj_str(&mut tmp_val, object_name);
    }

    if item.items_count == 1 {
        insert_into_tmp_val(&mut tmp_val, b"~", None);
    } else {
        insert_into_tmp_val(&mut tmp_val, b"ch~", Some(b"ches"));
        insert_into_tmp_val(&mut tmp_val, b"~", Some(b"s"));
    }

    if !add_prefix {
        if tmp_val.starts_with(b"some") {
            c_strcpy_obj(out, &tmp_val[5..]);
        } else if tmp_val[0] == b'&' {
            c_strcpy_obj(out, &tmp_val[2..]);
        } else {
            c_strcpy_obj(out, &tmp_val);
        }
        return;
    }

    if item.special_name_id != SpecialNameIds::SN_NULL as u8 && spell_item_identified(item) {
        c_strcat_obj_str(&mut tmp_val, " ");
        c_strcat_obj_str(
            &mut tmp_val,
            SPECIAL_ITEM_NAMES[item.special_name_id as usize],
        );
    }

    if damstr[0] != 0 {
        c_strcat_obj(&mut tmp_val, &damstr);
    }

    let mut tmp_str = [0u8; MORIA_MESSAGE_SIZE];

    if spell_item_identified(item) {
        let abs_to_hit = item.to_hit.unsigned_abs();
        let abs_to_damage = item.to_damage.unsigned_abs();

        if (item.identification & ID_SHOW_HIT_DAM) != 0 {
            snprintf_vtype(
                &mut tmp_str,
                &format!(
                    " ({}{},{}{})",
                    if item.to_hit < 0 { '-' } else { '+' },
                    abs_to_hit,
                    if item.to_damage < 0 { '-' } else { '+' },
                    abs_to_damage,
                ),
            );
        } else if item.to_hit != 0 {
            snprintf_vtype(
                &mut tmp_str,
                &format!(
                    " ({}{})",
                    if item.to_hit < 0 { '-' } else { '+' },
                    abs_to_hit,
                ),
            );
        } else if item.to_damage != 0 {
            snprintf_vtype(
                &mut tmp_str,
                &format!(
                    " ({}{})",
                    if item.to_damage < 0 { '-' } else { '+' },
                    abs_to_damage,
                ),
            );
        } else {
            tmp_str[0] = 0;
        }
        c_strcat_obj(&mut tmp_val, &tmp_str);
    }

    let abs_to_ac = item.to_ac.unsigned_abs();
    if item.ac != 0 || item.category_id == TV_HELM {
        snprintf_vtype(&mut tmp_str, &format!(" [{}", item.ac));
        c_strcat_obj(&mut tmp_val, &tmp_str);
        if spell_item_identified(item) {
            snprintf_vtype(
                &mut tmp_str,
                &format!(",{}{}", if item.to_ac < 0 { '-' } else { '+' }, abs_to_ac),
            );
            c_strcat_obj(&mut tmp_val, &tmp_str);
        }
        c_strcat_obj_str(&mut tmp_val, "]");
    } else if item.to_ac != 0 && spell_item_identified(item) {
        snprintf_vtype(
            &mut tmp_str,
            &format!(" [{}{}]", if item.to_ac < 0 { '-' } else { '+' }, abs_to_ac),
        );
        c_strcat_obj(&mut tmp_val, &tmp_str);
    }

    if (item.identification & ID_NO_SHOW_P1) != 0 {
        misc_type = ItemMiscUse::Ignored;
    } else if (item.identification & ID_SHOW_P1) != 0 {
        misc_type = ItemMiscUse::ZPlusses;
    }

    tmp_str = [0u8; MORIA_MESSAGE_SIZE];
    if misc_type == ItemMiscUse::Light {
        snprintf_vtype(
            &mut tmp_str,
            &format!(" with {} turns of light", item.misc_use),
        );
    } else if misc_type != ItemMiscUse::Ignored && spell_item_identified(item) {
        let abs_misc_use = item.misc_use.unsigned_abs();
        match misc_type {
            ItemMiscUse::ZPlusses => {
                snprintf_vtype(
                    &mut tmp_str,
                    &format!(
                        " ({}{})",
                        if item.misc_use < 0 { '-' } else { '+' },
                        abs_misc_use,
                    ),
                );
            }
            ItemMiscUse::Charges => {
                snprintf_vtype(&mut tmp_str, &format!(" ({} charges)", item.misc_use));
            }
            ItemMiscUse::Plusses if item.misc_use != 0 => {
                snprintf_vtype(
                    &mut tmp_str,
                    &format!(
                        " ({}{})",
                        if item.misc_use < 0 { '-' } else { '+' },
                        abs_misc_use,
                    ),
                );
            }
            ItemMiscUse::Flags if item.misc_use != 0 => {
                if (item.flags & TR_STR) != 0 {
                    snprintf_vtype(
                        &mut tmp_str,
                        &format!(
                            " ({}{} to STR)",
                            if item.misc_use < 0 { '-' } else { '+' },
                            abs_misc_use,
                        ),
                    );
                } else if (item.flags & TR_STEALTH) != 0 {
                    snprintf_vtype(
                        &mut tmp_str,
                        &format!(
                            " ({}{} to stealth)",
                            if item.misc_use < 0 { '-' } else { '+' },
                            abs_misc_use,
                        ),
                    );
                }
            }
            _ => {}
        }
    }
    c_strcat_obj(&mut tmp_val, &tmp_str);

    if tmp_val[0] == b'&' {
        let suffix = &tmp_val[1..];
        if item.items_count > 1 {
            snprintf_obj_desc_truncated(
                out,
                &item.items_count.to_string(),
                suffix,
                (MORIA_OBJ_DESC_SIZE - 4) as usize,
            );
        } else if item.items_count < 1 {
            snprintf_obj_desc_truncated(out, "no more", suffix, (MORIA_OBJ_DESC_SIZE - 8) as usize);
        } else if is_vowel(tmp_val[2]) {
            snprintf_obj_desc_truncated(out, "an", suffix, (MORIA_OBJ_DESC_SIZE - 3) as usize);
        } else {
            snprintf_obj_desc_truncated(out, "a", suffix, (MORIA_OBJ_DESC_SIZE - 2) as usize);
        }
    } else if item.items_count < 1 {
        let max_width = MORIA_OBJ_DESC_SIZE as usize - "no more ".len();
        if tmp_val.starts_with(b"some") {
            let mut prefix = "no more ".to_string();
            let suffix = &tmp_val[5..];
            let suffix_len = c_strlen(suffix).min(max_width);
            prefix.push_str(std::str::from_utf8(&suffix[..suffix_len]).unwrap_or(""));
            c_strcpy_obj_str(out, &prefix);
        } else {
            let mut prefix = "no more ".to_string();
            let suffix_len = c_strlen(&tmp_val).min(max_width);
            prefix.push_str(std::str::from_utf8(&tmp_val[..suffix_len]).unwrap_or(""));
            c_strcpy_obj_str(out, &prefix);
        }
    } else {
        c_strcpy_obj(out, &tmp_val);
    }

    tmp_str = [0u8; MORIA_MESSAGE_SIZE];
    let offset = object_position_offset(item.category_id, item.sub_category_id);
    if offset >= 0 {
        indexx = (offset as usize) << 6;
        indexx += usize::from(item.sub_category_id & (ITEM_SINGLE_STACK_MIN - 1));
        if (state.objects_identified[indexx] & OD_TRIED) != 0
            && !item_store_bought(item.identification)
        {
            c_strcat_vtype_str(&mut tmp_str, "tried ");
        }
    }

    if (item.identification & (ID_MAGIK | ID_EMPTY | ID_DAMD)) != 0 {
        if (item.identification & ID_MAGIK) != 0 {
            c_strcat_vtype_str(&mut tmp_str, "magik ");
        }
        if (item.identification & ID_EMPTY) != 0 {
            c_strcat_vtype_str(&mut tmp_str, "empty ");
        }
        if (item.identification & ID_DAMD) != 0 {
            c_strcat_vtype_str(&mut tmp_str, "damned ");
        }
    }

    if item.inscription[0] != 0 {
        let end = item
            .inscription
            .iter()
            .position(|&ch| ch == 0)
            .unwrap_or(item.inscription.len());
        for &ch in &item.inscription[..end] {
            c_strcat_vtype(&mut tmp_str, &[ch as u8]);
        }
    } else {
        let len = c_strlen(&tmp_str);
        if len > 0 {
            tmp_str[len - 1] = 0;
        }
    }

    if tmp_str[0] != 0 {
        let brace = format!(" {{{}}}", c_str_from_buf(&tmp_str));
        c_strcat_obj_str(out, &brace);
    }

    c_strcat_obj_str(out, ".");
}

/// 874
pub fn item_charges_remaining_description(item_id: i32) {
    let message = with_state(|state| {
        let item = state.py.inventory[item_id as usize];
        if !spell_item_identified(item) {
            return None;
        }
        Some(format!("You have {} charges remaining.", item.misc_use))
    });
    if let Some(message) = message {
        terminal::print_message(Some(&message));
    }
}

/// 891
pub fn item_type_remaining_count_description(item_id: i32) {
    let message = with_state_mut(|state| {
        let mut item = state.py.inventory[item_id as usize];
        item.items_count -= 1;
        let mut tmp_str = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
        item_description_for_state(&mut tmp_str, item, true, state);
        format!("You have {}", c_str_from_buf(&tmp_str))
    });
    terminal::print_message(Some(&message));
}

/// 929
pub fn item_inscribe() {
    let (unique, equip) = with_state(|s| (s.py.pack.unique_items, s.py.equipment_count));
    if unique == 0 && equip == 0 {
        terminal::print_message(Some("You are not carrying anything to inscribe."));
        return;
    }

    let mut item_id = 0i32;
    if !inventory_get_input_for_item_id(
        &mut item_id,
        "Which one? ",
        0,
        i32::from(PLAYER_INVENTORY_SIZE),
        None,
        None,
    ) {
        return;
    }

    let mut msg = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
    with_state(|state| {
        item_description_for_state(&mut msg, state.py.inventory[item_id as usize], true, state);
    });

    terminal::print_message(Some(&format!("Inscribing {}", c_str_from_buf(&msg))));

    let mut inscription = [0u8; MORIA_OBJ_DESC_SIZE_LEN];
    if with_state(|state| state.py.inventory[item_id as usize].inscription[0]) != 0 {
        let existing = with_state(|state| {
            let item = &state.py.inventory[item_id as usize];
            let end = item
                .inscription
                .iter()
                .position(|&ch| ch == 0)
                .unwrap_or(INSCRIP_SIZE as usize);
            String::from_utf8_lossy(
                &item.inscription[..end]
                    .iter()
                    .map(|&ch| ch as u8)
                    .collect::<Vec<_>>(),
            )
            .into_owned()
        });
        let formatted = format!("Replace {existing} New inscription:");
        c_strcpy_obj_str(&mut inscription, &formatted);
    } else {
        c_strcpy_obj_str(&mut inscription, "Inscription: ");
    }

    let mut msg_len = 78 - c_strlen(&msg) as i32;
    if msg_len > 12 {
        msg_len = 12;
    }

    terminal::put_string_clear_to_eol(c_str_from_buf(&inscription), Coord { y: 0, x: 0 });

    let prompt_len = c_strlen(&inscription) as i32;
    if terminal::get_string_input(
        &mut inscription,
        Coord {
            y: 0,
            x: prompt_len,
        },
        msg_len,
    ) {
        with_state_mut(|state| {
            item_replace_inscription(&mut state.py.inventory[item_id as usize], &inscription);
        });
    }
}

/// 939
pub fn item_replace_inscription(item: &mut Inventory, inscription: &[u8]) {
    let len = c_strlen(inscription).min(INSCRIP_SIZE as usize - 1);
    for (dst, &src) in item.inscription[..len].iter_mut().zip(inscription.iter()) {
        *dst = src as i8;
    }
    item.inscription[len] = 0;
}
