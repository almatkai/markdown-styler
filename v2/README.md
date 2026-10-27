# Markdown Styler — Tezbar Native V2

Compiled-in Rust port of **Style Markdown** (`mp-style-markdown`), replacing
`raycast.markdown-styler` with `com.tezbar.native.markdown-styler`. Original author:
**qcrao**. All selected text conversion and session logic is Rust. No dynamic Rust
loader, JavaScript extension runtime, interpreter, subprocess converter, network
conversion service, source history, or data migration is used.

This directory is a reviewable port, **not an installed host extension**. Host
registration, application rebuild/install, and real desktop QA are NOT-RUN on
both macOS and Windows. Both manifest verification entries remain `not-run`.

## Behavior

Launch reads **selected text only**, then waits 50 ms as in V1. It applies JS trim,
UTF-16 length thresholds and the original indicator expression. Empty and short
inputs show the original English statuses. Successful conversion creates a full
standalone temporary HTML document and displays the original status, processing
time, execution ID and first 300 UTF-16 code units of the source.

Normal conversion does not open or copy anything. Actions, in source order:

1. **Open in Browser and Copy** — opens the current owned document, using a literal
   percent-encoded `file:` URL through the actual `shell:open` SDK route.
2. **Refresh Selection** — explicitly rereads selection and reconverts.
3. **Copy Original Markdown** — copies trimmed original Markdown as plain text,
   through `clipboard:write`, with strict acknowledgement.
4. **Try Sample Conversion** — uses the exact fixed sample and separate original
   sample template, then opens that sample. It never replaces the normal result.

Empty/short/read/conversion/write errors show **Try Sample Conversion**, then
**Refresh Selection**. Sample open retries reuse its already committed file.
Heartbeat `refresh-session`, search and load-more only return current state;
they never reread selection, mint IDs, write files, open or copy.

The external document retains the original styling, instructions, copy button,
modern `ClipboardItem(text/html)` helper, selection/`execCommand` fallback and
content-click selection. There is no invented native rich HTML clipboard action.
Browser success/failure helper tests use JSDOM with network disabled and recorder
clipboard mocks; actual browser and OS clipboard behavior remains NOT-RUN.

## Native adaptations and limits

- `pulldown-cmark=0.13.0` supplies native events. Compatibility overlays retain
  marked text/entity/URL handling, raw inline blocks, list continuations/tasks,
  HTML-block whitespace, original renderer callback bytes, and exact CSS. The
  executable **own TSX + actual locked marked 9.1.6** oracle covers 36 varied
  inputs plus the sample. This is finite evidence, not universal marked parser
  equivalence. Inputs outside this corpus may expose dialect differences.
- Core `styled_html` is the original unsanitized export representation: V1's code
  renderer inserts unescaped markup. It is **not** advertised as sanitized.
  The *browser document* is separately parsed/cleaned with pinned ammonia,
  dropping active elements, events, source IDs/classes, unsafe/relative URLs;
  remote images are blocked by CSP. Inline CSS is preserved. Browser copying
  copies this cleaned section, so dangerous markup and relative-image behavior
  deliberately differ from raw exported `styled_html`.
- The browser shell adds CSP (`default-src 'none'`, exact fixed-helper SHA-256,
  no connections/frames/forms/base) and removes the sample inline event handler
  in favor of an equivalent fixed listener. The sample helper reports failure
  when `execCommand` fails or throws; V1 unconditionally claimed success.
  JSDOM verifies removal/hash/helper behavior; it does not enforce CSP.
- Detail text uses numeric entities, retaining literal preview text without
  turning selected Markdown/HTML into host links, images, code-copy controls or
  privileged effects. Preview UTF-16 units remain exact in the core. If the 300th
  unit splits a surrogate pair, the UTF-8 UI displays a visible `\uD83D`-style
  escape; no lossy U+FFFD substitution is introduced. Invalid UTF-16 cannot be
  represented by the host's Rust string IPC and is rejected upstream.
- Execution IDs use a process-random nonce + checked monotonic counter. Time is
  local `%X` rather than JavaScript Intl formatting. The SDK has no HUD/toast/
  popToRoot API; supported Detail `message` feedback reports confirmed outcomes.
  There is no extra confirmation step, hide-window or claimed focus/dismissal.
- Open/copy success requires an object with `ok:true`, `error` absent/null and
  `success` absent/true. Null, booleans, malformed, negative, contradictory and
  transport failures remain unconfirmed. Capability rotation and effect intent
  are checkpointed **before await**. Old, forged and cross-session actions fail;
  cancellation cannot replay them. No uncertain effect is automatically retried.
- Input: 256 KiB UTF-8. Styled output/browser file: 8 MiB each. Parse: 100,000
  events, 64 nested containers. Session: 16 files / 32 MiB. No truncation beyond
  the source's preview rule. Resource exhaustion is an explicit error.
- File transactions and bounded conversion are synchronous between await points.
  There is no detached filesystem worker. This makes cancellation ownership
  explicit but can briefly occupy a runtime worker at the resource limit.
  Launch/refresh cancellation during selection/delay and open cancellation are
  exercised through real beta NativeSessions.
- Files live in unique private temporary namespaces (0700 directories and 0600
  files on Unix; Windows inherits the user's temporary-directory ACL). Exclusive
  staging + `persist_noclobber`, flush/sync and canonical path/content/OS file-ID
  checks precede effects. After publish, the staging guard is disarmed; recreating
  the retired stage name cannot cause its deletion. Pending/committed cleanup
  removes only the matching document then an empty directory, never recursively
  deleting foreign entries. Modified/replaced/symlink files are preserved with an
  error. This is not a security boundary against hostile same-user processes
  racing filesystem APIs or browser path resolution; no such guarantee is made.
  Process termination cannot run cleanup, as with V1.
- No private text or raw OS error is logged. Error text is fixed/redacted.

## Reproduce

From repository root (Node 22+, Python 3.9+, installed Rust toolchains/target):

```sh
mkdir -p v2/.v1
cp -R v2/fixtures/own/. v2/.v1/
npm ci --prefix v2/.v1 --ignore-scripts --no-audit --no-fund
npm test --prefix v2/.v1
npm run typecheck --prefix v2/.v1
npm ci --prefix v2/scripts --ignore-scripts --no-audit --no-fund
node v2/scripts/oracle.cjs
node v2/scripts/schema.cjs
node v2/scripts/browser.cjs
node v2/scripts/contracts.cjs
python3 v2/scripts/integration-test.py
python3 v2/scripts/provenance.py
cargo +stable test --offline --locked --manifest-path v2/Cargo.toml
cargo +1.85.0 test --offline --locked --manifest-path v2/Cargo.toml
cargo +stable fmt --manifest-path v2/Cargo.toml --check
cargo +1.85.0 fmt --manifest-path v2/Cargo.toml --check
cargo +stable clippy --offline --locked --manifest-path v2/Cargo.toml --all-targets -- -D warnings
cargo +1.85.0 clippy --offline --locked --manifest-path v2/Cargo.toml --all-targets -- -D warnings
cargo +stable check --offline --locked --manifest-path v2/Cargo.toml --all-targets --target x86_64-pc-windows-gnu
cargo +stable clippy --offline --locked --manifest-path v2/Cargo.toml --all-targets --target x86_64-pc-windows-gnu -- -D warnings
cargo +1.85.0 check --offline --locked --manifest-path v2/Cargo.toml --all-targets --target x86_64-pc-windows-gnu
cargo +1.85.0 clippy --offline --locked --manifest-path v2/Cargo.toml --all-targets --target x86_64-pc-windows-gnu -- -D warnings
python3 v2/scripts/archive-test.py
```

Oracle regeneration is explicit: `node v2/scripts/oracle.cjs --write`; it derives
expectations from actual TSX, never Rust. `cargo run --example preview` derives the
native browser fixture; the helper tests are separate from conversion parity.
`prepare-harness.py` derives a reviewable pinned host harness, then run cargo fmt.
No full registry-source archive is bundled: offline builds require cached crates.

See [INTEGRATION.md](INTEGRATION.md), [PROVENANCE.md](PROVENANCE.md) and
[VALIDATION.md](VALIDATION.md). V1 files and lock remain intact except the short
root README discovery link. Changes remain unstaged and uncommitted for parent
review; parent owns commits, pushes, remote verification and eventual clone removal.
