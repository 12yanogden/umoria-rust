//! Umoria library crate.
#![allow(
    dead_code,
    reason = "public API surface includes symbols not yet referenced from all call sites"
)]

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

pub mod ui;
pub mod ui_inventory;
pub mod ui_io;

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

pub mod entry;
pub mod game_death;
pub mod game_files;
pub mod game_run;
pub mod game_save;
pub mod scores;
