"""Derive reviewable host harness from immutable beta production bodies.
Only excludes original tests; stubs Tauri emitter and unrelated adapters.
No host filesystem read/write is performed.
"""
import pathlib,json,re
from prerequisites import apply as prerequisites
v=pathlib.Path(__file__).resolve().parent.parent
h=v/'integration/harness/native_core';e=h/'extensions';e.mkdir(parents=True,exist_ok=True)
b=v/'fixtures/beta/src-tauri/src/native_core'
for p in ['common.rs','extensions/v2_sdk.rs','extensions/native.rs','extensions/registry.rs','extensions/v2_catalog.rs','extensions/v2_runtime.rs']:
 s=(b/p).read_text()
 # Original unit tests target unrelated apps; production bodies remain verbatim.
 s=re.split(r'\n#\[cfg\(test\)\]\n(?:#\[path = "native_tests.rs"\]\n)?mod tests',s)[0]
 # Native test-only TestAdapter seam remains exactly as pinned.
 if p.endswith('v2_runtime.rs'):
  s=s.replace('"perplexity" => Some(value),','"perplexity" | "markdown-styler" => Some(value),')
  s=s.replace('        _ => &[],','        "markdown-styler" => super::v2_markdown_styler::commands(),\n        _ => &[],')
  s=s.replace('        _ => Err(format!', '        "markdown-styler" => super::v2_markdown_styler::handle(ctx, command, operation, payload, state).await,\n        _ => Err(format!')
 if p.endswith('native.rs'): s=s.replace('#[cfg(test)]\nuse super::perplexity;\n','')
 # Public visibility only exposes host-private entry points to harness checks.
 s=s.replace('pub(super)', 'pub')
 t=h/p;t.write_text(s)
manifest=json.loads((v/'manifest.json').read_text());catalog=json.loads((b/'extensions/v2_catalog.json').read_text());catalog.append(manifest)
(e/'v2_catalog.json').write_text(json.dumps(catalog,indent=2)+'\n')
mods='''pub mod v2_sdk;
pub mod native;
pub mod registry;
pub mod v2_catalog;
pub mod v2_runtime;
#[path="../../../../adapter/v2_markdown_styler.rs"]
pub mod v2_markdown_styler;
pub mod perplexity {}
'''
for m in catalog[:-1]:
 slug=m['slug'].replace('-','_');commands=[c['name'] for c in m['commands']]
 mods+=f'''pub mod v2_{slug} {{
 pub fn commands()-> &'static [&'static str] {{ &{json.dumps(commands)} }}
 pub async fn handle(_: &crate::native_core::common::CoreContext,_:&str,_:&str,_:&serde_json::Value,_:&mut serde_json::Value)->crate::native_core::common::CoreResult {{ Err("Unrelated adapter stub".into()) }}
}}
'''
mods+='#[cfg(test)]\n#[path="../../../adapter_tests.rs"]\nmod migration_tests;\n'
(e/'mod.rs').write_text(mods)
prereq_paths=['src-tauri/src/native_core/extensions/v2_catalog.rs','src-tauri/src/native_core/extensions/registry.rs','src/renderer/extensionImages.ts','src/renderer/src/raycast-api/detail-runtime.tsx']
pf={p:(h/p.removeprefix('src-tauri/src/native_core/')).read_text() if p.startswith('src-tauri/') else (v/'fixtures/current'/p).read_text() for p in prereq_paths}
# Prerequisite transformation expects host-private visibility; restore it only in memory.
pf[prereq_paths[0]]=pf[prereq_paths[0]].replace('pub fn store_item','pub(super) fn store_item')
for p,s in prerequisites(pf).items():
 if p.startswith('src-tauri/'):(h/p.removeprefix('src-tauri/src/native_core/')).write_text(s.replace('pub(super)','pub'))
(h/'mod.rs').write_text('pub mod common;\npub mod extensions;\n')
print('Pinned beta harness production common/SDK/NativeSessions/registry/catalog/runtime prepared')
