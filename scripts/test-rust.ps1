# Rust test runner (Windows).
# Windows needs a Common-Controls v6 manifest dependency linked into TEST binaries:
# tauri-build embeds a manifest into the app binary only, so bare `cargo test`
# dies at load time with STATUS_ENTRYPOINT_NOT_FOUND (comctl32 5.82 gets bound,
# which lacks TaskDialogIndirect).
$env:CARGO_ENCODED_RUSTFLAGS = "-Clink-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' publicKeyToken='6595b64144ccf1df' language='*' processorArchitecture='*'"
Set-Location "$PSScriptRoot\..\src-tauri"
cargo test --lib @args
exit $LASTEXITCODE
