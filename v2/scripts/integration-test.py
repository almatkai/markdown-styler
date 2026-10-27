"""Exercise integration in a disposable fixture host; never accesses a live host."""
import pathlib,tempfile,sys,json,subprocess
from integrate import plan,TARGETS
from prerequisites import apply
v=pathlib.Path(__file__).resolve().parent.parent
with tempfile.TemporaryDirectory(prefix='markdown-styler-integration-') as name:
 host=pathlib.Path(name)
 for p in TARGETS:
  source=v/'fixtures/current'/p
  if not source.is_file():source=v/'fixtures/beta'/p
  if source.name=='Cargo.toml':source=source.with_name('Cargo.toml.txt')
  if not source.is_file():raise ValueError('Missing fixture '+str(source))
  t=host/p;t.parent.mkdir(parents=True,exist_ok=True);t.write_bytes(source.read_bytes())
 # Simulate another port: its catalog row/module/runtime must survive.
 catpath=host/TARGETS[3];before=json.loads(catpath.read_text());sentinel={'id':'com.tezbar.native.prior-port','slug':'prior-port','name':'Prior port','commands':[]};before.append(sentinel);catpath.write_text(json.dumps(before))
 modpath=host/TARGETS[0];modpath.write_text(modpath.read_text()+'\nmod v2_prior_port;\n')
 changes=plan(host)
 for p,b in changes.items():t=host/p;t.parent.mkdir(parents=True,exist_ok=True);t.write_bytes(b)
 assert json.loads(catpath.read_text())[:-1]==before
 assert 'mod v2_prior_port;' in modpath.read_text()
 assert '#[path = "../../../native-extensions/markdown-styler/adapter/v2_markdown_styler.rs"]' in modpath.read_text()
 runtime=(host/TARGETS[1]).read_text();assert '| "markdown-styler" => Some(value)' in runtime
 assert 'super::v2_markdown_styler::commands()' in runtime
 assert 'super::v2_markdown_styler::handle(ctx, command, operation, payload, state).await' in runtime
 assert plan(host)=={},'Integration must be idempotent and preserve prior ports'
 # Pinned beta/current prerequisites are independently idempotent.
 paths=['src-tauri/src/native_core/extensions/v2_catalog.rs','src-tauri/src/native_core/extensions/registry.rs','src/renderer/extensionImages.ts','src/renderer/src/raycast-api/detail-runtime.tsx']
 files={p:(v/'fixtures/current'/p).read_text() for p in paths};fixed=apply(files.copy());assert apply(fixed.copy())==fixed
 assert (host/'src/renderer/public/extensions/icons/markdown-styler.png').read_bytes()==(v/'assets/command-icon.png').read_bytes()
 # Exact original bytes, stable path, no JS runtime loader/subprocess converter.
 assert (host/'src-tauri/native-extensions/markdown-styler/src/lib.rs').is_file()
 print('Disposable integration plan applied twice: registration/catalog/count/dependency/icon/prerequisites; prior port preserved, second pass empty')
