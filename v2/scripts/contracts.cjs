// Executes actual pinned Detail + Surface callbacks and MarkdownView. Framework,
// highlighting and window bridge are stubbed; parser and sanitizer are real.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict');
const ts=require('../.v1/node_modules/typescript'),{JSDOM}=require('jsdom');const v=path.resolve(__dirname,'..');
const dom=new JSDOM('<body/>');const effects=[],listeners={},invoked=[];
const jsx=(type,props)=>({type,props});const react={memo:f=>f,useCallback:f=>f,useMemo:f=>f(),useState:s=>[s,()=>{}],useRef:s=>({current:s}),useEffect:f=>effects.push(f)};
function load(file,stubs={},rewrite=s=>s){const source=rewrite(fs.readFileSync(file,'utf8'));const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX,esModuleInterop:true}}).outputText;const module={exports:{}};vm.runInNewContext(js,{module,exports:module.exports,require:n=>{if(n==='react')return react;if(n==='react/jsx-runtime')return {jsx,jsxs:jsx};if(n in stubs)return stubs[n];throw Error(n)},window:{addEventListener:(k,f)=>listeners[k]=f,removeEventListener(){},setTimeout(){},clearTimeout(){}},DOMParser:dom.window.DOMParser,Element:dom.window.Element,Set,Map,console},{filename:file});return module.exports;}
function nodes(n,out=[]){if(!n)return out;if(Array.isArray(n)){n.forEach(x=>nodes(x,out));return out}out.push(n);nodes(n.props?.children,out);return out}
const basedir=path.join(v,'fixtures/beta/src/renderer/src/raycast-api');
const detailFile=path.join(basedir,'detail-runtime.tsx');
const detail=(fixed)=>load(detailFile,{'../../colorConverter':{},'../../ui/Markdown':{Markdown:'Markdown'},'../../ui/primitives':{cx:()=>''}},s=>fixed?s.replace('onClick={onRunPrimaryAction}','onClick={() => onRunPrimaryAction()}'):s).DetailRuntime;
const stubs={'./action-runtime':{},'./action-runtime-registry':{ActionRegistryContext:{Provider:'ActionRegistry'}},'./detail-runtime':{DetailRuntime:'DetailRuntime'},'./form-runtime':{},'./grid-runtime':{},'./list-runtime':{},'./hooks':{NavigationContext:{Provider:'Navigation'},setRuntimeContext(){}}};
const surface=load(path.join(basedir,'index.tsx'),stubs).ExtensionRuntimeSurface;
const root={type:'Detail',props:{markdown:'Fixture detail'}};const tree=surface({root,title:'Style Markdown',actions:[{id:'canonical-action',title:'Open in Browser and Copy',kind:'action'}],onInvokeAction:(...args)=>invoked.push(args),onBack(){},onSearchTextChanged(){},onLoadMore(){}});const props=nodes(tree).find(n=>n.type==='DetailRuntime').props;
// Reproduce pinned bug, then prove the minimal wrapper routes the current ID.
let t=detail(false)(props);let enter=nodes(t).find(n=>n.type==='button'&&n.props.children==='Enter');const event={type:'click'};enter.props.onClick(event);assert.strictEqual(invoked.pop()[0],event);
t=detail(true)(props);enter=nodes(t).find(n=>n.type==='button'&&n.props.children==='Enter');enter.props.onClick(event);assert.equal(invoked.pop()[0],'canonical-action');
effects.forEach(f=>f());listeners.keydown({key:'Enter',preventDefault(){}});assert.equal(invoked.pop()[0],'canonical-action');
(async()=>{
 const {Marked}=await import('marked');const DOMPurify=require('dompurify')(dom.window);
 const m=load(path.join(v,'fixtures/beta/src/renderer/ui/Markdown.tsx'),{'marked':{Marked},'marked-highlight':{markedHighlight:()=>({})},'highlight.js/lib/common':{getLanguage:()=>false},dompurify:DOMPurify});
 const f=JSON.parse(fs.readFileSync(path.join(v,'fixtures/oracle/native-preview.json')));const output=m.Markdown({text:f.detail});const markup=output.props.dangerouslySetInnerHTML.__html;const doc=new JSDOM(markup).window.document;assert.equal(doc.querySelectorAll('pre,code,a,img,script').length,0);assert(doc.body.textContent.includes('window.untrusted=1'));
 // Current image path regression and scoped fix, with negatives.
 const file=path.join(v,'fixtures/current/src/renderer/extensionImages.ts');const raw=load(file);assert.equal(raw.extensionImageSource('/extensions/icons/markdown-styler.png'),'file:///extensions/icons/markdown-styler.png');
 const fixed=load(file,{},s=>s.replace('export function extensionImageSource(value: string): string {','export function extensionImageSource(value: string): string {\n  if (/^\\/extensions\\/(icons|artwork)\\/[a-z0-9-]+\\.(png|webp|svg)$/.test(value)) return value'));
 assert.equal(fixed.extensionImageSource('/extensions/icons/markdown-styler.png'),'/extensions/icons/markdown-styler.png');for(const s of ['/extensions/icons/../secret.png','/Users/private.png','/extensions/icons/x.png?url=x'])assert(fixed.extensionImageSource(s).startsWith('file://'));
 console.log('Actual beta Detail/Surface Enter callback regression and wrapper; actual host marked@18.0.2 + DOMPurify Markdown safe display; current bundled icon regression and scoped fix passed');dom.window.close();
})().catch(e=>{console.error(e);process.exitCode=1});
