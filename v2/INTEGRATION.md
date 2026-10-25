# Compiled-in host integration

Infrastructure reference: **fd300e276356587726566026f072563eb9c34cb1**, not host
origin/main. Regression reference: **fe71ca87f21ac4ff31505010c228f8abda0f2a89**.
All inspected host content was read from committed Git objects; the dirty live
host was never edited or used as a server/test target.

Stable vendoring destination is `src-tauri/native-extensions/markdown-styler`.
Vendor this complete V2 package without `target`, `node_modules`, `.v1`, `.scratch`
or Python caches. The exact original 44,175-byte icon goes from
`assets/command-icon.png` to
`src/renderer/public/extensions/icons/markdown-styler.png` and is referenced as
`/extensions/icons/markdown-styler.png`.

`src-tauri/Cargo.toml` needs the path dependency:

```toml
tezbar-markdown-styler = { path = "native-extensions/markdown-styler" }
```

The adapter is a **sibling** of `v2_sdk`, registered in extension `mod.rs`:

```rust
#[path = "../../../native-extensions/markdown-styler/adapter/v2_markdown_styler.rs"]
mod v2_markdown_styler;
```

The host already has serde/serde_json/tokio/chrono as used by its beta
SDK; retain those dependencies and merge this package's locked dependency needs.
The dependency crate compiles the offline renderer, document isolation and temp
ownership into the application. The adapter is also compiled into the application,
not loaded from its path at runtime.

Add only `markdown-styler` to runtime slug matching, command dispatch and handle
dispatch, preserving every existing adapter. Append `manifest.json` to
`v2_catalog.json`; keep every existing row. Replace a hardcoded catalog test count
with `source().len()` so prior ports survive. Test compiled command coverage,
canonical ID, Raycast alias, platform status, install markers and V1 coexistence.
The real beta registry already prefers an installed native alias and falls back
to the V1 worker only when that native command is absent.

## Required shared prerequisites

[prerequisites-fe71ca8.patch](integration/prerequisites-fe71ca8.patch) is scoped to:

- `v2_catalog` store iconUrl, `registry` icon path/data and renderer
  `extensionImageSource`: preserve narrowly validated bundled icon/artwork paths.
  The current committed host clears these or treats them as filesystem paths.
  Arbitrary filesystem paths and traversal do not gain bundled-asset privileges.
- `DetailRuntime` Enter button: `onClick={() => onRunPrimaryAction()}`. The old
  direct callback passes its MouseEvent as `actionId` to Surface. Keyboard Enter
  already supplies the canonical first action ID. This is a required functional
  prerequisite, shared with earlier Detail ports.

Merge these with equivalent earlier-port prerequisites; do not overwrite prior
work. The script recognizes exact already-applied forms. Other forms intentionally
fail for review. No Markdown code-copy prerequisite is required here: the port
entity-encodes all selected text, and executable actual host Markdown tests prove
it cannot produce a code-copy control, link or image. Pinned host Markdown's null
clipboard result bug remains outside this port's rendered surface.

The reviewable scoped script defaults to printing its plan:

```sh
python3 v2/scripts/integrate.py --host /path/to/selected-host-checkout
# Parent's separately authorized host integration step only:
python3 v2/scripts/integrate.py --host /path/to/selected-host-checkout --apply
```

It precomputes changes, fails on conflicting vendored files/icon/module/dependency,
keeps existing catalog entries and modules, and has a disposable fixture-host
idempotence test. It is not executed against the actual host in this task.
A package manifest is not a loader or install operation.

After parent integration: refresh the **host** Cargo.lock without changing V1,
run actual host Rust/frontend checks, build the native prerequisites with the
host's documented `pnpm build` / Tauri build workflow, install the rebuilt app,
and then install the catalog Native V2 command. This has not been performed.
Real macOS/Windows selected-text permissions, browser file URLs/CSP, rich HTML
copy, fallback copy, focus, fonts, keyboard and desktop filesystem QA remain
NOT-RUN. GNU cross-check/clippy do not constitute Windows execution.
