//! Crate binary entry point — `main.cpp` maps to `entry.rs`; this file is the thin wrapper.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    std::process::exit(umoria::entry::run_with_args(&args) as i32);
}
