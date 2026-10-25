"use strict";

// Independent handheld rendering shell. The Rust API remains the single source
// of truth for profiles, active pins, module permissions, and offline tools.
// No camera, RF, GNSS, game, RustGIS or TAK connection occurs on navigation.
const $ = (id) => document.getElementById(id);
const ui = {
  screen: $("micro-device"),
  roleSwitch: $("role-switch"),
  roleShort: $("role-short"),
  clock: $("local-clock"),
  code: $("page-code"),
  title: $("page-title"),
  mode: $("mode-indicator"),
  viewport: $("micro-viewport"),
  content: $("micro-content"),
  tabs: Array.from(document.querySelectorAll("#micro-rail [data-tab]")),
  foot: $("micro-foot-message"),
  rolesDialog: $("roles-dialog"),
  rolesList: $("roles-list"),
  chosenName: $("chosen-role-name"),
  chosenDetail: $("chosen-role-detail"),
  applyRole: $("apply-role"),
  closeRoles: $("close-roles"),
  cancelRoles: $("cancel-roles"),
  toast: $("micro-toast")
};
const pages = ["stat", "gear", "map", "data", "link", "conf"];
const pageTitles = {
  stat: "SYSTEM STATUS", gear: "MODULE DECK", map: "NAVIGATION",
  data: "DATA & RECORDS", link: "DEVICE LINKS", conf: "CONFIGURATION"
};
const glyphs = {
  "host": "⌁", "notes": "≡", "library": "▤", "game": "▣",
  "net-lab": "⌘", "byte-lab": "0x", "simulator": "◈",
  "geo-lab": "⌖"
};
let snapshot = null;
let tab = "stat";
let openedTool = null;
let geoMode = "distance";
let focusedCamera = null;
let pendingProfile = null;
let busy = false;
let toastTimer = null;
let reducedEffects = false;

const make = (name, cls, text) => {
  const node = document.createElement(name);
  if (cls) node.className = cls;
  if (text !== undefined) node.textContent = String(text);
  return node;
};

const button = (text, click, cls = "micro-button") => {
  const node = make("button", cls, text);
  node.type = "button";
  node.addEventListener("click", click);
  return node;
};

function notify(message, isError = false) {
  ui.toast.textContent = message;
  ui.toast.classList.toggle("error", isError);
  ui.toast.hidden = false;
  ui.foot.textContent = message;
  if (toastTimer !== null) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { ui.toast.hidden = true; }, 3500);
}

function updateClock() {
  try {
    ui.clock.textContent = new Intl.DateTimeFormat(undefined, {
      hour: "2-digit", minute: "2-digit", hour12: false
    }).format(new Date());
  } catch {
    ui.clock.textContent = "--:--";
  }
}

async function api(path, data) {
  const options = { cache: "no-store", credentials: "omit" };
  if (data !== undefined) {
    options.method = "POST";
    options.headers = { "Content-Type": "application/json" };
    options.body = JSON.stringify(data);
  }
  const response = await fetch(path, options);
  let body;
  try {
    body = await response.json();
  } catch {
    throw new Error("Local Rust dashboard returned an invalid response");
  }
  if (!response.ok) throw new Error(body.error || "Local dashboard request refused");
  return body;
}

function activeProfile() {
  return snapshot?.profiles.find((item) => item.id === snapshot.active.profile) || null;
}

function allPinned() {
  return snapshot?.active.slots.map((item) => item.tool) || [];
}

function hasTool(id) {
  return allPinned().includes(id);
}

function toolInfo(id) {
  return snapshot?.palette.find((entry) => entry.id === id) || null;
}

function labelForProfile(profile) {
  const title = profile?.title || "RUST-BOY";
  if (profile?.id === "custom") return "CUSTOM";
  return title.replace(/\s+(Operations|Station|Field Kit)$/i, "").toUpperCase();
}

function renderHeading() {
  ui.roleShort.textContent = labelForProfile(activeProfile());
  const index = pages.indexOf(tab);
  ui.code.textContent = String(index + 1).padStart(2, "0") + " / " + tab.toUpperCase();
  const label = openedTool ? toolInfo(openedTool)?.name : pageTitles[tab];
  ui.title.textContent = label || pageTitles[tab];
  ui.mode.textContent = snapshot.engine.rustgis === "integrated" ? "LOCAL" : "OFFLINE";
  for (const item of ui.tabs) {
    const current = item.dataset.tab === tab;
    if (current) item.setAttribute("aria-current", "page");
    else item.removeAttribute("aria-current");
  }
}

function setTab(next) {
  if (!pages.includes(next) || !snapshot) return;
  tab = next;
  openedTool = null;
  focusedCamera = null;
  render();
  ui.viewport.scrollTop = 0;
}

function panel() {
  return make("div", "micro-panel");
}

function sectionTop(title, tag) {
  const row = make("div", "section-top");
  row.append(make("strong", "", title), make("span", "flag", tag));
  return row;
}

function field(key, title, value, options = {}) {
  const label = make("label", "field", title);
  const input = make("input");
  input.name = key;
  input.type = "text";
  input.value = value;
  input.required = true;
  input.maxLength = options.maxLength || 128;
  input.autocomplete = "off";
  input.spellcheck = false;
  if (options.numeric) input.inputMode = "decimal";
  if (options.hex) input.inputMode = "text";
  label.append(input);
  return label;
}

function showStat() {
  const root = panel();
  const heading = make("div", "hero-status");
  const status = make("div", "block-box lit");
  status.append(
    make("span", "micro-label", "ROLE / CURRENTLY LOADED"),
    make("div", "hero-title scan-glow", labelForProfile(activeProfile())),
    make("div", "hero-sub", "Operator station · profile locally saved")
  );
  const display = make("div", "instrument");
  display.setAttribute("aria-label", "Decorative local instrument grid; no sensor data");
  display.append(make("span", "", "NO LIVE SENSOR FEED"));
  heading.append(status, display);
  root.append(heading);
  const grid = make("div", "signal-grid");
  for (const [key, value] of [
    ["MODULES", String(allPinned().length).padStart(2, "0") + " PINNED"],
    ["NETWORK", "LOOPBACK"],
    ["POWER", "NOT DETECTED"],
    ["DATA SOURCE", "LOCAL ONLY"]
  ]) {
    const cell = make("div", "signal-cell");
    cell.append(make("span", "", key), make("strong", "", value));
    grid.append(cell);
  }
  root.append(grid);
  const row = make("div", "button-row");
  row.append(button("OPEN MODULES", () => setTab("gear"), "micro-button solid"));
  const shortcut = snapshot.active.profile === "reclamation-scout"
    ? ["SCOUT MAP", "map"]
    : snapshot.active.profile === "security-chief"
      ? ["CAMERA GRID", "link"]
      : ["CHOOSE JOB", null];
  row.append(button(shortcut[0], () => shortcut[1] ? setTab(shortcut[1]) : openRoles(), "micro-button"));
  root.append(row);
  const note = make("p", "panel-copy",
    "This is a local field workbench. No camera, sensor, console, map engine or radio device is connected automatically.");
  root.append(note);
  return root;
}

function toolTile(id, index) {
  const meta = toolInfo(id);
  const card = button("", () => openTool(id), "tile");
  if (!meta) {
    card.disabled = true;
    return card;
  }
  card.setAttribute("aria-label", "Open " + meta.name);
  const symbol = make("span", "tile-icon", glyphs[id] || "◧");
  symbol.setAttribute("aria-hidden", "true");
  const copy = make("span", "tile-copy");
  copy.append(
    make("strong", "", meta.name),
    make("small", "", meta.availability === "ready" ? "READY / LOCAL"
      : meta.availability === "simulated" ? "SYNTHETIC ONLY"
        : meta.availability === "explicit_connection" ? "OPT-IN / NOT CONNECTED"
          : "NOT IMPLEMENTED")
  );
  card.append(symbol, copy);
  card.title = "Tile " + (index + 1) + ": " + meta.description;
  return card;
}

function showGear() {
  const root = panel();
  root.append(sectionTop("PINNED MODULES", String(allPinned().length).padStart(2, "0") + " SLOTS"));
  const pinned = allPinned();
  if (pinned.length) {
    const grid = make("div", "tile-grid");
    pinned.forEach((id, index) => grid.append(toolTile(id, index)));
    root.append(grid);
  } else {
    root.append(make("div", "custom-empty",
      "CUSTOM WORKBENCH / EMPTY. Choose CONF to pin your own modules."));
    root.append(button("OPEN TOOL PALETTE", () => setTab("conf"), "micro-button solid"));
  }
  root.append(make("p", "panel-note",
    "Opening a tool shows its local controls or manual handoff. Loading a profile does not execute tools."));
  return root;
}

function showMap() {
  const root = panel();
  root.append(sectionTop("SCOUT / GEOSPATIAL", hasTool("geo-lab") ? "GIS LAB READY" : "NOT PINNED"));
  const map = make("div", "offline-grid");
  map.setAttribute("role", "img");
  map.setAttribute("aria-label", "Decorative geospatial reference grid, not a working map");
  map.append(make("span", "grid-label", "RUSTGIS NOT ATTACHED / NO LIVE POSITION"));
  root.append(map);
  root.append(make("p", "panel-copy",
    "Offline waypoint and distance math is available when Scout GIS is pinned. The real RustGIS map renderer and GNSS integration are later gates."));
  const actions = make("div", "quick-row");
  if (hasTool("geo-lab")) {
    actions.append(
      button("WAYPOINT", () => openTool("geo-lab", "waypoint"), "micro-button solid"),
      button("DISTANCE", () => openTool("geo-lab", "distance"))
    );
  } else {
    actions.append(button("LOAD SCOUT ROLE", openRoles, "micro-button solid"));
  }
  root.append(actions);
  return root;
}

function showData() {
  const root = panel();
  const dataTools = allPinned().filter((id) =>
    ["notes", "library", "byte-lab", "simulator", "geo-lab", "net-lab"].includes(id));
  root.append(sectionTop("RECORDS / OFFLINE TOOLS", String(dataTools.length) + " AVAILABLE"));
  if (dataTools.length) {
    const list = make("div", "tile-grid");
    dataTools.forEach((id, index) => list.append(toolTile(id, index)));
    root.append(list);
  } else {
    root.append(make("div", "custom-empty", "No data tools are pinned in this profile."));
    root.append(button("VIEW MODULES", () => setTab("gear"), "micro-button"));
  }
  root.append(make("p", "panel-note",
    "Notes remain in their local CLI store; this screen does not inspect private note contents or transmit observations."));
  return root;
}

function cameraTile(number, focus = false) {
  const cell = button("", () => {
    focusedCamera = focus ? null : number;
    render();
  }, "camera-tile" + (focus ? " camera-focus" : ""));
  cell.setAttribute("aria-label", focus
    ? "Camera " + number + " placeholder, return to all four"
    : "Focus camera " + number + " placeholder");
  cell.append(make("strong", "", "CAM " + String(number).padStart(2, "0")),
    make("small", "", "NO VIDEO / NO SOURCE"));
  return cell;
}

function showLink() {
  const root = panel();
  if (snapshot.active.profile === "security-chief") {
    root.append(sectionTop("CAMERA WALL / UI FIXTURE", "0/4 CONNECTED"));
    if (focusedCamera !== null) {
      root.append(cameraTile(focusedCamera, true));
      root.append(button("◀ BACK TO FOUR FEEDS", () => {
        focusedCamera = null;
        render();
      }, "micro-button"));
    } else {
      const quad = make("div", "camera-grid");
      for (let number = 1; number <= 4; number++) quad.append(cameraTile(number));
      root.append(quad);
    }
    root.append(make("p", "panel-note",
      "Tap a tile to focus. This is a layout-only prototype: no IP cameras are scanned, streamed or recorded."));
    return root;
  }
  root.append(sectionTop("TRANSPORT / DEVICE STATUS", "NO LIVE LINKS"));
  const grid = make("div", "signal-grid");
  for (const [type, state] of [
    ["TAK / COT", "NOT CONNECTED"],
    ["GNSS", "NOT ENABLED"],
    ["IP CAMERAS", "NOT CONNECTED"],
    ["PLAYSTATION", "MANUAL OPT-IN"]
  ]) {
    const cell = make("div", "signal-cell");
    cell.append(make("span", "", type), make("strong", "", state));
    grid.append(cell);
  }
  root.append(grid);
  if (hasTool("game")) root.append(button("GAME LINK INSTRUCTIONS", () => openTool("game"), "micro-button"));
  root.append(make("p", "panel-copy",
    "Network operations require their own explicit CLI/device-broker authorization. No connection begins from this screen."));
  return root;
}

function showConf() {
  const root = panel();
  root.append(sectionTop("OPERATOR CONFIG", "LOCAL SETTINGS"));
  const current = make("div", "block-box lit");
  current.append(
    make("div", "micro-label", "SAVED JOB TEMPLATE"),
    make("div", "preview-row")
  );
  current.lastElementChild.append(
    make("strong", "", activeProfile()?.title || "UNKNOWN"),
    button("CHANGE", openRoles, "micro-button compact")
  );
  root.append(current);
  const effects = make("div", "button-row");
  effects.append(
    button(reducedEffects ? "CRT EFFECTS OFF" : "CRT EFFECTS ON", () => {
      reducedEffects = !reducedEffects;
      document.body.classList.toggle("scanlines-off", reducedEffects);
      document.body.classList.toggle("glow-off", reducedEffects);
      render();
    }, "micro-button"),
    button("RELOAD STATE", reload, "micro-button")
  );
  root.append(effects);
  if (snapshot.active.profile === "custom") {
    root.append(sectionTop("ADVANCED / CUSTOM PALETTE", "ZERO AUTO GRANTS"));
    const grid = make("div", "tool-palette");
    const pinned = new Set(allPinned());
    for (const item of snapshot.palette) {
      const exists = pinned.has(item.id);
      const unavailable = item.availability === "planned";
      const tile = button("", () => exists ? removeCustom(item.id) : addCustom(item.id), "tool-pick");
      tile.disabled = unavailable || busy;
      tile.append(
        make("strong", "", (exists ? "− " : "+ ") + item.name),
        make("small", "", unavailable ? "NOT BUILT" : exists ? "REMOVE PIN" : item.availability.toUpperCase().replace(/_/g, " "))
      );
      grid.append(tile);
    }
    root.append(grid);
  } else {
    root.append(make("p", "panel-copy",
      "Custom is the last job template. Switch to Custom to start with an empty, user-selected tool deck."));
  }
  root.append(make("p", "panel-note",
    "No measured battery indicator is available yet. Power, GPS and camera statuses are not simulated as live readings."));
  return root;
}

function openTool(id, mode = "distance") {
  if (!hasTool(id)) {
    notify("This module is not pinned. Choose a profile containing it.", true);
    return;
  }
  openedTool = id;
  geoMode = mode;
  render();
  ui.viewport.scrollTop = 0;
  ui.viewport.focus({ preventScroll: true });
}

function inputForm(root, id) {
  const form = make("form", "micro-panel");
  form.id = "micro-lab-form";
  if (id === "geo-lab") {
    const modes = make("div", "button-row");
    for (const mode of ["distance", "waypoint"]) {
      modes.append(button(mode === "distance" ? "DISTANCE" : "WAYPOINT", () => {
        geoMode = mode;
        render();
      }, "micro-button " + (geoMode === mode ? "solid" : "ghost")));
    }
    form.append(modes);
    const fields = make("div", "form-grid");
    fields.append(field("lat", "LATITUDE", "42.0", { numeric:true }));
    fields.append(field("lon", "LONGITUDE", "-70.75", { numeric:true }));
    if (geoMode === "distance") {
      fields.append(field("lat2", "DEST LATITUDE", "42.01", { numeric:true }));
      fields.append(field("lon2", "DEST LONGITUDE", "-70.76", { numeric:true }));
    } else {
      const name = field("name", "WAYPOINT LABEL", "Camp Alpha");
      name.classList.add("wide");
      fields.append(name);
    }
    form.append(fields);
  } else if (id === "net-lab") {
    form.append(field("input", "IPV4 / CIDR", "192.168.1.50/24"));
  } else if (id === "byte-lab") {
    form.append(field("input", "HEXADECIMAL BYTES", "48656c6c6f", { maxLength: 3000, hex: true }));
  }
  const row = make("div", "form-actions");
  const run = make("button", "micro-button solid", id === "host" ? "SAMPLE LOCAL STATUS" :
    ["game", "notes", "simulator", "library"].includes(id) ? "VIEW SAFE HANDOFF" : "RUN LOCAL TOOL");
  run.type = "submit";
  row.append(run);
  form.append(row);
  const output = make("pre", "mono-output", "Awaiting explicit operator action.");
  output.setAttribute("aria-live", "polite");
  form.append(output);
  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    if (busy) return;
    busy = true;
    run.disabled = true;
    output.textContent = "LOCAL TOOL / PROCESSING";
    const payload = { tool: id };
    if (id === "geo-lab") payload.mode = geoMode;
    for (const [key, val] of new FormData(form).entries()) payload[key] = val;
    try {
      const result = await api("/api/experiment", payload);
      if (openedTool === id) output.textContent = result.output;
      ui.foot.textContent = result.kind === "computed" ? "RESULT / LOCAL ONLY" : "INFORMATION / NO CONNECTION";
    } catch (error) {
      if (openedTool === id) output.textContent = "ERROR / " + error.message;
      notify(error.message, true);
    } finally {
      busy = false;
      run.disabled = false;
    }
  });
  root.append(form);
}

function showTool() {
  const root = panel();
  const meta = toolInfo(openedTool);
  const row = make("div", "frame-head");
  row.append(
    button("◀ BACK", () => { openedTool = null; render(); }, "micro-button compact"),
    make("span", "mini-pill", meta?.availability === "ready" ? "OFFLINE / LOCAL" : "MANUAL ONLY")
  );
  root.append(row);
  if (!meta) {
    root.append(make("p", "warning", "Unknown module. Return to the deck."));
    return root;
  }
  root.append(make("p", "panel-copy", meta.description));
  const supported = ["geo-lab", "net-lab", "byte-lab", "host", "notes", "library", "simulator", "game"];
  if (supported.includes(openedTool)) {
    inputForm(root, openedTool);
  } else {
    root.append(make("div", "block-box", "Not available in this UI. No hardware permission was granted."));
  }
  if (snapshot.active.profile === "custom") {
    const item = snapshot.active.slots.find((s) => s.tool === openedTool);
    if (item) root.append(button("REMOVE FROM CUSTOM", () => removeCustom(openedTool), "micro-button ghost"));
  }
  return root;
}

function render() {
  if (!snapshot) return;
  renderHeading();
  ui.content.replaceChildren(openedTool ? showTool() :
    tab === "stat" ? showStat() :
      tab === "gear" ? showGear() :
        tab === "map" ? showMap() :
          tab === "data" ? showData() :
            tab === "link" ? showLink() : showConf());
}

async function reload() {
  try {
    snapshot = await api("/api/state");
    if (openedTool && !hasTool(openedTool)) openedTool = null;
    focusedCamera = null;
    render();
    notify("LOCAL WORKBENCH / SYNCHRONIZED");
  } catch (error) {
    ui.code.textContent = "LOCAL / ERROR";
    ui.content.replaceChildren(make("div", "block-box error", error.message));
    notify(error.message, true);
  }
}

function openRoles() {
  if (!snapshot || busy) return;
  pendingProfile = snapshot.active.profile;
  ui.rolesList.replaceChildren();
  snapshot.profiles.forEach((role, index) => {
    const row = button("", () => selectRole(role.id), "profile-row");
    row.dataset.role = role.id;
    row.append(make("span", "num", String(index + 1).padStart(2, "0")));
    const info = make("div");
    info.append(make("strong", "", role.title), make("span", "", role.id === "custom"
      ? "EMPTY / ADVANCED USER" : String(role.tools.length) + " PRELOADED TOOLS"));
    row.append(info);
    ui.rolesList.append(row);
  });
  selectRole(pendingProfile);
  if (!ui.rolesDialog.open) ui.rolesDialog.showModal();
}

function selectRole(id) {
  pendingProfile = id;
  const choice = snapshot.profiles.find((role) => role.id === id);
  if (!choice) return;
  for (const row of ui.rolesList.querySelectorAll("[data-role]")) {
    row.setAttribute("aria-pressed", String(row.dataset.role === id));
  }
  ui.chosenName.textContent = choice.title;
  ui.chosenDetail.textContent = choice.description;
  const unchanged = id === snapshot.active.profile
    && !(id === "custom" && snapshot.active.slots.length);
  ui.applyRole.disabled = unchanged || busy;
  ui.applyRole.textContent = id === "custom" && snapshot.active.profile === "custom"
    ? "CONFIRM RESET CUSTOM" : "CONFIRM REPLACE";
}

async function activateRole() {
  if (!pendingProfile || busy || ui.applyRole.disabled) return;
  busy = true;
  ui.applyRole.disabled = true;
  const id = pendingProfile;
  try {
    snapshot = await api("/api/profile", { id, confirm: true });
    ui.rolesDialog.close();
    openedTool = null;
    focusedCamera = null;
    tab = "stat";
    render();
    ui.viewport.scrollTop = 0;
    notify("PROFILE LOADED / NO TOOL EXECUTED");
  } catch (error) {
    notify(error.message, true);
  } finally {
    busy = false;
    if (ui.rolesDialog.open) selectRole(pendingProfile);
  }
}

async function addCustom(id) {
  if (snapshot.active.profile !== "custom" || busy) return;
  busy = true;
  try {
    snapshot = await api("/api/custom/add", { id });
    notify("TOOL PINNED / NOT EXECUTED");
    render();
  } catch (error) {
    notify(error.message, true);
  } finally { busy = false; }
}

async function removeCustom(id) {
  if (snapshot.active.profile !== "custom" || busy) return;
  const slot = snapshot.active.slots.find((item) => item.tool === id);
  if (!slot) return;
  busy = true;
  try {
    snapshot = await api("/api/custom/remove", { id: slot.id });
    if (openedTool === id) openedTool = null;
    render();
    notify("TOOL REMOVED / LAYOUT ONLY");
  } catch (error) {
    notify(error.message, true);
  } finally { busy = false; }
}

function back() {
  if (ui.rolesDialog.open) {
    ui.rolesDialog.close();
  } else if (openedTool) {
    openedTool = null;
    render();
  } else if (focusedCamera !== null) {
    focusedCamera = null;
    render();
  } else if (tab !== "stat") {
    setTab("stat");
  }
}

function init() {
  ui.tabs.forEach((item) => item.addEventListener("click", () => setTab(item.dataset.tab)));
  ui.roleSwitch.addEventListener("click", openRoles);
  ui.closeRoles.addEventListener("click", () => ui.rolesDialog.close());
  ui.cancelRoles.addEventListener("click", () => ui.rolesDialog.close());
  ui.applyRole.addEventListener("click", activateRole);
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      // Browsers handle closing the native modal themselves.
      if (ui.rolesDialog.open) return;
      back();
      return;
    }
    if (!snapshot || ui.rolesDialog.open || event.altKey || event.metaKey || event.ctrlKey) return;
    const target = event.target;
    if (target instanceof Element &&
        target.closest("input,select,textarea,[contenteditable='true']")) return;
    if (tab === "link" && snapshot.active.profile === "security-chief" &&
        openedTool === null && /^[1-4]$/.test(event.key)) {
      focusedCamera = Number(event.key);
      render();
      return;
    }
    if (/^[1-6]$/.test(event.key)) {
      setTab(pages[Number(event.key) - 1]);
      return;
    }
    if ((event.key === "ArrowLeft" || event.key === "ArrowRight") &&
        !openedTool && target === ui.viewport) {
      const offset = event.key === "ArrowLeft" ? -1 : 1;
      setTab(pages[(pages.indexOf(tab) + offset + pages.length) % pages.length]);
    }
  });
  updateClock();
  setInterval(updateClock, 60000);
  reload();
}

init();
