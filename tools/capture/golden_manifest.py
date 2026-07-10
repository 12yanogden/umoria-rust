#!/usr/bin/env python3
"""Phase 1.4.5 - Generate/verify tests/golden/manifest.json.

Enumerates every golden under tests/golden/{rng,save,scores,transcripts}
(excluding checks/, the manifest itself, and volatile *.raw intermediates),
records seed/inputs/env/volatile-byte-range metadata, and computes a sha256 for
each.

For clock-volatile save/score goldens the raw file bytes change every capture
(the timestamp / date_of_birth / birth_date fields), and because of the chained
XOR the change cascades through the whole ciphertext. So those entries are
hashed over the *decoded, masked* plaintext (``hash_method: sha256-masked-<scheme>``)
which is stable across regen; deterministic goldens use ``hash_method: sha256``
over the raw bytes.

Usage:
    golden_manifest.py write    # (re)write tests/golden/manifest.json
    golden_manifest.py verify   # exit 0 iff on-disk manifest matches reality
"""
import hashlib
import json
import os
import subprocess
import sys

from compare_masked import decode, apply_mask

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
GOLDEN = os.path.join(ROOT, "tests", "golden")
MANIFEST = os.path.join(GOLDEN, "manifest.json")
UMORIA_VERSION = "5.7.15"

ENV_2480 = {"TERM": "xterm", "LINES": 24, "COLS": 80}

# Per-file overrides keyed by path relative to tests/golden/. Anything not listed
# falls back to directory-derived defaults (kind + raw sha256).
OVERRIDES = {
    "save/newchar_seed42.sav": {
        "kind": "save",
        "seed": 42,
        "inputs": "transcripts/newchar_seed42.keys",
        "env": ENV_2480,
        "scheme": "save",
        "volatile_byte_ranges": [
            {"offset": 3894, "length": 4, "why": "save timestamp l = getCurrentUnixTime() (game_save.cpp:299)"},
            {"offset": 3910, "length": 4, "why": "py.misc.date_of_birth (game_save.cpp:309)"},
        ],
    },
    "scores/scores_initial.dat": {
        "kind": "scores",
        "note": "pristine committed high-score file (copy of data/scores.dat)",
        "scheme": "score",
        "volatile_byte_ranges": [
            {"offset": 8, "length": 4, "why": "HighScore_t.birth_date record 0 = 8 + 64*N (game_save.cpp:1189)"},
        ],
    },
    "scores/scores_screen.txt": {
        "kind": "scores",
        "seed": 42,
        "inputs": "transcripts/scores_screen.keys",
        "extra_args": ["-d"],
        "env": ENV_2480,
        "note": "umoria -d showScoresScreen() output (deterministic; no timestamps)",
    },
    "transcripts/newchar_seed42.screen": {
        "kind": "transcript",
        "seed": 42,
        "inputs": "transcripts/newchar_seed42.keys",
        "env": ENV_2480,
    },
    "transcripts/newchar_seed42.keys": {"kind": "transcript", "note": "keystroke input script"},
    "transcripts/scores_screen.keys": {"kind": "transcript", "note": "keystroke input script"},
}

KIND_BY_DIR = {"rng": "rng", "save": "save", "scores": "scores", "transcripts": "transcript"}


def iter_golden_files():
    for sub in ("rng", "save", "scores", "transcripts"):
        d = os.path.join(GOLDEN, sub)
        if not os.path.isdir(d):
            continue
        for name in sorted(os.listdir(d)):
            if name.startswith("."):
                continue  # dot/intermediate files (e.g. .scores_screen.raw)
            if name.endswith(".raw"):
                continue  # raw pty output is a volatile intermediate, not a golden
            yield sub, name, os.path.join(d, name)


def seed_from_name(name):
    # rnd_seed42.txt / randomNumber_seed12345.txt / normalDist_seed1.txt
    if "_seed" in name:
        tail = name.split("_seed", 1)[1]
        digits = tail.split(".", 1)[0]
        if digits.isdigit():
            return int(digits)
    if name == "z10001.txt":
        return 1
    return None


def hash_entry(path, override):
    data = open(path, "rb").read()
    scheme = override.get("scheme")
    ranges = override.get("volatile_byte_ranges")
    if scheme and ranges:
        mask = [(r["offset"], r["length"]) for r in ranges]
        masked = apply_mask(decode(data, scheme), mask)
        return "sha256-masked-" + scheme, hashlib.sha256(bytes(masked)).hexdigest()
    return "sha256", hashlib.sha256(data).hexdigest()


def build_goldens():
    goldens = []
    for sub, name, path in iter_golden_files():
        rel = f"{sub}/{name}"
        ov = OVERRIDES.get(rel, {})
        kind = ov.get("kind", KIND_BY_DIR[sub])
        stem = os.path.splitext(name)[0]
        entry = {"id": f"{kind}_{stem}", "kind": kind, "file": rel}

        seed = ov.get("seed", seed_from_name(name) if sub == "rng" else None)
        if seed is not None:
            entry["seed"] = seed
        for key in ("inputs", "extra_args", "env", "volatile_byte_ranges", "note"):
            if key in ov:
                entry[key] = ov[key]

        method, digest = hash_entry(path, ov)
        entry["hash_method"] = method
        entry["sha256"] = digest
        goldens.append(entry)
    return goldens


def _cmd(args):
    try:
        return subprocess.check_output(args, stderr=subprocess.DEVNULL).decode().splitlines()[0].strip()
    except Exception:
        return None


def ncurses_version():
    for tool in ("ncurses6-config", "ncurses5-config"):
        v = _cmd([tool, "--version"])
        if v:
            return f"{tool} {v}"
    v = _cmd(["pkg-config", "--modversion", "ncurses"])
    return f"pkg-config ncurses {v}" if v else "system ncurses (version unknown)"


def generated_with():
    meta = {"cmake": None, "compiler": None, "os": None}
    meta_path = os.path.join(ROOT, "build", "ref", "build_meta.txt")
    if os.path.isfile(meta_path):
        for line in open(meta_path):
            if ":" in line:
                k, v = line.split(":", 1)
                if k.strip() in meta:
                    meta[k.strip()] = v.strip()
    meta["cmake"] = meta["cmake"] or _cmd(["cmake", "--version"])
    meta["compiler"] = meta["compiler"] or _cmd(["c++", "--version"])
    meta["os"] = meta["os"] or _cmd(["uname", "-a"])
    meta["ncurses"] = ncurses_version()
    meta["faketime"] = None  # macOS SIP blocks libfaketime; masked comparison used instead
    meta["regen_command"] = "tools/capture/regen.sh"
    return meta


def build_manifest():
    return {
        "umoria_version": UMORIA_VERSION,
        "generated_with": generated_with(),
        "goldens": build_goldens(),
    }


def write():
    m = build_manifest()
    with open(MANIFEST, "w") as fh:
        json.dump(m, fh, indent=2)
        fh.write("\n")
    print(f"wrote {MANIFEST} ({len(m['goldens'])} goldens)")
    return 0


def verify():
    if not os.path.isfile(MANIFEST):
        print("FAIL: manifest.json missing", file=sys.stderr)
        return 1
    on_disk = json.load(open(MANIFEST))
    fresh = build_manifest()

    listed = {g["file"] for g in on_disk.get("goldens", [])}
    actual = {f"{sub}/{name}" for sub, name, _ in iter_golden_files()}
    missing = actual - listed
    extra = listed - actual
    if missing:
        print(f"FAIL: goldens not listed in manifest: {sorted(missing)}", file=sys.stderr)
        return 1
    if extra:
        print(f"FAIL: manifest lists nonexistent goldens: {sorted(extra)}", file=sys.stderr)
        return 1

    fresh_by_file = {g["file"]: g for g in fresh["goldens"]}
    for g in on_disk["goldens"]:
        f = g["file"]
        fh = fresh_by_file[f]
        if g.get("sha256") != fh["sha256"] or g.get("hash_method") != fh["hash_method"]:
            print(f"FAIL: hash mismatch for {f}: manifest={g.get('hash_method')}:{g.get('sha256')} "
                  f"disk={fh['hash_method']}:{fh['sha256']}", file=sys.stderr)
            return 1
    print(f"OK: manifest lists {len(listed)} goldens, all sha256 match on-disk files")
    return 0


def main():
    if len(sys.argv) != 2 or sys.argv[1] not in ("write", "verify"):
        print(__doc__)
        return 2
    return write() if sys.argv[1] == "write" else verify()


if __name__ == "__main__":
    sys.exit(main())
