"use strict";
// Exact-size Chromium frames, production handheld HTML/CSS and aid.js.
// Reuses only the process driver; all aid state and requests are local fixtures.
const assert=require("node:assert/strict"),fs=require("node:fs"),http=require("node:http"),path=require("node:path");
const {findChrome,chromeRun}=require("./test_dashboard_viewport.cjs");
const assets=path.resolve(__dirname,"../crates/pipboy/assets");
const fixture=()=>fs.readFileSync(path.join(assets,"micro.html"),"utf8")
  .replace(/<script\b[^>]*>[\s\S]*?<\/script>/g,"")
  .replace("</body>",'<script src="/fixture.js"></script><script src="/assets/aid.js"></script><script src="/probe.js"></script></body>');
const fixtureScript=`
window.fixture={state:{config:{doctor_peer:"doctor-one",emergency_peer:"doctor-one",location:"UNKNOWN",tier:"primary"},beacon:null},
 peers:["doctor-one"],records:[{id:1,peer:"medic-one",direction:"incoming",packet:{kind:"nine_line",mode:"peacetime",lines:Array(9).fill("UNKNOWN")}}],
 medical_allowed:true,storage:"plaintext_local_only",journal_available:true};
window.outbound=0;
window.fetch=async(url,options)=>{
 if(url!=="/api/aid" || options?.method==="POST"){window.outbound++;throw Error("Unexpected action in layout fixture");}
 return {ok:true,json:async()=>structuredClone(window.fixture)};
};`;
const probe=`
(async()=>{
 const checks=[];
 const wait=()=>new Promise(r=>setTimeout(r,60));
 const click=text=>{const b=[...document.querySelectorAll(".aid-dialog button")].find(b=>b.textContent===text);if(!b)throw Error("Missing "+text);b.click();};
 const check=name=>{
   const dialog=document.querySelector(".aid-dialog"),body=document.querySelector(".aid-body");
   const controls=[...dialog.querySelectorAll("button,input,select")].filter(e=>e.getClientRects().length);
   const d=dialog.getBoundingClientRect();
   const fits=d.top>=0 && d.bottom<=innerHeight+1 && d.left>=0 && d.right<=innerWidth+1 &&
      body.scrollHeight<=body.clientHeight+1 && body.scrollWidth<=body.clientWidth+1;
   checks.push({name,fits,scroll:body.scrollHeight,space:body.clientHeight,touch:controls.every(e=>e.getBoundingClientRect().height>=44)});
 };
 try{
   window.RustBoyAid.open("beacon");await wait();check("idle");
   window.fixture.state.beacon={id:1,active:true,peer:"doctor-one",location:"UNKNOWN / NOT PROVIDED"};
   window.RustBoyAid.open("beacon");await wait();check("active");
   window.fixture.state.beacon.active=false;
   window.RustBoyAid.open("beacon");await wait();check("cancelled");
   window.RustBoyAid.open("setup");await wait();check("destinations");click("NEXT ▶");await wait();check("location-route");
   window.RustBoyAid.open("nine");await wait();
   for(let i=0;i<9;i++){
     check("line-"+(i+1));const input=document.querySelector(".aid-body input");input.value="UNKNOWN";input.dispatchEvent(new Event("input"));click(i===8?"REVIEW ▶":"NEXT ▶");await wait();
   }
   check("review");
   window.RustBoyAid.open("inbox");await wait();check("doctor-inbox");
   parent.postMessage({type:"dashboard-viewport",value:{checks,outbound:window.outbound}},location.origin);
 }catch(e){parent.postMessage({type:"dashboard-viewport",value:{error:e.stack,checks}},location.origin);}
})();`;
function host(width,height){return `<!doctype html><style>body{margin:0}iframe{border:0}</style>
<iframe width="${width}" height="${height}" src="/fixture"></iframe><pre id="viewport-report" hidden></pre>
<script>window.addEventListener("message",e=>{if(e.origin===location.origin && e.data?.type==="dashboard-viewport")document.querySelector("pre").textContent=btoa(String.fromCharCode(...new TextEncoder().encode(JSON.stringify(e.data.value))));});</script>`;}
async function main(){
 const bin=findChrome();
 const server=http.createServer((req,res)=>{
   const url=new URL(req.url,"http://127.0.0.1");
   const send=(body,type="text/html")=>{res.writeHead(200,{"Content-Type":type});res.end(body);};
   if(req.method!=="GET"){res.writeHead(405);return res.end();}
   if(url.pathname==="/host")return send(host(Number(url.searchParams.get("w")),Number(url.searchParams.get("h"))));
   if(url.pathname==="/fixture")return send(fixture());
   if(url.pathname==="/fixture.js")return send(fixtureScript,"text/javascript");
   if(url.pathname==="/probe.js")return send(probe,"text/javascript");
   if(/^\/assets\/[a-z-]+\.(js|css)$/.test(url.pathname)){
     const file=path.join(assets,path.basename(url.pathname));
     if(fs.existsSync(file))return send(fs.readFileSync(file),file.endsWith(".css")?"text/css":"text/javascript");
   }
   res.writeHead(404);res.end();
 });
 await new Promise(resolve=>server.listen(0,"127.0.0.1",resolve));
 try{
   for(const [width,height] of [[480,320],[800,480],[1280,720]]){
     const result=await chromeRun(bin,server.address().port,width,height,"aid");
     assert.ok(!result.error,result.error);assert.equal(result.outbound,0);
     for(const check of result.checks){assert.equal(check.fits,true,JSON.stringify({width,height,...check}));assert.equal(check.touch,true,check.name);}
     console.log("PASS aid Chromium "+width+"x"+height+": "+result.checks.length+" screens fit; 44px controls; no outbound actions");
   }
 }finally{await new Promise(resolve=>server.close(resolve));}
}
main().catch(error=>{console.error(error);process.exitCode=1;});
