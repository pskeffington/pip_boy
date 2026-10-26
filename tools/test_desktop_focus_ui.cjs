"use strict";
// Production desktop structure and consent checks. No network or hardware I/O.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const root = path.resolve(__dirname, "..", "crates", "pipboy", "assets");
const html = fs.readFileSync(path.join(root,"dashboard.html"),"utf8");
const js = fs.readFileSync(path.join(root,"dashboard.js"),"utf8");
const css = fs.readFileSync(path.join(root,"dashboard.css"),"utf8");
const ids = [...html.matchAll(/\bid="([^"]+)"/g)].map(x=>x[1]);
assert.equal(new Set(ids).size,ids.length,"duplicate desktop element IDs");
for(const id of [
  "role-list","role-pager","role-explanation","workspace-title",
  "workspace-subtitle","workspace-stage","module-grid","module-pager",
  "palette-grid","palette-pager","extension-content",
  "detail-info","detail-description","detail-status",
  "network-workspace","testbench","lab-form","lab-output",
  "messenger-dock","inspector-return","load-role","confirm-dialog"
]) assert.ok(ids.includes(id),"missing desktop contract "+id);
for(const id of ["role-explanation","detail-info"]){
  const pattern=new RegExp('<details\\b[^>]*\\bid="'+id+'"[^>]*>');
  const tag=html.match(pattern)?.[0];
  assert.ok(tag,"About disclosure missing: "+id);
  assert.ok(!/\sopen(?:\s|>|=)/.test(tag),"About disclosure must start closed: "+id);
}
const messageStage=html.match(/<section\b[^>]*data-stage="messages"[^>]*>/)?.[0];
assert.ok(messageStage&&messageStage.includes("hidden"),
  "MESSAGES must be an initially hidden, user-selectable stage");
for(const stage of ["modules","viewport","messages","palette"])
  assert.ok(html.includes('data-stage-target="'+stage+'"'),"missing stage "+stage);
for(const pane of ["roles","workspace","inspector"])
  assert.ok(html.includes('data-pane-target="'+pane+'"'),"missing desktop pane "+pane);
assert.ok(html.includes('src="/assets/map-zip.js"'),
  "ZIP-to-county MAP dependency must be preserved");
assert.ok(!html.includes("EXPANSION PORT"),"redundant viewport copy reintroduced");
assert.ok(!html.includes("SECURITY BOUNDARY"),"persistent verbose safety copy reintroduced");
assert.ok(html.includes("<summary>Safety &amp; access</summary>"));
assert.ok(html.includes("<summary>About this station</summary>"));
assert.ok(html.includes("<summary>About this tool</summary>"));
assert.match(html,/id="load-role"[^>]*disabled/,"load must remain gated");
assert.match(js,/ui\.confirm\.showModal\(\)/,
  "station replacement must retain explicit confirmation");
assert.match(js,/stage === "messages" && !desktopMessenger && window\.RustBoyMessenger/,
  "Messenger must mount only on user-selected MESSAGES stage");
assert.match(js,/desktopMessenger = window\.RustBoyMessenger\.mount\(byId\("messenger-dock"\)\)/,
  "on-demand Messenger mount missing");
assert.ok(!js.includes('window.RustBoyMessenger.mount(document.getElementById("messenger-dock"))'),
  "do not mount messaging on page load");
assert.match(js,/void reload\(\{ silent: true \}\)/,
  "startup must not generate an unnecessary success toast");
assert.match(js,/ui\.detailInfo\.open = false;/,
  "changing selection should close extra prose");
assert.match(js,/button\.title = role\.description \|\| role\.title/,
  "full Rust role descriptions must remain accessible as titles");
assert.match(js,/card\.title = info\.description \|\| info\.name/,
  "full Rust module descriptions must remain accessible as titles");
assert.ok(!js.includes('el("span", "role-desc"'),
  "permanent role paragraphs should not occupy the station menu");
assert.match(js,/rolesPerPage\(\)/,"mainline adaptive role pagination must remain");
assert.match(js,/cardsPerPage\(\)/,"mainline adaptive module pagination must remain");
assert.match(js,/function extensionTabs\(/,"focused Scout MAP tabs must remain");
assert.match(js,/inspector-return/,"inspector return control must remain wired");
assert.match(css,/\.module-card\s*\{[^}]*min-height:\s*48px/s,
  "regular desktop tool controls must be at least 48px");
assert.match(css,/\.role-card\s*\{[^}]*min-height:\s*44px/s,
  "desktop station controls must be at least 44px");
assert.match(css,/@media\(max-height:480px\)/,
  "short-screen legacy navigation must remain");
assert.match(css, /\.copy-disclosure summary:focus-visible/,
  "keyboard-visible disclosure focus is required");
console.log("PASS desktop focus: compact copy, preserved pagers/focused MAP, explicit messaging, consent, disclosures");
