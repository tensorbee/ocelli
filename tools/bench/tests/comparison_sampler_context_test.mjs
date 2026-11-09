import assert from 'node:assert/strict';
import test from 'node:test';
import {readFile,writeFile,mkdtemp,rm,stat} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {collectPreparedColourComparison,colourCaseSpec} from '../src/comparison_colour.mjs';

const POISON = 'PRIVATE_SYNTHETIC_778899_/not-a-real-input.dcm';
const sourceUrl = new URL('../src/comparison_run.mjs', import.meta.url);
const block = (pid,footprint=10,peak=20) => `fixture [${pid}]: 64-bit Footprint: ${footprint} B\nphys_footprint: ${footprint} B\nphys_footprint_peak: ${peak} B\n`;
function foreign(code) {
  return Object.assign(new Error(POISON), {code,stdout:POISON,stderr:POISON,path:POISON,pid:778899});
}
async function fixture(mode='success') {
  const dir = await mkdtemp(join(tmpdir(),'comparison-sampler-context-'));
  const key = '__samplerContext'+dir.replace(/\W/g,'');
  const state = {mode,calls:[],actors:[],active:null,productCalls:0,infraCalls:0};
  const browser = {
    version:()=> 'fixture-browser',
    newBrowserCDPSession:async()=>({
      send:async()=> {if(state.mode==='inventory') throw foreign('EPERM');return {processInfo:[{type:'browser',id:10},{type:'renderer',id:11}]};},
      detach:async()=>{},
    }),
  };
  state.exec = async(file,args,options) => {
    state.calls.push({file,args:[...args],maxBuffer:options?.maxBuffer});
    if(file==='ps') {
      if(args.includes('pid=,ppid=')) return {stdout:'10 1\n11 10\n20 1\n'};
      assert(args.includes('pid=,time='));
      const time=state.active?.navigated?'00:02.00':'00:01.00';
      return {stdout:`10 ${time}\n11 ${time}\n20 ${time}\n`};
    }
    assert.equal(file,'/usr/bin/footprint');
    assert.deepEqual(args.slice(0,2),['-f','bytes']);
    const ids=args.filter((_,i)=>i>=3&&i%2===1).map(Number);
    const infra=ids.length===1&&(ids[0]===99||ids[0]===process.pid);
    if(infra) {
      state.infraCalls++;
      assert.equal(options.maxBuffer,4*1024*1024);
      if(state.mode==='infrastructure-tool') throw foreign('ENOENT');
      return {stdout:state.mode==='infrastructure-parse'?POISON:block(99)};
    }
    state.productCalls++;
    assert.deepEqual(ids,state.active?.side==='cornerstone'?[10,11]:[10,11,20]);
    assert.equal(options.maxBuffer,16*1024*1024);
    if(state.mode==='product-tool') throw foreign('ERR_CHILD_PROCESS_STDIO_MAXBUFFER');
    if(state.mode==='permission') throw foreign('EACCES');
    if(state.mode==='numeric-code') throw foreign(73);
    if(state.mode==='killed') throw Object.assign(foreign(undefined),{killed:true});
    if(state.mode==='unknown-code') throw foreign(POISON);
    if(state.mode==='object-code') throw foreign({toString(){throw new Error(POISON);}});
    if(state.mode==='product-parse') return {stdout:POISON};
    if(state.mode==='product-coverage') return {stdout:block(10)+block(20)};
    if(state.mode==='product-values') return {stdout:block(10,0)+block(11)+block(20)};
    if(state.mode==='bad-peak') return {stdout:block(10,10,9)+block(11)+block(20)};
    return {stdout:ids.map(pid=>block(pid)).join('')};
  };
  state.launchSide = async(side,context) => {
    const actor={side,browser,server:side==='ocelli'?{pid:20}:null,navigated:false,closed:false,
      nativeBudgetBytes:side==='ocelli'?512*1024*1024:null,
      page:{evaluate:async()=> 'fixture-renderer'},
      navigate:async()=> {actor.navigated=true;},
      close:async()=> {actor.closed=true;context.actors.delete(actor);},
    };
    state.active=actor;state.actors.push(actor);context.actors.add(actor);return actor;
  };
  state.openFolder=async()=>({open:1,first_image:2});state.windowStep=async()=>1;
  globalThis[key]=state;
  let source=await readFile(sourceUrl,'utf8');
  assert.equal(source.split('const exec = promisify(execFile);').length,2);
  source=source.replace('const exec = promisify(execFile);',`const exec=globalThis[${JSON.stringify(key)}].exec;`);
  for(const name of ['launchSide','openFolder','windowStep']) {
    const declaration='export async function '+name+'(';
    assert.equal(source.split(declaration).length,2);
    source=source.replace(declaration,'export async function unusedTransport_'+name+'(');
    source+=`\nconst ${name}=globalThis[${JSON.stringify(key)}].${name};\n`;
  }
  source=source.replace(/from "(\.[^"]+)"/g,(_,name)=>'from "'+new URL(name,sourceUrl).href+'"');
  const copied=join(dir,'comparison_run.mjs');await writeFile(copied,source);
  const module=await import(pathToFileURL(copied).href);
  return {state,module,browser,close:async()=>{delete globalThis[key];await rm(dir,{recursive:true,force:true});}};
}
function safe(error,stage,category) {
  assert.equal(error.message,`process memory collection failed [stage=${stage},category=${category}]`);
  assert.equal(error.cause.exitCode,1);
  assert(!String(error).includes(POISON));assert(!JSON.stringify(error).includes(POISON));
  assert(!error.message.includes('778899'));assert(!error.message.includes('/not-a-real'));
}

for(const [mode,stage,category] of [
  ['inventory','inventory','permission'],['product-tool','product-tool','output-limit'],
  ['permission','product-tool','permission'],['numeric-code','product-tool','numeric-code'],
  ['killed','product-tool','killed'],['unknown-code','product-tool','unclassified'],
  ['object-code','product-tool','unclassified'],['product-parse','product-parse','unclassified'],
  ['product-coverage','product-coverage','comparison-refusal'],['product-values','product-values','comparison-refusal'],
  ['bad-peak','product-values','comparison-refusal'],['infrastructure-tool','infrastructure-tool','tool-missing'],
  ['infrastructure-parse','infrastructure-parse','unclassified'],
]) test('actual sampler preserves safe '+mode+' context without retry',async()=>{
  const f=await fixture(mode);
  try {
    const sampler=f.module.startProcessSampler(f.browser,20,99);
    await assert.rejects(sampler.stop(),error=>{safe(error,stage,category);return true;});
    const calls=f.state.calls.length;
    await assert.rejects(sampler.stop(),error=>{safe(error,stage,category);return true;});
    assert.equal(f.state.calls.length,calls,'another stop cannot retry I/O');
    assert(f.state.productCalls<=1);assert(f.state.infraCalls<=1);
  } finally {await f.close();}
});

test('actual successful sampler retains receipts and original command arguments',async()=>{
  const f=await fixture();
  try {
    const result=await f.module.startProcessSampler(f.browser,20,99).stop();
    assert.deepEqual(Object.keys(result).sort(),['figures','infrastructure','sample_times_ms','samples']);
    assert.deepEqual(result.figures,{sampled_peak:30,sum_of_peaks:60});
    assert.deepEqual(result.samples,[[[10,{footprint:10,peak:20}],[11,{footprint:10,peak:20}],[20,{footprint:10,peak:20}]]]);
    assert.deepEqual(result.infrastructure,[[[99,{footprint:10,peak:20}]]]);
    assert.equal(result.sample_times_ms.length,1);assert(Number.isFinite(result.sample_times_ms[0]));
    assert.equal(f.state.productCalls,1);assert.equal(f.state.infraCalls,1);
  } finally {await f.close();}
});

test('actual colour consumer propagates sampler stage and closes its actor',async()=>{
  const f=await fixture('product-tool');const context={actors:new Set()};
  try {
    await assert.rejects(f.module.measureColourMemorySession('ocelli',{directory:'fixture-only',caseSpec:{serverCacheMiB:512},context}),error=>{safe(error,'product-tool','output-limit');return true;});
    assert.equal(f.state.actors.length,1);assert(f.state.actors[0].closed);assert.equal(context.actors.size,0);
    assert.equal(f.state.productCalls,1,'finally stop is not a second sample attempt');
  } finally {await f.close();}
});

test('actual comparison boundary preserves safe stage and removes unaccepted output',async()=>{
  const f=await fixture('infrastructure-parse');const out=await mkdtemp(join(tmpdir(),'sampler-consumer-output-'));
  try {
    const metadata=[0,1].map(()=>({version:'fixture-browser',renderer:'fixture-renderer',defaults:{}}));
    await assert.rejects(f.module.runComparison({pseudo:'fixture-only',out,build:false},{
      exclusive:async()=>{},prepare:async()=>({identity:'a'.repeat(64),selections:{},directories:{},denySet:{names:[],ids:[],dates:[],institutions:[],uids:[],paths:[]}}),
      build:async()=>({actors:new Set(),close:async()=>{}}),checkPixels:async()=>({verdict:{passes:true,low_information:false},metadata}),
      measure:async()=>{await f.module.startProcessSampler(f.browser,20,99).stop();throw new Error('unreachable');},
    }),error=>{safe(error,'infrastructure-parse','unclassified');assert.equal(error.safeMessage,error.message);assert.equal(error.exitCode,1);return true;});
    for(const name of ['lock','comparison.json','comparison.txt']) await assert.rejects(stat(join(out,name)),{code:'ENOENT'});
    assert.equal(f.state.productCalls,1);assert.equal(f.state.infraCalls,1);
  } finally {await f.close();await rm(out,{recursive:true,force:true});}
});

// Actual colour collector orchestration and actual colour memory consumer, with
// OS/browser transports only substituted. Earlier timing/window work is fake.
test('actual collector aborts at a failed memory ordinal without replacing a session',async()=>{
  const f=await fixture();const spec=colourCaseSpec('icc16-table');let calls=0;
  const context={actors:new Set(),host:{platform:'fixture',release:'fixture',arch:'fixture',cpu_model:'fixture',cpu_count:1,memory_bytes:1},startedAt:'2026-11-09T00:00:00.000Z',source:{head:'a'.repeat(40),index_tree:'b'.repeat(40),worktree_matches_index:true,binary_sha256:'c'.repeat(64),comparator_sha256:'d'.repeat(64),page_sha256:'e'.repeat(64)},pixelPrecheck:{verdict:{passes:true,low_information:false},input_identity:'f'.repeat(64),browser:{},versions:{},defaults:{},not_shown:[]}};
  const input={spec,targetDirectory:'fixture-only',publicIdentity:{manifest_sha256:'1'.repeat(64),dataset_sha256:'2'.repeat(64),series_sha256:'3'.repeat(64),profile:{kind:spec.profileKind,bytes:560,sha256:'4'.repeat(64)}}};
  const receipt=side=>({open_ms:1,first_image_ms:2,browser_version:'fixture-browser',renderer:'fixture-renderer',native_budget_bytes:side==='ocelli'?512*1024*1024:null});
  try {
    await assert.rejects(collectPreparedColourComparison(input,context,{
      measureColourTrial:async side=>receipt(side),launchSide:async side=>({nativeBudgetBytes:side==='ocelli'?512*1024*1024:null,close:async()=>{}}),openFolder:async()=>{},windowStep:async()=>1,
      measureColourMemorySession:async(side,request)=>{calls++;f.state.mode=calls===2?'product-tool':'success';return f.module.measureColourMemorySession(side,request);},
    }),error=>{safe(error,'product-tool','output-limit');return true;});
    assert.equal(calls,2,'failed second ordinal stops remaining four memory calls');
    assert(f.state.actors.every(actor=>actor.closed));assert.equal(context.actors.size,0);
  } finally {await f.close();}
});
