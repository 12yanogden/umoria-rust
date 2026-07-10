//! Port of src/dice.cpp / src/dice.h.

use crate::game::random_number;

/// Port of `Dice_t` in dice.h.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dice {
    pub dice: u8,
    pub sides: u8,
}

/// Port of `diceRoll` in dice.cpp.
pub fn dice_roll(dice: Dice) -> i32 {
    let mut sum = 0;
    for _ in 0..dice.dice {
        sum += random_number(dice.sides as i32);
    }
    sum
}

/// Port of `maxDiceRoll` in dice.cpp.
pub fn max_dice_roll(dice: Dice) -> i32 {
    dice.dice as i32 * dice.sides as i32
}
