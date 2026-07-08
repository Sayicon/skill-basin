#!/usr/bin/env bash
# Rust test runner (Linux/macOS). Windows: use scripts/test-rust.ps1 —
# it links a Common-Controls v6 manifest into test binaries, which bare
# `cargo test` lacks (STATUS_ENTRYPOINT_NOT_FOUND at load time).
set -euo pipefail
cd "$(dirname "$0")/../src-tauri"
cargo test --lib "$@"
