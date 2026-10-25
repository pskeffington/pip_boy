"use strict";

/* Shared desktop/micro TASK CONTROL. No external assets or automatic delivery.
   All user-entered strings enter the DOM through textContent or input.value. */
(() => {
  const d = document;
  const node = (tag, css = "", text) => {
    const result = d.createElement(tag);
    if (css) result.className = css;
    if (text !== undefined) result.textContent = String(text);
    return result;
  };
  const button = (text, action, style = "") => {
    const result = node("button", "task-button " + style, text);
    result.type = "button";
    result.addEventListener("click", action);
    return result;
  };
  const label = (name, text, max = 96, value = "") => {
    const wrapper = node("label", "task-field", text);
    const input = node("input");
    input.name = name;
    input.maxLength = max;
    input.value = value;
    input.required = !["contact", "due", "details", "note"].includes(name);
    wrapper.append(input);
    return wrapper;
  };

  const get = async (path, payload) => {
    const opts = { cache: "no-store", credentials: "omit" };
    if (payload !== undefined) {
      opts.method = "POST";
      opts.headers = { "Content-Type": "application/json" };
      opts.body = JSON.stringify(payload);
    }
    const response = await fetch(path, opts);
    let body;
    try { body = await response.json(); }
    catch { throw new Error("Unrecognized Rust-Boy task response"); }
    if (!response.ok) throw new Error(body.error || "Task request rejected");
    return body;
  };

  const today = () => {
    const now = new Date();
    const pad = (value) => String(value).padStart(2, "0");
    return [now.getFullYear(), pad(now.getMonth() + 1), pad(now.getDate())].join("-");
  };
  const done = (task) => task.status === "done";
  const overdue = (task) => !!task.due && task.due < today() && !done(task);
  const sortTasks = (tasks) => tasks.slice().sort((a, b) => {
    if (done(a) !== done(b)) return done(a) ? 1 : -1;
    if (overdue(a) !== overdue(b)) return overdue(a) ? -1 : 1;
    const levels = { critical: 0, high: 1, normal: 2, low: 3 };
    return (levels[a.priority] - levels[b.priority]) || (b.id - a.id);
  });
  const counts = (tasks) => ({
    open: tasks.filter((t) => !done(t)).length,
    due: tasks.filter(overdue).length,
    blocked: tasks.filter((t) => t.status === "blocked").length,
    awaiting: tasks.filter((t) => t.delivery && !t.remote_progress?.acknowledged && !done(t)).length,
    done: tasks.filter(done).length
  });
  const statusText = (status) => ({
    todo: "TO DO", in_progress: "IN PROGRESS", blocked: "BLOCKED", done: "COMPLETE"
  }[status] || "UNKNOWN");
  const shortId = (id) => "RB-" + String(id).padStart(4, "0");
  const dateBadge = (task) => task.due
    ? (overdue(task) ? "OVERDUE " : "DUE ") + task.due
    : "NO DUE DATE";
  const relativeTime = (timestamp) => {
    if (!timestamp) return "NOT RECORDED";
    try { return new Date(timestamp * 1000).toLocaleString(); }
    catch { return "UNKNOWN TIME"; }
  };
  const badge = (text, type = "") => node("span", "task-tag " + type, text);
  const infoText = (message) => node("p", "task-help", message);
  function summary(root, options = {}) {
    root.replaceChildren(node("p", "task-help", "Loading local assignment summary…"));
    get("/api/tasks").then((response) => {
      if (!root.isConnected) return;
      const all = response.board.tasks;
      const own = options.role && !response.manager
        ? all.filter((task) => task.role === options.role) : all;
      const count = counts(own);
      const panel = node("section", "task-summary");
      const title = node("div", "task-heading");
      title.append(node("strong", "", response.manager ? "OPERATIONS / TASK CONTROL" : "ASSIGNED DUTIES"));
      title.append(badge(String(own.length) + " TASKS"));
      panel.append(title);
      const meter = node("div", "task-counts");
      for (const [name, value, tint] of [
        ["OPEN", count.open, ""], ["OVERDUE", count.due, "warn"], ["BLOCKED", count.blocked, "warn"]
      ]) {
        const cell = node("div", "task-count");
        cell.append(node("span", "", name), node("strong", tint, value));
        meter.append(cell);
      }
      panel.append(meter);
      const list = node("div", "task-mini-list");
      const top = sortTasks(own.filter((item) => !done(item))).slice(0, 3);
      if (!top.length) list.append(infoText("No outstanding assignments recorded for this station."));
      for (const item of top) {
        const line = node("div", "task-mini-row");
        line.append(node("strong", "", shortId(item.id)), node("span", "", item.title),
          badge(statusText(item.status), item.status === "blocked" ? "warn" : ""));
        list.append(line);
      }
      panel.append(list);
      if (typeof options.onOpen === "function") {
        panel.append(button(response.manager ? "OPEN TASK CONTROL" : "VIEW ASSIGNED DUTIES",
          options.onOpen, "solid"));
      }
      if (response.manager) panel.append(infoText("Assignments remain local until sent to a paired Rust-Boy."));
      root.replaceChildren(panel);
    }).catch((error) => {
      if (root.isConnected) root.replaceChildren(infoText("Task board unavailable: " + error.message));
    });
  }

  async function mount(root, options = {}) {
    const state = {
      data: null, peers: null, view: "board", filter: "open", selected: null,
      editor: null, action: false, status: "", error: ""
    };
    let epochs = 0;

    const showError = (message) => { state.error = message; render(); };
    const execute = async (request, success) => {
      if (state.action) return;
      state.action = true;
      state.error = "";
      state.status = "";
      try {
        await request();
        state.editor = null;
        state.status = success;
        await refresh();
      } catch (error) {
        state.error = error.message;
        render();
      } finally {
        state.action = false;
      }
    };

    async function refresh() {
      const epoch = ++epochs;
      try {
        const result = await get("/api/tasks");
        if (epoch !== epochs || !root.isConnected) return;
        state.data = result;
        if (result.manager) {
          try { state.peers = await get("/api/peers"); }
          catch { state.peers = null; }
        }
        if (epoch !== epochs || !root.isConnected) return;
        render();
      } catch (error) {
        if (epoch !== epochs || !root.isConnected) return;
        showError("Cannot load tasks: " + error.message);
      }
    }

    const fieldSelect = (name, caption, entries, value) => {
      const wrap = node("label", "task-field", caption);
      const sel = node("select");
      sel.name = name;
      for (const [id, title] of entries) {
        const item = node("option", "", title);
        item.value = id;
        sel.append(item);
      }
      if (value) sel.value = value;
      wrap.append(sel);
      return wrap;
    };

    function createForm() {
      const form = node("form", "task-edit-form");
      const title = node("div", "task-heading");
      title.append(node("strong", "", "NEW FIELD ASSIGNMENT"),
        button("CANCEL", () => { state.view = "board"; render(); }, "ghost"));
      form.append(title);
      const top = node("div", "task-form-grid");
      top.append(label("title", "TASK TITLE", 96));
      top.append(label("assignee", "ASSIGNEE / PERSON", 64));
      const roster = (options.profiles || []).map((profile) => [profile.id, profile.title]);
      top.append(fieldSelect("role", "ASSIGNED JOB STATION", roster, "head-engineer"));
      top.append(fieldSelect("priority", "PRIORITY",
        [["normal", "Normal"], ["low", "Low"], ["high", "High"], ["critical", "Critical"]], "normal"));
      const due = label("due", "DUE DATE (OPTIONAL)", 10);
      due.querySelector("input").type = "date";
      top.append(due);
      form.append(top);
      const description = node("label", "task-field", "INSTRUCTIONS (OPTIONAL)");
      const textarea = node("textarea");
      textarea.name = "details";
      textarea.maxLength = 400;
      textarea.rows = 2;
      textarea.placeholder = "Action, expected outcome and reporting instruction";
      textarea.addEventListener("input", () => {
        // The bounded handoff format is intentionally one logical line.
        textarea.value = textarea.value.replace(/[\r\n]+/g, " ");
      });
      description.append(textarea);
      form.append(description);
      form.append(infoText("Creation saves to local tasks.json only. No assignment is sent automatically."));
      const actions = node("div", "task-actions");
      const create = node("button", "task-button solid", "CREATE ASSIGNMENT");
      create.type = "submit";
      actions.append(create);
      form.append(actions);
      form.addEventListener("submit", async (event) => {
        event.preventDefault();
        if (state.action) return;
        const value = Object.fromEntries(new FormData(form).entries());
        value.due ||= null;
        await execute(() => get("/api/tasks/create", value), "Assignment saved locally.");
        if (!state.error) state.view = "board";
        render();
      });
      return form;
    }

    const statusControl = (task) => {
      const wrap = node("div", "task-actions");
      const stateSelect = fieldSelect("status", "STATUS", [
        ["todo", "To do"], ["in_progress", "In progress"],
        ["blocked", "Blocked"], ["done", "Complete"]
      ], task.status);
      wrap.append(stateSelect);
      const note = label("note", "UPDATE NOTE", 110);
      note.querySelector("input").placeholder = "Optional progress note";
      wrap.append(note);
      wrap.append(button("UPDATE", () => execute(
        () => get("/api/tasks/change", {
          id: task.id, revision: task.revision, operation: "status",
          status: stateSelect.querySelector("select").value,
          note: note.querySelector("input").value
        }), "Status recorded locally."), "solid"));
      return wrap;
    };

    function reassignForm(task) {
      const form = node("form", "task-edit-form");
      form.append(node("strong", "", "REASSIGN / DISPATCH RESET"));
      const cols = node("div", "task-form-grid");
      cols.append(label("assignee", "NEW ASSIGNEE", 64, task.assignee));
      cols.append(fieldSelect("role", "JOB", (options.profiles || [])
        .map((profile) => [profile.id, profile.title]), task.role));
      form.append(cols);
      form.append(infoText("Reassigning clears the prior device-delivery link and receipt state. Resend explicitly."));
      const save = node("button", "task-button solid", "SAVE REASSIGNMENT");
      save.type = "submit";
      form.append(save);
      form.addEventListener("submit", (event) => {
        event.preventDefault();
        const values = Object.fromEntries(new FormData(form).entries());
        execute(() => get("/api/tasks/change", {
          id: task.id, revision: task.revision, operation: "reassign",
          assignee: values.assignee, role: values.role, contact: null
        }), "Assignee changed; handoff reset.");
        state.editor = null;
      });
      return form;
    }

    function deviceDispatch(task) {
      const card = node("section", "task-handoff");
      card.append(node("strong", "", "SEND ASSIGNMENT / PEER DEVICE"));
      const available = state.peers?.peers || [];
      if (!available.length) {
        card.append(infoText("No Rust-Boy peer is paired. Set up device identity, trusted SSH host key, and a destination with the peer CLI before sending."));
        const hint = node("code", "task-packet",
          "pipboy peer init overseer-a --confirm\n" +
          "pipboy peer add scout-one pi@scout.local ABSOLUTE_SSH_KEY ABSOLUTE_KNOWN_HOSTS --confirm");
        card.append(hint);
        return card;
      }
      const destination = fieldSelect("peer", "PAIRED RUST-BOY", available
        .map((peer) => [peer.alias, peer.alias + " / " + peer.target]), available[0].alias);
      card.append(destination);
      card.append(infoText("Uses OpenSSH with a previously pinned host key. The receiver stores the task before a delivery receipt is recorded. No automatic polling or broadcast."));
      card.append(button("SEND TO SELECTED DEVICE", () => {
        const alias = destination.querySelector("select").value;
        if (!window.confirm("Send " + shortId(task.id) + " to paired device " + alias +
          " using OpenSSH? Remote storage is not a human acknowledgment.")) return;
        execute(() => get("/api/peer/send", {
          id: task.id, revision: task.revision, alias, confirm: true
        }), "Receiving Rust-Boy confirmed durable task storage. Operator acknowledgment pending.");
      }, "solid"));
      return card;
    }

    function deliveredStatus(task) {
      const card = node("section", "task-handoff");
      const receipt = task.delivery;
      card.append(node("strong", "", "DEVICE TRANSPORT / DELIVERY RECORDED"));
      card.append(infoText("Stored at " + receipt.peer + " / remote task " +
        shortId(receipt.remote_task_id) + ". Delivered over a trusted SSH session; human acknowledgment is separate."));
      card.append(infoText(task.remote_progress
        ? "Last received report: " + statusText(task.remote_progress.status) +
          " / acknowledged: " + (task.remote_progress.acknowledged ? "YES" : "NO") +
          " / checked " + relativeTime(task.remote_progress.observed_at)
        : "No device progress receipt imported yet."));
      card.append(button("REFRESH REMOTE STATUS", () => {
        if (!window.confirm("Connect to paired device " + receipt.peer +
          " to refresh task acknowledgment and progress?")) return;
        execute(() => get("/api/peer/sync", {
          alias: receipt.peer, confirm: true
        }), "Peer progress synchronized explicitly. No background polling enabled.");
      }, "solid"));
      return card;
    }

    function detail(task) {
      const container = node("div", "task-detail");
      const top = node("div", "task-heading");
      top.append(
        button("← BOARD", () => { state.selected = null; render(); }, "ghost"),
        badge(shortId(task.id))
      );
      container.append(top);
      container.append(node("h3", "task-title", task.title));
      const tags = node("div", "task-tags");
      tags.append(
        badge(task.priority.toUpperCase(), task.priority === "critical" ? "warn" : ""),
        badge(statusText(task.status), task.status === "blocked" ? "warn" : ""),
        badge(dateBadge(task), overdue(task) ? "warn" : "")
      );
      container.append(tags);
      const pairs = [
        ["ASSIGNEE", task.assignee],
        ["JOB", (options.profiles || []).find((p) => p.id === task.role)?.title || task.role],
        ["DEVICE", task.delivery
          ? "STORED AT " + task.delivery.peer
          : task.origin ? "RECEIVED FROM " + task.origin.device : "NOT SENT"],
        ["ACKNOWLEDGMENT", task.origin
          ? (task.acknowledged_at ? "ACKNOWLEDGED LOCALLY" : "ACTION REQUIRED")
          : task.remote_progress?.acknowledged ? "REPORTED BY REMOTE DEVICE"
            : task.acknowledged_at ? "MANUAL LEGACY FLAG" : "NOT RECEIVED"],
        ["REVISION", task.revision]
      ];
      const specs = node("div", "task-specs");
      pairs.forEach(([key, value]) => {
        const cell = node("div", "");
        cell.append(node("span", "", key), node("strong", "", value));
        specs.append(cell);
      });
      container.append(specs);
      if (task.details) container.append(node("p", "task-instructions", task.details));
      container.append(statusControl(task));
      const actions = node("div", "task-actions");
      if (state.data.manager && !task.origin) {
        if (!task.delivery) {
          actions.append(button("REASSIGN", () => {
            state.editor = state.editor === task.id ? null : task.id;
            render();
          }, "ghost"));
        }
      }
      if (task.origin && task.sent_at && !task.acknowledged_at) {
        actions.append(button("ACKNOWLEDGE RECEIPT", () => {
          if (!window.confirm("Acknowledge receipt of this assignment on this Rust-Boy?")) return;
          execute(() => get("/api/tasks/change", {
            id: task.id, revision: task.revision, operation: "acknowledge"
          }), "Acknowledgment saved on this device. The sender must sync to retrieve it.");
        }, "solid"));
      }
      container.append(actions);
      if (state.editor === task.id && state.data.manager && !task.delivery && !task.origin) {
        container.append(reassignForm(task));
      }
      if (state.data.manager && !task.origin) {
        container.append(task.delivery ? deliveredStatus(task) : deviceDispatch(task));
      }
      const history = node("details", "task-history");
      const heading = node("summary", "", "AUDIT HISTORY (" + task.events.length + ")");
      history.append(heading);
      for (const event of task.events.slice().reverse()) {
        const line = node("div", "task-event");
        line.append(node("strong", "", event.kind.toUpperCase()),
          node("span", "", relativeTime(event.at)),
          node("p", "", event.detail));
        history.append(line);
      }
      container.append(history);
      return container;
    }

    function lineItem(task) {
      const row = button("", () => { state.selected = task.id; render(); }, "task-row");
      const header = node("div", "task-row-head");
      header.append(node("strong", "", shortId(task.id) + "  " + task.title),
        badge(task.priority.toUpperCase(), task.priority === "critical" ? "warn" : ""));
      row.append(header);
      const sub = node("div", "task-row-foot");
      sub.append(
        node("span", "", task.assignee + " · " + task.role.replace(/-/g, " ")),
        badge(statusText(task.status), task.status === "blocked" ? "warn" : ""),
        badge(dateBadge(task), overdue(task) ? "warn" : "")
      );
      row.append(sub);
      return row;
    }

    function render() {
      if (!root.isConnected) return;
      root.replaceChildren();
      const wrap = node("div", "task-control");
      if (state.error) wrap.append(node("p", "task-message error", state.error));
      if (state.status) wrap.append(node("p", "task-message good", state.status));
      if (!state.data) {
        wrap.append(infoText("Loading task state from local Rust storage…"));
        root.append(wrap);
        return;
      }
      const data = state.data;
      const tasks = data.board.tasks;
      if (state.view === "new" && data.manager) {
        wrap.append(createForm());
        root.append(wrap);
        return;
      }
      const selected = tasks.find((entry) => entry.id === state.selected);
      if (selected) {
        wrap.append(detail(selected));
        root.append(wrap);
        return;
      }
      const top = node("div", "task-heading");
      top.append(node("strong", "", data.manager ? "OVERSEER / TASK COMMAND" : "STATION / DUTY QUEUE"),
        button("REFRESH", refresh, "ghost"));
      wrap.append(top);
      const totals = counts(tasks);
      const metrics = node("div", "task-counts");
      for (const [key, n, className] of [
        ["OPEN", totals.open, ""],
        ["OVERDUE", totals.due, "warn"],
        ["BLOCKED", totals.blocked, "warn"],
        ["AWAIT ACK", totals.awaiting, "warn"]
      ]) {
        const item = node("div", "task-count");
        item.append(node("span", "", key), node("strong", className, n));
        metrics.append(item);
      }
      wrap.append(metrics);
      if (data.manager) {
        wrap.append(button("+ NEW ASSIGNMENT", () => { state.view = "new"; state.error = ""; render(); }, "solid"));
      }
      const filters = node("div", "task-filters");
      for (const [id, text] of [
        ["open", "ACTIVE"], ["overdue", "OVERDUE"], ["blocked", "BLOCKED"],
        ["done", "DONE"], ["all", "ALL"]
      ]) {
        const filt = button(text, () => { state.filter = id; render(); },
          state.filter === id ? "selected" : "ghost");
        filt.setAttribute("aria-pressed", String(state.filter === id));
        filters.append(filt);
      }
      wrap.append(filters);
      const filtered = sortTasks(tasks).filter((task) => {
        if (state.filter === "open") return !done(task);
        if (state.filter === "overdue") return overdue(task);
        if (state.filter === "blocked") return task.status === "blocked";
        if (state.filter === "done") return done(task);
        return true;
      });
      const list = node("div", "task-list");
      if (!filtered.length) {
        list.append(node("div", "task-empty", data.manager
          ? "No tasks in this filter. Create an assignment or choose ALL."
          : "No assigned tasks in this filter. Role queues only show tasks recorded on this device."));
      } else {
        filtered.forEach((task) => list.append(lineItem(task)));
      }
      wrap.append(list);
      wrap.append(infoText("Device-to-device dispatch uses paired OpenSSH peers only. All sending and remote-status checks require explicit operator action."));
      root.append(wrap);
    }
    await refresh();
    return { refresh };
  }

  window.RustBoyTasks = Object.freeze({ mount, summary });
})();
