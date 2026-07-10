//! `data_store_owners` store owner table parity.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use umoria::data_store_owners::STORE_OWNERS;
use umoria::data_stores::{
    SPEECH_BUYING_HAGGLE, SPEECH_BUYING_HAGGLE_FINAL, SPEECH_GET_OUT_OF_MY_STORE,
    SPEECH_HAGGLING_TRY_AGAIN, SPEECH_INSULTED_HAGGLING_DONE, SPEECH_SALE_ACCEPTED,
    SPEECH_SELLING_HAGGLE, SPEECH_SELLING_HAGGLE_FINAL, SPEECH_SORRY,
};
use umoria::store::{
    MAX_OWNERS, SPEECH_BUYING_HAGGLE as SPEECH_BUYING_HAGGLE_N,
    SPEECH_BUYING_HAGGLE_FINAL as SPEECH_BUYING_HAGGLE_FINAL_N,
    SPEECH_GET_OUT_OF_MY_STORE as SPEECH_GET_OUT_OF_MY_STORE_N,
    SPEECH_HAGGLING_TRY_AGAIN as SPEECH_HAGGLING_TRY_AGAIN_N,
    SPEECH_INSULTED_HAGGLING_DONE as SPEECH_INSULTED_HAGGLING_DONE_N,
    SPEECH_SALE_ACCEPTED as SPEECH_SALE_ACCEPTED_N,
    SPEECH_SELLING_HAGGLE as SPEECH_SELLING_HAGGLE_N,
    SPEECH_SELLING_HAGGLE_FINAL as SPEECH_SELLING_HAGGLE_FINAL_N, SPEECH_SORRY as SPEECH_SORRY_N,
};

// ---------------------------------------------------------------------------
// 1. Length assertions
// ---------------------------------------------------------------------------
#[test]
fn store_owners_length() {
    assert_eq!(STORE_OWNERS.len(), 18);
    assert_eq!(STORE_OWNERS.len(), MAX_OWNERS as usize);
}

#[test]
fn speech_array_lengths() {
    assert_eq!(SPEECH_SALE_ACCEPTED.len(), 14);
    assert_eq!(SPEECH_SALE_ACCEPTED.len(), SPEECH_SALE_ACCEPTED_N as usize);
    assert_eq!(SPEECH_SELLING_HAGGLE_FINAL.len(), 3);
    assert_eq!(
        SPEECH_SELLING_HAGGLE_FINAL.len(),
        SPEECH_SELLING_HAGGLE_FINAL_N as usize
    );
    assert_eq!(SPEECH_SELLING_HAGGLE.len(), 16);
    assert_eq!(
        SPEECH_SELLING_HAGGLE.len(),
        SPEECH_SELLING_HAGGLE_N as usize
    );
    assert_eq!(SPEECH_BUYING_HAGGLE_FINAL.len(), 3);
    assert_eq!(
        SPEECH_BUYING_HAGGLE_FINAL.len(),
        SPEECH_BUYING_HAGGLE_FINAL_N as usize
    );
    assert_eq!(SPEECH_BUYING_HAGGLE.len(), 15);
    assert_eq!(SPEECH_BUYING_HAGGLE.len(), SPEECH_BUYING_HAGGLE_N as usize);
    assert_eq!(SPEECH_INSULTED_HAGGLING_DONE.len(), 5);
    assert_eq!(
        SPEECH_INSULTED_HAGGLING_DONE.len(),
        SPEECH_INSULTED_HAGGLING_DONE_N as usize
    );
    assert_eq!(SPEECH_GET_OUT_OF_MY_STORE.len(), 5);
    assert_eq!(
        SPEECH_GET_OUT_OF_MY_STORE.len(),
        SPEECH_GET_OUT_OF_MY_STORE_N as usize
    );
    assert_eq!(SPEECH_HAGGLING_TRY_AGAIN.len(), 10);
    assert_eq!(
        SPEECH_HAGGLING_TRY_AGAIN.len(),
        SPEECH_HAGGLING_TRY_AGAIN_N as usize
    );
    assert_eq!(SPEECH_SORRY.len(), 5);
    assert_eq!(SPEECH_SORRY.len(), SPEECH_SORRY_N as usize);
}

// ---------------------------------------------------------------------------
// 2. Spot-check store_owners[0] and [17] (src/data_store_owners.cpp)
// ---------------------------------------------------------------------------
#[test]
fn store_owners_first_entry() {
    let o = &STORE_OWNERS[0];
    assert_eq!(o.name, "Erick the Honest       (Human)      General Store");
    assert_eq!(o.max_cost, 250);
    assert_eq!(o.max_inflate, 175);
    assert_eq!(o.min_inflate, 108);
    assert_eq!(o.haggles_per, 4);
    assert_eq!(o.race, 0);
    assert_eq!(o.max_insults, 12);
}

#[test]
fn store_owners_last_entry() {
    let o = &STORE_OWNERS[17];
    assert_eq!(o.name, "Inglorian the Mage     (Human?)     Magic Shop");
    assert_eq!(o.max_cost, 32000);
    assert_eq!(o.max_inflate, 200);
    assert_eq!(o.min_inflate, 110);
    assert_eq!(o.haggles_per, 7);
    assert_eq!(o.race, 0);
    assert_eq!(o.max_insults, 10);
}

// ---------------------------------------------------------------------------
// 3. Spot-check speech arrays (src/data_store_owners.cpp)
// ---------------------------------------------------------------------------
#[test]
fn speech_sale_accepted_spot_checks() {
    assert_eq!(SPEECH_SALE_ACCEPTED[0], "Done!");
    assert_eq!(
        SPEECH_SALE_ACCEPTED[13],
        "My spouse will skin me, but accepted."
    );
}

#[test]
fn speech_selling_haggle_spot_checks() {
    assert_eq!(
        SPEECH_SELLING_HAGGLE[0],
        "%A1 for such a fine item?  HA!  No less than %A2."
    );
    assert_eq!(
        SPEECH_SELLING_HAGGLE[15],
        "Your mother was a Troll!  %A2 or I'll tell."
    );
}

#[test]
fn speech_sorry_spot_checks() {
    assert_eq!(SPEECH_SORRY[0], "I must have heard you wrong.");
    assert_eq!(SPEECH_SORRY[4], "Sorry, what was that again?");
}
