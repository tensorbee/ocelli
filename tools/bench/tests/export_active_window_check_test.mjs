// F-227 caller-input export controls, derived from the accepted contract.
// Every file and window string in this VM is synthetic.
import fs from "node:fs";
import vm from "node:vm";
import assert from "node:assert/strict";
import test from "node:test";
const source = fs.readFileSync(new URL("../export_active_window_check.js", import.meta.url), "utf8");
function setup(images = [{file:{webkitRelativePath:"root/a"},frame:0},{file:{webkitRelativePath:"root/b"},frame:0}]) {
  const notes=[],downloads=[],timers=[],revoked=[],blobs=new Map(); let serial=0;
  const vp={viewIndex:0},series={key:"synthetic-series",images},entry={center:"7.0000",width:"1.0000"};
  const context={cacheGeneration:1,seriesWindows:new Map([[series.key,entry]]),seriesWindowJobs:new Map(),seriesWindowRefused:new Set(),
    activeViewport:()=>vp,seriesOf:x=>x===vp?series:null,currentImage:()=>series.images[vp.viewIndex]||null,
    seriesHeader:(_,view)=>view.overrideWindow||view.activePreset?"":entry.center+" "+entry.width,
    alert:x=>notes.push(x),Blob:class{constructor(parts){this.text=parts.join("");}},
    URL:{createObjectURL:b=>{const u="blob:synthetic-"+(++serial);blobs.set(u,b);return u;},revokeObjectURL:u=>revoked.push(u)},
    document:{body:{appendChild:a=>{a.attached=true;}},createElement:()=>({remove(){this.attached=false;},click(){downloads.push({name:this.download,value:JSON.parse(blobs.get(this.href).text)});}})},
    setTimeout:fn=>timers.push(fn)};
  context.window=context;vm.createContext(context);vm.runInContext(source,context);
  return {context,vp,series,entry,notes,downloads,timers,revoked,run(mode="auto"){const returned=context.exportActiveWindowCheck(mode);timers.splice(0).forEach(f=>f());return returned;}};
}
const tests = {
  ordinary_files_keep_frame_zero(){const x=setup();assert.equal(x.run(),undefined);assert.deepEqual(x.downloads[0].value.records.map(r=>r.frame),[0,0]);},
  reordered_multiframe_indices_preserved(){const x=setup([2,0,1].map(frame=>({file:{webkitRelativePath:"root/multi"},frame})));x.run();assert.deepEqual(x.downloads[0].value.records.map(r=>r.frame),[2,0,1]);},
  invalid_source_frame_refused(){for(const frame of [-1,.5,NaN]){const x=setup([{file:{webkitRelativePath:"root/a"},frame}]);x.run();assert.equal(x.downloads.length,0);assert.deepEqual(x.notes,["Window check unavailable."]);}},
  exact_paths_and_unrounded_strings(){const x=setup([{file:{webkitRelativePath:"root/sub/a"},frame:0}]);x.run();const y=x.downloads[0];assert.equal(y.name,"ocelli-window-check.json");assert.deepEqual(y.value.records,[{path:"sub/a",frame:0}]);assert.deepEqual(y.value.observed,[{center:"7.0000",width:"1.0000"}]);assert.equal(x.revoked.length,1);},
  unsafe_or_nonfolder_path_refused(){for(const path of ["a","/root/a","root/../a","root//a","C:/a","root/a\\b","root/a\0b"]){const x=setup([{file:{webkitRelativePath:path},frame:0}]);x.run();assert.equal(x.downloads.length,0);}},
  mixed_folder_roots_refused(){const x=setup([{file:{webkitRelativePath:"a/x"},frame:0},{file:{webkitRelativePath:"b/y"},frame:0}]);x.run();assert.equal(x.downloads.length,0);},
  auto_manual_preset_draft_refused(){for(const key of ["overrideWindow","activePreset","draftWindow"]){const x=setup();x.vp[key]={};x.run();assert.equal(x.downloads.length,0);}},
  jobs_and_refusal_refused(){for(const key of ["seriesWindowJobs","seriesWindowRefused"]){const x=setup();if(x.context[key].set){x.context[key].set(x.series.key,{});}else{x.context[key].add(x.series.key);}x.run();assert.equal(x.downloads.length,0);}},
  nonauto_has_no_observation_claim(){const x=setup();x.context.seriesWindows.clear();x.run("file-voi-control");assert.deepEqual(x.downloads[0].value.observed,[]);},
  changed_generation_refused(){const x=setup();Object.defineProperty(x.series.images[0].file,"webkitRelativePath",{get(){x.context.cacheGeneration++;return "root/a";}});x.run();assert.equal(x.downloads.length,0);},
  failed_download_releases_blob(){const x=setup();x.context.document.createElement=()=>({remove(){},click(){throw Error("synthetic");}});x.run();assert.equal(x.revoked.length,1);assert.deepEqual(x.notes,["Window check unavailable."]);}
};

for (const [name, run] of Object.entries(tests)) test(name, run);
