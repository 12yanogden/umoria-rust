//! Binary entry point — delegates to [`umoria::entry`].
fn main() {
    let args: Vec<String> = std::env::args().collect();
    std::process::exit(umoria::entry::run_with_args(&args) as i32);
}
