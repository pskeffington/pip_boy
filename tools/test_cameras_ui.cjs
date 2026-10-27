"use strict";
const assert=require("node:assert/strict"),fs=require("node:fs"),vm=require("node:vm"),path=require("node:path");
class E {
  constructor(tag){this.tag=tag;this.children=[];this.attrs={};this.listeners={};this.classList={add(){}};}
  append(...nodes){this.children.push(...nodes);}
  replaceChildren(...nodes){this.children=nodes;}
  setAttribute(k,v){this.attrs[k]=v;}
  removeAttribute(k){delete this.attrs[k];if(k==="src")delete this.src;}
  addEventListener(k,f){this.listeners[k]=f;}
  canPlayType(){return "";}
  pause(){this.paused=true;}
  load(){}
}
const walk=node=>[node,...node.children.flatMap(walk)];
const instances=[];
class Hls {
  static Events={ERROR:"error"};
  static isSupported(){return true;}
  constructor(config){this.config=config;instances.push(this);}
  on(name,fn){this[name]=fn;}
  loadSource(url){this.url=url;}
  attachMedia(video){this.video=video;}
  destroy(){this.destroyed=true;}
}
const root=new E("div"),window={Hls},events={},timers=[];
const document={hidden:false,createElement:tag=>new E(tag),addEventListener:(k,fn)=>events[k]=fn,removeEventListener:k=>delete events[k]};
const source=fs.readFileSync(path.join(__dirname,"../crates/pipboy/assets/cameras.js"),"utf8");
assert.ok(!/youtube|iframe/i.test(source),"Camera runtime must not refer to hosted video pages or frames");
vm.runInNewContext(source,{window,document,Date,setTimeout:(fn,delay)=>{timers.push({fn,delay});return timers.length;},clearTimeout:id=>timers[id-1].cleared=true});
const feeds=[
  {city:"DC",name:"Pending",publisher:"Fixture",kind:"unavailable",media_url:"",source_url:"https://www.earthcam.com/cams/dc/"},
  {city:"NYC",name:"Unavailable",publisher:"Fixture",kind:"unavailable",media_url:"",source_url:"https://www.nyc.gov/html/dot/html/motorist/atis.shtml"},
  {city:"BOSTON",name:"Fixture video",publisher:"Fixture",kind:"hls",media_url:"/api/cameras/boston/playlist.m3u8",source_url:"https://www.mwra.com/about-mwra/live-camera-feeds"},
  {city:"SF",name:"Fixture snapshot",publisher:"Fixture",kind:"jpeg",media_url:"https://fogcam.org/fogcam2.jpg",source_url:"https://fogcam.org/",refresh_seconds:20}
];
const nodes=tag=>walk(root).filter(n=>n.tag===tag);
let selected=null;
let controller=window.RustBoyCameras.mount(root,{feeds,onSelect:value=>selected=value});
assert.equal(nodes("video").length,1);assert.equal(nodes("img").length,1);
assert.equal(nodes("iframe").length,0);
assert.equal(instances[0].url,"/api/cameras/boston/playlist.m3u8");
assert.equal(instances[0].config.enableWorker,false);
assert.equal(nodes("video")[0].muted,true);
const image=nodes("img")[0];image.onload();
assert.equal(timers[0].delay,20000,"Snapshot cadence follows provider interval");
assert.ok(nodes("a").every(n=>!n.href.includes("youtube")));
const focus=nodes("button")[3];focus.listeners.click();
assert.equal(selected,4);assert.equal(nodes("img").length,1);assert.equal(nodes("video").length,0);
assert.equal(instances[0].destroyed,true,"Focus change destroys previous HLS session");
controller.destroy();assert.equal(root.children.length,0);assert.equal(timers[0].cleared,true);
controller=window.RustBoyCameras.mount(root,{feeds,compact:true});
assert.equal(nodes("img").length+nodes("video").length,0,"Compact grid opens no media before selection");
nodes("button")[2].listeners.click();assert.equal(nodes("video").length,1);
document.hidden=true;events.visibilitychange();assert.equal(instances.at(-1).destroyed,true);
document.hidden=false;events.visibilitychange();assert.equal(instances.at(-1).destroyed,undefined);
controller.destroy();assert.equal(events.visibilitychange,undefined);
controller=window.RustBoyCameras.mount(root,{feeds:[{...feeds[2],media_url:"https://untrusted.example/playlist.m3u8"},{...feeds[3],media_url:"https://private.example/frame.jpg"}]});
assert.equal(nodes("video").length+nodes("img").length,0,"Arbitrary endpoints are rejected");controller.destroy();
console.log("PASS direct cameras: local bounded HLS, direct JPEG cadence, unavailable sources, focus, visibility, teardown, endpoint rejection, no hosted players");
