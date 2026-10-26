/* Rust-Boy dedicated message view. No cloud, polling, dynamic HTML, or
 * transport calls until SEND/RETRY. All content is rendered as text nodes.
 * PACE routes are operator-selected; storage mode is verified by the local API.
 */
(function () {
  "use strict";
  function el(tag, className = "", content = "") {
    const node = document.createElement(tag);
    node.className = className;
    node.textContent = content;
    return node;
  }
  function btn(text, fn) {
    const node = el("button", "rb-msg-button", text);
    node.type = "button";
    node.addEventListener("click", fn);
    return node;
  }
  async function request(url, body, signal) {
    const options = {cache:"no-store", credentials:"omit"};
    if (signal) options.signal = signal;
    if (body !== undefined) {
      options.method = "POST";
      options.headers = {"Content-Type":"application/json"};
      options.body = JSON.stringify(body);
    }
    const response = await fetch(url,options);
    let data = {};
    try { data = await response.json(); } catch (_) { /* Fail closed below. */ }
    if (!response.ok) throw new Error(data.error || "Local Messenger request failed");
    return data;
  }

  function mount(root, {compact=false}={}) {
    if (!root) return null;
    let closed = false, version = 0, busy = false, state = null;
    let selected = "", selectedTier = "primary", page = 0, feedback = "", failure = false;
    let controller = null;
    const pageSize = compact ? 3 : 7;
    const title = el("div","rb-msg-title","RB / MESSENGER");
    const transport = el("div","rb-msg-transport",
      "PAIRED OPENSSH / MANUAL DELIVERY");
    const warning = el("div","rb-msg-warning",
      "ENCRYPTED IN TRANSIT ONLY · LOCAL HISTORY PLAINTEXT · NOT E2EE");
    const status = el("div","rb-msg-status","Reading local message log…");
    status.setAttribute("role","status");
    status.setAttribute("aria-live","polite");

    const peerRow = el("div","rb-msg-toolbar");
    const peerLabel = el("label","rb-msg-label","DEVICE");
    const peerSelect = el("select","rb-msg-select");
    peerSelect.setAttribute("aria-label","Messenger destination device");
    peerLabel.append(peerSelect);
    const refresh = btn("REFRESH", () => { void load(); });
    const routeLabel=el("label","rb-msg-label","PACE");
    const routeSelect=el("select","rb-msg-select");
    routeSelect.setAttribute("aria-label","PACE delivery route");
    routeLabel.append(routeSelect);
    peerRow.append(peerLabel,routeLabel,refresh);

    const thread = el("div","rb-msg-thread");
    thread.setAttribute("aria-label","Stored messages on this device");
    const pager = el("div","rb-msg-pager");
    const prev = btn("◀",() => {page=Math.max(0,page-1);drawMessages();});
    const pageLabel = el("span","","1/1");
    const next = btn("▶",() => {page++;drawMessages();});
    pager.append(prev,pageLabel,next);

    const form = el("form","rb-msg-compose");
    const entryLabel = el("label","rb-msg-label","MESSAGE");
    const input = el("textarea","rb-msg-input");
    input.maxLength = 1024;
    input.rows = compact ? 2 : 3;
    input.placeholder = "Type a short device message";
    input.setAttribute("aria-label","Message body; 1024 UTF-8 bytes maximum");
    entryLabel.append(input);
    const send = el("button","rb-msg-button rb-msg-send","SEND VIA SSH");
    send.type = "submit";
    form.append(entryLabel,send);
    root.replaceChildren(title,transport,warning,status,peerRow,thread,pager,form);

    function planForPeer() {
      return state?.pace?.find(plan=>plan.peer===selected);
    }
    function selectedLane() {
      const plan=planForPeer();
      if(plan)return plan.lanes.find(lane=>lane.tier===selectedTier);
      return {selectable:selectedTier==="primary" && state?.peers?.includes(selected)};
    }
    function canCompose() {
      return Boolean(selected && state?.peers?.includes(selected) && selectedLane()?.selectable);
    }
    function drawRoutes() {
      const plan=planForPeer();
      const lanes=plan?.lanes || [{tier:"primary",selectable:state?.peers?.includes(selected)}];
      if(!lanes.some(lane=>lane.tier===selectedTier && lane.selectable))selectedTier="primary";
      routeSelect.replaceChildren();
      const names={primary:"P · PAIRED SSH",alternate:"A · LOCAL IP",contingency:"C · RX ONLY",emergency:"E · LOCAL HOLD"};
      for(const lane of lanes){
        const option=el("option","",names[lane.tier] || "UNAVAILABLE");
        option.value=lane.tier;option.disabled=!lane.selectable;routeSelect.append(option);
      }
      routeSelect.value=selectedTier;routeSelect.disabled=busy || !state?.peers?.includes(selected);
      send.textContent=selectedTier==="emergency"?"QUEUE / HOLD":selectedTier==="alternate"?"SEND VIA LOCAL IP":"SEND VIA SSH";
      send.disabled=busy || !canCompose();
      transport.textContent=plan ? "P/A SHARE LAN · C RX ONLY · E LOCAL HOLD" : "PAIRED OPENSSH / MANUAL DELIVERY";
      warning.textContent=state?.storage==="age_os_keyring" ?
        "SSH IN TRANSIT · AGE HISTORY ENCRYPTION · NOT E2EE" :
        "ENCRYPTED IN TRANSIT ONLY · LOCAL HISTORY PLAINTEXT · NOT E2EE";
    }
    function notice(message, error=false) {
      feedback=message;failure=error;
      if(!closed)renderStatus();
    }
    function renderStatus() {
      if (feedback) {
        status.textContent = feedback;
      } else if (!state) {
        status.textContent = "READING LOCAL JOURNAL";
      } else {
        status.textContent = (state.device_id || "UNENROLLED").toUpperCase() +
          " · " + state.messages.length + " LOCAL RECORDS";
      }
      status.className = "rb-msg-status" + (failure ? " rb-msg-error" : "");
    }
    function drawMessages() {
      thread.replaceChildren();
      if (!state) return;
      const messages = state.messages.filter(item => item.peer === selected)
        .sort((a,b) => b.id-a.id);
      const pages = Math.max(1, Math.ceil(messages.length/pageSize));
      page=Math.min(page,pages-1);
      pageLabel.textContent = (page+1)+"/"+pages;
      prev.disabled=page===0;
      next.disabled=page===pages-1;
      if (!selected) {
        thread.append(el("div","rb-msg-empty",
          "NO PAIRED DEVICES · SET UP A DEDICATED SSH KEY AND PIN HOST FINGERPRINT"));
      } else if(!state.peers.includes(selected)) {
        thread.append(el("div","rb-msg-empty",
          "RECEIVE ONLY / PAIR A RETURN SSH KEY TO REPLY"));
      }
      if(selected && !messages.length) {
        thread.append(el("div","rb-msg-empty",
          "NO SAVED MESSAGES FOR "+selected.toUpperCase()));
      }
      for(const message of messages.slice(page*pageSize,(page+1)*pageSize)){
        const row=el("article","rb-msg-item");
        const head=el("div","rb-msg-meta",
          (message.direction==="incoming"?"IN":"OUT")+
          " #"+message.id+" · "+
          (message.delivery==="stored"?"STORED ON PEER":
           message.delivery==="queued"?"QUEUED / NOT DELIVERED":"RECEIVED ON DEVICE"));
        const body=el("p","rb-msg-body",message.body);
        row.append(head,body);
        if (message.direction==="outgoing" && message.delivery==="queued") {
          const retry=btn("RETRY",async () => {
            if (busy || closed) return;
            busy=true;send.disabled=true;retry.disabled=true;routeSelect.disabled=true;peerSelect.disabled=true;
            notice(selectedTier==="emergency" ? "HOLDING LOCALLY / NO CONNECTION" : "CONTACTING PAIRED DEVICE / EXPLICIT RETRY");
            try{
              const response=await request("/api/messages/retry",{id:message.id,confirm:true,tier:selectedTier});
              if(closed)return;
              notice(response.stored_on_receiver ?
                "MESSAGE STORED ON RECEIVER / NOT READ" :
                response.held_locally ? "HELD LOCALLY / NOT DELIVERED" : "DELIVERY NOT CONFIRMED");
              await load(true);
            }catch(error){
              if(!closed)notice("RETRY FAILED / LOCAL MESSAGE PRESERVED",true);
            }finally{
              busy=false;if(!closed){drawRoutes();peerSelect.disabled=!state?.conversations?.length;retry.disabled=false;}
            }
          });
          retry.setAttribute("aria-label","Retry queued message "+message.id+
            " to "+message.peer);
          row.append(retry);
        }
        thread.append(row);
      }
    }

    function refreshPeers() {
      const choices=Array.isArray(state?.conversations) ? state.conversations : [];
      peerSelect.replaceChildren();
      if (!choices.length) {
        const option=el("option","","NO PAIRED DEVICES");
        option.value="";peerSelect.append(option);selected="";
      } else {
        if (!choices.includes(selected))selected=choices[0];
        for (const alias of choices) {
          const option=el("option","",alias.toUpperCase());
          option.value=alias;peerSelect.append(option);
        }
      }
      peerSelect.value=selected;
      peerSelect.disabled=!choices.length || busy;
      drawRoutes();
      renderStatus();
      drawMessages();
    }
    async function load(preserveFeedback=false){
      if(closed) return;
      const current=++version;
      if(controller)controller.abort();
      controller=new AbortController();
      if(!preserveFeedback){feedback="";failure=false;}
      renderStatus();
      try{
        const data=await request("/api/messages",undefined,controller.signal);
        if(closed || current!==version)return;
        if(data.schema_version!==1 || !Array.isArray(data.messages) ||
           !Array.isArray(data.peers) || !Array.isArray(data.conversations) ||
           data.transport!=="pinned_openssh_explicit" ||
           data.e2ee!==false || !["plaintext_local_only","age_os_keyring"].includes(data.storage)){
          throw new Error("Unrecognized Messenger security contract");
        }
        state=data;
        refreshPeers();
      }catch(error){
        if(!closed && current===version &&
           !controller.signal.aborted)notice("LOCAL MESSENGER UNAVAILABLE / "+error.message,true);
      }
    }

    peerSelect.addEventListener("change",()=>{
      selected=peerSelect.value;selectedTier="primary";page=0;drawRoutes();drawMessages();
    });
    routeSelect.addEventListener("change",()=>{
      selectedTier=routeSelect.value;drawRoutes();
    });
    form.addEventListener("submit",async event=>{
      event.preventDefault();
      if(closed || busy || !canCompose())return;
      const body=input.value;
      const bytes=new TextEncoder().encode(body).length;
      if(!body.trim() || bytes>1024){
        notice("MESSAGE MUST BE 1–1024 UTF-8 BYTES",true);return;
      }
      busy=true;send.disabled=true;refresh.disabled=true;peerSelect.disabled=true;routeSelect.disabled=true;
      notice(selectedTier==="emergency" ? "HOLDING LOCALLY / NO CONNECTION" : "CONNECTING TO PEER / NO READ RECEIPT");
      try{
        const data=await request("/api/messages/send",{alias:selected,body,confirm:true,tier:selectedTier});
        if(closed)return;
        input.value="";
        page=0;
        notice(data.stored_on_receiver ?
          "STORED ON PEER / NOT MARKED READ" :
          data.held_locally ? "HELD LOCALLY / NOT DELIVERED" : "QUEUED LOCALLY / PEER UNAVAILABLE · RETRY MANUALLY",
          !data.stored_on_receiver && !data.held_locally);
        await load(true);
      }catch(error){
        if(!closed)notice("NOT SENT / "+error.message,true);
      }finally{
        busy=false;
        if(!closed){
          refresh.disabled=false;peerSelect.disabled=!state?.conversations?.length;
          drawRoutes();
        }
      }
    });
    void load();
    return {
      refresh:()=>load(),
      destroy:()=>{
        closed=true;version++;if(controller)controller.abort();
        root.replaceChildren();
      }
    };
  }
  window.RustBoyMessenger=Object.freeze({mount});
})();
