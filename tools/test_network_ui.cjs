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
  replaceChildren(...items) { this.text = ""; this.children = [...items]; }
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
  document.createElement = (tag) => new Element(tag);
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
  choose(env, "SCAN LAN");
  authorize(env, true);
  await flush();
  assert.equal(env.calls.length, 1);
  assert.equal(env.calls[0].url, "/api/net/scan");
  assert.deepEqual(env.calls[0].body, { cidr: "local", confirm: true });
  assert.match(env.container.textContent, /4 RESPONDING \/ 1 CACHED/);
  assert.ok(!env.container.textContent.includes("192.168.1.3"));
  button(env.container, "▶").dispatch("click");
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
  button(offline.container, "▶").dispatch("click");
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
  console.log("PASS: unified network UI consent, paging, refresh, visibility, cancellation, stale results and safe labels");
}
main().catch((error) => { console.error(error); process.exitCode = 1; });
