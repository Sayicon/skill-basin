fn main() {
    // 确保替换图标后，`tauri dev` 的构建会重新触发（否则 Cargo 可能不重跑 build.rs，Dock 仍显示旧图标）。
    println!("cargo:rerun-if-changed=icons/icon.png");
    println!("cargo:rerun-if-changed=icons/icon.icns");
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    // Headless builds (hub-agent linking with default-features=false) skip the
    // Tauri build step entirely; there is no window, icon, or conf to wire.
    if std::env::var("CARGO_FEATURE_DESKTOP").is_ok() {
        tauri_build::build()
    }
}
