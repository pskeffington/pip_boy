"use strict";
// Production handheld journeys with an invented API snapshot. No requests leave
// this process; browser geometry and physical touch acceptance are separate.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const assets = path.resolve(__dirname, "../crates/pipboy/assets");
const walk = node => [node, ...node.children.flatMap(walk)];
class Element {
  constructor(tag, document) {
    this.tagName = tag; this.ownerDocument = document; this.children = [];
    this.attributes = {}; this.dataset = {}; this.events = new Map();
    this.className = ""; this.text = ""; this.clientHeight = 210;
    this.scrollHeight = 210; this.open = false;
    this.classList = {
      add: name => { this.className += " " + name; },
      remove: name => { this.className = this.className.split(" ").filter(x => x !== name).join(" "); },
      contains: name => this.className.split(" ").includes(name),
      toggle: (name, on) => on ? this.classList.add(name) : this.classList.remove(name)
    };
  }
  append(...nodes) { this.children.push(...nodes); }
  replaceChildren(...nodes) { this.children = nodes; }
  set textContent(text) { this.text = String(text); this.children = []; }
  get textContent() { return this.text + this.children.map(x => x.textContent).join(" "); }
  setAttribute(key, value) { this.attributes[key] = String(value); }
  removeAttribute(key) { delete this.attributes[key]; }
  addEventListener(key, callback) { this.events.set(key, callback); }
  click() { if (!this.disabled) this.events.get("click")?.(); }
  focus() { this.ownerDocument.activeElement = this; }
  showModal() { this.open = true; }
  close() { this.open = false; }
  querySelectorAll(selector) {
    const nodes = walk(this).slice(1);
    if (selector === "[data-role]") return nodes.filter(x => x.dataset.role);
    if (selector === ".micro-pager button") {
      return nodes.filter(x => x.className === "micro-pager").flatMap(x => x.children.filter(y => y.tagName === "button"));
    }
    if (selector.startsWith(".")) return nodes.filter(x => x.className.split(" ").includes(selector.slice(1)));
    return [];
  }
  querySelector(selector) { return this.querySelectorAll(selector)[0] || null; }
}
const ids = new Map();
const document = {
  events: new Map(), activeElement: null,
  getElementById(id) { if (!ids.has(id)) ids.set(id, new Element("div", this)); return ids.get(id); },
  createElement(tag) { return new Element(tag, this); },
  querySelectorAll() { return []; },
  addEventListener(key, callback) { this.events.set(key, callback); }
};
document.body = document.createElement("body");
const events = new Map(), requests = [];
const window = {innerHeight:320, addEventListener:(key, callback) => events.set(key, callback)};
const roleIds = ["byte-lab", "simulator", "geo-lab", "reclamation", "net-lab", "gpio", "ble", "nfc"];
const dataIds = ["notes", "library", "game"];
const toolIds = [...roleIds, ...dataIds, "host", "diag"];
const fixture = {
  profiles: Array.from({length:8}, (_, i) => ({id:i === 7 ? "custom" : "role-" + i, title:"Fixture role " + i, tools:toolIds})),
  palette:toolIds.map(id => ({id, section:roleIds.includes(id) ? "role" : dataIds.includes(id) ? "data" : "stat", name:id.toUpperCase(), availability:"ready", description:"Invented test tool"})),
  active:{profile:"role-0", slots:toolIds.map((tool, id) => ({id, tool}))}
};
const packageFixture = {
  catalog_state:"reference_designs_only",
  active_socket_package_id:null,
  packages:[
    {manifest:{name:"TV",package_id:"org.rustboy.tv"},installed:true,activatable:true,state:"registered"},
    {manifest:{name:"COMMS",package_id:"org.rustboy.comms"},installed:false,activatable:false},
    {manifest:{name:"MUSIC",package_id:"org.rustboy.music"},installed:false,activatable:false},
    {manifest:{name:"<img src=x>",package_id:"org.test.untrusted"},installed:false,activatable:false}
  ]
};
const context = vm.createContext({document, window, Element, console, Intl, Date,
  setTimeout:() => 1, clearTimeout() {}, setInterval() {},
  MutationObserver:class {observe() {}},
  fetch:async url => {
    requests.push(url);
    return {ok:true,json:async()=>url==="/api/packages"?packageFixture:url==="/api/packages/select"?{active_package_id:"org.rustboy.tv",view:{title:"TV",text:"Ready",items:["Channel 1"]}}:fixture};
  }
});
vm.runInContext(fs.readFileSync(path.join(assets, "micro.js"), "utf8"), context);
const run = code => vm.runInContext(code, context);
const content = document.getElementById("micro-content");
const buttons = () => content.querySelectorAll(".micro-pager button");
const tiles = () => content.querySelectorAll(".tile");
(async () => {
  await new Promise(setImmediate);
  assert.deepEqual(requests, ["/api/state"], "Startup must only read local state");
  assert.equal(run("toastTimer"), null, "Startup toast should not obscure tools");
  run('setTab("role")');
  assert.equal(tiles().length, 4);
  assert.equal(buttons()[0].disabled, true);
  buttons()[1].click();
  assert.equal(tiles()[0].textContent.includes("NET-LAB"), true);
  assert.equal(document.activeElement, buttons()[0], "Last page must retain focus on an enabled arrow");
  // Enlarging the screen retains the first visible tool's containing page.
  document.getElementById("micro-viewport").clientHeight = 330;
  events.get("resize")();
  assert.equal(tiles().length, 6);
  assert.equal(run("viewPage.role"), 0);
  document.getElementById("micro-viewport").clientHeight = 210;
  run('setTab("data")');
  assert.equal(tiles().length, 4, "DATA has a paged deck including shared actions");
  const seen = new Set();
  do {
    tiles().forEach(x => seen.add(x.attributes["aria-label"] || x.textContent));
    const next = buttons()[1];
    if (!next || next.disabled) break;
    next.click();
  } while (true);
  assert.equal(seen.size, dataIds.length + 2, "DATA tools, Messenger and duties must remain reachable");
  assert.deepEqual(Array.from(run("pages")), ["stat", "role", "data", "map", "radio"]);
  const categorized = ["stat", "role", "data", "radio"].flatMap(key => Array.from(run(`sectionTools("${key}")`)));
  assert.equal(categorized.length, toolIds.length, "Every pinned module belongs to exactly one section");
  assert.equal(new Set(categorized).size, toolIds.length, "Sections must not duplicate module tiles");
  let radioMounted = 0, radioDestroyed = 0;
  window.RustBoyRadio = {mount:() => {radioMounted++; return {destroy:() => radioDestroyed++};}};
  run('setTab("radio")');
  assert.equal(radioMounted, 1, "RADIO directly mounts native playback controls");
  run('setTab("stat")');
  assert.equal(radioDestroyed, 1, "Leaving RADIO cleans up its view controller");
  assert.equal(tiles().length, 2, "System tools belong under STATUS");
  document.getElementById("micro-settings").click();
  assert.equal(run("tab"), "conf", "Settings stays reachable outside the five-section rail");
  let locationMounted=0;
  window.RustBoyMap={unmount(){},mountSettings(){locationMounted++;}};
  const locationButton=walk(content).find(x=>x.textContent === "LOCATION / MAP");
  assert.ok(locationButton,"Universal Settings must expose location ingestion");
  locationButton.click();
  assert.equal(locationMounted,1,"Settings opens the shared location component");
  assert.equal(run("openedTool"),"location-settings");
  run("back()");
  assert.equal(run("tab"),"conf","Back returns from location setup to Settings");
  delete window.RustBoyMap;
  run('setTab("conf"); openRoles()');
  const roles = document.getElementById("roles-list");
  assert.equal(roles.children.length, 2);
  roles.children[1].click();
  const staged = run("pendingProfile");
  document.getElementById("role-next").click();
  assert.equal(run("pendingProfile"), staged, "Paging must not select a different role");
  assert.deepEqual(requests, ["/api/state"], "Browsing must never apply profiles or run tools");
  window.innerHeight = 480;
  events.get("resize")();
  assert.equal(roles.children.length, 4);
  assert.equal(run("pendingProfile"), staged);
  run('ui.rolesDialog.close(); snapshot.active.profile = "custom"; paletteOpen = true; setTab("conf")');
  assert.equal(content.querySelectorAll(".tool-pick").length, 2);
  run("back()");
  assert.equal(run("tab"), "conf");
  assert.equal(run("paletteOpen"), false, "Back must return to Settings");
  document.getElementById("micro-modules").click();
  await new Promise(setImmediate);
  const moduleDialog=document.getElementById("modules-dialog");
  const moduleRows=document.getElementById("modules-list").children;
  assert.equal(moduleDialog.open,true,"The persistent host Modules control opens its dialog");
  assert.deepEqual(requests,["/api/state","/api/packages"],"Opening Modules reads only the local package registry");
  assert.equal(moduleRows.length,4,"The switcher displays every returned registry entry");
  assert.ok(moduleRows[0].textContent.includes("org.rustboy.tv"));
  assert.ok(moduleRows[1].textContent.includes("org.rustboy.comms"));
  assert.ok(moduleRows[2].classList.contains("module-music-package"),"MUSIC has distinct phosphor styling");
  assert.equal(moduleRows[2].dataset.packageId,"org.rustboy.music");
  assert.ok(moduleRows[3].textContent.includes("<img src=x>"),"Registry labels render as inert text");
  assert.ok(moduleRows.slice(1).every(row=>row.children[2].disabled),"Reference packages never get an activation control");
  assert.equal(moduleRows[0].children[2].disabled,false,"A mounted host adapter gets a switch action");
  moduleRows[0].children[2].click();
  await new Promise(setImmediate);
  assert.deepEqual(requests,["/api/state","/api/packages","/api/packages/select"],"Switch uses the host lifecycle route");
  assert.equal(content.querySelector(".module-live-view").textContent.includes("Channel 1"),true,"Package view is rendered in the live shell");
  moduleRows[3].children[2].click();
  assert.deepEqual(requests,["/api/state","/api/packages","/api/packages/select"],"Untrusted package row cannot dispatch a request");
  document.getElementById("close-modules").click();
  assert.equal(moduleDialog.open,false,"The module switcher has a visible close action");
  // A resize must leave mounted tool editors intact.
  run('snapshot.active.profile = "role-0"; openTool("byte-lab")');
  const editor = content.children[0];
  events.get("resize")();
  assert.equal(content.children[0], editor);
  console.log("PASS micro: adaptive paging, complete tool access, focus, staged roles, safe resize, Back and silent startup");
})().catch(error => {console.error(error); process.exitCode = 1;});
