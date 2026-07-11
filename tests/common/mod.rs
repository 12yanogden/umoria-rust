//! Shared helpers for golden / harness integration tests.

#![allow(
    dead_code,
    reason = "helpers may be unused across individual test crates"
)]

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const HEX_WINDOW: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GoldenKind {
    Rng,
    Save,
    Scores,
    Transcript,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VolatileByteRange {
    pub offset: usize,
    pub length: usize,
    #[serde(default)]
    pub why: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoldenEntry {
    pub id: String,
    pub kind: GoldenKind,
    pub file: String,
    #[serde(default)]
    pub seed: Option<u32>,
    #[serde(default)]
    pub inputs: Option<String>,
    #[serde(default)]
    pub extra_args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, Value>,
    #[serde(default)]
    pub volatile_byte_ranges: Vec<VolatileByteRange>,
    #[serde(default)]
    pub note: Option<String>,
    pub hash_method: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeneratedWith {
    #[serde(default)]
    pub cargo: Option<String>,
    #[serde(default)]
    pub rustc: Option<String>,
    #[serde(default)]
    pub cmake: Option<String>,
    #[serde(default)]
    pub compiler: Option<String>,
    pub os: Option<String>,
    pub ncurses: Option<String>,
    pub faketime: Option<String>,
    pub regen_command: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub umoria_version: String,
    pub generated_with: GeneratedWith,
    pub goldens: Vec<GoldenEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    pub offset: usize,
    pub expected: u8,
    pub actual: u8,
    pub expected_len: usize,
    pub actual_len: usize,
}

impl Diff {
    pub fn render(&self) -> String {
        if self.expected_len != self.actual_len {
            return format!(
                "length mismatch at offset {}: expected {} bytes, actual {} bytes",
                self.offset, self.expected_len, self.actual_len
            );
        }

        format!(
            "byte mismatch at offset {:#06x}: expected {:#04x}, actual {:#04x}\n{}",
            self.offset,
            self.expected,
            self.actual,
            hex_window(self.offset, self.expected, self.actual, self.expected_len)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenBuffer {
    rows: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenDiff {
    pub row: usize,
    pub col: usize,
    pub expected: char,
    pub actual: char,
}

impl ScreenDiff {
    pub fn render(&self) -> String {
        format!(
            "screen mismatch at row {} col {}: expected {:?}, actual {:?}",
            self.row, self.col, self.expected, self.actual
        )
    }
}

impl GoldenEntry {
    pub fn path(&self) -> PathBuf {
        golden_root().join(&self.file)
    }

    pub fn env(&self) -> &HashMap<String, Value> {
        &self.env
    }
}

impl ScreenBuffer {
    pub fn from_bytes(data: &[u8]) -> Self {
        let text = String::from_utf8_lossy(data);
        let rows: Vec<String> = text
            .lines()
            .map(str::trim_end)
            .map(str::to_string)
            .collect();
        Self { rows }
    }

    pub fn rows(&self) -> usize {
        self.rows.len()
    }

    pub fn cols(&self) -> usize {
        self.rows
            .iter()
            .map(|row| row.chars().count())
            .max()
            .unwrap_or(0)
    }
}

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

pub fn golden_root() -> PathBuf {
    repo_root().join("tests/golden")
}

pub fn manifest_path() -> PathBuf {
    golden_root().join("manifest.json")
}

pub fn regen_enabled() -> bool {
    env::var("UMORIA_REGEN_GOLDEN").is_ok_and(|value| value == "1")
}

pub fn load_manifest() -> io::Result<Manifest> {
    let raw = fs::read_to_string(manifest_path())?;
    serde_json::from_str(&raw).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

pub fn read_golden_bytes(entry: &GoldenEntry) -> Vec<u8> {
    match fs::read(entry.path()) {
        Ok(bytes) => bytes,
        Err(err) => {
            let path = entry.path();
            assert!(
                path.is_file(),
                "golden file missing for {}: {} ({err})",
                entry.id,
                path.display()
            );
            Vec::new()
        }
    }
}

pub fn verify_manifest(manifest: &Manifest) -> io::Result<()> {
    for entry in &manifest.goldens {
        let path = entry.path();
        if !path.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "manifest entry {} points at missing file {}",
                    entry.id,
                    path.display()
                ),
            ));
        }

        let data = fs::read(&path)?;
        let digest = hash_golden(entry, &data);
        if digest != entry.sha256 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "hash mismatch for {} ({}): manifest={} on-disk={}",
                    entry.id, entry.hash_method, entry.sha256, digest
                ),
            ));
        }
    }
    Ok(())
}

pub fn byte_diff(expected: &[u8], actual: &[u8]) -> Option<Diff> {
    let min_len = expected.len().min(actual.len());
    for offset in 0..min_len {
        if expected[offset] != actual[offset] {
            return Some(Diff {
                offset,
                expected: expected[offset],
                actual: actual[offset],
                expected_len: expected.len(),
                actual_len: actual.len(),
            });
        }
    }

    if expected.len() != actual.len() {
        return Some(Diff {
            offset: min_len,
            expected: expected.get(min_len).copied().unwrap_or(0),
            actual: actual.get(min_len).copied().unwrap_or(0),
            expected_len: expected.len(),
            actual_len: actual.len(),
        });
    }

    None
}

pub fn load_rng_sequence(path: &Path) -> io::Result<Vec<u32>> {
    let raw = fs::read_to_string(path)?;
    let mut values = Vec::new();
    for (line_no, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: u32 = line.parse().map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "invalid rng value on line {} of {}: {err}",
                    line_no + 1,
                    path.display()
                ),
            )
        })?;
        values.push(value);
    }
    Ok(values)
}

pub fn screen_diff(expected: &ScreenBuffer, actual: &ScreenBuffer) -> Option<ScreenDiff> {
    let rows = expected.rows().max(actual.rows());
    for row in 0..rows {
        let expected_row = expected.rows.get(row).map_or("", String::as_str);
        let actual_row = actual.rows.get(row).map_or("", String::as_str);
        let cols = expected_row.chars().count().max(actual_row.chars().count());
        for col in 0..cols {
            let expected_ch = expected_row.chars().nth(col).unwrap_or('\0');
            let actual_ch = actual_row.chars().nth(col).unwrap_or('\0');
            if expected_ch != actual_ch {
                return Some(ScreenDiff {
                    row,
                    col,
                    expected: expected_ch,
                    actual: actual_ch,
                });
            }
        }
    }
    None
}

#[cfg(feature = "differential_live")]
fn render_pty_screen_vt100(raw: &[u8]) -> String {
    let mut parser = vt100::Parser::new(24, 80, 0);
    parser.process(raw);
    let screen = parser.screen();
    let mut lines = Vec::with_capacity(24);
    for row in 0..24 {
        let mut line = String::with_capacity(80);
        for col in 0..80 {
            if let Some(cell) = screen.cell(row, col) {
                line.push_str(&cell.contents());
            }
        }
        lines.push(line.trim_end().to_owned());
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

#[cfg(feature = "differential_live")]
fn render_pty_screen_pyte(raw: &[u8]) -> Option<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    const SCRIPT: &str = r"
import sys
import pyte
raw = sys.stdin.buffer.read()
screen = pyte.Screen(80, 24)
pyte.ByteStream(screen).feed(raw)
lines = [line.rstrip() for line in screen.display]
while lines and not lines[-1]:
    lines.pop()
sys.stdout.write('\n'.join(lines) + '\n')
";

    let mut child = Command::new("python3")
        .args(["-c", SCRIPT])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.as_mut()?.write_all(raw).ok()?;
    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(feature = "differential_live")]
fn render_pty_screen(raw: &[u8]) -> String {
    render_pty_screen_pyte(raw).unwrap_or_else(|| render_pty_screen_vt100(raw))
}

#[cfg(feature = "differential_live")]
pub fn replay_transcript(
    seed: u32,
    keys_path: &Path,
    env: &HashMap<String, Value>,
) -> io::Result<ScreenBuffer> {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};
    use std::io::{Read, Write};
    use std::process::Command;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    // Match tools/capture/play.sh pacing for deterministic prompt handling.
    const CHAR_DELAY: Duration = Duration::from_millis(150);
    const TIMEOUT: Duration = Duration::from_secs(30);

    let status = Command::new("cargo")
        .args(["build", "--bin", "umoria"])
        .current_dir(repo_root())
        .status()
        .map_err(|err| io::Error::other(err.to_string()))?;
    if !status.success() {
        return Err(io::Error::other("cargo build --bin umoria failed"));
    }

    let run_dir = repo_root();
    // Match tools/capture/play.sh: golden transcripts assume a fresh save slot.
    let _ = fs::remove_file(run_dir.join("game.sav"));

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: env_string(env, "LINES")
                .and_then(|v| v.parse().ok())
                .unwrap_or(24),
            cols: env_string(env, "COLS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(80),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| io::Error::other(e.to_string()))?;

    let mut cmd = CommandBuilder::new(repo_root().join("target/debug/umoria"));
    cmd.cwd(run_dir);
    cmd.arg("-s");
    cmd.arg(seed.to_string());
    if let Some(term) = env_string(env, "TERM") {
        cmd.env("TERM", term);
    }
    if let Some(lines) = env_string(env, "LINES") {
        cmd.env("LINES", lines);
    }
    if let Some(cols) = env_string(env, "COLS") {
        cmd.env("COLS", cols);
    }

    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| io::Error::other(e.to_string()))?;
    drop(pair.slave);

    let raw = Arc::new(Mutex::new(Vec::<u8>::new()));
    let raw_reader = Arc::clone(&raw);
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| io::Error::other(e.to_string()))?;
    let reader_thread = thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if let Ok(mut guard) = raw_reader.lock() {
                        guard.extend_from_slice(&buf[..n]);
                    }
                }
                Err(_) => break,
            }
        }
    });

    let keys = fs::read(keys_path)?;
    let mut writer = pair
        .master
        .take_writer()
        .map_err(|e| io::Error::other(e.to_string()))?;

    let deadline = Instant::now() + TIMEOUT;
    let mut key_index = 0usize;
    let mut last_write = Instant::now()
        .checked_sub(CHAR_DELAY)
        .unwrap_or_else(Instant::now);
    let mut pty_closed = false;

    while Instant::now() < deadline {
        if key_index < keys.len() && !pty_closed && last_write.elapsed() >= CHAR_DELAY {
            match writer.write_all(std::slice::from_ref(&keys[key_index])) {
                Ok(()) => {
                    let _ = writer.flush();
                    key_index += 1;
                    last_write = Instant::now();
                }
                Err(_) => pty_closed = true,
            }
        }

        if key_index >= keys.len() {
            if let Ok(Some(_)) = child.try_wait() {
                break;
            }
        }

        thread::sleep(Duration::from_millis(20));
    }

    thread::sleep(Duration::from_millis(200));

    if child.try_wait()?.is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
    drop(writer);
    let _ = reader_thread.join();

    let raw_bytes = raw.lock().map(|guard| guard.clone()).unwrap_or_default();
    Ok(ScreenBuffer::from_bytes(
        render_pty_screen(&raw_bytes).as_bytes(),
    ))
}

fn hash_golden(entry: &GoldenEntry, data: &[u8]) -> String {
    match entry.hash_method.as_str() {
        "sha256" => sha256_hex(data),
        "sha256-masked-save" => {
            let decoded = decode_xor_chain(data, &[0, 1, 2, 3]);
            let masked = apply_mask(&decoded, &entry.volatile_byte_ranges);
            sha256_hex(&masked)
        }
        "sha256-masked-score" => {
            let decoded = decode_xor_chain(data, &[0, 1, 2]);
            let masked = apply_mask(&decoded, &entry.volatile_byte_ranges);
            sha256_hex(&masked)
        }
        other => {
            assert!(
                ["sha256", "sha256-masked-save", "sha256-masked-score"].contains(&other),
                "unsupported hash_method {other:?} for golden {}",
                entry.id
            );
            String::new()
        }
    }
}

fn decode_xor_chain(data: &[u8], resets: &[usize]) -> Vec<u8> {
    let reset_set: HashSet<usize> = resets.iter().copied().collect();
    let mut out = Vec::with_capacity(data.len());
    let mut prev = 0u8;
    for (index, &byte) in data.iter().enumerate() {
        if reset_set.contains(&index) {
            prev = 0;
        }
        out.push(byte ^ prev);
        prev = byte;
    }
    out
}

fn apply_mask(data: &[u8], ranges: &[VolatileByteRange]) -> Vec<u8> {
    let mut masked = data.to_vec();
    for range in ranges {
        for index in range.offset..range.offset.saturating_add(range.length).min(masked.len()) {
            masked[index] = 0;
        }
    }
    masked
}

fn sha256_hex(data: &[u8]) -> String {
    use std::fmt::Write as _;

    let digest = Sha256::digest(data);
    digest
        .iter()
        .fold(String::with_capacity(digest.len() * 2), |mut acc, byte| {
            let _ = write!(acc, "{byte:02x}");
            acc
        })
}

fn env_string(env: &HashMap<String, Value>, key: &str) -> Option<String> {
    env.get(key).and_then(|value| match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => number.as_i64().map(|n| n.to_string()),
        _ => None,
    })
}

fn hex_window(offset: usize, expected: u8, actual: u8, len: usize) -> String {
    let start = offset.saturating_sub(HEX_WINDOW);
    let end = (offset + HEX_WINDOW + 1).min(len);
    let mut lines = Vec::new();
    lines.push(format!("hex window [{start:#06x}..{end:#06x}):"));
    for index in start..end {
        let marker = if index == offset { "<<" } else { "  " };
        lines.push(format!(
            "  {index:#06x}: mismatch expected={expected:#04x} actual={actual:#04x} {marker}"
        ));
    }
    lines.join("\n")
}
