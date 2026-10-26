"use strict";

/**
 * Production desktop HTML/CSS/JS layout test in offline headless Chromium.
 * The HTTP fixture only serves repository assets and a synthetic /api/state.
 * No private data, real services, hardware, LAN scans or external providers.
 */
const assert = require("node:assert/strict");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");
const {spawn, spawnSync} = require("node:child_process");

const root = path.resolve(__dirname, "..");
const assets = path.join(root, "crates", "pipboy", "assets");
const styles = ["dashboard.css", "map.css", "tasks.css",
  "messenger.css", "network.css", "reclamation.css"];
const allowed = new Map(styles.map(name => ["/assets/" + name,
  [path.join(assets, name), "text/css; charset=utf-8"]]));
for (const name of ["dashboard.js", "messenger.js"]) {
  allowed.set("/assets/" + name,
    [path.join(assets, name), "text/javascript; charset=utf-8"]);
}
const profiles = [
  {id:"overseer", title:"Overseer Station",
    description:"Manage assignments and workbench operations.", tools:["host","diag"]},
  {id:"head-engineer", title:"Head Engineer Station",
    description:"System checks, network engineering and local diagnostics.", tools:["host","net-lab","byte-lab","diag","simulator"]},
  {id:"security-chief", title:"Security Chief Station",
    description:"Defensive audit and scoped triage.", tools:["host"]},
  {id:"vault-doctor", title:"Vault Doctor Station",
    description:"Local reference and equipment review.", tools:["host"]},
  {id:"scout", title:"Scout Station",
    description:"GIS reference and navigation.", tools:["host"]},
  {id:"reclamation-scout", title:"Reclamation Scout",
    description:"Offline site-record survey.", tools:["host"]},
  {id:"quartermaster", title:"Quartermaster Station",
    description:"Equipment and local inventory.", tools:["host"]},
  {id:"custom", title:"Custom",
    description:"Select only the tools you need.", tools:[]}
];
const tools = [
  {id:"host", name:"HOST STAT", availability:"ready", command:"pipboy host",
    description:"Read-only local CPU and memory status."},
  {id:"net-lab", name:"NET ENGINEER", availability:"ready", command:"pipboy workbench net",
    description:"Local network calculation and opt-in bounded private-LAN checks."},
  {id:"byte-lab", name:"BYTE LAB", availability:"ready", command:"pipboy bytes",
    description:"Byte display and hex analysis."},
  {id:"diag", name:"DIAGNOSTICS", availability:"ready", command:"pipboy diag",
    description:"Local diagnostics and manual ingestion."},
  {id:"simulator", name:"SIMULATOR", availability:"simulated", command:"pipboy simulate",
    description:"Synthetic fixtures, no physical hardware."}
];
const state = {
  profiles, palette:tools, net_operations:[],
  active:{profile:"head-engineer",slots:profiles[1].tools.map((tool,index)=>({id:index+1,tool}))}
};

function chromeBinary() {
  const names = [process.env.CHROME_BIN,
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "/usr/bin/google-chrome", "/usr/bin/chromium",
    "google-chrome", "chromium", "chromium-browser"].filter(Boolean);
  for (const name of names) {
    if (path.isAbsolute(name) && fs.existsSync(name)) return name;
    const result = spawnSync("which", [name], {encoding:"utf8"});
    if (result.status === 0 && result.stdout.trim())
      return result.stdout.trim().split("\n")[0];
  }
  throw Error("Chromium unavailable; set CHROME_BIN");
}

function fixturePage() {
  let html = fs.readFileSync(path.join(assets, "dashboard.html"), "utf8");
  // Only load the desktop controller and Messenger widget: other independent
  // subsystems run their own separate component and viewport acceptance tests.
  html = html.replace(/<script defer src="(\/assets\/[^"]+)"><\/script>/g,
    (full,url) => ["/assets/dashboard.js","/assets/messenger.js"].includes(url) ? full : "");
  const mock = '<script>window.__fixtureRequests=[];' +
    'window.fetch=function(url,opts){' +
    'const method=opts&&opts.method||"GET";' +
    'window.__fixtureRequests.push({url:String(url),method});' +
    'if(url==="/api/state"&&method==="GET")return Promise.resolve({' +
    'ok:true,json:()=>Promise.resolve(' + JSON.stringify(state) + ')});' +
    'return Promise.resolve({ok:false,json:()=>Promise.resolve({error:"Fixture denies remote actions"})});' +
    '};</script>';
  html = html.replace("</head>", mock + "</head>");
  const reporter = [
    '<pre id="viewport-report" hidden></pre><script>',
    'window.addEventListener("load",function(){setTimeout(function(){',
    'try{',
    'const rect=selector=>{const r=document.querySelector(selector).getBoundingClientRect();',
    'return {x:r.x,y:r.y,w:r.width,h:r.height,right:r.right,bottom:r.bottom};};',
    'const roles=[...document.querySelectorAll(".role-card")];',
    'const modules=[...document.querySelectorAll(".module-card")];',
    'const role=document.querySelector(".role-panel");',
    'const main=document.querySelector(".main-panel");',
    'const detail=document.querySelector(".detail-panel");',
    'const report={width:innerWidth,height:innerHeight,',
    'scrollWidth:document.documentElement.scrollWidth,',
    'scrollHeight:document.documentElement.scrollHeight,',
    'role:rect(".role-panel"),main:rect(".main-panel"),',
    'detail:rect(".detail-panel"),modules:rect(".module-grid"),',
    'stage:rect("#workspace-stage"),',
    'roleVisible:getComputedStyle(role).display!=="none",',
    'mainVisible:getComputedStyle(main).display!=="none",',
    'detailVisible:getComputedStyle(detail).display!=="none",',
    'roles:roles.length,tools:modules.length,',
    'roleHeights:roles.map(x=>x.getBoundingClientRect().height),',
    'toolHeights:modules.map(x=>x.getBoundingClientRect().height),',
    'collapsedRole:!document.getElementById("role-explanation").open,',
    'collapsedDetail:!document.getElementById("detail-info").open,',
    'messagesInitiallyHidden:document.querySelector(\'[data-stage="messages"]\').hidden,',
    'messageChildrenInitially:document.getElementById("messenger-dock").children.length,',
    'mainRole:document.getElementById("workspace-title").textContent,',
    'toolCount:document.getElementById("module-count").textContent,',
    'initialRequests:[...window.__fixtureRequests],',
    'paneTabs:document.querySelectorAll("[data-pane-target]").length,',
    'stageTabs:document.querySelectorAll("[data-stage-target]").length,',
    'rolePager:document.getElementById("role-pager").textContent,',
    'activeStage:document.querySelector(\'[data-stage="modules"]\').hidden===false};',
    'document.querySelector(\'[data-stage-target="messages"]\').click();',
    'setTimeout(function(){',
    'report.messagesAfterClick=!document.querySelector(\'[data-stage="messages"]\').hidden;',
    'report.messageChildrenAfterClick=document.getElementById("messenger-dock").children.length;',
    'report.afterRequests=[...window.__fixtureRequests];',
    'document.getElementById("viewport-report").textContent=',
    'btoa(String.fromCharCode(...new TextEncoder().encode(JSON.stringify(report))));',
    '},140);',
    '}catch(e){document.getElementById("viewport-report").textContent=',
    'btoa(JSON.stringify({error:String(e)}));}',
    '},350);});</script>'
  ].join("");
  return html.replace("</body>", reporter + "</body>");
}

function hostPage(width,height) {
  const inline = [
    'const frame=document.querySelector("#test-frame");',
    'const output=document.getElementById("viewport-report");',
    'let checks=0;',
    'function copy(){const doc=frame.contentDocument;',
    'const result=doc&&doc.getElementById("viewport-report")?.textContent;',
    'if(result){output.textContent=result;return;}',
    'if(++checks>90){output.textContent=btoa(JSON.stringify({error:"No iframe layout report"}));return;}',
    'setTimeout(copy,30);}',
    'frame.addEventListener("load",copy);copy();'
  ].join("");
  return '<!doctype html><html><head><meta charset="utf-8">' +
    '<style>html,body{margin:0;background:#111}iframe{border:0;display:block}</style>' +
    '</head><body><iframe id="test-frame" title="Desktop fixture" style="width:' +
    width + 'px;height:' + height + 'px" src="/fixture"></iframe>' +
    '<pre hidden id="viewport-report"></pre><script>' + inline + '</script></body></html>';
}

function render(chrome,port,width,height) {
  return new Promise((resolve,reject)=>{
    const dir=fs.mkdtempSync(path.join(os.tmpdir(),"rustboy-desktop-"));
    const url="http://127.0.0.1:"+port+"/host?w="+width+"&h="+height;
    const args=["--headless","--no-sandbox","--disable-gpu","--disable-dev-shm-usage",
      "--disable-background-networking","--no-first-run","--no-default-browser-check",
      "--force-device-scale-factor=1","--window-size=1680,1080",
      "--virtual-time-budget=4000","--dump-dom",
      "--user-data-dir="+dir,url];
    const proc=spawn(chrome,args,{stdio:["ignore","pipe","pipe"]});
    let stdout="",stderr="",done=false;
    const timer=setTimeout(()=>{
      proc.kill("SIGKILL");
      fail(Error("Chromium timeout at "+width+"x"+height));
    },35000);
    function cleanup(){clearTimeout(timer);fs.rmSync(dir,{recursive:true,force:true});}
    function fail(error){if(done)return;done=true;cleanup();reject(error);}
    proc.stdout.on("data",data=>{stdout+=data.toString();
      if(stdout.length>8_000_000)proc.kill("SIGKILL");});
    proc.stderr.on("data",data=>{stderr+=data.toString();
      if(stderr.length>12_000)stderr=stderr.slice(-12_000);});
    proc.on("error",fail);
    proc.on("close",code=>{
      if(done)return;done=true;cleanup();
      if(code!==0)return reject(Error("Chromium exited "+code+": "+stderr.slice(-1200)));
      const match=stdout.match(/<pre hidden="" id="viewport-report">([A-Za-z0-9+/=]+)<\/pre>/) ||
        stdout.match(/<pre id="viewport-report" hidden="">([A-Za-z0-9+/=]+)<\/pre>/) ||
        stdout.match(/<pre hidden id="viewport-report">([A-Za-z0-9+/=]+)<\/pre>/);
      if(!match)return reject(Error("Missing viewport report: "+stdout.slice(-1300)));
      try{resolve(JSON.parse(Buffer.from(match[1],"base64").toString("utf8")));}
      catch(error){reject(Error("Invalid viewport report: "+error.message));}
    });
  });
}

function check(data,width,height) {
  console.log("Desktop " + width + "x" + height + ": " + JSON.stringify(data));
  assert.ok(!data.error,data.error || "");
  assert.equal(data.width,width,"Chromium iframe viewport width");
  assert.equal(data.height,height,"Chromium iframe viewport height");
  assert.ok(data.scrollWidth<=width+2,"Unwanted horizontal overflow");
  assert.ok(data.scrollHeight<=height+2,"One-screen workspace requires no document scrolling");
  assert.equal(data.paneTabs,3,"Focused STATIONS/WORKSPACE/INSPECTOR navigation missing");
  assert.equal(data.stageTabs,4,"TOOLS/VIEW/MESSAGES/ADD TOOLS pages missing");
  assert.equal(data.roles,6,"First page of 8 station templates should contain 6 stations");
  assert.match(data.rolePager,/1\s*\/\s*2/,"Station pagination unavailable");
  assert.equal(data.tools,5,"Five head-engineer tool fixtures missing");
  assert.equal(data.toolCount,"5","Tools count must reflect Rust snapshot");
  assert.equal(data.mainRole,"Head Engineer Station");
  assert.equal(data.mainVisible,true,"Workspace must start active");
  assert.ok(data.activeStage,"Tools stage must be visible on initial load");
  assert.ok(data.collapsedRole && data.collapsedDetail,
    "Explanations must stay collapsed until requested");
  assert.ok(data.messagesInitiallyHidden && data.messageChildrenInitially===0,
    "Messenger must not mount or be visible when first opening desktop");
  assert.deepEqual(data.initialRequests,[{url:"/api/state",method:"GET"}],
    "Initial page made unexpected network requests");
  assert.ok(data.messagesAfterClick && data.messageChildrenAfterClick>0,
    "MESSAGES selection failed to mount the local Messenger widget");
  assert.ok(data.afterRequests.some(x=>x.url==="/api/messages"&&x.method==="GET"),
    "Manually selecting Messages must load local journal status");
  assert.ok(data.afterRequests.every(x=>x.method==="GET"),
    "Changing pages must never issue POSTs, scans or messages");
  assert.ok(data.stage.y>=data.main.y && data.stage.y<height-60,
    "Primary stage navigation must be visible near first screen");
  assert.ok(data.toolHeights.every(h=>h>=44),
    "Visible tool controls must be at least 44px tall");
  if(width>1100){
    assert.ok(data.roleVisible && data.detailVisible,"Wide layout requires three panes");
    assert.ok(data.roleHeights.every(h=>h>=44),
      "Visible station targets must be at least 44px tall");
    for(const pane of [data.role,data.main,data.detail])
      assert.ok(pane.w>135 && pane.x>=-2 && pane.right<=width+2,
        "Pane clipped horizontally: "+JSON.stringify(pane));
    assert.ok(data.role.right<=data.main.x+2 && data.main.right<=data.detail.x+2,
      "Desktop panes overlap");
  }else{
    assert.ok(!data.roleVisible && !data.detailVisible,
      "Narrow layouts should show focused workspace pane only");
    assert.ok(data.main.w>width*.75 && data.main.right<=width+2,
      "Narrow workspace must span available viewport");
  }
}

async function main(){
  const chrome=chromeBinary();
  const server=http.createServer((req,res)=>{
    const url=new URL(req.url,"http://localhost");
    if(req.method==="GET"&&url.pathname==="/host"){
      const width=Number(url.searchParams.get("w"));
      const height=Number(url.searchParams.get("h"));
      if(!Number.isInteger(width)||!Number.isInteger(height)||
        width<600||width>1600||height<560||height>1080){
        res.writeHead(400);res.end("Invalid desktop viewport");return;
      }
      res.writeHead(200,{"Content-Type":"text/html; charset=utf-8",
        "Content-Security-Policy":"default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; frame-src 'self'",
        "Cache-Control":"no-store"});
      res.end(hostPage(width,height));return;
    }
    if(req.method==="GET"&&url.pathname==="/fixture"){
      res.writeHead(200,{"Content-Type":"text/html; charset=utf-8",
        "Content-Security-Policy":"default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self'",
        "Cache-Control":"no-store"});
      res.end(fixturePage());return;
    }
    const file=allowed.get(url.pathname);
    if(req.method!=="GET"||!file){
      res.writeHead(403,{"Content-Type":"text/plain"});res.end("Fixture refused");return;
    }
    res.writeHead(200,{"Content-Type":file[1],
      "Cache-Control":"no-store",
      "Content-Security-Policy":"default-src 'self'"});
    fs.createReadStream(file[0]).pipe(res);
  });
  await new Promise((resolve,reject)=>{
    server.once("error",reject);server.listen(0,"127.0.0.1",resolve);
  });
  try{
    const port=server.address().port;
    for(const [width,height] of [[1440,900],[1280,800],[1024,768],[800,600]]){
      check(await render(chrome,port,width,height),width,height);
      console.log("PASS desktop "+width+"x"+height+": compact tools, default disclosures, no auto I/O");
    }
  }finally{
    await new Promise(resolve=>server.close(resolve));
  }
}
main().catch(error=>{console.error(error);process.exitCode=1;});
