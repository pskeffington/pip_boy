"use strict";

// Real headless Chromium layout acceptance. No live scan or controller access.
// Uses only Node built-ins and production micro.css/network.css/network.js.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");
const { spawn, spawnSync } = require("node:child_process");

const root = path.resolve(__dirname, "..");
const fixture = path.join(__dirname, "fixtures", "network-viewport.html");
const allowed = new Map([
  ["/fixture", [fixture, "text/html; charset=utf-8"]],
  ["/assets/micro.css", [path.join(root, "crates/pipboy/assets/micro.css"), "text/css; charset=utf-8"]],
  ["/assets/network.css", [path.join(root, "crates/pipboy/assets/network.css"), "text/css; charset=utf-8"]],
  ["/assets/network.js", [path.join(root, "crates/pipboy/assets/network.js"), "text/javascript; charset=utf-8"]]
]);
const chromeCandidates = [
  process.env.CHROME_BIN, "/usr/bin/google-chrome", "/usr/bin/chromium",
  "google-chrome", "chromium", "chromium-browser"
].filter(Boolean);

function findChrome() {
  for (const name of chromeCandidates) {
    if (path.isAbsolute(name) && fs.existsSync(name)) return name;
    const test = spawnSync("which", [name], { encoding:"utf8" });
    if (test.status === 0 && test.stdout.trim()) return test.stdout.trim().split("\n")[0];
  }
  throw new Error("Headless Chrome/Chromium not installed; set CHROME_BIN");
}

// Headless Chrome enforces a >=500px top-level window width. An iframe supplies
// exact inner CSS viewports (including 480x320 and 320px portrait media queries).
function hostHtml(width,height,view) {
  const script = [
    'const frame=document.querySelector("#handheld");',
    'const output=document.getElementById("viewport-report");',
    'const encode=value=>btoa(String.fromCharCode(...new TextEncoder().encode(JSON.stringify(value))));',
    'let checks=0;',
    'function copy(){',
    'const doc=frame.contentDocument;',
    'const result=doc?.getElementById("viewport-report")?.textContent;',
    'if(result){output.textContent=result;return;}',
    'if(++checks>=40){',
    'output.textContent=encode({error:"Iframe report unavailable; state="+doc?.readyState+',
    '"; network="+doc?.querySelectorAll(".net-tile").length+',
    '"; body="+doc?.body?.innerText?.slice(0,240)});return;}',
    'setTimeout(copy,20);',
    '}',
    'window.addEventListener("message",event=>{',
    'if(event.origin===location.origin&&event.source===frame.contentWindow&&',
    'event.data?.type==="rustboy-layout-report")output.textContent=encode(event.data.value);',
    '});',
    'frame.addEventListener("load",copy);copy();'
  ].join("");
  return '<!doctype html><html lang="en"><head><meta charset="utf-8">' +
    '<title>Rust-Boy viewport host</title>' +
    '<style>html,body{margin:0}iframe{border:0;display:block}</style></head><body>' +
    '<iframe id="handheld" title="Handheld preview" width="' + width +
    '" height="' + height + '" style="width:' + width + 'px;height:' + height +
    'px" src="/fixture?view=' + view + '"></iframe>' +
    '<pre id="viewport-report" hidden></pre><script>' + script + '</script></body></html>';
}

function render(chrome, port, width, height, view) {
  return new Promise((resolve,reject) => {
    const profile = fs.mkdtempSync(path.join(os.tmpdir(), "rustboy-viewport-"));
    const url = "http://127.0.0.1:" + port + "/host?width=" + width + "&height=" + height + "&view=" + view;
    const args = [
      "--headless", "--no-sandbox", "--disable-gpu", "--disable-dev-shm-usage",
      "--disable-background-networking", "--no-first-run", "--no-default-browser-check",
      "--hide-scrollbars", "--force-device-scale-factor=1",
      "--window-size=1000,900",
      "--virtual-time-budget=2500", "--dump-dom",
      "--user-data-dir=" + profile, url
    ];
    const proc = spawn(chrome,args,{stdio:["ignore","pipe","pipe"]});
    let stdout = "", stderr = "", done = false;
    const timeout = setTimeout(() => {
      proc.kill("SIGKILL");
      fail(new Error("Chromium timed out for " + width + "x" + height + "/" + view));
    },30000);
    const cleanup = () => { clearTimeout(timeout); fs.rmSync(profile,{recursive:true,force:true}); };
    const fail = (e) => {if(done) return;done=true;cleanup();reject(e);};
    proc.stdout.on("data",chunk=>{stdout += chunk.toString();if(stdout.length>10_000_000) proc.kill("SIGKILL");});
    proc.stderr.on("data",chunk=>{stderr += chunk.toString();if(stderr.length>12_000)stderr=stderr.slice(-12_000);});
    proc.on("error",fail);
    proc.on("close",code=>{
      if(done)return;
      done=true;cleanup();
      if(code!==0) return reject(new Error("Chromium exited "+code+": "+stderr.slice(-2500)));
      const match = stdout.match(/<pre id="viewport-report"[^>]*>([A-Za-z0-9+/=]+)<\/pre>/);
      if(!match) return reject(new Error("No layout report from Chromium for "+url+"\n"+stdout.slice(-1200)));
      try{resolve(JSON.parse(Buffer.from(match[1],"base64").toString("utf8")));}
      catch(e){reject(new Error("Invalid Chromium report: "+e.message));}
    });
  });
}

function check(data,width,height,view) {
  console.log("Viewport evidence "+width+"x"+height+" "+view+": "+JSON.stringify(data));
  assert.ok(!data.error, data.error || "");
  assert.equal(data.view,view);
  assert.equal(data.window.width,width,"Chromium did not honor requested viewport width");
  assert.equal(data.window.height,height,"Chromium did not honor requested viewport height");
  assert.ok(data.viewport && data.device && data.pagerCount===2,"Required shell/pager missing");
  assert.ok(data.pagerSizes.every(x=>x.width>=84 && x.height>=48),
    "PREV/NEXT touch hit targets below 84x48 px");
  assert.equal(data.pagerVisibleWhenNavigated,true,
    "Page controls cannot be brought completely into the viewport");
  assert.equal(data.requestsWereOnlyExplicit,true,"Unexpected fixture request");
  if(view==="menu"){
    assert.equal(data.mockRequests,0,"Opening the Network Engineer triggered network I/O");
    assert.equal(data.menuTileCount,4,"Expected 2x2 handheld tool deck");
    assert.equal(data.allTilesVisible,true,"A handheld tool tile is clipped");
    assert.equal(data.footerVisibleInitially,true,"Tool-page controls invisible on load");
    if(width===480 && height===320)
      assert.ok(data.scrollHeight<=data.clientHeight+1,
        "480x320 primary tool menu requires scrolling: "+data.scrollHeight+"/"+data.clientHeight);
  }else{
    assert.equal(data.mockRequests,1,"Expected one explicitly authorized synthetic request");
    assert.equal(data.stopVisibleInitially,true,
      "STOP REFRESH must be visible without scrolling on handheld results");
  }
  console.log("PASS real Chromium "+width+"x"+height+" "+view+": "+
    (view==="menu" ? "four visible tiles" : "visible STOP REFRESH")+
    ", 48px pager, requests="+data.mockRequests);
}

async function main() {
  const chrome=findChrome();
  const server=http.createServer((request,response)=>{
    const url=new URL(request.url,"http://localhost");
    if(request.method==="GET" && url.pathname==="/host"){
      const width=Number(url.searchParams.get("width"));
      const height=Number(url.searchParams.get("height"));
      const view=url.searchParams.get("view");
      if(!Number.isInteger(width)||!Number.isInteger(height)||
          width<300||width>900||height<300||height>700||
          !["menu","result"].includes(view)){
        response.writeHead(400);response.end("Invalid viewport fixture");return;
      }
      response.writeHead(200,{"Content-Type":"text/html; charset=utf-8","Cache-Control":"no-store",
        "Content-Security-Policy":"default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; frame-src 'self'"});
      response.end(hostHtml(width,height,view));
      return;
    }
    const file=allowed.get(url.pathname);
    if(request.method!=="GET"||!file){
      response.writeHead(403,{"Content-Type":"text/plain"});response.end("Fixture denied");return;
    }
    response.writeHead(200,{"Content-Type":file[1],"Cache-Control":"no-store",
      "Content-Security-Policy":"default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self'"});
    fs.createReadStream(file[0]).pipe(response);
  });
  await new Promise((resolve,reject)=>{
    server.once("error",reject);server.listen(0,"127.0.0.1",resolve);
  });
  try {
    const port=server.address().port;
    // Compact landscape is the release target; portrait and wider shells guard regressions.
    for(const [width,height,view] of [
      [480,320,"menu"],[480,320,"result"],
      [800,480,"menu"],[320,568,"menu"]
    ]) check(await render(chrome,port,width,height,view),width,height,view);
  } finally {
    await new Promise(resolve=>server.close(resolve));
  }
}
main().catch(e=>{console.error(e);process.exitCode=1;});
