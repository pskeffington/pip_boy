"use strict";

// Offline RADIO HUD interaction regression. No real broadcasters, Bluetooth
// scans, or audio devices are opened by this test.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const root = path.resolve(__dirname, "..");
const script = fs.readFileSync(
  path.join(root, "crates/pipboy/assets/radio-ui.js"), "utf8"
);
const css = fs.readFileSync(
  path.join(root, "crates/pipboy/assets/radio-ui.css"), "utf8"
);

class Element {
  constructor(tag) {
    this.tagName = tag;
    this.children = [];
    this.events = new Map();
    this.attributes = {};
    this.dataset = {};
    this.className = "";
    this.value = "";
    this.text = "";
    this.disabled = false;
    this.hidden = false;
    this.classList = { add: (name) => { this.className += " " + name; } };
  }
  get textContent() {
    return this.text + this.children.map((item) => item.textContent).join(" ");
  }
  set textContent(value) {
    this.text = String(value);
    this.children = [];
  }
  append(...items) { this.children.push(...items); }
  prepend(...items) { this.children.unshift(...items); }
  replaceChildren(...items) { this.text = ""; this.children = [...items]; }
  setAttribute(name, value) { this.attributes[name] = value; }
  addEventListener(name, fn) { this.events.set(name, fn); }
  dispatch(name) {
    const fn = this.events.get(name);
    assert.ok(fn, "Missing " + name + " handler on " + this.className);
    fn({ preventDefault() {} });
  }
}

const walk = (node) => [node, ...node.children.flatMap(walk)];
function control(container, className) {
  const found = walk(container).find((node) =>
    node.className.split(/\s+/).includes(className)
  );
  assert.ok(found, "Missing RADIO control ." + className);
  return found;
}
const flush = () => new Promise(setImmediate);
const stations = [{
  id: "skylab", name: "Skylab Radio (UK)", genre: "Chillout / lounge / ambient"
}];
const system = {
  id: "system", name: "System default", default: true, available: true
};
const bluetooth = {
  id: "output-1-bb78", name: "Bluetooth Headphones",
  default: false, available: true
};

async function main() {
  let state = {
    source: "none", status: "stopped", station: null,
    volume: 60, error: null, output: "system", output_available: true,
    output_failure: false, stations, outputs: [system, bluetooth],
    sdr: "not_connected"
  };
  let poll = null;
  const calls = [];
  const window = {};
  const document = { createElement: (tag) => new Element(tag) };
  const context = {
    document, window,
    setInterval: (fn, delay) => {
      assert.equal(delay, 1800);
      poll = fn;
      return 1;
    },
    clearInterval: (id) => {
      assert.equal(id, 1);
      poll = null;
    },
    fetch: async (url, options) => {
      assert.equal(url, "/api/radio");
      assert.equal(options.credentials, "omit");
      if (options.method !== "POST") {
        calls.push({ action: "poll" });
      } else {
        assert.equal(options.headers["Content-Type"], "application/json");
        const body = JSON.parse(options.body);
        calls.push(body);
        if (body.action === "play") {
          assert.equal(body.station, "skylab");
          assert.equal(body.confirm, true);
          state = { ...state, station: "skylab", source: "native_ip",
            status: "playing" };
        } else if (body.action === "output") {
          assert.ok(state.outputs.some((device) => device.id === body.output));
          state = { ...state, output: body.output,
            output_failure: false, output_available: true, error: null,
            status: state.status === "failed" ? "connecting" : state.status };
        } else if (body.action === "stop") {
          state = { ...state, station: null, source: "none",
            status: "stopped", output_failure: false };
        } else if (body.action === "volume") {
          state = { ...state, volume: body.volume };
        } else {
          throw Error("Unexpected command " + body.action);
        }
      }
      return { ok: true, json: async () => ({ ...state }) };
    }
  };

  vm.runInNewContext(script, context, { filename: "radio-ui.js" });
  const rootNode = new Element("div");
  const mounted = window.RustBoyRadio.mount(rootNode, { compact: true });
  await flush();
  assert.deepEqual(calls.map((x) => x.action), ["poll"],
    "Loading the HUD must not start a broadcast");
  assert.equal(control(rootNode, "rb-radio-play").disabled, false);
  assert.equal(control(rootNode, "rb-radio-fallback").hidden, true);
  assert.ok(poll, "HUD status refresh must be enabled");

  control(rootNode, "rb-radio-play").dispatch("click");
  await flush();
  await flush();
  assert.ok(calls.some((x) => x.action === "play"));
  assert.equal(control(rootNode, "rb-radio-signal").textContent, "PLAYING");

  const outputs = control(rootNode, "rb-radio-output");
  outputs.value = bluetooth.id;
  outputs.dispatch("change");
  await flush();
  await flush();
  assert.equal(state.output, bluetooth.id);
  assert.equal(state.station, "skylab");

  // Simulate a physical Bluetooth disappearance (changed CPAL enumeration)
  // and decoder-detected output failure; the HUD must not auto-switch.
  state = { ...state, outputs: [system], output_available: false,
    output_failure: true, error: "Native output disconnected",
    status: "failed" };
  const priorCommands = calls.filter((x) => x.action !== "poll").length;
  poll();
  await flush();
  assert.equal(calls.filter((x) => x.action !== "poll").length, priorCommands,
    "Device disappearance must not automatically switch audio");
  assert.equal(control(rootNode, "rb-radio-signal").textContent, "OUTPUT FAILED");
  assert.equal(control(rootNode, "rb-radio-fallback").hidden, false);
  assert.equal(control(rootNode, "rb-radio-fallback").disabled, false);
  assert.match(control(rootNode, "rb-radio-message").textContent, /OUTPUT FAILED/);
  assert.ok(walk(outputs).some((item) =>
    item.tagName === "option" && item.value === bluetooth.id && item.disabled
  ), "Disconnected selected output must remain visibly identified");

  control(rootNode, "rb-radio-fallback").dispatch("click");
  await flush();
  await flush();
  assert.equal(state.output, "system");
  assert.equal(state.station, "skylab", "Explicit recovery must retain station");
  assert.equal(state.status, "connecting");
  assert.equal(control(rootNode, "rb-radio-fallback").hidden, true);

  state = { ...state, status: "recovering" };
  poll();
  await flush();
  assert.match(control(rootNode, "rb-radio-message").textContent, /Reconnecting/);
  control(rootNode, "rb-radio-stop").dispatch("click");
  await flush();
  await flush();
  assert.equal(state.status, "stopped");

  // If there is no default speaker, recovery may be shown but cannot be
  // activated until the OS makes a legitimate output available.
  state = { ...state, output: bluetooth.id, output_available: false,
    output_failure: true, status: "failed",
    outputs: [{ ...system, available: false }] };
  poll();
  await flush();
  assert.equal(control(rootNode, "rb-radio-fallback").disabled, true);

  mounted.destroy();
  assert.equal(poll, null, "Unmount must release status polling");
  assert.match(css, /\.rb-radio \.rb-radio-fallback/);
  assert.match(css, /min-height:\s*38px/);
  console.log("RADIO HUD: native playback consent, Bluetooth hotplug, manual recovery, stop OK");
}
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
