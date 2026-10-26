"use strict";

// Shared desktop/handheld network workspace. Mounting never performs I/O.
// A mounted view owns its requests and refresh timer; navigation revokes both.
window.RustBoyNetwork = (() => {
  const node = (tag, cls, text) => {
    const item = document.createElement(tag);
    if (cls) item.className = cls;
    if (text !== undefined) item.textContent = text;
    return item;
  };
  const control = (text, callback) => {
    const item = node("button", "net-button", text);
    item.type = "button";
    item.addEventListener("click", callback);
    return item;
  };
  const ms = (value) => Number.isFinite(value) ? value.toFixed(2) + " ms" : "—";

  function mount(container, { operations, compact = false, onStatus = () => {} }) {
    const root = node("section", "net-workspace" + (compact ? " net-compact" : ""));
    root.setAttribute("aria-label", "Network Engineer");
    container.replaceChildren(root);
    let destroyed = false;
    let timer = null;
    let controller = null;
    let generation = 0;
    let menuPage = 0;
    let resultPage = 0;
    let operation = null;
    let values = {};
    let lastResult = null;
    let watch = false;
    let refreshes = 0;
    let status = null;
    let screen = "menu";
    let activePager = null;
    let menuButtons = [];
    const candidates = Array.isArray(operations) ? operations : [];
    const toolsPerPage = compact ? 4 : 6;

    function stop() {
      generation++;
      if (timer !== null) window.clearTimeout(timer);
      timer = null;
      if (controller) controller.abort();
      controller = null;
      watch = false;
    }

    function header(backLabel, backAction, label) {
      const row = node("div", "net-actions");
      row.append(control(backLabel, backAction), node("strong", "net-heading", label));
      root.append(row);
    }

    // Both touch taps and keyboard-emulated physical D-pads use this pager.
    function pager(page, total, change, target = root) {
      const row = node("nav", "net-actions net-pager");
      row.setAttribute("aria-label", screen === "menu" ? "Network tool pages" : "Network result pages");
      const go = (step) => {
        const nextPage = page + step;
        if (nextPage < 0 || nextPage >= total) return false;
        change(nextPage);
        // Keep a hardware/keyboard user's focus on a reachable page button.
        const selected = root.querySelector?.(step < 0 ? ".net-page-prev" : ".net-page-next");
        const fallback = root.querySelector?.(step < 0 ? ".net-page-next" : ".net-page-prev");
        (selected && !selected.disabled ? selected : fallback)?.focus?.({ preventScroll: true });
        return true;
      };
      const previous = control("◀ PREV", () => go(-1));
      const next = control("NEXT ▶", () => go(1));
      previous.classList.add("net-page-arrow");
      previous.classList.add("net-page-prev");
      next.classList.add("net-page-arrow");
      next.classList.add("net-page-next");
      previous.disabled = page === 0;
      next.disabled = page + 1 === total;
      previous.setAttribute("aria-label", "Previous page");
      next.setAttribute("aria-label", "Next page");
      const index = node("span", "net-page-index", "PAGE " + (page + 1) + " / " + total);
      index.setAttribute("aria-label", "Page " + (page + 1) + " of " + total);
      row.append(previous, index, next);
      target.append(row);
      activePager = go;
    }

    // Hardware D-pad navigation uses native focus and browser button activation.
    // Arrow Up/Down moves focus; Left/Right and PageUp/PageDown turn pages.
    // Enter/Space still activate the focused button or checkbox normally.
    function focusWithPad(step) {
      const controls = screen === "menu" ? menuButtons : Array.from(
        root.querySelectorAll?.("button,input,select,textarea,summary") || []
      ).filter((item) => !item.disabled);
      if (!controls.length) return false;
      const current = controls.indexOf(document.activeElement);
      const next = current === -1 ? (step > 0 ? 0 : controls.length - 1)
        : Math.max(0, Math.min(controls.length - 1, current + step));
      if (current === next) return false;
      controls[next].focus?.({ preventScroll: true });
      return true;
    }

    function onPageKey(event) {
      if (destroyed || !compact || document.hidden || root.isConnected === false ||
          document.querySelector?.("dialog[open]") ||
          event.altKey || event.ctrlKey || event.metaKey || event.repeat) return;
      const target = event.target;
      if (target && root.contains && !root.contains(target) &&
          !target.contains?.(root) &&
          target !== document.body && target !== document.documentElement) return;
      if (target?.isContentEditable) return;
      const tag = String(target?.tagName || "").toUpperCase();
      // On one-line inputs, Up/Down advances hardware focus between fields;
      // Left/Right still edits the caret. Native selects/textareas keep their arrows.
      if (["TEXTAREA", "SELECT"].includes(tag) ||
          (tag === "INPUT" && target.type !== "checkbox" &&
           event.key !== "ArrowUp" && event.key !== "ArrowDown")) return;
      if (event.key === "ArrowUp" || event.key === "ArrowDown") {
        const moved = focusWithPad(event.key === "ArrowUp" ? -1 : 1);
        // Avoid accidental document scrolling at the ends of the tool deck.
        if (moved || screen === "menu") event.preventDefault();
        return;
      }
      if (screen !== "menu" && screen !== "result") return;
      const step = ["ArrowLeft", "PageUp"].includes(event.key) ? -1
        : ["ArrowRight", "PageDown"].includes(event.key) ? 1 : 0;
      if (!step || !activePager) return;
      const moved = activePager(step);
      // After hardware page-turning, land on a tool instead of a now-replaced arrow.
      if (moved && screen === "menu") menuButtons[0]?.focus?.({ preventScroll: true });
      // On a tool-deck boundary the key is still handled, not a scroll request.
      // Result pages permit native scrolling at the boundary for longer detail output.
      if (moved || screen === "menu") event.preventDefault();
    }

    const toolDescription = Object.freeze({
      scan: "PRIVATE LAN DISCOVERY",
      "unifi-sites": "CONTROLLER SITE ROSTER",
      "unifi-clients": "CONNECTED DEVICE ROSTER",
      "unifi-compare": "CONTROLLER / PROBE MATCH",
      icmp: "LATENCY / RESPONSE LOSS",
      health: "TCP CONNECT TIMINGS",
      cidr4: "IPv4 RANGE / HOSTS",
      cidr6: "IPv6 NETWORK PREFIX",
      split4: "ADDRESS PLAN / SUBNETS",
      route: "LONGEST PREFIX MATCH",
      aggregate4: "MERGE IPv4 BLOCKS",
      overlap4: "INTERSECT IPv4 RANGES",
      mac: "MAC / INTERFACE ID",
      vlan: "802.1Q TAG DECODE",
      mtu: "PACKET SIZE BUDGET",
      tcp: "SINGLE PORT CHECK"
    });

    // This is a snapshot of the last explicit request, not a live link indicator.
    function lastReadout() {
      if (!lastResult) return "NONE";
      const data = lastResult;
      const report = data.report || data.probe;
      const inventory = data.inventory || data.controller;
      if (data.comparison) {
        return data.comparison.controller_clients + " UNIFI / " +
          data.comparison.matched_active_probe + " MATCH";
      }
      if (report) {
        return report.responding_devices + " RESP / " + report.cached_only_devices + " CACHE";
      }
      if (inventory?.clients) return inventory.connected_clients + " CLIENTS";
      if (inventory?.sites) return inventory.sites.length + " SITES";
      if (data.metrics) return data.metrics.connected + " / " + data.metrics.attempts + " REPLIES";
      return data.kind === "computed" ? "CALCULATED" : "COMPLETE";
    }

    function menu() {
      screen = "menu";
      stop();
      activePager = null;
      menuButtons = [];
      root.replaceChildren();
      const total = Math.max(1, Math.ceil(candidates.length / toolsPerPage));
      menuPage = Math.max(0, Math.min(menuPage, total - 1));
      const landing = node("div", "net-landing");
      const heading = node("div", "net-launch-head");
      heading.append(
        node("strong", "net-caption", compact ? "SELECT TOOL" : "NETWORK ENGINEER / SELECT TOOL"),
        node("span", "net-count", compact ? "UP/DOWN SELECT / L/R PAGE" :
          candidates.length + " TOOLS / " + total + " PAGES")
      );
      landing.append(heading);

      const stats = node("div", "net-status-strip");
      for (const [label, value] of [
        ["SESSION", "STANDBY"],
        ["REQUESTS", "MANUAL ONLY"],
        ["LAST RESULT / NOT LIVE", lastReadout()]
      ]) {
        const cell = node("div", "net-status-cell");
        cell.append(node("span", "net-status-label", label),
          node("strong", "net-status-value", value));
        stats.append(cell);
      }
      landing.append(stats);

      const grid = node("div", "net-menu");
      for (const op of candidates.slice(menuPage * toolsPerPage, (menuPage + 1) * toolsPerPage)) {
        const item = control("", () => {
          operation = op;
          values = { input: op.example, extra: op.extra_example || "", samples: "5" };
          lastResult = null;
          configure();
        });
        item.classList.add("net-tile");
        item.append(
          node("strong", "net-tile-title", op.name),
          node("span", "net-tile-detail", toolDescription[op.id] || "NETWORK TOOL"),
          node("small", "net-tile-mode", op.active ? "AUTHORIZATION REQUIRED" : "OFFLINE CALCULATION")
        );
        grid.append(item);
        menuButtons.push(item);
      }
      if (!candidates.length) grid.append(node("p", "", "Tool catalog unavailable. Reload the dashboard."));
      landing.append(grid);

      const footer = node("div", "net-launch-footer");
      if (!compact) footer.append(node("span", "net-page-hint", "SELECT TOOL / CONFIGURE / RUN"));
      if (total > 1) pager(menuPage, total, (page) => { menuPage = page; menu(); }, footer);
      landing.append(footer);
      root.append(landing);
    }

    function field(form, key, label, value) {
      const wrap = node("label", "net-field", label);
      const input = node("input");
      input.name = key;
      input.value = value || "";
      input.maxLength = key === "extra" ? 1024 : 256;
      input.required = true;
      input.autocomplete = "off";
      input.spellcheck = false;
      if (key === "samples" || (key === "extra" && ["tcp", "health", "icmp"].includes(operation.id))) {
        input.inputMode = "numeric";
      }
      wrap.append(input);
      form.append(wrap);
      return input;
    }

    function configure() {
      screen = "inputs";
      stop();
      root.replaceChildren();
      header("TOOLS", menu, operation.name);
      const form = node("form", "net-form");
      const fields = node("div", "net-fields");
      if (operation.input_label) field(fields, "input", operation.input_label, values.input);
      if (operation.extra_label) field(fields, "extra", operation.extra_label, values.extra);
      if (operation.id === "health") field(fields, "samples", "SAMPLES (3–8)", values.samples);
      form.append(fields);
      const run = node("button", "net-button net-primary", operation.active ? "NEXT" : "CALCULATE");
      run.type = "submit";
      form.append(run);
      form.addEventListener("submit", (event) => {
        event.preventDefault();
        values = { ...values, ...Object.fromEntries(new FormData(form)) };
        if (operation.active) consent();
        else execute();
      });
      root.append(form);
      if (!operation.input_label) {
        root.append(node("p", "net-caption", "Read the configured controller's sites."));
      }
    }

    function checkbox(form, text) {
      const label = node("label", "net-check");
      const input = node("input");
      input.type = "checkbox";
      label.append(input, node("span", "", text));
      form.append(label);
      return input;
    }

    function consent() {
      screen = "consent";
      root.replaceChildren();
      header("EDIT", configure, operation.name);
      root.append(node("p", "net-target", values.input || "Configured local UniFi controller"));
      const form = node("form", "net-form");
      const accepted = checkbox(form, "AUTHORIZED NETWORK / CONTROLLER");
      accepted.required = true;
      const repeatable = operation.id === "scan" || operation.id.startsWith("unifi-");
      const repeat = repeatable ? checkbox(form, "REFRESH 60 SEC / MAX 60 RUNS") : null;
      const run = node("button", "net-button net-primary", "RUN " + operation.name);
      run.type = "submit";
      form.append(run);
      form.addEventListener("submit", (event) => {
        event.preventDefault();
        if (!accepted.checked) return;
        watch = Boolean(repeat?.checked);
        refreshes = 0;
        execute();
      });
      root.append(form);
    }

    function request() {
      if (operation.id === "scan") {
        return ["/api/net/scan", { cidr: values.input, confirm: true }];
      }
      if (operation.id.startsWith("unifi-")) {
        const action = operation.id.slice(6);
        const body = { operation: action, confirm: true };
        if (action === "clients") body.site = values.input;
        if (action === "compare") { body.cidr = values.input; body.site = values.extra; }
        return ["/api/net/unifi", body];
      }
      return ["/api/experiment", {
        tool: "net-lab", mode: operation.id, input: values.input,
        ...(operation.extra_label ? { extra: values.extra } : {}),
        ...(operation.id === "health" ? { samples: values.samples } : {}),
        ...(operation.active ? { confirm: true } : {})
      }];
    }

    function result() {
      screen = "result";
      activePager = null;
      root.replaceChildren();
      header("EDIT", configure, operation.name);
      const data = lastResult;
      const report = data.report || data.probe;
      const inventory = data.inventory || data.controller;
      const metrics = data.metrics;
      let summary = data.kind === "computed" ? "OFFLINE RESULT" : "READ COMPLETE";
      let rows = [];
      if (data.comparison) {
        const c = data.comparison;
        summary = c.controller_clients + " UNIFI / " + c.matched_active_probe + " MATCHED";
      } else if (report) {
        summary = report.responding_devices + " RESPONDING / " + report.cached_only_devices + " CACHED";
      } else if (inventory?.clients) {
        summary = inventory.connected_clients + " CONTROLLER CLIENTS / " + inventory.clients_with_ip + " WITH IP";
      } else if (inventory?.sites) summary = inventory.sites.length + " UNIFI SITES";
      if (metrics) summary = metrics.connected + " / " + metrics.attempts + " REPLIES / MEAN " + ms(metrics.mean_ms);
      root.append(node("div", "net-summary", summary));
      if (report) rows = report.devices.map((d) => [d.address,
        d.evidence.replaceAll("_", " ") + " / " + ms(d.latency_ms)]);
      else if (inventory?.clients) rows = inventory.clients.map((c) => [c.name,
        (c.ip_address || "IP unknown") + " / " + c.connection_type]);
      else if (inventory?.sites) rows = inventory.sites.map((s) => [s.name, s.id]);
      else if (metrics) rows = [
        ["VARIATION", ms(metrics.variation_ms)],
        [metrics.method === "icmp_echo" ? "ICMP LOSS" : "CONNECT FAILURES", metrics.failure_percent + "%"],
        ["MIN / MAX", ms(metrics.min_ms) + " / " + ms(metrics.max_ms)]
      ];
      if (rows.length) {
        const pageSize = compact ? 2 : 12;
        const pages = Math.ceil(rows.length / pageSize);
        resultPage = Math.max(0, Math.min(resultPage, pages - 1));
        const list = node("dl", "net-results");
        for (const [title, detail] of rows.slice(resultPage * pageSize, resultPage * pageSize + pageSize)) {
          const row = node("div", "net-result-row");
          row.append(node("dt", "", title), node("dd", "", detail));
          list.append(row);
        }
        root.append(list);
        if (pages > 1) pager(resultPage, pages, (page) => { resultPage = page; result(); });
      }
      if (!rows.length || !compact) root.append(node("pre", "net-output", data.output));
      else {
        const detail = node("details", "net-details");
        detail.append(node("summary", "", "DETAILS"), node("pre", "net-output", data.output));
        root.append(detail);
      }
      status = node("div", "net-caption");
      status.setAttribute("role", "status");
      status.textContent = watch ? "REFRESH IN 60 SEC / RUN " + refreshes + " OF 60" : "COMPLETE / MANUAL RUN";
      root.append(status);
      if (watch) root.append(control("STOP REFRESH", () => {
        stop();
        status.textContent = "REFRESH STOPPED";
      }));
    }

    async function execute() {
      if (destroyed) return;
      if (timer !== null) window.clearTimeout(timer);
      timer = null;
      screen = "running";
      const runId = ++generation;
      controller = new AbortController();
      root.replaceChildren();
      header("CANCEL", configure, operation.name);
      root.append(node("div", "net-summary", "RUNNING / " + operation.name));
      try {
        const [path, body] = request();
        const response = await fetch(path, {
          method: "POST", credentials: "same-origin", signal: controller.signal,
          headers: { "Content-Type": "application/json" }, body: JSON.stringify(body)
        });
        const data = await response.json();
        if (!response.ok) throw new Error(data.error || "Network tool failed");
        if (destroyed || runId !== generation) return;
        controller = null;
        refreshes++;
        if (refreshes >= 60) watch = false;
        lastResult = data;
        resultPage = 0;
        result();
        onStatus(operation.active ? "NETWORK / MANUAL REQUEST" : "OFFLINE / COMPUTED");
        if (watch) timer = window.setTimeout(() => {
          timer = null;
          if (!destroyed && !document.hidden) execute();
        }, 60000);
      } catch (error) {
        if (destroyed || runId !== generation) return;
        stop();
        screen = "error";
        root.replaceChildren();
        header("EDIT", configure, operation.name);
        const failure = node("p", "net-error", error.message);
        failure.setAttribute("role", "alert");
        root.append(failure);
        onStatus("NETWORK / ERROR");
      }
    }

    function visibility() {
      if (document.hidden) {
        stop();
        if (lastResult) result();
        else if (operation) configure();
      }
    }
    function pagehide() { stop(); }
    document.addEventListener("visibilitychange", visibility);
    document.addEventListener("keydown", onPageKey);
    window.addEventListener("pagehide", pagehide);
    menu();
    return {
      back() {
        if (destroyed || screen === "menu") return false;
        if (screen === "inputs") menu();
        else configure();
        return true;
      },
      destroy() {
        if (destroyed) return;
        destroyed = true;
        stop();
        document.removeEventListener("visibilitychange", visibility);
        document.removeEventListener("keydown", onPageKey);
        window.removeEventListener("pagehide", pagehide);
      }
    };
  }
  return Object.freeze({ mount });
})();
