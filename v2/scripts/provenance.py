"""Verify immutable fixture size/hash/SRI and exact independent icon/library pins.
--write regenerates only the inventory, never repairs fixture contents.
"""
import pathlib,hashlib,base64,json,sys
v=pathlib.Path(__file__).resolve().parent.parent
OWN='7bd8f532fae79ce15a2f96d62fc2538fb4b24cfd';ORIGINAL='6734d85992290066fd31a7f5bee3cca3d3545a1a';BETA='fd300e276356587726566026f072563eb9c34cb1';CURRENT='fe71ca87f21ac4ff31505010c228f8abda0f2a89'
def origin(rel):
 s=rel.as_posix()
 for prefix,repo,pin in [('fixtures/own/','almatkai/markdown-styler',OWN),('fixtures/beta/','almatkai/Tezbar',BETA),('fixtures/current/','almatkai/Tezbar',CURRENT)]:
  if s.startswith(prefix):return f'{"git-object:Tezbar-host" if prefix in ["fixtures/beta/","fixtures/current/"] else "git:"+repo}@{pin}:{s[len(prefix):].removesuffix(".txt") if s.endswith("Cargo.toml.txt") else s[len(prefix):]}'
 if s.startswith('fixtures/original/'):
  p=s[len('fixtures/original/'):];p='LICENSE' if p=='repository-LICENSE' else 'extensions/markdown-styler/'+p
  return f'https://raw.githubusercontent.com/raycast/extensions/{ORIGINAL}/{p}'
 if s.startswith('fixtures/libraries/marked/'):
  return 'https://registry.npmjs.org/marked/-/marked-9.1.6.tgz#'+s[len('fixtures/libraries/marked/'):]
 if s.startswith('fixtures/licenses/rust/'):
  package=s.split('/')[3];return 'crates.io:'+package+':'+s.split('/')[-1]
 if s.startswith('fixtures/libraries/'):
  package=s.split('/')[2];return 'crates.io:'+package+':'+s.split('/',3)[-1]
 if s.startswith('fixtures/licenses/npm/'):return 'npm-locked:scripts/package-lock.json:'+s[len('fixtures/licenses/npm/'):]
 if s=='fixtures/licenses/rust-packages.json':return 'generated:Cargo.lock+registry/Cargo.toml+exact-license-files'
 if s=='fixtures/oracle/native-preview.json':return 'generated:examples/preview.rs (native helper verification only)'
 return 'generated:scripts/oracle.cjs executing own TSX + marked@9.1.6'
rows=[]
for p in sorted((v/'fixtures').rglob('*')):
 if not p.is_file() or p.name=='INDEX.json':continue
 b=p.read_bytes();rel=p.relative_to(v);rows.append({'path':rel.as_posix(),'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest(),'sri':'sha256-'+base64.b64encode(hashlib.sha256(b).digest()).decode(),'source':origin(rel)})
index=v/'fixtures/INDEX.json'
if '--write' in sys.argv:index.write_text(json.dumps(rows,indent=2)+'\n')
else:assert json.loads(index.read_text())==rows,'Fixture inventory changed; inspect every record before regenerating'
icon=v/'assets/command-icon.png';expected='1048ba8570be6cdfd942e97df1a222253b63bf6e20c591b9c4565a865dc80eae';assert len(icon.read_bytes())==44175 and hashlib.sha256(icon.read_bytes()).hexdigest()==expected;assert icon.read_bytes()==(v/'fixtures/original/assets/command-icon.png').read_bytes()
locked=json.loads((v/'fixtures/own/package-lock.json').read_text())['packages']['node_modules/marked'];assert locked['version']=='9.1.6';tar=(v/'fixtures/libraries/marked/marked-9.1.6.tgz').read_bytes();assert 'sha512-'+base64.b64encode(hashlib.sha512(tar).digest()).decode()==locked['integrity']
assert json.loads((v/'fixtures/libraries/marked/package.json').read_text())['version']=='9.1.6'
print(f'{len(rows)} fixture size/SHA256/SRI/source records verified; independent original icon and actual marked lock integrity match')
