//! Port of src/data_stores.cpp — immutable store definition tables.

use std::sync::LazyLock;

use crate::player::PLAYER_MAX_RACES;
use crate::store::{
    Store, MAX_STORES, SPEECH_BUYING_HAGGLE as SPEECH_BUYING_HAGGLE_LEN,
    SPEECH_BUYING_HAGGLE_FINAL as SPEECH_BUYING_HAGGLE_FINAL_LEN,
    SPEECH_GET_OUT_OF_MY_STORE as SPEECH_GET_OUT_OF_MY_STORE_LEN,
    SPEECH_HAGGLING_TRY_AGAIN as SPEECH_HAGGLING_TRY_AGAIN_LEN,
    SPEECH_INSULTED_HAGGLING_DONE as SPEECH_INSULTED_HAGGLING_DONE_LEN,
    SPEECH_SALE_ACCEPTED as SPEECH_SALE_ACCEPTED_LEN,
    SPEECH_SELLING_HAGGLE as SPEECH_SELLING_HAGGLE_LEN,
    SPEECH_SELLING_HAGGLE_FINAL as SPEECH_SELLING_HAGGLE_FINAL_LEN,
    SPEECH_SORRY as SPEECH_SORRY_LEN, STORE_MAX_ITEM_TYPES,
};

pub static STORES: LazyLock<[Store; MAX_STORES as usize]> =
    LazyLock::new(|| [Store::default(); MAX_STORES as usize]);

pub const STORE_CHOICES: [[u16; STORE_MAX_ITEM_TYPES as usize]; MAX_STORES as usize] = [
    // General Store
    [
        366, 365, 364, 84, 84, 365, 123, 366, 365, 350, 349, 348, 347, 346, 346, 345, 345, 345,
        344, 344, 344, 344, 344, 344, 344, 344,
    ],
    // Armory
    [
        94, 95, 96, 109, 103, 104, 105, 106, 110, 111, 112, 114, 116, 124, 125, 126, 127, 129, 103,
        104, 124, 125, 91, 92, 95, 96,
    ],
    // Weaponsmith
    [
        29, 30, 34, 37, 45, 49, 57, 58, 59, 65, 67, 68, 73, 74, 75, 77, 79, 80, 81, 83, 29, 30, 80,
        83, 80, 83,
    ],
    // Temple
    [
        322, 323, 324, 325, 180, 180, 233, 237, 240, 241, 361, 362, 57, 58, 59, 260, 358, 359, 265,
        237, 237, 240, 240, 241, 323, 359,
    ],
    // Alchemy shop
    [
        173, 174, 175, 351, 351, 352, 353, 354, 355, 356, 357, 206, 227, 230, 236, 252, 253, 352,
        353, 354, 355, 356, 359, 363, 359, 359,
    ],
    // Magic-User store
    [
        318, 141, 142, 153, 164, 167, 168, 140, 319, 320, 320, 321, 269, 270, 282, 286, 287, 292,
        293, 294, 295, 308, 269, 290, 319, 282,
    ],
];

pub const RACE_GOLD_ADJUSTMENTS: [[u8; PLAYER_MAX_RACES as usize]; PLAYER_MAX_RACES as usize] = [
    // Hum, HfE, Elf, Hal, Gno, Dwa, HfO, HfT — Human
    [100, 105, 105, 110, 113, 115, 120, 125],
    // Half-Elf
    [110, 100, 100, 105, 110, 120, 125, 130],
    // Elf
    [110, 105, 100, 105, 110, 120, 125, 130],
    // Halfling
    [115, 110, 105, 95, 105, 110, 115, 130],
    // Gnome
    [115, 115, 110, 105, 95, 110, 115, 130],
    // Dwarf
    [115, 120, 120, 110, 110, 95, 125, 135],
    // Half-Orc
    [115, 120, 125, 115, 115, 130, 110, 115],
    // Half-Troll
    [110, 115, 115, 110, 110, 130, 110, 110],
];

pub const SPEECH_SALE_ACCEPTED: [&str; SPEECH_SALE_ACCEPTED_LEN as usize] = [
    "Done!",
    "Accepted!",
    "Fine.",
    "Agreed!",
    "Ok.",
    "Taken!",
    "You drive a hard bargain, but taken.",
    "You'll force me bankrupt, but it's a deal.",
    "Sigh.  I'll take it.",
    "My poor sick children may starve, but done!",
    "Finally!  I accept.",
    "Robbed again.",
    "A pleasure to do business with you!",
    "My spouse will skin me, but accepted.",
];

pub const SPEECH_SELLING_HAGGLE_FINAL: [&str; SPEECH_SELLING_HAGGLE_FINAL_LEN as usize] = [
    "%A2 is my final offer; take it or leave it.",
    "I'll give you no more than %A2.",
    "My patience grows thin.  %A2 is final.",
];

pub const SPEECH_SELLING_HAGGLE: [&str; SPEECH_SELLING_HAGGLE_LEN as usize] = [
    "%A1 for such a fine item?  HA!  No less than %A2.",
    "%A1 is an insult!  Try %A2 gold pieces.",
    "%A1?!?  You would rob my poor starving children?",
    "Why, I'll take no less than %A2 gold pieces.",
    "Ha!  No less than %A2 gold pieces.",
    "Thou knave!  No less than %A2 gold pieces.",
    "%A1 is far too little, how about %A2?",
    "I paid more than %A1 for it myself, try %A2.",
    "%A1?  Are you mad?!?  How about %A2 gold pieces?",
    "As scrap this would bring %A1.  Try %A2 in gold.",
    "May the fleas of 1000 Orcs molest you.  I want %A2.",
    "My mother you can get for %A1, this costs %A2.",
    "May your chickens grow lips.  I want %A2 in gold!",
    "Sell this for such a pittance?  Give me %A2 gold.",
    "May the Balrog find you tasty!  %A2 gold pieces?",
    "Your mother was a Troll!  %A2 or I'll tell.",
];

pub const SPEECH_BUYING_HAGGLE_FINAL: [&str; SPEECH_BUYING_HAGGLE_FINAL_LEN as usize] = [
    "I'll pay no more than %A1; take it or leave it.",
    "You'll get no more than %A1 from me.",
    "%A1 and that's final.",
];

pub const SPEECH_BUYING_HAGGLE: [&str; SPEECH_BUYING_HAGGLE_LEN as usize] = [
    "%A2 for that piece of junk?  No more than %A1.",
    "For %A2 I could own ten of those.  Try %A1.",
    "%A2?  NEVER!  %A1 is more like it.",
    "Let's be reasonable. How about %A1 gold pieces?",
    "%A1 gold for that junk, no more.",
    "%A1 gold pieces and be thankful for it!",
    "%A1 gold pieces and not a copper more.",
    "%A2 gold?  HA!  %A1 is more like it.",
    "Try about %A1 gold.",
    "I wouldn't pay %A2 for your children, try %A1.",
    "*CHOKE* For that!?  Let's say %A1.",
    "How about %A1?",
    "That looks war surplus!  Say %A1 gold.",
    "I'll buy it as scrap for %A1.",
    "%A2 is too much, let us say %A1 gold.",
];

pub const SPEECH_INSULTED_HAGGLING_DONE: [&str; SPEECH_INSULTED_HAGGLING_DONE_LEN as usize] = [
    "ENOUGH!  You have abused me once too often!",
    "THAT DOES IT!  You shall waste my time no more!",
    "This is getting nowhere.  I'm going home!",
    "BAH!  No more shall you insult me!",
    "Begone!  I have had enough abuse for one day.",
];

pub const SPEECH_GET_OUT_OF_MY_STORE: [&str; SPEECH_GET_OUT_OF_MY_STORE_LEN as usize] = [
    "Out of my place!",
    "out... Out... OUT!!!",
    "Come back tomorrow.",
    "Leave my place.  Begone!",
    "Come back when thou art richer.",
];

pub const SPEECH_HAGGLING_TRY_AGAIN: [&str; SPEECH_HAGGLING_TRY_AGAIN_LEN as usize] = [
    "You will have to do better than that!",
    "That's an insult!",
    "Do you wish to do business or not?",
    "Hah!  Try again.",
    "Ridiculous!",
    "You've got to be kidding!",
    "You'd better be kidding!",
    "You try my patience.",
    "I don't hear you.",
    "Hmmm, nice weather we're having.",
];

pub const SPEECH_SORRY: [&str; SPEECH_SORRY_LEN as usize] = [
    "I must have heard you wrong.",
    "What was that?",
    "I'm sorry, say that again.",
    "What did you say?",
    "Sorry, what was that again?",
];
