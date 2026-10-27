// JSDOM executes only generated fixture markup. Network and OS effects are fake.
const {JSDOM,ResourceLoader,VirtualConsole}=require('jsdom');const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const f=JSON.parse(fs.readFileSync(path.join(__dirname,'../fixtures/oracle/native-preview.json')));
class Offline extends ResourceLoader{fetch(url){throw Error('Unexpected network: '+url)}}
async function run(html,{modern='ok',fallback=true}={}){
 const calls=[],timers=[],alerts=[];const dom=new JSDOM(html,{url:'file:///disposable-preview.html',runScripts:'dangerously',resources:new Offline(),virtualConsole:new VirtualConsole(),beforeParse(w){w.execCommand=undefined;w.document.execCommand=command=>{calls.push(['execCommand',command]);if(fallback==='throw')throw Error('fixture failure');return fallback};w.ClipboardItem=class{constructor(data){this.data=data}};if(modern!==false)Object.defineProperty(w.navigator,'clipboard',{value:{write:async data=>{calls.push(['html',data[0].data['text/html']]);if(modern==='fail')throw Error('fixture failure')}}});w.setTimeout=f=>{timers.push(f);return timers.length};w.alert=s=>alerts.push(s)}});
 await new Promise(r=>dom.window.document.addEventListener('DOMContentLoaded',r));assert.equal(dom.window.untrusted,undefined);
 const d=dom.window.document;assert.equal(d.querySelectorAll('script').length,1);assert(!d.querySelector('.content-container script,iframe,meta[http-equiv=refresh],base'));assert(![...d.querySelectorAll('.content-container *')].some(e=>[...e.attributes].some(a=>a.name.startsWith('on'))));
 const csp=d.querySelector('meta[http-equiv="Content-Security-Policy"]').content;const hash=crypto.createHash('sha256').update(d.querySelector('script').textContent).digest('base64');assert(csp.includes(`'sha256-${hash}'`));assert(csp.includes("connect-src 'none'"));
 d.querySelector('.button').click();for(let i=0;i<12;i++)await Promise.resolve();
 return {dom,calls,alerts,timers,d};
}
(async()=>{
 assert(f.styled.includes('<script>window.untrusted=1</script>')); // core output is not claimed sanitized.
 let r=await run(f.normal);assert.equal(r.calls[0][0],'html');assert.equal(r.calls[0][1].type,'text/html');assert.equal(r.calls.length,1);assert.equal(r.d.querySelector('#copySuccess').style.display,'block');assert.match(r.d.querySelector('#copyButton').textContent,/Successful/);r.timers.forEach(f=>f());assert.equal(r.d.querySelector('#copySuccess').style.display,'none');
 r.d.querySelector('.content-container').click();assert(r.dom.window.getSelection().rangeCount>0);r.dom.window.close();
 for(const modern of ['fail',false]){r=await run(f.normal,{modern,fallback:true});assert(r.calls.some(c=>c[0]==='execCommand'));assert.equal(r.d.querySelector('#copySuccess').style.display,'block');r.dom.window.close()}
 for(const fallback of [false,'throw']){r=await run(f.normal,{modern:'fail',fallback});assert.notEqual(r.d.querySelector('#copySuccess').style.display,'block');assert(!r.d.querySelector('#copyButton').textContent.includes('Successful'));r.dom.window.close()}
 for(const fallback of [true,false,'throw']){r=await run(f.sample,{modern:false,fallback});assert.equal(r.alerts.length,1);assert.equal(r.alerts[0].startsWith('Content copied!'),fallback===true);r.dom.window.close()}
 // The adapter entity-encodes all selected characters; the actual host marked
 // grammar cannot turn the source into links/images/HTML or code-copy headers.
 const marked=require('../fixtures/libraries/marked/lib/marked.cjs').marked;const d=new JSDOM(marked(f.detail)).window.document;assert.equal(d.querySelectorAll('pre,code,a,img,script').length,0);assert(d.body.textContent.includes('<script>window.untrusted=1</script>'));
 console.log('Browser helper: modern/fallback success, failure/throw, sample failure feedback, content selection, isolation and CSP hash passed; JSDOM does not enforce CSP, real browser QA not run');
})().catch(e=>{console.error(e);process.exitCode=1});
