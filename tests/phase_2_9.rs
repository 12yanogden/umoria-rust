//! Phase 2.9 — data_recall.cpp recall description string tables.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::data_recall::{
    RECALL_DESCRIPTION_ATTACK_METHOD, RECALL_DESCRIPTION_ATTACK_TYPE, RECALL_DESCRIPTION_BREATH,
    RECALL_DESCRIPTION_HOW_MUCH, RECALL_DESCRIPTION_MOVE, RECALL_DESCRIPTION_SPELL,
    RECALL_DESCRIPTION_WEAKNESS,
};

// ---------------------------------------------------------------------------
// 1. Length assertions (phase_2.4 / recall.h)
// ---------------------------------------------------------------------------
#[test]
fn recall_description_array_lengths() {
    assert_eq!(RECALL_DESCRIPTION_ATTACK_TYPE.len(), 25);
    assert_eq!(RECALL_DESCRIPTION_ATTACK_METHOD.len(), 20);
    assert_eq!(RECALL_DESCRIPTION_HOW_MUCH.len(), 8);
    assert_eq!(RECALL_DESCRIPTION_MOVE.len(), 6);
    assert_eq!(RECALL_DESCRIPTION_SPELL.len(), 15);
    assert_eq!(RECALL_DESCRIPTION_BREATH.len(), 5);
    assert_eq!(RECALL_DESCRIPTION_WEAKNESS.len(), 6);
}

// ---------------------------------------------------------------------------
// 2. First/last spot-checks vs C++ source (src/data_recall.cpp)
// ---------------------------------------------------------------------------
#[test]
fn recall_description_attack_type_spot_checks() {
    let table = &RECALL_DESCRIPTION_ATTACK_TYPE;
    assert_eq!(table[0], "do something undefined");
    assert_eq!(table[24], "absorb charges");
}

#[test]
fn recall_description_attack_method_spot_checks() {
    let table = &RECALL_DESCRIPTION_ATTACK_METHOD;
    assert_eq!(table[0], "make an undefined advance");
    assert_eq!(table[19], "insult");
}

#[test]
fn recall_description_how_much_spot_checks() {
    let table = &RECALL_DESCRIPTION_HOW_MUCH;
    assert_eq!(table[0], " not at all");
    assert_eq!(table[2], "");
    assert_eq!(table[7], " extremely");
}

#[test]
fn recall_description_move_spot_checks() {
    let table = &RECALL_DESCRIPTION_MOVE;
    assert_eq!(table[0], "move invisibly");
    assert_eq!(table[5], "breed explosively");
}

#[test]
fn recall_description_spell_spot_checks() {
    let table = &RECALL_DESCRIPTION_SPELL;
    assert_eq!(table[0], "teleport short distances");
    assert_eq!(table[14], "unknown 2");
}

#[test]
fn recall_description_breath_spot_checks() {
    let table = &RECALL_DESCRIPTION_BREATH;
    assert_eq!(table[0], "lightning");
    assert_eq!(table[4], "fire");
}

#[test]
fn recall_description_weakness_spot_checks() {
    let table = &RECALL_DESCRIPTION_WEAKNESS;
    assert_eq!(table[0], "frost");
    assert_eq!(table[5], "rock remover");
}
