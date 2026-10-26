"use strict";

/**
 * Dependency-free checks of the production desktop shell's disclosure and
 * navigation contracts. This test does not open a device or contact a server.
 */
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..", "crates", "pipboy", "assets");
const html = fs.readFileSync(path.join(root, "dashboard.html"), "utf8");
const css = fs.readFileSync(path.join(root, "dashboard.css"), "utf8");
const js = fs.readFileSync(path.join(root, "dashboard.js"), "utf8");

const allIds = [...html.matchAll(/\bid="([^"]+)"/g)].map((match) => match[1]);
assert.equal(new Set(allIds).size, allIds.length, "duplicate document IDs");

for (const id of [
  "role-list", "workspace-title", "workspace-subtitle", "role-explanation",
  "module-grid", "status-role", "status-modules", "extension-content",
  "detail-info", "detail-description", "detail-status", "network-workspace",
  "testbench", "lab-form", "lab-output", "messenger-disclosure",
  "messenger-dock", "load-role", "confirm-dialog", "refresh-state"
]) {
  assert.ok(allIds.includes(id), "missing production DOM node: " + id);
}
for (const id of ["role-explanation", "detail-info", "messenger-disclosure"]) {
  assert.match(html, new RegExp('<details\\b[^>]*\\bid="' + id + '"[^>]*>'),
    "required disclosure unavailable: " + id);
  const tag = html.match(new RegExp('<details\\b[^>]*\\bid="' + id + '"[^>]*>'))[0];
  assert.ok(!/\sopen(?:\s|>|=)/.test(tag), id + " must start closed");
}
assert.ok(html.indexOf('id="role-list"') < html.indexOf('id="module-grid"'));
assert.ok(html.indexOf('id="module-grid"') < html.indexOf('id="extension-content"'));
assert.ok(html.indexOf('id="extension-content"') < html.indexOf('id="messenger-disclosure"'));
assert.ok(!html.includes("JOB TEMPLATES"), "verbose title reintroduced");
assert.ok(!html.includes("SECURITY BOUNDARY"), "persistent warning block reintroduced");
assert.ok(!html.includes("EXPANSION PORT"), "redundant section text reintroduced");
assert.match(html, /id="load-role"[^>]*disabled/, "profile loads must be gated until state loads");
assert.match(html, /id="confirm-dialog"/, "replacement consent dialog must remain");
assert.match(html, /<summary>Safety &amp; access<\/summary>/);
assert.match(html, /<summary>About this tool<\/summary>/);
assert.match(html, /<summary>About this station<\/summary>/);

assert.match(css, /\.module-card\s*\{[^}]*min-height:\s*48px/s,
  "compact module cards need >=44px touch targets");
assert.match(css, /\.role-card\s*\{[^}]*min-height:\s*46px/s,
  "station controls need >=44px touch targets");
assert.match(css, /\.desktop-messenger\s*>\s*summary\s*\{[^}]*min-height:\s*48px/s,
  "collapsed Messenger must offer a usable target");
assert.match(css, /@media\s*\(max-width:\s*1020px\)/,
  "two smaller desktop layouts must reflow");
assert.match(js, /ui\.messengerDisclosure\.addEventListener\("toggle"/,
  "Messenger must only mount after an intentional disclosure");
assert.match(js, /if\s*\(ui\.messengerDisclosure\.open\s*&&\s*!messenger\)/,
  "Messenger must not auto-mount on initial page load");
assert.match(js, /void reload\(\{ silent: true \}\)/,
  "initial load must not create a transient success notice");
assert.match(js, /ui\.detailInfo\.open = false;/,
  "switching modules must reset optional descriptive content");
assert.match(js, /button\.title = role\.description \|\| role\.title/,
  "full station description must stay accessible on hover");
assert.match(js, /card\.title = info\.description \|\| info\.name/,
  "module description must remain accessible on hover");
assert.ok(!js.includes('el("span", "role-desc"'),
  "duplicate permanent role descriptions must not return");
assert.match(js, /ui\.confirm\.showModal\(\)/,
  "load confirmation must be preserved");

console.log("PASS desktop focus UI: IDs, compact controls, disclosures, consent and no-autoload contracts");
