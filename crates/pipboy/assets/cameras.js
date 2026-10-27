"use strict";
// Shared public-camera renderer. Catalog metadata comes from Rust /api/state.
// No URL input, camera discovery, credentials, recording or status polling.
(function () {
  const make=(tag,cls,text)=>{
    const node=document.createElement(tag);
    node.className=cls;
    if(text!==undefined)node.textContent=text;
    return node;
  };
  function mount(root,options={}) {
    let connected=true;
    let selected=Number.isInteger(options.selected)?options.selected:null;
    const feeds=(options.feeds||[]).filter(feed=>
      typeof feed.video_id==="string" && /^[A-Za-z0-9_-]{11}$/.test(feed.video_id) &&
      feed.source_url === "https://www.youtube.com/watch?v="+feed.video_id).slice(0,4);
    const choose=value=>{
      if(!connected)return;
      selected=value;
      options.onSelect?.(value);
      render();
    };
    const button=(label,fn)=>{
      const node=make("button","rb-camera-button",label);
      node.type="button";node.addEventListener("click",fn);return node;
    };
    function render() {
      root.replaceChildren();
      root.classList.add("rb-camera-view");
      const heading=make("div","rb-camera-heading","PUBLIC VIEWS · PRIVATE CAMERAS UNCONFIGURED");
      root.append(heading);
      if(!feeds.length){root.append(make("p","","PUBLIC CAMERA CATALOG UNAVAILABLE"));return;}
      const grid=make("div","rb-camera-grid"+(selected!==null?" rb-camera-focused":""));
      feeds.forEach((feed,index)=>{
        if(selected!==null && selected!==index+1)return;
        const tile=make("section","rb-camera-tile");
        const top=make("div","rb-camera-top");
        top.append(make("strong","",feed.city),button(selected===null?"OPEN":"GRID",()=>choose(selected===null?index+1:null)));
        tile.append(top);
        // Tiny handhelds select one view before creating its player. Desktop
        // opens four muted publisher players only upon entering Cameras.
        if(!options.compact || selected!==null) {
          const frame=make("iframe","rb-camera-player");
          frame.title=feed.city+" / "+feed.name+" / public view by "+feed.publisher;
          frame.src="https://www.youtube-nocookie.com/embed/"+feed.video_id+"?autoplay=1&mute=1&playsinline=1&rel=0";
          frame.referrerPolicy="strict-origin-when-cross-origin";
          frame.setAttribute("sandbox","allow-scripts allow-same-origin allow-presentation");
          frame.allow="autoplay; encrypted-media; picture-in-picture; fullscreen";
          frame.allowFullscreen=true;
          tile.append(frame);
        } else tile.append(make("span","rb-camera-name",feed.name));
        const source=make("a","rb-camera-source",feed.publisher+" ↗");
        source.href=feed.source_url;source.target="_blank";source.rel="noopener noreferrer";
        source.setAttribute("aria-label","Open original "+feed.city+" camera publisher page");
        tile.append(source);grid.append(tile);
      });
      root.append(grid);
    }
    render();
    return {destroy(){connected=false;root.replaceChildren();}};
  }
  window.RustBoyCameras=Object.freeze({mount});
})();
