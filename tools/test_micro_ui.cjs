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
const context = vm.createContext({document, window, Element, console, Intl, Date,
  setTimeout:() => 1, clearTimeout() {}, setInterval() {},
  MutationObserver:class {observe() {}},
  fetch:async url => {requests.push(url); return {ok:true, json:async () => fixture};}
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
  // A resize must leave mounted tool editors intact.
  run('snapshot.active.profile = "role-0"; openTool("byte-lab")');
  const editor = content.children[0];
  events.get("resize")();
  assert.equal(content.children[0], editor);
  console.log("PASS micro: adaptive paging, complete tool access, focus, staged roles, safe resize, Back and silent startup");
})().catch(error => {console.error(error); process.exitCode = 1;});
