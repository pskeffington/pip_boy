"use strict";
const assert = require("node:assert/strict");
const fs = require("node:fs"), path = require("node:path"), vm = require("node:vm");
class E {
  constructor(tag) {
    this.tag = tag; this.children = []; this.listeners = {};
    this.attrs = {}; this.className = "";
    this.classList = {add: () => {}};
    if (tag === "img") createdImages.push(this);
  }
  append(...items) { this.children.push(...items); }
  replaceChildren(...items) { this.children = items; }
  setAttribute(k, v) { this.attrs[k] = v; }
  removeAttribute(k) { delete this.attrs[k]; this.src = ""; }
  addEventListener(k, fn) { this.listeners[k] = fn; }
  get textContent() { return (this.text || "") + this.children.map(c => c.textContent).join(" "); }
  set textContent(value) { this.text = String(value); this.children = []; }
}
const createdImages = [], timers = new Map();
let nextTimer = 1;
const setTimeoutFake = (fn, delay) => { const id = nextTimer++; timers.set(id, {fn, delay}); return id; };
const clearTimeoutFake = id => timers.delete(id);
const document = {createElement: tag => new E(tag)};
const window = {}, root = new E("div");
const context = vm.createContext({document, window, URL, Date, setTimeout: setTimeoutFake, clearTimeout: clearTimeoutFake});
vm.runInContext(fs.readFileSync(path.join(__dirname, "../crates/pipboy/assets/cameras.js"), "utf8"), context);
const cameras = window.RustBoyCameras;
const walk = node => [node, ...node.children.flatMap(walk)];
const text = () => walk(root).map(n => n.textContent).join(" ");
const buttons = () => walk(root).filter(n => n.tag === "button");
const primaryTitle = () => walk(root).find(n => n.className.includes("rb-camera-primary"))?.textContent || "";
const signal = (image, event) => assert.ok(image.listeners[event], event + " event available") || image.listeners[event]();
function feed(id, lat, lon) {
  return {id, city: id.toUpperCase(), name: id + " road camera", publisher: "VDOT",
    source_url: "https://511.vdot.virginia.gov/",
    image_url: "https://snapshot.vdotcameras.com/thumbs/" + id + ".flv.png",
    lat, lon, region: "VA", format: "jpeg_or_png_snapshot"};
}
const origin = {lat:40.75, lon:-73.99, source:"manual"};
const near = feed("near", 40.751, -73.989);
const medium = feed("medium", 40.85, -73.8);
const far = feed("far", 42.36, -71.06);
const other = feed("other", 38.87, -77.15);
const gated = {...feed("gated",40.751,-73.991), image_url:"",format:"unavailable"};
const invalid = {...near,id:"untrusted",image_url:"https://evil.example/thumbs/near.flv.png"};
assert.equal(cameras.distanceMiles(origin, origin), 0);
assert.ok(cameras.distanceMiles(origin, near) < cameras.distanceMiles(origin, medium));
assert.deepEqual(Array.from(cameras.rankFeeds([far,gated,other,near,medium,invalid],origin)).map(f=>f.id),
  ["near","medium","far","other","gated"],"Distance order, gated sources last, hostile URLs excluded");
assert.equal(cameras.pickNearestActive([near,medium,far],{near:"reachable",medium:"live",far:"live"}).id,
  "medium","A verified live feed outranks a nearer unverified image");
assert.equal(cameras.pickNearestActive([near,medium],{near:"offline",medium:"reachable"}).id,
  "medium","Offline camera skipped");

let selected = null;
let controller = cameras.mount(root,{feeds:[far,other,near,medium,gated,invalid],
  location:origin,onSelect:id=>selected=id});
assert.equal(createdImages.length, 2,"At most two parallel media probes, only on camera view open");
assert.equal(walk(root).filter(n=>n.tag==="iframe").length,0,"No YouTube or iframe");
assert.ok(createdImages.every(i=>i.src.startsWith("https://snapshot.vdotcameras.com/thumbs/")));
assert.ok(text().includes("CHECKING NEAREST CAMERA"));
signal(createdImages[0],"error");
assert.equal(createdImages.length,3,"Failed nearest camera advances to next candidate");
signal(createdImages[1],"load");
assert.ok(primaryTitle().includes("MEDIUM"),"Nearest reachable candidate chosen after first failure");
assert.ok(text().includes("FRESHNESS UNVERIFIED"),"Loaded image is not falsely described as live");
const focus = buttons().find(b=>b.textContent === "FOCUS NEAREST");
assert.ok(focus,"A clear focus action targets automatically selected camera");
focus.listeners.click();
assert.equal(selected,"medium","Selection persists by camera ID instead of mutable grid index");
assert.equal(walk(root).filter(n=>n.className.includes("rb-camera-tile")).length,1);
signal(createdImages[2],"load");
const staleCheck=[...timers.values()].find(t=>t.delay>=119000 && t.delay<=120000);
assert.ok(staleCheck,"A successful source check expires");
staleCheck.fn();
assert.equal(selected,"far","Focused source fails over to the next available camera");
assert.ok(primaryTitle().includes("FAR"),"Expired source is no longer primary");
controller.destroy();
assert.equal(root.children.length,0);
assert.equal(timers.size,0,"Destroy cancels probe and expiry timers");
createdImages[1].listeners.load();
assert.equal(root.children.length,0,"Late media events cannot resurrect destroyed view");

createdImages.length=0;
const now=new Date();
const live={...far,active:true,
  status_checked_at:now.toISOString(),frame_captured_at:now.toISOString()};
controller=cameras.mount(root,{feeds:[near,live],location:origin,compact:true});
assert.equal(createdImages.length,2,"Compact mode probes when Cameras explicitly opens");
signal(createdImages[0],"load");
signal(createdImages[1],"load");
assert.ok(primaryTitle().includes("FAR"),"Fresh agency-evidenced LIVE outranks nearer reachable");
assert.ok(text().includes("AGENCY FRAME TIMESTAMP VERIFIED"));
controller.destroy();

createdImages.length=0;
const inactive={...near,active:false};
const stale={...medium,active:true,
  status_checked_at:now.toISOString(),frame_captured_at:new Date(now-600000).toISOString()};
controller=cameras.mount(root,{feeds:[inactive,stale,far],location:origin});
assert.equal(createdImages.length,1,"Inactive and agency-stale feeds never automatically probed");
assert.ok(text().includes("AGENCY INACTIVE"));
assert.ok(text().includes("AGENCY FRAME STALE"));
controller.destroy();

createdImages.length=0;
controller=cameras.mount(root,{feeds:[invalid,gated],location:origin});
assert.equal(createdImages.length,0,"Blocked/private camera metadata causes no requests");
assert.ok(text().includes("PROVIDER ACCESS REQUIRED"));
controller.destroy();
console.log("PASS cameras: nearest live prioritization, bounded probes, offline/stale failover, manual focus, teardown, no YouTube");
