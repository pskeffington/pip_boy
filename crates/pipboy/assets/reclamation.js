"use strict";
// Manual, local field observations. No geolocation request or network destination.
(() => {
  const statuses = ["logged", "in_progress", "needs_assistance", "completed"];
  const e = (tag, text, className = "") => {
    const node = document.createElement(tag);
    node.textContent = text;
    node.className = className;
    return node;
  };
  async function api(path, data) {
    const options = { cache: "no-store", credentials: "omit" };
    if (data !== undefined) {
      options.method = "POST";
      options.headers = { "Content-Type": "application/json" };
      options.body = JSON.stringify(data);
    }
    const response = await fetch(path, options);
    const body = await response.json();
    if (!response.ok) throw new Error(body.error || "Reclamation request refused");
    return body;
  }
  function field(label, name, value, options = []) {
    const wrapper = e("label", label, "rb-rec-field");
    const input = e("input", "");
    input.name = name;
    input.value = value;
    input.maxLength = name === "notes" ? 512 : name === "name" ? 64 : 40;
    input.autocomplete = "off";
    if (["latitude", "longitude", "accuracy_m"].includes(name)) input.inputMode = "decimal";
    if (name !== "accuracy_m" && name !== "notes") input.required = true;
    if (options.length) {
      const select = e("select", "");
      select.name = name;
      options.forEach(([value, label]) => {
        const option = e("option", label);
        option.value = value;
        select.append(option);
      });
      wrapper.append(select);
    } else wrapper.append(input);
    return wrapper;
  }
  function mount(root, {compact = false} = {}) {
    let epoch = 0;
    let busy = false;
    const title = e("div", "SITE REGISTER / LOCAL", "rb-rec-title");
    const status = e("p", "Reading local sites…", "rb-rec-status");
    const actions = e("div", "", "rb-rec-actions");
    const list = e("div", "", "rb-rec-list");
    const form = e("form", "", "rb-rec-form");
    const add = e("button", "NEW SITE", "rb-rec-button");
    add.type = "button";
    const refresh = e("button", "REFRESH", "rb-rec-button");
    refresh.type = "button";
    const exportBtn = e("button", "GEOJSON", "rb-rec-button");
    exportBtn.type = "button";
    actions.append(add, refresh, exportBtn);
    form.hidden = true;
    form.append(
      field("SITE NAME", "name", ""),
      field("TYPE", "kind", "", [["water","WATER"],["sanitation","SANITATION"],["infrastructure","INFRASTRUCTURE"],["terrain","TERRAIN"],["supplies","SUPPLIES"],["other","OTHER"]]),
      field("CONDITION", "condition", "", [["unknown","UNKNOWN"],["serviceable","SERVICEABLE"],["degraded","DEGRADED"],["unsafe","UNSAFE"]]),
      field("PRIORITY", "priority", "", [["routine","ROUTINE"],["high","HIGH"],["urgent","URGENT"]]),
      field("LATITUDE", "latitude", ""),
      field("LONGITUDE", "longitude", ""),
      field("POSITION UNCERTAINTY (M, OPTIONAL)", "accuracy_m", ""),
      field("NOTES", "notes", "")
    );
    const formButtons = e("div", "", "rb-rec-actions");
    const save = e("button", "SAVE SITE", "rb-rec-button rb-rec-primary");
    save.type = "submit";
    const cancel = e("button", "CANCEL", "rb-rec-button");
    cancel.type = "button";
    cancel.addEventListener("click", () => { form.hidden = true; });
    formButtons.append(save, cancel);
    form.append(formButtons);
    root.replaceChildren(title, status, actions, form, list);
    async function load() {
      const ticket = ++epoch;
      try {
        const result = await api("/api/reclamation");
        if (ticket !== epoch || !root.isConnected) return;
        list.replaceChildren();
        const sites = result.book.sites;
        status.textContent = sites.length + " SITES · MANUAL POSITIONS · OFFLINE";
        if (!sites.length) list.append(e("p", "No sites recorded. Select NEW SITE.", "rb-rec-empty"));
        for (const site of sites) {
          const card = e("article", "", "rb-rec-site");
          const head = e("div", "", "rb-rec-row");
          head.append(e("strong", "#" + site.id + " " + site.name),
            e("span", site.priority.toUpperCase(), "rb-rec-priority"));
          card.append(head, e("div", site.kind.toUpperCase() + " / " + site.condition.toUpperCase() +
            " / " + site.status.replaceAll("_", " ").toUpperCase(), "rb-rec-meta"));
          card.append(e("div", site.latitude.toFixed(6) + ", " + site.longitude.toFixed(6) +
            " · MANUAL" + (site.accuracy_m === null ? " · ACCURACY UNKNOWN" : " · ±" + site.accuracy_m + " M (EST.)"), "rb-rec-meta"));
          if (site.notes) card.append(e("p", site.notes, "rb-rec-note"));
          const controls = e("div", "", "rb-rec-actions");
          const change = e("select", "");
          change.setAttribute("aria-label", "Status for site " + site.id);
          for (const value of statuses) {
            const option = e("option", value.replaceAll("_", " ").toUpperCase());
            option.value = value;
            change.append(option);
          }
          change.value = site.status;
          const apply = e("button", "UPDATE", "rb-rec-button");
          apply.type = "button";
          apply.addEventListener("click", async () => {
            if (busy || change.value === site.status) return;
            busy = true;
            apply.disabled = true;
            try {
              await api("/api/reclamation/change", {id:site.id, revision:site.revision, status:change.value});
              await load();
            } catch (err) { status.textContent = "ERROR / " + err.message; }
            finally { busy = false; apply.disabled = false; }
          });
          controls.append(change, apply);
          card.append(controls);
          list.append(card);
        }
      } catch (err) {
        if (ticket === epoch && root.isConnected) status.textContent = "ERROR / " + err.message;
      }
    }
    add.addEventListener("click", () => { form.hidden = !form.hidden; });
    refresh.addEventListener("click", load);
    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      if (busy) return;
      const data = Object.fromEntries(new FormData(form));
      const latitude = Number(data.latitude);
      const longitude = Number(data.longitude);
      const accuracy = data.accuracy_m === "" ? null : Number(data.accuracy_m);
      if (!Number.isFinite(latitude) || !Number.isFinite(longitude) ||
          (accuracy !== null && (!Number.isFinite(accuracy) || accuracy < 0))) {
        status.textContent = "ERROR / INVALID COORDINATES";
        return;
      }
      busy = true;
      save.disabled = true;
      try {
        await api("/api/reclamation/create", {
          name:data.name, kind:data.kind, condition:data.condition, priority:data.priority,
          latitude, longitude, accuracy_m:accuracy, notes:data.notes
        });
        form.reset();
        form.hidden = true;
        await load();
      } catch (err) { status.textContent = "ERROR / " + err.message; }
      finally { busy = false; save.disabled = false; }
    });
    exportBtn.addEventListener("click", async () => {
      try {
        const collection = await api("/api/reclamation/geojson");
        const blob = new Blob([JSON.stringify(collection, null, 2)], {type:"application/geo+json"});
        const url = URL.createObjectURL(blob);
        const a = e("a", "");
        a.href = url;
        a.download = "rust-boy-reclamation.geojson";
        a.click();
        setTimeout(() => URL.revokeObjectURL(url), 1000);
      } catch (err) { status.textContent = "ERROR / " + err.message; }
    });
    load();
    return {refresh:load, compact};
  }
  window.RustBoyReclamation = Object.freeze({mount});
})();
