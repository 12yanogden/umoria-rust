//! Port of src/rng.cpp — Schrage PMMLCG proxied through `State.rng`.

use crate::game::{with_state, with_state_mut, State};

pub const RNG_M: i32 = 2_147_483_647;
pub const RNG_A: i32 = 16_807;
pub const RNG_Q: i32 = RNG_M / RNG_A;
pub const RNG_R: i32 = RNG_M % RNG_A;

pub fn get_seed() -> u32 {
    with_state(|s| s.rng.seed)
}

pub fn set_seed(seed: u32) {
    with_state_mut(|s| set_seed_state(s, seed));
}

pub(crate) fn set_seed_state(state: &mut State, seed: u32) {
    state.rng.seed = (seed % (RNG_M as u32 - 1)) + 1;
}

/// Returns a pseudo-random number from set 1, 2, ..., `RNG_M` - 1.
pub fn rnd() -> i32 {
    with_state_mut(rnd_state)
}

pub(crate) fn rnd_state(state: &mut State) -> i32 {
    let high = (state.rng.seed / RNG_Q as u32) as i32;
    let low = (state.rng.seed % RNG_Q as u32) as i32;
    let mut test = RNG_A * low - RNG_R * high;

    if test > 0 {
        state.rng.seed = test as u32;
    } else {
        test += RNG_M;
        state.rng.seed = test as u32;
    }
    test
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::reset_for_new_game;

    #[test]
    fn set_seed_edge_cases_match_cpp_widths() {
        reset_for_new_game(None);
        let cases = [
            (0u32, 1u32),
            (1, 2),
            (RNG_M as u32 - 1, 1),
            (RNG_M as u32, 2),
            (u32::MAX, u32::MAX % (RNG_M as u32 - 1) + 1),
        ];
        for (input, expected) in cases {
            set_seed(input);
            assert_eq!(get_seed(), expected, "set_seed({input})");
        }
    }
}
