"use strict";

// Offline RADIO HUD interaction regression; only the local JSON API is mocked.
// No broadcaster, audio hardware, network, Bluetooth or browser storage is used.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const root = path.resolve(__dirname, "..");
const script = fs.readFileSync(path.join(root, "crates/pipboy/assets/radio-ui.js"), "utf8");
const css = fs.readFileSync(path.join(root, "crates/pipboy/assets/radio-ui.css"), "utf8");

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
const flush = async () => { await new Promise(setImmediate); await new Promise(setImmediate); };
const stations = [
  { id: "skylab", name: "Skylab Radio (UK)", genre: "Chillout / lounge / ambient",
    category: "CHILL", format: "Live", nominal_kbps: 0 },
  { id: "groovesalad", name: "Groove Salad", genre: "Ambient / downtempo",
    category: "CHILL", format: "MP3", nominal_kbps: 128 },
  { id: "dronezone", name: "Drone Zone", genre: "Atmospheric / ambient",
    category: "AMBIENT", format: "MP3", nominal_kbps: 128 },
  { id: "defcon", name: "DEF CON Radio", genre: "Electronic / hacker radio",
    category: "ELECTRONIC", format: "MP3", nominal_kbps: 128 }
];
const system = { id: "system", name: "System default", default: true, available: true };
const bluetooth = { id: "output-1-bb78", name: "Bluetooth Headphones",
  default: false, available: true };

async function main() {
  let state = {
    source: "none", status: "stopped", station: null,
    volume: 60, error: null, output: "system", output_available: true,
    output_failure: false, stations, outputs: [system, bluetooth],
    presets: { schema_version: 1, favorites: [], last_selected: null },
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
        if (body.action === "select") {
          assert.ok(stations.some((station) => station.id === body.station));
          assert.equal(state.status === "stopped" || state.status === "failed", true);
          state = { ...state, presets: { ...state.presets, last_selected: body.station } };
        } else if (body.action === "favorite") {
          assert.ok(stations.some((station) => station.id === body.station));
          let favorites = [...state.presets.favorites];
          if (body.enabled && !favorites.includes(body.station)) favorites.push(body.station);
          if (!body.enabled) favorites = favorites.filter((id) => id !== body.station);
          state = { ...state, presets: { ...state.presets, favorites } };
        } else if (body.action === "move_favorite") {
          const favorites = [...state.presets.favorites];
          const index = favorites.indexOf(body.station);
          assert.ok(index >= 0);
          const other = body.direction === "earlier"
            ? Math.max(0, index - 1) : Math.min(favorites.length - 1, index + 1);
          [favorites[index], favorites[other]] = [favorites[other], favorites[index]];
          state = { ...state, presets: { ...state.presets, favorites } };
        } else if (body.action === "play") {
          assert.equal(body.station, "skylab");
          assert.equal(body.confirm, true);
          state = { ...state, station: body.station, source: "native_ip", status: "playing",
            presets: { ...state.presets, last_selected: body.station } };
        } else if (body.action === "sdrconnect") {
          assert.equal(state.status, "stopped");
          state = {...state, sdrconnect: {...state.sdrconnect, endpoint: body.endpoint}};
        } else if (body.action === "play_sdr") {
          assert.equal(body.confirm, true);
          state = {...state, source:"native_sdr", status:"playing", station:null,
            receiver:{frequency_hz:99100000,demodulator:"WFM",antenna:"Antenna A"}};
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
    "Opening radio must not start streaming or mutate preferences");
  assert.equal(control(rootNode, "rb-radio-count").textContent, "01 / 04 · CHILL");
  assert.equal(control(rootNode, "rb-radio-output-panel").hidden, true);
  assert.equal(control(rootNode, "rb-radio-play").disabled, false);
  const tuner = control(rootNode, "rb-radio-station");
  assert.equal(walk(tuner).filter((item) => item.tagName === "optgroup").length, 3);
  const next = control(rootNode, "rb-radio-next");
  const previous = control(rootNode, "rb-radio-prev");
  next.dispatch("click");
  await flush();
  assert.equal(tuner.value, "groovesalad");
  assert.equal(state.presets.last_selected, "groovesalad");
  assert.equal(control(rootNode, "rb-radio-count").textContent, "02 / 04 · CHILL");
  previous.dispatch("click");
  await flush();
  assert.equal(tuner.value, "skylab");
  previous.dispatch("click");
  await flush();
  assert.equal(tuner.value, "defcon", "Station navigation wraps");
  next.dispatch("click");
  await flush();
  assert.equal(tuner.value, "skylab");
  assert.equal(calls.filter((c) => c.action === "play").length, 0,
    "Browsing must never initiate native audio");

  const star = control(rootNode, "rb-radio-favorite");
  star.dispatch("click");
  await flush();
  assert.deepEqual(state.presets.favorites, ["skylab"]);
  assert.equal(star.attributes["aria-pressed"], "true");
  next.dispatch("click");
  await flush();
  star.dispatch("click");
  await flush();
  assert.deepEqual(state.presets.favorites, ["skylab", "groovesalad"]);

  const filter = control(rootNode, "rb-radio-filter");
  filter.dispatch("click");
  assert.equal(filter.attributes["aria-pressed"], "true");
  assert.equal(control(rootNode, "rb-radio-count").textContent, "02 / 02 ★");
  assert.equal(walk(tuner).filter((item) => item.tagName === "optgroup").length, 0);
  control(rootNode, "rb-radio-output-toggle").dispatch("click");
  assert.equal(control(rootNode, "rb-radio-output-panel").hidden, false);
  const earlier = control(rootNode, "rb-radio-order-earlier");
  earlier.dispatch("click");
  await flush();
  assert.deepEqual(state.presets.favorites, ["groovesalad", "skylab"]);
  assert.equal(tuner.children[0].value, "groovesalad", "Favorite order is user controlled");
  assert.equal(control(rootNode, "rb-radio-count").textContent, "01 / 02 ★");

  star.dispatch("click");
  await flush();
  assert.deepEqual(state.presets.favorites, ["skylab"]);
  assert.equal(tuner.value, "skylab", "Removing favorite selects the remaining preset");
  filter.dispatch("click");
  assert.equal(filter.attributes["aria-pressed"], "false");
  await flush();
  assert.ok(walk(tuner).some((item) => item.tagName === "option"
    && item.textContent.includes("★ Skylab")));

  const lastSelected = state.presets.last_selected;
  assert.equal(typeof lastSelected, "string");
  // An automatic list-filter change is not a user selection; choosing a
  // different station and returning explicitly updates saved selection.
  tuner.value = "defcon";
  tuner.dispatch("change");
  await flush();
  assert.equal(state.presets.last_selected, "defcon");
  tuner.value = "skylab";
  tuner.dispatch("change");
  await flush();
  assert.equal(state.presets.last_selected, "skylab");

  control(rootNode, "rb-radio-play").dispatch("click");
  await flush();
  assert.equal(state.status, "playing");
  assert.equal(control(rootNode, "rb-radio-signal").textContent, "IP / PLAYING");
  assert.equal(next.disabled, true, "Cannot retune an active session");
  assert.equal(tuner.disabled, true);
  assert.equal(filter.disabled, true);

  const outputs = control(rootNode, "rb-radio-output");
  outputs.value = bluetooth.id;
  outputs.dispatch("change");
  await flush();
  assert.equal(state.output, bluetooth.id);
  assert.equal(state.station, "skylab");
  assert.equal(control(rootNode, "rb-radio-output-panel").hidden, true,
    "Output menu collapses when a requested switch succeeds");

  state = { ...state, outputs: [system], output_available: false,
    output_failure: true, error: "Native output disconnected", status: "failed" };
  const priorCommands = calls.filter((c) => c.action !== "poll").length;
  poll();
  await flush();
  assert.equal(calls.filter((c) => c.action !== "poll").length, priorCommands,
    "Bluetooth failure must not trigger automatic output switching");
  assert.equal(control(rootNode, "rb-radio-signal").textContent, "OUTPUT FAILED");
  assert.equal(control(rootNode, "rb-radio-output-panel").hidden, false);
  assert.equal(control(rootNode, "rb-radio-fallback").disabled, false);
  assert.ok(walk(outputs).some((item) =>
    item.tagName === "option" && item.value === bluetooth.id && item.disabled
  ));
  control(rootNode, "rb-radio-fallback").dispatch("click");
  await flush();
  assert.equal(state.output, "system");
  assert.equal(state.station, "skylab");
  assert.equal(state.status, "connecting");

  state = { ...state, status: "recovering" };
  poll();
  await flush();
  assert.match(control(rootNode, "rb-radio-message").textContent, /Reconnecting/);
  control(rootNode, "rb-radio-stop").dispatch("click");
  await flush();
  assert.equal(state.status, "stopped");
  assert.equal(state.presets.last_selected, "skylab", "Stop must preserve last station");

  state = { ...state, output: bluetooth.id, output_available: false,
    output_failure: true, status: "failed",
    outputs: [{ ...system, available: false }] };
  poll();
  await flush();
  assert.equal(control(rootNode, "rb-radio-fallback").disabled, true);

  mounted.destroy();
  assert.equal(poll, null);
  state = { ...state, outputs: [system], output: "system",
    output_available: true, output_failure: false, status: "stopped", error: null };
  const beforeRestart = calls.filter((c) => c.action === "play").length;
  const restarted = new Element("div");
  const resumed = window.RustBoyRadio.mount(restarted, { compact: true });
  await flush();
  assert.equal(control(restarted, "rb-radio-station").value, "skylab",
    "Last-selected station survives HUD recreation");
  assert.equal(control(restarted, "rb-radio-favorite").attributes["aria-pressed"], "true");
  assert.equal(calls.filter((c) => c.action === "play").length, beforeRestart,
    "Restoring favorites/selection must not autoplay");
  state = {...state,sdrconnect:{available:true,endpoint:"127.0.0.1:5454"}};
  poll(); await flush();
  const receiverSelector=control(restarted,"rb-radio-station");
  const beforeReceiverSelect=calls.length;
  receiverSelector.value="__sdrconnect__"; receiverSelector.dispatch("change"); await flush();
  assert.equal(calls.slice(beforeReceiverSelect).filter(c=>c.action!=="poll").length,0,
    "Selecting receiver must not connect or persist a fake station ID");
  assert.equal(control(restarted,"rb-radio-favorite").hidden,true);
  assert.equal(state.presets.last_selected,"skylab");
  control(restarted,"rb-radio-output-toggle").dispatch("click");
  assert.equal(control(restarted,"rb-radio-receiver").hidden,false);
  const address=control(restarted,"rb-radio-address");
  address.value="127.0.0.1:5455"; address.dispatch("input");
  poll(); await flush();
  assert.equal(address.value,"127.0.0.1:5455","Polling must not overwrite an address being edited");
  control(restarted,"rb-radio-receiver-set").dispatch("click"); await flush();
  assert.equal(state.status,"stopped");
  assert.equal(calls.some(c=>c.action==="play_sdr"),false);
  control(restarted,"rb-radio-play").dispatch("click"); await flush();
  assert.equal(state.source,"native_sdr");
  assert.equal(control(restarted,"rb-radio-signal").textContent,"SDR / PLAYING");
  assert.equal(address.disabled,true);
  assert.equal(receiverSelector.disabled,true);
  state={...state,status:"stopped",stopping:true}; poll(); await flush();
  assert.equal(control(restarted,"rb-radio-play").disabled,true,"Stop must retain the resource cleanup gate");
  assert.match(control(restarted,"rb-radio-message").textContent,/STOPPING/);
  state={...state,source:"none",status:"stopped",stopping:false}; poll(); await flush();
  receiverSelector.value="skylab"; receiverSelector.dispatch("change"); await flush();
  assert.equal(control(restarted,"rb-radio-favorite").hidden,false);
  resumed.destroy();

  assert.match(css, /\.rb-radio-compact \.rb-radio-transport\{/);
  assert.match(css, /\.rb-radio-compact \.rb-radio-output-panel\{position:absolute/);
  assert.match(css, /min-height:40px/);
  console.log("RADIO HUD: favorites, ordering, last station, no autoplay, Bluetooth recovery, SDR source and cleanup gates OK");
}
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
