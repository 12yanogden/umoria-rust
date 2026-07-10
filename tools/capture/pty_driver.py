#!/usr/bin/env python3
"""Phase 1.4.3 - OS-uniform pseudo-terminal driver for the reference binary.

ncurses requires a real tty (``initscr``), so a plain stdin pipe is not enough.
This wrapper spawns the Umoria reference binary under a pty with a fixed window
size (24x80) and ``TERM=xterm``, feeds a recorded keystroke script with pacing,
captures the raw pty output, and emits a stable ``*.screen`` text.

The ``*.screen`` is produced by feeding the raw pty bytes through a real terminal
emulator (``pyte``) into a fixed 24x80 cell buffer and dumping the final screen.
This is the correct model of "what is on screen": every write is applied at its
cursor position, so the final visible screen is identical regardless of how the
byte stream was chunked across reads or how many intermediate redraws ncurses
emitted. The older approach (strip ANSI escapes and concatenate the whole byte
stream) was timing/chunking dependent and is kept only as a graceful fallback if
``pyte`` is unavailable. See phase_1.4.6.

It is a capture *tool* only: it never modifies game logic and links nothing.

Usage:
    pty_driver.py --binary PATH --seed N [--save PATH] --keys FILE
                  [--raw FILE] [--screen FILE] [--cwd DIR]
                  [--char-delay SEC] [--timeout SEC]
"""
import argparse
import errno
import fcntl
import os
import pty
import re
import select
import struct
import sys
import termios
import time


def set_winsize(fd, rows, cols):
    winsize = struct.pack("HHHH", rows, cols, 0, 0)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, winsize)


def run(args):
    with open(args.keys, "rb") as fh:
        keys = fh.read()

    env = dict(os.environ)
    env["TERM"] = "xterm"
    env["LINES"] = "24"
    env["COLS"] = "80"

    argv = [args.binary, "-s", str(args.seed)]
    for extra in args.extra_arg or []:
        argv.append(extra)
    if args.save:
        argv.append(args.save)

    pid, master_fd = pty.fork()
    if pid == 0:
        # Child: became the session leader with the slave pty as controlling tty.
        try:
            set_winsize(sys.stdout.fileno(), 24, 80)
        except Exception:
            pass
        if args.cwd:
            os.chdir(args.cwd)
        os.execvpe(argv[0], argv, env)
        os._exit(127)  # exec failed

    # Parent.
    try:
        set_winsize(master_fd, 24, 80)
    except Exception:
        pass

    raw = bytearray()
    key_index = 0
    deadline = time.time() + args.timeout
    last_write = 0.0

    def drain():
        while True:
            r, _, _ = select.select([master_fd], [], [], 0)
            if master_fd not in r:
                return True
            try:
                chunk = os.read(master_fd, 4096)
            except OSError as e:
                if e.errno in (errno.EIO,):
                    return False  # child exited / pty closed
                raise
            if not chunk:
                return False
            raw.extend(chunk)

    alive = True
    while alive and time.time() < deadline:
        r, _, _ = select.select([master_fd], [], [], 0.02)
        if master_fd in r:
            try:
                chunk = os.read(master_fd, 4096)
            except OSError as e:
                if e.errno in (errno.EIO,):
                    alive = False
                    break
                raise
            if not chunk:
                alive = False
                break
            raw.extend(chunk)

        now = time.time()
        if key_index < len(keys) and (now - last_write) >= args.char_delay:
            try:
                os.write(master_fd, keys[key_index:key_index + 1])
            except OSError:
                alive = False
                break
            key_index += 1
            last_write = now

        # If all keys sent, give the game a moment then keep draining until exit.
        if key_index >= len(keys) and not alive:
            break

    # Final drain.
    time.sleep(0.2)
    try:
        alive = drain()
    except Exception:
        pass

    try:
        os.close(master_fd)
    except OSError:
        pass

    # Reap the child.
    status = None
    try:
        _, status = os.waitpid(pid, os.WNOHANG)
    except OSError:
        pass
    if status is None:
        try:
            os.kill(pid, 9)
            os.waitpid(pid, 0)
        except OSError:
            pass

    if args.raw:
        with open(args.raw, "wb") as fh:
            fh.write(raw)

    screen = render_screen(raw)
    if args.screen:
        with open(args.screen, "w", encoding="utf-8", errors="replace") as fh:
            fh.write(screen)

    return 0


SCREEN_ROWS = 24
SCREEN_COLS = 80


def render_screen(raw: bytes) -> str:
    """Render the raw pty byte stream to the final 24x80 screen.

    Uses a real terminal emulator (``pyte``) so the result is deterministic
    regardless of read chunking / intermediate redraws. Falls back to the
    legacy escape-stripping concatenator if ``pyte`` is not importable.
    """
    try:
        import pyte
    except ImportError:
        sys.stderr.write(
            "pty_driver: pyte not installed; falling back to non-deterministic "
            "strip_terminal (pip install pyte for stable *.screen goldens)\n"
        )
        return strip_terminal(raw)

    screen = pyte.Screen(SCREEN_COLS, SCREEN_ROWS)
    stream = pyte.ByteStream(screen)
    stream.feed(bytes(raw))
    lines = [line.rstrip() for line in screen.display]
    # Trim trailing blank lines for a compact, still-deterministic dump.
    while lines and not lines[-1]:
        lines.pop()
    return "\n".join(lines) + "\n"


# Legacy fallback (used only when pyte is unavailable): strip escape sequences
# and concatenate the byte stream. NOTE: this is timing/chunking dependent and
# does NOT model cursor positioning; render_screen() (pyte) is authoritative.
# CSI / OSC / other escape sequences -> stripped for a comparable screen.
_ANSI_CSI = re.compile(rb"\x1b\[[0-?]*[ -/]*[@-~]")
_ANSI_OSC = re.compile(rb"\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)")
_ANSI_OTHER = re.compile(rb"\x1b[()][0-9A-Za-z]")
_ANSI_SIMPLE = re.compile(rb"\x1b[=>NODEHM78]")


def strip_terminal(raw: bytes) -> str:
    b = bytes(raw)
    b = _ANSI_OSC.sub(b"", b)
    b = _ANSI_CSI.sub(b"", b)
    b = _ANSI_OTHER.sub(b"", b)
    b = _ANSI_SIMPLE.sub(b"", b)
    b = b.replace(b"\r\n", b"\n").replace(b"\r", b"\n")
    b = b.replace(b"\x00", b"").replace(b"\x07", b"").replace(b"\x08", b"")
    text = b.decode("utf-8", errors="replace")
    # Trim trailing whitespace per line for stability.
    lines = [ln.rstrip() for ln in text.split("\n")]
    return "\n".join(lines)


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--binary", required=True)
    p.add_argument("--seed", type=int, required=True)
    p.add_argument("--save", default=None)
    p.add_argument("--extra-arg", action="append",
                   help="extra CLI flag to append after -s SEED (e.g. -d); repeatable")
    p.add_argument("--keys", required=True)
    p.add_argument("--raw", default=None)
    p.add_argument("--screen", default=None)
    p.add_argument("--cwd", default=None)
    p.add_argument("--char-delay", type=float, default=0.05)
    p.add_argument("--timeout", type=float, default=30.0)
    args = p.parse_args()
    return run(args)


if __name__ == "__main__":
    sys.exit(main())
