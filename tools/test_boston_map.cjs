"use strict";
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

function script(name) {
  return fs.readFileSync(
    path.join(__dirname, "../crates/pipboy/assets", name), "utf8");
}
class Element {
  constructor(tag) {
    this.tag=tag; this.children=[]; this.listeners={}; this.attrs={};
    this.hidden=false; this.isConnected=true; this.textContent="";
    this.className=""; this.files=[]; this.style={}; this.value="";
    this.ctx={setTransform(){},clearRect(){},fillRect(){},beginPath(){},
      moveTo(){},lineTo(){},stroke(){},fillText(){},arc(){},fill(){},closePath(){}};
  }
  append(...xs){this.children.push(...xs);}
  replaceChildren(...xs){this.children=xs;}
  setAttribute(k,v){this.attrs[k]=v;}
  addEventListener(k,cb){(this.listeners[k] ||= []).push(cb);}
  getBoundingClientRect(){return{width:480,height:320,left:0,top:0};}
  getContext(){return this.ctx;}
  setPointerCapture(){}
  focus(){}
  click(){}
}
const document={createElement:tag=>new Element(tag)};
const window={
  devicePixelRatio:1, isSecureContext:true,
  addEventListener(){}, removeEventListener(){},
  requestAnimationFrame(cb){cb();}
};
const navigator={onLine:false,geolocation:{
  watchPosition(){throw new Error("GPS must remain opt-in");},
  clearWatch(){throw new Error("No automatic GPS watch");}
}};
const requests=[];
async function fetchMock(...args){requests.push(args);throw Error("No external map calls permitted");}
new Function("window",script("boston-base.js"))(window);
new Function("window",script("boston-map.js"))(window);
new Function("window","document","navigator","TextEncoder","fetch",script("map.js"))(
  window, document, navigator, TextEncoder, fetchMock);

async function main(){
  const source = window.RustBoyBostonBase;
  assert.equal(source.schema,1);
  assert.equal(source.source.source_blob_sha,"a86dc5f6913987585e429e625bdc61781d03d40e");
  assert.equal(source.roads.length,10634);
  assert.equal(source.names.length,2522);
  const decoded=window.RustBoyBoston.load();
  assert.equal(decoded.featureCount,10634);
  assert.equal(decoded.vertexCount,47994);
  const original=decoded.shapes.find(x=>x.sourceId===1);
  assert.deepEqual(original.paths[0].map(([lon,lat])=>[
    +lon.toFixed(6),+lat.toFixed(6)]),[
      [-71.056735,42.340774],[-71.056325,42.341115]
    ],"Source segment 1 must retain true real-world line vertices");

  const view=window.RustBoyMap.mount(new Element("div"));
  assert.ok(view.boston);
  assert.equal(view.baseVisible,true,"Boston real road geometry loads by default");
  assert.equal(view.center.lat,42.356);
  assert.equal(view.center.lon,-71.065);
  assert.equal(view.zoom,11.65);
  assert.equal(view.baseAttribution.hidden,false);
  assert.match(view.sourceLines.textContent,/City of Boston \/ SAM/);
  assert.match(view.status.textContent,/BOSTON \/ REAL STREETS/);
  assert.equal(requests.length,0,"Built-in map must never contact provider");
  assert.equal(view.fix,null,"Map center is not a GPS position");
  assert.equal(view.shapes.length,0,"Bundled reference must not impersonate imported GIS");

  view.baseButton.listeners.click[0]();
  assert.equal(view.baseVisible,false,"Operator can hide actual Boston basemap");
  assert.equal(view.baseAttribution.hidden,true);
  view.baseButton.listeners.click[0]();
  assert.equal(view.baseVisible,true);
  const cityLine=decoded.shapes.find(x=>x.sourceId===2);
  const fixture={type:"FeatureCollection",features:[{type:"Feature",
    properties:{name:"Imported official test"},
    geometry:{type:"LineString",coordinates:cityLine.paths[0]}}]};
  const encoded=JSON.stringify(fixture);
  view.file.files=[{name:"sam-snippet.geojson",size:Buffer.byteLength(encoded),text:async()=>encoded}];
  await view.file.listeners.change[0]();
  assert.equal(view.shapes.length,1,"Imported local GIS stays a separate top layer");
  assert.equal(view.baseVisible,true);
  assert.equal(requests.length,0,"Importing vector geometry must not initiate web lookup");
  assert.equal(view.fix,null,"Map import must never claim GPS");
  window.RustBoyMap.unmount();
  const restored=window.RustBoyMap.mount(new Element("div"));
  assert.equal(restored.baseVisible,true);
  assert.equal(restored.shapes.length,1,"User-imported geometry retained in page memory");
  assert.equal(restored.fix,null);
  window.RustBoyMap.unmount();

  const untrusted=JSON.parse(JSON.stringify(source));
  untrusted.roads[0][3]=9000000;
  const badWindow={RustBoyBostonBase:untrusted};
  new Function("window",script("boston-map.js"))(badWindow);
  assert.throws(()=>badWindow.RustBoyBoston.load(),/outside verified extract/);
  assert.equal(requests.length,0);
  console.log("Boston geometry: pinned real roads, offline default, overlays, toggles, source and anti-fabrication PASS");
}
main().catch(err=>{console.error(err);process.exitCode=1;});
