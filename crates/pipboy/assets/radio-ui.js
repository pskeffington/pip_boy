"use strict";

// Same-origin RADIO controls for the native Rust audio engine. Favorites and
// last-selected station are saved by Rust, never in browser storage. Only PLAY
// may initiate audio/network I/O; station browsing sends metadata updates.
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
    const favorite = node("button", "rb-radio-favorite", "☆");
    favorite.type = "button";
    favorite.setAttribute("aria-label", "Add station to favorites");
    favorite.setAttribute("aria-pressed", "false");
    const filter = node("button", "rb-radio-filter", "ALL");
    filter.type = "button";
    filter.setAttribute("aria-label", "Show favorites only");
    filter.setAttribute("aria-pressed", "false");
    const count = node("span", "rb-radio-count", "-- / --");
    const indicator = node("span", "rb-radio-signal", "OFF");
    top.append(label, favorite, filter, count, indicator);

    const tuner = node("div", "rb-radio-tuner");
    const previous = node("button", "rb-radio-prev rb-radio-step", "‹");
    previous.type = "button";
    previous.setAttribute("aria-label", "Previous curated station");
    const selector = node("select", "rb-radio-station");
    selector.setAttribute("aria-label", "Select a curated radio station");
    const next = node("button", "rb-radio-next rb-radio-step", "›");
    next.type = "button";
    next.setAttribute("aria-label", "Next curated station");
    tuner.append(previous, selector, next);
    const details = node("div", "rb-radio-details", "SELECT STATION");

    const transport = node("div", "rb-radio-transport");
    const controls = node("div", "rb-radio-controls");
    const play = node("button", "rb-radio-play", "▶ PLAY");
    const stop = node("button", "rb-radio-stop", "■ STOP");
    play.type = stop.type = "button";
    controls.append(play, stop);
    const gain = node("div", "rb-radio-volume");
    const minus = node("button", "rb-radio-volume-down", "−");
    const plus = node("button", "rb-radio-volume-up", "+");
    minus.type = plus.type = "button";
    minus.setAttribute("aria-label", "Decrease radio volume");
    plus.setAttribute("aria-label", "Increase radio volume");
    const volume = node("span", "rb-radio-volume-value", "VOL --");
    gain.append(minus, volume, plus);
    transport.append(controls, gain);

    const outputToggle = node("button", "rb-radio-output-toggle", "SETTINGS ▾");
    outputToggle.type = "button";
    outputToggle.setAttribute("aria-label", "Show audio output and favorite ordering");
    const outputPanel = node("div", "rb-radio-output-panel");
    const outputLabel = node("label", "rb-radio-output-label", "AUDIO OUTPUT");
    const outputSelector = node("select", "rb-radio-output");
    outputSelector.setAttribute("aria-label", "Audio output device including connected Bluetooth");
    outputLabel.append(outputSelector);
    const fallback = node("button", "rb-radio-fallback", "USE DEFAULT OUTPUT");
    fallback.type = "button";
    fallback.hidden = true;
    fallback.setAttribute("aria-label", "Choose system audio output after device failure");
    const order = node("div", "rb-radio-order");
    const orderLabel = node("span", "rb-radio-order-label", "FAVORITE ORDER");
    const orderEarlier = node("button", "rb-radio-order-earlier", "↑ EARLIER");
    const orderLater = node("button", "rb-radio-order-later", "↓ LATER");
    orderEarlier.type = orderLater.type = "button";
    orderEarlier.setAttribute("aria-label", "Move selected favorite earlier in preset order");
    orderLater.setAttribute("aria-label", "Move selected favorite later in preset order");
    order.append(orderLabel, orderEarlier, orderLater);
    outputPanel.append(outputLabel, fallback, order);

    const message = node("div", "rb-radio-message", "Select a station and press Play.");
    message.setAttribute("role", "status");
    message.setAttribute("aria-live", "polite");
    root.append(top, tuner, details, transport, outputToggle, outputPanel, message);

    let current = null;
    let busy = false;
    let alive = true;
    let serial = 0;
    let selected = null;
    let requestedOutput = null;
    let favoritesOnly = false;
    let outputExpanded = !compact;

    const favorites = (data) => Array.isArray(data?.presets?.favorites)
      ? data.presets.favorites : [];
    const stationList = (data) => {
      const stations = Array.isArray(data?.stations) ? data.stations : [];
      if (!favoritesOnly) return stations;
      const ordered = favorites(data)
        .map((id) => stations.find((station) => station.id === id))
        .filter(Boolean);
      const running = data && !["stopped", "failed"].includes(data.status);
      // A favorite removed while playing must never hide the live station.
      if (running && data.station && !ordered.some((item) => item.id === data.station)) {
        const active = stations.find((item) => item.id === data.station);
        if (active) ordered.unshift(active);
      }
      return ordered;
    };
    const showOutput = (expanded) => {
      outputExpanded = expanded;
      outputPanel.hidden = !expanded;
      outputToggle.setAttribute("aria-expanded", String(expanded));
      if (current) {
        const selectedOutput = current.outputs?.find(
          (item) => item.id === (current.output || "system")
        );
        setText(outputToggle, "SETTINGS / " +
          (selectedOutput?.name || "DISCONNECTED") + (expanded ? " ▴" : " ▾"));
      }
    };
    showOutput(outputExpanded);

    const stationInfo = (list) => {
      const index = list.findIndex((item) => item.id === selected);
      const chosen = index < 0 ? null : list[index];
      const suffix = favoritesOnly ? " ★" : " · " + (chosen?.category || "IP");
      setText(count, chosen
        ? String(index + 1).padStart(2, "0") +
            " / " + String(list.length).padStart(2, "0") + suffix
        : "00 / " + list.length + suffix);
      const quality = chosen && chosen.nominal_kbps > 0
        ? " · " + chosen.nominal_kbps + "K " + chosen.format : " · LIVE IP";
      const text = chosen ? chosen.genre + quality : favoritesOnly
        ? "NO FAVORITES / SELECT ALL TO BROWSE" : "NO STATIONS AVAILABLE";
      setText(details, text);
      details.title = text;
    };

    const apply = (data) => {
      if (!alive) return;
      current = data;
      const status = data.status || "stopped";
      const running = !["stopped", "failed"].includes(status);
      const list = stationList(data);
      const saved = favorites(data);
      const active = running && data.station && list.some((s) => s.id === data.station)
        ? data.station : null;
      const desired = active || selected || data.presets?.last_selected || data.station;
      selected = list.some((s) => s.id === desired) ? desired : (list[0]?.id || "");
      selector.replaceChildren();
      if (favoritesOnly) {
        for (const station of list) {
          const option = node("option", "", station.name);
          option.value = station.id;
          selector.append(option);
        }
      } else {
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
          const option = node("option", "",
            (saved.includes(station.id) ? "★ " : "") + station.name);
          option.value = station.id;
          group.append(option);
        }
      }
      selector.value = selected;
      stationInfo(list);
      const isFavorite = saved.includes(selected);
      setText(favorite, isFavorite ? "★" : "☆");
      favorite.setAttribute("aria-pressed", String(isFavorite));
      favorite.setAttribute("aria-label", isFavorite
        ? "Remove selected station from favorites" : "Add selected station to favorites");
      favorite.disabled = busy || !selected;
      setText(filter, favoritesOnly ? "★ ONLY" : "ALL");
      filter.setAttribute("aria-pressed", String(favoritesOnly));
      filter.disabled = busy || running;
      const favIndex = saved.indexOf(selected);
      order.hidden = favIndex < 0;
      orderEarlier.disabled = busy || favIndex <= 0;
      orderLater.disabled = busy || favIndex < 0 || favIndex >= saved.length - 1;

      const outputs = Array.isArray(data.outputs) ? data.outputs : [];
      const chosenOutput = requestedOutput || data.output || "system";
      outputSelector.replaceChildren();
      for (const device of outputs) {
        const option = node("option", "", device.name);
        option.value = device.id;
        option.disabled = device.available === false;
        outputSelector.append(option);
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
      const outputFailed = data.output_failure || data.output_available === false;
      if (outputFailed) showOutput(true);
      else showOutput(outputExpanded);
      outputSelector.disabled = busy || !outputs.some((device) => device.available !== false);
      const defaultOutput = outputs.find((device) => device.id === "system");
      fallback.hidden = !outputFailed;
      fallback.disabled = busy || !defaultOutput || defaultOutput.available === false;

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
      else setText(message, favoritesOnly && !list.length
        ? "NO FAVORITES / SELECT ALL TO BROWSE"
        : status === "failed" ? "STREAM FAILED / Press PLAY to retry."
          : "READY / SELECT STATION, PRESS PLAY");
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
      if (!alive || busy) return;
      busy = true;
      ++serial; // invalidate any outstanding status poll
      play.disabled = stop.disabled = true;
      previous.disabled = next.disabled = selector.disabled = true;
      favorite.disabled = filter.disabled = true;
      orderEarlier.disabled = orderLater.disabled = true;
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

    const selectStation = (id) => {
      if (!current || busy || !["stopped", "failed"].includes(current.status)) return;
      const list = stationList(current);
      if (!list.some((station) => station.id === id) || selected === id) return;
      selected = id;
      selector.value = id;
      stationInfo(list);
      command({ action: "select", station: id });
    };
    const stepStation = (delta) => {
      if (!current || busy) return;
      const list = stationList(current);
      if (!list.length) return;
      const index = list.findIndex((station) => station.id === selected);
      selectStation(list[((index < 0 ? 0 : index) + delta + list.length) % list.length].id);
    };

    selector.addEventListener("change", () => selectStation(selector.value));
    previous.addEventListener("click", () => stepStation(-1));
    next.addEventListener("click", () => stepStation(1));
    favorite.addEventListener("click", () => {
      if (!selected || !current) return;
      command({ action: "favorite", station: selected,
        enabled: !favorites(current).includes(selected) });
    });
    filter.addEventListener("click", () => {
      if (busy || !current || !["stopped", "failed"].includes(current.status)) return;
      favoritesOnly = !favoritesOnly;
      apply(current);
    });
    orderEarlier.addEventListener("click", () => {
      if (selected) command({ action: "move_favorite", station: selected, direction: "earlier" });
    });
    orderLater.addEventListener("click", () => {
      if (selected) command({ action: "move_favorite", station: selected, direction: "later" });
    });
    outputToggle.addEventListener("click", () => {
      if (!busy) showOutput(!outputExpanded);
    });
    outputSelector.addEventListener("change", () => {
      requestedOutput = outputSelector.value;
      command({ action: "output", output: requestedOutput });
    });
    fallback.addEventListener("click", () => {
      requestedOutput = "system";
      command({ action: "output", output: "system" });
    });
    play.addEventListener("click", () => {
      if (selected) command({ action: "play", station: selected, confirm: true });
    });
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
