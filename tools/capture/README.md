# Golden-capture tooling

Capture tooling for the Rust `umoria` binary. Records golden artifacts under
`tests/golden/` for differential / transcript tests.

## Scripts

| Script | Purpose |
|--------|---------|
| `capture_rng.sh` + `rng_capture` bin | Emits RNG goldens (z10001, sequences, normal_table) via `cargo run --bin rng_capture`. |
| `pty_driver.py` | OS-uniform pseudo-terminal driver: spawns `umoria -s <seed> [extra] [save]` with `TERM=xterm LINES=24 COLS=80`, feeds a `*.keys` script with pacing, captures raw pty output and renders the final `*.screen`. |
| `play.sh` | Thin wrapper: `play.sh <name> <seed>` → `transcripts/<name>.{screen,raw}` and copies `game.sav` → `save/<name>.sav`. Uses `target/release/umoria` (or `$UMORIA_BIN`). |
| `regen.sh` | Rebuild Rust binary, re-capture all goldens, rewrite `manifest.json`. |
| `compare_masked.py` | Decode-aware masked comparison (see below). |

## Deterministic screen rendering (`*.screen`)

`pty_driver.py` renders each `*.screen` by feeding the raw pty byte stream
through a real terminal emulator (`pyte`, `pip install pyte`) into a fixed
**24×80** cell buffer and dumping the final visible screen (each line
`rstrip`ped, trailing blank lines trimmed). Because every write is applied at its
cursor position, the final screen is identical no matter how the byte stream was
chunked across reads or how many intermediate redraws ncurses emitted.

The previous approach stripped ANSI escapes and concatenated the *entire* raw
byte stream. That captured "everything ever written" (all intermediate redraws)
rather than "what is on screen", making it timing/chunking dependent: stable
within one capture burst but drifting across bursts. It is retained only as a
graceful fallback (with a stderr warning) when `pyte` is not installed.
`checks/07` enforces cross-burst stability and ≤ 80-column lines. The raw
`*.raw` byte stream is itself timing-variable and is excluded from the manifest.

## XOR obfuscation & masked comparison

Save/score byte streams use a *chained* running XOR key:
`ciphertext[i] = ciphertext[i-1] ^ plaintext[i]` (`wr_byte` / `wr_long` in
`src/game_save.rs`). A single differing plaintext byte therefore cascades to
every following ciphertext byte, so comparison must be done on the **decoded
plaintext**, where `plaintext[i] = ciphertext[i] ^ ciphertext[i-1]` is local.

`compare_masked.py --scheme {save,score,raw}` decodes both files and compares
outside `--mask OFFSET:LEN` ranges (offsets in the decoded plaintext).

### Clock-derived (volatile) byte ranges

macOS SIP blocks `libfaketime` DYLD injection, so time cannot be frozen; masked
comparison is the **primary** strategy (`faketime: null` in the manifest).
Measured offsets:

| Golden | Field | Decoded offset:len | Source |
|--------|-------|--------------------|--------|
| `save/newchar_seed42.sav` | save timestamp `l` = `get_current_unix_time()` | `3894:4` | `game_save.rs` (sv_write) |
| `save/newchar_seed42.sav` | `py.misc.date_of_birth` | `3910:4` | `game_save.rs` (sv_write) |
| `scores/*.dat` | `HighScore.birth_date` (record N) | `8 + 64*N : 4` | `game_save.rs` / scores |

Save offsets are scenario-specific (they follow the variable-length recall /
inventory / store data). They were located empirically by decoding two captures
taken ≥1 s apart and diffing the plaintext, then confirmed against the field
order in `sv_write()` (only the timestamp `l` and `date_of_birth` differ; the
intervening `character_died_from="(saved)"` and `max_score` are deterministic).

## Score WRITE golden (documented limitation)

`scores/scores_initial.dat` is a genuine populated high-score file (one real
high-score record) and pins the score-file read/format; `scores/scores_screen.txt`
pins the `umoria -d` display. A *fresh* score-WRITE golden
(`scores_<scenario>.dat` produced by `record_new_high_score` on death) is
**deferred**: recording a score requires the character to die in-game, i.e. a
long brittle deterministic keystroke death sequence disproportionate to this
capture leaf. The write byte layout is already pinned by `scores_initial.dat`,
the birth_date masking mechanism is proven in `checks/06`, and the identical
clock field (`date_of_birth`) is exercised byte-for-byte by the save golden
(`checks/05`).
