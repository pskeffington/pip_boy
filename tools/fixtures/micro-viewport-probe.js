"use strict";
// Production assets, invented local data, no device operations.
const calls=[];
const localFetch=window.fetch.bind(window);
window.fetch=(url,options)=>{
  calls.push({url:String(url),method:options?.method||"GET"});
  return localFetch(url,options);
};
const settle=()=>new Promise(resolve=>setTimeout(resolve,120));
const inside=(node,box)=>{
  const r=node.getBoundingClientRect();
  return r.width>0 && r.height>0 && r.left>=box.left-1 && r.right<=box.right+1
    && r.top>=box.top-1 && r.bottom<=box.bottom+1;
};
(async()=>{
  try {
    for(let n=0;!snapshot&&n<30;n++)await settle();
    if(!snapshot)throw Error("Micro state unavailable");
    const report={tabs:ui.tabs.map(x=>x.dataset.tab),pages:[],calls};
    report.settings=inside(document.getElementById("micro-settings"),
      document.querySelector(".topbar").getBoundingClientRect());
    for(const id of ["stat","role","data","map","radio","conf"]){
      setTab(id);await settle();
      if(id === "map") {
        if(ui.content.querySelector(".rb-map-manual,.rb-map-zip,.rb-map-layers,.rb-map-tools"))
          throw Error("Location/source ingestion leaked onto Map");
        const nav=ui.content.querySelector(".rb-map-toolbar");
        if(nav || ui.content.querySelector(".rb-map button"))
          throw Error("Map must not show any buttons");
      }
      const box=ui.viewport.getBoundingClientRect();
      const header=document.querySelector(".topbar").getBoundingClientRect();
      if(header.height > 45 || Math.abs(box.top-header.bottom)>1)
        throw Error("Extra header row or gap above " + id);
      if(!inside(ui.title,header))throw Error("Page title clipped on " + id);
      const headerControls=[...document.querySelectorAll(".topbar button,.topbar a")]
        .filter(x=>x.getClientRects().length>0);
      if(!headerControls.every(x=>inside(x,header) && x.getBoundingClientRect().height>=44))
        throw Error("Header touch control clipped or undersized on " + id);
      const controls=[...ui.content.querySelectorAll("button,select,input")]
        .filter(x=>x.getClientRects().length>0);
      if(id === "radio" && !inside(ui.content.querySelector(".rb-radio-play"),box))
        throw Error("Primary Radio playback control clipped");
      report.pages.push({id,fits:ui.content.scrollHeight<=ui.viewport.clientHeight+1,
        height:ui.content.scrollHeight,available:ui.viewport.clientHeight,
        controlsFit:controls.every(x=>inside(x,box))});
    }
    if(hasTool("byte-lab")) {
      setTab("role"); openTool("byte-lab"); await settle();
      const header=document.querySelector(".topbar").getBoundingClientRect();
      if(ui.back.hidden || !inside(ui.back,header) || !inside(ui.title,header))
        throw Error("Detail Back/title missing from shared header");
      back();
    }
    parent.postMessage({type:"dashboard-viewport",value:report},location.origin);
  }catch(error){parent.postMessage({type:"dashboard-viewport",value:{error:error.message}},location.origin);}
})();
