"use strict";

// Independent handheld rendering shell. The Rust API remains the single source
// of truth for profiles, active pins, module permissions, and offline tools.
// No camera, RF, GNSS, game, RustGIS or TAK connection occurs on navigation.
const $ = (id) => document.getElementById(id);
const ui = {
  screen: $("micro-device"),
  back: $("micro-back"),
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
  toast: $("micro-toast"),
  rolePrev: $("role-prev"),
  roleNext: $("role-next"),
  rolePaging: $("role-page")
};
const pages = ["stat", "gear", "map", "data", "link", "conf"];
const pageTitles = {
  stat: "STAT", gear: "GEAR", map: "MAP",
  data: "DATA", link: "LINK", conf: "SETUP"
};
const glyphs = {
  "mission-board": "▣", "host": "⌁", "notes": "≡", "library": "▤", "game": "▣",
  "diag": "!", "net-lab": "⌘", "byte-lab": "0x", "simulator": "◈",
  "geo-lab": "⌖", "reclamation": "▧"
};
const roleStations = Object.freeze({
  "overseer": {
    heading: "OPS",
    purpose: "TASKS / TEAM",
    quick: "TASKS", tool: "mission-board"
  },
  "head-engineer": {
    heading: "ENGINEER",
    purpose: "SYSTEM / NET",
    quick: "NETWORK", tool: "net-lab"
  },
  "security-chief": {
    heading: "SECURITY",
    purpose: "CAMERAS / WATCH",
    quick: "CAM WALL", page: "link"
  },
  "vault-doctor": {
    heading: "MEDICAL",
    purpose: "REFERENCE / NOTES",
    quick: "ARCHIVES", tool: "library"
  },
  "university-researcher": {
    heading: "RESEARCH",
    purpose: "LAB / DATA",
    quick: "BYTE LAB", tool: "byte-lab"
  },
  "reclamation-scout": {
    heading: "SCOUT",
    purpose: "SITES / SURVEYS",
    quick: "SITES", tool: "reclamation", page: "map"
  },
  "records-custodian": {
    heading: "RECORDS",
    purpose: "ARCHIVES / JOURNAL",
    quick: "ARCHIVES", tool: "library"
  },
  "custom": {
    heading: "CUSTOM",
    purpose: "USER / TOOLS",
    quick: "ADD TOOLS", page: "conf"
  }
});

function openDuties() {
  openedTool = "assigned-duties";
  render();
  ui.viewport.scrollTop = 0;
}

let snapshot = null;
let tab = "stat";
let openedTool = null;
let geoMode = "distance";
const labDrafts = new Map();
let focusedCamera = null;
let pendingProfile = null;
let busy = false;
let toastTimer = null;
let reducedEffects = false;
let activeTaskUI = null;
let activeNetworkUI = null;
let activeMessengerUI = null;
let activeRadioUI = null;
const viewPage = { gear: 0, data: 0, conf: 0 };
let paletteOpen = false;
let rolePage = 0;
const MICRO_PAGE_SIZE = 6;

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
  ui.code.textContent = String(index + 1).padStart(2, "0");
  const label = openedTool === "native-radio" ? "RADIO"
    : openedTool === "secure-messenger" ? "MESSENGER"
    : openedTool === "assigned-duties" ? "ASSIGNED DUTIES"
    : openedTool ? toolInfo(openedTool)?.name : pageTitles[tab];
  ui.title.textContent = label || pageTitles[tab];
  ui.mode.textContent = "LOCAL";
  ui.back.hidden = !openedTool && focusedCamera === null && tab === "stat";
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
  activeTaskUI = null;
  focusedCamera = null;
  render();
  ui.viewport.scrollTop = 0;
  ui.viewport.focus({ preventScroll: true });
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
  root.classList.add("micro-home");
  const profileId = snapshot.active.profile;
  const station = roleStations[profileId] || roleStations.custom;
  root.append(sectionTop("QUICK ACCESS", ""));
  const actions = make("div", "quick-row micro-home-actions");
  if (station.tool && hasTool(station.tool)) {
    actions.append(button(station.quick, () => openTool(station.tool), "micro-button solid"));
  } else if (station.page) {
    actions.append(button(station.quick, () => setTab(station.page), "micro-button solid"));
  }
  if (station.page && station.tool) {
    actions.append(button("MAP", () => setTab(station.page), "micro-button"));
  }
  actions.append(button("TOOLS", () => setTab("gear"), "micro-button"));
  root.append(actions);
  const taskSection = make("div", "task-station-summary");
  root.append(taskSection);
  if (window.RustBoyTasks) {
    window.RustBoyTasks.summary(taskSection, {
      compact: true,
      role: profileId,
      onOpen: profileId === "overseer" && hasTool("mission-board")
        ? () => openTool("mission-board") : openDuties
    });
  }
  return root;
}

/** Paging is bounded to the visible screen; modules are not hidden offscreen. */
function pagedTiles(root, ids, key, renderItem = toolTile, pageSize = MICRO_PAGE_SIZE) {
  const count = Math.max(1, Math.ceil(ids.length / pageSize));
  viewPage[key] = Math.min(viewPage[key], count - 1);
  const offset = viewPage[key] * pageSize;
  const grid = make("div", "tile-grid");
  ids.slice(offset, offset + pageSize).forEach((id, index) =>
    grid.append(renderItem(id, offset + index)));
  root.append(grid);
  if (count > 1) {
    const pager = make("div", "micro-pager");
    const prev = button("◀", () => { viewPage[key]--; render(); }, "micro-button compact");
    const next = button("▶", () => { viewPage[key]++; render(); }, "micro-button compact");
    prev.disabled = viewPage[key] === 0;
    next.disabled = viewPage[key] >= count - 1;
    prev.setAttribute("aria-label", "Previous tool page");
    next.setAttribute("aria-label", "Next tool page");
    pager.append(prev, make("span", "micro-label", (viewPage[key] + 1) + "/" + count), next);
    root.append(pager);
  }
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
    make("small", "", meta.availability === "ready" ? "READY"
      : meta.availability === "simulated" ? "SIM"
        : meta.availability === "explicit_connection" ? "OPT-IN"
          : "PLANNED")
  );
  card.append(symbol, copy);
  card.title = "Tile " + (index + 1) + ": " + meta.description;
  return card;
}

function showGear() {
  const root = panel();
  const pins = allPinned();
  root.append(sectionTop("PINNED", String(pins.length) + " TOOLS"));
  if (pins.length) pagedTiles(root, pins, "gear");
  else {
    root.append(make("div", "custom-empty", "NO TOOLS"));
    root.append(button("ADD TOOLS", () => { paletteOpen = true; setTab("conf"); },
      "micro-button solid"));
  }
  return root;
}

function showMap() {
  const root = panel();
  root.classList.add("rb-map-micro-host");
  const holder = make("div", "rb-map-mount");
  root.append(holder);
  if (window.RustBoyMap) {
    requestAnimationFrame(() => {
      if (holder.isConnected) window.RustBoyMap.mount(holder, {
        onTools: hasTool("geo-lab") ? () => openTool("geo-lab", "waypoint") : null,
        reclamation: hasTool("reclamation")
      });
    });
  } else {
    holder.textContent = "LOCAL MAP MODULE UNAVAILABLE";
  }
  return root;
}

function showData() {
  const root = panel();
  const tools = allPinned().filter((id) =>
    ["notes", "library", "byte-lab", "simulator", "geo-lab", "reclamation", "net-lab", "diag"].includes(id));
  root.append(sectionTop("DATA", String(tools.length) + " TOOLS"));
  if (snapshot.active.profile !== "overseer") {
    root.append(button("MY DUTIES", openDuties, "micro-button solid"));
  }
  if (tools.length) pagedTiles(root, tools, "data");
  else root.append(make("div", "custom-empty", "NO DATA TOOLS"));
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
    make("small", "", "NO SIGNAL"));
  return cell;
}

function showLink() {
  const root = panel();
  if (snapshot.active.profile === "security-chief") {
    root.append(button("RADIO / IP", () => { openedTool = "native-radio"; render(); }, "micro-button solid"));
    root.append(button("MESSENGER / SSH", () => {
      openedTool = "secure-messenger";
      render();
    }, "micro-button solid"));
    root.append(sectionTop("CAMERAS", "0 / 4 LIVE"));
    if (focusedCamera !== null) {
      root.append(cameraTile(focusedCamera, true));
      root.append(button("◀ GRID", () => {
        focusedCamera = null;
        render();
      }, "micro-button"));
    } else {
      const quad = make("div", "camera-grid");
      for (let number = 1; number <= 4; number++) quad.append(cameraTile(number));
      root.append(quad);
    }
    return root;
  }
  root.append(button("RADIO / IP", () => {
    openedTool = "native-radio";
    render();
    ui.viewport.scrollTop = 0;
  }, "micro-button solid"));
  root.append(sectionTop("MESSENGER", "LOCAL / SSH"));
  const host = make("div", "rb-msg-root");
  root.append(host);
  if (window.RustBoyMessenger) {
    activeMessengerUI = window.RustBoyMessenger.mount(host, {compact:true});
  } else {
    host.append(make("div", "block-box", "MESSENGER MODULE UNAVAILABLE"));
  }
  if (hasTool("game")) root.append(button("GAME LINK", () => openTool("game"), "micro-button"));
  return root;
}

function showConf() {
  const root = panel();
  const custom = snapshot.active.profile === "custom";
  if (custom && paletteOpen) {
    root.append(sectionTop("TOOL PALETTE", "CUSTOM"));
    const back = button("◀ SETTINGS", () => { paletteOpen = false; render(); }, "micro-button compact");
    root.append(back);
    const pinned = new Set(allPinned());
    pagedTiles(root, snapshot.palette, "conf", (item) => {
      const exists = pinned.has(item.id);
      const tile = button("", () => exists ? removeCustom(item.id) : addCustom(item.id),
        "tool-pick");
      tile.disabled = item.availability === "planned" || busy;
      tile.append(make("strong", "", (exists ? "− " : "+ ") + item.name),
        make("small", "", item.availability === "planned"
          ? "PLANNED" : exists ? "PINNED" : "ADD"));
      return tile;
    }, 4);
    return root;
  }
  root.append(sectionTop("SETTINGS", "LOCAL"));
  const current = make("div", "block-box lit preview-row compact-profile");
  current.append(make("strong", "", activeProfile()?.title || "UNKNOWN"),
    button("CHANGE", openRoles, "micro-button compact"));
  root.append(current);
  const effects = make("div", "button-row");
  effects.append(
    button(reducedEffects ? "CRT OFF" : "CRT ON", () => {
      reducedEffects = !reducedEffects;
      document.body.classList.toggle("scanlines-off", reducedEffects);
      document.body.classList.toggle("glow-off", reducedEffects);
      render();
    }, "micro-button"),
    button("RELOAD", reload, "micro-button")
  );
  if (custom) effects.append(button("ADD TOOLS", () => { paletteOpen = true; render(); },
    "micro-button solid"));
  root.append(effects);
  return root;
}

function openTool(id, mode = "distance") {
  if (!hasTool(id)) {
    notify("This module is not pinned. Choose a profile containing it.", true);
    return;
  }
  openedTool = id;
  activeTaskUI = null;
  geoMode = mode;
  render();
  ui.viewport.scrollTop = 0;
  ui.viewport.focus({ preventScroll: true });
}

function inputForm(root, id) {
  const form = make("form", "micro-panel");
  form.id = "micro-lab-form";
  form.classList.add("micro-lab");
  const draftKey = snapshot.active.profile + ":" + id + ":" + (id === "geo-lab" ? geoMode : "default");
  const saveDraft = () => labDrafts.set(draftKey, Object.fromEntries(new FormData(form)));
  form.addEventListener("input", saveDraft);
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
  } else if (id === "byte-lab") {
    form.append(field("input", "HEXADECIMAL BYTES", "48656c6c6f", { maxLength: 3000, hex: true }));
  }
  const row = make("div", "form-actions");
  const run = make("button", "micro-button solid", id === "diag" ? "CHECK HOST" : id === "host" ? "READ STATUS" :
    ["game", "notes", "simulator", "library"].includes(id) ? "VIEW DETAILS" : "RUN");
  run.type = "submit";
  row.append(run);
  form.append(row);
  const output = make("pre", "mono-output lab-result", "");
  output.hidden = true;
  const edit = button("‹ EDIT INPUTS", () => {
    form.classList.remove("show-result");
    output.hidden = true;
    edit.hidden = true;
    (form.querySelector("input") || run).focus({ preventScroll: true });
  }, "micro-button compact lab-edit");
  edit.hidden = true;
  form.append(edit);
  output.setAttribute("aria-live", "polite");
  form.append(output);
  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    if (busy) return;
    saveDraft();
    busy = true;
    run.disabled = true;
    output.hidden = false;
    output.textContent = "WORKING…";
    const payload = { tool: id };
    if (id === "geo-lab") payload.mode = geoMode;
    for (const [key, val] of new FormData(form).entries()) payload[key] = val;
    try {
      const result = await api("/api/experiment", payload);
      if (openedTool === id && form.isConnected) {
        output.textContent = result.output;
        form.classList.add("show-result");
        edit.hidden = false;
        edit.focus({ preventScroll: true });
        ui.foot.textContent = "LOCAL TOOL / COMPLETE";
      }
    } catch (error) {
      if (openedTool === id && form.isConnected) {
        output.textContent = "ERROR / " + error.message;
        notify(error.message, true);
      }
    } finally {
      busy = false;
      run.disabled = false;
    }
  });
  const draft = labDrafts.get(draftKey);
  if (draft) for (const input of form.querySelectorAll("input[name]")) {
    if (Object.hasOwn(draft, input.name)) input.value = draft[input.name];
  }
  root.append(form);
}

function showTool() {
  const root = panel();
  if (openedTool === "native-radio") {
    const host = make("div", "rb-radio-host");
    root.append(host);
    if (window.RustBoyRadio) {
      activeRadioUI = window.RustBoyRadio.mount(host, { compact: true });
    } else {
      host.textContent = "NATIVE RADIO UI UNAVAILABLE";
    }
    return root;
  }
  if (openedTool === "secure-messenger") {
    const host = make("div", "rb-msg-root");
    root.append(host);
    if (window.RustBoyMessenger)
      activeMessengerUI = window.RustBoyMessenger.mount(host, {compact:true});
    return root;
  }
  if (openedTool === "assigned-duties" || openedTool === "mission-board") {
    const container = make("div", "task-manager-viewport");
    root.append(container);
    if (window.RustBoyTasks) {
      window.RustBoyTasks.mount(container, {
        profiles: snapshot.profiles,
        compact: true,
        onExit: () => {
          openedTool = null;
          activeTaskUI = null;
          render();
        }
      }).then((controller) => {
        if (container.isConnected) activeTaskUI = controller;
      });
    }
    return root;
  }
  const meta = toolInfo(openedTool);
  if (!meta) {
    root.append(make("p", "warning", "Unknown module. Return to the deck."));
    return root;
  }
  // The module title is already in the page heading; keep advanced help on
  // the dedicated tool screen rather than repeating long descriptions.
  const supported = ["geo-lab", "net-lab", "byte-lab", "host", "diag", "notes", "library", "simulator", "game"];
  if (openedTool === "reclamation" && window.RustBoyReclamation) {
    const host = make("div");
    root.append(host);
    window.RustBoyReclamation.mount(host, {
      compact: true,
      onMap: (site) => {
        if (!window.RustBoyMap || !window.RustBoyMap.queueSiteFocus(site.id)) return;
        setTab("map");
      }
    });
  } else if (openedTool === "net-lab" && window.RustBoyNetwork) {
    const container = make("div");
    root.append(container);
    activeNetworkUI = window.RustBoyNetwork.mount(container, {
      operations: snapshot.net_operations, compact: true,
      onStatus: (message) => { ui.foot.textContent = message; }
    });
  } else if (supported.includes(openedTool)) {
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
  if (activeNetworkUI) activeNetworkUI.destroy();
  activeNetworkUI = null;
  if (activeRadioUI) activeRadioUI.destroy();
  activeRadioUI = null;
  if (activeMessengerUI) activeMessengerUI.destroy();
  activeMessengerUI = null;
  if (window.RustBoyMap) window.RustBoyMap.unmount();
  renderHeading();
  ui.screen.dataset.depth = ["assigned-duties", "mission-board"].includes(openedTool)
    ? "tasks" : openedTool ? "detail" : "root";
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
    if (openedTool && openedTool !== "native-radio" && !hasTool(openedTool)) openedTool = null;
    focusedCamera = null;
    render();
    notify("SYNCED");
  } catch (error) {
    ui.code.textContent = "LOCAL / ERROR";
    ui.content.replaceChildren(make("div", "block-box error", error.message));
    notify(error.message, true);
  }
}

function openRoles() {
  if (!snapshot || busy) return;
  pendingProfile = snapshot.active.profile;
  rolePage = Math.max(0, Math.floor(snapshot.profiles.findIndex((r) =>
    r.id === pendingProfile) / 4));
  renderRolePage();
  selectRole(pendingProfile);
  if (!ui.rolesDialog.open) ui.rolesDialog.showModal();
}

function renderRolePage() {
  ui.rolesList.replaceChildren();
  const roles = snapshot.profiles.slice(rolePage * 4, rolePage * 4 + 4);
  roles.forEach((role, index) => {
    const row = button("", () => selectRole(role.id), "profile-row");
    row.dataset.role = role.id;
    row.append(make("span", "num", String(rolePage * 4 + index + 1).padStart(2, "0")));
    const info = make("div");
    info.append(make("strong", "", role.title));
    row.append(info);
    ui.rolesList.append(row);
  });
  const count = Math.max(1, Math.ceil(snapshot.profiles.length / 4));
  ui.rolePaging.textContent = (rolePage + 1) + " / " + count;
  ui.rolePrev.disabled = rolePage === 0;
  ui.roleNext.disabled = rolePage >= count - 1;
}

function selectRole(id) {
  pendingProfile = id;
  const choice = snapshot.profiles.find((role) => role.id === id);
  if (!choice) return;
  for (const row of ui.rolesList.querySelectorAll("[data-role]")) {
    row.setAttribute("aria-pressed", String(row.dataset.role === id));
  }
  ui.chosenName.textContent = choice.title;
  ui.chosenDetail.textContent = choice.id === "custom" ? "EMPTY" : choice.tools.length + " TOOLS";
  const unchanged = id === snapshot.active.profile
    && !(id === "custom" && snapshot.active.slots.length);
  ui.applyRole.disabled = unchanged || busy;
  ui.applyRole.textContent = id === "custom" && snapshot.active.profile === "custom"
    ? "RESET CUSTOM" : "LOAD ROLE";
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
    paletteOpen = false;
    notify("ROLE LOADED");
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
    notify("PINNED");
    render();
  } catch (error) {
    notify(error.message, true);
  } finally {
    busy = false;
    render();
  }
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
    notify("REMOVED");
  } catch (error) {
    notify(error.message, true);
  } finally {
    busy = false;
    render();
  }
}

function back() {
  if (ui.rolesDialog.open) {
    ui.rolesDialog.close();
  } else if (openedTool) {
    if (openedTool === "net-lab" && activeNetworkUI?.back()) return;
    const resultForm = ui.content.querySelector(".micro-lab.show-result");
    if (resultForm) {
      resultForm.querySelector(".lab-edit").click();
      return;
    }
    if (["assigned-duties", "mission-board"].includes(openedTool)
        && activeTaskUI?.back()) return;
    openedTool = null;
    activeTaskUI = null;
    render();
  } else if (focusedCamera !== null) {
    focusedCamera = null;
    render();
  } else if (tab !== "stat") {
    setTab("stat");
  }
}

function ensureVisible() {
  // Primary pages are normally no-scroll. At zoom or with unexpected content,
  // recover scroll access rather than clipping actions.
  ui.screen.classList.remove("micro-overflow");
  if (["root", "tasks"].includes(ui.screen.dataset.depth)
      && ui.content.scrollHeight > ui.viewport.clientHeight + 1) {
    ui.screen.classList.add("micro-overflow");
  }
}

function init() {
  const observer = new MutationObserver(ensureVisible);
  observer.observe(ui.content, { childList: true, subtree: true });
  window.addEventListener("resize", ensureVisible);
  ui.tabs.forEach((item) => item.addEventListener("click", () => setTab(item.dataset.tab)));
  ui.roleSwitch.addEventListener("click", openRoles);
  ui.back.addEventListener("click", back);
  ui.closeRoles.addEventListener("click", () => ui.rolesDialog.close());
  ui.cancelRoles.addEventListener("click", () => ui.rolesDialog.close());
  ui.applyRole.addEventListener("click", activateRole);
  ui.rolePrev.addEventListener("click", () => {
    if (rolePage === 0) return;
    rolePage--;
    renderRolePage();
    selectRole(snapshot.profiles[rolePage * 4].id);
  });
  ui.roleNext.addEventListener("click", () => {
    if ((rolePage + 1) * 4 >= snapshot.profiles.length) return;
    rolePage++;
    renderRolePage();
    selectRole(snapshot.profiles[rolePage * 4].id);
  });
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
