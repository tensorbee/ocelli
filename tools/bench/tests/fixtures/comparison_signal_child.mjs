// No browser is launched. A child process receives the real SIGTERM and
// proves the public controller releases its owned resources and directory.
import {setInterval,clearInterval} from "node:timers";
import {writeFile,mkdir,access} from "node:fs/promises";
import {join} from "node:path";
import {runComparison} from "../../src/comparison_run.mjs";
const out=process.argv[2];let ownedTemp;
// Fake actors have no socket to keep Node alive while a signal is pending.
const keepAlive=setInterval(()=>{},1000);
await mkdir(join(out,"input"),{recursive:true});
try{await runComparison({pseudo:join(out,"input"),out:join(out,"out"),build:false},{
 prepare:async(_root,temp)=>{ownedTemp=temp;return{denySet:{},selections:{},directories:{},identity:"a".repeat(64)};},
 exclusive:async()=>{},
 build:async()=>({close:async()=>writeFile(join(out,"infrastructure-closed"),"yes")}),
 checkPixels:async context=>new Promise(resolve=>{
  context.actors.add({close:async()=>{await writeFile(join(out,"actor-closed"),"yes");resolve({});}});
  setTimeout(()=>process.kill(process.pid,"SIGTERM"),30);
 }),
});process.exitCode=99;}catch(error){
 process.exitCode=error.exitCode||98;let remains=true;try{await access(ownedTemp);}catch{remains=false;}
 console.log(JSON.stringify({exit_code:process.exitCode,temporary_directory_remains:remains}));
}

clearInterval(keepAlive);
