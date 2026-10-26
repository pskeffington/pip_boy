"use strict";

// Radio UI is a control panel for the same-origin native Rust singleton.
// Browsing is local: no stream requests, audio or device scans are triggered.
(function () {
  const node = (tag, className, text) => {
    const element = document.createElement(tag);
    if (className) element.className = className;
    if (text !== undefined) element.textContent = String(text);
    return element;
  };
  const setText = (element, text) => {
    if (element.textContent !== text) element.textContent = text;
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
    const compact = options.compact === true;
    if (compact) root.classList.add("rb-radio-compact");

    const top = node("div", "rb-radio-top");
    const label = node("strong", "rb-radio-title", "RADIO / IP");
    const count = node("span", "rb-radio-count", "-- / --");
    const indicator = node("span", "rb-radio-signal", "OFF");
    top.append(label, count, indicator);

    const tuner = node("div", "rb-radio-tuner");
    const previous = node("button", "rb-radio-prev rb-radio-step", "‹");
    previous.type = "button";
    previous.setAttribute("aria-label", "Previous curated station");
    const selector = node("select", "rb-radio-station");
    selector.setAttribute("aria-label", "Curated radio stations grouped by genre");
    const next = node("button", "rb-radio-next rb-radio-step", "›");
    next.type = "button";
    next.setAttribute("aria-label", "Next curated station");
    tuner.append(previous, selector, next);
    const details = node("div", "rb-radio-details", "SELECT STATION");

    const controls = node("div", "rb-radio-controls");
    const play = node("button", "rb-radio-play", "▶ PLAY");
    const stop = node("button", "rb-radio-stop", "■ STOP");
    play.type = stop.type = "button";
    controls.append(play, stop);

    const volumeControls = node("div", "rb-radio-volume");
    const minus = node("button", "rb-radio-volume-down", "−");
    const plus = node("button", "rb-radio-volume-up", "+");
    minus.type = plus.type = "button";
    minus.setAttribute("aria-label", "Decrease radio volume");
    plus.setAttribute("aria-label", "Increase radio volume");
    const volume = node("span", "rb-radio-volume-value", "VOL --");
    volumeControls.append(minus, volume, plus);

    const outputToggle = node("button", "rb-radio-output-toggle", "OUT / SYSTEM ▾");
    outputToggle.type = "button";
    outputToggle.setAttribute("aria-label", "Show or hide audio output devices");
    const outputPanel = node("div", "rb-radio-output-panel");
    const outputLabel = node("label", "rb-radio-output-label", "AUDIO OUTPUT");
    const outputSelector = node("select", "rb-radio-output");
    outputSelector.setAttribute("aria-label", "Audio output device including connected Bluetooth");
    outputLabel.append(outputSelector);
    const fallback = node("button", "rb-radio-fallback", "USE DEFAULT OUTPUT");
    fallback.type = "button";
    fallback.hidden = true;
    fallback.setAttribute("aria-label", "Choose system output after audio device failure");
    outputPanel.append(outputLabel, fallback);

    const message = node("div", "rb-radio-message", "Ready. Select a station and press Play.");
    message.setAttribute("role", "status");
    message.setAttribute("aria-live", "polite");
    root.append(top, tuner, details, controls, volumeControls, outputToggle, outputPanel, message);

    let current = null;
    let busy = false;
    let alive = true;
    let serial = 0;
    let selected = null;
    let requestedOutput = null;
    let outputExpanded = !compact;

    const showOutput = (expanded) => {
      outputExpanded = expanded;
      outputPanel.hidden = !expanded;
      outputToggle.setAttribute("aria-expanded", String(expanded));
    };
    showOutput(outputExpanded);

    const stationInfo = (list) => {
      const index = list.findIndex((item) => item.id === selected);
      const chosen = index < 0 ? null : list[index];
      setText(count, chosen ? String(index + 1).padStart(2, "0") +
        " / " + String(list.length).padStart(2, "0") + " · " +
        (chosen.category || "IP") : "00 / " + list.length);
      const quality = chosen && chosen.nominal_kbps > 0
        ? " · " + chosen.nominal_kbps + "K " + chosen.format : " · LIVE IP";
      const line = chosen ? chosen.genre + quality : "NO STATIONS AVAILABLE";
      setText(details, line);
      details.title = line;
    };
    const setStation = (id) => {
      if (!current || busy || !["stopped", "failed"].includes(current.status)) return;
      const list = Array.isArray(current.stations) ? current.stations : [];
      if (!list.some((item) => item.id === id)) return;
      selected = id;
      selector.value = id;
      stationInfo(list);
    };
    const stepStation = (delta) => {
      if (!current || busy) return;
      const list = Array.isArray(current.stations) ? current.stations : [];
      if (!list.length) return;
      const index = list.findIndex((item) => item.id === selected);
      setStation(list[((index < 0 ? 0 : index) + delta + list.length) % list.length].id);
    };
    const apply = (data) => {
      if (!alive) return;
      current = data;
      const list = Array.isArray(data.stations) ? data.stations : [];
      const preferred = selected || selector.value || data.station;
      selector.replaceChildren();
      const groups = new Map();
      for (const station of list) {
        const groupName = station.category || "OTHER";
        let group = groups.get(groupName);
        if (!group) {
          group = node("optgroup");
          group.setAttribute("label", groupName);
          groups.set(groupName, group);
          selector.append(group);
        }
        const item = node("option", "", station.name);
        item.value = station.id;
        group.append(item);
      }
      selected = list.some((item) => item.id === preferred) ? preferred
        : (list.some((item) => item.id === data.station) ? data.station : (list[0]?.id || ""));
      selector.value = selected;
      stationInfo(list);

      const outputs = Array.isArray(data.outputs) ? data.outputs : [];
      const chosenOutput = requestedOutput || data.output || "system";
      outputSelector.replaceChildren();
      for (const device of outputs) {
        const item = node("option", "", device.name);
        item.value = device.id;
        item.disabled = device.available === false;
        outputSelector.append(item);
      }
      const selectedDevice = outputs.find((device) => device.id === chosenOutput);
      if (!selectedDevice) {
        const missing = node("option", "", "OUTPUT DISCONNECTED — SELECT DEVICE");
        missing.value = chosenOutput;
        missing.disabled = true;
        outputSelector.prepend(missing);
      }
      outputSelector.value = chosenOutput;
      requestedOutput = null;
      const outputName = selectedDevice ? selectedDevice.name : "DISCONNECTED";
      setText(outputToggle, "OUT / " + outputName + (outputExpanded ? " ▴" : " ▾"));
      outputToggle.title = "Select output device; Bluetooth pairing happens in the OS";
      const outputFailed = data.output_failure || data.output_available === false;
      if (outputFailed) showOutput(true);
      outputSelector.disabled = busy || !outputs.some((device) => device.available !== false);
      const defaultOutput = outputs.find((device) => device.id === "system");
      fallback.hidden = !outputFailed;
      fallback.disabled = busy || !defaultOutput || defaultOutput.available === false;

      const status = data.status || "stopped";
      const running = !["stopped", "failed"].includes(status);
      const failed = status === "failed";
      setText(indicator, data.output_failure ? "OUTPUT FAILED" :
        data.output_available === false ? "NO OUTPUT" : "IP / " + status.toUpperCase());
      indicator.dataset.state = outputFailed ? "failed" : status;
      setText(volume, "VOL " + data.volume + "%");
      const canTune = !busy && !running && list.length > 0;
      selector.disabled = !canTune;
      previous.disabled = next.disabled = !canTune || list.length < 2;
      play.disabled = !canTune || data.output_available === false;
      stop.disabled = busy || (status === "stopped" && !data.station);
      minus.disabled = busy || !running || data.volume === 0;
      plus.disabled = busy || !running || data.volume === 100;
      if (data.output_failure) setText(message,
        "OUTPUT FAILED / Select a connected device to retry.");
      else if (data.output_available === false) setText(message,
        "NO OUTPUT / Connect audio or select System default.");
      else if (data.error) setText(message, data.error);
      else if (running) setText(message,
        status === "playing" ? "NATIVE AUDIO / PLAYING" :
        status === "connecting" ? "CONNECTING TO BROADCASTER" :
        status === "buffering" ? "BUFFERING AUDIO" :
        status === "recovering" ? "Reconnecting to approved broadcast..." :
        "RECEIVER / " + status.toUpperCase());
      else setText(message, failed ? "STREAM FAILED / Press PLAY to retry." :
        "READY / SELECT STATION, PRESS PLAY");
    };
    const refresh = async () => {
      const sequence = ++serial;
      try {
        const data = await request();
        if (alive && !busy && sequence === serial) apply(data);
      } catch (error) {
        if (alive && !busy) setText(message, error.message);
      }
    };
    const command = async (data) => {
      if (busy) return;
      busy = true;
      play.disabled = stop.disabled = true;
      previous.disabled = next.disabled = selector.disabled = true;
      outputSelector.disabled = outputToggle.disabled = fallback.disabled = true;
      minus.disabled = plus.disabled = true;
      setText(message, "APPLYING RADIO COMMAND");
      try {
        const result = await request(data);
        if (alive) {
          apply(result);
          if (data.action === "output" && !result.output_failure &&
              result.output_available !== false && compact) showOutput(false);
        }
      } catch (error) {
        if (alive) {
          requestedOutput = null;
          setText(message, error.message);
        }
      } finally {
        busy = false;
        if (alive) {
          outputToggle.disabled = false;
          refresh();
        }
      }
    };
    selector.addEventListener("change", () => setStation(selector.value));
    previous.addEventListener("click", () => stepStation(-1));
    next.addEventListener("click", () => stepStation(1));
    outputToggle.addEventListener("click", () => {
      if (!busy) showOutput(!outputExpanded);
      setText(outputToggle, "OUT / " +
        (current?.outputs?.find((device) => device.id === (current.output || "system"))?.name
          || "DISCONNECTED") + (outputExpanded ? " ▴" : " ▾"));
    });
    outputSelector.addEventListener("change", () => {
      requestedOutput = outputSelector.value;
      command({ action: "output", output: requestedOutput });
    });
    fallback.addEventListener("click", () => {
      requestedOutput = "system";
      command({ action: "output", output: "system" });
    });
    play.addEventListener("click", () => command({
      action: "play", station: selected, confirm: true
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
