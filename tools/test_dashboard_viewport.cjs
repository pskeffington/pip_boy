"use strict";

// Real Chromium viewport acceptance for the production desktop shell.
// Loopback fixture only: no peers, hardware, scanning, or external network.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");
const {spawn,spawnSync} = require("node:child_process");

const root = path.resolve(__dirname, "..");
const assets = path.join(root, "crates/pipboy/assets");
const htmlFile = path.join(assets, "dashboard.html");
const probeFile = path.join(__dirname, "fixtures/dashboard-viewport-probe.js");
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
  return {
    profiles:roleIds.map((id,index)=>({
      id,title:id.toUpperCase().replace(/-/g," "),
      description:"Synthetic role "+(index+1),
      tools:id==="custom"?[]:toolIds.slice()
    })),
    palette:toolIds.map(id=>({
      id,name:id.toUpperCase().replace(/-/g," "),
      description:"Fixture-only "+id,command:"no action",availability:"ready"
    })),
    active:{
      profile:custom?"custom":"overseer",
      slots:custom?[]:toolIds.map((tool,index)=>({id:"slot-"+index,tool}))
    },
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
function fixture() {
  return fs.readFileSync(htmlFile,"utf8")
    .replace(/<script defer src="\/assets\/(?!dashboard\.js)[^"]+"><\/script>/g,"")
    .replace("</body>",'<script defer src="/probe.js"></script></body>');
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
    const send=(status,body,type)=>{
      res.writeHead(status,{"Content-Type":type,"Cache-Control":"no-store",
        "Content-Security-Policy":"default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; frame-src 'self'"});
      res.end(body);
    };
    if(req.method!=="GET")return send(405,"Denied","text/plain");
    if(url.pathname==="/host"){
      const w=Number(url.searchParams.get("w")),h=Number(url.searchParams.get("h"));
      const scenario=url.searchParams.get("scenario");
      if(!Number.isInteger(w)||!Number.isInteger(h)||w<300||w>2000||
        h<300||h>1200||!["default","custom"].includes(scenario))
        return send(400,"Invalid viewport","text/plain");
      return send(200,host(w,h,scenario),"text/html");
    }
    if(url.pathname==="/fixture")return send(200,fixture(),"text/html");
    if(url.pathname==="/probe.js")return send(200,fs.readFileSync(probeFile,"utf8"),"text/javascript");
    if(url.pathname==="/api/state"){
      const scenario=req.headers.referer?.includes("scenario=custom")?"custom":"default";
      return send(200,JSON.stringify(state(scenario)),"application/json");
    }
    if(url.pathname.startsWith("/assets/")){
      const name=url.pathname.slice("/assets/".length);
      if(!/^[a-z0-9-]+\.(js|css)$/.test(name))return send(403,"Denied","text/plain");
      const file=path.join(assets,name);
      if(!fs.existsSync(file))return send(404,"Missing","text/plain");
      return send(200,fs.readFileSync(file,"utf8"),name.endsWith(".css")?"text/css":"text/javascript");
    }
    return send(403,"Denied","text/plain");
  });
  await new Promise((resolve,reject)=>{
    server.once("error",reject);server.listen(0,"127.0.0.1",resolve);
  });
  try{
    for(const [width,height,scenario] of [
      [480,320,"default"],[800,480,"default"],[1280,720,"default"],
      [1920,1080,"default"],[480,320,"custom"],[1280,720,"custom"]
    ]){
      const r=await chromeRun(bin,server.address().port,width,height,scenario);
      console.log("Viewport "+width+"x"+height+" "+scenario+": "+JSON.stringify(r));
      assert.ok(!r.error,r.error);
      assert.equal(r.ok,true);
      assert.equal(r.width,width);assert.equal(r.height,height);
      assert.equal(r.documentFits,true,"Page scrollbar present on load");
      assert.equal(r.documentFitsAfter,true,"Page scrollbar present after navigation");
      assert.equal(r.initial.deckFits,true,"Module deck clipped");
      assert.equal(r.initial.cardsFit,true,"Module cards clipped");
      assert.equal(r.initial.rolesFit,true,"Job selector clipped");
      assert.equal(r.initial.pagerFits,true,"Pager clipped");
      assert.equal(r.initial.paneTabsVisible,true);
      assert.equal(r.rolePane,true,"Role pane clipped");
      assert.equal(r.viewport,true,"Viewport panel clipped");
      assert.equal(r.messages,true,"Messages panel clipped");
      if(scenario==="default"){
        assert.equal(r.inspector,true,"No inspector after module selection");
        assert.ok(r.nextModules?.count>0,"Module next page missing");
        assert.equal(r.nextModules?.fit,true,"Paged modules clipped");
      }else{
        assert.equal(r.palette?.visible,true,"Custom tool palette unavailable");
        assert.equal(r.palette?.fit,true,"Custom palette clipped");
        assert.ok(r.palette?.next>0,"Custom palette next page missing");
      }
    }
  }finally{await new Promise(resolve=>server.close(resolve));}
}
main().catch(e=>{console.error(e);process.exitCode=1;});
