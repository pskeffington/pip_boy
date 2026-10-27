"use strict";
(async()=>{
  const settle=()=>new Promise(resolve=>setTimeout(resolve,120));
  const send=value=>parent.postMessage({type:"dashboard-viewport",value},location.origin);
  try {
    for(let n=0;!snapshot&&n<25;n++)await settle();
    if(!snapshot)throw Error("Desktop state unavailable");
    activateStage("viewport");
    await settle();
    const canvas=document.querySelector(".rb-map-canvas-only canvas");
    if(!canvas)throw Error("Desktop map did not mount");
    const map=canvas.getBoundingClientRect();
    const zone=document.getElementById("workspace").getBoundingClientRect();
    const nav=document.getElementById("desktop-map-nav");
    const rect=nav.getBoundingClientRect();
    const result={coverage:(map.width*map.height)/(zone.width*zone.height),
      map:{width:map.width,height:map.height},zone:{width:zone.width,height:zone.height},
      mapButtons:document.querySelectorAll(".rb-map-canvas-only button").length,
      navigation:rect.width>0 && rect.height>=44 && rect.bottom<=zone.top+1};
    nav.value="settings";
    nav.dispatchEvent(new Event("change",{bubbles:true}));
    result.settings=activeStage==="settings" && !document.querySelector('[data-stage="settings"]').hidden;
    send(result);
  }catch(error){send({error:error.message});}
})();
