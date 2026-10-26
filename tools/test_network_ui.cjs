"use strict";

// Dependency-free component journeys. API responses are controlled fixtures;
// this harness never scans a LAN or contacts a controller.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const root = path.resolve(__dirname, "..");

class Element {
  constructor(tag) {
    this.tagName = tag;
    this.children = [];
    this.events = new Map();
    this.attributes = {};
    this.className = "";
    this.value = "";
    this.text = "";
    this.classList = { add: (name) => { this.className += " " + name; } };
  }
  set textContent(value) { this.text = String(value); this.children = []; }
  get textContent() { return this.text + this.children.map((item) => item.textContent).join(" "); }
  append(...items) { this.children.push(...items); }
  replaceChildren(...items) {
    if (this.ownerDocument && walk(this).includes(this.ownerDocument.activeElement)) {
      this.ownerDocument.activeElement = this.ownerDocument.body;
    }
    this.text = "";
    this.children = [...items];
  }
  contains(item) { return walk(this).includes(item); }
  querySelector(selector) {
    if (!selector.startsWith(".")) return null;
    const className = selector.slice(1);
    return walk(this).find((item) => item.className.split(/\\s+/).includes(className)) || null;
  }
  querySelectorAll(selector) {
    const tags = new Set(selector.split(",").map((item) => item.trim().toLowerCase()));
    return walk(this).filter((item) => tags.has(item.tagName));
  }
  setAttribute(key, value) { this.attributes[key] = value; }
  addEventListener(name, callback) { this.events.set(name, callback); }
  removeEventListener(name) { this.events.delete(name); }
  dispatch(name) { return this.events.get(name)?.({ preventDefault() {} }); }
}

function walk(item) { return [item, ...item.children.flatMap(walk)]; }
function find(item, predicate) {
  const found = walk(item).find(predicate);
  assert.ok(found, "Missing control in: " + item.textContent);
  return found;
}
function button(item, text) {
  return find(item, (element) => element.tagName === "button" && element.textContent.startsWith(text));
}
function form(item) { return find(item, (element) => element.tagName === "form"); }
const flush = () => new Promise(setImmediate);
function pageKey(env, key, options = {}) {
  let prevented = false;
  const handler = env.document.events.get("keydown");
  assert.ok(handler, "Missing scoped physical-button / arrow handler");
  handler({ key, target:env.document.activeElement,
    repeat:false, altKey:false, ctrlKey:false, metaKey:false,
    ...options, preventDefault() { prevented = true; } });
  return prevented;
}

const styles = fs.readFileSync(path.join(root, "crates/pipboy/assets/network.css"), "utf8");
assert.match(styles, /\.net-pager \.net-page-arrow\s*\{[^}]*min-height:48px/s,
  "Paging buttons must offer at least 48px height");
assert.match(styles, /\.net-compact \.net-button\s*\{[^}]*min-height:44px/s,
  "Handheld tool controls must retain a usable touch target");

const source = fs.readFileSync(path.join(root, "crates/pipboy-core/src/network_suite.rs"), "utf8");
const operations = [...source.matchAll(/Operation \{\s+id: "([^"]+)",\s+name: "([^"]+)",\s+input_label: "([^"]*)",\s+example: "([^"]*)",\s+extra_label: (None|Some\("[^"]*"\)),\s+extra_example: (None|Some\("[^"]*"\)),\s+active: (true|false),/g)].map((match) => ({
  id: match[1], name: match[2], input_label: match[3], example: match[4],
  extra_label: match[5] === "None" ? null : match[5].slice(6, -2),
  extra_example: match[6] === "None" ? null : match[6].slice(6, -2),
  active: match[7] === "true"
}));
assert.equal(operations.length, 16);
assert.equal(operations.filter((op) => !op.active).length, 9);
assert.equal(new Set(operations.map((op) => op.id)).size, operations.length);

const fixture = {
  kind: "network_scan", output: "Responders are observations, not all connected devices.",
  report: {
    responding_devices: 4, cached_only_devices: 1,
    devices: Array.from({ length: 5 }, (_, index) => ({
      address: "192.168.1." + (index + 1), evidence: index === 4 ? "neighbor_cache" : "icmp_echo",
      latency_ms: index === 4 ? null : index + 1
    }))
  }
};

function environment() {
  const timers = new Map();
  const calls = [];
  const document = new Element("document");
  document.createElement = (tag) => {
    const element = new Element(tag);
    element.ownerDocument = document;
    element.focus = () => { document.activeElement = element; };
    return element;
  };
  document.body = document.createElement("body");
  document.activeElement = document.body;
  document.hidden = false;
  const window = new Element("window");
  let timerId = 0;
  window.setTimeout = (callback, delay) => {
    assert.equal(delay, 60000);
    timers.set(++timerId, callback);
    return timerId;
  };
  window.clearTimeout = (id) => timers.delete(id);
  let nextResponse = () => Promise.resolve({ ok: true, json: async () => fixture });
  const context = vm.createContext({
    window, document, AbortController,
    FormData: class {
      constructor(element) { this.element = element; }
      *[Symbol.iterator]() {
        for (const element of walk(this.element)) {
          if (element.name) yield [element.name, element.value];
        }
      }
    },
    fetch: (url, options) => {
      calls.push({ url, body: JSON.parse(options.body), signal: options.signal });
      return nextResponse();
    }
  });
  vm.runInContext(fs.readFileSync(path.join(root, "crates/pipboy/assets/network.js"), "utf8"), context);
  const container = new Element("div");
  const ui = window.RustBoyNetwork.mount(container, { operations, compact: true });
  return { container, ui, timers, calls, document, window,
    response: (callback) => { nextResponse = callback; } };
}

function choose(env, label) { button(env.container, label).dispatch("click"); }
function authorize(env, repeat) {
  form(env.container).dispatch("submit");
  const checks = walk(env.container).filter((element) => element.type === "checkbox");
  // Form submission alone is insufficient; explicit checkbox consent is required.
  form(env.container).dispatch("submit");
  assert.equal(env.calls.length, 0);
  checks[0].checked = true;
  if (checks[1]) checks[1].checked = repeat;
  form(env.container).dispatch("submit");
}

async function main() {
  const env = environment();
  assert.equal(env.calls.length, 0, "Mounting made an unsolicited request");
  assert.match(env.container.textContent, /SESSION STANDBY/);
  assert.match(env.container.textContent, /REQUESTS MANUAL ONLY/);
  assert.match(env.container.textContent, /LAST RESULT \/ NOT LIVE NONE/);
  assert.match(env.container.textContent, /PRIVATE LAN DISCOVERY/);
  assert.match(env.container.textContent, /AUTHORIZATION REQUIRED/);
  assert.match(env.container.textContent, /PAGE 1 \/ 4/);
  const menuPager = find(env.container, (item) => item.tagName === "nav");
  assert.equal(menuPager.attributes["aria-label"], "Network tool pages");
  assert.equal(button(env.container, "◀ PREV").disabled, true);
  assert.equal(button(env.container, "NEXT ▶").disabled, false);
  assert.equal(pageKey(env, "ArrowLeft"), false, "First page has no previous page");
  assert.equal(pageKey(env, "ArrowRight", { repeat:true }), false, "Held D-pad should not rapidly skip pages");
  assert.equal(pageKey(env, "ArrowRight",
    { target:env.document.createElement("input") }), false, "Input arrow editing must not turn pages");
  assert.equal(pageKey(env, "ArrowRight"), true);
  assert.match(env.container.textContent, /PAGE 2 \/ 4/);
  assert.match(env.container.textContent, /PING \/ JITTER/);
  assert.equal(env.calls.length, 0, "Physical-button paging must not issue a network request");
  assert.equal(pageKey(env, "ArrowLeft"), true);
  assert.match(env.container.textContent, /PAGE 1 \/ 4/);
  assert.equal(env.calls.length, 0);

  choose(env, "SCAN LAN");
  authorize(env, true);
  await flush();
  assert.equal(env.calls.length, 1);
  assert.equal(env.calls[0].url, "/api/net/scan");
  assert.deepEqual(env.calls[0].body, { cidr: "local", confirm: true });
  assert.match(env.container.textContent, /4 RESPONDING \/ 1 CACHED/);
  assert.ok(!env.container.textContent.includes("192.168.1.3"));
  button(env.container, "NEXT ▶").dispatch("click");
  assert.match(env.container.textContent, /192\.168\.1\.3/);
  assert.equal(pageKey(env, "ArrowLeft"), true);
  assert.ok(!env.container.textContent.includes("192.168.1.3"));
  assert.equal(pageKey(env, "ArrowRight"), true);
  assert.match(env.container.textContent, /192\.168\.1\.3/);
  assert.equal(env.calls.length, 1, "Paging generated a new network request");
  assert.equal(env.timers.size, 1);
  button(env.container, "STOP REFRESH").dispatch("click");
  assert.equal(env.timers.size, 0);
  assert.match(env.container.textContent, /REFRESH STOPPED/);
  assert.equal(env.ui.back(), true, "Result Back should open preserved inputs");
  assert.equal(find(env.container, (item) => item.name === "input").value, "local");
  assert.equal(env.ui.back(), true, "Input Back should open tool menu");
  assert.equal(env.ui.back(), false, "Menu Back should delegate to shell");
  assert.equal(env.timers.size, 0);
  env.ui.destroy();
  assert.equal(env.ui.back(), false);
  assert.equal(env.document.events.size, 0);
  assert.equal(env.window.events.size, 0);

  const pad = environment();
  assert.equal(pageKey(pad, "ArrowDown"), true, "D-pad down should focus first tool");
  assert.match(pad.document.activeElement.textContent, /SCAN LAN/);
  assert.equal(pageKey(pad, "ArrowDown"), true);
  assert.match(pad.document.activeElement.textContent, /UNIFI SITES/);
  assert.equal(pageKey(pad, "ArrowDown"), true);
  assert.match(pad.document.activeElement.textContent, /UNIFI CLIENTS/);
  assert.equal(pageKey(pad, "ArrowUp"), true);
  assert.match(pad.document.activeElement.textContent, /UNIFI SITES/);
  assert.equal(pageKey(pad, "ArrowRight"), true, "D-pad right should turn the page");
  assert.match(pad.container.textContent, /PAGE 2 \/ 4/);
  assert.match(pad.document.activeElement.textContent, /PING \/ JITTER/);
  assert.equal(pageKey(pad, "PageDown"), true, "Hardware page-down should turn page");
  assert.match(pad.container.textContent, /PAGE 3 \/ 4/);
  assert.match(pad.document.activeElement.textContent, /SUBNET SPLIT/);
  assert.equal(pageKey(pad, "PageDown"), true);
  assert.match(pad.container.textContent, /PAGE 4 \/ 4/);
  assert.equal(pageKey(pad, "PageDown"), false, "Last page is bounded");
  assert.equal(pageKey(pad, "PageUp"), true);
  assert.match(pad.container.textContent, /PAGE 3 \/ 4/);
  assert.equal(pageKey(pad, "ArrowDown"), true);
  assert.match(pad.document.activeElement.textContent, /ROUTE MATCH/);
  assert.equal(pad.calls.length, 0, "D-pad should never execute an active network operation");
  // Native Enter activates a focused button. The fixture simulates that click.
  pad.document.activeElement.dispatch("click");
  assert.equal(pad.calls.length, 0, "Opening a tool should not run it");
  assert.equal(pageKey(pad, "ArrowDown"), true);
  assert.match(pad.document.activeElement.textContent, /^TOOLS$/);
  assert.equal(pageKey(pad, "ArrowDown"), true);
  assert.equal(pad.document.activeElement.tagName, "input");
  assert.equal(pageKey(pad, "ArrowDown"), false, "D-pad must not steal text input editing");
  assert.equal(pageKey(pad, "PageDown"), false, "Input must not be paged");
  pad.ui.destroy();

  const confirmation = environment();
  choose(confirmation, "SCAN LAN");
  form(confirmation.container).dispatch("submit");
  assert.equal(confirmation.calls.length, 0);
  assert.equal(pageKey(confirmation, "ArrowDown"), true);
  assert.match(confirmation.document.activeElement.textContent, /^EDIT$/);
  assert.equal(pageKey(confirmation, "ArrowDown"), true);
  assert.equal(confirmation.document.activeElement.type, "checkbox");
  assert.equal(pageKey(confirmation, "ArrowDown"), true,
    "Checkbox does not trap navigation toward RUN");
  assert.equal(pageKey(confirmation, "ArrowDown"), true);
  assert.match(confirmation.document.activeElement.textContent, /RUN SCAN LAN/);
  assert.equal(confirmation.calls.length, 0, "Moving focus across consent cannot scan");
  form(confirmation.container).dispatch("submit");
  assert.equal(confirmation.calls.length, 0, "Focus is not authorization");
  confirmation.ui.destroy();

  const refresh = environment();
  choose(refresh, "SCAN LAN");
  authorize(refresh, true);
  await flush();
  const [id, callback] = [...refresh.timers][0];
  refresh.timers.delete(id);
  callback();
  await flush();
  assert.equal(refresh.calls.length, 2);
  assert.equal(refresh.timers.size, 1);
  refresh.document.hidden = true;
  refresh.document.dispatch("visibilitychange");
  assert.equal(refresh.timers.size, 0, "Hidden tab continued monitoring");
  refresh.document.hidden = false;
  refresh.document.dispatch("visibilitychange");
  assert.equal(refresh.calls.length, 2, "Visible tab restarted without operator action");
  refresh.ui.destroy();

  const stale = environment();
  let resolveRequest;
  stale.response(() => new Promise((resolve) => { resolveRequest = resolve; }));
  choose(stale, "SCAN LAN");
  authorize(stale, true);
  stale.ui.destroy();
  assert.equal(stale.calls[0].signal.aborted, true);
  resolveRequest({ ok: true, json: async () => fixture });
  await flush();
  assert.equal(stale.timers.size, 0, "Destroyed request scheduled a refresh");
  assert.ok(!stale.container.textContent.includes("4 RESPONDING"), "Late response repainted destroyed view");

  const offline = environment();
  button(offline.container, "NEXT ▶").dispatch("click");
  choose(offline, "IPv6 CIDR");
  form(offline.container).dispatch("submit");
  await flush();
  assert.equal(offline.calls.length, 1);
  assert.equal(offline.calls[0].url, "/api/experiment");
  assert.equal(offline.calls[0].body.mode, "cidr6");
  assert.equal(offline.calls[0].body.confirm, undefined);
  offline.ui.destroy();

  const unifi = environment();
  choose(unifi, "UNIFI CLIENTS");
  find(unifi.container, (element) => element.name === "input").value = "00000000-0000-0000-0000-000000000001";
  unifi.response(() => Promise.resolve({ ok: true, json: async () => ({
    kind: "unifi_clients", output: "Read only", inventory: { connected_clients: 1, clients_with_ip: 1,
      clients: [{ name: "<img src=x>", ip_address: "192.168.1.10", connection_type: "WIRED" }] }
  }) }));
  authorize(unifi, false);
  await flush();
  assert.equal(unifi.calls[0].url, "/api/net/unifi");
  assert.equal(unifi.calls[0].body.operation, "clients");
  assert.match(unifi.container.textContent, /<img src=x>/);
  assert.ok(!walk(unifi.container).some((item) => item.tagName === "img"), "Controller label became HTML");
  unifi.ui.destroy();

  const failed = environment();
  failed.response(() => Promise.resolve({ ok: false, json: async () => ({ error: "Controller unavailable" }) }));
  choose(failed, "SCAN LAN");
  authorize(failed, true);
  await flush();
  assert.match(failed.container.textContent, /Controller unavailable/);
  assert.equal(failed.timers.size, 0, "Failed request kept auto-retrying");
  failed.ui.destroy();
  console.log("PASS: touch targets, D-pad focus and page keys, consent, refresh, visibility, cancellation and safe labels");
}
main().catch((error) => { console.error(error); process.exitCode = 1; });
