# Contributing to Umoria

Thanks for your interest in contributing!

These are guidelines, not rigid rules. Use good judgement, and feel free to propose changes to this document in a pull request.

Classic game discussion and history also live under the [Dungeons of Moria organization](https://github.com/dungeons-of-moria) on GitHub.


## Code of Conduct

This project and everyone participating in it is governed by the [Umoria Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to [info@umoria.org](mailto:info@umoria.org).


## Questions and discussions

For general game discussion, try the [MoriaRL Reddit](https://www.reddit.com/r/moriarl/) or the broader [Roguelike Reddit](https://www.reddit.com/r/roguelikes/). Both are friendly communities.


## What should I know before I get started?

**Do not change gameplay.** Umoria’s rules were fine-tuned over many years. Prefer bug fixes, tests, docs, and build/CI hygiene over new features or balance tweaks.

Work that *is* in scope:

- Fixing bugs and regressions (include seed and repro steps)
- Improving tests, goldens, docs, and build/CI hygiene
- Platform fixes on the supported Unix path (macOS/Linux)


## How can I contribute?

### Reporting bugs

Before filing, skim existing issues. If a closed issue matches what you see, open a new one and link the old report.

When you file a bug:

* Use a clear, descriptive title.
* Give exact steps to reproduce (seed, CLI flags, key sequence if possible).
* Describe what you saw vs. what you expected.
* Note OS, terminal, and how you built (`cargo build --release`, commit hash).
* For crashes, include a backtrace (`RUST_BACKTRACE=1`) when you can.


### Code contributions

Before opening a PR:

1. **Format:** `cargo fmt --all`
2. **Lint:** `cargo clippy --all-targets --all-features -- -Dwarnings`
3. **Full local gate:** `./scripts/check.sh` 
 (fmt check, clippy, tests, RefCell-nest scan, docs; runs `cargo deny` if installed)

Style follows normal Rust conventions and this repo’s `clippy.toml` / workspace lints. Prefer matching existing module structure.

**Goldens:** when changing behavior-sensitive code, extend or update differential tests rather than loosening assertions. Regenerating goldens (when intentional) goes through `tools/capture/` — see `tools/capture/README.md`.

General reminders:

* Avoid unnecessary platform-dependent code; Unix (macOS/Linux) is the proven target.
* Preserve integer/overflow and RNG semantics the game relies on.
* Keep commits focused; separate pure refactors from behavior fixes when you can.
