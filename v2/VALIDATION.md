# Validation record

Scope: only the Markdown Styler clone, branch `feat/native-v2-rust`, base own SHA
`7bd8f532fae79ce15a2f96d62fc2538fb4b24cfd`. No staging, commit, push, PR, ledger,
host/profile edits, clone deletion, subdelegation or real selected-text/browser/
clipboard/focus/reveal/paste effects were performed.

Passed checks:

- Disposable V1 install using **unchanged original package-lock.json**: `npm ci
  --ignore-scripts`; original test (1) and original TypeScript typecheck pass.
  No `--legacy-peer-deps`, dependency change or lock repair was necessary.
- Executable complete own TSX/component/custom renderer with actual marked
  **9.1.6**: **36** conversion/classification/status/preview/style/wrapper/full
  template/action fixtures plus exact fixed sample; selection-only mount,
  StrictMode setup/cleanup/setup guard, explicit refresh, normal no-auto-open,
  plain original copy, separate sample, read/write/open/copy errors and cleanup.
  Recorded source traces include the V1 sample's pre-open success toast; native
  intentionally waits for a positive open acknowledgement before success.
- **15 Rust tests** on stable and **1.85.0**, offline and locked: 9 real beta
  adapter/lifecycle/registry/catalog tests, 5 filesystem/resource/UTF-16 tests,
  1 parameterized exact TSX oracle test. All filesystem effects use disposable
  test directories and no real platform/clipboard/browser service.
- Real pinned beta common/SDK/NativeSessions/runtime/registry/catalog bodies are
  compiled in the harness. Original unrelated unit tests are excluded, visibility
  is opened for inspection, a test-only unused import is removed, and formatting
  is normalized. Tauri event emitter and unrelated adapters are stubs; routers
  record exact supported channel arguments. No production session behavior is
  replaced by a mock session manager.
- Cancellation through real NativeSessions during launch selection, explicit
  refresh selection and browser-open; dropped sample-open future; canonical file
  checkpoint; no heartbeat effects/ID minting; stale/forged/cross-session IDs;
  alias/V1 coexistence, resource limit, disposal, positive/malformed/null/
  contradictory acknowledgements, fixed/redacted transport errors.
- Real filesystem tests: Unix 0700/0600, unique exclusive files, content/file-ID/
  canonical-path verification, missing/altered/replaced/symlink rejection, RAII
  pending cleanup, foreign-entry preservation, retired staging guard disarm,
  percent-encoded literal file URL round-trip and resource/depth rejection.
- Stable and 1.85 formatting checks and strict `--all-targets` Clippy with
  `-D warnings`. No new-toolchain-only lint allowances are used.
- Windows GNU `x86_64-pc-windows-gnu` all-target check and strict Clippy pass
  on stable and 1.85.0.
  This is cross-compilation/checking, **not Windows execution**.
- Draft **2020-12** full schema validation using Ajv2020: manifest plus 12 invalid
  variants; both macOS and Windows verification entries stay `not-run`.
- JSDOM fixed browser helper with network disabled and recorder clipboard:
  modern HTML success, modern rejection then fallback, fallback false/throw,
  truthful sample failure, selection backup, no source script execution and exact
  CSP helper hash. JSDOM does not enforce CSP or emulate an OS clipboard.
- Actual pinned Detail/Surface code reproduces the MouseEvent-as-actionId bug;
  the scoped wrapper and keyboard Enter route the canonical first action.
  Actual host Markdown code with marked **18.0.2** and DOMPurify **3.4.0** proves
  entity-encoded selected text creates no host code-copy controls, links/images
  or script nodes. Highlighting is irrelevant for this literal-text surface.
- Current committed bundled-icon path regression reproduced; scoped prerequisite
  preserves only validated app asset paths. Disposable integration applies
  registration/catalog/count/dependency/icon/prerequisites, preserves a prior
  port, and is idempotent.
- **350** fixture inventory records: byte length/SHA-256/SRI/source, independent
  original 44,175-byte icon and actual locked marked npm tarball SHA-512.
- Actual Cargo archive is inspected for every indexed fixture, manifest, adapter,
  templates, icon and prerequisites. Nested Cargo fixture manifests use `.txt`
  names. No `node_modules`, `.v1`, `.scratch` or build cache enters the archive.
  Extracted archive passes offline locked tests on stable and 1.85 using cached
  crates, without absolute host/source build paths. Git visibility is checked
  without staging. An initial packaging check caught overly broad root globs
  and a missing package manifest; both were corrected before final verification.

Remaining integration/QA gaps:

- No real host registration, lock resolution, application rebuild or Native V2
  catalog install. The host's unrelated dirty work remains untouched. Shared
  dependency ranges allow host lock resolution; this package's Cargo.lock fixes
  the versions verified here. Parent must review the resulting host lock diff.
- No macOS/Windows real selection permissions, browser file opening/CSP, rich
  clipboard, fallback clipboard, desktop filesystem, focus, fonts or keyboard QA.
  Both platform verification states remain honestly `not-run`.
- Parser compatibility is finite corpus evidence, not universal marked parity.
  Browser markup cleaning/CSP, safe literal Detail preview, metadata formatting,
  fixed sample failure feedback, resource budgets and private temp namespace are
  explicit bounded adaptations documented in README.
- XERJ coverage is unavailable after own HTTP429/RSS circuit-breaker failure;
  complete files and exhaustive `rg` checks were used. No global reset or bypass.

Parent owns review, staging and rehashing every INDEX record, commits/pushes,
remote verification and later removal of this temporary clone. Clone is intact.

Parent review caught a real channel mismatch: the first adapter and test routers used `platform:selected-text`, while unchanged beta PlatformCore accepts only `core:platform:selected-text` and CoreContext forwards names verbatim. The adapter and all recording routers now use the actual channel. A regression binds the test-router constant to the exact immutable PlatformCore dispatch arm, preventing a self-consistent mock from blessing an invented channel. Core/adapter tests, strict Clippy, Windows checks and extracted-package tests were repeated for the final change.
