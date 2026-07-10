//! Crate binary entry point — `main.cpp` maps to `entry.rs`; this file is the thin wrapper.
fn main() {
    std::process::exit(umoria::entry::run_with_args(std::env::args().collect()) as i32);
}
