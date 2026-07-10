# Contributing to Umoria (Rust)

Thanks for your interest in contributing to this Rust translation of Umoria!

These are guidelines, not rigid rules. Use good judgement, and feel free to propose changes to this document in a pull request.

This tree is a faithful port of [Umoria 5.7.15](https://github.com/dungeons-of-moria/umoria). Upstream discussion and history also live under the [Dungeons of Moria organization](https://github.com/dungeons-of-moria) on GitHub.


## Code of Conduct

This project and everyone participating in it is governed by the [Umoria Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to [info@umoria.org](mailto:info@umoria.org).


## Questions and discussions

For general game discussion, try the [MoriaRL Reddit](https://www.reddit.com/r/moriarl/) or the broader [Roguelike Reddit](https://www.reddit.com/r/roguelikes/). Both are friendly communities.


## What should I know before I get started?

**Do not change gameplay.** Umoria’s rules were fine-tuned over many years. This port’s charter is behavioral fidelity to 5.7.15, not new features or balance tweaks.

Work that *is* in scope:

- Fixing Rust/C++ behavioral diffs (treat them as bugs)
- Improving tests, goldens, docs, and build/CI hygiene
- Clarifying or hardening the dual-tree reference workflow
- Platform fixes on the supported Unix path (macOS/Linux)

The C++ sources remain as a **differential reference** for capture tooling and goldens (`tools/capture/`, `tests/golden/`). Players and normal development use the Rust crate; CMake is for reference builds, not the primary player path.


## How can I contribute?

### Reporting bugs

Before filing, skim existing issues. If a closed issue matches what you see, open a new one and link the old report.

When you file a bug:

* Use a clear, descriptive title.
* Give exact steps to reproduce (seed, CLI flags, key sequence if possible).
* Describe what you saw vs. what Umoria 5.7.15 / the C++ reference does.
* Note OS, terminal, and how you built (`cargo build --release`, commit hash).
* For crashes, include a backtrace (`RUST_BACKTRACE=1`) when you can.

Parity bugs (Rust differs from the C++ reference) are especially valuable — include the seed and any golden or screen evidence you have.


### Code contributions

Before opening a PR:

1. **Format:** `cargo fmt --all`
2. **Lint:** `cargo clippy --all-targets --all-features -- -Dwarnings`
3. **Full local gate:** `./scripts/check.sh`  
   (fmt check, clippy, tests, RefCell-nest scan, docs; runs `cargo deny` if installed)

Style follows normal Rust conventions and this repo’s `clippy.toml` / workspace lints. Prefer matching existing module structure (1:1 with the C++ translation units where practical).

**C++ reference / goldens:** when changing behavior-sensitive code, extend or update differential tests rather than “fixing” by loosening assertions. Regenerating goldens (when intentional) goes through `tools/capture/` — see `tools/capture/README.md`. Do not treat `.clang-format` or C++ style as the main contribution path; that tooling exists for the reference tree only.

General reminders:

* Avoid unnecessary platform-dependent code; Unix (macOS/Linux) is the proven target.
* Preserve integer/overflow and RNG semantics that the C++ game relied on.
* Keep commits focused; separate pure refactors from fidelity fixes when you can.
