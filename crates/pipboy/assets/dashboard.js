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
  detailInfo: byId("detail-info"),
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
  "mission-board": "▣", "host": "⌁", "notes": "≡", "library": "▤", "game": "▣",
  "net-lab": "⌘", "byte-lab": "0x", "simulator": "◈",
  "geo-lab": "⌖", "reclamation": "▧", "gpio": "◧", "radio": ")))",
  "ble": "⋈", "nfc": "⊞", "mesh": "◇"
};
let snapshot = null;
let previewId = null;
let selectedTool = null;
let activeNetworkUI = null;
let selectedCamera = null;
let working = false;
let lastOutput = "";
let rolePage = 0, modulePage = 0, palettePage = 0;
let activeStage = "modules";
let activePane = "workspace";
let settingsReturnStage = "modules";
let settingsReturnPane = "workspace";
let settingsReturnRadioPaneOpen = false;
let settingsReturnExtensionMode = "map";
let desktopMessenger = null;
let desktopCommandUI = null;
let initialWorkspaceLoaded = false;
let desktopRadioUI = null;
let cameraWallUI = null;
let radioPaneOpen = false;
let extensionMode = "map";
let queuedSiteId = null;
const PAGE_SIZE = 6;
const rolesPerPage = () => window.innerHeight <= 480 ? 2 : PAGE_SIZE;
const cardsPerPage = () => window.innerHeight <= 480 ? 4 : PAGE_SIZE;
function pager(id, total, current, setter, pageSize = PAGE_SIZE) {
  const host = byId(id);
  host.replaceChildren();
  const pages = Math.max(1, Math.ceil(total / pageSize));
  if (pages < 2) return;
  const previous = el("button", "deck-button", "◀ PREV");
  const next = el("button", "deck-button", "NEXT ▶");
  previous.type = next.type = "button";
  previous.disabled = current <= 0;
  next.disabled = current >= pages - 1;
  previous.addEventListener("click", () => setter(current - 1));
  next.addEventListener("click", () => setter(current + 1));
  host.append(previous, el("span", "deck-count", `${current + 1} / ${pages}`), next);
}
function activateStage(stage) {
  if (stage === "palette" && snapshot?.active.profile !== "custom") stage = "settings";
  if (!["command", "modules", "viewport", "messages", "settings", "palette", "adapter"].includes(stage)) return;
  const previous = activeStage;
  const wasSettings = previous === "settings" || previous === "palette";
  const entersSettings = stage === "settings" || stage === "palette";
  const leavingAdapter = stage !== "adapter" && stage !== "settings" &&
    (previous === "adapter" || (wasSettings && settingsReturnStage === "adapter"));
  if (leavingAdapter) {
    void api("/api/host/foreground", {package_id:null, close_settings:wasSettings})
      .then(applyHostForegroundResult)
      .catch(error => notify(`HOST PACKAGE SUSPEND ERROR: ${error.message}`, true));
  } else if (wasSettings !== entersSettings) {
    syncHostSettings(entersSettings);
  }
  if (stage === "settings" && previous !== "settings" && previous !== "palette") {
    settingsReturnStage = previous;
    settingsReturnPane = activePane;
    settingsReturnRadioPaneOpen = radioPaneOpen;
    settingsReturnExtensionMode = extensionMode;
  }
  if (stage === "settings") activatePane("workspace");
  // Radio is a subview, not a sticky override of later VIEWPORT visits.
  if (stage !== "viewport") radioPaneOpen = false;
  if (stage !== "viewport" && cameraWallUI) { cameraWallUI.destroy(); cameraWallUI = null; }
  if (!["settings", "palette"].includes(stage) && previewId !== snapshot?.active.profile) {
    previewId = snapshot.active.profile;
    renderRoles();
  }
  activeStage = stage;
  for (const button of document.querySelectorAll("[data-stage-target]")) {
    button.setAttribute("aria-pressed", String(button.dataset.stageTarget === stage));
  }
  for (const section of document.querySelectorAll(".stage-surface")) section.hidden = section.dataset.stage !== stage;
  if (stage === "viewport" && snapshot && !radioPaneOpen) renderExtension();
  else if (previous === "viewport" && window.RustBoyMap) window.RustBoyMap.unmount();
  if (stage === "adapter") renderHostAdapterSurface();
  syncRadioSubview();
  if (stage === "command" && snapshot) renderCommandCenter();
  // First local journal read occurs only after operator opens MESSAGES.
  if (stage === "messages" && !desktopMessenger && window.RustBoyMessenger) {
    desktopMessenger = window.RustBoyMessenger.mount(byId("messenger-dock"));
  }
}
function syncHostSettings(settings_open) {
  void api("/api/host/settings", { settings_open }).then(applyHostForegroundResult).catch(error =>
    notify(`Host Settings state could not be saved: ${error.message}`, true));
}

function applyHostForegroundResult(result) {
  if (!snapshot?.host_packages || !result) return;
  snapshot.host_packages = { ...snapshot.host_packages, ...result };
  updateHostPackageSelect(snapshot.host_packages);
  if (activeStage === "adapter") renderHostAdapterSurface();
}

function updateHostPackageSelect(hostState) {
  const select = byId("host-package-select");
  if (!select) return;
  const packages = Array.isArray(hostState?.packages) ? hostState.packages : [];
  const selected = hostState?.selected_package_id || "";
  select.replaceChildren();
  const placeholder = document.createElement("option");
  placeholder.value = "";
  placeholder.textContent = packages.length ? "HOST WORKBENCH" : "NO PACKAGES";
  select.append(placeholder);
  for (const entry of packages) {
    const id = typeof entry?.package_id === "string" ? entry.package_id : "";
    if (id) {
      const option = document.createElement("option");
      option.value = id;
      option.textContent = id.split(".").pop().toUpperCase();
      select.append(option);
    }
  }
  select.value = selected;
  select.disabled = packages.length === 0;
  select.title = packages.length ? "Select a mounted package foreground" : "No trusted adapter is mounted";
}

function renderHostAdapterSurface() {
  const state = snapshot?.host_packages;
  const packageId = state?.selected_package_id;
  byId("host-adapter-id").textContent = packageId || "HOST";
  byId("host-adapter-status").textContent = packageId
    ? "Package view is rendered by the host from bounded data."
    : "No package is mounted.";
  renderPackageView(byId("host-adapter-view"), state?.foreground_view);
}

function renderPackageView(root, view) {
  root.replaceChildren();
  if (!view || view.version !== 1 || typeof view.title !== "string" ||
      !Array.isArray(view.blocks) || view.blocks.length > 24) {
    root.textContent = "No valid foreground view is available.";
    return;
  }
  const validText = (value, limit) => typeof value === "string" && value.trim().length > 0 &&
    value.length <= limit && !/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/.test(value);
  const validBlock = block => block && (
    (block.kind === "heading" && validText(block.text, 512)) ||
    (block.kind === "text" && validText(block.text, 512)) ||
    (block.kind === "status" && validText(block.label, 80) && validText(block.value, 160)) ||
    (block.kind === "list" && validText(block.label, 80) && Array.isArray(block.items) &&
      block.items.length > 0 && block.items.length <= 24 && block.items.every(item => validText(item, 160)))
  );
  if (!validText(view.title, 80) || view.blocks.length === 0 || !view.blocks.every(validBlock)) {
    root.textContent = "No valid foreground view is available.";
    return;
  }
  const title = el("h3", "package-view-title", view.title);
  root.append(title);
  for (const block of view.blocks) {
    if (block.kind === "heading") {
      root.append(el("h4", "package-view-heading", block.text));
    } else if (block.kind === "text") {
      root.append(el("p", "package-view-text", block.text));
    } else if (block.kind === "status") {
      const row = el("div", "package-view-status");
      row.append(el("span", "package-view-label", block.label),
        el("strong", "package-view-value", block.value));
      root.append(row);
    } else if (block.kind === "list") {
      root.append(el("h4", "package-view-heading", block.label));
      const list = el("ul", "package-view-list");
      for (const item of block.items) list.append(el("li", "", item));
      root.append(list);
    }
  }
}

function selectHostPackage(packageId) {
  if (!packageId) {
    if (activeStage === "adapter") activateStage("modules");
    else void api("/api/host/foreground", {package_id:null}).then(applyHostForegroundResult).catch(error =>
      notify(`HOST PACKAGE CLEAR ERROR: ${error.message}`, true));
    return;
  }
  void api("/api/host/foreground", {package_id:packageId}).then(result => {
    snapshot.host_packages = {
      ...snapshot.host_packages,
      selected_package_id: result.selected_package_id,
      foreground: result.foreground,
      settings_open: result.settings_open,
      foreground_view: result.foreground_view
    };
    updateHostPackageSelect(snapshot.host_packages);
    activateStage("adapter");
  }).catch(error => {
    updateHostPackageSelect(snapshot.host_packages);
    notify(`HOST PACKAGE ERROR: ${error.message}`, true);
  });
}
function returnFromSettings() {
  const stage = ["command", "modules", "viewport", "messages", "adapter"].includes(settingsReturnStage)
    ? settingsReturnStage
    : "modules";
  const pane = settingsReturnPane === "inspector" && stage === "modules"
    ? "inspector"
    : "workspace";
  if (stage === "viewport") {
    radioPaneOpen = settingsReturnRadioPaneOpen;
    extensionMode = settingsReturnExtensionMode;
  }
  activateStage(stage);
  activatePane(pane);
}
// Desktop is the vault command station. Role authority comes from Rust's
// persisted mission-board slot, never from a browser toggle or inferred title.
// The task editor reads local storage on explicit entry; it never sends by itself.
function renderCommandCenter() {
  if (!snapshot) return;
  const manager = snapshot.active.slots.some(slot => slot.tool === "mission-board");
  byId("command-authority").textContent = manager ? "ASSIGN / DISPATCH ENABLED" : "ROLE-SCOPED DUTIES";
  byId("command-current-role").textContent = profile()?.title || "UNKNOWN";
  byId("command-policy").textContent = manager
    ? "Create or reassign missions here; review receipts before considering an operator notified."
    : "Review and update your assigned duties. Only a station with Task Control can issue or reassign jobs.";
  const host = byId("command-board");
  if (!host) return;
  if (!window.RustBoyTasks) {
    host.textContent = "Task Control is unavailable. Reload this command center to retry.";
    return;
  }
  if (desktopCommandUI?.profile === snapshot.active.profile &&
      desktopCommandUI.root.isConnected) {
    if (desktopCommandUI.controller) void desktopCommandUI.controller.refresh();
    return;
  }
  // Separate the mount node so an in-flight response from the old role cannot
  // repaint the next role's queue after a confirmed profile switch.
  const root = el("div", "command-task-mount");
  host.replaceChildren(root);
  const current = {profile: snapshot.active.profile, root, controller: null};
  desktopCommandUI = current;
  void window.RustBoyTasks.mount(root, {profiles: snapshot.profiles}).then(controller => {
    if (desktopCommandUI === current) current.controller = controller;
  }).catch(error => {
    if (desktopCommandUI === current && root.isConnected)
      root.textContent = "Mission board unavailable: " + error.message;
  });
}

// The RADIO dock is a VIEWPORT subview, not a permanent second page below it.
// Radio UI mounting is lazy; leaving VIEWPORT stops local status polling only.
// The Rust native engine and any explicitly playing audio remain undisturbed.
function syncRadioSubview() {
  const dock = byId("radio-dock");
  const toggle = byId("radio-view-toggle");
  if (!dock || !toggle) return;
  const visible = radioPaneOpen && activeStage === "viewport";
  if (visible && cameraWallUI) { cameraWallUI.destroy(); cameraWallUI = null; }
  ui.extension.hidden = visible;
  dock.hidden = !visible;
  toggle.textContent = visible ? "◀ ACTIVE VIEW" : "RADIO / IP";
  toggle.setAttribute("aria-pressed", String(visible));
  toggle.setAttribute("aria-label", visible ? "Return to active viewport" : "Open native IP radio");
  if (visible && !desktopRadioUI && window.RustBoyRadio) {
    dock.replaceChildren();
    desktopRadioUI = window.RustBoyRadio.mount(dock, {compact:true});
  } else if (visible && !window.RustBoyRadio) {
    dock.textContent = "Radio controls unavailable. Reload the desktop to retry.";
  } else if (!visible) {
    if (desktopRadioUI) desktopRadioUI.destroy();
    desktopRadioUI = null;
    dock.replaceChildren();
  }
}
function toggleRadioSubview() {
  if (radioPaneOpen && activeStage === "viewport") {
    radioPaneOpen = false;
    syncRadioSubview();
    if (snapshot) renderExtension();
    return;
  }
  // This button lives in the global stage navigation: it must work from
  // MODULES, MESSAGES and SETTINGS as well as from the normal viewport.
  radioPaneOpen = true;
  if (window.RustBoyMap) window.RustBoyMap.unmount();
  // Remove stale map markup too: map-only CSS uses :has() selectors which
  // otherwise keep the Radio navigation/header hidden after switching.
  ui.extension.replaceChildren();
  activatePane("workspace");
  activateStage("viewport");
}
function activatePane(pane) {
  if (pane !== "workspace" && cameraWallUI) { cameraWallUI.destroy(); cameraWallUI = null; }
  activePane = pane;
  document.querySelector(".dashboard").dataset.pane = pane;
  if (pane === "workspace" && activeStage === "viewport" && !radioPaneOpen && snapshot?.active.profile === "security-chief" && !cameraWallUI) renderExtension();
  for (const button of document.querySelectorAll("[data-pane-target]")) button.setAttribute("aria-pressed", String(button.dataset.paneTarget === pane));
}

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
  return snapshot?.profiles.find((entry) => entry.id === snapshot.active.profile) || null;
}

function pendingProfile() {
  return snapshot?.profiles.find((entry) => entry.id === previewId) || null;
}

function selectedAvailable() {
  return Boolean(profile());
}

function renderSettingsSelection() {
  if (!snapshot) return;
  const chosen = pendingProfile();
  const active = profile();
  byId("settings-current-role").textContent = active?.title || "UNKNOWN";
  byId("open-palette").hidden = snapshot.active.profile !== "custom";
  const same = chosen?.id === snapshot.active.profile;
  const resetCustom = same && chosen?.id === "custom" && snapshot.active.slots.length > 0;
  byId("settings-selection").textContent = !chosen ? "Choose a role."
    : same ? "Current: " + chosen.title
      : "Selected: " + chosen.title;
  ui.load.disabled = working || (same && !resetCustom) || !chosen;
  ui.load.textContent = resetCustom ? "Reset Custom"
    : same ? "Loaded" : "LOAD PROFILE";
}

function toolInfo(id) {
  return snapshot?.palette.find((entry) => entry.id === id) || null;
}

function visibleModuleIds() {
  return snapshot ? snapshot.active.slots.map((entry) => entry.tool) : [];
}

function renderRoles() {
  ui.roles.replaceChildren();
  const roles = snapshot.profiles;
  const size = rolesPerPage();
  rolePage = Math.min(rolePage, Math.max(0, Math.ceil(roles.length / size) - 1));
  for (const [index, role] of roles.entries()) {
    if (Math.floor(index / size) !== rolePage) continue;
    const button = el("button", "role-card");
    button.type = "button";
    button.setAttribute("aria-pressed", String(role.id === previewId));
    button.setAttribute("aria-label", "Preview " + role.title);
    const numeral = el("span", "role-number", String(index + 1).padStart(2, "0"));
    const desc = el("span", "role-info");
    desc.append(el("span", "role-title", role.title));
    button.title = role.description || role.title;
    button.append(numeral, desc);
    if (snapshot.active.profile === role.id) {
      button.append(el("span", "role-activated", "SAVED"));
    }
    button.addEventListener("click", () => {
      // Preview remains inside Settings; active role, pins, and live tools do not change.
      previewId = role.id;
      renderRoles();
    });
    ui.roles.append(button);
  }
  pager("role-pager", roles.length, rolePage, (page) => { rolePage = page; renderRoles(); }, size);
  renderSettingsSelection();
}

function renderModules() {
  ui.modules.replaceChildren();
  const ids = visibleModuleIds();
  ui.moduleCount.textContent = String(ids.length);
  ui.moduleStat.textContent = String(ids.length) + " PINNED";
  if (ids.length === 0) {
    const blank = el("div", "empty-state");
    blank.append(
      el("strong", "", "No tools yet"),
      el("p", "", selectedAvailable()
        ? "Open Settings to add tools."
        : "Load Custom in Settings to add tools.")
    );
    ui.modules.append(blank);
  }
  if (!ids.includes(selectedTool)) selectedTool = ids[0] || null;
  const size = cardsPerPage();
  modulePage = Math.min(modulePage, Math.max(0, Math.ceil(ids.length / size) - 1));
  for (const [index, id] of ids.entries()) {
    if (Math.floor(index / size) !== modulePage) continue;
    const info = toolInfo(id);
    if (!info) continue;
    const card = el("button", "module-card");
    card.type = "button";
    card.setAttribute("aria-pressed", String(id === selectedTool));
    card.setAttribute("aria-label", "Inspect " + info.name + " (" +
      info.availability.replace(/_/g, " ") + ")");
    card.title = info.description || info.name;
    const top = el("span", "module-top", symbols[id] || "□");
    const base = el("span");
    base.append(el("span", "module-name", info.name));
    if (info.availability !== "ready") {
      const shortStatus = {simulated:"SIM",explicit_connection:"OPT-IN",planned:"PLANNED"};
      base.append(el("span", "module-category " + info.availability,
        shortStatus[info.availability] || info.availability.replace(/_/g, " ")));
    }
    card.append(top, base);
    card.addEventListener("click", () => {
      selectedTool = id;
      lastOutput = "";
      ui.detailInfo.open = false;
      renderModules();
      renderDetail();
      activatePane("inspector");
    });
    ui.modules.append(card);
  }
  pager("module-pager", ids.length, modulePage, (page) => {
    modulePage = page;
    selectedTool = ids[page * size] || null;
    lastOutput = "";
    renderModules();
    renderDetail();
  }, size);
  renderCustomPalette();
}

function renderCustomPalette() {
  const activeCustom = snapshot.active.profile === "custom";
  ui.paletteGrid.replaceChildren();
  if (!activeCustom) return;
  const pinned = new Set(snapshot.active.slots.map((slot) => slot.tool));
  const all = snapshot.palette;
  const size = cardsPerPage();
  palettePage = Math.min(palettePage, Math.max(0, Math.ceil(all.length / size) - 1));
  for (const [index, item] of all.entries()) {
    if (Math.floor(index / size) !== palettePage) continue;
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
  pager("palette-pager", all.length, palettePage, (page) => { palettePage = page; renderCustomPalette(); }, size);
}

function extensionTabs(options, selected) {
  const nav = el("nav", "extension-tabs");
  nav.setAttribute("aria-label", "Module viewport panels");
  for (const [id, label] of options) {
    const button = el("button", "", label);
    button.type = "button";
    button.setAttribute("aria-pressed", String(id === selected));
    button.addEventListener("click", () => { extensionMode = id; renderExtension(); });
    nav.append(button);
  }
  return nav;
}

function renderExtension() {
  if (cameraWallUI) cameraWallUI.destroy();
  cameraWallUI = null;
  if (window.RustBoyMap) window.RustBoyMap.unmount();
  ui.extension.replaceChildren();
  const selected = profile();
  if (!selected) return;
  if (selected.id === "overseer") {
    // A vault console has a dedicated situational map, separate from the
    // assignment board in COMMAND. The shared map renderer remains read-only
    // until the operator explicitly configures an area in Settings.
    const map = el("div", "rb-map-desktop-host");
    ui.extension.append(map);
    if (window.RustBoyMap && map.isConnected && activeStage === "viewport") {
      window.RustBoyMap.mount(map, {navigationOnly:true, reclamation:false});
    } else {
      map.append(el("p", "panel-note", "Map renderer unavailable. Open COMMAND for the mission board."));
    }
    return;
  }
  if (selectedTool === "mission-board") {
    const manager = el("section", "job-command-panel");
    const heading = el("div", "section-heading");
    heading.append(el("h2", "", "Vault Operations"),
      el("span", "section-counter", "LOCAL"));
    manager.append(heading);
    manager.append(el("p", "task-help",
      "Mission planning, assignments, dispatch and receipts are managed from COMMAND. " +
      "This viewport remains available for active module work."));
    const open = el("button", "primary-button", "OPEN MISSION COMMAND");
    open.type = "button";
    open.addEventListener("click", () => activateStage("command"));
    manager.append(open);
    ui.extension.append(manager);
    return;
  }
  if (["reclamation-scout", "recon"].includes(selected.id)) {
    if (!["map", "sites", "tasks"].includes(extensionMode)) extensionMode = "map";
    const siteAccess = selectedAvailable() && snapshot.active.slots.some(slot => slot.tool === "reclamation");
    const options = [["map","MAP"], ...(siteAccess ? [["sites","SITES"]] : []), ...(selectedAvailable() ? [["tasks","DUTIES"]] : [])];
    ui.extension.append(extensionTabs(options, extensionMode));
    const body = el("div", "extension-focused-body");
    ui.extension.append(body);
    if (extensionMode === "map" || !selectedAvailable()) {
      const map = el("div", "rb-map-desktop-host");
      body.append(map);
      if (window.RustBoyMap && map.isConnected && activeStage === "viewport") {
        window.RustBoyMap.mount(map, {
          navigationOnly: true,
          reclamation: selectedAvailable() &&
            snapshot.active.slots.some((slot) => slot.tool === "reclamation")
        });
        if (queuedSiteId && window.RustBoyMap.queueSiteFocus(queuedSiteId)) queuedSiteId = null;
      }
    } else if (extensionMode === "sites" && window.RustBoyReclamation) {
      const station = el("div", "rb-reclamation-desktop");
      body.append(station);
      window.RustBoyReclamation.mount(station, {
        onMap: (site) => {
          queuedSiteId = site.id;
          extensionMode = "map";
          renderExtension();
        }
      });
    } else if (extensionMode === "tasks" && window.RustBoyTasks) {
      const queue = el("div", "job-assignments");
      body.append(queue);
      window.RustBoyTasks.summary(queue, { role: selected.id });
    } else {
      body.append(el("p", "panel-note", "This component is not available."));
    }
    return;
  }
  if (selected.id === "security-chief") {
    if (!["cameras", "tasks"].includes(extensionMode)) extensionMode = "cameras";
    if (selectedAvailable()) {
      ui.extension.append(extensionTabs([["cameras","CAMERAS"],["tasks","DUTIES"]], extensionMode));
      if (extensionMode === "tasks") {
        const queue = el("div", "extension-focused-body");
        ui.extension.append(queue);
        if (window.RustBoyTasks) window.RustBoyTasks.summary(queue, { role: selected.id });
        return;
      }
    }
    const wall = el("div", "camera-wall");
    ui.extension.append(wall);
    if (activeStage === "viewport" && activePane === "workspace" && selectedAvailable() && window.RustBoyCameras) {
      cameraWallUI = window.RustBoyCameras.mount(wall, {
        feeds: snapshot.camera_feeds, selected: selectedCamera,
        onSelect: value => { selectedCamera = value; }
      });
    } else wall.append(el("div", "camera-wall-heading", "PUBLIC VIEWS / OPEN CAMERAS TO LOAD"));
    return;
  }
  if (selectedAvailable() && selected.id !== "custom" && window.RustBoyTasks) {
    const station = el("div", "job-station-summary");
    const name = el("strong", "", "ASSIGNED OPERATIONS / " + selected.title.toUpperCase());
    const purpose = el("p", "workspace-subtitle", selected.description);
    station.append(name, purpose);
    const queue = el("div", "job-assignments");
    station.append(queue);
    ui.extension.append(station);
    window.RustBoyTasks.summary(queue, { role: selected.id });
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
  "byte-lab":
    field("input", "HEX BYTES (UP TO 4096)", "48656c6c6f", 'autocomplete="off" spellcheck="false"'),
  "diag": '<p class="workspace-subtitle">Check the local host using read-only CPU/RAM evidence. For scoped JSONL imports use pipboy diag ingest on the CLI.</p>',
  "host": '<p class="workspace-subtitle">Sample local host status on demand. No continuous background collection.</p>',
  "game": '<p class="workspace-subtitle">Manual connection only. The browser does not contact a PlayStation.</p>',
  "library": '<p class="workspace-subtitle">Shows the offline reference handoff. No network fetch.</p>',
  "notes": '<p class="workspace-subtitle">Shows the local notes command. Your note contents stay private.</p>',
  "simulator": '<p class="workspace-subtitle">Use deterministic fixtures. No physical interface is enabled.</p>'
});

function renderDetail() {
  if (activeNetworkUI) activeNetworkUI.destroy();
  activeNetworkUI = null;
  const networkHost = byId("network-workspace");
  networkHost.hidden = true;
  networkHost.replaceChildren();
  ui.detailsTitle.textContent = "Select a tool";
  ui.detailsDesc.textContent = "Choose a tool to see details.";
  ui.detailStatus.replaceChildren();
  ui.testbench.hidden = true;
  ui.handoff.hidden = true;
  ui.fields.replaceChildren();
  ui.form.hidden = false;
  const info = selectedTool && toolInfo(selectedTool);
  if (!info) {
    ui.detailInfo.hidden = true;
    return;
  }
  ui.detailInfo.hidden = false;
  ui.detailsTitle.textContent = info.name;
  ui.detailsDesc.textContent = info.description;
  const capability = el("span", "tag" +
    (info.availability === "ready" ? "" : " amber"),
    info.availability.replace(/_/g, " ").toUpperCase());
  const state = el("span", "tag" + (selectedAvailable() ? "" : " amber"),
    selectedAvailable() ? "PINNED" : "PREVIEW");
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
  if (selectedTool === "reclamation") {
    ui.handoff.hidden = false;
    ui.manual.textContent = "Site register is available in the Reclamation profile viewport and handheld TOOL screen.";
    return;
  }
  if (selectedTool === "mission-board") {
    ui.handoff.hidden = false;
    ui.manual.textContent = "Open COMMAND to create, assign, track and explicitly dispatch missions. Local tasks are never sent automatically.";
    return;
  }
  if (selectedTool === "net-lab" && window.RustBoyNetwork) {
    networkHost.hidden = false;
    activeNetworkUI = window.RustBoyNetwork.mount(networkHost, {
      operations: snapshot.net_operations,
      onStatus: (message) => { ui.footer.textContent = message; }
    });
    return;
  }
  if (Object.hasOwn(forms, selectedTool)) {
    ui.testbench.hidden = false;
    ui.fields.innerHTML = forms[selectedTool]; // Only static authored form markup.
    ui.run.textContent = info.availability === "ready" ? "Run" : "Instructions";
    ui.run.disabled = working;
    ui.kind.textContent = info.availability === "ready" ? "OFFLINE" : "OPT-IN";
    ui.output.textContent = lastOutput || "Ready.";
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
  ui.status.innerHTML = '<span class="live-dot" aria-hidden="true"></span> READY';
  ui.title.textContent = profile()?.title || "ROLE";
  ui.subtitle.textContent = "Local tools only.";
  ui.viewMode.textContent = "ACTIVE";
  ui.roleStat.textContent = current.title.toUpperCase();
  renderRoles();
  renderModules();
  renderDetail();
  activateStage(activeStage);
  activatePane(activePane);
}

async function reload({silent = false} = {}) {
  try {
    snapshot = await api("/api/state");
    updateHostPackageSelect(snapshot.host_packages);
    if (!initialWorkspaceLoaded) {
      activeStage = snapshot.foreground?.surface === "settings"
        ? "settings"
        : snapshot.foreground?.surface === "package" ? "adapter"
          : snapshot.active.profile === "overseer" ? "command" : "modules";
      initialWorkspaceLoaded = true;
    } else if (snapshot.foreground?.surface === "settings" && activeStage !== "settings") {
      settingsReturnStage = snapshot.host_packages?.selected_package_id ? "adapter" : activeStage;
      activeStage = "settings";
    } else if (snapshot.foreground?.surface === "package") {
      activeStage = "adapter";
    } else if (activeStage === "adapter") {
      activeStage = "modules";
    }
    previewId = snapshot.active.profile;
    if (!selectedTool) selectedTool = visibleModuleIds()[0] || null;
    rolePage = Math.floor(Math.max(0, snapshot.profiles.findIndex(p => p.id === previewId)) / rolesPerPage());
    renderAll();
    if (!silent) notify("Workspace refreshed.");
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
    modulePage = 0;
    extensionMode = "map";
    ui.detailInfo.open = false;
    notify("Role loaded: " + profile().title);
    activeStage = id === "overseer" ? "command" : "modules";
    activePane = "workspace";
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
  const current = pendingProfile();
  if (!current) return;
  ui.confirmText.textContent = "Load " + current.title +
    " and replace pinned tools? Notes and permissions remain unchanged. No device connections are opened.";
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
    notify("Tool added. Nothing was started.");
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
    notify("Tool removed.");
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
  ui.run.textContent = "Running…";
  ui.outputKind.textContent = "RUNNING";
  try {
    const result = await api("/api/experiment", { tool, ...input });
    if (selectedTool !== tool || !selectedAvailable()) return;
    lastOutput = result.output;
    ui.output.textContent = lastOutput;
    ui.outputKind.textContent = result.kind.toUpperCase().replaceAll("_", " ");
    ui.copy.disabled = false;
    ui.footer.textContent = "Local tool complete";
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
      ui.run.textContent = toolInfo(tool)?.availability === "ready"
        ? "Run" : "Instructions";
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

document.querySelectorAll("[data-stage-target]").forEach(button => button.addEventListener("click", () => {
  // VIEWPORT is a way back to the map/workspace while the radio is open.
  if (button.dataset.stageTarget === "viewport" && activeStage === "viewport" && radioPaneOpen)
    toggleRadioSubview();
  else activateStage(button.dataset.stageTarget);
}));
byId("open-palette").addEventListener("click", () => activateStage("palette"));
byId("palette-back").addEventListener("click", () => activateStage("settings"));
byId("settings-return").addEventListener("click", returnFromSettings);
byId("desktop-settings-open").addEventListener("click", () => {
  if (activeStage === "settings") returnFromSettings();
  else activateStage("settings");
});
byId("radio-view-toggle").addEventListener("click", toggleRadioSubview);
byId("command-open-messages").addEventListener("click", () => activateStage("messages"));
byId("command-open-viewport").addEventListener("click", () => activateStage("viewport"));
byId("command-open-modules").addEventListener("click", () => activateStage("modules"));
byId("inspector-return").addEventListener("click", () => { activateStage("modules"); activatePane("workspace"); });
window.addEventListener("resize", () => { if (!snapshot) return; renderRoles(); renderModules(); });
document.querySelectorAll("[data-pane-target]").forEach(button => button.addEventListener("click", () => activatePane(button.dataset.paneTarget)));
ui.load.addEventListener("click", loadSelected);
ui.refresh.addEventListener("click", () => { void reload(); });
ui.form.addEventListener("submit", runLab);
ui.copy.addEventListener("click", copyOutput);
ui.confirm.addEventListener("close", () => {
  if (ui.confirm.returnValue === "replace" && pendingProfile()) changeProfile(previewId);
});
document.addEventListener("keydown", (event) => {
  if (!snapshot) return;
  if (event.key === "Escape" && !ui.confirm.open) {
    if (selectedCamera !== null) {
      selectedCamera = null;
      if (activeStage === "viewport") renderExtension();
      return;
    }
    if (event.target?.matches?.("input,textarea,select,[contenteditable]")) return;
    if (window.matchMedia("(max-width: 1100px)").matches && activePane !== "workspace") {
      activateStage("modules");
      activatePane("workspace");
      return;
    }
    if (activeStage === "palette") {
      activateStage("settings");
      return;
    }
    if (activeStage === "viewport" && radioPaneOpen) {
      toggleRadioSubview();
      return;
    }
    if (activeStage !== "modules") {
      activateStage("modules");
      return;
    }
  }
  if (!event.altKey || event.ctrlKey || event.metaKey) return;
  if (event.key === "ArrowUp" || event.key === "ArrowDown") {
    event.preventDefault();
    if (activeStage !== "settings") return;
    const i = snapshot.profiles.findIndex((p) => p.id === previewId);
    const delta = event.key === "ArrowUp" ? -1 : 1;
    const nextIndex = (i + delta + snapshot.profiles.length) % snapshot.profiles.length;
    previewId = snapshot.profiles[nextIndex].id;
    rolePage = Math.floor(nextIndex / rolesPerPage());
    renderRoles();
  } else if (event.key === "Enter" && activeStage === "settings") {
    event.preventDefault();
    loadSelected();
  }
});

// Messenger remains in the same workspace tab for every occupational profile.
void reload({silent:true});

// RADIO is explicitly mounted only via VIEWPORT → RADIO / IP. No autoplay,
 // background status polling, or permanently visible dock on dashboard launch.

// Shared location ingestion is opened explicitly from universal Settings.
byId("location-settings-open")?.addEventListener("click", () => {
  const dialog = byId("location-settings-dialog");
  if (!window.RustBoyMap?.mountSettings || !dialog) return;
  dialog.showModal();
  window.RustBoyMap.mountSettings(byId("location-settings-host"), {
    reclamation: snapshot?.active.slots.some(slot => slot.tool === "reclamation")
  });
});
byId("location-settings-close")?.addEventListener("click", () => byId("location-settings-dialog").close());
byId("location-settings-dialog")?.addEventListener("close", () => window.RustBoyMap?.unmount());

byId("desktop-map-nav")?.addEventListener("change", (event) => {
  const destination = event.target.value;
  event.target.value = "map";
  if (destination === "radio") {
    toggleRadioSubview();
  } else if (["sites", "tasks", "map"].includes(destination)) {
    radioPaneOpen = false;
    extensionMode = destination;
    activateStage("viewport");
  } else if (["command", "modules", "messages", "settings"].includes(destination)) {
    activateStage(destination);
  }
});
byId("host-package-select")?.addEventListener("change", (event) => {
  selectHostPackage(event.currentTarget.value);
});
