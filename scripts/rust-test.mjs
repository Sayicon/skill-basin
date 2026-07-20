#!/usr/bin/env node
// Runs `cargo test` with the Windows test manifest enabled.
//
// Desktop-feature test binaries import comctl32 v6 symbols (TaskDialogIndirect,
// SetWindowSubclass, ...). tauri_build only manifests app.exe, so an
// unmanifested test binary binds to system32's comctl32 v5.82 and dies at load
// with STATUS_ENTRYPOINT_NOT_FOUND before running a single test. See
// src-tauri/windows-tests.manifest and build.rs.
//
// The flag cannot simply be set inline in package.json: `VAR=1 cmd` is not
// valid on Windows cmd.exe, and this is exactly where it matters.
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

// `--lib` on purpose: every test in this crate lives in app_lib (core/ and
// commands/ pull their test modules in with #[path]), and main.rs is a thin
// entry point with none. Building the *bin's* test target would also link
// tauri_build's resource.lib, whose manifest collides with the one embedded
// here (CVT1100 duplicate resource).
const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const result = spawnSync('cargo', ['test', '--lib', ...process.argv.slice(2)], {
  cwd: join(root, 'src-tauri'),
  stdio: 'inherit',
  shell: true,
  env: { ...process.env, SKILLBASIN_TEST_MANIFEST: '1' },
})

process.exit(result.status ?? 1)
