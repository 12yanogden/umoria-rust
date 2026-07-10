mod common;

#[test]
fn probe_rng_values() {
    use umoria::game::{random_number, reset_for_new_game};
    reset_for_new_game(Some(42));
    eprintln!("rn7={}", random_number(7));
    reset_for_new_game(Some(42));
    eprintln!("frost rolls");
    for _ in 0..3 { eprintln!("100={}", random_number(100)); }
    reset_for_new_game(Some(99));
    eprintln!("poison roll={}", random_number(10));
    reset_for_new_game(Some(42));
    eprintln!("light roll={}", random_number(250));
}
