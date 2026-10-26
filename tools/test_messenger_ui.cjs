"use strict";
const assert=require("node:assert/strict"),fs=require("node:fs"),path=require("node:path");
class Element{
  constructor(tag){
    this.tag=tag;this.children=[];this.listeners={};this.attrs={};
    this.className="";this.textContent="";this.value="";this.hidden=false;
    this.maxLength=0;this.rows=0;this.disabled=false;this.isConnected=true;
  }
  replaceChildren(...xs){this.children=xs}
  append(...xs){this.children.push(...xs)}
  addEventListener(event,fn){(this.listeners[event] ||= []).push(fn)}
  setAttribute(k,v){this.attrs[k]=v}
}
const document={createElement:tag=>new Element(tag)};
const window={};
let state={schema_version:1,device_id:"head-engineer",peers:["scout-one"],
  conversations:["scout-one"],transport:"pinned_openssh_explicit",storage:"plaintext_local_only",e2ee:false,
  messages:[]};
const calls=[];
let failSend=false,failRetry=false;
function pace(){return [{peer:"scout-one",lanes:[
  {tier:"primary",selectable:true},{tier:"alternate",selectable:true},
  {tier:"contingency",selectable:false},{tier:"emergency",selectable:true}]}]}
state.pace=pace();
async function fakeFetch(url,options={}){
  calls.push({url,options});
  if(url==="/api/messages")
    return {ok:true,json:async()=>structuredClone(state)};
  if(url==="/api/messages/send"){
    const frame=JSON.parse(options.body);
    assert.equal(frame.confirm,true);
    assert.equal(frame.alias,"scout-one");
    const id=state.messages.length+1;
    state.messages.push({id,peer:"scout-one",body:frame.body,
      direction:"outgoing",delivery:failSend||frame.tier==="emergency"?"queued":"stored",
      created_at:10,remote_origin_id:null,remote_stored_id:failSend?null:id,
      progress:{attempts:frame.tier==="emergency"?0:1,held_locally:frame.tier==="emergency",
        last_attempt:frame.tier==="emergency"?null:{route:{tier:frame.tier},started_at:10,
          outcome:failSend?"exchange_unconfirmed":"stored"}}});
    return {ok:true,json:async()=>({
      stored_on_receiver:!failSend&&frame.tier!=="emergency",held_locally:frame.tier==="emergency",message:state.messages.at(-1),
      read_by_human:false
    })};
  }
  if(url==="/api/messages/retry"){
    const frame=JSON.parse(options.body);
    const m=state.messages.find(x=>x.id===frame.id);
    assert.ok(m&&m.delivery==="queued");
    assert.ok(["primary","alternate","emergency"].includes(frame.tier));
    if(frame.tier==="emergency")m.progress.held_locally=true;
    else{
      m.progress.attempts++;m.progress.held_locally=false;
      m.progress.last_attempt={route:{tier:frame.tier},started_at:11,
        outcome:failRetry?"receipt_rejected":"stored"};
    }
    if(failRetry)return {ok:false,json:async()=>({error:"retry unconfirmed"})};
    m.delivery=frame.tier==="emergency"?"queued":"stored";m.remote_stored_id=frame.tier==="emergency"?null:90;
    return {ok:true,json:async()=>({stored_on_receiver:frame.tier!=="emergency",held_locally:frame.tier==="emergency",message:m})};
  }
  throw new Error("Unexpected Messenger fetch: "+url);
}
const source=fs.readFileSync(path.join(__dirname,"../crates/pipboy/assets/messenger.js"),"utf8");
new Function("window","document","fetch","TextEncoder",source)(window,document,fakeFetch,TextEncoder);
async function tick(){
  for(let i=0;i<8;i++)await new Promise(resolve=>setImmediate(resolve));
}
const walk=(node)=>[node,...(node.children||[]).flatMap(walk)];
const find=(node,pred)=>walk(node).find(pred);
const press=(node,kind="click",event)=>{assert.ok(node?.listeners?.[kind],"missing action");
  node.listeners[kind][0](event||{preventDefault(){}})};
async function main(){
  const root=new Element("section");
  const app=window.RustBoyMessenger.mount(root,{compact:true});
  await tick();
  assert.equal(calls.length,1,"mount only reads local data");
  assert.equal(calls[0].url,"/api/messages");
  assert.ok(!calls.some(x=>x.url.includes("send")));
  assert.ok(walk(root).some(x=>x.textContent.includes("NOT E2EE")));
  const input=find(root,x=>x.tag==="textarea"),form=find(root,x=>x.tag==="form");
  assert.ok(input&&form);
  input.value="x".repeat(1025);
  press(form,"submit");await tick();
  assert.equal(calls.length,1,"no over-limit POST");
  input.value="<img src=x onerror=alert(1)> test";
  press(form,"submit");await tick();
  assert.equal(calls.filter(x=>x.url==="/api/messages/send").length,1);
  assert.equal(input.value,"");
  assert.equal(state.messages[0].body,"<img src=x onerror=alert(1)> test");
  assert.ok(!("innerHTML" in new Element("p")));
  assert.ok(walk(root).some(x=>x.textContent===state.messages[0].body),
    "untrusted text displayed literally");
  failSend=true;
  input.value="Offline delivery";
  press(form,"submit");await tick();
  assert.equal(state.messages[1].delivery,"queued");
  assert.ok(walk(root).some(x=>x.textContent.includes("QUEUED LOCALLY")));
  assert.ok(walk(root).some(x=>x.textContent.includes("EXCHANGE UNCONFIRMED / DELIVERY UNKNOWN · 1 TRY · LAST P")));
  const retry=find(root,x=>x.tag==="button"&&x.textContent==="RETRY");
  assert.ok(retry,"queued message offers explicit retry");
  assert.equal(retry.disabled,false,"queued retry re-enables after send reload");
  failRetry=true;
  press(retry);await tick();
  assert.equal(state.messages[1].delivery,"queued");
  assert.ok(walk(root).some(x=>x.textContent.includes("RECEIPT REJECTED / DELIVERY UNKNOWN · 2 TRIES")),
    "failed retry reloads durable attempt evidence");
  failRetry=false;
  press(find(root,x=>x.tag==="button"&&x.textContent==="RETRY"));await tick();
  assert.equal(state.messages[1].delivery,"stored");
  assert.equal(calls.filter(x=>x.url==="/api/messages/retry").length,2);
  assert.ok(walk(root).some(x=>x.textContent.includes("RECEIPT VERIFIED / NOT READ · 3 TRIES")));
  assert.ok(!find(root,x=>x.tag==="button"&&x.textContent==="RETRY"));
  const route=find(root,x=>x.attrs["aria-label"]==="PACE delivery route");
  assert.ok(route);
  assert.equal(route.children.find(x=>x.value==="contingency").disabled,true);
  const beforeRoute=calls.length;
  route.value="alternate";press(route,"change");await tick();
  assert.equal(calls.length,beforeRoute,"route selection cannot contact a peer");
  input.value="Alternate lane report";failSend=false;
  press(form,"submit");await tick();
  assert.equal(JSON.parse(calls.findLast(x=>x.url==="/api/messages/send").options.body).tier,"alternate");
  route.value="emergency";press(route,"change");await tick();
  input.value="Hold this report";press(form,"submit");await tick();
  assert.equal(state.messages.at(-1).delivery,"queued");
  assert.ok(walk(root).some(x=>x.textContent.includes("HELD LOCALLY / NOT DELIVERED")));
  assert.ok(walk(root).some(x=>x.textContent.includes("LOCAL HOLD / MANUAL RETRY · 0 TRIES")));
  const heldRetry=find(root,x=>x.tag==="button"&&x.textContent==="RETRY");
  press(heldRetry);await tick();
  assert.equal(state.messages.at(-1).delivery,"queued","hold retry cannot claim remote delivery");
  assert.equal(state.messages.at(-1).progress.attempts,0,"hold is not a network attempt");
  const held=state.messages.at(-1);
  held.progress={attempts:1,held_locally:false,last_attempt:{route:{tier:"primary"},started_at:10,outcome:"started"}};
  await app.refresh();await tick();
  assert.ok(walk(root).some(x=>x.textContent.includes("ATTEMPT OUTCOME UNKNOWN")));
  delete held.progress;
  await app.refresh();await tick();
  assert.ok(walk(root).some(x=>x.textContent==="PRIOR ATTEMPT DETAILS UNAVAILABLE"));
  const before=calls.length;
  app.destroy();
  assert.equal(root.children.length,0);
  assert.equal(calls.length,before,"unmount has no outbound request");
  const emptyRoot=new Element("section");
  state={...state,device_id:null,peers:[],conversations:[],messages:[],pace:[]};
  const empty=window.RustBoyMessenger.mount(emptyRoot,{compact:true});
  await tick();
  assert.ok(walk(emptyRoot).some(x=>x.textContent.includes("NO PAIRED DEVICES")));
  const compose=find(emptyRoot,x=>x.tag==="form");
  const locked=find(emptyRoot,x=>x.tag==="button"&&x.textContent==="SEND VIA SSH");
  assert.equal(locked.disabled,true);
  press(compose,"submit");await tick();
  assert.equal(calls.filter(x=>x.url==="/api/messages/send").length,4);
  empty.destroy();
  state={...state,device_id:"scout-one",peers:[],conversations:["overseer-a"],pace:[],
    messages:[{id:1,peer:"overseer-a",direction:"incoming",body:"Inbound from HQ",
      delivery:"received",created_at:100,remote_origin_id:1,remote_stored_id:null}]};
  const receiver=new Element("div");
  const receiverApp=window.RustBoyMessenger.mount(receiver,{compact:true});
  await tick();
  assert.ok(walk(receiver).some(x=>x.textContent==="Inbound from HQ"));
  assert.ok(walk(receiver).some(x=>x.textContent.includes("RECEIVE ONLY")));
  assert.equal(find(receiver,x=>x.tag==="button"&&x.textContent==="SEND VIA SSH").disabled,true);
  receiverApp.destroy();
  state={...state,device_id:"overseer-a",peers:["scout-one"],conversations:["scout-one"],pace:pace(),storage:"age_os_keyring",messages:[]};
  const encryptedRoot=new Element("div");
  const encryptedApp=window.RustBoyMessenger.mount(encryptedRoot);await tick();
  assert.ok(walk(encryptedRoot).some(x=>x.textContent.includes("AGE HISTORY ENCRYPTION")));
  assert.ok(!walk(encryptedRoot).some(x=>x.textContent.includes("LOCAL HISTORY PLAINTEXT")));
  encryptedApp.destroy();
  console.log("Messenger UI: manual send, safe text, bounded input, queued retry, no auto transport PASS");
}
main().catch(e=>{console.error(e);process.exitCode=1;});
