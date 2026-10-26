"use strict";

// Render the actual micro/radio CSS+JS in headless Chromium at exact viewport
// sizes; all radio state is synthetic and GET-only, with no audio or Internet.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");
const { spawn, spawnSync } = require("node:child_process");

const root = path.resolve(__dirname, "..");
const fixture = path.join(__dirname, "fixtures", "radio-viewport.html");
const allowed = new Map([
  ["/fixture", [fixture, "text/html; charset=utf-8"]],
  ["/assets/micro.css", [path.join(root, "crates/pipboy/assets/micro.css"), "text/css; charset=utf-8"]],
  ["/assets/radio-ui.css", [path.join(root, "crates/pipboy/assets/radio-ui.css"), "text/css; charset=utf-8"]],
  ["/assets/radio-ui.js", [path.join(root, "crates/pipboy/assets/radio-ui.js"), "text/javascript; charset=utf-8"]]
]);
const chromeCandidates = [
  process.env.CHROME_BIN, "/usr/bin/google-chrome", "/usr/bin/chromium",
  "google-chrome", "chromium", "chromium-browser"
].filter(Boolean);

function findChrome() {
  for (const candidate of chromeCandidates) {
    if (path.isAbsolute(candidate) && fs.existsSync(candidate)) return candidate;
    const result = spawnSync("which", [candidate], { encoding: "utf8" });
    if (result.status === 0 && result.stdout.trim())
      return result.stdout.trim().split("\n")[0];
  }
  throw Error("Chromium unavailable; set CHROME_BIN");
}

// Top-level Chromium has a minimum viewport width near 500px; same-origin
// iframe measurement gives the real 480x320/320x568 CSS viewport.
function hostHtml(width, height, scenario) {
  const script = [
    'const frame=document.getElementById("handheld");',
    'const output=document.getElementById("viewport-report");',
    'function encode(v){return btoa(String.fromCharCode(...new TextEncoder().encode(JSON.stringify(v))))}',
    'let attempts=0;',
    'function copy(){',
    'const result=frame.contentDocument?.getElementById("viewport-report")?.textContent;',
    'if(result){output.textContent=result;return;}',
    'if(++attempts>65){output.textContent=encode({error:"No iframe report"});return;}',
    'setTimeout(copy,20);}',
    'window.addEventListener("message",event=>{',
    'if(event.origin===location.origin&&event.source===frame.contentWindow&&',
    'event.data?.type==="rustboy-radio-layout")',
    'output.textContent=encode(event.data.value);});',
    'frame.addEventListener("load",copy);copy();'
  ].join("");
  return '<!doctype html><html lang="en"><head><meta charset="utf-8">' +
    '<style>html,body{margin:0}iframe{border:0;display:block}</style></head><body>' +
    '<iframe id="handheld" title="Radio viewport" width="' + width + '" height="' +
    height + '" style="width:' + width + 'px;height:' + height +
    'px" src="/fixture?scenario=' + scenario + '"></iframe>' +
    '<pre id="viewport-report" hidden></pre><script>' + script + '</script></body></html>';
}

function render(chrome, port, width, height, scenario) {
  return new Promise((resolve, reject) => {
    const profile = fs.mkdtempSync(path.join(os.tmpdir(), "rb-radio-viewport-"));
    const url = "http://127.0.0.1:" + port + "/host?width=" + width +
      "&height=" + height + "&scenario=" + scenario;
    const args = [
      "--headless", "--no-sandbox", "--disable-gpu", "--disable-dev-shm-usage",
      "--disable-background-networking", "--no-first-run",
      "--no-default-browser-check", "--force-device-scale-factor=1",
      "--window-size=1000,900", "--virtual-time-budget=2400", "--dump-dom",
      "--user-data-dir=" + profile, url
    ];
    const proc = spawn(chrome, args, { stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "", stderr = "", done = false;
    const cleanup = () => {
      clearTimeout(timer);
      fs.rmSync(profile, { recursive: true, force: true });
    };
    const fail = (error) => {
      if (done) return;
      done = true;
      cleanup();
      reject(error);
    };
    const timer = setTimeout(() => {
      proc.kill("SIGKILL");
      fail(Error("Radio Chromium timeout: " + scenario));
    }, 30000);
    proc.stdout.on("data", chunk => {
      stdout += chunk.toString();
      if (stdout.length > 10_000_000) proc.kill("SIGKILL");
    });
    proc.stderr.on("data", chunk => {
      stderr += chunk.toString();
      if (stderr.length > 12_000) stderr = stderr.slice(-12_000);
    });
    proc.on("error", fail);
    proc.on("close", code => {
      if (done) return;
      done = true;
      cleanup();
      if (code !== 0) return reject(Error("Chromium failed: " + stderr.slice(-1500)));
      const match = stdout.match(/<pre id="viewport-report"[^>]*>([A-Za-z0-9+/=]+)<\/pre>/);
      if (!match) return reject(Error("No layout report: " + stdout.slice(-1400)));
      try {
        resolve(JSON.parse(Buffer.from(match[1], "base64").toString("utf8")));
      } catch (error) {
        reject(Error("Invalid layout report: " + error.message));
      }
    });
  });
}

function check(data, width, height, scenario) {
  console.log("RADIO viewport " + width + "x" + height + "/" + scenario +
    " scroll " + data.scrollHeight + "/" + data.clientHeight);
  assert.ok(!data.error, data.error || "Fixture failed");
  assert.equal(data.window.width, width);
  assert.equal(data.window.height, height);
  assert.ok(data.viewport && data.device, "Micro shell not rendered");
  assert.ok(data.controls.length >= 10 && data.primaryVisible,
    "Radio controls clipped: " + JSON.stringify(data.controls.filter(x => !x.visible)));
  assert.ok(data.controls.every(x => x.rect && x.rect.width >= 40 && x.rect.height >= 40),
    "Radio control below 40px touch target: " +
    JSON.stringify(data.controls.filter(x => !x.rect || x.rect.width < 40 || x.rect.height < 40)));
  assert.ok(data.scrollWidth <= data.clientWidth + 1, "Radio overflowed horizontally");
  assert.ok(data.scrollHeight <= data.clientHeight + 1,
    "Radio requires vertical scrolling: " + data.scrollHeight + "/" + data.clientHeight);
  assert.ok(data.requests.length >= 1 && data.requests.every(
    request => request.url === "/api/radio" && request.method === "GET"
  ), "Opening/browsing RADIO must not call Play or contact an external station");
  if (scenario === "favorites") {
    assert.equal(data.favoriteMode, "true");
    assert.equal(data.visibleStationCount, 2);
  }
  if (scenario === "settings" || scenario === "output-lost") {
    assert.equal(data.settingsExpanded, true);
    assert.equal(data.settingsVisible, true, "Settings overlay exceeds viewport");
  }
  if (scenario === "output-lost")
    assert.equal(data.fallbackVisible, true, "Lost output must show manual recovery");
}

async function main() {
  const chrome = findChrome();
  const server = http.createServer((req, response) => {
    const url = new URL(req.url, "http://localhost");
    if (req.method === "GET" && url.pathname === "/host") {
      const width = Number(url.searchParams.get("width"));
      const height = Number(url.searchParams.get("height"));
      const scenario = url.searchParams.get("scenario");
      if (!Number.isInteger(width) || !Number.isInteger(height) ||
          width < 300 || width > 900 || height < 300 || height > 700 ||
          !["ready", "favorites", "playing", "settings", "output-lost"].includes(scenario)) {
        response.writeHead(400); response.end("Invalid viewport"); return;
      }
      response.writeHead(200, {
        "Content-Type": "text/html; charset=utf-8",
        "Cache-Control": "no-store",
        "Content-Security-Policy": "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; frame-src 'self'"
      });
      response.end(hostHtml(width, height, scenario));
      return;
    }
    const asset = allowed.get(url.pathname);
    if (req.method !== "GET" || !asset) {
      response.writeHead(403); response.end("Denied"); return;
    }
    response.writeHead(200, {
      "Content-Type": asset[1], "Cache-Control": "no-store",
      "Content-Security-Policy": "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self'"
    });
    fs.createReadStream(asset[0]).pipe(response);
  });
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  try {
    const port = server.address().port;
    for (const [width, height, scenario] of [
      [480, 320, "ready"], [480, 320, "favorites"],
      [480, 320, "playing"], [480, 320, "settings"],
      [480, 320, "output-lost"], [320, 568, "ready"],
      [800, 480, "ready"]
    ]) check(await render(chrome, port, width, height, scenario), width, height, scenario);
    console.log("RADIO Chromium viewport: all compact scenarios passed");
  } finally {
    await new Promise(resolve => server.close(resolve));
  }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
