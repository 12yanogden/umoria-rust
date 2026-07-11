//! Dice roll helpers.

use crate::game::random_number;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dice {
    pub dice: u8,
    pub sides: u8,
}

pub fn dice_roll(dice: Dice) -> i32 {
    let mut sum = 0;
    for _ in 0..dice.dice {
        sum += random_number(dice.sides as i32);
    }
    sum
}

pub fn max_dice_roll(dice: Dice) -> i32 {
    dice.dice as i32 * dice.sides as i32
}
