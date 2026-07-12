//! `ui_inventory` inventory/equipment screens & interaction.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use std::os::raw::c_char;

use umoria::config::treasure::flags::TR_CURSED;
use umoria::data_treasure::GAME_OBJECTS;
use umoria::game::{reset_for_new_game, with_state, with_state_mut};
use umoria::inventory::{inventory_item_copy_to, PlayerEquipment};
use umoria::treasure::{
    TV_AMULET, TV_ARROW, TV_BOLT, TV_BOOTS, TV_BOW, TV_CLOAK, TV_DIGGING, TV_FOOD, TV_GLOVES,
    TV_HAFTED, TV_HARD_ARMOR, TV_HELM, TV_LIGHT, TV_NOTHING, TV_POLEARM, TV_RING, TV_SCROLL1,
    TV_SHIELD, TV_SLING_AMMO, TV_SOFT_ARMOR, TV_SPIKE, TV_SWORD,
};
use umoria::types::Screen;
use umoria::ui_inventory::{
    apply_switch_screen_bottom_pos, build_command_heading, equipment_position_description,
    inventory_get_item_matching_inscription, inventory_get_slot_to_wear_equipment,
    inventory_item_weight_text, player_item_wearing_description, request_and_show_inventory_screen,
    switch_screen_line_pos, ui_command_inventory_drop_item, ui_command_inventory_take_off_item,
    ui_command_inventory_wear_wield_item, ui_command_switch_screen,
};
use umoria::ui_io::test_set_ncurses_stub;

fn set_item_weight(item_id: usize, weight: u16, count: u8) {
    with_state_mut(|s| {
        s.py.inventory[item_id].weight = weight;
        s.py.inventory[item_id].items_count = count;
    });
}

fn set_inscription(item_id: usize, ch: u8) {
    with_state_mut(|s| {
        s.py.inventory[item_id].inscription[0] = ch as c_char;
        s.py.inventory[item_id].inscription[1] = 0;
    });
}

// --------------------------------------------------------------------------
// Pure logic — inventoryItemWeightText
// --------------------------------------------------------------------------
#[test]
fn weight_text_123_total() {
    reset_for_new_game(None);
    set_item_weight(0, 41, 3); // 41*3=123
    assert_eq!(inventory_item_weight_text(0), " 12.3 lb");
}

#[test]
fn weight_text_5_total() {
    reset_for_new_game(None);
    set_item_weight(0, 5, 1);
    assert_eq!(inventory_item_weight_text(0), "  0.5 lb");
}

#[test]
fn weight_text_1000_total() {
    reset_for_new_game(None);
    set_item_weight(0, 100, 10);
    assert_eq!(inventory_item_weight_text(0), "100.0 lb");
}

#[test]
fn weight_text_zero_total() {
    reset_for_new_game(None);
    set_item_weight(0, 0, 5);
    assert_eq!(inventory_item_weight_text(0), "  0.0 lb");
}

// --------------------------------------------------------------------------
// equipmentPositionDescription
// --------------------------------------------------------------------------
#[test]
fn equipment_position_all_slots() {
    assert_eq!(
        equipment_position_description(PlayerEquipment::Head as u8, 0, 18),
        "On head"
    );
    assert_eq!(
        equipment_position_description(PlayerEquipment::Neck as u8, 0, 18),
        "Around neck"
    );
    assert_eq!(
        equipment_position_description(PlayerEquipment::Body as u8, 0, 18),
        "On body"
    );
    assert_eq!(
        equipment_position_description(PlayerEquipment::Arm as u8, 0, 18),
        "On arm"
    );
    assert_eq!(
        equipment_position_description(PlayerEquipment::Hands as u8, 0, 18),
        "On hands"
    );
    assert_eq!(
        equipment_position_description(PlayerEquipment::Right as u8, 0, 18),
        "On right hand"
    );
    assert_eq!(
        equipment_position_description(PlayerEquipment::Left as u8, 0, 18),
        "On left hand"
    );
    assert_eq!(
        equipment_position_description(PlayerEquipment::Feet as u8, 0, 18),
        "On feet"
    );
    assert_eq!(
        equipment_position_description(PlayerEquipment::Outer as u8, 0, 18),
        "About body"
    );
    assert_eq!(
        equipment_position_description(PlayerEquipment::Light as u8, 0, 18),
        "Light source"
    );
    assert_eq!(
        equipment_position_description(PlayerEquipment::Auxiliary as u8, 0, 18),
        "Spare weapon"
    );
}

#[test]
fn equipment_position_wield_just_lifting() {
    assert_eq!(
        equipment_position_description(PlayerEquipment::Wield as u8, 16 * 15, 15),
        "Just lifting"
    );
}

#[test]
fn equipment_position_wield_wielding_at_boundary() {
    assert_eq!(
        equipment_position_description(PlayerEquipment::Wield as u8, 15 * 15, 15),
        "Wielding"
    );
    assert_eq!(
        equipment_position_description(PlayerEquipment::Wield as u8, 100, 18),
        "Wielding"
    );
}

#[test]
fn equipment_position_unknown_id() {
    assert_eq!(
        equipment_position_description(21, 0, 18),
        "Unknown equipment position ID"
    );
}

// --------------------------------------------------------------------------
// playerItemWearingDescription
// --------------------------------------------------------------------------
#[test]
fn player_item_wearing_all_slots() {
    assert_eq!(
        player_item_wearing_description(PlayerEquipment::Wield as u8),
        "wielding"
    );
    assert_eq!(
        player_item_wearing_description(PlayerEquipment::Head as u8),
        "wearing on your head"
    );
    assert_eq!(
        player_item_wearing_description(PlayerEquipment::Neck as u8),
        "wearing around your neck"
    );
    assert_eq!(
        player_item_wearing_description(PlayerEquipment::Body as u8),
        "wearing on your body"
    );
    assert_eq!(
        player_item_wearing_description(PlayerEquipment::Arm as u8),
        "wearing on your arm"
    );
    assert_eq!(
        player_item_wearing_description(PlayerEquipment::Hands as u8),
        "wearing on your hands"
    );
    assert_eq!(
        player_item_wearing_description(PlayerEquipment::Right as u8),
        "wearing on your right hand"
    );
    assert_eq!(
        player_item_wearing_description(PlayerEquipment::Left as u8),
        "wearing on your left hand"
    );
    assert_eq!(
        player_item_wearing_description(PlayerEquipment::Feet as u8),
        "wearing on your feet"
    );
    assert_eq!(
        player_item_wearing_description(PlayerEquipment::Outer as u8),
        "wearing about your body"
    );
    assert_eq!(
        player_item_wearing_description(PlayerEquipment::Light as u8),
        "using to light the way"
    );
    assert_eq!(
        player_item_wearing_description(PlayerEquipment::Auxiliary as u8),
        "holding ready by your side"
    );
}

#[test]
fn player_item_wearing_default() {
    assert_eq!(player_item_wearing_description(21), "carrying in your pack");
}

// --------------------------------------------------------------------------
// buildCommandHeading
// --------------------------------------------------------------------------
#[test]
fn build_command_heading_blank_screen_list_suffix() {
    let s = build_command_heading(0, 2, "", b'w', "Wear/Wield", Screen::Blank);
    assert_eq!(
        s,
        "(a-c, * to list, 0-9, space to break, ESC to exit) Wear/Wield which one?"
    );
}

#[test]
fn build_command_heading_drop_with_digits_and_swap() {
    let s = build_command_heading(0, 4, ", / for Equip", b'd', "Drop", Screen::Inventory);
    assert_eq!(
        s,
        "(a-e, / for Equip, 0-9, space to break, ESC to exit) Drop which one?"
    );
}

#[test]
fn build_command_heading_take_off_no_digits() {
    let s = build_command_heading(0, 1, ", / for Inven", b'r', "Throw off", Screen::Equipment);
    assert_eq!(
        s,
        "(a-b, / for Inven, space to break, ESC to exit) Throw off which one?"
    );
}

// --------------------------------------------------------------------------
// inventoryGetItemMatchingInscription
// --------------------------------------------------------------------------
#[test]
fn inscription_match_digit_in_range() {
    reset_for_new_game(None);
    set_inscription(2, b'5');
    assert_eq!(inventory_get_item_matching_inscription(b'5', b'w', 0, 5), 2);
}

#[test]
fn inscription_match_digit_not_found() {
    reset_for_new_game(None);
    assert_eq!(
        inventory_get_item_matching_inscription(b'3', b'w', 0, 5),
        -1
    );
}

#[test]
fn inscription_match_digit_skipped_for_r_command() {
    reset_for_new_game(None);
    set_inscription(2, b'5');
    // command 'r' falls through to which-'a' branch: '5'-'a' = negative / wrong
    assert_eq!(
        inventory_get_item_matching_inscription(b'5', b'r', 0, 5),
        -44
    );
}

#[test]
fn inscription_match_digit_skipped_for_t_command() {
    reset_for_new_game(None);
    set_inscription(2, b'5');
    assert_eq!(
        inventory_get_item_matching_inscription(b'5', b't', 0, 5),
        -44
    );
}

#[test]
fn inscription_match_uppercase_a() {
    assert_eq!(inventory_get_item_matching_inscription(b'A', b'w', 0, 5), 0);
}

#[test]
fn inscription_match_lowercase_c() {
    assert_eq!(inventory_get_item_matching_inscription(b'c', b'd', 0, 5), 2);
}

// --------------------------------------------------------------------------
// inventoryGetSlotToWearEquipment non-ring
// --------------------------------------------------------------------------
#[test]
fn wear_slot_weapons_to_wield() {
    for cat in [
        TV_SLING_AMMO,
        TV_BOLT,
        TV_ARROW,
        TV_BOW,
        TV_HAFTED,
        TV_POLEARM,
        TV_SWORD,
        TV_DIGGING,
        TV_SPIKE,
    ] {
        assert_eq!(
            inventory_get_slot_to_wear_equipment(cat),
            PlayerEquipment::Wield as i32
        );
    }
}

#[test]
fn wear_slot_armor_pieces() {
    assert_eq!(
        inventory_get_slot_to_wear_equipment(TV_LIGHT),
        PlayerEquipment::Light as i32
    );
    assert_eq!(
        inventory_get_slot_to_wear_equipment(TV_BOOTS),
        PlayerEquipment::Feet as i32
    );
    assert_eq!(
        inventory_get_slot_to_wear_equipment(TV_GLOVES),
        PlayerEquipment::Hands as i32
    );
    assert_eq!(
        inventory_get_slot_to_wear_equipment(TV_CLOAK),
        PlayerEquipment::Outer as i32
    );
    assert_eq!(
        inventory_get_slot_to_wear_equipment(TV_HELM),
        PlayerEquipment::Head as i32
    );
    assert_eq!(
        inventory_get_slot_to_wear_equipment(TV_SHIELD),
        PlayerEquipment::Arm as i32
    );
    assert_eq!(
        inventory_get_slot_to_wear_equipment(TV_HARD_ARMOR),
        PlayerEquipment::Body as i32
    );
    assert_eq!(
        inventory_get_slot_to_wear_equipment(TV_SOFT_ARMOR),
        PlayerEquipment::Body as i32
    );
    assert_eq!(
        inventory_get_slot_to_wear_equipment(TV_AMULET),
        PlayerEquipment::Neck as i32
    );
}

#[test]
fn wear_slot_unknown_category() {
    assert_eq!(inventory_get_slot_to_wear_equipment(TV_NOTHING), -1);
}

#[test]
fn wear_slot_ring_path() {
    // empty right hand → Right; empty left (right occupied) → Left.
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Right as usize].category_id = TV_NOTHING;
        s.py.inventory[PlayerEquipment::Left as usize].category_id = TV_NOTHING;
    });
    assert_eq!(
        inventory_get_slot_to_wear_equipment(TV_RING),
        PlayerEquipment::Right as i32
    );
    with_state_mut(|s| {
        s.py.inventory[PlayerEquipment::Right as usize].category_id = TV_RING;
        s.py.inventory[PlayerEquipment::Left as usize].category_id = TV_NOTHING;
    });
    assert_eq!(
        inventory_get_slot_to_wear_equipment(TV_RING),
        PlayerEquipment::Left as i32
    );
    test_set_ncurses_stub(false);
}

// --------------------------------------------------------------------------
// switch_screen_line_pos + bottom pos
// --------------------------------------------------------------------------
#[test]
fn switch_screen_line_pos_branches() {
    assert_eq!(switch_screen_line_pos(Screen::Blank, 5, 0, 2, 3), 0);
    assert_eq!(switch_screen_line_pos(Screen::Wrong, 5, 0, 2, 3), 0);
    assert_eq!(switch_screen_line_pos(Screen::Help, 5, 0, 2, 3), 7);
    assert_eq!(switch_screen_line_pos(Screen::Inventory, 5, 0, 2, 3), 5);
    assert_eq!(switch_screen_line_pos(Screen::Wear, 5, 1, 3, 3), 3); // 3-1+1
    assert_eq!(switch_screen_line_pos(Screen::Equipment, 5, 0, 2, 4), 4);
}

#[test]
fn apply_switch_screen_bottom_extend() {
    // currentLinePos >= screen_bottom_pos → bottom = currentLinePos+1
    assert_eq!(apply_switch_screen_bottom_pos(5, 3), (6, true));
}

#[test]
fn apply_switch_screen_bottom_shrink() {
    // currentLinePos < screen_bottom_pos → erase from currentLinePos+1 .. bottom
    assert_eq!(apply_switch_screen_bottom_pos(2, 5), (5, false));
}

// --------------------------------------------------------------------------
// uiCommandSwitchScreen state
// --------------------------------------------------------------------------
#[test]
fn ui_command_switch_screen_noop_same_screen() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.game.screen.current_screen_id = Screen::Help;
        s.game.screen.screen_bottom_pos = 8;
    });
    ui_command_switch_screen(Screen::Help);
    with_state(|s| {
        assert_eq!(s.game.screen.current_screen_id, Screen::Help);
        assert_eq!(s.game.screen.screen_bottom_pos, 8);
    });
}

#[test]
fn ui_command_switch_screen_help_sets_bottom() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.game.screen.current_screen_id = Screen::Blank;
        s.game.screen.screen_left_pos = 50;
        s.game.screen.screen_bottom_pos = 3;
    });
    ui_command_switch_screen(Screen::Help);
    with_state(|s| {
        assert_eq!(s.game.screen.current_screen_id, Screen::Help);
        assert_eq!(s.game.screen.screen_bottom_pos, 8); // 7+1
    });
    test_set_ncurses_stub(false);
}

// --------------------------------------------------------------------------
// requestAndShowInventoryScreen
// --------------------------------------------------------------------------
#[test]
fn request_inventory_screen_fresh_start() {
    reset_for_new_game(None);
    with_state_mut(|s| s.game.doing_inventory_command = 0);
    request_and_show_inventory_screen(false);
    with_state(|s| {
        assert_eq!(s.game.screen.screen_left_pos, 50);
        assert_eq!(s.game.screen.screen_bottom_pos, 0);
        assert_eq!(s.game.screen.current_screen_id, Screen::Blank);
    });
}

#[test]
fn request_inventory_screen_resume_wrong_trick() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.game.doing_inventory_command = b'?';
        s.game.screen.current_screen_id = Screen::Help;
        s.game.screen.screen_left_pos = 40;
        s.game.screen.screen_bottom_pos = 5;
    });
    request_and_show_inventory_screen(false);
    with_state(|s| {
        assert_eq!(s.game.screen.current_screen_id, Screen::Help);
    });
    test_set_ncurses_stub(false);
}

#[test]
fn request_inventory_screen_changed_recover_aborts() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.game.doing_inventory_command = b'w';
        s.game.screen.current_screen_id = Screen::Wear;
    });
    with_state_mut(|s| s.screen_has_changed = true);
    request_and_show_inventory_screen(true);
    with_state(|s| {
        assert_eq!(s.game.doing_inventory_command, 0);
    });
}

// --------------------------------------------------------------------------
// Command helpers
// --------------------------------------------------------------------------
#[test]
fn take_off_no_equipment() {
    reset_for_new_game(None);
    assert!(!ui_command_inventory_take_off_item(false));
}

#[test]
fn take_off_pack_full_blocks() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.equipment_count = 1;
        s.py.pack.unique_items = PlayerEquipment::Wield as i16;
    });
    with_state_mut(|s| s.game.doing_inventory_command = 0);
    assert!(!ui_command_inventory_take_off_item(false));
}

#[test]
fn take_off_switches_to_equipment() {
    // only switch when current != Blank.
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    with_state_mut(|s| s.py.equipment_count = 1);
    with_state_mut(|s| {
        s.game.doing_inventory_command = b't';
        s.game.screen.current_screen_id = Screen::Blank;
    });
    let selecting = ui_command_inventory_take_off_item(false);
    assert!(selecting);
    with_state(|s| assert_eq!(s.game.screen.current_screen_id, Screen::Blank));

    with_state_mut(|s| s.game.screen.current_screen_id = Screen::Inventory);
    let selecting = ui_command_inventory_take_off_item(false);
    assert!(selecting);
    with_state(|s| assert_eq!(s.game.screen.current_screen_id, Screen::Equipment));
    test_set_ncurses_stub(false);
}

#[test]
fn drop_item_empty() {
    reset_for_new_game(None);
    let mut command = b'd';
    assert!(!ui_command_inventory_drop_item(&mut command, false));
}

#[test]
fn drop_item_floor_treasure_blocks() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.pack.unique_items = 1;
        s.py.pos.y = 0;
        s.py.pos.x = 0;
        s.dg.floor[0][0].treasure_id = 1;
    });
    let mut command = b'd';
    assert!(!ui_command_inventory_drop_item(&mut command, false));
}

#[test]
fn drop_item_remaps_to_equipment() {
    reset_for_new_game(None);
    test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.pack.unique_items = 0;
        s.py.equipment_count = 1;
        s.py.inventory[PlayerEquipment::Wield as usize].category_id = TV_SWORD;
        s.py.pos.y = 1;
        s.py.pos.x = 1;
        s.dg.floor[1][1].treasure_id = 0;
    });
    with_state_mut(|s| s.game.screen.current_screen_id = Screen::Equipment);
    let mut command = b'd';
    let selecting = ui_command_inventory_drop_item(&mut command, false);
    assert!(selecting);
    assert_eq!(command, b'r');
    with_state(|s| assert_eq!(s.game.screen.current_screen_id, Screen::Equipment));
    test_set_ncurses_stub(false);
}

#[test]
fn wear_wield_scans_range() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.pack.unique_items = 3;
        s.py.inventory[0].category_id = TV_FOOD;
        s.py.inventory[1].category_id = TV_SWORD;
        s.py.inventory[2].category_id = TV_FOOD;
    });
    let selecting = ui_command_inventory_wear_wield_item(false);
    assert!(selecting);
    with_state(|s| {
        assert_eq!(s.game.screen.wear_low_id, 1);
        assert_eq!(s.game.screen.wear_high_id, 2);
    });
}

#[test]
fn wear_wield_nothing_message() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.pack.unique_items = 2;
        s.py.inventory[0].category_id = TV_FOOD;
        s.py.inventory[1].category_id = TV_SCROLL1;
    });
    assert!(!ui_command_inventory_wear_wield_item(false));
}

// --------------------------------------------------------------------------
// RefCell nest regression — must panic on old nested-borrow code
// --------------------------------------------------------------------------

fn push_keys_in_consume_order(keys: &[i32]) {
    let mut reversed = keys.to_vec();
    reversed.reverse();
    umoria::ui_io::test_push_getch_keys(&reversed);
}

fn make_pack_food() {
    let id = GAME_OBJECTS
        .iter()
        .position(|o| o.category_id == TV_FOOD)
        .expect("food object") as i16;
    with_state_mut(|s| {
        inventory_item_copy_to(id, &mut s.py.inventory[0]);
        s.py.inventory[0].items_count = 1;
        s.py.pack.unique_items = 1;
    });
}

fn make_wielded_sword(cursed: bool) {
    let id = GAME_OBJECTS
        .iter()
        .position(|o| o.category_id == TV_SWORD)
        .expect("sword object") as i16;
    with_state_mut(|s| {
        let slot = PlayerEquipment::Wield as usize;
        inventory_item_copy_to(id, &mut s.py.inventory[slot]);
        s.py.inventory[slot].items_count = 1;
        if cursed {
            s.py.inventory[slot].flags |= TR_CURSED;
        }
        s.py.equipment_count = 1;
    });
}

#[test]
fn display_equipment_with_real_item_does_not_panic() {
    reset_for_new_game(Some(1));
    test_set_ncurses_stub(true);
    make_wielded_sword(false);
    let _ = umoria::ui_inventory::display_equipment(true, 50);
    test_set_ncurses_stub(false);
}

#[test]
fn inventory_execute_equipment_screen_does_not_panic() {
    reset_for_new_game(Some(1));
    test_set_ncurses_stub(true);
    umoria::ui_io::test_clear_getch_keys();
    make_wielded_sword(false);
    push_keys_in_consume_order(&[i32::from(umoria::ui_io::ESCAPE)]);
    umoria::ui_inventory::inventory_execute_command(b'e');
    test_set_ncurses_stub(false);
}

#[test]
fn inventory_execute_inventory_screen_does_not_panic() {
    reset_for_new_game(Some(1));
    test_set_ncurses_stub(true);
    umoria::ui_io::test_clear_getch_keys();
    make_pack_food();
    push_keys_in_consume_order(&[i32::from(umoria::ui_io::ESCAPE)]);
    umoria::ui_inventory::inventory_execute_command(b'i');
    test_set_ncurses_stub(false);
}

#[test]
fn inventory_drop_uppercase_verify_does_not_panic() {
    reset_for_new_game(Some(1));
    test_set_ncurses_stub(true);
    umoria::ui_io::test_clear_getch_keys();
    make_pack_food();
    with_state_mut(|s| {
        s.py.pos.y = 5;
        s.py.pos.x = 5;
        s.dg.floor[5][5].treasure_id = 0;
    });
    // d → select A (uppercase verify) → n (decline) → ESC
    push_keys_in_consume_order(&[
        i32::from(umoria::ui_io::ESCAPE),
        i32::from(b'n'),
        i32::from(b'A'),
    ]);
    umoria::ui_inventory::inventory_execute_command(b'd');
    test_set_ncurses_stub(false);
}

#[test]
fn inventory_drop_stack_confirm_does_not_panic() {
    reset_for_new_game(Some(1));
    test_set_ncurses_stub(true);
    umoria::ui_io::test_clear_getch_keys();
    make_pack_food();
    with_state_mut(|s| {
        s.py.inventory[0].items_count = 3;
        s.py.pos.y = 5;
        s.py.pos.x = 5;
        s.dg.floor[5][5].treasure_id = 0;
    });
    // d → a → n (don't drop all / abort path) → ESC
    push_keys_in_consume_order(&[
        i32::from(umoria::ui_io::ESCAPE),
        i32::from(b'n'),
        i32::from(b'a'),
    ]);
    umoria::ui_inventory::inventory_execute_command(b'd');
    test_set_ncurses_stub(false);
}

#[test]
fn inventory_unwield_cursed_does_not_panic() {
    reset_for_new_game(Some(1));
    test_set_ncurses_stub(true);
    umoria::ui_io::test_clear_getch_keys();
    make_wielded_sword(true);
    push_keys_in_consume_order(&[i32::from(umoria::ui_io::ESCAPE)]);
    umoria::ui_inventory::inventory_execute_command(b'x');
    test_set_ncurses_stub(false);
}

#[test]
fn inventory_wear_cursed_slot_message_does_not_panic() {
    reset_for_new_game(Some(1));
    test_set_ncurses_stub(true);
    umoria::ui_io::test_clear_getch_keys();
    let sword = GAME_OBJECTS
        .iter()
        .position(|o| o.category_id == TV_SWORD)
        .expect("sword") as i16;
    with_state_mut(|s| {
        inventory_item_copy_to(sword, &mut s.py.inventory[0]);
        s.py.inventory[0].items_count = 1;
        s.py.pack.unique_items = 1;
        let slot = PlayerEquipment::Wield as usize;
        inventory_item_copy_to(sword, &mut s.py.inventory[slot]);
        s.py.inventory[slot].items_count = 1;
        s.py.inventory[slot].flags |= TR_CURSED;
        s.py.equipment_count = 1;
    });
    // w → a (try wear over cursed) → ESC
    push_keys_in_consume_order(&[i32::from(umoria::ui_io::ESCAPE), i32::from(b'a')]);
    umoria::ui_inventory::inventory_execute_command(b'w');
    test_set_ncurses_stub(false);
}
