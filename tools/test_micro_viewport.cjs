"use strict";

// Real Chromium viewport acceptance for the production handheld shell.
// Loopback fixture only: no peers, hardware, scanning, or external network.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");
const {spawn,spawnSync} = require("node:child_process");

const root = path.resolve(__dirname, "..");
const assets = path.join(root, "crates/pipboy/assets");
const htmlFile = path.join(assets, "micro.html");
const probeFile = path.join(__dirname, "fixtures/micro-viewport-probe.js");
const toolIds = [
  "library","notes","host","byte-lab","game","geo-lab",
  "simulator","diag","net-lab","mesh"
];
const roleIds = [
  "overseer","head-engineer","security-chief","vault-doctor",
  "university-researcher","reclamation-scout","records-custodian","custom"
];
function state(scenario) {
  const custom=scenario==="custom";
  const activeProfile=custom?"custom":scenario==="scout"?"reclamation-scout":scenario==="security"?"security-chief":"overseer";
  return {
    profiles:roleIds.map((id,index)=>({
      id,title:id.toUpperCase().replace(/-/g," "),
      description:"Synthetic role "+(index+1),
      tools:id==="custom"?[]:toolIds.slice()
    })),
    palette:toolIds.map(id=>({
      id,name:id.toUpperCase().replace(/-/g," "),
      section:["host","diag"].includes(id)?"stat":["notes","library","game"].includes(id)?"data":"role",
      description:"Fixture-only "+id,command:"no action",availability:"ready"
    })),
    active:{
      profile:activeProfile,
      slots:custom?[]:toolIds.map((tool,index)=>({id:"slot-"+index,tool}))
    },
    camera_feeds:["DC","NYC","BOSTON","SF"].map((city,index)=>({id:"gate-"+index,city,name:city+" access-gated fixture",publisher:"Fixture DOT",region:"MA",lat:40+index,lon:-75+index,format:"unavailable",image_url:"",source_url:"https://example.org/camera"})),
    net_operations:[]
  };
}
function findChrome() {
  for(const p of [
    process.env.CHROME_BIN,"/usr/bin/google-chrome","/usr/bin/chromium",
    "google-chrome","chromium","chromium-browser"
  ].filter(Boolean)){
    if(path.isAbsolute(p)&&fs.existsSync(p))return p;
    const r=spawnSync("which",[p],{encoding:"utf8"});
    if(r.status===0&&r.stdout.trim())return r.stdout.trim().split("\n")[0];
  }
  throw new Error("Chromium required; set CHROME_BIN");
}
function fixture(desktop=false) {
  return fs.readFileSync(desktop ? path.join(assets,"dashboard.html") : htmlFile,"utf8")
    .replace("<head>",'<head><meta http-equiv="Content-Security-Policy" content="frame-src \'none\'">')
    .replace("</body>",'<script defer src="'+(desktop?'/desktop-probe.js':'/probe.js')+'"></script></body>');
}
function host(width,height,scenario) {
  const script=[
    'const frame=document.querySelector("iframe");',
    'const out=document.querySelector("#viewport-report");',
    'window.addEventListener("message",event=>{',
    'if(event.origin===location.origin&&event.source===frame.contentWindow&&',
    'event.data?.type==="dashboard-viewport")',
    'out.textContent=btoa(JSON.stringify(event.data.value));',
    '});',
    'setTimeout(()=>{if(!out.textContent)',
    'out.textContent=btoa(JSON.stringify({error:"Missing frame report"}));},4500);'
  ].join("");
  return '<!doctype html><html lang="en"><meta charset="utf-8">' +
    '<style>html,body{margin:0}iframe{border:0;display:block}</style>' +
    '<iframe width="'+width+'" height="'+height+'" style="width:'+width+'px;height:'+height+
    'px" src="/fixture?scenario='+scenario+'"></iframe>' +
    '<pre hidden id="viewport-report"></pre><script>'+script+'</script></html>';
}
function chromeRun(bin,port,width,height,scenario) {
  return new Promise((resolve,reject)=>{
    const profile=fs.mkdtempSync(path.join(os.tmpdir(),"rb-dash-view-"));
    const url="http://127.0.0.1:"+port+"/host?w="+width+"&h="+height+"&scenario="+scenario;
    const proc=spawn(bin,[
      "--headless","--no-sandbox","--disable-gpu","--disable-dev-shm-usage",
      "--disable-background-networking","--no-first-run","--no-default-browser-check",
      "--force-device-scale-factor=1","--window-size=2000,1200",
      "--virtual-time-budget=6000","--dump-dom","--user-data-dir="+profile,url
    ],{stdio:["ignore","pipe","pipe"]});
    let output="",error="",done=false;
    const cleanup=()=>fs.rmSync(profile,{recursive:true,force:true});
    const fail=e=>{if(done)return;done=true;clearTimeout(timer);cleanup();reject(e);};
    const timer=setTimeout(()=>{
      proc.kill("SIGKILL");
      fail(new Error("Timeout "+width+"x"+height+" "+scenario));
    },35000);
    proc.stdout.on("data",part=>{
      output+=part.toString();
      if(output.length>10_000_000)proc.kill("SIGKILL");
    });
    proc.stderr.on("data",part=>{
      error+=part.toString();
      if(error.length>16000)error=error.slice(-16000);
    });
    proc.on("error",fail);
    proc.on("close",code=>{
      if(done)return;
      done=true;clearTimeout(timer);cleanup();
      if(code!==0)return reject(new Error("Chromium "+code+": "+error.slice(-1500)));
      const m=output.match(/<pre[^>]*id="viewport-report"[^>]*>([A-Za-z0-9+/=]+)<\/pre>/);
      if(!m)return reject(new Error("Missing browser report: "+output.slice(-1000)));
      try{resolve(JSON.parse(Buffer.from(m[1],"base64").toString("utf8")));}
      catch(e){reject(e);}
    });
  });
}
async function main() {
  const bin=findChrome();
  const server=http.createServer((req,res)=>{
    const url=new URL(req.url,"http://127.0.0.1");
    const send=(status,body,type="application/json")=>{
      res.writeHead(status,{"Content-Type":type,"Cache-Control":"no-store"});res.end(body);
    };
    if(req.method!=="GET")return send(405,"Denied","text/plain");
    if(url.pathname==="/host")return send(200,host(Number(url.searchParams.get("w")),
      Number(url.searchParams.get("h")),url.searchParams.get("scenario")),"text/html");
    if(url.pathname==="/fixture")return send(200,fixture(url.searchParams.get("scenario")==="desktop-map"),"text/html");
    if(url.pathname==="/desktop-probe.js")return send(200,fs.readFileSync(path.join(__dirname,"fixtures/desktop-map-probe.js"),"utf8"),"text/javascript");
    if(url.pathname==="/probe.js")return send(200,fs.readFileSync(probeFile,"utf8"),"text/javascript");
    if(url.pathname==="/api/state")return send(200,JSON.stringify(state(
      (req.headers.referer||"").includes("scenario=desktop-map")?"scout":
      (req.headers.referer||"").includes("scenario=custom")?"custom":"security")));
    if(url.pathname==="/api/tasks")return send(200,JSON.stringify({board:{tasks:[]},manager:true}));
    if(url.pathname==="/api/radio")return send(200,JSON.stringify({source:"none",status:"stopped",
      station:null,volume:60,outputs:[],output:"system",presets:{favorites:[],last_selected:null},
      stations:[{id:"skylab",name:"Invented station",genre:"Test",category:"TEST",format:"MP3"}],sdr:"not_connected"}));
    if(url.pathname.startsWith("/assets/")){
      const name=url.pathname.slice(8);
      if(!/^[a-z0-9-]+\.(js|css)$/.test(name))return send(403,"Denied","text/plain");
      const file=path.join(assets,name);
      if(fs.existsSync(file))return send(200,fs.readFileSync(file,"utf8"),
        name.endsWith(".css")?"text/css":"text/javascript");
    }
    return send(404,JSON.stringify({error:"Fixture has no such resource"}));
  });
  await new Promise(resolve=>server.listen(0,"127.0.0.1",resolve));
  try {
    for(const [w,h,scenario] of [[480,320,"default"],[800,480,"default"],[320,568,"default"],[480,320,"custom"]]){
      const r=await chromeRun(bin,server.address().port,w,h,scenario);
      assert.ok(!r.error,r.error);
      assert.deepEqual(r.tabs,["stat","role","data","map","radio"]);
      assert.equal(r.settings,true,"Settings header target clipped");
      for(const page of r.pages){
        assert.ok(page.fits,`${w}x${h} ${page.id} overflow: ${JSON.stringify(page)}`);
        assert.ok(page.controlsFit,`${w}x${h} ${page.id} controls clipped`);
      }
      assert.ok(r.calls.every(x=>x.method==="GET" && x.url.startsWith("/api/")),
        "Navigation must only read local state");
      console.log(`PASS micro production viewport ${w}x${h} ${scenario}`);
    }
    for(const [w,h] of [[1280,800],[1920,1080],[800,600]]){
      const r=await chromeRun(bin,server.address().port,w,h,"desktop-map");
      assert.ok(!r.error,r.error);
      assert.ok(r.coverage>=0.95,`Desktop map coverage ${r.coverage}: ${JSON.stringify(r)}`);
      assert.equal(r.contained,true,"Map coverage must fit inside the workspace without clipping");
      assert.equal(r.mapButtons,0,"Desktop map workspace must have no visible buttons");
      assert.equal(r.navigation,true,"Navigation must remain outside the map zone");
      assert.equal(r.settings,true,"Settings must remain reachable from map navigation");
      console.log(`PASS desktop map ${w}x${h}: ${(r.coverage*100).toFixed(1)}% coverage`);
    }
  } finally { await new Promise(resolve=>server.close(resolve)); }
}
main().catch(e=>{console.error(e);process.exitCode=1;});
