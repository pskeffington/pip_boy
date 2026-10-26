"use strict";

// Native Rust radio controller. This UI never starts browser audio or external
// applications. Polling only reads state; all playback requires a button press.
(function () {
  const $ = (parent, sel) => parent.querySelector(sel);
  const node = (name, cls, text) => {
    const element = document.createElement(name);
    if (cls) element.className = cls;
    if (text !== undefined) element.textContent = String(text);
    return element;
  };
  const request = async (data) => {
    const options = { cache: "no-store", credentials: "omit" };
    if (data) {
      options.method = "POST";
      options.headers = { "Content-Type": "application/json" };
      options.body = JSON.stringify(data);
    }
    const response = await fetch("/api/radio", options);
    const result = await response.json();
    if (!response.ok) throw new Error(result.error || "Native radio request failed");
    return result;
  };
  function mount(root, options = {}) {
    root.replaceChildren();
    root.classList.add("rb-radio");
    if (options.compact) root.classList.add("rb-radio-compact");
    const top = node("div", "rb-radio-top");
    const label = node("strong", "", "RUST-BOY / RADIO");
    const indicator = node("span", "rb-radio-signal", "LOCAL / OFF");
    top.append(label, indicator);
    const selector = node("select", "rb-radio-station");
    selector.setAttribute("aria-label", "Choose curated radio station");
    const details = node("div", "rb-radio-details", "SELECT STATION");
    const outputLabel = node("label", "rb-radio-output-label", "AUDIO OUTPUT");
    const outputSelector = node("select", "rb-radio-output");
    outputSelector.setAttribute("aria-label", "Audio output device including connected Bluetooth");
    outputLabel.append(outputSelector);
    const controls = node("div", "rb-radio-controls");
    const play = node("button", "rb-radio-play", "▶ PLAY");
    play.type = "button";
    const stop = node("button", "rb-radio-stop", "■ STOP");
    stop.type = "button";
    controls.append(play, stop);
    const gain = node("div", "rb-radio-volume");
    const minus = node("button", "", "−");
    const plus = node("button", "", "+");
    minus.type = plus.type = "button";
    minus.setAttribute("aria-label", "Decrease radio volume");
    plus.setAttribute("aria-label", "Increase radio volume");
    const volume = node("span", "", "VOL --");
    gain.append(minus, volume, plus);
    const message = node("div", "rb-radio-message", "No receiver connected. IP radio is opt-in.");
    message.setAttribute("role", "status");
    message.setAttribute("aria-live", "polite");
    root.append(top, selector, details, outputLabel, controls, gain, message);
    let current = null;
    let busy = false;
    let alive = true;
    let serial = 0;
    let selected = null;
    let requestedOutput = null;
    const apply = (data) => {
      if (!alive) return;
      current = data;
      const list = Array.isArray(data.stations) ? data.stations : [];
      const previous = selected || selector.value || data.station;
      selector.replaceChildren();
      for (const station of list) {
        const option = node("option", "", station.name);
        option.value = station.id;
        selector.append(option);
      }
      selector.value = list.some((station) => station.id === previous)
        ? previous : (data.station || list[0]?.id || "");
      selected = selector.value;
      const outputs = Array.isArray(data.outputs) ? data.outputs : [];
      const chosenOutput = requestedOutput || data.output || "system";
      outputSelector.replaceChildren();
      for (const device of outputs) {
        const option = node("option", "", device.name);
        option.value = device.id;
        outputSelector.append(option);
      }
      outputSelector.value = outputs.some((d) => d.id === chosenOutput)
        ? chosenOutput : (data.output || "system");
      requestedOutput = null;
      outputSelector.disabled = busy || outputs.length === 0;
      const station = list.find((item) => item.id === selected);
      details.textContent = station ? station.genre.toUpperCase() + " / IP STREAM" : "NO STATIONS";
      indicator.textContent = data.status.toUpperCase();
      indicator.dataset.state = data.status;
      volume.textContent = "VOL " + data.volume + "%";
      const running = !["stopped", "failed"].includes(data.status);
      play.disabled = busy || running || !station;
      stop.disabled = busy || !running;
      selector.disabled = busy || running;
      minus.disabled = busy || !running || data.volume === 0;
      plus.disabled = busy || !running || data.volume === 100;
      if (data.error) message.textContent = data.error;
      else if (running) message.textContent =
        data.status === "playing" ? "Native PCM output active" :
          data.status === "connecting" ? "Connecting to broadcaster..." :
            data.status === "buffering" ? "Buffering audio..." :
            data.status === "recovering" ? "Reconnecting to approved broadcast..." :
              "Receiver: " + data.status;
      else message.textContent = data.status === "failed"
        ? "Stream failed. Select PLAY to retry." : "OFFLINE / READY";
    };
    const refresh = async () => {
      const sequence = ++serial;
      try {
        const data = await request();
        if (alive && !busy && sequence === serial) apply(data);
      } catch (error) {
        if (alive && !busy) message.textContent = error.message;
      }
    };
    const command = async (data) => {
      if (busy) return;
      busy = true;
      play.disabled = stop.disabled = true;
      message.textContent = "Applying native radio command...";
      try {
        const result = await request(data);
        if (alive) apply(result);
      } catch (error) {
        if (alive) message.textContent = error.message;
      } finally {
        busy = false;
        if (alive) refresh();
      }
    };
    selector.addEventListener("change", () => { selected = selector.value; });
    outputSelector.addEventListener("change", () => {
      requestedOutput = outputSelector.value;
      command({ action: "output", output: requestedOutput });
    });
    play.addEventListener("click", () => command({
      action: "play", station: selector.value, confirm: true
    }));
    stop.addEventListener("click", () => command({ action: "stop" }));
    minus.addEventListener("click", () => {
      if (current) command({ action: "volume", volume: Math.max(0, current.volume - 10) });
    });
    plus.addEventListener("click", () => {
      if (current) command({ action: "volume", volume: Math.min(100, current.volume + 10) });
    });
    refresh();
    const timer = setInterval(refresh, 1800);
    return {
      destroy() {
        alive = false;
        clearInterval(timer);
      }
    };
  }
  window.RustBoyRadio = Object.freeze({ mount });
})();
