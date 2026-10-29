//! Link against libleptris.
//!
//! Resolution order:
//!   1. LEPTRIS_LIB_PATH — full path to the shared library file, or a
//!      directory containing it. CI sets this after building the C
//!      library from source.
//!   2. Fallback: system linker search (`-lleptris`).

use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    // The lib path changes the emitted link directives; without
    // this the build script output is cached across LEPTRIS_LIB_PATH
    // changes and tests link a stale library.
    println!("cargo:rerun-if-env-changed=LEPTRIS_LIB_PATH");
    // #1900: the wasm feature compiles the engine INTO the crate for
    // wasm32-unknown-unknown (freestanding; wasm/shim/ is the libc).
    // LEPTRIS_SRC_PATH points at an engine source tree (CI fetches
    // the pinned tarball); native targets keep the dynamic link.
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target_arch == "wasm32" && env::var("CARGO_FEATURE_WASM").is_ok() {
        build_wasm();
        return;
    }
    // Windows: the DLL is libleptris.dll, so the import library is
    // libleptris.lib — link.exe resolves `-l <name>` as <name>.lib.
    // (TODO.concurrency/05 renamed the DLL from leptris.dll.)
    let name = if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        "libleptris"
    } else {
        "leptris"
    };
    if let Ok(path) = env::var("LEPTRIS_LIB_PATH") {
        // Tolerate repo-relative paths: cargo runs build.rs from the
        // crate directory, so a path like `build-shared/src` (natural
        // when set from the repository root) would not resolve.
        let mut p = PathBuf::from(&path);
        if !p.exists() {
            if let Ok(manifest) = env::var("CARGO_MANIFEST_DIR") {
                let repo_relative = Path::new(&manifest).parent().map(|root| root.join(&p));
                if let Some(cand) = repo_relative {
                    if cand.exists() {
                        p = cand;
                    }
                }
            }
        }
        let dir: PathBuf = if p.is_file() {
            p.parent().unwrap().to_path_buf()
        } else {
            p.clone()
        };
        // Absolute for -rpath: a relative rpath resolves against the
        // test binary's directory, not the crate.
        let dir = dir.canonicalize().unwrap_or(dir);
        let found = if p.is_file() { Some(p) } else { find_lib(&p) };
        assert!(
            found.is_some(),
            "LEPTRIS_LIB_PATH set but no libleptris shared library found under {path}"
        );
        println!("cargo:rustc-link-search=native={}", dir.display());
        println!("cargo:rustc-link-lib=dylib={name}");
        // Let test binaries find the dylib at run time without
        // requiring DYLD_LIBRARY_PATH.
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", dir.display());
        println!("cargo:rustc-env=LEPTRIS_LIB_DIR={}", dir.display());
        return;
    }
    println!("cargo:rustc-link-lib=dylib={name}");
}

fn find_lib(dir: &Path) -> Option<PathBuf> {
    let prefixes = ["lib", ""];
    let exts = ["dylib", "so", "dll"];
    for ext in exts {
        for prefix in prefixes {
            let cand = dir.join(format!("{prefix}leptris.{ext}"));
            if cand.exists() {
                return Some(cand);
            }
        }
    }
    // Windows often drops the prefix.
    let _ = fs::metadata(dir);
    None
}

/// #1900 slice 2: compile the engine + the freestanding libc shim
/// into the crate for wasm32-unknown-unknown.
fn build_wasm() {
    let src_root = env::var("LEPTRIS_SRC_PATH").unwrap_or_else(|_| {
        panic!(
            "LEPTRIS_SRC_PATH must point at the libleptris source tree \
             (the pinned release tarball, unpacked) for the wasm build"
        )
    });
    let src_root = PathBuf::from(src_root);
    println!("cargo:rerun-if-env-changed=LEPTRIS_SRC_PATH");
    println!("cargo:rerun-if-changed={}", shim_dir().display());

    let mut build = cc::Build::new();
    build.target("wasm32-unknown-unknown");
    // Host gcc cannot emit wasm; rustc's sysroot ships a clang that can.
    if env::var_os("CC_wasm32_unknown_unknown").is_none() {
        build.compiler("clang");
    }
    build.opt_level(2);
    build.define("LEPTRIS_ENABLE_UTF8PROC", Some("0"));
    build.define("LEPTRIS_ENABLE_ICONV", Some("0"));
    build.define("LEPTRIS_VERSION", Some("\"wasm-1.4.x\""));
    build.include(shim_dir());
    build.include(src_root.join("src/include"));
    build.include(src_root.join("src"));
    for sub in [
        "leptris/dom",
        "leptris/parse",
        "leptris/serialize",
        "leptris/xpath",
        "leptris/encoding",
        "leptris/memory",
        "leptris/flat",
    ] {
        build.include(src_root.join("src").join(sub));
    }
    // The same exclusions the probe validated: encoding conversion,
    // utf8proc, and the arch SIMD TUs (scalar dispatch serves).
    let engine_src = src_root.join("src/leptris");
    let mut files: Vec<PathBuf> = Vec::new();
    collect_c(&engine_src, &mut files);
    files.retain(|f| {
        let p = f.to_string_lossy();
        !(p.contains("encoding/encoding.c")
            || p.contains("unicode/unicode.c")
            || p.contains("simd_text_avx2.c")
            || p.contains("simd_text_neon.c")
            || p.contains("/rng/"))
    });
    for f in &files {
        build.file(f);
        println!("cargo:rerun-if-changed={}", f.display());
    }
    // The freestanding libc shim.
    build.file(shim_dir().join("shim.c"));
    build.compile("leptris_wasm");
}

fn shim_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("wasm/shim")
}

fn collect_c(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect_c(&p, out);
            } else if p.extension().and_then(|s| s.to_str()) == Some("c") {
                out.push(p);
            }
        }
    }
}
