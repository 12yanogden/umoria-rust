//! Phase 2.11 — data_stores.cpp static store data tables parity.

use umoria::data_stores::{RACE_GOLD_ADJUSTMENTS, STORE_CHOICES};
use umoria::player::PLAYER_MAX_RACES;
use umoria::store::{MAX_STORES, STORE_MAX_ITEM_TYPES};

// ---------------------------------------------------------------------------
// 1. Dimension tests
// ---------------------------------------------------------------------------
#[test]
fn race_gold_adjustments_dimensions() {
    assert_eq!(RACE_GOLD_ADJUSTMENTS.len(), 8);
    assert_eq!(RACE_GOLD_ADJUSTMENTS.len(), PLAYER_MAX_RACES as usize);
    for row in &RACE_GOLD_ADJUSTMENTS {
        assert_eq!(row.len(), 8);
        assert_eq!(row.len(), PLAYER_MAX_RACES as usize);
    }
}

#[test]
fn store_choices_dimensions() {
    assert_eq!(STORE_CHOICES.len(), 6);
    assert_eq!(STORE_CHOICES.len(), MAX_STORES as usize);
    for row in &STORE_CHOICES {
        assert_eq!(row.len(), 26);
        assert_eq!(row.len(), STORE_MAX_ITEM_TYPES as usize);
    }
}

// ---------------------------------------------------------------------------
// 2. race_gold_adjustments spot-checks (src/data_stores.cpp lines 12–22)
// ---------------------------------------------------------------------------
#[test]
fn race_gold_adjustments_diagonal() {
    assert_eq!(RACE_GOLD_ADJUSTMENTS[0][0], 100);
    assert_eq!(RACE_GOLD_ADJUSTMENTS[1][1], 100);
    assert_eq!(RACE_GOLD_ADJUSTMENTS[2][2], 100);
    assert_eq!(RACE_GOLD_ADJUSTMENTS[3][3], 95);
    assert_eq!(RACE_GOLD_ADJUSTMENTS[4][4], 95);
    assert_eq!(RACE_GOLD_ADJUSTMENTS[5][5], 95);
    assert_eq!(RACE_GOLD_ADJUSTMENTS[6][6], 110);
    assert_eq!(RACE_GOLD_ADJUSTMENTS[7][7], 110);
}

#[test]
fn race_gold_adjustments_off_diagonal() {
    assert_eq!(RACE_GOLD_ADJUSTMENTS[0][7], 125);
    assert_eq!(RACE_GOLD_ADJUSTMENTS[7][0], 110);
    assert_eq!(RACE_GOLD_ADJUSTMENTS[5][7], 135);
    assert_eq!(RACE_GOLD_ADJUSTMENTS[6][5], 130);
    assert_eq!(RACE_GOLD_ADJUSTMENTS[0][3], 110);
}

// ---------------------------------------------------------------------------
// 3. store_choices spot-checks (src/data_stores.cpp lines 25–56)
// ---------------------------------------------------------------------------
#[test]
fn store_choices_general_store_row() {
    assert_eq!(
        STORE_CHOICES[0],
        [
            366, 365, 364, 84, 84, 365, 123, 366, 365, 350, 349, 348, 347, 346, 346, 345, 345, 345,
            344, 344, 344, 344, 344, 344, 344, 344,
        ]
    );
}

#[test]
fn store_choices_anchor_cells() {
    assert_eq!(STORE_CHOICES[1][0], 94); // Armory
    assert_eq!(STORE_CHOICES[2][0], 29); // Weaponsmith
    assert_eq!(STORE_CHOICES[3][0], 322); // Temple
    assert_eq!(STORE_CHOICES[4][0], 173); // Alchemy
    assert_eq!(STORE_CHOICES[5][0], 318); // Magic-User
    assert_eq!(STORE_CHOICES[5][25], 282);
}
