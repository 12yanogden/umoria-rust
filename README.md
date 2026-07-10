# Umoria (Rust)

A faithful Rust translation of [Umoria 5.7.15](https://github.com/dungeons-of-moria/umoria) — *The Dungeons of Moria*, a single-player dungeon simulation originally written by Robert Alan Koeneke (first public release 1983), ported to C by James E. Wilson in 1988 as Umoria.

Moria/Umoria has had many variants over the years, with [*Angband*](http://rephial.org/) being the most well known. Umoria was also an inspiration for one of the most commercially successful action roguelike games, *Diablo*.

This repository is a **line-faithful Rust port** of that classic. The goal is observable parity with the C++ 5.7.15 reference — not new gameplay. Upstream project sites: [umoria.org](https://umoria.org/) and [dungeons-of-moria/umoria](https://github.com/dungeons-of-moria/umoria).

**Distributed via git only** — the crate sets `publish = false` and is not on [crates.io](https://crates.io/).


## Platforms

**Primary:** Unix-like systems with ncurses — **macOS** and **Linux**.

Windows is not a proven target for this Rust build. Some platform stubs exist in the port, but play and CI are oriented around macOS/Linux. Contributions that harden Windows support are welcome; do not assume it works out of the box.


## Building

Requirements:

- A recent stable Rust toolchain (MSRV **1.80**; see `rust-toolchain.toml`)
- System **ncurses** development libraries and **pkg-config**
  - macOS (Homebrew): `brew install ncurses pkg-config`
  - Debian/Ubuntu: `sudo apt-get install libncurses-dev pkg-config`

```bash
cargo build --release
```

The binary is `target/release/umoria`.


## Playing

Run from the repository root so relative paths resolve (`data/…`, `LICENSE`, default `game.sav` / `scores.dat`):

```bash
./target/release/umoria
# or
cargo run --release
```

Useful options (same CLI as upstream):

```text
umoria [OPTIONS] SAVEGAME

SAVEGAME is an optional save game filename (default: game.sav)

Options:
    -n           Force start of a new game
    -r           Enable classic roguelike keys on startup
    -d           Display high scores and exit
    -s NUMBER    Game seed (decimal, max 2147483647)
    -v           Print version and exit
    -h           Display help
```

Example: `./target/release/umoria -n -s 42`


## Dual tree: Rust + C++ reference

| Tree | Role |
|------|------|
| **Rust** (`Cargo.toml`, `src/*.rs`) | Primary build for players and day-to-day development |
| **C++** (`src/*.cpp` / `*.h`, CMake) | Differential reference — golden capture, tools under `tools/capture/`, and fidelity checks |

Players should use the Rust binary. The C++ sources remain in-tree so goldens and capture tooling can rebuild the reference and compare behavior. You do not need CMake to play.


## Behavioral parity (honest scope)

This is a **faithful translation** with strong automated checks:

- RNG sequence goldens (bit-exact PMMLCG / related helpers)
- Save and score file round-trips against C++-captured fixtures
- Scripted terminal screen replay for short recorded paths

Coverage is **strongest for core RNG and the short new-character path**. It is **not** a proof of full-playthrough identity with every corner of a long dungeon run. If you see behavior that differs from Umoria 5.7.15, please [report it as a bug](CONTRIBUTING.md).


## Historical documents

Most of the original document files from the Umoria 5.6 sources live in [`historical/`](historical). That includes older changelogs, the original Moria Manual, and FAQ material — useful for history even where details are outdated.


## Code of Conduct and contributions

See the [Code of Conduct](CODE_OF_CONDUCT.md) and [contributing guide](CONTRIBUTING.md).


## License and attribution

Umoria is released under the [GNU General Public License v3.0 or later](LICENSE) (`GPL-3.0-or-later`).

Original authors and maintainers include **Robert Alan Koeneke** (Moria), **James E. Wilson** (Umoria), and many later contributors — see [`AUTHORS`](AUTHORS). The 5.7.x restoration work was led in the [dungeons-of-moria](https://github.com/dungeons-of-moria) project.

In 2007 Ben Asselstine and Ben Shadwick started the [*free-moria*](http://free-moria.sourceforge.net/) project to re-license UMoria 5.5.2 under GPL-2 by obtaining permission from all contributing authors. A year later they succeeded, and in late 2008 official maintainer David Grabiner released Umoria 5.6 under a GPL-3.0-or-later license.
