//! Port of src/identification.cpp — flavor state, identification flags, identify logic.

pub const MAX_COLORS: u8 = 49;
pub const MAX_MUSHROOMS: u8 = 22;
pub const MAX_WOODS: u8 = 25;
pub const MAX_METALS: u8 = 25;
pub const MAX_ROCKS: u8 = 32;
pub const MAX_AMULETS: u8 = 11;
pub const MAX_TITLES: u8 = 45;
pub const MAX_SYLLABLES: u8 = 153;

/// Port of `SpecialNameIds` in identification.h.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum SpecialNameIds {
    SN_NULL = 0,
    SN_R,
    SN_RA,
    SN_RF,
    SN_RC,
    SN_RL,
    SN_HA,
    SN_DF,
    SN_SA,
    SN_SD,
    SN_SE,
    SN_SU,
    SN_FT,
    SN_FB,
    SN_FREE_ACTION,
    SN_SLAYING,
    SN_CLUMSINESS,
    SN_WEAKNESS,
    SN_SLOW_DESCENT,
    SN_SPEED,
    SN_STEALTH,
    SN_SLOWNESS,
    SN_NOISE,
    SN_GREAT_MASS,
    SN_INTELLIGENCE,
    SN_WISDOM,
    SN_INFRAVISION,
    SN_MIGHT,
    SN_LORDLINESS,
    SN_MAGI,
    SN_BEAUTY,
    SN_SEEING,
    SN_REGENERATION,
    SN_STUPIDITY,
    SN_DULLNESS,
    SN_BLINDNESS,
    SN_TIMIDNESS,
    SN_TELEPORTATION,
    SN_UGLINESS,
    SN_PROTECTION,
    SN_IRRITATION,
    SN_VULNERABILITY,
    SN_ENVELOPING,
    SN_FIRE,
    SN_SLAY_EVIL,
    SN_DRAGON_SLAYING,
    SN_EMPTY,
    SN_LOCKED,
    SN_POISON_NEEDLE,
    SN_GAS_TRAP,
    SN_EXPLOSION_DEVICE,
    SN_SUMMONING_RUNES,
    SN_MULTIPLE_TRAPS,
    SN_DISARMED,
    SN_UNLOCKED,
    SN_SLAY_ANIMAL,
    SN_ARRAY_SIZE,
}

use crate::config::dungeon::objects::OBJ_NOTHING;
use crate::config::identification::{
    ID_DAMD, ID_EMPTY, ID_KNOWN2, ID_MAGIK, ID_STORE_BOUGHT, OD_KNOWN1, OD_TRIED,
};
use crate::data_creatures::CREATURES_LIST;
use crate::data_treasure::{AMULETS, COLORS, METALS, MUSHROOMS, ROCKS, SYLLABLES, WOODS};
use crate::game::{
    random_number_state, seed_reset_to_old_seed_state, seed_set_state, with_state, with_state_mut,
    State,
};
use crate::inventory::{
    inventory_item_copy_to, inventory_item_is_cursed, inventory_item_single_stackable, Inventory,
    ITEM_SINGLE_STACK_MIN,
};
use crate::recall::recall_monster_attributes;
use crate::treasure::{
    TV_AMULET, TV_FOOD, TV_POTION1, TV_POTION2, TV_RING, TV_SCROLL1, TV_SCROLL2, TV_STAFF, TV_WAND,
};
use crate::ui_io::terminal::{self, Coord};

/// Shuffled flavor-array order + scroll titles (identification.cpp global state).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlavorTables {
    pub color_order: [u8; MAX_COLORS as usize],
    pub wood_order: [u8; MAX_WOODS as usize],
    pub metal_order: [u8; MAX_METALS as usize],
    pub rock_order: [u8; MAX_ROCKS as usize],
    pub amulet_order: [u8; MAX_AMULETS as usize],
    pub mushroom_order: [u8; MAX_MUSHROOMS as usize],
    pub magic_item_titles: [[u8; 10]; MAX_TITLES as usize],
}

impl Default for FlavorTables {
    fn default() -> Self {
        Self::from_static_defaults()
    }
}

impl FlavorTables {
    pub fn from_static_defaults() -> Self {
        Self {
            color_order: std::array::from_fn(|i| i as u8),
            wood_order: std::array::from_fn(|i| i as u8),
            metal_order: std::array::from_fn(|i| i as u8),
            rock_order: std::array::from_fn(|i| i as u8),
            amulet_order: std::array::from_fn(|i| i as u8),
            mushroom_order: std::array::from_fn(|i| i as u8),
            magic_item_titles: [[0; 10]; MAX_TITLES as usize],
        }
    }

    pub fn color_name(&self, index: usize) -> &str {
        COLORS[self.color_order[index] as usize]
    }

    pub fn wood_name(&self, index: usize) -> &str {
        WOODS[self.wood_order[index] as usize]
    }

    pub fn metal_name(&self, index: usize) -> &str {
        METALS[self.metal_order[index] as usize]
    }

    pub fn rock_name(&self, index: usize) -> &str {
        ROCKS[self.rock_order[index] as usize]
    }

    pub fn amulet_name(&self, index: usize) -> &str {
        AMULETS[self.amulet_order[index] as usize]
    }

    pub fn mushroom_name(&self, index: usize) -> &str {
        MUSHROOMS[self.mushroom_order[index] as usize]
    }

    pub fn magic_item_title(&self, index: usize) -> &str {
        c_str_from_title(&self.magic_item_titles[index])
    }
}

fn c_str_from_title(buf: &[u8; 10]) -> &str {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(10);
    std::str::from_utf8(&buf[..end]).unwrap_or("")
}

fn append_to_title(title: &mut [u8; 10], pos: &mut usize, text: &str) {
    for byte in text.bytes() {
        if *pos >= 9 {
            break;
        }
        title[*pos] = byte;
        *pos += 1;
    }
}

fn object_ident_index(category_id: u8, sub_category_id: u8) -> Option<usize> {
    let offset = object_position_offset(category_id, sub_category_id);
    if offset < 0 {
        return None;
    }
    let id = (offset as usize) << 6;
    Some(id + usize::from(sub_category_id & (ITEM_SINGLE_STACK_MIN - 1)))
}

fn clear_object_tried_flag(state: &mut State, id: usize) {
    state.objects_identified[id] &= !OD_TRIED;
}

fn set_object_tried_flag(state: &mut State, id: usize) {
    state.objects_identified[id] |= OD_TRIED;
}

fn is_object_known(state: &State, id: usize) -> bool {
    (state.objects_identified[id] & OD_KNOWN1) != 0
}

fn item_store_bought(identification: u8) -> bool {
    (identification & ID_STORE_BOUGHT) != 0
}

/// C++ identification.cpp lines 16–216.
pub fn object_description(command: u8) -> String {
    with_state(|state| object_description_for_state(state, command))
}

fn object_description_for_state(state: &State, command: u8) -> String {
    match command {
        b' ' => "  - An open pit.".to_string(),
        b'!' => "! - A potion.".to_string(),
        b'"' => "\" - An amulet, periapt, or necklace.".to_string(),
        b'#' => "# - A stone wall.".to_string(),
        b'$' => "$ - Treasure.".to_string(),
        b'%' if !state.options.highlight_seams => "% - Not used.".to_string(),
        b'%' => "% - A magma or quartz vein.".to_string(),
        b'&' => "& - Treasure chest.".to_string(),
        b'\'' => "' - An open door.".to_string(),
        b'(' => "( - Soft armor.".to_string(),
        b')' => ") - A shield.".to_string(),
        b'*' => "* - Gems.".to_string(),
        b'+' => "+ - A closed door.".to_string(),
        b',' => ", - Food or mushroom patch.".to_string(),
        b'-' => "- - A wand".to_string(),
        b'.' => ". - Floor.".to_string(),
        b'/' => "/ - A pole weapon.".to_string(),
        b'1' => "1 - Entrance to General Store.".to_string(),
        b'2' => "2 - Entrance to Armory.".to_string(),
        b'3' => "3 - Entrance to Weaponsmith.".to_string(),
        b'4' => "4 - Entrance to Temple.".to_string(),
        b'5' => "5 - Entrance to Alchemy shop.".to_string(),
        b'6' => "6 - Entrance to Magic-Users store.".to_string(),
        b':' => ": - Rubble.".to_string(),
        b';' => "; - A loose rock.".to_string(),
        b'<' => "< - An up staircase.".to_string(),
        b'=' => "= - A ring.".to_string(),
        b'>' => "> - A down staircase.".to_string(),
        b'?' => "? - A scroll.".to_string(),
        b'@' => {
            let end = state
                .py
                .misc
                .name
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(state.py.misc.name.len());
            String::from_utf8_lossy(&state.py.misc.name[..end]).into_owned()
        }
        b'A' => "A - Giant Ant Lion.".to_string(),
        b'B' => "B - The Balrog.".to_string(),
        b'C' => "C - Gelatinous Cube.".to_string(),
        b'D' => "D - An Ancient Dragon (Beware).".to_string(),
        b'E' => "E - Elemental.".to_string(),
        b'F' => "F - Giant Fly.".to_string(),
        b'G' => "G - Ghost.".to_string(),
        b'H' => "H - Hobgoblin.".to_string(),
        b'J' => "J - Jelly.".to_string(),
        b'K' => "K - Killer Beetle.".to_string(),
        b'L' => "L - Lich.".to_string(),
        b'M' => "M - Mummy.".to_string(),
        b'O' => "O - Ooze.".to_string(),
        b'P' => "P - Giant humanoid.".to_string(),
        b'Q' => "Q - Quylthulg (Pulsing Flesh Mound).".to_string(),
        b'R' => "R - Reptile.".to_string(),
        b'S' => "S - Giant Scorpion.".to_string(),
        b'T' => "T - Troll.".to_string(),
        b'U' => "U - Umber Hulk.".to_string(),
        b'V' => "V - Vampire.".to_string(),
        b'W' => "W - Wight or Wraith.".to_string(),
        b'X' => "X - Xorn.".to_string(),
        b'Y' => "Y - Yeti.".to_string(),
        b'[' => "[ - Hard armor.".to_string(),
        b'\\' => "\\ - A hafted weapon.".to_string(),
        b']' => "] - Misc. armor.".to_string(),
        b'^' => "^ - A trap.".to_string(),
        b'_' => "_ - A staff.".to_string(),
        b'a' => "a - Giant Ant.".to_string(),
        b'b' => "b - Giant Bat.".to_string(),
        b'c' => "c - Giant Centipede.".to_string(),
        b'd' => "d - Dragon.".to_string(),
        b'e' => "e - Floating Eye.".to_string(),
        b'f' => "f - Giant Frog.".to_string(),
        b'g' => "g - Golem.".to_string(),
        b'h' => "h - Harpy.".to_string(),
        b'i' => "i - Icky Thing.".to_string(),
        b'j' => "j - Jackal.".to_string(),
        b'k' => "k - Kobold.".to_string(),
        b'l' => "l - Giant Louse.".to_string(),
        b'm' => "m - Mold.".to_string(),
        b'n' => "n - Naga.".to_string(),
        b'o' => "o - Orc or Ogre.".to_string(),
        b'p' => "p - Person (Humanoid).".to_string(),
        b'q' => "q - Quasit.".to_string(),
        b'r' => "r - Rodent.".to_string(),
        b's' => "s - Skeleton.".to_string(),
        b't' => "t - Giant Tick.".to_string(),
        b'w' => "w - Worm or Worm Mass.".to_string(),
        b'y' => "y - Yeek.".to_string(),
        b'z' => "z - Zombie.".to_string(),
        b'{' => "{ - Arrow, bolt, or bullet.".to_string(),
        b'|' => "| - A sword or dagger.".to_string(),
        b'}' => "} - Bow, crossbow, or sling.".to_string(),
        b'~' => "~ - Miscellaneous item.".to_string(),
        _ => "Not Used.".to_string(),
    }
}

/// C++ identification.cpp lines 218–227.
pub fn identify_game_object() {
    let mut item_id = 0u8;
    if !terminal::get_tile_character("Enter character to be identified :", &mut item_id) {
        return;
    }

    let desc = object_description(item_id);
    terminal::put_string_clear_to_eol(&desc, Coord { y: 0, x: 0 });
    recall_monster_attributes(item_id);
}

/// C++ identification.cpp lines 230–304.
pub fn magic_initialize_item_names() {
    with_state_mut(magic_initialize_item_names_state);
}

pub(crate) fn magic_initialize_item_names_state(state: &mut State) {
    seed_set_state(state, state.game.magic_seed);

    for i in 3..MAX_COLORS as i32 {
        let id = random_number_state(state, (MAX_COLORS - 3) as i32) + 2;
        state.flavor.color_order.swap(i as usize, id as usize);
    }

    for i in 0..MAX_WOODS as usize {
        let id = random_number_state(state, MAX_WOODS as i32) - 1;
        state.flavor.wood_order.swap(i, id as usize);
    }

    for i in 0..MAX_METALS as usize {
        let id = random_number_state(state, MAX_METALS as i32) - 1;
        state.flavor.metal_order.swap(i, id as usize);
    }

    for i in 0..MAX_ROCKS as usize {
        let id = random_number_state(state, MAX_ROCKS as i32) - 1;
        state.flavor.rock_order.swap(i, id as usize);
    }

    for i in 0..MAX_AMULETS as usize {
        let id = random_number_state(state, MAX_AMULETS as i32) - 1;
        state.flavor.amulet_order.swap(i, id as usize);
    }

    for i in 0..MAX_MUSHROOMS as usize {
        let id = random_number_state(state, MAX_MUSHROOMS as i32) - 1;
        state.flavor.mushroom_order.swap(i, id as usize);
    }

    for i in 0..MAX_TITLES as usize {
        let mut title = [0u8; 10];
        let mut pos = 0usize;
        let k = random_number_state(state, 2) + 1;

        for group in 0..k {
            let syllable_count = random_number_state(state, 2);
            for _ in 0..syllable_count {
                let idx = random_number_state(state, MAX_SYLLABLES as i32) - 1;
                append_to_title(&mut title, &mut pos, SYLLABLES[idx as usize]);
            }
            if group < k - 1 {
                append_to_title(&mut title, &mut pos, " ");
            }
        }

        if title[8] == b' ' {
            title[8] = 0;
        } else {
            title[9] = 0;
        }
        state.flavor.magic_item_titles[i] = title;
    }

    seed_reset_to_old_seed_state(state);
}

/// C++ identification.cpp lines 306–330.
pub fn object_position_offset(category_id: u8, sub_category_id: u8) -> i16 {
    match category_id {
        TV_AMULET => 0,
        TV_RING => 1,
        TV_STAFF => 2,
        TV_WAND => 3,
        TV_SCROLL1 | TV_SCROLL2 => 4,
        TV_POTION1 | TV_POTION2 => 5,
        TV_FOOD if (sub_category_id & (ITEM_SINGLE_STACK_MIN - 1)) < MAX_MUSHROOMS => 6,
        _ => -1,
    }
}

/// Tomb/death flow: identify one player inventory slot by index.
pub(crate) fn identify_player_inventory_slot(state: &mut State, index: usize) {
    let (category_id, sub_category_id) = {
        let item = &state.py.inventory[index];
        (item.category_id, item.sub_category_id)
    };
    if let Some(id) = object_ident_index(category_id, sub_category_id) {
        state.objects_identified[id] |= OD_KNOWN1;
        clear_object_tried_flag(state, id);
    }
    {
        let item = &mut state.py.inventory[index];
        item.identification &= !(ID_MAGIK | ID_EMPTY);
    }
    if let Some(id) = object_ident_index(category_id, sub_category_id) {
        clear_object_tried_flag(state, id);
    }
    state.py.inventory[index].identification |= ID_KNOWN2;
}

/// C++ identification.cpp lines 345–359.
pub fn item_set_as_identified(category_id: u8, sub_category_id: u8) {
    with_state_mut(|state| {
        let Some(id) = object_ident_index(category_id, sub_category_id) else {
            return;
        };
        state.objects_identified[id] |= OD_KNOWN1;
        clear_object_tried_flag(state, id);
    });
}

/// C++ identification.cpp lines 362–377.
fn unsample(state: &mut State, item: &mut Inventory) {
    item.identification &= !(ID_MAGIK | ID_EMPTY);

    let Some(id) = object_ident_index(item.category_id, item.sub_category_id) else {
        return;
    };
    clear_object_tried_flag(state, id);
}

/// C++ identification.cpp lines 380–383.
pub fn spell_item_identify_and_remove_random_inscription(item: &mut Inventory) {
    with_state_mut(|state| {
        unsample(state, item);
        item.identification |= ID_KNOWN2;
    });
}

pub(crate) fn spell_item_identify_and_remove_random_inscription_for_state(
    state: &mut State,
    treasure_id: usize,
) {
    let (category_id, sub_category_id) = {
        let item = &mut state.game.treasure.list[treasure_id];
        item.identification &= !(ID_MAGIK | ID_EMPTY);
        (item.category_id, item.sub_category_id)
    };
    if let Some(id) = object_ident_index(category_id, sub_category_id) {
        clear_object_tried_flag(state, id);
    }
    state.game.treasure.list[treasure_id].identification |= ID_KNOWN2;
}

/// C++ identification.cpp lines 385–387.
pub fn spell_item_identified(item: Inventory) -> bool {
    (item.identification & ID_KNOWN2) != 0
}

/// C++ identification.cpp lines 389–391.
pub fn spell_item_remove_identification(item: &mut Inventory) {
    item.identification &= !ID_KNOWN2;
}

/// C++ identification.cpp lines 393–395.
pub fn item_identification_clear_empty(item: &mut Inventory) {
    item.identification &= !ID_EMPTY;
}

/// C++ identification.cpp lines 397–400.
pub fn item_identify_as_store_bought(item: &mut Inventory) {
    item.identification |= ID_STORE_BOUGHT;
    spell_item_identify_and_remove_random_inscription(item);
}

/// C++ identification.cpp lines 408–422.
pub fn item_set_colorless_as_identified_for_state(
    state: &State,
    category_id: u8,
    sub_category_id: u8,
    identification: u8,
) -> bool {
    let id = object_position_offset(category_id, sub_category_id);
    if id < 0 {
        return OD_KNOWN1 != 0;
    }
    if item_store_bought(identification) {
        return OD_KNOWN1 != 0;
    }

    let id = (id as usize) << 6;
    let id = id + usize::from(sub_category_id & (ITEM_SINGLE_STACK_MIN - 1));
    is_object_known(state, id)
}

pub fn item_set_colorless_as_identified(
    category_id: u8,
    sub_category_id: u8,
    identification: u8,
) -> bool {
    with_state(|state| {
        item_set_colorless_as_identified_for_state(
            state,
            category_id,
            sub_category_id,
            identification,
        )
    })
}

/// C++ identification.cpp lines 425–436.
pub fn item_set_as_tried(item: Inventory) {
    with_state_mut(|state| {
        let Some(id) = object_ident_index(item.category_id, item.sub_category_id) else {
            return;
        };
        set_object_tried_flag(state, id);
    });
}

/// Which item slot [`item_identify_for_slot`] reads/writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemIdentifySlot {
    Inventory,
    Treasure,
}

/// C++ identification.cpp lines 440–485 — explicit item reference (inventory or treasure).
pub fn item_identify_for_slot(
    state: &mut State,
    slot: ItemIdentifySlot,
    index: i32,
    item_id: &mut i32,
) -> bool {
    let item = match slot {
        ItemIdentifySlot::Inventory => state.py.inventory[index as usize],
        ItemIdentifySlot::Treasure => state.game.treasure.list[index as usize],
    };

    if inventory_item_is_cursed(item) {
        match slot {
            ItemIdentifySlot::Inventory => {
                item_append_to_inscription(&mut state.py.inventory[index as usize], ID_DAMD);
            }
            ItemIdentifySlot::Treasure => {
                item_append_to_inscription(&mut state.game.treasure.list[index as usize], ID_DAMD);
            }
        }
    }

    if item_set_colorless_as_identified_for_state(
        state,
        item.category_id,
        item.sub_category_id,
        item.identification,
    ) {
        return false;
    }

    if let Some(id) = object_ident_index(item.category_id, item.sub_category_id) {
        state.objects_identified[id] |= OD_KNOWN1;
        clear_object_tried_flag(state, id);
    }
    if !inventory_item_single_stackable(item) {
        return false;
    }

    let mut merged = false;
    let mut i = 0i32;
    while i < state.py.pack.unique_items as i32 {
        let t_ptr = state.py.inventory[i as usize];
        let matching_cat = t_ptr.category_id == item.category_id;
        let matching_sub_cat = t_ptr.sub_category_id == item.sub_category_id;
        let total_items_count = i32::from(t_ptr.items_count) + i32::from(item.items_count);

        if matching_cat && matching_sub_cat && i != *item_id && total_items_count < 256 {
            if *item_id > i {
                std::mem::swap(&mut *item_id, &mut i);
            }

            merged = true;

            let merge_from = i as usize;
            let merge_to = *item_id as usize;
            let add_count = state.py.inventory[merge_from].items_count;
            state.py.inventory[merge_to].items_count += add_count;
            state.py.pack.unique_items -= 1;

            let new_unique = state.py.pack.unique_items as usize;
            for j in merge_from..new_unique {
                state.py.inventory[j] = state.py.inventory[j + 1];
            }

            inventory_item_copy_to(OBJ_NOTHING as i16, &mut state.py.inventory[new_unique]);
        }
        i += 1;
    }
    merged
}

/// C++ identification.cpp lines 440–485.
pub fn item_identify(item_id: &mut i32) {
    let merged = with_state_mut(|state| {
        item_identify_for_slot(state, ItemIdentifySlot::Inventory, *item_id, item_id)
    });

    if merged {
        terminal::print_message(Some(
            "You combine similar objects from the shop and dungeon.",
        ));
    }
}

/// C++ identification.cpp lines 489–491.
pub fn item_remove_magic_naming(item: &mut Inventory) {
    item.special_name_id = SpecialNameIds::SN_NULL as u8;
}

#[path = "identification_desc.rs"]
mod identification_desc;
pub use identification_desc::{
    bow_damage_value, item_charges_remaining_description, item_description,
    item_description_for_state, item_inscribe, item_replace_inscription,
    item_type_remaining_count_description,
};

/// C++ identification.cpp lines 941–956.
pub fn object_blocked_by_monster(monster_id: i32) {
    let (lit, creature_id) = with_state(|state| {
        let monster = &state.monsters[monster_id as usize];
        (monster.lit, monster.creature_id)
    });
    let description = if lit {
        format!("The {}", CREATURES_LIST[creature_id as usize].name)
    } else {
        "Something".to_string()
    };
    terminal::print_message(Some(&format!("{description} is in your way!")));
}

/// C++ identification.cpp lines 932–934.
pub fn item_append_to_inscription(item: &mut Inventory, item_ident_type: u8) {
    item.identification |= item_ident_type;
}
