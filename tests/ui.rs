//! `ui` viewport, status line & character screens.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::config::player::PLAYER_MAX_EXP;
use umoria::config::spells::{NAME_OFFSET_PRAYERS, NAME_OFFSET_SPELLS, SPELL_TYPE_MAGE};
use umoria::data_player::{CLASSES, CLASS_LEVEL_ADJ, MAGIC_SPELLS, SPELL_NAMES};
use umoria::game::{reset_for_new_game, with_state, with_state_mut};
use umoria::player::{
    PlayerAttr, PlayerClassLevelAdj, BTH_PER_PLUS_TO_HIT_ADJUST, PLAYER_MAX_LEVEL,
};
use umoria::spells::spell_chance_of_success;
use umoria::types::Coord_t;
use umoria::ui::{
    blank_string_tail, compute_ability_values, compute_panel_change, coord_inside_panel_bounds,
    coord_outside_panel, count_experience_level_ups, display_character_experience,
    experience_exp_halving, format_character_current_depth, format_exp_to_advance_line,
    format_header_long_number, format_header_long_number7_spaces, format_header_number,
    format_long_number, format_number, format_spell_comment, format_spell_row,
    movement_state_string, panel_bounds_fields, simulate_exp_clamp, speed_display_string,
    stat_rating, stats_as_string, BLANK_LENGTH,
};

// --------------------------------------------------------------------------
// A1 — statsAsString
// --------------------------------------------------------------------------
#[test]
fn a1_stats_as_string_at_or_below_18() {
    assert_eq!(stats_as_string(3), "     3");
    assert_eq!(stats_as_string(18), "    18");
}

#[test]
fn a1_stats_as_string_percentile_cases() {
    assert_eq!(stats_as_string(118), "18/100");
    assert_eq!(stats_as_string(19), " 18/01");
    assert_eq!(stats_as_string(117), " 18/99");
}

// --------------------------------------------------------------------------
// A2 — statRating
// --------------------------------------------------------------------------
#[test]
fn a2_stat_rating_buckets() {
    assert_eq!(stat_rating(Coord_t { x: -3, y: 1 }), "Very Bad");
    assert_eq!(stat_rating(Coord_t { x: -2, y: 1 }), "Very Bad");
    assert_eq!(stat_rating(Coord_t { x: -1, y: 1 }), "Very Bad");
    assert_eq!(stat_rating(Coord_t { x: 0, y: 1 }), "Bad");
    assert_eq!(stat_rating(Coord_t { x: 1, y: 1 }), "Bad");
    assert_eq!(stat_rating(Coord_t { x: 2, y: 1 }), "Poor");
    assert_eq!(stat_rating(Coord_t { x: 3, y: 1 }), "Fair");
    assert_eq!(stat_rating(Coord_t { x: 4, y: 1 }), "Fair");
    assert_eq!(stat_rating(Coord_t { x: 5, y: 1 }), "Good");
    assert_eq!(stat_rating(Coord_t { x: 6, y: 1 }), "Very Good");
    assert_eq!(stat_rating(Coord_t { x: 7, y: 1 }), "Excellent");
    assert_eq!(stat_rating(Coord_t { x: 8, y: 1 }), "Excellent");
    assert_eq!(stat_rating(Coord_t { x: 9, y: 1 }), "Superb");
    assert_eq!(stat_rating(Coord_t { x: 100, y: 1 }), "Superb");
}

#[test]
fn a2_stat_rating_expected_truncation_toward_zero() {
    assert_eq!(stat_rating(Coord_t { x: 12, y: -5 }), "Very Bad");
    assert_eq!(stat_rating(Coord_t { x: -1, y: 12 }), "Bad");
}

// --------------------------------------------------------------------------
// A3 — printCharacterAbilities integer math
// --------------------------------------------------------------------------
#[test]
fn a3_ability_computations_with_zero_adjustments() {
    let misc = umoria::player::PlayerMisc {
        bth: 12,
        bth_with_bows: 8,
        plusses_to_hit: 2,
        fos: 15,
        chance_in_search: 18,
        stealth_factor: 4,
        disarm: 10,
        saving_throw: 20,
        class_id: 0,
        level: 6,
        ..Default::default()
    };
    let (xbth, xbthb, xfos, xsrh, xstl, xdis, xsave, xdev, xinfra) =
        compute_ability_values(&misc, 0, 0, 0, 0);

    let level = i32::from(misc.level);
    let class = misc.class_id as usize;
    assert_eq!(
        xbth,
        misc.bth as i32
            + misc.plusses_to_hit as i32 * i32::from(BTH_PER_PLUS_TO_HIT_ADJUST)
            + i32::from(CLASS_LEVEL_ADJ[class][PlayerClassLevelAdj::BTH as usize]) * level
    );
    assert_eq!(
        xbthb,
        misc.bth_with_bows as i32
            + misc.plusses_to_hit as i32 * i32::from(BTH_PER_PLUS_TO_HIT_ADJUST)
            + i32::from(CLASS_LEVEL_ADJ[class][PlayerClassLevelAdj::BTHB as usize]) * level
    );
    assert_eq!(xfos, 40 - misc.fos as i32);
    assert_eq!(xsrh, misc.chance_in_search as i32);
    assert_eq!(xstl, misc.stealth_factor as i32 + 1);
    assert_eq!(
        xdis,
        misc.disarm as i32
            + i32::from(CLASS_LEVEL_ADJ[class][PlayerClassLevelAdj::DISARM as usize]) * level / 3
    );
    assert_eq!(
        xsave,
        misc.saving_throw as i32
            + i32::from(CLASS_LEVEL_ADJ[class][PlayerClassLevelAdj::SAVE as usize]) * level / 3
    );
    assert_eq!(
        xdev,
        misc.saving_throw as i32
            + i32::from(CLASS_LEVEL_ADJ[class][PlayerClassLevelAdj::DEVICE as usize]) * level / 3
    );
    assert_eq!(xinfra, format!("{} feet", 0));
    let _ = xinfra;
}

#[test]
fn a3_ability_computations_clamp_fos_and_infra() {
    let misc = umoria::player::PlayerMisc {
        fos: 50,
        ..Default::default()
    };
    let (_, _, xfos, _, _, _, _, _, xinfra) = compute_ability_values(&misc, 5, 3, 4, 4);
    assert_eq!(xfos, 0);
    assert_eq!(xinfra, "40 feet");
}

// --------------------------------------------------------------------------
// A4 — panelBounds + coordOutsidePanel + coordInsidePanel
// --------------------------------------------------------------------------
#[test]
fn a4_panel_bounds_from_row_col() {
    let fields = panel_bounds_fields(2, 3);
    assert_eq!(fields.top, 22);
    assert_eq!(fields.bottom, 43);
    assert_eq!(fields.row_prt, 21);
    assert_eq!(fields.left, 99);
    assert_eq!(fields.right, 164);
    assert_eq!(fields.col_prt, 86);
}

#[test]
fn a4_coord_outside_panel_recalculation_and_clamps() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.dg.panel.row = 0;
        s.dg.panel.col = 0;
        s.dg.panel.max_rows = 2;
        s.dg.panel.max_cols = 2;
        let fields = panel_bounds_fields(0, 0);
        s.dg.panel.top = fields.top;
        s.dg.panel.bottom = fields.bottom;
        s.dg.panel.left = fields.left;
        s.dg.panel.right = fields.right;
        s.dg.panel.row_prt = fields.row_prt;
        s.dg.panel.col_prt = fields.col_prt;
    });

    let panel = with_state(|s| s.dg.panel);
    let changed = compute_panel_change(&panel, Coord_t { y: 30, x: 70 }, false);
    assert_eq!(changed, Some((2, 1)));

    let unchanged_force = compute_panel_change(&panel, Coord_t { y: 0, x: 0 }, true);
    assert_eq!(unchanged_force, None);

    let inside = with_state_mut(|s| {
        s.dg.panel.row = 1;
        s.dg.panel.col = 1;
        let fields = panel_bounds_fields(1, 1);
        s.dg.panel.top = fields.top;
        s.dg.panel.bottom = fields.bottom;
        s.dg.panel.left = fields.left;
        s.dg.panel.right = fields.right;
        s.dg.panel.row_prt = fields.row_prt;
        s.dg.panel.col_prt = fields.col_prt;
        coord_inside_panel_bounds(
            &s.dg.panel,
            Coord_t {
                y: s.dg.panel.top,
                x: s.dg.panel.left,
            },
        )
    });
    assert!(inside);
}

#[test]
fn a4_coord_inside_panel_boundary_equalities() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.dg.panel.row = 0;
        s.dg.panel.col = 0;
        let fields = panel_bounds_fields(0, 0);
        s.dg.panel.top = fields.top;
        s.dg.panel.bottom = fields.bottom;
        s.dg.panel.left = fields.left;
        s.dg.panel.right = fields.right;
        let top = fields.top;
        let bottom = fields.bottom;
        let left = fields.left;
        let right = fields.right;
        assert!(coord_inside_panel_bounds(
            &s.dg.panel,
            Coord_t { y: top, x: left }
        ));
        assert!(coord_inside_panel_bounds(
            &s.dg.panel,
            Coord_t {
                y: bottom,
                x: right
            }
        ));
        assert!(!coord_inside_panel_bounds(
            &s.dg.panel,
            Coord_t {
                y: top - 1,
                x: left
            }
        ));
        assert!(!coord_inside_panel_bounds(
            &s.dg.panel,
            Coord_t {
                y: top,
                x: right + 1
            }
        ));
    });
}

#[test]
fn a4_coord_outside_panel_find_bound_calls_player_end_running() {
    reset_for_new_game(None);
    umoria::ui_io::test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.dg.height = 66;
        s.dg.width = 198;
        s.dg.panel.row = 0;
        s.dg.panel.col = 0;
        s.dg.panel.max_rows = 2;
        s.dg.panel.max_cols = 2;
        let fields = panel_bounds_fields(0, 0);
        s.dg.panel.top = fields.top;
        s.dg.panel.bottom = fields.bottom;
        s.dg.panel.left = fields.left;
        s.dg.panel.right = fields.right;
        s.dg.panel.row_prt = fields.row_prt;
        s.dg.panel.col_prt = fields.col_prt;
        s.options.find_bound = true;
        s.py.running_tracker = 5;
        s.py.pos = Coord_t { y: 5, x: 5 };
    });

    assert!(coord_outside_panel(Coord_t { y: 30, x: 70 }, true));
    with_state(|s| assert_eq!(s.py.running_tracker, 0));

    with_state_mut(|s| {
        s.dg.panel.row = 0;
        s.dg.panel.col = 0;
        let fields = panel_bounds_fields(0, 0);
        s.dg.panel.top = fields.top;
        s.dg.panel.bottom = fields.bottom;
        s.dg.panel.left = fields.left;
        s.dg.panel.right = fields.right;
        s.dg.panel.row_prt = fields.row_prt;
        s.dg.panel.col_prt = fields.col_prt;
        s.options.find_bound = false;
        s.py.running_tracker = 5;
    });
    assert!(coord_outside_panel(Coord_t { y: 30, x: 70 }, true));
    with_state(|s| assert_eq!(s.py.running_tracker, 5));
    umoria::ui_io::test_set_ncurses_stub(false);
}

// --------------------------------------------------------------------------
// A5 — printCharacterCurrentDepth formatting
// --------------------------------------------------------------------------
#[test]
fn a5_character_current_depth_strings() {
    assert_eq!(format_character_current_depth(0), "Town level");
    assert_eq!(format_character_current_depth(3), "150 feet");
}

// --------------------------------------------------------------------------
// A6 — printCharacterMovementState
// --------------------------------------------------------------------------
#[test]
fn a6_movement_state_paralysis_and_rest() {
    use umoria::config::player::status::{PY_REPEAT, PY_REST};

    let (s, st) = movement_state_string(2, 0, 0, 0, false);
    assert_eq!(s, "Paralysed");
    assert_eq!(st & PY_REPEAT, 0);

    let (s, _) = movement_state_string(0, PY_REST, -1, 0, false);
    assert_eq!(s, "Rest *");

    let (s, _) = movement_state_string(0, PY_REST, 42, 0, true);
    assert_eq!(s, "Rest 42   ");

    let (s, _) = movement_state_string(0, PY_REST, 42, 0, false);
    assert_eq!(s, "Rest");
}

#[test]
fn a6_movement_state_repeat_search_and_blank() {
    use umoria::config::player::status::{PY_REPEAT, PY_SEARCH};

    let (s, st) = movement_state_string(0, 0, 0, 7, true);
    assert_eq!(s, "Repeat 007");
    assert_eq!(st & PY_REPEAT, PY_REPEAT);

    let (s, _) = movement_state_string(0, PY_SEARCH, 0, 7, true);
    assert_eq!(s, "Search");

    let (s, _) = movement_state_string(0, PY_SEARCH, 0, 0, false);
    assert_eq!(s, "Searching");

    let (s, _) = movement_state_string(0, 0, 0, 0, false);
    assert_eq!(s, blank_string_tail(10));
}

// --------------------------------------------------------------------------
// A7 — printCharacterSpeed
// --------------------------------------------------------------------------
#[test]
fn a7_speed_display_buckets() {
    assert_eq!(speed_display_string(3, false), "Very Slow");
    assert_eq!(speed_display_string(1, false), "Slow     ");
    assert_eq!(speed_display_string(0, false), blank_string_tail(9));
    assert_eq!(speed_display_string(-1, false), "Fast     ");
    assert_eq!(speed_display_string(-3, false), "Very Fast");
    assert_eq!(speed_display_string(0, true), "Fast     ");
}

// --------------------------------------------------------------------------
// A8 — printCharacterLevelExperience exp-to-advance
// --------------------------------------------------------------------------
fn sample_base_exp_levels() -> [u32; PLAYER_MAX_LEVEL as usize] {
    [
        10, 25, 45, 70, 100, 140, 200, 280, 380, 500, 650, 850, 1100, 1400, 1800, 2300, 2900, 3600,
        4400, 5400, 6800, 8400, 10200, 12500, 17500, 25000, 35000, 50000, 75000, 100000, 150000,
        200000, 300000, 400000, 500000, 750000, 1500000, 2500000, 5000000, 10000000,
    ]
}

#[test]
fn a8_exp_to_advance_formatting() {
    let levels = sample_base_exp_levels();
    assert_eq!(
        format_exp_to_advance_line(PLAYER_MAX_LEVEL, &levels, 100),
        "Exp to Adv.: *******"
    );
    assert_eq!(
        format_exp_to_advance_line(2, &levels, 100),
        format_header_long_number7_spaces("Exp to Adv.", 25)
    );
}

// --------------------------------------------------------------------------
// A9 — displayCharacterExperience clamp + loop
// --------------------------------------------------------------------------
#[test]
fn a9_experience_clamp_and_level_up_count() {
    let levels = sample_base_exp_levels();
    assert_eq!(simulate_exp_clamp(20_000_000), PLAYER_MAX_EXP);

    let (level, exp, max_exp, gains) = count_experience_level_ups(1, 100, 0, &levels, 100);
    assert_eq!(gains, 3);
    assert_eq!(level, 4);
    assert!(exp <= levels[3] as i32);
    assert_eq!(max_exp, exp);
}

// --------------------------------------------------------------------------
// A10 — playerGainLevel exp halving
// --------------------------------------------------------------------------
#[test]
fn a10_player_gain_level_exp_halving_math() {
    let levels = sample_base_exp_levels();
    let (new_level, new_exp) = experience_exp_halving(1, 100, &levels, 100);
    assert_eq!(new_level, 2);
    assert_eq!(new_exp, 62);
}

#[test]
fn a10_player_gain_level_full_side_effects() {
    reset_for_new_game(None);
    umoria::ui_io::test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.misc.class_id = 1; // Mage — triggers INT spell/mana recalculation
        s.py.misc.level = 1;
        s.py.misc.exp = 100;
        s.py.misc.max_exp = 0;
        s.py.misc.experience_factor = 100;
        s.py.base_exp_levels.fill(10_000);
        s.py.base_exp_levels[0] = 10;
        s.py.misc.max_hp = 10;
        s.py.misc.current_hp = 10;
        s.py.misc.current_hp_fraction = 0;
        s.py.base_hp_levels[0] = 8;
        s.py.base_hp_levels[1] = 16;
        s.py.stats.used[PlayerAttr::A_CON as usize] = 10;
        s.py.stats.used[PlayerAttr::A_INT as usize] = 18;
        s.py.flags.spells_learnt = 1;
        s.py.misc.mana = 0;
        s.py.misc.current_mana = 0;
        s.py.misc.current_mana_fraction = 0;
    });

    let hp_before = with_state(|s| s.py.misc.max_hp);
    display_character_experience();

    with_state(|s| {
        assert!(s.py.misc.level >= 2, "level should advance");
        assert_ne!(
            s.py.misc.max_hp, hp_before,
            "player_calculate_hit_points should refresh max_hp"
        );
        assert!(
            s.py.misc.mana > 0,
            "mage level-up should call player_gain_mana"
        );
        assert!(s.py.misc.max_exp >= s.py.misc.exp);
    });
    umoria::ui_io::test_set_ncurses_stub(false);
}

// --------------------------------------------------------------------------
// A11 — displaySpellsList row formatting
// --------------------------------------------------------------------------
#[test]
fn a11_spell_comment_selection() {
    assert_eq!(format_spell_comment(false, 0, 0, 0, 0), "");
    assert_eq!(format_spell_comment(true, 1 << 3, 0, 0, 3), " forgotten");
    assert_eq!(format_spell_comment(true, 0, 0, 1 << 2, 2), " unknown");
    assert_eq!(format_spell_comment(true, 0, 1 << 2, 0, 2), " untried");
    assert_eq!(format_spell_comment(true, 0, 1 << 2, 1 << 2, 2), "");
}

#[test]
fn a11_spell_row_format_skeleton() {
    let spell = MAGIC_SPELLS[0][0];
    let row = format_spell_row(
        'a',
        SPELL_NAMES[0],
        spell.level_required,
        spell.mana_required,
        22,
        "",
    );
    assert_eq!(
        row,
        format!(
            "  a) {:<30}{:2} {:4} {:3}%{}",
            SPELL_NAMES[0], spell.level_required, spell.mana_required, 22, ""
        )
    );
}

#[test]
fn a11_spell_list_column_and_offset_selection() {
    reset_for_new_game(None);
    with_state_mut(|s| s.py.misc.class_id = 1);
    let class = &CLASSES[1];
    assert_eq!(class.class_to_use_mage_spells, SPELL_TYPE_MAGE);
    let offset = if class.class_to_use_mage_spells == SPELL_TYPE_MAGE {
        NAME_OFFSET_SPELLS
    } else {
        NAME_OFFSET_PRAYERS
    };
    assert_eq!(offset, NAME_OFFSET_SPELLS);
    let _ = (22usize, 31usize);
}

#[test]
fn a11_display_spells_list_full_string_with_fail_chance() {
    reset_for_new_game(None);
    with_state_mut(|s| {
        s.py.misc.class_id = 1;
        s.py.misc.level = 10;
        s.py.stats.used[PlayerAttr::A_INT as usize] = 18;
        s.py.misc.current_mana = 100;
    });
    let chance = spell_chance_of_success(0);
    assert_eq!(chance, 5);
    let spell = MAGIC_SPELLS[0][0];
    let row = format_spell_row(
        'a',
        SPELL_NAMES[0],
        spell.level_required,
        spell.mana_required,
        chance,
        "",
    );
    assert_eq!(
        row,
        format!(
            "  a) {:<30}{:2} {:4} {:3}%{}",
            SPELL_NAMES[0], spell.level_required, spell.mana_required, chance, ""
        )
    );
}

// --------------------------------------------------------------------------
// C26 — blank_string tail slices
// --------------------------------------------------------------------------
#[test]
fn c26_blank_string_tail_slice_lengths() {
    for n in [13, 6, 5, 8, 6, 8, 10, 9, 5, 23] {
        let slice = blank_string_tail(n);
        assert_eq!(slice.len(), n);
        assert!(slice.chars().all(|c| c == ' '));
    }
    assert_eq!(BLANK_LENGTH, 24);
}

// --------------------------------------------------------------------------
// C27 — header/number formatters
// --------------------------------------------------------------------------
#[test]
fn c27_header_and_number_snprintf_widths() {
    assert_eq!(format_header_number("LEV ", 3), "LEV :      3");
    assert_eq!(format_header_long_number("EXP ", 12345), "EXP :  12345");
    assert_eq!(format_header_long_number7_spaces("Level      ", 7), "Level      :       7");
    assert_eq!(format_long_number(-42), "   -42");
    assert_eq!(format_number(999999), "999999");
}

#[test]
fn print_character_level_experience_does_not_panic() {
    reset_for_new_game(Some(1));
    umoria::ui_io::test_set_ncurses_stub(true);
    with_state_mut(|s| {
        s.py.misc.level = 5;
        s.py.misc.exp = 100;
        s.py.misc.max_exp = 100;
        s.py.misc.au = 50;
        s.py.misc.max_hp = 30;
        s.py.misc.current_hp = 25;
        s.py.misc.mana = 10;
        s.py.misc.current_mana = 8;
        s.py.misc.experience_factor = 100;
        s.py.base_exp_levels.fill(100);
        s.py.base_exp_levels[0] = 10;
        s.py.base_exp_levels[1] = 20;
        s.py.base_exp_levels[2] = 40;
        s.py.base_exp_levels[3] = 80;
        s.py.base_exp_levels[4] = 160;
    });
    umoria::ui::print_character_level_experience();
    umoria::ui_io::test_set_ncurses_stub(false);
}
