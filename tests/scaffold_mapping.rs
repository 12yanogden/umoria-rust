//! Meta-tests verifying the Rust crate module layout stays coherent.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    reason = "integration-test helpers sit outside #[test]; clippy.toml allow-*-in-tests only covers test fn bodies"
)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn src_dir() -> PathBuf {
    repo_root().join("src")
}

fn declared_modules(lib_rs: &str) -> BTreeSet<String> {
    lib_rs
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
        .chain(lib_rs.lines().filter_map(|line| {
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
fn test_no_c_or_cxx_sources_in_src() {
    let entries = fs::read_dir(src_dir()).expect("src/ directory must exist");
    for entry in entries {
        let path = entry.expect("src entry").path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        assert!(
            !matches!(ext, "cpp" | "cc" | "cxx" | "c" | "h" | "hpp" | "hxx"),
            "C/C++ source must not remain in src/: {}",
            path.display()
        );
    }
    assert!(
        !repo_root().join("CMakeLists.txt").exists(),
        "CMakeLists.txt must not be present (Rust-only repo)"
    );
}

#[test]
fn test_main_declares_every_module() {
    let lib_rs = fs::read_to_string(src_dir().join("lib.rs")).expect("src/lib.rs must exist");
    let declared = declared_modules(&lib_rs);

    for name in &declared {
        let path = src_dir().join(format!("{name}.rs"));
        assert!(
            path.is_file(),
            "declared module `{name}` missing file {}",
            path.display()
        );
    }

    assert!(
        !declared.contains("main"),
        "binary entry is src/main.rs; there must be no `mod main;`"
    );
    assert!(
        declared.contains("entry"),
        "src/lib.rs missing `mod entry;`"
    );
    assert!(src_dir().join("entry.rs").is_file());
    assert!(src_dir().join("main.rs").is_file());
    // Nested path module owned by identification.rs (not declared from lib.rs).
    assert!(src_dir().join("identification_desc.rs").is_file());
    assert_eq!(
        declared.len(),
        54,
        "expected exactly 54 library modules, found {}",
        declared.len()
    );
}

#[test]
fn test_curses_and_headers_have_no_module() {
    assert!(
        !src_dir().join("curses.rs").exists(),
        "curses.rs must not exist"
    );
    assert!(
        !src_dir().join("headers.rs").exists(),
        "headers.rs must not exist"
    );
}

#[test]
fn test_data_files_present() {
    let data_dir = repo_root().join("data");
    let required = [
        "help.txt",
        "welcome.txt",
        "splash.txt",
        "versions.txt",
        "scores.dat",
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
