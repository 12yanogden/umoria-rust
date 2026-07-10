//! Port of src/ui.cpp / ui.h UI section — viewport, status line, character screens.

use crate::config::player::status::{
    PY_BLIND, PY_CONFUSED, PY_FEAR, PY_HUNGRY, PY_POISONED, PY_REPEAT, PY_REST, PY_SEARCH,
    PY_STUDY, PY_WEAK,
};
use crate::config::player::PLAYER_MAX_EXP;
use crate::config::spells::{
    NAME_OFFSET_PRAYERS, NAME_OFFSET_SPELLS, SPELL_TYPE_MAGE, SPELL_TYPE_PRIEST,
};
use crate::data_player::{CHARACTER_RACES, CLASSES, CLASS_LEVEL_ADJ, MAGIC_SPELLS, SPELL_NAMES};
use crate::dungeon::cave_get_tile_symbol;
use crate::dungeon_tile::TILE_LIGHT_FLOOR;
use crate::game::{with_state, with_state_mut};
use crate::player::{
    player_calculate_allowed_spells_count, player_calculate_hit_points, player_disarm_adjustment,
    player_gain_mana, player_rank_title, player_stat_adjustment_wisdom_intelligence, PlayerAttr,
    PlayerClassLevelAdj, PlayerMisc, BTH_PER_PLUS_TO_HIT_ADJUST, PLAYER_MAX_LEVEL,
};
use crate::player_run::player_end_running;
use crate::spells::spell_chance_of_success_for_state;
use crate::types::Coord_t;
use crate::ui_io::terminal::{self, Coord};

/// Port of `Panel_t` in ui.h.
#[derive(Clone, Copy, Debug, Default)]
pub struct Panel {
    pub row: i32,
    pub col: i32,
    pub top: i32,
    pub bottom: i32,
    pub left: i32,
    pub right: i32,
    pub col_prt: i32,
    pub row_prt: i32,
    pub max_rows: i16,
    pub max_cols: i16,
}

/// C++ `dungeon.h` `SCREEN_HEIGHT`.
const SCREEN_HEIGHT: i32 = 22;
/// C++ `dungeon.h` `SCREEN_WIDTH`.
const SCREEN_WIDTH: i32 = 66;
/// C++ `ui.h` `STAT_COLUMN`.
const STAT_COLUMN: i32 = 0;

/// C++ ui.cpp line 11.
pub const BLANK_LENGTH: usize = 24;
/// C++ ui.cpp line 12 — fixed 24-space source for tail-slice padding.
const BLANK_STRING: &str = "                        ";

/// C++ ui.cpp lines 8–10.
const STAT_NAMES: [&str; 6] = ["STR : ", "INT : ", "WIS : ", "DEX : ", "CON : ", "CHR : "];

/// C++ ui.cpp line 12 — `&blank_string[BLANK_LENGTH - N]` pointer-into-array semantics.
#[must_use]
pub fn blank_string_tail(n: usize) -> &'static str {
    &BLANK_STRING[BLANK_LENGTH - n..]
}

/// Derived panel boundary fields for a row/col pair (ui.cpp lines 22–28).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelBoundsFields {
    pub top: i32,
    pub bottom: i32,
    pub row_prt: i32,
    pub left: i32,
    pub right: i32,
    pub col_prt: i32,
}

#[must_use]
pub fn panel_bounds_fields(row: i32, col: i32) -> PanelBoundsFields {
    let half_h = SCREEN_HEIGHT / 2;
    let half_w = SCREEN_WIDTH / 2;
    let top = row * half_h;
    PanelBoundsFields {
        top,
        bottom: top + SCREEN_HEIGHT - 1,
        row_prt: top - 1,
        left: col * half_w,
        right: col * half_w + SCREEN_WIDTH - 1,
        col_prt: col * half_w - 13,
    }
}

/// C++ ui.cpp lines 22–28 — writes `dg.panel` bounds from current row/col.
pub fn panel_bounds() {
    with_state_mut(|state| {
        let fields = panel_bounds_fields(state.dg.panel.row, state.dg.panel.col);
        state.dg.panel.top = fields.top;
        state.dg.panel.bottom = fields.bottom;
        state.dg.panel.row_prt = fields.row_prt;
        state.dg.panel.left = fields.left;
        state.dg.panel.right = fields.right;
        state.dg.panel.col_prt = fields.col_prt;
    });
}

/// Pure panel-change math from ui.cpp lines 34–54.
#[must_use]
pub fn compute_panel_change(current: &Panel, coord: Coord_t, force: bool) -> Option<(i32, i32)> {
    let half_h = SCREEN_HEIGHT / 2;
    let half_w = SCREEN_WIDTH / 2;
    let quart_h = SCREEN_HEIGHT / 4;
    let quart_w = SCREEN_WIDTH / 4;

    let mut panel_y = current.row;
    let mut panel_x = current.col;

    if force || coord.y < current.top + 2 || coord.y > current.bottom - 2 {
        panel_y = (coord.y - quart_h) / half_h;
        if panel_y > i32::from(current.max_rows) {
            panel_y = i32::from(current.max_rows);
        } else if panel_y < 0 {
            panel_y = 0;
        }
    }

    if force || coord.x < current.left + 3 || coord.x > current.right - 3 {
        panel_x = (coord.x - quart_w) / half_w;
        if panel_x > i32::from(current.max_cols) {
            panel_x = i32::from(current.max_cols);
        } else if panel_x < 0 {
            panel_x = 0;
        }
    }

    if panel_y != current.row || panel_x != current.col {
        Some((panel_y, panel_x))
    } else {
        None
    }
}

/// C++ ui.cpp lines 34–71.
#[must_use]
pub fn coord_outside_panel(coord: Coord_t, force: bool) -> bool {
    let changed = with_state_mut(|state| {
        let current = state.dg.panel;
        compute_panel_change(&current, coord, force)
    });

    if let Some((row, col)) = changed {
        // Snapshot find_bound and release the borrow before player_end_running
        // (it re-enters game state).
        let end_running = with_state_mut(|state| {
            state.dg.panel.row = row;
            state.dg.panel.col = col;
            let fields = panel_bounds_fields(row, col);
            state.dg.panel.top = fields.top;
            state.dg.panel.bottom = fields.bottom;
            state.dg.panel.row_prt = fields.row_prt;
            state.dg.panel.left = fields.left;
            state.dg.panel.right = fields.right;
            state.dg.panel.col_prt = fields.col_prt;
            state.options.find_bound
        });
        if end_running {
            player_end_running();
        }
        return true;
    }
    false
}

/// Pure inside-panel check (ui.cpp lines 74–79).
#[must_use]
pub fn coord_inside_panel_bounds(panel: &Panel, coord: Coord_t) -> bool {
    coord.y >= panel.top
        && coord.y <= panel.bottom
        && coord.x >= panel.left
        && coord.x <= panel.right
}

/// C++ ui.cpp lines 74–79.
#[must_use]
pub fn coord_inside_panel(coord: Coord_t) -> bool {
    crate::game::with_state(|state| coord_inside_panel_bounds(&state.dg.panel, coord))
}

/// C++ ui.cpp lines 82–100.
pub fn draw_dungeon_panel() {
    let (top, bottom, left, right) = with_state(|state| {
        (
            state.dg.panel.top,
            state.dg.panel.bottom,
            state.dg.panel.left,
            state.dg.panel.right,
        )
    });

    for (line, coord_y) in (1..).zip(top..=bottom) {
        terminal::erase_line(Coord { y: line, x: 13 });

        for coord_x in left..=right {
            let coord = Coord_t {
                y: coord_y,
                x: coord_x,
            };
            let ch = cave_get_tile_symbol(coord);
            if ch != b' ' {
                terminal::panel_put_tile(
                    ch,
                    Coord {
                        y: coord_y,
                        x: coord_x,
                    },
                );
            }
        }
    }
}

/// C++ ui.cpp lines 103–108.
pub fn draw_cave_panel() {
    terminal::clear_screen();
    print_character_stats_block();
    draw_dungeon_panel();
    print_character_current_depth();
}

/// C++ ui.cpp lines 111–140.
pub fn dungeon_reset_view() {
    let (pos, tile, blind) = with_state_mut(|state| {
        let pos = state.py.pos;
        (
            pos,
            state.dg.floor[pos.y as usize][pos.x as usize],
            state.py.flags.blind,
        )
    });

    if coord_outside_panel(pos, false) {
        draw_dungeon_panel();
    }

    crate::dungeon::dungeon_move_character_light(pos, pos);

    if tile.feature_id == TILE_LIGHT_FLOOR {
        if blind < 1 && !tile.permanent_light {
            crate::dungeon::dungeon_light_room(pos);
        }
        return;
    }

    if tile.perma_lit_room && blind < 1 {
        for i in pos.y - 1..=pos.y + 1 {
            for j in pos.x - 1..=pos.x + 1 {
                let neighbor = with_state_mut(|state| state.dg.floor[i as usize][j as usize]);
                if neighbor.feature_id == TILE_LIGHT_FLOOR && !neighbor.permanent_light {
                    crate::dungeon::dungeon_light_room(Coord_t { y: i, x: j });
                }
            }
        }
    }
}

/// C++ ui.cpp lines 144–154 — `%6d`, `"18/100"`, `" 18/%02d"`.
#[must_use]
pub fn stats_as_string(stat: u8) -> String {
    let percentile = i32::from(stat) - 18;
    if stat <= 18 {
        // ui.cpp line 148: snprintf(..., "%6d", stat)
        format!("{stat:6}")
    } else if percentile == 100 {
        "18/100".to_string()
    } else {
        // ui.cpp line 152: snprintf(..., " 18/%02d", percentile)
        format!(" 18/{percentile:02}")
    }
}

/// C++ ui.cpp lines 157–162.
pub fn display_character_stats(stat: i32) {
    let stat_idx = stat as usize;
    let used = with_state_mut(|state| state.py.stats.used[stat_idx]);
    let text = stats_as_string(used);
    terminal::put_string(
        STAT_NAMES[stat_idx],
        Coord {
            y: 6 + stat,
            x: STAT_COLUMN,
        },
    );
    terminal::put_string(
        &text,
        Coord {
            y: 6 + stat,
            x: STAT_COLUMN + 6,
        },
    );
}

/// C++ ui.cpp lines 166–171.
fn print_character_info_in_field(info: &str, coord: Coord) {
    terminal::put_string(blank_string_tail(13), coord);
    terminal::put_string(info, coord);
}

/// C++ ui.cpp lines 174–178 — `"%s: %6d"`.
#[must_use]
pub fn format_header_long_number(header: &str, num: i32) -> String {
    format!("{header}: {num:6}")
}

/// C++ ui.cpp lines 181–185 — `"%s: %7d"`.
#[must_use]
pub fn format_header_long_number7_spaces(header: &str, num: i32) -> String {
    format!("{header}: {num:7}")
}

/// C++ ui.cpp lines 188–192 — `"%s: %6d"`.
#[must_use]
pub fn format_header_number(header: &str, num: i32) -> String {
    format!("{header}: {num:6}")
}

/// C++ ui.cpp lines 195–199 — `"%6d"`.
#[must_use]
pub fn format_long_number(num: i32) -> String {
    format!("{num:6}")
}

/// C++ ui.cpp lines 202–206 — `"%6d"`.
#[must_use]
pub fn format_number(num: i32) -> String {
    format!("{num:6}")
}

fn print_header_long_number(header: &str, num: i32, coord: Coord) {
    terminal::put_string(&format_header_long_number(header, num), coord);
}

fn print_header_long_number7_spaces(header: &str, num: i32, coord: Coord) {
    terminal::put_string(&format_header_long_number7_spaces(header, num), coord);
}

fn print_header_number(header: &str, num: i32, coord: Coord) {
    terminal::put_string(&format_header_number(header, num), coord);
}

fn print_long_number(num: i32, coord: Coord) {
    terminal::put_string(&format_long_number(num), coord);
}

fn print_number(num: i32, coord: Coord) {
    terminal::put_string(&format_number(num), coord);
}

/// C++ ui.cpp lines 209–211.
pub fn print_character_title() {
    print_character_info_in_field(
        player_rank_title(),
        Coord {
            y: 4,
            x: STAT_COLUMN,
        },
    );
}

/// C++ ui.cpp lines 214–216.
pub fn print_character_level() {
    with_state_mut(|state| {
        print_number(
            i32::from(state.py.misc.level),
            Coord {
                y: 13,
                x: STAT_COLUMN + 6,
            },
        );
    });
}

/// C++ ui.cpp lines 219–221.
pub fn print_character_current_mana() {
    with_state_mut(|state| {
        print_number(
            i32::from(state.py.misc.current_mana),
            Coord {
                y: 15,
                x: STAT_COLUMN + 6,
            },
        );
    });
}

/// C++ ui.cpp lines 224–226.
pub fn print_character_max_hit_points() {
    with_state_mut(|state| {
        print_number(
            i32::from(state.py.misc.max_hp),
            Coord {
                y: 16,
                x: STAT_COLUMN + 6,
            },
        );
    });
}

/// C++ ui.cpp lines 229–231.
pub fn print_character_current_hit_points() {
    with_state_mut(|state| {
        print_number(
            i32::from(state.py.misc.current_hp),
            Coord {
                y: 17,
                x: STAT_COLUMN + 6,
            },
        );
    });
}

/// C++ ui.cpp lines 234–236.
pub fn print_character_current_armor_class() {
    with_state_mut(|state| {
        print_number(
            i32::from(state.py.misc.display_ac),
            Coord {
                y: 19,
                x: STAT_COLUMN + 6,
            },
        );
    });
}

/// C++ ui.cpp lines 239–241.
pub fn print_character_gold_value() {
    with_state_mut(|state| {
        print_long_number(
            state.py.misc.au,
            Coord {
                y: 20,
                x: STAT_COLUMN + 6,
            },
        );
    });
}

/// C++ ui.cpp lines 244–256 depth string (without terminal write).
#[must_use]
pub fn format_character_current_depth(current_level: i16) -> String {
    let depth = i32::from(current_level) * 50;
    if depth == 0 {
        "Town level".to_string()
    } else {
        format!("{depth} feet")
    }
}

/// C++ ui.cpp lines 244–256.
pub fn print_character_current_depth() {
    let depths = with_state_mut(|state| format_character_current_depth(state.dg.current_level));
    terminal::put_string_clear_to_eol(&depths, Coord { y: 23, x: 65 });
}

/// C++ ui.cpp lines 259–267.
pub fn print_character_hunger_status() {
    let text = with_state(|state| {
        if (state.py.flags.status & PY_WEAK) != 0 {
            "Weak  "
        } else if (state.py.flags.status & PY_HUNGRY) != 0 {
            "Hungry"
        } else {
            blank_string_tail(6)
        }
    });
    terminal::put_string(text, Coord { y: 23, x: 0 });
}

/// C++ ui.cpp lines 270–276.
pub fn print_character_blind_status() {
    with_state_mut(|state| {
        if (state.py.flags.status & PY_BLIND) != 0 {
            terminal::put_string("Blind", Coord { y: 23, x: 7 });
        } else {
            terminal::put_string(blank_string_tail(5), Coord { y: 23, x: 7 });
        }
    });
}

/// C++ ui.cpp lines 279–285.
pub fn print_character_confused_state() {
    with_state_mut(|state| {
        if (state.py.flags.status & PY_CONFUSED) != 0 {
            terminal::put_string("Confused", Coord { y: 23, x: 13 });
        } else {
            terminal::put_string(blank_string_tail(8), Coord { y: 23, x: 13 });
        }
    });
}

/// C++ ui.cpp lines 288–294.
pub fn print_character_fear_state() {
    with_state_mut(|state| {
        if (state.py.flags.status & PY_FEAR) != 0 {
            terminal::put_string("Afraid", Coord { y: 23, x: 22 });
        } else {
            terminal::put_string(blank_string_tail(6), Coord { y: 23, x: 22 });
        }
    });
}

/// C++ ui.cpp lines 297–303.
pub fn print_character_poisoned_state() {
    with_state_mut(|state| {
        if (state.py.flags.status & PY_POISONED) != 0 {
            terminal::put_string("Poisoned", Coord { y: 23, x: 29 });
        } else {
            terminal::put_string(blank_string_tail(8), Coord { y: 23, x: 29 });
        }
    });
}

/// Pure movement-state builder (ui.cpp lines 306–357).
#[must_use]
pub fn movement_state_string(
    paralysis: i16,
    status: u32,
    rest: i16,
    command_count: u32,
    display_counts: bool,
) -> (String, u32) {
    let mut status = status & !PY_REPEAT;

    if paralysis > 1 {
        return ("Paralysed".to_string(), status);
    }

    if (status & PY_REST) != 0 {
        let rest_string = if rest < 0 {
            "Rest *".to_string()
        } else if display_counts {
            // ui.cpp line 320: "Rest %-5d"
            format!("Rest {rest:<5}")
        } else {
            "Rest".to_string()
        };
        return (rest_string, status);
    }

    if command_count > 0 {
        let repeat_string = if display_counts {
            // ui.cpp line 334: "Repeat %-.3d"
            format!("Repeat {command_count:03}")
        } else {
            "Repeat".to_string()
        };
        status |= PY_REPEAT;
        let mut out = repeat_string;
        if (status & PY_SEARCH) != 0 {
            out = "Search".to_string();
        }
        return (out, status);
    }

    if (status & PY_SEARCH) != 0 {
        return ("Searching".to_string(), status);
    }

    (blank_string_tail(10).to_string(), status)
}

/// C++ ui.cpp lines 306–357.
pub fn print_character_movement_state() {
    with_state_mut(|state| {
        let display_counts = state.options.display_counts;
        let command_count = state.game.command_count;
        let paralysis = state.py.flags.paralysis;
        let rest = state.py.flags.rest;
        let status_in = state.py.flags.status;
        let (text, status_out) =
            movement_state_string(paralysis, status_in, rest, command_count, display_counts);
        state.py.flags.status = status_out;
        terminal::put_string(&text, Coord { y: 23, x: 38 });
    });
}

/// Pure speed string (ui.cpp lines 360–379).
#[must_use]
pub fn speed_display_string(speed: i16, searching: bool) -> String {
    let mut speed = speed;
    if searching {
        speed -= 1;
    }
    if speed > 1 {
        "Very Slow".to_string()
    } else if speed == 1 {
        "Slow     ".to_string()
    } else if speed == 0 {
        blank_string_tail(9).to_string()
    } else if speed == -1 {
        "Fast     ".to_string()
    } else {
        "Very Fast".to_string()
    }
}

/// C++ ui.cpp lines 360–379.
pub fn print_character_speed() {
    with_state_mut(|state| {
        let searching = (state.py.flags.status & PY_SEARCH) != 0;
        let text = speed_display_string(state.py.flags.speed, searching);
        terminal::put_string(&text, Coord { y: 23, x: 49 });
    });
}

/// C++ ui.cpp lines 381–389.
pub fn print_character_study_instruction() {
    with_state_mut(|state| {
        state.py.flags.status &= !PY_STUDY;
        if state.py.flags.new_spells_to_learn == 0 {
            terminal::put_string(blank_string_tail(5), Coord { y: 23, x: 59 });
        } else {
            terminal::put_string("Study", Coord { y: 23, x: 59 });
        }
    });
}

/// C++ ui.cpp lines 392–406.
pub fn print_character_winner() {
    with_state_mut(|state| {
        if (state.game.noscore & 0x2) != 0 {
            if state.game.wizard_mode {
                terminal::put_string("Is wizard  ", Coord { y: 22, x: 0 });
            } else {
                terminal::put_string("Was wizard ", Coord { y: 22, x: 0 });
            }
        } else if (state.game.noscore & 0x1) != 0 {
            terminal::put_string("Resurrected", Coord { y: 22, x: 0 });
        } else if (state.game.noscore & 0x4) != 0 {
            terminal::put_string("Duplicate", Coord { y: 22, x: 0 });
        } else if state.game.total_winner {
            terminal::put_string("*Winner*   ", Coord { y: 22, x: 0 });
        }
    });
}

/// C++ ui.cpp lines 409–461.
pub fn print_character_stats_block() {
    let rank_title = player_rank_title();
    with_state_mut(|state| {
        let race_id = state.py.misc.race_id as usize;
        let class_id = state.py.misc.class_id as usize;
        print_character_info_in_field(
            CHARACTER_RACES[race_id].name,
            Coord {
                y: 2,
                x: STAT_COLUMN,
            },
        );
        print_character_info_in_field(
            CLASSES[class_id].title,
            Coord {
                y: 3,
                x: STAT_COLUMN,
            },
        );
        print_character_info_in_field(
            rank_title,
            Coord {
                y: 4,
                x: STAT_COLUMN,
            },
        );
    });

    for i in 0..6 {
        display_character_stats(i);
    }

    with_state_mut(|state| {
        let misc = &state.py.misc;
        print_header_number(
            "LEV ",
            i32::from(misc.level),
            Coord {
                y: 13,
                x: STAT_COLUMN,
            },
        );
        print_header_long_number(
            "EXP ",
            misc.exp,
            Coord {
                y: 14,
                x: STAT_COLUMN,
            },
        );
        print_header_number(
            "MANA",
            i32::from(misc.current_mana),
            Coord {
                y: 15,
                x: STAT_COLUMN,
            },
        );
        print_header_number(
            "MHP ",
            i32::from(misc.max_hp),
            Coord {
                y: 16,
                x: STAT_COLUMN,
            },
        );
        print_header_number(
            "CHP ",
            i32::from(misc.current_hp),
            Coord {
                y: 17,
                x: STAT_COLUMN,
            },
        );
        print_header_number(
            "AC  ",
            i32::from(misc.display_ac),
            Coord {
                y: 19,
                x: STAT_COLUMN,
            },
        );
        print_header_long_number(
            "GOLD",
            misc.au,
            Coord {
                y: 20,
                x: STAT_COLUMN,
            },
        );
    });

    print_character_winner();

    let status = with_state_mut(|state| state.py.flags.status);

    if (PY_HUNGRY | PY_WEAK) & status != 0 {
        print_character_hunger_status();
    }
    if status & PY_BLIND != 0 {
        print_character_blind_status();
    }
    if status & PY_CONFUSED != 0 {
        print_character_confused_state();
    }
    if status & PY_FEAR != 0 {
        print_character_fear_state();
    }
    if status & PY_POISONED != 0 {
        print_character_poisoned_state();
    }
    if (PY_SEARCH | PY_REST) & status != 0 {
        print_character_movement_state();
    }

    let speed = with_state_mut(|state| {
        let status = state.py.flags.status;
        state.py.flags.speed - i16::try_from((status & PY_SEARCH) >> 8).unwrap_or(0)
    });
    if speed != 0 {
        print_character_speed();
    }

    print_character_study_instruction();
}

/// C++ ui.cpp lines 464–480.
pub fn print_character_information() {
    terminal::clear_screen();
    terminal::put_string("Name        :", Coord { y: 2, x: 1 });
    terminal::put_string("Race        :", Coord { y: 3, x: 1 });
    terminal::put_string("Sex         :", Coord { y: 4, x: 1 });
    terminal::put_string("Class       :", Coord { y: 5, x: 1 });

    with_state_mut(|state| {
        if !state.game.character_generated {
            return;
        }
        let name = c_str(&state.py.misc.name);
        let race_id = state.py.misc.race_id as usize;
        let class_id = state.py.misc.class_id as usize;
        let gender_label = if state.py.misc.gender {
            "Male"
        } else {
            "Female"
        };
        terminal::put_string(&name, Coord { y: 2, x: 15 });
        terminal::put_string(CHARACTER_RACES[race_id].name, Coord { y: 3, x: 15 });
        terminal::put_string(gender_label, Coord { y: 4, x: 15 });
        terminal::put_string(CLASSES[class_id].title, Coord { y: 5, x: 15 });
    });
}

/// C++ ui.cpp lines 483–501.
pub fn print_character_stats() {
    with_state_mut(|state| {
        for (i, stat_name) in STAT_NAMES.iter().enumerate() {
            let mut buf = stats_as_string(state.py.stats.used[i]);
            terminal::put_string(
                stat_name,
                Coord {
                    y: 2 + i as i32,
                    x: 61,
                },
            );
            terminal::put_string(
                &buf,
                Coord {
                    y: 2 + i as i32,
                    x: 66,
                },
            );

            if state.py.stats.max[i] > state.py.stats.current[i] {
                buf = stats_as_string(state.py.stats.max[i]);
                terminal::put_string(
                    &buf,
                    Coord {
                        y: 2 + i as i32,
                        x: 73,
                    },
                );
            }
        }

        print_header_number(
            "+ To Hit    ",
            i32::from(state.py.misc.display_to_hit),
            Coord { y: 9, x: 1 },
        );
        print_header_number(
            "+ To Damage ",
            i32::from(state.py.misc.display_to_damage),
            Coord { y: 10, x: 1 },
        );
        print_header_number(
            "+ To AC     ",
            i32::from(state.py.misc.display_to_ac),
            Coord { y: 11, x: 1 },
        );
        print_header_number(
            "  Total AC  ",
            i32::from(state.py.misc.display_ac),
            Coord { y: 12, x: 1 },
        );
    });
}

/// C++ ui.cpp lines 504–528 — C++ integer division `coord.x / coord.y`.
#[must_use]
pub fn stat_rating(coord: Coord_t) -> &'static str {
    match coord.x / coord.y {
        -3..=-1 => "Very Bad",
        0 | 1 => "Bad",
        2 => "Poor",
        3 | 4 => "Fair",
        5 => "Good",
        6 => "Very Good",
        7 | 8 => "Excellent",
        _ => "Superb",
    }
}

/// C++ ui.cpp lines 531–536.
pub fn print_character_vital_statistics() {
    with_state_mut(|state| {
        print_header_number(
            "Age          ",
            i32::from(state.py.misc.age),
            Coord { y: 2, x: 38 },
        );
        print_header_number(
            "Height       ",
            i32::from(state.py.misc.height),
            Coord { y: 3, x: 38 },
        );
        print_header_number(
            "Weight       ",
            i32::from(state.py.misc.weight),
            Coord { y: 4, x: 38 },
        );
        print_header_number(
            "Social Class ",
            i32::from(state.py.misc.social_class),
            Coord { y: 5, x: 38 },
        );
    });
}

/// Exp-to-advance line for character sheet (ui.cpp lines 544–548).
#[must_use]
pub fn format_exp_to_advance_line(
    level: u8,
    base_exp_levels: &[u32],
    experience_factor: u8,
) -> String {
    if level >= PLAYER_MAX_LEVEL {
        "Exp to Adv.: *******".to_string()
    } else {
        let val =
            (base_exp_levels[(level - 1) as usize] * u32::from(experience_factor) / 100) as i32;
        format_header_long_number7_spaces("Exp to Adv.", val)
    }
}

/// C++ ui.cpp lines 539–555.
pub fn print_character_level_experience() {
    let (level, exp, max_exp, au, max_hp, current_hp, mana, current_mana, base_exp, exp_factor) =
        with_state(|state| {
            let misc = state.py.misc;
            (
                misc.level,
                misc.exp,
                misc.max_exp,
                misc.au,
                misc.max_hp,
                misc.current_hp,
                misc.mana,
                misc.current_mana,
                state.py.base_exp_levels,
                misc.experience_factor,
            )
        });

    print_header_long_number7_spaces("Level      ", i32::from(level), Coord { y: 9, x: 28 });
    print_header_long_number7_spaces("Experience ", exp, Coord { y: 10, x: 28 });
    print_header_long_number7_spaces("Max Exp    ", max_exp, Coord { y: 11, x: 28 });

    if level >= u16::from(PLAYER_MAX_LEVEL) {
        terminal::put_string_clear_to_eol("Exp to Adv.: *******", Coord { y: 12, x: 28 });
    } else {
        let val = (base_exp[(level - 1) as usize] * u32::from(exp_factor) / 100) as i32;
        print_header_long_number7_spaces("Exp to Adv.", val, Coord { y: 12, x: 28 });
    }

    print_header_long_number7_spaces("Gold       ", au, Coord { y: 13, x: 28 });
    print_header_number("Max Hit Points ", i32::from(max_hp), Coord { y: 9, x: 52 });
    print_header_number(
        "Cur Hit Points ",
        i32::from(current_hp),
        Coord { y: 10, x: 52 },
    );
    print_header_number("Max Mana       ", i32::from(mana), Coord { y: 11, x: 52 });
    print_header_number(
        "Cur Mana       ",
        i32::from(current_mana),
        Coord { y: 12, x: 52 },
    );
}

/// Pure ability math from ui.cpp lines 561–582.
#[must_use]
pub fn compute_ability_values(
    misc: &PlayerMisc,
    disarm_adj: i32,
    int_adj: i32,
    wis_adj: i32,
    see_infra: i16,
) -> (i32, i32, i32, i32, i32, i32, i32, i32, String) {
    let class = misc.class_id as usize;
    let level = i32::from(misc.level);

    let xbth = misc.bth as i32
        + misc.plusses_to_hit as i32 * i32::from(BTH_PER_PLUS_TO_HIT_ADJUST)
        + i32::from(CLASS_LEVEL_ADJ[class][PlayerClassLevelAdj::BTH as usize]) * level;
    let xbthb = misc.bth_with_bows as i32
        + misc.plusses_to_hit as i32 * i32::from(BTH_PER_PLUS_TO_HIT_ADJUST)
        + i32::from(CLASS_LEVEL_ADJ[class][PlayerClassLevelAdj::BTHB as usize]) * level;

    let mut xfos = 40 - misc.fos as i32;
    if xfos < 0 {
        xfos = 0;
    }

    let xsrh = misc.chance_in_search as i32;
    let xstl = misc.stealth_factor as i32 + 1;
    let xdis = misc.disarm as i32
        + 2 * disarm_adj
        + int_adj
        + i32::from(CLASS_LEVEL_ADJ[class][PlayerClassLevelAdj::DISARM as usize]) * level / 3;
    let xsave = misc.saving_throw as i32
        + wis_adj
        + i32::from(CLASS_LEVEL_ADJ[class][PlayerClassLevelAdj::SAVE as usize]) * level / 3;
    let xdev = misc.saving_throw as i32
        + int_adj
        + i32::from(CLASS_LEVEL_ADJ[class][PlayerClassLevelAdj::DEVICE as usize]) * level / 3;

    let xinfra = format!("{} feet", see_infra * 10);

    (xbth, xbthb, xfos, xsrh, xstl, xdis, xsave, xdev, xinfra)
}

/// C++ ui.cpp lines 558–605.
pub fn print_character_abilities() {
    terminal::clear_to_bottom(14);

    let (misc, see_infra, disarm_adj, int_adj, wis_adj) = with_state(|state| {
        (
            state.py.misc,
            state.py.flags.see_infra,
            player_disarm_adjustment(),
            player_stat_adjustment_wisdom_intelligence(PlayerAttr::A_INT),
            player_stat_adjustment_wisdom_intelligence(PlayerAttr::A_WIS),
        )
    });
    let (xbth, xbthb, xfos, xsrh, xstl, xdis, xsave, xdev, xinfra) =
        compute_ability_values(&misc, disarm_adj, int_adj, wis_adj, see_infra);

    terminal::put_string("(Miscellaneous Abilities)", Coord { y: 15, x: 25 });
    terminal::put_string("Fighting    :", Coord { y: 16, x: 1 });
    terminal::put_string(
        stat_rating(Coord_t { x: 12, y: xbth }),
        Coord { y: 16, x: 15 },
    );
    terminal::put_string("Bows/Throw  :", Coord { y: 17, x: 1 });
    terminal::put_string(
        stat_rating(Coord_t { x: 12, y: xbthb }),
        Coord { y: 17, x: 15 },
    );
    terminal::put_string("Saving Throw:", Coord { y: 18, x: 1 });
    terminal::put_string(
        stat_rating(Coord_t { x: 6, y: xsave }),
        Coord { y: 18, x: 15 },
    );

    terminal::put_string("Stealth     :", Coord { y: 16, x: 28 });
    terminal::put_string(
        stat_rating(Coord_t { x: 1, y: xstl }),
        Coord { y: 16, x: 42 },
    );
    terminal::put_string("Disarming   :", Coord { y: 17, x: 28 });
    terminal::put_string(
        stat_rating(Coord_t { x: 8, y: xdis }),
        Coord { y: 17, x: 42 },
    );
    terminal::put_string("Magic Device:", Coord { y: 18, x: 28 });
    terminal::put_string(
        stat_rating(Coord_t { x: 6, y: xdev }),
        Coord { y: 18, x: 42 },
    );

    terminal::put_string("Perception  :", Coord { y: 16, x: 55 });
    terminal::put_string(
        stat_rating(Coord_t { x: 3, y: xfos }),
        Coord { y: 16, x: 69 },
    );
    terminal::put_string("Searching   :", Coord { y: 17, x: 55 });
    terminal::put_string(
        stat_rating(Coord_t { x: 6, y: xsrh }),
        Coord { y: 17, x: 69 },
    );
    terminal::put_string("Infra-Vision:", Coord { y: 18, x: 55 });
    terminal::put_string(&xinfra, Coord { y: 18, x: 69 });
}

/// C++ ui.cpp lines 608–614.
pub fn print_character() {
    print_character_information();
    print_character_vital_statistics();
    print_character_stats();
    print_character_level_experience();
    print_character_abilities();
}

/// C++ ui.cpp lines 617–628.
pub fn get_character_name() {
    terminal::put_string_clear_to_eol(
        "Enter your player's name  [press <RETURN> when finished]",
        Coord { y: 21, x: 2 },
    );
    terminal::put_string(blank_string_tail(23), Coord { y: 2, x: 15 });

    // Read into a local buffer: `get_string_input` → `get_key_input` borrows
    // game state, so we must not hold `with_state_mut` across the call.
    let mut name = [0u8; crate::player::PLAYER_NAME_SIZE as usize];
    let ok = terminal::get_string_input(&mut name, Coord { y: 2, x: 15 }, 23);
    if !ok || name[0] == 0 {
        terminal::get_default_player_name(&mut name);
        let display = c_str(&name);
        terminal::put_string(&display, Coord { y: 2, x: 15 });
    }
    with_state_mut(|state| state.py.misc.name = name);

    terminal::clear_to_bottom(20);
}

/// C++ ui.cpp lines 631–665.
pub fn change_character_name() {
    print_character();

    loop {
        terminal::put_string_clear_to_eol(
            "<f>ile character description. <c>hange character name.",
            Coord { y: 21, x: 2 },
        );

        match terminal::get_key_input() {
            b'c' => {
                get_character_name();
                break;
            }
            b'f' => {
                let mut temp = [0u8; crate::types::MORIA_MESSAGE_SIZE];
                terminal::put_string_clear_to_eol("File name:", Coord { y: 0, x: 0 });
                if terminal::get_string_input(&mut temp, Coord { y: 0, x: 10 }, 60) && temp[0] != 0
                {
                    let path = c_str(&temp);
                    if crate::game_files::output_player_character_to_file(&path) {
                        break;
                    }
                }
            }
            crate::ui_io::ESCAPE | b' ' | b'\n' | b'\r' => break,
            _ => {
                terminal::terminal_bell_sound();
            }
        }
    }
}

/// Spell comment suffix (ui.cpp lines 698–709).
#[must_use]
pub fn format_spell_comment(
    comment: bool,
    spells_forgotten: u32,
    spells_learnt: u32,
    spells_worked: u32,
    spell_id: i32,
) -> &'static str {
    if !comment {
        ""
    } else if (spells_forgotten & (1u32 << spell_id)) != 0 {
        " forgotten"
    } else if (spells_learnt & (1u32 << spell_id)) == 0 {
        " unknown"
    } else if (spells_worked & (1u32 << spell_id)) == 0 {
        " untried"
    } else {
        ""
    }
}

/// Single spell row (ui.cpp line 721) with explicit fail chance for testing.
#[must_use]
pub fn format_spell_row(
    spell_char: char,
    name: &str,
    level_required: u8,
    mana_required: u8,
    fail_chance: i32,
    comment_suffix: &str,
) -> String {
    format!(
        "  {spell_char}) {name:<30}{level_required:2} {mana_required:4} {fail_chance:3}%{comment_suffix}"
    )
}

/// C++ ui.cpp lines 670–724.
pub fn display_spells_list(
    spell_ids: &[i32],
    mut number_of_choices: i32,
    comment: bool,
    non_consecutive: i32,
) {
    let col = if comment { 22 } else { 31 };

    let consecutive_offset = with_state(|state| {
        if CLASSES[state.py.misc.class_id as usize].class_to_use_mage_spells == SPELL_TYPE_MAGE {
            i32::from(NAME_OFFSET_SPELLS)
        } else {
            i32::from(NAME_OFFSET_PRAYERS)
        }
    });

    terminal::erase_line(Coord { y: 1, x: col });
    terminal::put_string("Name", Coord { y: 1, x: col + 5 });
    terminal::put_string("Lv Mana Fail", Coord { y: 1, x: col + 35 });

    if number_of_choices > 22 {
        number_of_choices = 22;
    }

    let rows = with_state(|state| {
        let class_id = state.py.misc.class_id as usize;
        let spells_forgotten = state.py.flags.spells_forgotten;
        let spells_learnt = state.py.flags.spells_learnt;
        let spells_worked = state.py.flags.spells_worked;
        let mut rows = Vec::with_capacity(number_of_choices as usize);

        for i in 0..number_of_choices {
            let spell_id = spell_ids[i as usize];
            let spell = &MAGIC_SPELLS[class_id - 1][spell_id as usize];
            let comment_suffix = format_spell_comment(
                comment,
                spells_forgotten,
                spells_learnt,
                spells_worked,
                spell_id,
            );

            let spell_char = if non_consecutive == -1 {
                (b'a' + i as u8) as char
            } else {
                (b'a' + (spell_id - non_consecutive) as u8) as char
            };

            let name = SPELL_NAMES[(spell_id + consecutive_offset) as usize];
            let fail = spell_chance_of_success_for_state(state, spell_id);
            rows.push(format_spell_row(
                spell_char,
                name,
                spell.level_required,
                spell.mana_required,
                fail,
                comment_suffix,
            ));
        }

        rows
    });

    for (i, out_val) in rows.iter().enumerate() {
        terminal::put_string_clear_to_eol(
            out_val,
            Coord {
                y: 2 + i as i32,
                x: col,
            },
        );
    }
}

/// Exp halving on level gain (ui.cpp lines 737–743).
#[must_use]
pub fn experience_exp_halving(
    level: u16,
    exp: i32,
    base_exp_levels: &[u32],
    experience_factor: u8,
) -> (u16, i32) {
    let new_level = level + 1;
    let new_exp_threshold =
        (base_exp_levels[(new_level - 1) as usize] * u32::from(experience_factor) / 100) as i32;
    let new_exp = if exp > new_exp_threshold {
        let dif_exp = exp - new_exp_threshold;
        new_exp_threshold + dif_exp / 2
    } else {
        exp
    };
    (new_level, new_exp)
}

/// C++ ui.cpp lines 728–757.
fn player_gain_level() {
    let msg = with_state_mut(|state| {
        state.py.misc.level += 1;
        let msg = format!("Welcome to level {}.", state.py.misc.level);

        let new_exp_threshold = (state.py.base_exp_levels[(state.py.misc.level - 1) as usize]
            * u32::from(state.py.misc.experience_factor)
            / 100) as i32;
        if state.py.misc.exp > new_exp_threshold {
            let dif_exp = state.py.misc.exp - new_exp_threshold;
            state.py.misc.exp = new_exp_threshold + dif_exp / 2;
        }
        msg
    });
    player_calculate_hit_points();
    terminal::print_message(Some(&msg));

    print_character_level();
    print_character_title();

    let class_id = with_state(|state| state.py.misc.class_id);
    let class = &CLASSES[class_id as usize];
    if class.class_to_use_mage_spells == SPELL_TYPE_MAGE {
        player_calculate_allowed_spells_count(PlayerAttr::A_INT);
        player_gain_mana(PlayerAttr::A_INT);
    } else if class.class_to_use_mage_spells == SPELL_TYPE_PRIEST {
        player_calculate_allowed_spells_count(PlayerAttr::A_WIS);
        player_gain_mana(PlayerAttr::A_WIS);
    }
}

/// Clamp exp to `PLAYER_MAX_EXP` (ui.cpp line 761–763).
#[must_use]
pub fn simulate_exp_clamp(exp: i32) -> i32 {
    exp.min(PLAYER_MAX_EXP)
}

/// Level-up loop simulation (ui.cpp lines 765–771).
#[must_use]
pub fn count_experience_level_ups(
    mut level: u16,
    mut exp: i32,
    mut max_exp: i32,
    base_exp_levels: &[u32],
    experience_factor: u8,
) -> (u16, i32, i32, u32) {
    exp = simulate_exp_clamp(exp);
    let mut gains = 0u32;

    while (level as u8) < PLAYER_MAX_LEVEL {
        let threshold =
            (base_exp_levels[(level - 1) as usize] * u32::from(experience_factor) / 100) as i32;
        if threshold > exp {
            break;
        }
        gains += 1;
        let (new_level, new_exp) =
            experience_exp_halving(level, exp, base_exp_levels, experience_factor);
        level = new_level;
        exp = new_exp;
    }

    if exp > max_exp {
        max_exp = exp;
    }

    (level, exp, max_exp, gains)
}

/// C++ ui.cpp lines 760–773.
pub fn display_character_experience() {
    with_state_mut(|state| {
        if state.py.misc.exp > PLAYER_MAX_EXP {
            state.py.misc.exp = PLAYER_MAX_EXP;
        }
    });

    loop {
        let should_gain = with_state_mut(|state| {
            if (state.py.misc.level as u8) >= PLAYER_MAX_LEVEL {
                return false;
            }
            let threshold = (state.py.base_exp_levels[(state.py.misc.level - 1) as usize]
                * u32::from(state.py.misc.experience_factor)
                / 100) as i32;
            threshold <= state.py.misc.exp
        });
        if !should_gain {
            break;
        }
        player_gain_level();
    }

    with_state_mut(|state| {
        if state.py.misc.exp > state.py.misc.max_exp {
            state.py.misc.max_exp = state.py.misc.exp;
        }

        let exp = state.py.misc.exp;
        print_long_number(
            exp,
            Coord {
                y: 14,
                x: STAT_COLUMN + 6,
            },
        );
    });
}

fn c_str(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}
