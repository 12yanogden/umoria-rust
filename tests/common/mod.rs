//! Shared helpers for phase 1.5 differential fidelity tests.

#![allow(dead_code)]

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
    pub cmake: Option<String>,
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
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        let text = String::from_utf8_lossy(data);
        let rows: Vec<String> = text
            .lines()
            .map(str::trim_end)
            .map(str::to_string)
            .collect();
        Ok(Self { rows })
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
    fs::read(entry.path()).unwrap_or_else(|err| {
        panic!(
            "golden file missing for {}: {} ({err})",
            entry.id,
            entry.path().display()
        )
    })
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
        let expected_row = expected.rows.get(row).map(String::as_str).unwrap_or("");
        let actual_row = actual.rows.get(row).map(String::as_str).unwrap_or("");
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
pub fn replay_transcript(
    seed: u32,
    keys_path: &Path,
    env: &HashMap<String, Value>,
) -> io::Result<ScreenBuffer> {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};
    use std::io::{Read, Write};

    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: env_string(env, "LINES")
            .and_then(|v| v.parse().ok())
            .unwrap_or(24),
        cols: env_string(env, "COLS")
            .and_then(|v| v.parse().ok())
            .unwrap_or(80),
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = CommandBuilder::new(repo_root().join("target/debug/umoria"));
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

    let mut child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);

    let keys = fs::read(keys_path)?;
    if let Some(mut writer) = pair.master.take_writer() {
        writer.write_all(&keys)?;
    }

    let mut output = String::new();
    if let Some(mut reader) = pair.master.try_clone_reader()? {
        reader.read_to_string(&mut output)?;
    }

    let _ = child.wait();
    ScreenBuffer::from_bytes(output.as_bytes())
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
        other => panic!("unsupported hash_method {other:?} for golden {}", entry.id),
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
    let digest = Sha256::digest(data);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
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
            "  {index:#06x}: mismatch expected={expected:#04x} actual={actual:#04x} {marker}",
            index = index,
            expected = expected,
            actual = actual,
            marker = marker
        ));
    }
    lines.join("\n")
}
