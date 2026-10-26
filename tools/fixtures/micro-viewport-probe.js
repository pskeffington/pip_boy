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
    for(const id of ["stat","role","data","radio","conf"]){
      setTab(id);await settle();
      const box=ui.viewport.getBoundingClientRect();
      const controls=[...ui.content.querySelectorAll("button,select,input")];
      report.pages.push({id,fits:ui.content.scrollHeight<=ui.viewport.clientHeight+1,
        height:ui.content.scrollHeight,available:ui.viewport.clientHeight,
        controlsFit:controls.every(x=>inside(x,box))});
    }
    parent.postMessage({type:"dashboard-viewport",value:report},location.origin);
  }catch(error){parent.postMessage({type:"dashboard-viewport",value:{error:error.message}},location.origin);}
})();
