"use strict";
// Direct media only: Rust-owned catalog, local bounded HLS relay, refreshed JPEG.
(function () {
  const make=(tag,cls,text)=>{
    const node=document.createElement(tag);node.className=cls;
    if(text!==undefined)node.textContent=text;
    return node;
  };
  function mount(root,options={}) {
    let connected=true,epoch=0,disposers=[];
    let selected=Number.isInteger(options.selected)?options.selected:null;
    const feeds=(options.feeds||[]).filter(feed=>{
      if(feed.kind==="unavailable")return !feed.media_url;
      if(feed.kind==="hls")return feed.media_url==="/api/cameras/boston/playlist.m3u8";
      return feed.kind==="jpeg" && feed.media_url==="https://fogcam.org/fogcam2.jpg";
    }).slice(0,4);
    const stop=()=>{epoch++;disposers.forEach(dispose=>dispose());disposers=[];};
    const button=(text,fn)=>{const node=make("button","rb-camera-button",text);node.type="button";node.addEventListener("click",fn);return node;};
    function render() {
      stop();root.replaceChildren();
      const current=epoch;
      root.classList.add("rb-camera-view");
      root.append(make("div","rb-camera-heading","DIRECT PUBLIC MEDIA · PRIVATE CAMERAS UNCONFIGURED"));
      if(!feeds.length){root.append(make("p","","DIRECT CAMERA CATALOG UNAVAILABLE"));return;}
      const grid=make("div","rb-camera-grid"+(selected!==null?" rb-camera-focused":""));
      feeds.forEach((feed,index)=>{
        if(selected!==null && selected!==index+1)return;
        const tile=make("section","rb-camera-tile");
        const top=make("div","rb-camera-top");
        const choose=()=>{selected=selected===null?index+1:null;options.onSelect?.(selected);render();};
        top.append(make("strong","",feed.city),button(selected===null?"OPEN":"GRID",choose));tile.append(top);
        const status=make("div","rb-camera-state",feed.kind==="jpeg"?"DIRECT JPEG SNAPSHOTS":feed.kind==="hls"?"DIRECT HLS VIDEO":"UNAVAILABLE / NO VERIFIED DIRECT FEED");
        if(feed.kind==="unavailable")tile.append(make("span","rb-camera-name",feed.name));
        else if(!options.compact || selected!==null) {
          if(feed.kind==="jpeg") {
            const image=make("img","rb-camera-player");image.alt=feed.city+" / "+feed.name+" / refreshed snapshot";
            let timer=null;
            const valid=()=>connected && epoch===current && !document.hidden;
            const load=()=>{if(valid())image.src=feed.media_url+"?rb="+Date.now();};
            image.onload=()=>{
              if(!valid())return;
              status.textContent="SNAPSHOT RECEIVED / "+new Date().toLocaleTimeString();
              timer=setTimeout(load,Math.max(20,Math.min(60,feed.refresh_seconds||20))*1000);
            };
            image.onerror=()=>{if(valid())status.textContent="DIRECT SNAPSHOT UNAVAILABLE";};
            tile.append(image);load();
            disposers.push(()=>{if(timer!==null)clearTimeout(timer);image.onload=null;image.onerror=null;image.removeAttribute("src");});
          } else {
            const video=make("video","rb-camera-player");
            video.muted=true;video.autoplay=true;video.playsInline=true;video.controls=true;
            video.setAttribute("aria-label",feed.city+" / "+feed.name+" / direct video");
            let hls=null;
            const valid=()=>connected && epoch===current;
            video.onplaying=()=>{if(valid())status.textContent="DIRECT VIDEO / PLAYING";};
            video.onerror=()=>{if(valid())status.textContent="DIRECT VIDEO UNAVAILABLE";};
            tile.append(video);
            if(video.canPlayType?.("application/vnd.apple.mpegurl")) {
              video.src=feed.media_url;
              video.play?.()?.catch(()=>{if(valid())status.textContent="PRESS PLAY FOR DIRECT VIDEO";});
            } else if(window.Hls?.isSupported()) {
              hls=new window.Hls({enableWorker:false,maxBufferLength:12,maxMaxBufferLength:30,backBufferLength:0,maxBufferSize:8*1024*1024,fragLoadingMaxRetry:1,manifestLoadingMaxRetry:1,levelLoadingMaxRetry:1});
              hls.on(window.Hls.Events.ERROR,(_event,data)=>{
                if(data.fatal && valid()){status.textContent="DIRECT VIDEO UNAVAILABLE";hls.destroy();hls=null;}
              });
              hls.loadSource(feed.media_url);hls.attachMedia(video);
            } else status.textContent="DIRECT HLS NOT SUPPORTED IN THIS BROWSER";
            disposers.push(()=>{if(hls)hls.destroy();video.onplaying=null;video.onerror=null;video.pause?.();video.removeAttribute("src");video.load?.();});
          }
        } else tile.append(make("span","rb-camera-name",feed.name));
        tile.append(status);
        const source=make("a","rb-camera-source",feed.publisher+" / SOURCE ↗");
        // Source links are documentation only, never used as media URLs.
        if(typeof feed.source_url==="string" && /^https:\/\/(www\.mwra\.com|fogcam\.org|www\.nyc\.gov|www\.earthcam\.com)\//.test(feed.source_url)) {
          source.href=feed.source_url;source.target="_blank";source.rel="noopener noreferrer";tile.append(source);
        }
        grid.append(tile);
      });
      root.append(grid);
    }
    const visibility=()=>{if(document.hidden)stop();else if(connected)render();};
    document.addEventListener?.("visibilitychange",visibility);
    render();
    return {destroy(){connected=false;stop();document.removeEventListener?.("visibilitychange",visibility);root.replaceChildren();}};
  }
  window.RustBoyCameras=Object.freeze({mount});
})();
