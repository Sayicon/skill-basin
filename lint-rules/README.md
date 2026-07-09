# Structural lint rules

Each rule here encodes a bug class this codebase has actually shipped, so it
cannot silently return. Run them with `npm run lint:rules` (CI runs the same);
they are [ast-grep](https://ast-grep.github.io/) rules — Semgrep would fit
equally well but has no first-class Windows support, and Windows is a
first-class platform here.

| Rule | Guards against |
|---|---|
| `no-raw-remove-file` | `remove_file` on a directory junction fails with os error 5 on Windows; skill deletion, unsync, and disable all shipped this bug, leaving orphaned links after the DB row was gone. |
| `no-discarded-apply-results` | A command dropping a core function's partial-failure results, so the UI reports success over a sync the engine refused. |
| `no-conflated-installed-flag` | Folding "present on disk" and "user enabled it" into one boolean, which made disabled tools read as undetected and vanish from the UI. |
| `no-native-js-dialogs` | `window.alert/confirm/prompt` do nothing in WebView2, so dialog-gated flows are dead buttons in the packaged app. |

## Suppressing a finding

Put `// ast-grep-ignore: <rule-id>` on the line directly above, with a separate
comment line explaining why the case is genuinely safe. A suppression without a
reason is a smell — the rules exist because these mistakes are easy to repeat.

## Adding a rule

When a bug turns out to be an instance of a class rather than a one-off, write
the rule before writing the fix. Verify it fires: drop a canary file containing
the bad pattern under a scanned directory, run `npx ast-grep scan <file>`,
confirm the finding, then delete the canary.
