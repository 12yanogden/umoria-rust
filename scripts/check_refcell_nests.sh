#!/usr/bin/env bash
# Mechanical scan for known RefCell nest anti-patterns in src/.
# Catches item_description / terminal I/O / get_input_confirmation called
# directly inside with_state(_mut) closures. Complements the debug_assertions
# reentrancy detector in src/game.rs.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

python3 - <<'PY'
import re
import sys
from pathlib import Path

src = Path("src")
mut_danger = re.compile(
 r"\b("
 r"item_description\s*\(|"
 r"print_message\s*\(|"
 r"get_key_input\s*\(|"
 r"put_string_clear_to_eol\s*\(|"
 r"erase_line\s*\(|"
 r"get_input_confirmation|"
 r"verify_action\s*\(|"
 r"display_inventory_items\s*\(|"
 r"display_equipment\s*\(|"
 r"inventory_item_is_cursed_message\s*\(|"
 r"player_adjust_bonuses_for_item\s*\(|"
 r"player_recalculate_bonuses\s*\(|"
 r"random_number\s*\("
 r")"
)
imm_danger = re.compile(
 r"\b("
 r"print_message\s*\(|"
 r"get_key_input\s*\(|"
 r"put_string_clear_to_eol\s*\(|"
 r"erase_line\s*\(|"
 r"get_input_confirmation|"
 r"verify_action\s*\(|"
 r"display_inventory_items\s*\(|"
 r"display_equipment\s*\("
 r")"
)
pat_mut = re.compile(r"with_state_mut\s*\(\s*\|([^|]*)\|\s*\{", re.M)
pat_imm = re.compile(r"with_state\s*\(\s*\|([^|]*)\|\s*\{", re.M)


def bodies(text, pat):
 for m in pat.finditer(text):
 start = m.end() - 1
 depth = 0
 i = start
 while i < len(text):
 if text[i] == "{":
 depth += 1
 elif text[i] == "}":
 depth -= 1
 if depth == 0:
 yield text.count("\n", 0, m.start()) + 1, text[start : i + 1]
 break
 i += 1


hits = []
for path in sorted(src.rglob("*.rs")):
 text = path.read_text()
 rel = str(path)
 for line, body in bodies(text, pat_mut):
 found = sorted({m.group(0).rstrip("(") for m in mut_danger.finditer(body)})
 # Allow random_number_state; reject bare random_number.
 found = [h for h in found if h != "random_number_state"]
 if "random_number" in found and "random_number_state" in body:
 # still flag bare random_number(
 if not re.search(r"(?<!_state)(?<!_number_state)\brandom_number\s*\(", body.replace("random_number_state", "X")):
 found = [h for h in found if h != "random_number"]
 if found and re.search(r"(?<![_a-zA-Z])item_description\s*\(", body) is None:
 found = [h for h in found if h != "item_description"]
 if found:
 hits.append((rel, line, "with_state_mut", found))
 for line, body in bodies(text, pat_imm):
 found = sorted({m.group(0).rstrip("(") for m in imm_danger.finditer(body)})
 if found:
 hits.append((rel, line, "with_state", found))

if hits:
 print("RefCell nest anti-patterns found:", file=sys.stderr)
 for rel, line, kind, found in hits:
 print(f" [{kind}] {rel}:{line} -> {found}", file=sys.stderr)
 sys.exit(1)

print("No RefCell nest anti-patterns found in src/")
PY
