"use strict";
const assert=require("node:assert/strict"),fs=require("node:fs"),vm=require("node:vm"),path=require("node:path");
class E {
 constructor(tag){this.tag=tag;this.children=[];this.attrs={};this.listeners={};this.classList={add(){}};}
 append(...nodes){this.children.push(...nodes);}
 replaceChildren(...nodes){this.children=nodes;}
 setAttribute(k,v){this.attrs[k]=v;}
 removeAttribute(k){delete this.attrs[k];}
 addEventListener(k,f){this.listeners[k]=f;}
}
const walk=node=>[node,...node.children.flatMap(walk)];
const root=new E("div"),window={},document={createElement:tag=>new E(tag)};
vm.runInNewContext(fs.readFileSync(path.join(__dirname,"../crates/pipboy/assets/cameras.js"),"utf8"),{window,document,URL,Date});
const feeds=[
{id:"dc",city:"DC METRO",name:"US-50",publisher:"VDOT",image_url:"https://snapshot.vdotcameras.com/thumbs/RI0003.flv.png",source_url:"https://511.vdot.virginia.gov/",format:"jpeg_or_png_snapshot",region:"VA"},
{id:"nyc",city:"NYC",name:"Agreement pending",publisher:"NYC DOT",image_url:"",source_url:"https://webcams.nyctmc.org/subscribers",format:"unavailable",region:"NY"},
{id:"boston",city:"BOSTON",name:"Access pending",publisher:"MassDOT",image_url:"",source_url:"https://www.mass.gov/info-details/highway-data-for-developers",format:"unavailable",region:"MA"},
{id:"sf",city:"SF",name:"US-101",publisher:"Caltrans",image_url:"https://cwwp2.dot.ca.gov/data/d4/cctv/image/tv301us101atoctaviast/tv301us101atoctaviast.jpg",source_url:"https://cwwp2.dot.ca.gov/documentation/cctv/cctv.htm",format:"jpeg_snapshot",region:"CA"}
];
let selected=null;
let controller=window.RustBoyCameras.mount(root,{feeds,onSelect:x=>selected=x});
const imgs=()=>walk(root).filter(x=>x.tag==="img");
const buttons=()=>walk(root).filter(x=>x.tag==="button");
assert.equal(imgs().length,2,"Direct images only; two access-gated placeholders");
assert.equal(walk(root).filter(x=>x.tag==="iframe").length,0,"Never embed YouTube");
buttons()[0].listeners.click();assert.equal(selected,1);assert.equal(imgs().length,1);
controller.destroy();assert.equal(root.children.length,0);
controller=window.RustBoyCameras.mount(root,{feeds,compact:true});
assert.equal(imgs().length,0,"Compact view requires explicit selection");
buttons()[0].listeners.click();assert.equal(imgs().length,1);
controller.destroy();
controller=window.RustBoyCameras.mount(root,{feeds:[{...feeds[0],image_url:"https://evil.example/thumbs/RI0003.flv.png"},{...feeds[3],image_url:"https://cwwp2.dot.ca.gov@evil.example/a.jpg"}]});
assert.equal(imgs().length,0,"Reject arbitrary media hosts");
controller.destroy();
console.log("PASS cameras: DOT HTTPS snapshots, unavailable placeholders, no embeds, focus and cleanup");
