//! Port of src/game.cpp / src/game.h — central mutable game state owner.

#[cfg(debug_assertions)]
use std::cell::Cell;
use std::cell::RefCell;
use std::io::{self, Write};

use crate::rng::{rnd_state, set_seed_state};
use crate::store::Store;
use crate::types::{
    Coord_t, Dungeon, Inventory, Monster, Player, Recall, Screen, Vtype, LEVEL_MAX_OBJECTS,
    MAX_DUNGEON_OBJECTS, MAX_STORES, MESSAGE_HISTORY_SIZE, MON_MAX_CREATURES, MON_MAX_LEVELS,
    MON_TOTAL_ALLOCATIONS, MORIA_MESSAGE_SIZE, NORMAL_TABLE_SIZE, OBJECT_IDENT_SIZE,
    TREASURE_MAX_LEVELS,
};

/// Port of `NORMAL_TABLE_SD` in game.h — upstream gap: belongs in types.rs (`phase_2.4`).
pub const NORMAL_TABLE_SD: u8 = 64;

const SHRT_MAX: i32 = 32_767;

/// Authoritative `normal_table` from `data_tables.cpp` lines 93-126.
const NORMAL_TABLE_VALUES: [u16; NORMAL_TABLE_SIZE] = [
    206, 613, 1022, 1430, 1838, 2245, 2652, 3058, 3463, 3867, 4271, 4673, 5075, 5475, 5874, 6271,
    6667, 7061, 7454, 7845, 8234, 8621, 9006, 9389, 9770, 10148, 10524, 10898, 11269, 11638, 12004,
    12367, 12727, 13085, 13440, 13792, 14140, 14486, 14828, 15168, 15504, 15836, 16166, 16492,
    16814, 17133, 17449, 17761, 18069, 18374, 18675, 18972, 19266, 19556, 19842, 20124, 20403,
    20678, 20949, 21216, 21479, 21738, 21994, 22245, 22493, 22737, 22977, 23213, 23446, 23674,
    23899, 24120, 24336, 24550, 24759, 24965, 25166, 25365, 25559, 25750, 25937, 26120, 26300,
    26476, 26649, 26818, 26983, 27146, 27304, 27460, 27612, 27760, 27906, 28048, 28187, 28323,
    28455, 28585, 28711, 28835, 28955, 29073, 29188, 29299, 29409, 29515, 29619, 29720, 29818,
    29914, 30007, 30098, 30186, 30272, 30356, 30437, 30516, 30593, 30668, 30740, 30810, 30879,
    30945, 31010, 31072, 31133, 31192, 31249, 31304, 31358, 31410, 31460, 31509, 31556, 31601,
    31646, 31688, 31730, 31770, 31808, 31846, 31882, 31917, 31950, 31983, 32014, 32044, 32074,
    32102, 32129, 32155, 32180, 32205, 32228, 32251, 32273, 32294, 32314, 32333, 32352, 32370,
    32387, 32404, 32420, 32435, 32450, 32464, 32477, 32490, 32503, 32515, 32526, 32537, 32548,
    32558, 32568, 32577, 32586, 32595, 32603, 32611, 32618, 32625, 32632, 32639, 32645, 32651,
    32657, 32662, 32667, 32672, 32677, 32682, 32686, 32690, 32694, 32698, 32702, 32705, 32708,
    32711, 32714, 32717, 32720, 32722, 32725, 32727, 32729, 32731, 32733, 32735, 32737, 32739,
    32740, 32742, 32743, 32745, 32746, 32747, 32748, 32749, 32750, 32751, 32752, 32753, 32754,
    32755, 32756, 32757, 32757, 32758, 32758, 32759, 32760, 32760, 32761, 32761, 32761, 32762,
    32762, 32763, 32763, 32763, 32764, 32764, 32764, 32764, 32765, 32765, 32765, 32765, 32766,
    32766, 32766, 32766, 32766,
];

thread_local! {
    static TEST_UNIX_TIME: std::cell::Cell<Option<u32>> = const { std::cell::Cell::new(None) };
}

/// Test hook for `seeds_initialize(0)` until phase_2.3 provides `helpers::get_current_unix_time`.
#[doc(hidden)]
pub fn set_test_unix_time(clock: Option<u32>) {
    TEST_UNIX_TIME.with(|t| t.set(clock));
}

/// Current Unix time, honoring [`set_test_unix_time`] in tests.
pub fn current_unix_time() -> u32 {
    get_current_unix_time()
}

fn get_current_unix_time() -> u32 {
    if let Some(clock) = TEST_UNIX_TIME.with(std::cell::Cell::get) {
        return clock;
    }
    crate::helpers::get_current_unix_time()
}

/// Port of `config::options` (config.cpp).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub display_counts: bool,
    pub find_bound: bool,
    pub run_cut_corners: bool,
    pub run_examine_corners: bool,
    pub run_ignore_doors: bool,
    pub run_print_self: bool,
    pub highlight_seams: bool,
    pub prompt_to_pickup: bool,
    pub use_roguelike_keys: bool,
    pub show_inventory_weights: bool,
    pub error_beep_sound: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            display_counts: true,
            find_bound: false,
            run_cut_corners: true,
            run_examine_corners: true,
            run_ignore_doors: false,
            run_print_self: false,
            highlight_seams: false,
            prompt_to_pickup: false,
            use_roguelike_keys: false,
            show_inventory_weights: false,
            error_beep_sound: true,
        }
    }
}

/// RNG state (`rnd_seed` + `old_seed` from rng.cpp / game.cpp).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rng {
    pub seed: u32,
    pub old_seed: u32,
}

/// Treasure heap inside `Game_t` (game.h).
#[derive(Clone, Debug)]
pub struct GameTreasure {
    pub current_id: i16,
    pub list: [Inventory; LEVEL_MAX_OBJECTS as usize],
}

impl Default for GameTreasure {
    fn default() -> Self {
        Self {
            current_id: 0,
            list: [Inventory::default(); LEVEL_MAX_OBJECTS as usize],
        }
    }
}

/// Screen state inside `Game_t` (game.h).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameScreen {
    pub current_screen_id: Screen,
    pub screen_left_pos: i32,
    pub screen_bottom_pos: i32,
    pub wear_low_id: i32,
    pub wear_high_id: i32,
}

/// Port of `Game_t` (game.h) — defaults match C++ member initializers.
#[derive(Clone, Debug)]
pub struct GameState {
    pub magic_seed: u32,
    pub town_seed: u32,
    pub character_generated: bool,
    pub character_saved: bool,
    pub character_is_dead: bool,
    pub total_winner: bool,
    pub teleport_player: bool,
    pub player_free_turn: bool,
    pub to_be_wizard: bool,
    pub wizard_mode: bool,
    pub noscore: i16,
    pub use_last_direction: bool,
    pub doing_inventory_command: u8,
    pub last_command: u8,
    pub command_count: u32,
    pub character_died_from: Vtype,
    pub treasure: GameTreasure,
    pub screen: GameScreen,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            magic_seed: 0,
            town_seed: 0,
            character_generated: false,
            character_saved: false,
            character_is_dead: false,
            total_winner: false,
            teleport_player: false,
            player_free_turn: false,
            to_be_wizard: false,
            wizard_mode: false,
            noscore: 0,
            use_last_direction: false,
            doing_inventory_command: 0,
            last_command: b' ',
            command_count: 0,
            character_died_from: [0; MORIA_MESSAGE_SIZE],
            treasure: GameTreasure::default(),
            screen: GameScreen::default(),
        }
    }
}

/// Single save/load + reset unit for all mutable game state.
#[derive(Clone, Debug)]
pub struct State {
    pub game: GameState,
    pub py: Player,
    pub dg: Dungeon,
    pub rng: Rng,
    pub normal_table: [u16; NORMAL_TABLE_SIZE],
    pub sorted_objects: [i16; MAX_DUNGEON_OBJECTS as usize],
    pub treasure_levels: [i16; TREASURE_MAX_LEVELS as usize + 1],
    pub monster_levels: [i16; MON_MAX_LEVELS as usize + 1],
    pub monsters: [Monster; MON_TOTAL_ALLOCATIONS as usize],
    pub creature_recall: [Recall; MON_MAX_CREATURES as usize],
    pub objects_identified: [u8; OBJECT_IDENT_SIZE as usize],
    pub flavor: crate::identification::FlavorTables,
    pub messages: [Vtype; MESSAGE_HISTORY_SIZE],
    pub last_message_id: i16,
    pub message_ready_to_print: bool,
    pub screen_has_changed: bool,
    pub next_free_monster_id: i16,
    pub monster_multiply_total: i16,
    pub hack_monptr: i32,
    pub missiles_counter: i16,
    pub stores: [Store; MAX_STORES as usize],
    pub options: Options,
    pub config_save_game: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            game: GameState::default(),
            py: Player::default(),
            dg: Dungeon::default(),
            rng: Rng::default(),
            normal_table: NORMAL_TABLE_VALUES,
            sorted_objects: [0; MAX_DUNGEON_OBJECTS as usize],
            treasure_levels: [0; TREASURE_MAX_LEVELS as usize + 1],
            monster_levels: [0; MON_MAX_LEVELS as usize + 1],
            monsters: [Monster::default(); MON_TOTAL_ALLOCATIONS as usize],
            creature_recall: [Recall::default(); MON_MAX_CREATURES as usize],
            objects_identified: [0; OBJECT_IDENT_SIZE as usize],
            flavor: crate::identification::FlavorTables::default(),
            messages: [[0; MORIA_MESSAGE_SIZE]; MESSAGE_HISTORY_SIZE],
            last_message_id: 0,
            message_ready_to_print: false,
            screen_has_changed: false,
            next_free_monster_id: 0,
            monster_multiply_total: 0,
            hack_monptr: -1,
            missiles_counter: 0,
            stores: [Store::default(); MAX_STORES as usize],
            options: Options::default(),
            config_save_game: "game.sav".to_string(),
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

const STATE_REENTRY_HINT: &str = "\
nested State borrow via with_state/with_state_mut. \
Hold only a short borrow, copy Inventory/flags/coords, release, then call \
re-entering helpers (item_description, terminal::*, player_stat_*, display_*). \
See store::display_store_inventory / dungeon_los::look_see.";

// Debug-only reentrancy tracking for clearer panics than bare RefCell.
// 0 = unlocked, 1 = shared (with_state), 2 = exclusive (with_state_mut).
#[cfg(debug_assertions)]
thread_local! {
    static STATE_BORROW_KIND: Cell<u8> = const { Cell::new(0) };
    static STATE_BORROW_DEPTH: Cell<u32> = const { Cell::new(0) };
}

#[cfg(debug_assertions)]
#[allow(
    clippy::panic,
    clippy::manual_assert,
    reason = "intentional panic on nested game-state RefCell re-entry; assert! used for nested borrow diagnostics in debug builds"
)]
fn enter_state_shared() {
    STATE_BORROW_KIND.with(|kind| {
        assert!(
            kind.get() != 2,
            "with_state re-entered while with_state_mut borrow is active \
             (nested RefCell borrow — snapshot state, release, then call helpers)"
        );
        kind.set(1);
    });
    STATE_BORROW_DEPTH.with(|d| d.set(d.get() + 1));
}

#[cfg(debug_assertions)]
fn exit_state_shared() {
    STATE_BORROW_DEPTH.with(|d| {
        let n = d.get().saturating_sub(1);
        d.set(n);
        if n == 0 {
            STATE_BORROW_KIND.with(|kind| kind.set(0));
        }
    });
}

#[cfg(debug_assertions)]
#[allow(
    clippy::panic,
    clippy::manual_assert,
    reason = "intentional panic on nested game-state RefCell re-entry; assert! used for nested borrow diagnostics in debug builds"
)]
fn enter_state_exclusive() {
    STATE_BORROW_KIND.with(|kind| {
        assert!(
            kind.get() == 0,
            "with_state_mut re-entered while game state is already borrowed \
             (nested RefCell borrow — snapshot state, release, then call helpers)"
        );
        kind.set(2);
    });
    STATE_BORROW_DEPTH.with(|d| d.set(1));
}

#[cfg(debug_assertions)]
fn exit_state_exclusive() {
    STATE_BORROW_DEPTH.with(|d| d.set(0));
    STATE_BORROW_KIND.with(|kind| kind.set(0));
}

#[cfg(debug_assertions)]
struct SharedBorrowGuard;

#[cfg(debug_assertions)]
impl Drop for SharedBorrowGuard {
    fn drop(&mut self) {
        exit_state_shared();
    }
}

#[cfg(debug_assertions)]
struct ExclusiveBorrowGuard;

#[cfg(debug_assertions)]
impl Drop for ExclusiveBorrowGuard {
    fn drop(&mut self) {
        exit_state_exclusive();
    }
}

#[allow(
    clippy::panic,
    reason = "intentional panic on nested game-state RefCell re-entry"
)]
pub fn with_state<R>(f: impl FnOnce(&State) -> R) -> R {
    #[cfg(debug_assertions)]
    {
        enter_state_shared();
        let _guard = SharedBorrowGuard;
        STATE.with(|s| {
            let borrow = s
                .try_borrow()
                .unwrap_or_else(|_| panic!("{STATE_REENTRY_HINT}"));
            f(&borrow)
        })
    }
    #[cfg(not(debug_assertions))]
    {
        STATE.with(|s| {
            let borrow = s
                .try_borrow()
                .unwrap_or_else(|_| panic!("{STATE_REENTRY_HINT}"));
            f(&borrow)
        })
    }
}

#[allow(
    clippy::panic,
    reason = "intentional panic on nested game-state RefCell re-entry"
)]
pub fn with_state_mut<R>(f: impl FnOnce(&mut State) -> R) -> R {
    #[cfg(debug_assertions)]
    {
        enter_state_exclusive();
        let _guard = ExclusiveBorrowGuard;
        STATE.with(|s| {
            let mut borrow = s
                .try_borrow_mut()
                .unwrap_or_else(|_| panic!("{STATE_REENTRY_HINT}"));
            f(&mut borrow)
        })
    }
    #[cfg(not(debug_assertions))]
    {
        STATE.with(|s| {
            let mut borrow = s
                .try_borrow_mut()
                .unwrap_or_else(|_| panic!("{STATE_REENTRY_HINT}"));
            f(&mut borrow)
        })
    }
}

/// Deterministic reset/reseed used by the harness between golden runs.
pub fn reset_for_new_game(seed: Option<u32>) {
    STATE.with(|s| *s.borrow_mut() = State::default());
    if let Some(s) = seed {
        crate::rng::set_seed(s);
    }
}

/// Port of `randomNumber` in game.cpp.
pub fn random_number(max: i32) -> i32 {
    with_state_mut(|state| random_number_state(state, max))
}

pub(crate) fn random_number_state(state: &mut State, max: i32) -> i32 {
    (rnd_state(state) % max) + 1
}

/// Port of `randomNumberNormalDistribution` in game.cpp.
pub fn random_number_normal_distribution(mean: i32, standard: i32) -> i32 {
    with_state_mut(|state| random_number_normal_distribution_state(state, mean, standard))
}

pub(crate) fn random_number_normal_distribution_state(
    state: &mut State,
    mean: i32,
    standard: i32,
) -> i32 {
    let tmp = random_number_state(state, SHRT_MAX);

    if tmp == SHRT_MAX {
        let mut offset = 4 * standard + random_number_state(state, standard);
        if random_number_state(state, 2) == 1 {
            offset = -offset;
        }
        return mean + offset;
    }

    let mut low = 0;
    let mut iindex = NORMAL_TABLE_SIZE >> 1;
    let mut high = NORMAL_TABLE_SIZE;

    loop {
        if state.normal_table[iindex] as i32 == tmp || high == low + 1 {
            break;
        }

        if state.normal_table[iindex] as i32 > tmp {
            high = iindex;
            iindex = low + ((iindex - low) >> 1);
        } else {
            low = iindex;
            iindex = iindex + ((high - iindex) >> 1);
        }
    }

    if (state.normal_table[iindex] as i32) < tmp {
        iindex += 1;
    }

    let mut offset =
        ((standard * iindex as i32) + (NORMAL_TABLE_SD as i32 >> 1)) / NORMAL_TABLE_SD as i32;

    if random_number_state(state, 2) == 1 {
        offset = -offset;
    }

    mean + offset
}

/// Port of `seedsInitialize` in game.cpp.
pub fn seeds_initialize(seed: u32) {
    with_state_mut(|state| {
        let mut clock_var = if seed == 0 {
            get_current_unix_time()
        } else {
            seed
        };

        state.game.magic_seed = clock_var as i32 as u32;
        clock_var = clock_var.wrapping_add(8762);
        state.game.town_seed = clock_var as i32 as u32;
        clock_var = clock_var.wrapping_add(113_452);
        set_seed_state(state, clock_var);

        let mut count = random_number_state(state, 100) as u32;
        while count != 0 {
            rnd_state(state);
            count -= 1;
        }
    });
}

/// Port of `seedSet` in game.cpp.
pub(crate) fn seed_set_state(state: &mut State, seed: u32) {
    state.rng.old_seed = state.rng.seed;
    set_seed_state(state, seed);
}

pub fn seed_set(seed: u32) {
    with_state_mut(|state| seed_set_state(state, seed));
}

/// Port of `seedResetToOldSeed` in game.cpp.
pub(crate) fn seed_reset_to_old_seed_state(state: &mut State) {
    set_seed_state(state, state.rng.old_seed);
}

pub fn seed_reset_to_old_seed() {
    with_state_mut(seed_reset_to_old_seed_state);
}

/// Read-only copy of a small scalar from `State.game`.
pub fn game<F, R>(f: F) -> R
where
    F: FnOnce(&GameState) -> R,
{
    with_state(|s| f(&s.game))
}

/// Read-only copy of a small scalar from `State.py`.
pub fn py<F, R>(f: F) -> R
where
    F: FnOnce(&Player) -> R,
{
    with_state(|s| f(&s.py))
}

/// Read-only copy of a small scalar from `State.dg`.
pub fn dg<F, R>(f: F) -> R
where
    F: FnOnce(&Dungeon) -> R,
{
    with_state(|s| f(&s.dg))
}

/// Mutate `State.game` inside a short-lived borrow.
#[macro_export]
macro_rules! g {
    ($($body:tt)*) => {
        $crate::game::with_state_mut(|state| {
            let game = &mut state.game;
            $($body)*
        })
    };
}

/// Mutate `State.py` inside a short-lived borrow.
#[macro_export]
macro_rules! py {
    ($($body:tt)*) => {
        $crate::game::with_state_mut(|state| {
            let py = &mut state.py;
            $($body)*
        })
    };
}

// C++ `Game_t game = Game_t{};` (game.cpp:14) — realized by `State::default()`.
// (No separate global; residuals read/write `State.game` / `State.options` via `with_state`.)

/// Terminal/UI callbacks for lifecycle helpers (avoids `game` ↔ `ui_io` module cycle).
#[derive(Clone, Copy)]
pub struct GameUiHooks {
    pub flush_input_buffer: fn(),
    pub terminal_restore: fn(),
    pub get_command: fn(&str, &mut u8) -> bool,
    pub get_key_input: fn() -> u8,
    pub put_string: fn(&str, Coord_t),
    pub put_string_clear_to_eol: fn(&str, Coord_t),
    pub move_cursor: fn(Coord_t),
    pub erase_line: fn(Coord_t),
    pub terminal_bell_sound: fn() -> isize,
}

thread_local! {
    static GAME_UI_HOOKS: RefCell<Option<GameUiHooks>> = const { RefCell::new(None) };
}

/// Register UI hooks from `ui_io` (production + test stub paths).
#[doc(hidden)]
pub fn install_game_ui_hooks(hooks: GameUiHooks) {
    GAME_UI_HOOKS.with(|h| *h.borrow_mut() = Some(hooks));
}

#[allow(
    clippy::panic,
    reason = "UI hooks are installed once at startup via register_game_ui_hooks()"
)]
fn game_ui() -> GameUiHooks {
    let hooks = GAME_UI_HOOKS.with(|h| *h.borrow());
    let Some(hooks) = hooks else {
        panic!("game UI hooks not installed — call ui_io::register_game_ui_hooks()");
    };
    hooks
}

/// One row of the C++ `game_options[]` table (game.cpp:118–134).
#[derive(Clone, Copy)]
pub struct GameOptionEntry {
    pub prompt: &'static str,
    pub get: fn(&Options) -> bool,
    pub set: fn(&mut Options, bool),
}

/// Ordered `game_options[]` — prompt strings and field order are save/UX significant.
pub fn game_options_table() -> &'static [GameOptionEntry] {
    static TABLE: [GameOptionEntry; 11] = [
        GameOptionEntry {
            prompt: "Running: cut known corners",
            get: |o| o.run_cut_corners,
            set: |o, v| o.run_cut_corners = v,
        },
        GameOptionEntry {
            prompt: "Running: examine potential corners",
            get: |o| o.run_examine_corners,
            set: |o, v| o.run_examine_corners = v,
        },
        GameOptionEntry {
            prompt: "Running: print self during run",
            get: |o| o.run_print_self,
            set: |o, v| o.run_print_self = v,
        },
        GameOptionEntry {
            prompt: "Running: stop when map sector changes",
            get: |o| o.find_bound,
            set: |o, v| o.find_bound = v,
        },
        GameOptionEntry {
            prompt: "Running: run through open doors",
            get: |o| o.run_ignore_doors,
            set: |o, v| o.run_ignore_doors = v,
        },
        GameOptionEntry {
            prompt: "Prompt to pick up objects",
            get: |o| o.prompt_to_pickup,
            set: |o, v| o.prompt_to_pickup = v,
        },
        GameOptionEntry {
            prompt: "Rogue like commands",
            get: |o| o.use_roguelike_keys,
            set: |o, v| o.use_roguelike_keys = v,
        },
        GameOptionEntry {
            prompt: "Show weights in inventory",
            get: |o| o.show_inventory_weights,
            set: |o, v| o.show_inventory_weights = v,
        },
        GameOptionEntry {
            prompt: "Highlight and notice mineral seams",
            get: |o| o.highlight_seams,
            set: |o, v| o.highlight_seams = v,
        },
        GameOptionEntry {
            prompt: "Beep for invalid character",
            get: |o| o.error_beep_sound,
            set: |o, v| o.error_beep_sound = v,
        },
        GameOptionEntry {
            prompt: "Display rest/repeat counts",
            get: |o| o.display_counts,
            set: |o, v| o.display_counts = v,
        },
    ];
    &TABLE
}

/// C++ `snprintf(..., "%-38s: %s", prompt, val ? "yes" : "no ")` (game.cpp:143).
pub fn format_option_line(prompt: &str, value: bool) -> String {
    let yes_no = if value { "yes" } else { "no " };
    format!("{prompt:<38}: {yes_no}")
}

/// C++ game.cpp lines 224–232.
pub fn get_random_direction() -> i32 {
    loop {
        let dir = random_number(9);
        if dir != 5 {
            return dir;
        }
    }
}

/// C++ game.cpp lines 235–258 (`mapRoguelikeKeysToKeypad`).
pub fn map_roguelike_keys_to_keypad(command: u8) -> u8 {
    match command {
        b'h' => b'4',
        b'y' => b'7',
        b'k' => b'8',
        b'u' => b'9',
        b'l' => b'6',
        b'n' => b'3',
        b'j' => b'2',
        b'b' => b'1',
        b'.' => b'5',
        other => other,
    }
}

/// C++ `setGameOptions()` (game.cpp:137–200).
pub fn set_game_options() {
    let ui = game_ui();
    let table = game_options_table();
    let max = table.len();

    (ui.put_string_clear_to_eol)(
        "  ESC when finished, y/n to set options, <return> or - to move cursor",
        Coord_t { y: 0, x: 0 },
    );

    for (index, entry) in table.iter().enumerate() {
        let line =
            with_state(|state| format_option_line(entry.prompt, (entry.get)(&state.options)));
        (ui.put_string_clear_to_eol)(
            &line,
            Coord_t {
                y: index as i32 + 1,
                x: 0,
            },
        );
    }
    (ui.erase_line)(Coord_t {
        y: max as i32 + 1,
        x: 0,
    });

    let mut option_id = 0usize;
    loop {
        (ui.move_cursor)(Coord_t {
            y: option_id as i32 + 1,
            x: 40,
        });

        match (ui.get_key_input)() {
            ESCAPE => return,
            b'-' => {
                if option_id > 0 {
                    option_id -= 1;
                } else {
                    option_id = max - 1;
                }
            }
            b' ' | b'\n' | b'\r' => {
                if option_id + 1 < max {
                    option_id += 1;
                } else {
                    option_id = 0;
                }
            }
            b'y' | b'Y' => {
                (ui.put_string)(
                    "yes",
                    Coord_t {
                        y: option_id as i32 + 1,
                        x: 40,
                    },
                );
                with_state_mut(|state| (table[option_id].set)(&mut state.options, true));
                if option_id + 1 < max {
                    option_id += 1;
                } else {
                    option_id = 0;
                }
            }
            b'n' | b'N' => {
                (ui.put_string)(
                    "no ",
                    Coord_t {
                        y: option_id as i32 + 1,
                        x: 40,
                    },
                );
                with_state_mut(|state| (table[option_id].set)(&mut state.options, false));
                if option_id + 1 < max {
                    option_id += 1;
                } else {
                    option_id = 0;
                }
            }
            _ => {
                (ui.terminal_bell_sound)();
            }
        }
    }
}

thread_local! {
    static TEST_DIRECTION: std::cell::Cell<Option<i32>> = const { std::cell::Cell::new(None) };
}

/// Test hook for direction-taking player actions.
#[doc(hidden)]
pub fn test_set_direction(direction: Option<i32>) {
    TEST_DIRECTION.with(|d| d.set(direction));
}

/// C++ `getDirectionWithMemory` (game.cpp:262–296).
pub fn get_direction_with_memory(prompt: Option<&str>, direction: &mut i32) -> bool {
    if let Some(dir) = TEST_DIRECTION.with(std::cell::Cell::get) {
        *direction = dir;
        return true;
    }

    if with_state(|state| state.game.use_last_direction) {
        *direction = with_state(|state| i32::from(state.py.prev_dir));
        return true;
    }

    let prompt = prompt.unwrap_or("Which direction?");
    let ui = game_ui();
    let mut command = 0u8;

    loop {
        let old_count = with_state(|state| state.game.command_count);
        if !(ui.get_command)(prompt, &mut command) {
            with_state_mut(|state| state.game.player_free_turn = true);
            return false;
        }
        with_state_mut(|state| state.game.command_count = old_count);

        if with_state(|state| state.options.use_roguelike_keys) {
            command = map_roguelike_keys_to_keypad(command);
        }

        if (b'1'..=b'9').contains(&command) && command != b'5' {
            let dir = i32::from(command - b'0');
            with_state_mut(|state| state.py.prev_dir = command - b'0');
            *direction = dir;
            return true;
        }

        (ui.terminal_bell_sound)();
    }
}

/// Port of `getAllDirections` in game.cpp lines 300–320.
pub fn get_all_directions(prompt: &str, direction: &mut i32) -> bool {
    let ui = game_ui();
    let mut command = 0u8;

    loop {
        if !(ui.get_command)(prompt, &mut command) {
            with_state_mut(|state| state.game.player_free_turn = true);
            return false;
        }

        if with_state(|state| state.options.use_roguelike_keys) {
            command = map_roguelike_keys_to_keypad(command);
        }

        if (b'1'..=b'9').contains(&command) {
            *direction = i32::from(command - b'0');
            return true;
        }

        (ui.terminal_bell_sound)();
    }
}

/// C++ `ui.h` `ESCAPE` (`'\033'`).
const ESCAPE: u8 = 0o33;

thread_local! {
    static TEST_EXIT_PROGRAM_CALLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static TEST_SKIP_PROCESS_EXIT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static TEST_CAPTURE_ABORT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static TEST_ABORT_OUTPUT: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Test hook: record [`exit_program`] without terminating the process.
#[doc(hidden)]
pub fn test_exit_program_called() -> bool {
    TEST_EXIT_PROGRAM_CALLED.with(std::cell::Cell::get)
}

#[doc(hidden)]
pub fn test_reset_exit_program_called() {
    TEST_EXIT_PROGRAM_CALLED.with(|c| c.set(false));
}

/// Port of `validGameVersion` in game.cpp lines 204–218.
pub fn valid_game_version(major: u8, minor: u8, patch: u8) -> bool {
    if major != 5 {
        return false;
    }
    if minor < 2 {
        return false;
    }
    if minor == 2 && patch < 2 {
        return false;
    }
    minor <= 7
}

/// Port of `isCurrentGameVersion` in game.cpp lines 220–222.
pub fn is_current_game_version(major: u8, minor: u8, patch: u8) -> bool {
    major == crate::version::CURRENT_VERSION_MAJOR
        && minor == crate::version::CURRENT_VERSION_MINOR
        && patch == crate::version::CURRENT_VERSION_PATCH
}

#[doc(hidden)]
pub fn test_set_skip_process_exit(skip: bool) {
    TEST_SKIP_PROCESS_EXIT.with(|c| c.set(skip));
}

#[doc(hidden)]
pub fn test_set_capture_abort(capture: bool) {
    TEST_CAPTURE_ABORT.with(|c| c.set(capture));
    if capture {
        TEST_ABORT_OUTPUT.with(|o| *o.borrow_mut() = None);
    }
}

fn pre_exit_sequence() {
    if let Some(hooks) = GAME_UI_HOOKS.with(|h| *h.borrow()) {
        (hooks.flush_input_buffer)();
        (hooks.terminal_restore)();
    }
    TEST_EXIT_PROGRAM_CALLED.with(|c| c.set(true));
}

/// Test seam: pre-exit flush/restore without terminating.
#[doc(hidden)]
pub fn test_pre_exit_sequence() {
    pre_exit_sequence();
}

/// Format bytes printed by `abort_program` after terminal restore.
pub fn abort_program_output(msg: &str) -> String {
    format!("Program was manually aborted with the message:\n{msg}\n")
}

/// C++ game.cpp lines 323–326.
pub fn exit_program() {
    pre_exit_sequence();
    if TEST_SKIP_PROCESS_EXIT.with(std::cell::Cell::get) {
        return;
    }
    std::process::exit(0);
}

/// C++ game.cpp lines 330–338.
pub fn abort_program(msg: &str) {
    pre_exit_sequence();
    let output = abort_program_output(msg);
    if TEST_CAPTURE_ABORT.with(std::cell::Cell::get) {
        TEST_ABORT_OUTPUT.with(|o| *o.borrow_mut() = Some(output));
        return;
    }
    let _ = write!(io::stdout(), "{output}");
    if TEST_SKIP_PROCESS_EXIT.with(std::cell::Cell::get) {
        return;
    }
    std::process::exit(0);
}

#[doc(hidden)]
pub fn test_take_abort_output() -> Option<String> {
    TEST_ABORT_OUTPUT.with(|o| o.borrow_mut().take())
}

/// Mutate `State.dg` inside a short-lived borrow.
#[macro_export]
macro_rules! dg {
    ($($body:tt)*) => {
        $crate::game::with_state_mut(|state| {
            let dg = &mut state.dg;
            $($body)*
        })
    };
}
