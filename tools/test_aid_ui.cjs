"use strict";
// Production UI, deterministic DOM/timers and invented responses. No network.
const assert=require("node:assert/strict"),fs=require("node:fs"),path=require("node:path"),vm=require("node:vm");
const walk=node=>[node,...node.children.flatMap(walk)];
class Element {
  constructor(tag){this.tagName=tag;this.children=[];this.className="";this.text="";this.events=new Map();this.attributes={};this.dataset={};this.disabled=false;}
  append(...nodes){this.children.push(...nodes);}
  replaceChildren(...nodes){this.children=nodes;this.text="";}
  set textContent(value){this.text=String(value);this.children=[];}
  get textContent(){return this.text+this.children.map(x=>x.textContent).join(" ");}
  setAttribute(key,value){this.attributes[key]=String(value);}
  removeAttribute(key){delete this.attributes[key];}
  addEventListener(key,fn){this.events.set(key,fn);}
  dispatch(key,props={}){return this.events.get(key)?.({button:0,pointerId:1,preventDefault(){},...props});}
  click(){if(!this.disabled)this.dispatch("click");}
  showModal(){this.open=true;}
  close(){this.open=false;this.dispatch("close");}
  focus(){}
  remove(){}
  querySelectorAll(selector){
    const all=walk(this).slice(1);
    if(selector.startsWith("."))return all.filter(x=>x.className.split(" ").includes(selector.slice(1)));
    const tags=selector.split(",");return all.filter(x=>tags.includes(x.tagName));
  }
  querySelector(selector){return this.querySelectorAll(selector)[0] || null;}
  get classList(){return {toggle:(key,on)=>{this.className=this.className.split(" ").filter(x=>x!==key).concat(on?[key]:[]).join(" ");}};}
}
const trigger=new Element("button");
const document={body:new Element("body"),hidden:false,events:new Map(),createElement:tag=>new Element(tag),getElementById:()=>trigger,addEventListener(key,fn){this.events.set(key,fn);}};
const window={events:new Map(),addEventListener(key,fn){this.events.set(key,fn);}};
const timers=new Map(),intervals=[],requests=[];
let serial=0,fixture={state:{config:{doctor_peer:"doctor-one",emergency_peer:"doctor-one",location:"Fictional training site",tier:"primary"},beacon:null},peers:["doctor-one"],records:[],medical_allowed:true,storage:"plaintext_local_only",journal_available:true};
const context=vm.createContext({document,window,console,TextEncoder,
  setTimeout(fn){const id=++serial;timers.set(id,fn);return id;},clearTimeout(id){timers.delete(id);},
  setInterval(fn){intervals.push(fn);return 1;},clearInterval(){},
  fetch:async (url,options)=>{
    const body=options.method==="POST"?JSON.parse(options.body):null;requests.push({url,body});
    let result=structuredClone(fixture);
    if(url==="/api/aid/activate"){
      fixture.state.beacon={id:1,active:true,peer:"doctor-one",location:"Fictional training site"};
      result={beacon:fixture.state.beacon,delivery:{status:"queued",stored_on_receiver:false}};
    }
    if(url==="/api/aid/cancel"){fixture.state.beacon.active=false;result={beacon:fixture.state.beacon,delivery:{status:"queued",stored_on_receiver:false}};}
    return {ok:true,json:async()=>structuredClone(result)};
  }
});
vm.runInContext(fs.readFileSync(path.resolve(__dirname,"../crates/pipboy/assets/aid.js"),"utf8"),context);
const flush=()=>new Promise(setImmediate);
const find=(text)=>{const node=walk(document.body).find(n=>n.tagName==="button" && n.textContent===text);assert.ok(node,"Missing button "+text);return node;};
const posts=()=>requests.filter(x=>x.body);
const fireTimers=()=>{for(const [id,fn] of [...timers]){timers.delete(id);fn();}};
(async()=>{
  await flush();assert.equal(posts().length,0);
  trigger.click();await flush();
  let hold=find("HOLD 2s FOR SOS");
  hold.dispatch("pointerdown");hold.dispatch("pointerup");
  fireTimers();await flush();assert.equal(posts().length,0,"Short press must not transmit");
  hold.click();find("BACK").click();assert.equal(posts().length,0,"Tap requires separate confirmation");
  hold=find("HOLD 2s FOR SOS");hold.dispatch("pointerdown");hold.dispatch("pointercancel");fireTimers();await flush();assert.equal(posts().length,0);
  hold.dispatch("keydown",{key:"Enter",repeat:false});hold.dispatch("keydown",{key:"Enter",repeat:true});
  assert.equal(timers.size,1);window.events.get("blur")();fireTimers();await flush();assert.equal(posts().length,0);
  hold.dispatch("pointerdown");document.hidden=true;document.events.get("visibilitychange")();fireTimers();await flush();assert.equal(posts().length,0);
  document.hidden=false;
  hold.dispatch("pointerdown");fireTimers();await flush();
  assert.equal(posts().length,1);assert.equal(posts()[0].url,"/api/aid/activate");
  assert.equal(posts()[0].body.confirm,true);assert.equal(trigger.textContent,"SOS WAIT");
  assert.ok(document.body.textContent.includes("NOT CONFIRMED"),"No false delivery success");
  const cancel=find("HOLD 3s TO CANCEL");cancel.click();assert.equal(posts().length,1);
  find("CONFIRM").click();await flush();assert.equal(posts()[1].url,"/api/aid/cancel");
  assert.equal(trigger.textContent,"CANCEL ?","Unconfirmed cancellation stays visible");
  fixture.beacon_delivery="stored";
  // Reordered/repeated alarms cannot revive a cancelled incident in the banner.
  fixture.records=[{peer:"medic-other",direction:"incoming",packet:{kind:"alarm",incident:2}},
    {peer:"medic-other",direction:"incoming",packet:{kind:"cancel",incident:2}}];
  intervals[0]();await flush();assert.equal(trigger.textContent,"SOS");
  const before=posts().length;intervals[0]();await flush();assert.equal(posts().length,before,"Local refresh must not send");
  window.RustBoyAid.open("nine");await flush();
  for(let i=0;i<9;i++){
    const input=walk(document.body).find(n=>n.tagName==="input");
    assert.ok(input);input.value="UNKNOWN";input.dispatch("input");
    find(i===8?"REVIEW ▶":"NEXT ▶").click();
  }
  find("SEND 9 LINE").click();assert.equal(posts().length,before);
  find("CONFIRM").click();await flush();
  const report=posts().at(-1);assert.equal(report.url,"/api/aid/nine-line");
  assert.equal(report.body.packet.lines.length,9);assert.equal(report.body.packet.mode,"peacetime");
  assert.ok(report.body.packet.lines.every(x=>x==="UNKNOWN"));
  trigger.dispatch("pointerdown");fireTimers();await flush();
  assert.equal(posts().at(-1).url,"/api/aid/activate","Dedicated header hold activates directly");
  window.RustBoyAid.destroy();
  console.log("PASS aid UI: hold/release/cancel/blur/visibility, explicit tap confirmation, local-only reads, truthful status and nine-line wizard");
})().catch(error=>{console.error(error);process.exitCode=1;});
