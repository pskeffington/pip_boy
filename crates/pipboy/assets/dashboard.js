"use strict";

// All profile and tool descriptors arrive from Rust PRESETS/PALETTE.
// User-supplied labels and outputs are always placed via textContent.
const byId = (id) => document.getElementById(id);
const ui = {
  status: byId("server-status"),
  roles: byId("role-list"),
  title: byId("workspace-title"),
  subtitle: byId("workspace-subtitle"),
  viewMode: byId("view-mode"),
  activeLabel: byId("active-label"),
  load: byId("load-role"),
  roleStat: byId("status-role"),
  moduleStat: byId("status-modules"),
  moduleCount: byId("module-count"),
  modules: byId("module-grid"),
  palette: byId("custom-palette"),
  paletteGrid: byId("palette-grid"),
  extension: byId("extension-content"),
  detailsTitle: byId("details-heading"),
  detailsDesc: byId("detail-description"),
  detailStatus: byId("detail-status"),
  testbench: byId("testbench"),
  form: byId("lab-form"),
  fields: byId("lab-fields"),
  run: byId("run-test"),
  kind: byId("lab-type"),
  output: byId("lab-output"),
  outputKind: byId("output-kind"),
  copy: byId("copy-output"),
  handoff: byId("detail-handoff"),
  manual: byId("manual-command"),
  confirm: byId("confirm-dialog"),
  confirmText: byId("confirm-description"),
  refresh: byId("refresh-state"),
  footer: byId("footer-message"),
  toast: byId("toast")
};
const symbols = {
  "host": "⌁", "notes": "≡", "library": "▤", "game": "▣",
  "net-lab": "⌘", "byte-lab": "0x", "simulator": "◈",
  "geo-lab": "⌖", "gpio": "◧", "radio": ")))",
  "ble": "⋈", "nfc": "⊞", "mesh": "◇"
};
let snapshot = null;
let previewId = null;
let selectedTool = null;
let selectedCamera = null;
let working = false;
let lastOutput = "";
let toastTimer = null;

const el = (tag, className, text) => {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = String(text);
  return node;
};

function notify(message, failure = false) {
  ui.toast.textContent = message;
  ui.toast.classList.toggle("error", failure);
  ui.toast.hidden = false;
  ui.footer.textContent = message;
  if (toastTimer !== null) window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => { ui.toast.hidden = true; }, 3900);
}

async function api(path, data) {
  const options = { cache: "no-store", credentials: "omit", headers: {} };
  if (data !== undefined) {
    options.method = "POST";
    options.headers["Content-Type"] = "application/json";
    options.body = JSON.stringify(data);
  }
  const response = await fetch(path, options);
  let payload;
  try {
    payload = await response.json();
  } catch {
    throw new Error("Invalid dashboard response");
  }
  if (!response.ok) throw new Error(payload.error || "Dashboard request failed");
  return payload;
}

function profile() {
  return snapshot?.profiles.find((entry) => entry.id === previewId) || null;
}

function selectedAvailable() {
  return previewId === snapshot?.active.profile;
}

function toolInfo(id) {
  return snapshot?.palette.find((entry) => entry.id === id) || null;
}

function visibleModuleIds() {
  const selected = profile();
  if (!selected) return [];
  return selectedAvailable()
    ? snapshot.active.slots.map((entry) => entry.tool)
    : selected.tools.slice();
}

function renderRoles() {
  ui.roles.replaceChildren();
  for (const [index, role] of snapshot.profiles.entries()) {
    const button = el("button", "role-card");
    button.type = "button";
    button.setAttribute("aria-pressed", String(role.id === previewId));
    button.setAttribute("aria-label", "Preview " + role.title);
    const numeral = el("span", "role-number", String(index + 1).padStart(2, "0"));
    const desc = el("span", "role-info");
    desc.append(el("span", "role-title", role.title));
    desc.append(el("span", "role-desc", role.id === "custom" ? "BLANK / ADVANCED" : role.description));
    button.append(numeral, desc);
    if (snapshot.active.profile === role.id) {
      button.append(el("span", "role-activated", "SAVED"));
    }
    button.addEventListener("click", () => {
      previewId = role.id;
      selectedTool = null;
      selectedCamera = null;
      lastOutput = "";
      renderAll();
    });
    ui.roles.append(button);
  }
}

function renderModules() {
  ui.modules.replaceChildren();
  const ids = visibleModuleIds();
  ui.moduleCount.textContent = String(ids.length).padStart(2, "0") + " MODULES";
  ui.moduleStat.textContent = String(ids.length).padStart(2, "0") + " PINNED";
  if (ids.length === 0) {
    const blank = el("div", "empty-state");
    blank.append(
      el("strong", "", "EMPTY WORKSPACE // READY FOR YOUR TOOLS"),
      el("p", "", selectedAvailable()
        ? "Custom has no modules by default. Add tools from the palette below."
        : "Custom starts empty. Load it to open the advanced tool palette.")
    );
    ui.modules.append(blank);
  }
  if (!ids.includes(selectedTool)) selectedTool = ids[0] || null;
  for (const [index, id] of ids.entries()) {
    const info = toolInfo(id);
    if (!info) continue;
    const card = el("button", "module-card");
    card.type = "button";
    card.setAttribute("aria-pressed", String(id === selectedTool));
    card.setAttribute("aria-label", "Inspect " + info.name);
    const top = el("span", "module-top");
    top.append(
      el("span", "module-glyph", symbols[id] || "□"),
      el("span", "module-index", "M" + String(index + 1).padStart(2, "0"))
    );
    const base = el("span");
    base.append(
      el("span", "module-name", info.name),
      el("span", "module-category " + info.availability, info.availability.replace(/_/g, " "))
    );
    card.append(top, base);
    card.addEventListener("click", () => {
      selectedTool = id;
      lastOutput = "";
      renderModules();
      renderDetail();
    });
    ui.modules.append(card);
  }
  renderCustomPalette();
}

function renderCustomPalette() {
  const activeCustom = previewId === "custom" && selectedAvailable();
  ui.palette.hidden = !activeCustom;
  ui.paletteGrid.replaceChildren();
  if (!activeCustom) return;
  const pinned = new Set(snapshot.active.slots.map((slot) => slot.tool));
  for (const item of snapshot.palette) {
    const unavailable = item.availability === "planned";
    const alreadyPinned = pinned.has(item.id);
    const button = el("button", "palette-button");
    button.type = "button";
    button.disabled = unavailable || alreadyPinned || working;
    button.append(
      el("span", "plus", unavailable ? "—" : alreadyPinned ? "✓" : "+"),
      el("span", "", item.name)
    );
    button.title = unavailable
      ? "Not implemented — hardware capability unavailable"
      : alreadyPinned ? "Already pinned" : "Pin this tool (does not execute it)";
    button.addEventListener("click", () => addCustomTool(item.id));
    ui.paletteGrid.append(button);
  }
}

function renderExtension() {
  ui.extension.replaceChildren();
  const selected = profile();
  if (!selected) return;
  if (selected.id === "reclamation-scout") {
    const map = el("div", "map-preview");
    const heading = el("div", "map-preview-header");
    heading.append(
      el("strong", "", "SCOUT / NAVIGATION VIEWPORT"),
      el("span", "", "RUSTGIS NOT ATTACHED")
    );
    const canvas = el("div", "map-canvas");
    canvas.setAttribute("role", "img");
    canvas.setAttribute("aria-label", "Empty reference grid. No live map or position is displayed.");
    canvas.append(el("span", "map-canvas-label", "REFERENCE GRID ONLY / NO LIVE MAP"));
    map.append(heading, canvas);
    ui.extension.append(map);
    return;
  }
  if (selected.id === "security-chief") {
    const wall = el("div", "camera-wall" + (selectedCamera !== null ? " focused" : ""));
    wall.append(el("div", "camera-wall-heading", "SECURITY / FOUR-CAMERA UI FIXTURE — NO LIVE VIDEO"));
    const grid = el("div", "camera-grid");
    const available = selectedCamera === null ? [1, 2, 3, 4] : [selectedCamera];
    for (const number of available) {
      const cell = el("button", "camera-cell");
      cell.type = "button";
      cell.setAttribute("aria-label", "Camera " + number + " placeholder; " +
        (selectedCamera === null ? "focus" : "return to grid"));
      cell.append(
        el("strong", "", "CAM 0" + number),
        el("span", "", "NO FEED / NOT CONNECTED")
      );
      cell.addEventListener("click", () => {
        selectedCamera = selectedCamera === null ? number : null;
        renderExtension();
      });
      grid.append(cell);
    }
    wall.append(grid, el("div", "camera-hint",
      selectedCamera === null
        ? "Click a tile to test one-press focus. No source or camera connection."
        : "Click focused tile or press Esc to return to the four-camera grid."
    ));
    ui.extension.append(wall);
    return;
  }
  const placeholder = el("div", "extension-placeholder");
  placeholder.append(
    el("span", "port-symbol", "◇"),
    el("div", "")
  );
  placeholder.lastElementChild.append(
    el("strong", "", "EXTENSION BAY / UNSLOTTED"),
    el("p", "", "Role-specific hardware and advanced modules will mount here after their acceptance gates. Nothing is auto-connected.")
  );
  ui.extension.append(placeholder);
}

function field(name, label, value, extra = "") {
  const input = '<input id="lab-' + name + '" name="' + name + '" value="' + value +
    '" maxlength="128" ' + extra + ' required>';
  return '<label for="lab-' + name + '">' + label + input + "</label>";
}

const forms = Object.freeze({
  "geo-lab":
    '<label for="lab-mode">OPERATION<select name="mode" id="lab-mode">' +
    '<option value="distance">Calculate distance</option>' +
    '<option value="waypoint">Export GeoJSON waypoint</option></select></label>' +
    '<div class="lab-row">' +
    field("lat", "START LATITUDE", "42.0", 'inputmode="decimal"') +
    field("lon", "START LONGITUDE", "-70.75", 'inputmode="decimal"') +
    '</div><div id="destination-fields" class="lab-row">' +
    field("lat2", "END LATITUDE", "42.01", 'inputmode="decimal"') +
    field("lon2", "END LONGITUDE", "-70.76", 'inputmode="decimal"') +
    '</div><div id="waypoint-fields" hidden>' +
    field("name", "WAYPOINT LABEL", "Camp Alpha") + "</div>",
  "net-lab":
    field("input", "IPV4 / CIDR", "192.168.1.50/24", 'autocomplete="off" spellcheck="false"'),
  "byte-lab":
    field("input", "HEX BYTES (UP TO 4096)", "48656c6c6f", 'autocomplete="off" spellcheck="false"'),
  "host": '<p class="workspace-subtitle">Sample local host status on demand. No continuous background collection.</p>',
  "game": '<p class="workspace-subtitle">Manual connection only. The browser does not contact a PlayStation.</p>',
  "library": '<p class="workspace-subtitle">Shows the offline reference handoff. No network fetch.</p>',
  "notes": '<p class="workspace-subtitle">Shows the local notes command. Your note contents stay private.</p>',
  "simulator": '<p class="workspace-subtitle">Use deterministic fixtures. No physical interface is enabled.</p>'
});

function renderDetail() {
  ui.detailsTitle.textContent = "Select a module";
  ui.detailsDesc.textContent = "Choose a tile to inspect its scope and run a local test.";
  ui.detailStatus.replaceChildren();
  ui.testbench.hidden = true;
  ui.handoff.hidden = true;
  ui.fields.replaceChildren();
  ui.form.hidden = false;
  const info = selectedTool && toolInfo(selectedTool);
  if (!info) {
    ui.detailsDesc.textContent = previewId === "custom"
      ? "Advanced Custom starts empty. Load Custom, then add tools from its palette."
      : "Select one of the role's pinned modules to inspect it.";
    return;
  }
  ui.detailsTitle.textContent = info.name;
  ui.detailsDesc.textContent = info.description;
  const capability = el("span", "tag" +
    (info.availability === "ready" ? "" : " amber"),
    info.availability.replace(/_/g, " ").toUpperCase());
  const state = el("span", "tag" + (selectedAvailable() ? "" : " amber"),
    selectedAvailable() ? "PINNED / ACTIVE" : "TEMPLATE PREVIEW");
  ui.detailStatus.append(capability, state);
  if (selectedAvailable() && snapshot.active.profile === "custom") {
    const slot = snapshot.active.slots.find((s) => s.tool === selectedTool);
    if (slot) {
      const remove = el("button", "mini-button", "Remove slot");
      remove.type = "button";
      remove.setAttribute("aria-label", "Remove " + info.name + " from Custom");
      remove.addEventListener("click", () => removeCustomTool(slot.id));
      ui.detailStatus.append(remove);
    }
  }
  if (!selectedAvailable()) {
    ui.handoff.hidden = false;
    ui.manual.textContent = "Load this role to activate its pinned module set.";
    return;
  }
  if (info.availability === "planned") {
    ui.handoff.hidden = false;
    ui.manual.textContent = "Not implemented: no hardware access";
    return;
  }
  if (Object.hasOwn(forms, selectedTool)) {
    ui.testbench.hidden = false;
    ui.fields.innerHTML = forms[selectedTool]; // Only static authored form markup.
    ui.run.textContent = info.availability === "ready" ? "Run local test" : "Show safe handoff";
    ui.run.disabled = working;
    ui.kind.textContent = info.availability === "ready" ? "OFFLINE" : "OPT-IN / NOT CONNECTED";
    ui.output.textContent = lastOutput || "Awaiting explicit action.";
    ui.outputKind.textContent = lastOutput ? "RESULT" : "IDLE";
    ui.copy.disabled = !lastOutput;
    const selector = byId("lab-mode");
    if (selector) selector.addEventListener("change", toggleGeoMode);
    toggleGeoMode();
  } else {
    ui.handoff.hidden = false;
    ui.manual.textContent = info.command;
  }
}

function toggleGeoMode() {
  const selector = byId("lab-mode");
  const destination = byId("destination-fields");
  const waypoint = byId("waypoint-fields");
  if (!selector || !destination || !waypoint) return;
  const exportPoint = selector.value === "waypoint";
  destination.hidden = exportPoint;
  waypoint.hidden = !exportPoint;
  for (const input of destination.querySelectorAll("input")) input.required = !exportPoint;
  for (const input of waypoint.querySelectorAll("input")) input.required = exportPoint;
}

function renderAll() {
  const current = profile();
  if (!current) return;
  ui.status.innerHTML = '<span class="live-dot" aria-hidden="true"></span> LOCAL / READY';
  ui.title.textContent = current.title;
  ui.subtitle.textContent = current.description;
  ui.viewMode.textContent = selectedAvailable() ? "ACTIVE STATION" : "TEMPLATE PREVIEW";
  ui.activeLabel.textContent = selectedAvailable() ? "ACTIVE / SAVED" : "PREVIEW / NOT LOADED";
  ui.activeLabel.className = "active-pill " + (selectedAvailable() ? "loaded" : "preview");
  ui.roleStat.textContent = current.id.toUpperCase().replace(/-/g, " ");
  const isSame = selectedAvailable();
  const customReset = current.id === "custom" && snapshot.active.slots.length > 0;
  ui.load.disabled = working || (isSame && !customReset);
  ui.load.textContent = isSame
    ? (customReset ? "Reset Custom" : "Active template")
    : (current.id === "custom" ? "Load Custom" : "Load station");
  renderRoles();
  renderModules();
  renderExtension();
  renderDetail();
}

async function reload() {
  try {
    snapshot = await api("/api/state");
    if (!previewId || !snapshot.profiles.some((p) => p.id === previewId)) {
      previewId = snapshot.active.profile;
    }
    if (!selectedTool) selectedTool = visibleModuleIds()[0] || null;
    renderAll();
    notify("Local workbench synchronized.");
  } catch (error) {
    ui.status.textContent = "LOCAL / UNAVAILABLE";
    notify(error.message, true);
  }
}

async function changeProfile(id) {
  if (working) return;
  working = true;
  ui.load.disabled = true;
  try {
    snapshot = await api("/api/profile", { id, confirm: true });
    previewId = id;
    selectedTool = null;
    selectedCamera = null;
    lastOutput = "";
    notify("Loaded " + profile().title + ". No tools executed.");
    renderAll();
  } catch (error) {
    notify(error.message, true);
  } finally {
    working = false;
    if (snapshot) renderAll();
  }
}

function loadSelected() {
  if (!snapshot || working || ui.load.disabled) return;
  const current = profile();
  if (!current) return;
  ui.confirmText.textContent = "Replace the current pinned modules with " + current.title +
    "? This changes only the local workspace layout. Notes, game sessions, camera connections and saved permissions are not touched.";
  if (typeof ui.confirm.showModal === "function") {
    // Reset each time: dismissing via Escape must never reuse a prior approval.
    ui.confirm.returnValue = "cancel";
    ui.confirm.showModal();
  } else if (window.confirm(ui.confirmText.textContent)) {
    changeProfile(current.id);
  }
}

async function addCustomTool(id) {
  if (working) return;
  working = true;
  try {
    snapshot = await api("/api/custom/add", { id });
    selectedTool = id;
    lastOutput = "";
    notify("Tool pinned to Custom. It was not executed.");
  } catch (error) {
    notify(error.message, true);
  } finally {
    working = false;
    renderAll();
  }
}

async function removeCustomTool(id) {
  if (working) return;
  working = true;
  try {
    snapshot = await api("/api/custom/remove", { id });
    selectedTool = null;
    lastOutput = "";
    notify("Tool unpinned. No external data changed.");
  } catch (error) {
    notify(error.message, true);
  } finally {
    working = false;
    renderAll();
  }
}

async function runLab(event) {
  event.preventDefault();
  if (!selectedTool || !selectedAvailable() || working) return;
  const tool = selectedTool;
  const input = Object.fromEntries(new FormData(ui.form).entries());
  working = true;
  ui.run.disabled = true;
  ui.run.textContent = "Processing locally…";
  ui.outputKind.textContent = "RUNNING";
  ui.output.textContent = "Executing the selected allowlisted local tool.";
  try {
    const result = await api("/api/experiment", { tool, ...input });
    if (selectedTool !== tool || !selectedAvailable()) return;
    lastOutput = result.output;
    ui.output.textContent = lastOutput;
    ui.outputKind.textContent = result.kind.toUpperCase();
    ui.copy.disabled = false;
    ui.footer.textContent = result.kind === "computed"
      ? "Local result computed; no external data uploaded"
      : "Information only; no connection established";
  } catch (error) {
    if (selectedTool !== tool || !selectedAvailable()) return;
    lastOutput = "";
    ui.output.textContent = error.message;
    ui.outputKind.textContent = "ERROR";
    ui.copy.disabled = true;
    notify(error.message, true);
  } finally {
    working = false;
    if (selectedTool === tool && selectedAvailable()) {
      ui.run.disabled = false;
      const info = toolInfo(tool);
      ui.run.textContent = info?.availability === "ready" ? "Run local test" : "Show safe handoff";
    }
  }
}

async function copyOutput() {
  if (!lastOutput || !navigator.clipboard || !window.isSecureContext) {
    notify("Clipboard unavailable. Select the output text to copy.", true);
    return;
  }
  try {
    await navigator.clipboard.writeText(lastOutput);
    notify("Local output copied.");
  } catch {
    notify("Clipboard permission denied.", true);
  }
}

ui.load.addEventListener("click", loadSelected);
ui.refresh.addEventListener("click", reload);
ui.form.addEventListener("submit", runLab);
ui.copy.addEventListener("click", copyOutput);
ui.confirm.addEventListener("close", () => {
  if (ui.confirm.returnValue === "replace" && profile()) changeProfile(previewId);
});
document.addEventListener("keydown", (event) => {
  if (!snapshot) return;
  if (event.key === "Escape" && selectedCamera !== null) {
    selectedCamera = null;
    renderExtension();
    return;
  }
  if (!event.altKey || event.ctrlKey || event.metaKey) return;
  if (event.key === "ArrowUp" || event.key === "ArrowDown") {
    event.preventDefault();
    const i = snapshot.profiles.findIndex((p) => p.id === previewId);
    const delta = event.key === "ArrowUp" ? -1 : 1;
    previewId = snapshot.profiles[(i + delta + snapshot.profiles.length) % snapshot.profiles.length].id;
    selectedTool = null;
    selectedCamera = null;
    lastOutput = "";
    renderAll();
  } else if (event.key === "Enter") {
    event.preventDefault();
    loadSelected();
  }
});

reload();
