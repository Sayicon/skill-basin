fn main() {
    // 确保替换图标后，`tauri dev` 的构建会重新触发（否则 Cargo 可能不重跑 build.rs，Dock 仍显示旧图标）。
    println!("cargo:rerun-if-changed=icons/icon.png");
    println!("cargo:rerun-if-changed=icons/icon.icns");
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    // Headless builds (hub-agent linking with default-features=false) skip the
    // Tauri build step entirely; there is no window, icon, or conf to wire.
    if std::env::var("CARGO_FEATURE_DESKTOP").is_ok() {
        embed_manifest_in_test_binaries();
        tauri_build::build()
    }
}

/// Give `cargo test` binaries the same Common-Controls v6 manifest the real
/// app gets.
///
/// tauri_build::build() embeds its manifest with `cargo:rustc-link-arg-bins`,
/// which covers app.exe but never the test harness. Unmanifested, the loader
/// binds comctl32.dll to system32's v5.82, which lacks TaskDialogIndirect and
/// the window-subclassing exports the desktop code imports — so the test
/// binary died with STATUS_ENTRYPOINT_NOT_FOUND before running any test, and
/// every desktop-feature test was silently unrunnable.
fn embed_manifest_in_test_binaries() {
    println!("cargo:rerun-if-env-changed=SKILLBASIN_TEST_MANIFEST");
    // Opt-in, because Cargo gives build scripts no way to say "tests only".
    // `rustc-link-arg-tests` covers [[test]] targets but NOT the lib's own
    // unit-test harness, and plain `rustc-link-arg` would also hit app.exe —
    // where tauri_build already embeds a manifest as a resource, so a second
    // one fails the link with CVT1100 "duplicate resource". `npm run rust:test`
    // sets this variable; run bare `cargo test` with it set by hand.
    if std::env::var("SKILLBASIN_TEST_MANIFEST").is_err() {
        return;
    }
    // MSVC-linker specific; other targets neither need nor understand these.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
    {
        return;
    }
    let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") else {
        return;
    };
    let manifest = std::path::Path::new(&dir).join("windows-tests.manifest");
    println!("cargo:rerun-if-changed=windows-tests.manifest");
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
}
