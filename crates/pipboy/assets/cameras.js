"use strict";
// DOT-first camera wall. Explicitly published HTTPS images only; no network
// access until the Cameras tool opens, and no claims of camera freshness.
(function(){
 const make=(tag,cls,text)=>{const el=document.createElement(tag);el.className=cls;if(text!==undefined)el.textContent=text;return el;};
 function safeFeed(feed) {
   if(!feed||typeof feed.city!=="string"||typeof feed.publisher!=="string")return false;
   if(!feed.image_url)return feed.format==="unavailable";
   try{
     const u=new URL(feed.image_url);
     if(u.protocol!=="https:"||u.username||u.password||u.port||u.search||u.hash)return false;
     const path=u.pathname;
     return (u.hostname==="snapshot.vdotcameras.com"&&/^\/thumbs\/[A-Za-z0-9_-]+\.flv\.png$/.test(path))||
       (u.hostname==="cwwp2.dot.ca.gov"&&/^\/data\/d[0-9]{1,2}\/cctv\/image\/[A-Za-z0-9_-]+\/[A-Za-z0-9_-]+\.jpg$/.test(path));
   }catch(_){return false;}
 }
 function mount(root,options={}){
   let connected=true, selected=Number.isInteger(options.selected)?options.selected:null;
   let feeds=(Array.isArray(options.feeds)?options.feeds:[]).filter(safeFeed);
   // Prioritize nearby sources when the operator has explicitly chosen a region.
   if(typeof options.region==="string"&&/^[A-Z]{2}$/.test(options.region)){
      const region=options.region;feeds=feeds.slice().sort((a,b)=>Number(b.region===region)-Number(a.region===region));
   }
   const choose=value=>{if(!connected)return;selected=value;options.onSelect?.(value);render();};
   const button=(label,fn)=>{const node=make("button","rb-camera-button",label);node.type="button";node.addEventListener("click",fn);return node;};
   function render(){
     root.replaceChildren();root.classList.add("rb-camera-view");
     root.append(make("div","rb-camera-heading","DOT / PUBLIC ROAD VIEWS · IMAGE AGE NOT VERIFIED"));
     if(!feeds.length){root.append(make("p","","NO APPROVED DOT FEEDS CONFIGURED"));return;}
     const grid=make("div","rb-camera-grid"+(selected!==null?" rb-camera-focused":""));
     feeds.forEach((feed,index)=>{
       if(selected!==null&&selected!==index+1)return;
       const tile=make("section","rb-camera-tile");
       const top=make("div","rb-camera-top");
       top.append(make("strong","",feed.city),button(selected===null?"OPEN":"GRID",()=>choose(selected===null?index+1:null)));
       tile.append(top,make("span","rb-camera-name",feed.name));
       if(feed.image_url&&(!options.compact||selected!==null)){
         const img=make("img","rb-camera-player");
         img.alt="DOT road camera snapshot; capture time not verified";
         img.referrerPolicy="no-referrer";img.loading="lazy";img.decoding="async";
         const status=make("span","rb-camera-status","LOADING · FRESHNESS UNKNOWN");
         img.addEventListener("load",()=>{if(connected)status.textContent="IMAGE RECEIVED · FRESHNESS UNKNOWN";});
         img.addEventListener("error",()=>{if(connected){status.textContent="FEED UNAVAILABLE";img.removeAttribute("src");}});
         img.src=feed.image_url;
         tile.append(img,status);
         // Refresh is opt-in and bounded: no unattended polling or browser playback.
         const refresh=button("REFRESH",()=>{if(connected){status.textContent="LOADING · FRESHNESS UNKNOWN";img.src=feed.image_url+"?view="+Date.now();}});
         tile.append(refresh);
       }else tile.append(make("span","rb-camera-status",feed.image_url?"OPEN TO FETCH IMAGE":"ACCESS REQUIRED / NOT LIVE"));
       if(feed.source_url&&/^https:\/\/[a-z0-9.-]+\//i.test(feed.source_url)){
         const source=make("a","rb-camera-source",feed.publisher+" / SOURCE");
         source.href=feed.source_url;source.target="_blank";source.rel="noopener noreferrer";
         tile.append(source);
       }
       grid.append(tile);
     });
     root.append(grid);
   }
   render();
   return {destroy(){connected=false;root.replaceChildren();}};
 }
 window.RustBoyCameras=Object.freeze({mount});
})();
