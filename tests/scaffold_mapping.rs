//! Meta-tests verifying the Rust crate scaffold mirrors the C++ source layout.
//! See `.cursor/plans/rust-translation/phase_1.1.md`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn src_dir() -> PathBuf {
    repo_root().join("src")
}

fn cpp_stems() -> Vec<String> {
    let mut stems: Vec<String> = fs::read_dir(src_dir())
        .expect("src/ directory must exist")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()?.to_str()? == "cpp" {
                path.file_stem()?.to_str().map(String::from)
            } else {
                None
            }
        })
        .collect();
    stems.sort();
    stems
}

fn non_main_cpp_stems() -> Vec<String> {
    cpp_stems().into_iter().filter(|s| s != "main").collect()
}

fn declared_modules(main_rs: &str) -> BTreeSet<String> {
    main_rs
        .lines()
        .filter_map(|line| {
            let line = line.split("//").next()?.trim();
            let rest = line.strip_prefix("pub mod ")?;
            let name = rest.strip_suffix(';')?.trim();
            if name.contains(' ') {
                return None;
            }
            Some(name.to_string())
        })
        .chain(main_rs.lines().filter_map(|line| {
            let line = line.split("//").next()?.trim();
            if line.starts_with("pub mod ") {
                return None;
            }
            let rest = line.strip_prefix("mod ")?;
            let name = rest.strip_suffix(';')?.trim();
            if name.contains(' ') {
                return None;
            }
            Some(name.to_string())
        }))
        .collect()
}

#[test]
fn test_every_cpp_has_sibling_rs_module() {
    for stem in cpp_stems() {
        let rs_path = src_dir().join(format!("{stem}.rs"));
        assert!(
            rs_path.is_file(),
            "missing sibling Rust module for src/{stem}.cpp: expected {}",
            rs_path.display()
        );
    }
}

#[test]
fn test_header_only_modules_exist() {
    for name in ["types", "version", "dungeon_tile"] {
        let path = src_dir().join(format!("{name}.rs"));
        assert!(
            path.is_file(),
            "missing header-only module: {}",
            path.display()
        );
    }
}

#[test]
fn test_main_declares_every_module() {
    let lib_rs = fs::read_to_string(src_dir().join("lib.rs")).expect("src/lib.rs must exist");

    let declared = declared_modules(&lib_rs);

    for stem in non_main_cpp_stems() {
        assert!(
            declared.contains(&stem),
            "src/lib.rs missing `mod {stem};` declaration"
        );
    }

    for name in ["types", "version", "dungeon_tile"] {
        assert!(
            declared.contains(name),
            "src/lib.rs missing `mod {name};` declaration"
        );
    }

    // 50 non-main cpp modules + entry (main.cpp logic) + 3 header-only = 54 declarations.
    // main.cpp maps to src/entry.rs; main.rs is a thin binary wrapper.
    assert!(
        declared.contains("entry"),
        "src/lib.rs missing `mod entry;` for main.cpp"
    );
    assert_eq!(
        declared.len(),
        54,
        "expected exactly 54 module declarations in lib.rs, found {}",
        declared.len()
    );
}

#[test]
fn test_no_orphan_cpp_or_unmapped() {
    let lib_rs = fs::read_to_string(src_dir().join("lib.rs")).expect("src/lib.rs must exist");
    let declared = declared_modules(&lib_rs);

    let expected: BTreeSet<String> = non_main_cpp_stems()
        .into_iter()
        .chain([
            "entry".into(),
            "types".into(),
            "version".into(),
            "dungeon_tile".into(),
        ])
        .collect();

    assert_eq!(
        declared, expected,
        "declared modules must exactly match non-main cpp stems plus header-only modules"
    );

    assert!(
        !declared.contains("main"),
        "main.cpp maps to src/entry.rs; there must be no `mod main;`"
    );

    assert!(
        src_dir().join("entry.rs").is_file(),
        "main.cpp counterpart must be src/entry.rs"
    );
    assert!(
        src_dir().join("main.rs").is_file(),
        "binary entry must be src/main.rs"
    );
}

#[test]
fn test_curses_and_headers_have_no_module() {
    assert!(
        !src_dir().join("curses.rs").exists(),
        "curses.h must not have a Rust module (phase_1.2 terminal binding)"
    );
    assert!(
        !src_dir().join("headers.rs").exists(),
        "headers.h must not have a Rust module (umbrella include)"
    );
}

#[test]
fn test_mapping_table_coverage() {
    let manifest_path = repo_root().join("MODULE_MAP");
    let manifest = fs::read_to_string(&manifest_path).unwrap_or_else(|_| {
        panic!(
            "MODULE_MAP manifest must exist at {}",
            manifest_path.display()
        )
    });

    let mut mapped_cpp: HashSet<String> = HashSet::new();
    let mut mapped_h: HashSet<String> = HashSet::new();

    for line in manifest.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        assert!(
            parts.len() >= 2,
            "invalid MODULE_MAP line (expected `<kind> <filename>`): {line}"
        );
        match parts[0] {
            "cpp" => {
                mapped_cpp.insert(parts[1].to_string());
            }
            "h" => {
                mapped_h.insert(parts[1].to_string());
            }
            other => panic!("unknown MODULE_MAP entry kind: {other}"),
        }
    }

    let actual_cpp: HashSet<String> = cpp_stems()
        .into_iter()
        .map(|s| format!("{s}.cpp"))
        .collect();
    let actual_h: HashSet<String> = fs::read_dir(src_dir())
        .expect("src/ directory must exist")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()?.to_str()? == "h" {
                path.file_name()?.to_str().map(String::from)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        mapped_cpp, actual_cpp,
        "MODULE_MAP cpp entries must match src/*.cpp"
    );
    assert_eq!(
        mapped_h, actual_h,
        "MODULE_MAP h entries must match src/*.h"
    );

    assert_eq!(mapped_cpp.len(), 51, "expected 51 cpp files in MODULE_MAP");
    assert_eq!(mapped_h.len(), 26, "expected 26 h files in MODULE_MAP");
    assert_eq!(mapped_cpp.len() + mapped_h.len(), 77);
}

#[test]
fn test_data_files_present() {
    let data_dir = repo_root().join("data");
    let required = [
        "help.txt",
        "welcome.txt",
        "splash.txt.in",
        "scores.dat",
        "versions.txt.in",
    ];
    for name in required {
        let path = data_dir.join(name);
        assert!(
            path.is_file(),
            "runtime data file must exist at {}",
            path.display()
        );
    }
}
