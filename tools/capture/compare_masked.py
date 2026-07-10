#!/usr/bin/env python3
"""Phase 1.4.4 - Masked comparison of Umoria golden binary files.

On macOS SIP blocks ``libfaketime`` DYLD injection, so clock-derived bytes in
the save/score files cannot be frozen. Instead we compare *outside* the
documented volatile byte ranges (the save timestamp ``l`` and the player
``date_of_birth``; the score ``birth_date``).

Umoria's save/score byte streams are XOR-obfuscated with a *chained* running
key: ``ciphertext[i] = ciphertext[i-1] ^ plaintext[i]`` (see ``wrByte``/``wrLong``
in ``src/game_save.cpp``). A single differing plaintext byte therefore cascades
through every following ciphertext byte, so masking must be done on the DECODED
plaintext, where ``plaintext[i] = ciphertext[i] ^ ciphertext[i-1]`` is local.

Decode schemes (which byte indices reset the running key to 0 before the byte):
  * ``save``  - the three version bytes plus the ``char_tmp`` byte each reset
                ``xor_byte`` to 0, i.e. indices {0, 1, 2, 3} (``saveChar()``).
  * ``score`` - the three version bytes reset (indices {0, 1, 2}); each 64-byte
                ``HighScore_t`` record additionally re-seeds the key from its own
                first raw byte (``readHighScore`` does ``xor_byte = getByte()``),
                which the chained decode already handles for the record body.
  * ``raw``   - no decoding (compare bytes as-is).

Usage:
    compare_masked.py --scheme save --mask 3894:4 --mask 3910:4 FILE_A FILE_B
    compare_masked.py --scheme raw FILE_A FILE_B
Exit code 0 == equal (outside masks), 1 == differ, 2 == usage/size error.
"""
import argparse
import sys


def decode(data: bytes, scheme: str) -> bytes:
    if scheme == "raw":
        return bytes(data)
    if scheme == "save":
        resets = {0, 1, 2, 3}
    elif scheme == "score":
        resets = {0, 1, 2}
    else:
        raise ValueError(f"unknown scheme {scheme!r}")
    out = bytearray()
    prev = 0
    for i, b in enumerate(data):
        if i in resets:
            prev = 0
        out.append(b ^ prev)
        prev = b
    return bytes(out)


def parse_masks(specs):
    ranges = []
    for spec in specs or []:
        off_s, len_s = spec.split(":")
        ranges.append((int(off_s), int(len_s)))
    return ranges


def apply_mask(data: bytes, ranges) -> bytearray:
    out = bytearray(data)
    for off, length in ranges:
        for i in range(off, min(off + length, len(out))):
            out[i] = 0
    return out


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--scheme", choices=["save", "score", "raw"], default="save")
    p.add_argument("--mask", action="append", metavar="OFFSET:LEN",
                   help="volatile plaintext range to ignore (repeatable)")
    p.add_argument("file_a")
    p.add_argument("file_b")
    args = p.parse_args()

    a = open(args.file_a, "rb").read()
    b = open(args.file_b, "rb").read()

    if len(a) != len(b):
        print(f"DIFFER: size {len(a)} != {len(b)}", file=sys.stderr)
        return 1

    ranges = parse_masks(args.mask)
    da = apply_mask(decode(a, args.scheme), ranges)
    db = apply_mask(decode(b, args.scheme), ranges)

    if da == db:
        print(f"EQUAL: {len(a)} bytes identical outside {len(ranges)} masked range(s)")
        return 0

    diffs = [i for i in range(len(da)) if da[i] != db[i]]
    print(f"DIFFER: {len(diffs)} plaintext byte(s) differ outside masks; "
          f"first at {diffs[0] if diffs else '?'}", file=sys.stderr)
    for i in diffs[:16]:
        print(f"  offset {i}: {da[i]:#04x} != {db[i]:#04x}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
