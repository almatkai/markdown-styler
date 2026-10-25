"""Package actual unstaged source, extract, verify fixture visibility, test offline.
Uses installed cached crates and disposable extraction; no absolute host paths.
"""
import pathlib,subprocess,tarfile,tempfile,json,os
v=pathlib.Path(__file__).resolve().parent.parent
subprocess.run(['cargo','+stable','package','--manifest-path',str(v/'Cargo.toml'),'--offline','--locked','--allow-dirty','--no-verify'],check=True)
archive=v/'target/package/tezbar-markdown-styler-0.1.0.crate'
with tempfile.TemporaryDirectory(prefix='markdown-styler-archive-') as name:
 with tarfile.open(archive,'r:gz') as tar:
  members=tar.getmembers();prefix='tezbar-markdown-styler-0.1.0/'
  assert all(m.name.startswith(prefix) and '..' not in pathlib.PurePosixPath(m.name).parts and not m.issym() and not m.islnk() for m in members)
  files={m.name[len(prefix):] for m in members if m.isfile()}
  expected={r['path'] for r in json.loads((v/'fixtures/INDEX.json').read_text())}|{'manifest.json','fixtures/INDEX.json','Cargo.lock','adapter/v2_markdown_styler.rs','integration/harness.rs','templates/normal.html','templates/sample.html','templates/sample.md','assets/command-icon.png','integration/prerequisites-fe71ca8.patch'}
  assert expected<=files,sorted(expected-files)
  assert not any('node_modules' in pathlib.PurePosixPath(p).parts or p.startswith(('.v1/','.scratch/','target/')) for p in files)
  assert not any(pathlib.PurePosixPath(p).name=='Cargo.toml' and p!='Cargo.toml' for p in files),'Nested fixture manifests must be .txt templates'
  tar.extractall(name)
 root=pathlib.Path(name)/prefix.rstrip('/')
 for folder in ['src','adapter','tests','integration']:
  for source in (v/folder).rglob('*'):
   if source.is_file() and source.name!='archive-files.txt' and not any(x in {'target','__pycache__'} for x in source.parts):
    assert (root/source.relative_to(v)).read_bytes()==source.read_bytes(),'Archive source differs: '+str(source.relative_to(v))
 subprocess.run(['python3',str(root/'scripts/provenance.py')],check=True)
 env=os.environ.copy();env['CARGO_TARGET_DIR']=str(pathlib.Path(name)/'fresh-build')
 for toolchain in ['stable','1.85.0']:
  subprocess.run(['cargo','+'+toolchain,'test','--manifest-path',str(root/'Cargo.toml'),'--offline','--locked'],check=True,env=env)
 (v/'integration/archive-files.txt').write_text('\n'.join(sorted(files))+'\n')
 print(f'Actual Cargo archive: {len(files)} files; all indexed fixtures visible; extracted offline locked tests passed on both toolchains')
# No staged files are needed to expose source to Git. Verify relevant artifacts
# are not hidden by parent ignore rules, while caches remain ignored.
repo=v.parent
if (repo/'.git').exists():
 for p in ['v2/manifest.json','v2/src/render.rs','v2/fixtures/INDEX.json','v2/assets/command-icon.png','v2/integration/harness.rs','v2/fixtures/oracle/oracle.json','v2/fixtures/beta/src-tauri/Cargo.toml.txt']:
  r=subprocess.run(['git','check-ignore','-q',p],cwd=repo)
  assert r.returncode==1,'Git ignores reviewable artifact '+p
 assert not subprocess.check_output(['git','diff','--cached','--name-only'],cwd=repo).strip(),'Task requires an empty staging area'
 print('Git visibility checked without staging; staging area remains empty')
