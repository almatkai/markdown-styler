"""Reviewable scoped compiled-in integration. Default prints a plan; --apply writes.
Never run this against a real host without its owner's authorized integration step.
Preserves all existing catalog rows and adapters; conflicts fail before mutation.
"""
import argparse,pathlib,json,re,shutil
from prerequisites import apply
v=pathlib.Path(__file__).resolve().parent.parent
TARGETS=['src-tauri/src/native_core/extensions/'+p for p in ['mod.rs','v2_runtime.rs','v2_catalog.rs','v2_catalog.json','registry.rs']]+['src-tauri/Cargo.toml','src/renderer/extensionImages.ts','src/renderer/src/raycast-api/detail-runtime.tsx']
def plan(host):
 originals={p:(host/p).read_text() for p in TARGETS};files=apply(originals.copy())
 p=TARGETS[0];s=files[p];decl='#[path = "../../../native-extensions/markdown-styler/adapter/v2_markdown_styler.rs"]\nmod v2_markdown_styler;'
 if 'mod v2_markdown_styler;' not in s:s=s.replace('mod v2_sdk;',decl+'\nmod v2_sdk;')
 if decl not in s:raise ValueError('Existing Markdown Styler module registration differs')
 files[p]=s
 p=TARGETS[1];s=files[p]
 if '"markdown-styler" => super::v2_markdown_styler::commands(),' not in s:
  # The source branches can contain other ports. Insert into the known default arms.
  m=re.search(r'(?m)^\s*_ => None,',s)
  if not m:raise ValueError('Runtime slug match missing')
  pos=s.rfind('=> Some(value)',0,m.start());start=s.rfind('"',0,pos)
  if pos<0 or start<0:raise ValueError('Runtime slug match unsupported')
  s=s[:pos]+'| "markdown-styler" '+s[pos:]
  s=s.replace('        _ => &[],','        "markdown-styler" => super::v2_markdown_styler::commands(),\n        _ => &[],',1)
  s=s.replace('        _ => Err(format!', '        "markdown-styler" => super::v2_markdown_styler::handle(ctx, command, operation, payload, state).await,\n        _ => Err(format!',1)
 files[p]=s
 p=TARGETS[3];catalog=json.loads(files[p]);manifest=json.loads((v/'manifest.json').read_text())
 existing=[x for x in catalog if x.get('id')==manifest['id']]
 if existing and existing!=[manifest]:raise ValueError('Existing Markdown Styler catalog differs')
 if not existing:catalog.append(manifest)
 files[p]=json.dumps(catalog,indent=2)+'\n'
 p=TARGETS[2];files[p]=re.sub(r'assert_eq!\(items\.len\(\), \d+\);','assert_eq!(items.len(), source().len());',files[p])
 p='src-tauri/Cargo.toml';line='tezbar-markdown-styler = { path = "native-extensions/markdown-styler" }'
 if line not in files[p]:
  if 'tezbar-markdown-styler' in files[p]:raise ValueError('Existing dependency differs')
  files[p]=files[p].replace('[dependencies]','[dependencies]\n'+line,1)
 if line not in files[p]:raise ValueError('Cargo dependencies anchor missing')
 changes={p:s.encode() for p,s in files.items() if s!=originals[p]}
 # Stable vendoring; excludes caches/builds and every node_modules directory.
 for source in v.rglob('*'):
  rel=source.relative_to(v)
  if not source.is_file() or any(x in {'target','node_modules','.v1','.scratch','__pycache__'} for x in rel.parts):continue
  target='src-tauri/native-extensions/markdown-styler/'+rel.as_posix()
  if (host/target).exists():
   if (host/target).read_bytes()!=source.read_bytes():raise ValueError('Vendored file conflict: '+target)
  else:changes[target]=source.read_bytes()
 icon='src/renderer/public/extensions/icons/markdown-styler.png';b=(v/'assets/command-icon.png').read_bytes()
 if (host/icon).exists() and (host/icon).read_bytes()!=b:raise ValueError('Existing icon differs')
 if not (host/icon).exists():changes[icon]=b
 return changes
if __name__=='__main__':
 parser=argparse.ArgumentParser();parser.add_argument('--host',required=True,type=pathlib.Path);parser.add_argument('--apply',action='store_true');args=parser.parse_args();changes=plan(args.host)
 print('\n'.join(sorted(changes)))
 if args.apply:
  for p,b in changes.items():t=args.host/p;t.parent.mkdir(parents=True,exist_ok=True);t.write_bytes(b)
