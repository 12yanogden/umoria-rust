# Umoria

*The Dungeons of Moria* — a single-player dungeon crawl for the terminal.

Umoria continues the classic Moria lineage begun by Robert Alan Koeneke (1983) and James E. Wilson.

This repository implements Umoria **5.7.15** in Rust (ncurses). Upstream project sites for the classic game: [umoria.org](https://umoria.org/) and [dungeons-of-moria/umoria](https://github.com/dungeons-of-moria/umoria).

**Distributed via git only** — the crate sets `publish = false` and is not on [crates.io](https://crates.io/).


## Platforms

**Primary:** Unix-like systems with ncurses — **macOS** and **Linux**.

Windows is not a proven target. Some platform stubs exist, but play and CI are oriented around macOS/Linux. Contributions that harden Windows support are welcome; do not assume it works out of the box.


## Building

Requirements:

- A recent stable Rust toolchain (MSRV **1.81**; see `rust-toolchain.toml`)
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

Useful options:

```text
umoria [OPTIONS] SAVEGAME

SAVEGAME is an optional save game filename (default: game.sav)

Options:
 -n Force start of a new game
 -r Enable classic roguelike keys on startup
 -d Display high scores and exit
 -s NUMBER Game seed (decimal, max 2147483647)
 -v Print version and exit
 -h Display help
```

Example: `./target/release/umoria -n -s 42`


## Testing and goldens

Automated checks cover RNG sequences, save/score round-trips, and short scripted terminal transcripts. Coverage is strongest for core RNG and the short new-character path. See `tools/capture/README.md` and `CONTRIBUTING.md` for regenerating goldens.


## Historical documents

Older document files from classic Umoria sources live in [`historical/`](historical). That includes older changelogs, the original Moria Manual, and FAQ material — useful for history even where details are outdated.


## Code of Conduct and contributions

See the [Code of Conduct](CODE_OF_CONDUCT.md) and [contributing guide](CONTRIBUTING.md).


## License and attribution

Umoria is released under the [GNU General Public License v3.0 or later](LICENSE) (`GPL-3.0-or-later`).

Original authors and maintainers include **Robert Alan Koeneke** (Moria), **James E. Wilson** (Umoria), and many later contributors — see [`AUTHORS`](AUTHORS). The 5.7.x restoration work was led in the [dungeons-of-moria](https://github.com/dungeons-of-moria) project.

In 2007 Ben Asselstine and Ben Shadwick started the [*free-moria*](http://free-moria.sourceforge.net/) project to re-license UMoria 5.5.2 under GPL-2 by obtaining permission from all contributing authors. A year later they succeeded, and in late 2008 official maintainer David Grabiner released Umoria 5.6 under a GPL-3.0-or-later license.
