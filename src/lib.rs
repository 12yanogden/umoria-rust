//! Umoria library crate — 1:1 mirror of `src/*.cpp` translation units.
#![allow(
    dead_code,
    reason = "translation units retain C++ symbols not yet referenced from Rust call sites"
)]

// --- phase 2: foundation (config, helpers, rng, data tables, types) ---
pub mod config;
pub mod data_creatures;
pub mod data_player;
pub mod data_recall;
pub mod data_store_owners;
pub mod data_stores;
pub mod data_tables;
pub mod data_treasure;
pub mod dice;
pub mod dungeon_tile;
pub mod game;
pub mod helpers;
pub mod rng;
pub mod types;
pub mod version;

// --- phase 3: UI layer ---
pub mod ui;
pub mod ui_inventory;
pub mod ui_io;

// --- phase 4: gameplay systems ---
pub mod character;
pub mod dungeon;
pub mod dungeon_generate;
pub mod dungeon_los;
pub mod game_objects;
pub mod identification;
pub mod inventory;
pub mod mage_spells;
pub mod monster;
pub mod monster_manager;
pub mod player;
pub mod player_bash;
pub mod player_eat;
pub mod player_magic;
pub mod player_move;
pub mod player_pray;
pub mod player_quaff;
pub mod player_run;
pub mod player_stats;
pub mod player_throw;
pub mod player_traps;
pub mod player_tunnel;
pub mod recall;
pub mod scrolls;
pub mod spells;
pub mod staves;
pub mod store;
pub mod store_inventory;
pub mod treasure;
pub mod wizard;

// --- phase 5: game loop, save/load, scores ---
pub mod entry;
pub mod game_death;
pub mod game_files;
pub mod game_run;
pub mod game_save;
pub mod scores;
