/* Structured aid UI. Only deliberate confirmed actions may transmit. The
 * background reader checks the local journal; it never contacts peer devices. */
(function () {
  "use strict";
  const make = (tag, cls="", value="") => {
    const node=document.createElement(tag); node.className=cls; node.textContent=value; return node;
  };
  const button = (label, action, cls="") => {
    const node=make("button","aid-button "+cls,label); node.type="button";
    node.addEventListener("click", action); return node;
  };
  async function api(path, body) {
    const options={cache:"no-store",credentials:"omit"};
    if(body!==undefined){options.method="POST";options.headers={"Content-Type":"application/json"};options.body=JSON.stringify({...body,confirm:true});}
    const response=await fetch(path,options); const data=await response.json();
    if(!response.ok)throw Error(data.error || "Local aid service unavailable");
    return data;
  }
  const labels=["Pickup location / grid zone","Contact frequency / call sign","Patients by precedence","Special equipment required","Patients by type (litter / ambulatory)","Wounds / illness (peacetime)","Pickup marking method","Patient nationality / status","Terrain / obstacles (peacetime)"];
  const hints=["Enter a known pickup location. No GPS is acquired.","How the receiving Doctor can contact this unit.","Record the Medic's assessment; no priority is inferred.","State required equipment, NONE or UNKNOWN.","Enter patient counts and transport needs.","Describe known information; avoid patient names.","Describe the actual marking, NONE or UNKNOWN.","Record only relevant operational information.","Describe known obstacles or UNKNOWN."];
  let state=null, busy=false, screen="beacon", step=0, review=0, inbox=0, detail=0, setupStep=0;
  let mode="peacetime", draft=Array(9).fill(""), configDraft=null, hold=null;
  let loadedVersion=0, destroyed=false;
  const trigger=document.getElementById("emergency-button");
  const dialog=make("dialog","aid-dialog"); dialog.setAttribute("aria-label","Emergency and medical communications");
  const header=make("header","aid-header");
  header.append(make("strong","","RB / AID"),button("REFRESH",()=>{void load(true);}),button("CLOSE",()=>dialog.close()));
  const tabs=make("nav","aid-tabs"); tabs.setAttribute("aria-label","Aid pages");
  const status=make("div","aid-status","LOCAL JOURNAL / NO RESCUE SERVICE"); status.setAttribute("role","status");
  const body=make("section","aid-body"); body.setAttribute("tabindex","-1");
  dialog.append(header,tabs,status,body); document.body.append(dialog);
  function notice(value,error=false){status.textContent=value;status.classList.toggle("aid-error",error);}
  function stopHold(){if(hold!==null){clearTimeout(hold);hold=null;}}
  function banner(){
    if(!trigger)return;
    const active=state?.state?.beacon?.active;
    const alarms=new Map();
    const cancelled=new Set((state?.records || []).filter(r=>r.direction==="incoming" && r.packet.kind==="cancel").map(r=>r.peer+":"+r.packet.incident));
    for(const record of (state?.records || []).slice().reverse()){
      if(record.direction!=="incoming")continue;
      const packet=record.packet, key=record.peer+":"+packet.incident;
      if(packet.kind==="alarm" && !cancelled.has(key))alarms.set(key,true);
      if(packet.kind==="cancel")alarms.delete(key);
    }
    trigger.textContent=active?"SOS ACTIVE":state?.journal_available===false?"SOS ?":alarms.size?"ALERT "+alarms.size:"SOS";
    trigger.dataset.active=String(Boolean(active || alarms.size));
    trigger.setAttribute("aria-label",active?"Emergency active; open beacon controls":alarms.size?"Incoming emergency alert; open aid inbox":"Open emergency beacon");
  }
  async function load(draw=false){
    const version=++loadedVersion;
    try {
      const data=await api("/api/aid");
      if(destroyed || version!==loadedVersion)return;
      state=data;banner();
      if(draw && dialog.open)render();
    }catch(error){
      if(destroyed || version!==loadedVersion)return;
      if(trigger){trigger.textContent="SOS ?";trigger.setAttribute("aria-label","Emergency status unavailable; open controls");}
      if(dialog.open)notice(error.message,true);
    }
  }
  async function action(path, payload={}, success="ACTION SAVED"){
    if(busy)return;
    busy=true;stopHold();
    dialog.setAttribute("aria-busy","true");
    for(const node of dialog.querySelectorAll("button,input,select"))node.disabled=true;
    notice("WORKING / DELIVERY NOT YET CONFIRMED");
    try{
      const result=await api(path,payload);
      if(result.beacon && state){state.state.beacon=result.beacon;banner();}
      await load(false);
      busy=false;render();
      const delivery=result.delivery;
      const message=delivery?delivery.stored_on_receiver?"STORED ON PEER / NOT HUMAN ACK":delivery.status==="local_only"?"LOCAL ALARM ONLY / NO DESTINATION":"NOT CONFIRMED / RETRY EXPLICITLY":
        result.message?result.stored_on_receiver?"STORED ON PEER / NOT HUMAN READ":"QUEUED / NOT CONFIRMED":success;
      notice(message);
      return result;
    }catch(error){busy=false;render();notice(error.message,true);}
    finally{dialog.removeAttribute("aria-busy");}
  }
  function field(label,value,onInput,{max=48,options=null}={}){
    const wrap=make("label","aid-field",label);
    const node=make(options?"select":"input");
    if(options){for(const [value,text] of options){const option=make("option","",text);option.value=value;node.append(option);}}
    else{node.type="text";node.maxLength=max;node.autocomplete="off";node.spellcheck=false;}
    node.value=value;node.addEventListener(options?"change":"input",()=>onInput(node.value));
    wrap.append(node);return wrap;
  }
  function row(...nodes){const node=make("div","aid-row");node.append(...nodes);return node;}
  function pager(index,count,onChange){
    const prev=button("◀ PREV",()=>onChange(index-1)),next=button("NEXT ▶",()=>onChange(index+1));
    prev.disabled=index===0;next.disabled=index>=count-1;
    return row(prev,make("span","aid-count",(index+1)+" / "+count),next);
  }
  function confirmAction(label,fn){
    stopHold();body.replaceChildren(make("p","",label),make("p","aid-muted","Only the configured paired device can receive this alert."),
      row(button("BACK",render),button("CONFIRM",()=>{void fn();},"aid-danger")));
    body.querySelector("button:last-child")?.focus();
  }
  // Touch/key hold is guarded against release, pointer loss, hidden tabs and
  // key repeat. A tap/click offers an explicit two-step accessible alternative.
  function guarded(label,fn,delay=2000,existing=null,onTap=null){
    let fired=false;
    const click=()=>{
      if(fired){fired=false;return;}
      if(onTap)onTap();else confirmAction(label,fn);
    };
    const node=existing || button(label,click,"aid-danger");
    if(existing)node.addEventListener("click",click);
    const start=()=>{if(busy || hold!==null)return;fired=false;notice("KEEP HOLDING / RELEASE TO ABORT");
      hold=setTimeout(()=>{hold=null;fired=true;void fn();},delay);};
    node.addEventListener("pointerdown",event=>{if(event.button!==0)return;node.setPointerCapture?.(event.pointerId);start();});
    node.addEventListener("pointerup",stopHold);
    node.addEventListener("pointercancel",()=>{fired=true;stopHold();});
    node.addEventListener("lostpointercapture",stopHold);
    node.addEventListener("pointerleave",stopHold);
    node.addEventListener("blur",stopHold);
    node.addEventListener("keydown",event=>{
      if(![" ","Enter"].includes(event.key))return;
      event.preventDefault();if(!event.repeat)start();
    });
    node.addEventListener("keyup",event=>{
      if(![" ","Enter"].includes(event.key))return;
      event.preventDefault();stopHold();if(!fired){if(onTap)onTap();else confirmAction(label,fn);}
    });
    return node;
  }
  function showBeacon(){
    const beacon=state?.state?.beacon;
    body.append(make("strong","",beacon?.active?"EMERGENCY LATCHED":"EMERGENCY BUTTON"));
    if(beacon){
      body.append(make("p","aid-muted","#"+beacon.id+" · "+(beacon.peer || "NO DESTINATION")+" · "+beacon.location));
      body.append(make("p","",(state.beacon_delivery || "NOT CONFIRMED").toUpperCase()+" / "+
        (state.human_acknowledged?"OPERATOR ACKNOWLEDGED":"NO HUMAN ACK")+(beacon.active?"":" / LOCALLY CANCELLED")));
    }else body.append(make("p","aid-muted","Hold 2s or tap to confirm. Alarm persists after restart."));
    body.append(make("p","aid-muted",state?.state?.config?.emergency_peer?"PAIRED DEVICE ONLY / NO PUBLIC RESCUE DISPATCH":"SETUP A DESTINATION / OTHERWISE LOCAL ALARM ONLY"));
    if(beacon?.active){
      body.append(row(guarded("HOLD 3s TO CANCEL",()=>action("/api/aid/cancel",{id:beacon.id}),3000),
        button("RETRY ALERT",()=>{void action("/api/aid/transmit",{id:beacon.id});})));
    }else{
      const activate=guarded("HOLD 2s FOR SOS",()=>action("/api/aid/activate"));
      body.append(beacon?row(activate,button("RETRY CANCEL",()=>{void action("/api/aid/transmit",{id:beacon.id});})):activate);
    }
  }
  function showNine(){
    if(!state?.medical_allowed){body.append(make("p","","Load Medic or Doctor to use nine-line medical communications."));return;}
    if(step<9){
      const format=step===0?field("FORMAT",mode,value=>{mode=value;}, {options:[["peacetime","PEACETIME"],["wartime","WARTIME"]]}):null;
      const title=step===5 && mode==="wartime"?"Pickup site security":step===8 && mode==="wartime"?"NBC / CBRN contamination":labels[step];
      const entry=field("LINE "+(step+1)+" · "+title,draft[step],value=>{draft[step]=value;});
      body.append(format?row(format,entry):entry);
      if(step!==0)body.append(make("p","aid-muted",hints[step]+" 48 UTF-8 bytes; UNKNOWN is explicit."));
      const prev=button("◀ BACK",()=>{step--;render();});prev.disabled=step===0;
      body.append(row(prev,button(step===8?"REVIEW ▶":"NEXT ▶",()=>{
        if(!draft[step].trim() || new TextEncoder().encode(draft[step]).length>48){notice("Enter 1–48 UTF-8 bytes, or UNKNOWN",true);return;}
        step++;review=0;render();body.querySelector("input")?.focus();
      })));
    }else{
      body.append(make("strong","","TO DOCTOR: "+(state.state.config.doctor_peer || "NOT CONFIGURED")));
      for(let i=review*3;i<review*3+3;i++)body.append(make("p","aid-line",(i+1)+" · "+draft[i]));
      body.append(pager(review,3,value=>{review=value;render();}));
      body.append(row(button("EDIT",()=>{step=review*3;render();}),button("SEND 9 LINE",()=>{
        confirmAction("SEND NINE-LINE TO "+(state.state.config.doctor_peer || "UNCONFIGURED DOCTOR"),()=>action("/api/aid/nine-line",{packet:{kind:"nine_line",mode,lines:draft}}));
      })));
    }
  }
  function showInbox(){
    const records=(state?.records || []).filter(r=>r.direction==="incoming");
    if(!records.length){body.append(make("p","","NO RECEIVED AID MESSAGES"));return;}
    inbox=Math.min(inbox,records.length-1);
    const record=records[inbox],packet=record.packet;
    body.append(make("strong","",record.peer+" / "+packet.kind.toUpperCase().replaceAll("_"," ")+(packet.mode?" / "+packet.mode.toUpperCase():"")));
    if(packet.kind==="nine_line"){
      for(let i=detail*3;i<detail*3+3;i++)body.append(make("p","aid-line",(i+1)+" · "+packet.lines[i]));
      body.append(button("LINES "+(detail*3+1)+"–"+(detail*3+3)+" / NEXT",()=>{detail=(detail+1)%3;render();}));
    }else{
      body.append(make("p","","INCIDENT "+packet.incident));
      if(packet.location)body.append(make("p","",packet.location));
      if(packet.kind==="alarm")body.append(button("ACKNOWLEDGE",()=>{
        confirmAction("ACKNOWLEDGE ALARM / DOES NOT PROMISE RESCUE",()=>action("/api/aid/ack",{message_id:record.id}));
      }));
    }
    body.append(pager(inbox,records.length,value=>{inbox=value;detail=0;render();}));
  }
  function showSetup(){
    if(!configDraft)configDraft={...state.state.config};
    const choices=[["","NOT CONFIGURED"],...state.peers.map(alias=>[alias,alias])];
    if(setupStep===0){
      body.append(field("DOCTOR DEVICE",configDraft.doctor_peer,v=>{configDraft.doctor_peer=v;},{options:choices}),
        field("EMERGENCY DESTINATION",configDraft.emergency_peer,v=>{configDraft.emergency_peer=v;},{options:choices}),
        button("NEXT ▶",()=>{setupStep=1;render();}));
    }else{
      body.append(field("MANUAL LOCATION (BLANK = UNKNOWN)",configDraft.location,v=>{configDraft.location=v;},{max:80}),
        field("TRANSPORT",configDraft.tier,v=>{configDraft.tier=v;},{options:[["primary","P / PAIRED SSH"],["alternate","A / CONFIGURED LOCAL IP"]]}),
        row(button("◀ BACK",()=>{setupStep=0;render();}),button("SAVE / NO SEND",()=>{
          void action("/api/aid/config",{config:configDraft},"CONFIGURATION SAVED / NO SEND");
        })));
    }
  }
  function render(){
    stopHold();tabs.replaceChildren();body.replaceChildren();
    for(const [id,label] of [["beacon","SOS"],["nine","9 LINE"],["inbox","INBOX"],["setup","SETUP"]]){
      const node=button(label,()=>{screen=id;render();});node.setAttribute("aria-pressed",String(screen===id));tabs.append(node);
    }
    if(!state){body.append(make("p","","LOCAL AID SERVICE NOT READY"));return;}
    notice(state.journal_available===false?"JOURNAL UNAVAILABLE / DELIVERY UNKNOWN":state.storage==="age_os_keyring"?"ENCRYPTED JOURNAL / NOT E2EE":"LOCAL HISTORY PLAINTEXT / NOT E2EE");
    if(screen==="nine")showNine();else if(screen==="inbox")showInbox();else if(screen==="setup")showSetup();else showBeacon();
  }
  function open(next="beacon"){
    screen=next;configDraft=null;
    if(!dialog.open)dialog.showModal();render();void load(true);
  }
  if(trigger){
    trigger.title="Hold 2 seconds for SOS; tap for controls";
    guarded("ACTIVATE EMERGENCY",()=>{
      if(state?.state?.beacon?.active){open("beacon");return;}
      screen="beacon";
      if(!dialog.open)dialog.showModal();
      return action("/api/aid/activate");
    },2000,trigger,()=>open(trigger.textContent.startsWith("ALERT")?"inbox":"beacon"));
  }
  dialog.addEventListener("close",stopHold);dialog.addEventListener("cancel",stopHold);
  document.addEventListener("visibilitychange",()=>{stopHold();});
  window.addEventListener("blur",stopHold);
  const timer=setInterval(()=>{if(!document.hidden && !busy)void load(false);},10000);
  window.RustBoyAid={open,guarded,destroy(){destroyed=true;stopHold();clearInterval(timer);dialog.remove();}};
  void load(false);
})();
