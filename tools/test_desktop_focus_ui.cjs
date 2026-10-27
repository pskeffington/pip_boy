"use strict";
// No network/device access: production desktop structure and consent contract.
const assert=require("node:assert/strict");
const fs=require("node:fs");
const path=require("node:path");
const root=path.resolve(__dirname,"..","crates","pipboy","assets");
const html=fs.readFileSync(path.join(root,"dashboard.html"),"utf8");
const js=fs.readFileSync(path.join(root,"dashboard.js"),"utf8");
const css=fs.readFileSync(path.join(root,"dashboard.css"),"utf8");
const ids=[...html.matchAll(/\bid="([^"]+)"/g)].map(m=>m[1]);
assert.equal(new Set(ids).size,ids.length,"Duplicate desktop element IDs");
for(const id of [
  "role-list","role-pager","settings-current-role","settings-selection",
  "open-palette","palette-back","load-role","confirm-dialog",
  "module-grid","module-pager","palette-grid","palette-pager",
  "workspace-title","status-role","status-modules","workspace-stage",
  "command-heading","command-authority","command-current-role","command-policy",
  "command-board","command-open-messages","command-open-viewport","command-open-modules",
  "extension-content","messenger-dock","inspector-return",
  "details-heading","detail-info","detail-description","detail-status",
  "testbench","network-workspace","lab-form","lab-output","refresh-state"
]) assert.ok(ids.includes(id),"Missing desktop contract "+id);
for(const stage of ["command","modules","viewport","messages","settings"])
  assert.ok(html.includes('data-stage-target="'+stage+'"'),"Missing workspace stage "+stage);
assert.ok(html.includes('data-stage="palette"') && html.includes('id="open-palette"'),
  "Custom palette must remain available through Settings");
for(const pane of ["workspace","inspector"])
  assert.ok(html.includes('data-pane-target="'+pane+'"'),"Missing narrow desktop pane "+pane);
assert.ok(html.indexOf('id="open-palette"')>html.indexOf('class="settings-choice"'),
  "Custom tool editor must stay in the visible Settings action row");
assert.ok(!html.includes('data-pane-target="roles"'),
  "Profile selection must remain only in Settings");
const messageStage=html.match(/<section\b[^>]*data-stage="messages"[^>]*>/)?.[0];
assert.ok(messageStage&&messageStage.includes("hidden"),
  "MESSAGES must be initially hidden");
const details=html.match(/<details\b[^>]*id="detail-info"[^>]*>/)?.[0];
assert.ok(details&&!/\sopen(?:\s|>|=)/.test(details),
  "Inspector explanation must initially be collapsed");
assert.ok(html.includes('<summary>Safety &amp; access</summary>'),
  "Safety details must remain available on demand");
assert.ok(!html.includes("EXPANSION PORT"),"Redundant viewport copy reintroduced");
assert.ok(!html.includes("SECURITY BOUNDARY"),"Persistent verbose warning reintroduced");
assert.ok(html.includes('src="/assets/map-zip.js"'),
  "Current ZIP-to-county MAP asset must remain loaded");
assert.match(html,/id="load-role"[^>]*disabled/,
  "Profile replacement must remain unavailable until data loads");
assert.match(js,/ui\.confirm\.showModal\(\)/,
  "Loading a different station must require operator confirmation");
assert.match(js,/stage === "messages" && !desktopMessenger && window\.RustBoyMessenger/,
  "MESSAGES must lazy-mount its local widget");
assert.match(js,/desktopMessenger = window\.RustBoyMessenger\.mount\(byId\("messenger-dock"\)\)/,
  "Lazy Messenger mount missing");
assert.ok(!js.includes('window.RustBoyMessenger.mount(document.getElementById("messenger-dock"))'),
  "Opening desktop must not mount Messenger");
assert.match(js,/void reload\(\{silent:true\}\)/,
  "Startup should not show a transient success toast");
assert.match(js,/card\.title = info\.description \|\| info\.name/,
  "Tool descriptions must remain available through tooltips");
assert.match(js,/button\.title = role\.description \|\| role\.title/,
  "Station descriptions must remain available through tooltips");
assert.ok(!js.includes('el("span", "role-desc"'),
  "Permanent role descriptions reintroduced in Settings");
assert.match(js,/function renderSettingsSelection\(/,
  "Current/selected station state must remain in Settings");
assert.match(js,/function extensionTabs\(/,
  "Scout MAP/camera focused tabs must remain");
assert.match(js,/rolesPerPage\(\)/,
  "Mainline adaptive station paging must remain");
assert.match(css,/\.module-card\s*\{[^}]*min-height:\s*48px/s,
  "Normal-height tool controls need a >=48px target");
assert.match(css,/@media\(max-height:480px\)/,
  "Short-screen adaptive paging must remain");
assert.match(css,/\.copy-disclosure summary:focus-visible/,
  "Disclosure keyboard focus must remain visible");
// Vault Command is a first-class role-independent page; authority is backend-gated.
assert.match(html,/data-stage="command"[^>]*hidden/,
  "Command stage must be available but initially hidden in raw HTML");
assert.match(js,/activeStage = snapshot.active.profile === "overseer" \? "command" : "modules";/,
  "Overseer should land in the vault command center");
assert.match(js,/snapshot.active.slots.some\(slot => slot.tool === "mission-board"\)/,
  "Command authority label must reflect Rust-provided tool slots");
assert.match(js,/window.RustBoyTasks.mount\(root, \{profiles: snapshot.profiles\}\)/,
  "Mission Control must use existing backend-gated task component");
assert.match(js,/host.replaceChildren\(root\)/,
  "Profile changes must replace old command board DOM before mounting the new role");
assert.match(css,/\.dashboard:has\(\.command-section:not\(\[hidden\]\)\)/,
  "Vault command must claim full desktop width");
assert.match(css,/\.command-board\s*\{[^}]*overflow:auto/,
  "Task list should scroll internally without scrolling the desktop");
// Desktop Radio is global navigation, including when MAP owns the viewport.
assert.match(js,/function toggleRadioSubview\(\)\s*\{[\s\S]*?activateStage\("viewport"\);/,
  "Global Radio button must activate viewport from Modules, Settings and Messages");
assert.match(js,/ui\.extension\.replaceChildren\(\);\s*activatePane\("workspace"\);\s*activateStage\("viewport"\);/,
  "Entering Radio must discard stale map markup and restore workspace pane");
assert.match(js,/if \(stage !== "viewport"\) radioPaneOpen = false;/,
  "Changing desktop pages must not leave a hidden radio override active");
assert.match(js,/ui\.extension\.hidden = visible;/,
  "Radio subview must never hide the active workspace while on another page");
assert.match(js,/radioPaneOpen = false;\s*extensionMode = destination;\s*activateStage\("viewport"\);/,
  "Map selector must return from Radio without rendering a hidden extension");
assert.match(js,/if \(button\.dataset\.stageTarget === "viewport" && activeStage === "viewport" && radioPaneOpen\)/,
  "Viewport button must return from the native radio surface");
console.log("PASS desktop focus: concise Settings, tools, consent, optional details and deferred messages");
