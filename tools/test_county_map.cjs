"use strict";
const assert=require("node:assert/strict");
const fs=require("node:fs");
const path=require("node:path");
function code(name){return fs.readFileSync(path.join(__dirname,"../crates/pipboy/assets",name),"utf8");}
const paint={coast:0,clipped:0,fill:0,dashed:0};
class Element{
  constructor(tag){
    this.tag=tag;this.listeners={};this.children=[];this.attrs={};
    this.isConnected=true;this.hidden=false;this.textContent="";this.value="";
    this.className="";this.files=[];this.style={};
    this.classList={add(name){this.className+=" "+name;}};
    this.ctx={setTransform(){},clearRect(){},fillRect(){},beginPath(){},moveTo(){},
      lineTo(){paint.coast++;},stroke(){},fill(){paint.fill++;},arc(){},closePath(){},
      fillText(){},save(){},restore(){},clip(){paint.clipped++;},
      setLineDash(a){if(a.length)paint.dashed++;}};
  }
  append(...children){this.children.push(...children);}
  replaceChildren(...children){this.children=children;}
  setAttribute(key,value){this.attrs[key]=value;}
  addEventListener(name,cb){(this.listeners[name] ||= []).push(cb);}
  getBoundingClientRect(){return {width:480,height:320,left:0,top:0};}
  getContext(){return this.ctx;}
  setPointerCapture(){}
  click(){}
  focus(){}
}
const window={isSecureContext:true,devicePixelRatio:1,
  requestAnimationFrame(cb){cb();},addEventListener(){},removeEventListener(){}};
let gpsRequested=0;
const navigator={onLine:true,geolocation:{
  getCurrentPosition(ok){gpsRequested++;ok({timestamp:Date.now(),
    coords:{latitude:42.36,longitude:-71.05,accuracy:9}});},
  watchPosition(){throw Error("Continuous GPS watch must not start");},
  clearWatch(){throw Error("No watch enabled");}
}};
const document={createElement(tag){return new Element(tag);}};
const county={type:"Feature",properties:{GEOID:"25025",NAME:"Suffolk",STATE:"25",COUNTY:"025"},
  geometry:{type:"Polygon",coordinates:[[[-71.2,42.1],[-70.8,42.1],
    [-70.8,42.6],[-71.2,42.6],[-71.2,42.1]]]}};
const line=(name,coords)=>({type:"Feature",properties:{NAME:name,MTFCC:"S1100"},
  geometry:{type:"LineString",coordinates:coords}});
let invalid=false,forceError=false;
const urls=[];
const features={
  "/Transportation/MapServer/2/":[line("HIGHWAY",[[-71.3,42.3],[-70.7,42.3]])],
  "/Transportation/MapServer/6/":[line("ROUTE",[[-71.15,42.2],[-71.12,42.3]])],
  "/Transportation/MapServer/8/":[line("STREET",[[-71.08,42.33],[-71.07,42.32]]),
    line("BEYOND COUNTY",[[-71.35,42.4],[-71.33,42.45]])],
  "/Transportation/MapServer/9/":[line("RAIL",[[-71.1,42.35],[-71.09,42.36]])],
  "/Hydro/MapServer/0/":[line("HARBOR INLET",[[-71.12,42.25],[-71.11,42.3]])],
  "/Hydro/MapServer/1/":[{
    type:"Feature",properties:{NAME:"WATER"},
    geometry:{type:"Polygon",coordinates:[[[-71.23,42.3],[-71.0,42.3],
      [-71.0,42.4],[-71.23,42.4],[-71.23,42.3]]]}}]
};
async function fetchMock(url,options={}){
  urls.push(url);
  if(forceError)throw Error("Mock provider offline");
  assert.match(url,/^https:\/\/tigerweb\.geo\.census\.gov\/arcgis\/rest\/services\/TIGERweb\//);
  assert.equal(options.credentials,"omit");
  assert.equal(options.referrerPolicy,"no-referrer");
  const u=new URL(url);
  const key=Object.keys(features).find(k=>u.pathname.endsWith(k+"query"));
  const body=key ? {type:"FeatureCollection",features:features[key]} :
    {type:"FeatureCollection",features:invalid?[]:[county]};
  return {ok:true,status:200,headers:{get:()=>null},
    text:async()=>JSON.stringify(body)};
}
new Function("window",code("boston-base.js"))(window);
new Function("window",code("boston-map.js"))(window);
new Function("window",code("boston-coast-base.js"))(window);
new Function("window",code("boston-land-base.js"))(window);
new Function("window","fetch",code("county-map.js"))(window,fetchMock);
new Function("window","document","navigator","TextEncoder","fetch",code("map.js"))(
  window,document,navigator,TextEncoder,fetchMock);
async function until(fn){
  for(let n=0;n<120;n++){if(fn())return;await new Promise(r=>setImmediate(r));}
  throw Error("Timeout waiting for county controls");
}
async function main(){
  const coast=window.RustBoyCoastBase,land=window.RustBoyBostonLandBase;
  assert.equal(coast.schema,1);
  assert.equal(coast.sources.county.fips,"25025");
  assert.ok(coast.coastline.flat().length>100,"Verified coastline has real points");
  assert.equal(land.schema,1);
  assert.equal(land.polygons.length,34);
  assert.ok(land.polygons.every(p=>p.every(r=>r.length>=4 &&
    r[0][0]===r.at(-1)[0]&&r[0][1]===r.at(-1)[1])));
  const view=window.RustBoyMap.mount(new Element("div"));
  assert.equal(urls.length,0,"Opening map may not contact Census or Overpass");
  assert.equal(view.center.lon,-71.065);
  assert.equal(view.fix,null);
  assert.ok(paint.coast>0 && paint.fill>0 && paint.dashed>0,
    "Real land, coast and old Suffolk boundary render offline");
  assert.equal(view.countyPanel.hidden,true);
  view.countyButton.listeners.click[0]();
  assert.equal(view.countyPanel.hidden,false);
  assert.equal(urls.length,0,"Showing county selector does not transmit position");
  view.countyLat.value="42.36";
  view.countyLon.value="-71.05";
  view.countyPanel.listeners.submit[0]({preventDefault(){}});
  await until(()=>view.county?.id==="25025"&&!view.countyBusy);
  assert.equal(view.county.name,"Suffolk");
  assert.equal(urls.length,7,"One county resolver + six bounded real geometry queries");
  assert.equal(view.county.year,2026);
  assert.ok(view.county.shapes.length>=5);
  assert.equal(view.onlineShapes.length,0);
  assert.ok(paint.clipped>0,"Only county polygon is visible under county clipping");
  const highway=view.county.shapes.find(s=>s.label==="HIGHWAY");
  assert.ok(highway,"Verified road is present");
  assert.ok(Math.abs(highway.paths[0][0][0]+71.2)<1e-6);
  assert.ok(Math.abs(highway.paths[0].at(-1)[0]+70.8)<1e-6);
  assert.ok(!view.county.shapes.some(s=>s.label==="BEYOND COUNTY"));
  const reqCount=urls.length;
  view.webButton.listeners.click[0]();
  assert.equal(view.webEnabled,false,"County context cannot silently enable Overpass");
  assert.equal(urls.length,reqCount);
  view.countyButton.listeners.click[0]();
  assert.equal(view.countyPanel.hidden,false);
  forceError=true;
  view.countyLat.value="42.37";
  view.countyLon.value="-71.01";
  view.countyPanel.listeners.submit[0]({preventDefault(){}});
  await until(()=>!view.countyBusy);
  assert.equal(view.county.id,"25025","Failed replacement preserves prior county");
  assert.match(view.countyStatus.textContent,/FAILED/);
  forceError=false;
  view.countyGPS.listeners.click[0]();
  await until(()=>gpsRequested===1&&view.countyLoads===3);
  assert.equal(view.fix,null,"One-shot GPS county request does not start position tracking");
  view.countyButton.listeners.click[0]();
  view.countyBack.listeners.click[0]();
  assert.equal(view.county,null);
  assert.equal(view.center.lon,-71.065);
  assert.equal(view.baseVisible,true);
  window.RustBoyMap.unmount();
  const restored=window.RustBoyMap.mount(new Element("div"));
  assert.equal(restored.county,null);
  assert.equal(restored.fix,null);
  window.RustBoyMap.unmount();
  const direct=window.RustBoyCountyMap;
  assert.throws(()=>direct.regionPolygons({type:"Point",coordinates:[1,1]}),
    /COUNTY BOUNDARY/);
  assert.throws(()=>direct.boundsOf([[[[-72,35],[-69,35],[-69,40],[-72,40]]]]),
    /COUNTY TOO LARGE/);
  assert.ok(direct.inCounty([-71,42.4],[county.geometry.coordinates]));
  assert.equal(direct.inCounty([-71.5,42.4],[county.geometry.coordinates]),false);
  console.log("County geometry: offline Boston shore, real 2026 county, manual/GPS, clipped roads, atomic failures PASS");
}
main().catch(e=>{console.error(e);process.exitCode=1;});
