"use strict";
const assert=require("node:assert/strict"),fs=require("node:fs"),vm=require("node:vm"),path=require("node:path");
class E {
  constructor(tag){this.tag=tag;this.children=[];this.attrs={};this.listeners={};this.classList={add(){}};}
  append(...nodes){this.children.push(...nodes);}
  replaceChildren(...nodes){this.children=nodes;}
  setAttribute(k,v){this.attrs[k]=v;}
  addEventListener(k,f){this.listeners[k]=f;}
}
const walk=node=>[node,...node.children.flatMap(walk)];
const root=new E("div"),window={},document={createElement:tag=>new E(tag)};
vm.runInNewContext(fs.readFileSync(path.join(__dirname,"../crates/pipboy/assets/cameras.js"),"utf8"),{window,document});
const feeds=["DC","NYC","BOSTON","SF"].map((city,index)=>({city,name:city+" fixture",publisher:"Invented publisher",video_id:"testcam000"+index,source_url:"https://www.youtube.com/watch?v=testcam000"+index}));
let selected=null;
let controller=window.RustBoyCameras.mount(root,{feeds,onSelect:value=>selected=value});
let frames=()=>walk(root).filter(x=>x.tag==="iframe");
assert.equal(frames().length,4,"Desktop creates four public fallback players");
for(const frame of frames()){
  const url=new URL(frame.src);
  assert.equal(url.origin,"https://www.youtube-nocookie.com");
  assert.equal(url.searchParams.get("mute"),"1");
  assert.equal(frame.referrerPolicy,"strict-origin-when-cross-origin");
  assert.ok(!frame.attrs.sandbox.includes("allow-popups"));
}
walk(root).find(x=>x.tag==="button").listeners.click();
assert.equal(selected,1);assert.equal(frames().length,1,"Focus shows only the selected publisher view");
controller.destroy();assert.equal(root.children.length,0,"Exit removes every embedded player");
controller=window.RustBoyCameras.mount(root,{feeds,compact:true});
assert.equal(frames().length,0,"Small handheld waits for an explicit view selection");
walk(root).find(x=>x.tag==="button").listeners.click();assert.equal(frames().length,1);
controller.destroy();
controller=window.RustBoyCameras.mount(root,{feeds:[{...feeds[0],video_id:'../../unsafe'},{...feeds[0],source_url:'https://untrusted.example/camera'}]});
assert.equal(frames().length,0,"Malformed IDs and arbitrary source URLs cannot create frames");
controller.destroy();
console.log("PASS public cameras: four cities, fixed embed origin, muted focus, compact opt-in, cleanup, malformed-source rejection");
