"use strict";
// Public, explicitly curated DOT cameras only. The Rust catalog owns endpoints.
// Image requests begin only after the operator opens the Cameras view.
(function () {
  const MAX_VIEWS = 4, MAX_ATTEMPTS = 12, MAX_PARALLEL = 2;
  const TIMEOUT_MS = 8000, RECENT_MS = 120000;
  const make = (tag, cls, value) => {
    const node = document.createElement(tag);
    node.className = cls;
    if (value !== undefined) node.textContent = value;
    return node;
  };
  const point = value => value && typeof value.lat === "number" &&
    typeof value.lon === "number" && Number.isFinite(value.lat) &&
    Number.isFinite(value.lon) && Math.abs(value.lat) <= 90 &&
    Math.abs(value.lon) <= 180 ? value : null;
  function distanceMiles(a, b) {
    const rad = Math.PI / 180;
    const dLat = (b.lat - a.lat) * rad, dLon = (b.lon - a.lon) * rad;
    const sine = Math.sin(dLat / 2) ** 2 +
      Math.cos(a.lat * rad) * Math.cos(b.lat * rad) *
      Math.sin(dLon / 2) ** 2;
    return 3958.7613 * 2 * Math.asin(Math.sqrt(Math.min(1, Math.max(0, sine))));
  }
  function safeImage(feed) {
    if (!feed || typeof feed.image_url !== "string" || !feed.image_url) return false;
    try {
      const url = new URL(feed.image_url);
      if (url.protocol !== "https:" || url.username || url.password ||
          url.port || url.search || url.hash) return false;
      return (url.hostname === "snapshot.vdotcameras.com" &&
        /^\/thumbs\/[A-Za-z0-9_-]+\.flv\.png$/.test(url.pathname)) ||
        (url.hostname === "cwwp2.dot.ca.gov" &&
        /^\/data\/d[0-9]{1,2}\/cctv\/image\/[A-Za-z0-9_-]+\/[A-Za-z0-9_-]+\.jpg$/.test(url.pathname));
    } catch (_) { return false; }
  }
  function safeSource(value) {
    try {
      const url = new URL(value);
      return url.protocol === "https:" && !url.username && !url.password &&
        !url.port && !url.hash ? url.href : null;
    } catch (_) { return null; }
  }
  // These signals must be generated from RECENT agency metadata. A camera URL
  // being present (or its image loading) is NOT proof of live video.
  function liveEvidence(feed, now) {
    const statusAt = Date.parse(feed.status_checked_at || "");
    const frameAt = Date.parse(feed.frame_captured_at || "");
    return feed.active === true && Number.isFinite(statusAt) &&
      Number.isFinite(frameAt) && now >= statusAt && now >= frameAt &&
      now - statusAt <= RECENT_MS && now - frameAt <= RECENT_MS;
  }
  function staleAgency(feed, now) {
    const frameAt = Date.parse(feed.frame_captured_at || "");
    const statusAt = Date.parse(feed.status_checked_at || "");
    return feed.active === false || (feed.active === true &&
      Number.isFinite(statusAt) && now >= statusAt &&
      now - statusAt <= RECENT_MS && Number.isFinite(frameAt) &&
      now - frameAt > RECENT_MS);
  }
  // Distance ranking is deterministic and operates on actual camera coordinates;
  // gated city placeholders cannot be automatically selected.
  function rankFeeds(input, location, region) {
    const origin = point(location), chosenRegion = /^[A-Z]{2}$/.test(region || "") ? region : null;
    const records = (Array.isArray(input) ? input : [])
      .filter(feed => feed && typeof feed.id === "string" &&
        /^[a-zA-Z0-9_-]{1,64}$/.test(feed.id) &&
        typeof feed.city === "string" && typeof feed.publisher === "string" &&
        (safeImage(feed) || feed.format === "unavailable"))
      .map((feed, index) => ({
        ...feed, availableSource: safeImage(feed),
        distance_mi: origin && point(feed) ? distanceMiles(origin, feed) : null,
        originalIndex: index
      }));
    return records.sort((a, b) => {
      if (a.availableSource !== b.availableSource) return a.availableSource ? -1 : 1;
      if (origin && a.distance_mi !== null && b.distance_mi !== null)
        return a.distance_mi - b.distance_mi || a.originalIndex - b.originalIndex;
      if (!origin && chosenRegion) {
        const diff = Number(b.region === chosenRegion) - Number(a.region === chosenRegion);
        if (diff) return diff;
      }
      return a.originalIndex - b.originalIndex;
    });
  }
  function pickNearestActive(sorted, health) {
    for (const status of ["live", "reachable"]) {
      const feed = sorted.find(f => f.availableSource && health[f.id] === status);
      if (feed) return feed;
    }
    return null;
  }
  function mount(root, options = {}) {
    let connected = true, inFlight = 0, attempts = 0, cursor = 0, probeSerial = 0;
    const location = point(options.location);
    const feeds = rankFeeds(options.feeds, location, options.region);
    const candidates = feeds.filter(feed => feed.availableSource);
    const statuses = new Map(); // per-camera, in memory only; never persisted
    let focused = typeof options.selected === "string" ? options.selected :
      Number.isInteger(options.selected) && options.selected > 0 ?
        feeds[options.selected - 1]?.id || null : null;
    const healthSnapshot = () => Object.fromEntries(
      [...statuses].map(([id, value]) => [id, value.status])
    );
    const primary = () => pickNearestActive(feeds, healthSnapshot());
    const button = (label, action) => {
      const node = make("button", "rb-camera-button", label);
      node.type = "button"; node.addEventListener("click", action);
      return node;
    };
    const choose = id => {
      if (!connected) return;
      focused = id;
      options.onSelect?.(id);
      render();
    };
    function showState(feed) {
      if (!feed.availableSource) return "PROVIDER ACCESS REQUIRED · NO DIRECT FEED";
      if (staleAgency(feed, Date.now())) return feed.active === false ?
        "AGENCY INACTIVE" : "AGENCY FRAME STALE";
      const state = statuses.get(feed.id);
      if (!state || state.status === "untried") return "AWAITING CHECK";
      if (state.status === "checking") return "CHECKING IMAGE ENDPOINT";
      if (state.status === "live") return "LIVE · AGENCY FRAME TIMESTAMP VERIFIED";
      if (state.status === "reachable") return "REACHABLE · FRESHNESS UNVERIFIED";
      if (state.status === "expired") return "CHECK EXPIRED · REFRESH REQUIRED";
      return "FEED UNAVAILABLE";
    }
    function render() {
      if (!connected) return;
      root.replaceChildren();
      root.classList.add("rb-camera-view");
      const best = primary();
      let title = "DOT / PUBLIC ROAD VIEWS · NO LOCATION SELECTED";
      if (location) {
        const label = options.location.source === "zip_county" ? "COUNTY APPROX" :
          options.location.source === "gps" ? "RECENT GPS" : "MANUAL CENTER";
        title = "DOT / " + label + " · " + (best ? "NEAREST " +
          (statuses.get(best.id)?.status === "live" ? "LIVE" : "REACHABLE") +
          " " + best.distance_mi.toFixed(1) + " MI" : "CHECKING NEAREST CAMERA");
      } else if (best) title = "DOT / DEFAULT REACHABLE CAMERA · SET LOCATION IN SETTINGS";
      root.append(make("div", "rb-camera-heading", title));
      if (!feeds.length) {
        root.append(make("p", "", "NO CURATED DOT SOURCES AVAILABLE"));
        return;
      }
      if (focused && !feeds.some(f => f.id === focused)) focused = null;
      const grid = make("div", "rb-camera-grid" + (focused ? " rb-camera-focused" : ""));
      const ready = feeds.filter(f => ["live", "reachable"].includes(statuses.get(f.id)?.status));
      const rest = feeds.filter(f => !ready.includes(f) && statuses.get(f.id)?.status !== "offline");
      const failed = feeds.filter(f => statuses.get(f.id)?.status === "offline");
      const visible = focused ? feeds.filter(f => f.id === focused) :
        [...(best ? [best] : []), ...ready.filter(f => f !== best), ...rest, ...failed]
          .filter((f, i, arr) => arr.indexOf(f) === i).slice(0, MAX_VIEWS);
      for (const feed of visible) {
        const tile = make("section", "rb-camera-tile" +
          (best?.id === feed.id ? " rb-camera-primary" : ""));
        const top = make("div", "rb-camera-top");
        top.append(make("strong", "", feed.city),
          button(focused ? "GRID" : "OPEN", () => choose(focused ? null : feed.id)));
        tile.append(top, make("span", "rb-camera-name", feed.name));
        if (feed.distance_mi !== null) {
          tile.append(make("span", "rb-camera-distance", feed.distance_mi.toFixed(1) + " MI · " +
            (best?.id === feed.id ? "PRIMARY" : feed.region || "DOT")));
        } else if (best?.id === feed.id) {
          tile.append(make("span", "rb-camera-distance", "PRIMARY / DEFAULT"));
        }
        const state = statuses.get(feed.id);
        if (state?.img && ["live", "reachable"].includes(state.status) &&
            (!options.compact || best?.id === feed.id || focused)) {
          state.img.className = "rb-camera-player";
          state.img.alt = "DOT camera image; freshness is not established by image load";
          tile.append(state.img);
        }
        tile.append(make("span", "rb-camera-status", showState(feed)));
        if (feed.availableSource) {
          tile.append(button("REFRESH", () => retry(feed)));
        }
        const url = safeSource(feed.source_url);
        if (url) {
          const link = make("a", "rb-camera-source", feed.publisher + " / SOURCE");
          link.href = url; link.target = "_blank"; link.rel = "noopener noreferrer";
          tile.append(link);
        }
        grid.append(tile);
      }
      root.append(grid);
      if (best && !focused) root.append(button("FOCUS NEAREST", () => choose(best.id)));
      if (!best && !inFlight && attempts) {
        root.append(make("div", "rb-camera-status", "NO WORKING FEED CONFIRMED"));
        root.append(button("RECHECK", recheck));
      }
    }
    function drop(state) {
      if (!state) return;
      // Cancel callbacks from an old probe before starting a replacement.
      state.finished = true;
      if (state.timer !== null) clearTimeout(state.timer);
      if (state.expiry !== null) clearTimeout(state.expiry);
      if (state.img) {
        state.img.onload = null; state.img.onerror = null;
        state.img.removeAttribute("src");
      }
    }
    function probe(feed) {
      if (!connected || !safeImage(feed) || staleAgency(feed, Date.now())) return;
      const old = statuses.get(feed.id);
      if (old?.status === "checking") return;
      drop(old);
      const state = { status: "checking", img: document.createElement("img"),
        timer: null, expiry: null, finished: false };
      state.img.referrerPolicy = "no-referrer";
      state.img.decoding = "async";
      state.img.loading = "eager";
      statuses.set(feed.id, state);
      inFlight++;
      const complete = ok => {
        if (state.finished) return;
        state.finished = true;
        if (state.timer !== null) clearTimeout(state.timer);
        inFlight = Math.max(0, inFlight - 1);
        if (!connected || statuses.get(feed.id) !== state) return;
        state.status = ok ? (liveEvidence(feed, Date.now()) ? "live" : "reachable") : "offline";
        if (!ok) state.img.removeAttribute("src");
        if (ok) state.expiry = setTimeout(() => {
          if (!connected || statuses.get(feed.id) !== state) return;
          state.status = "expired";
          state.img.removeAttribute("src");
          render();
          ensureProbes();
        }, RECENT_MS);
        render();
        ensureProbes();
      };
      state.img.addEventListener("load", () => complete(true));
      state.img.addEventListener("error", () => complete(false));
      state.timer = setTimeout(() => complete(false), TIMEOUT_MS);
      // Unique request prevents a browser cache hit being mistaken for a fresh check.
      // The image may still be a static upstream frame: never call it LIVE without
      // agency capture-time evidence.
      state.img.src = feed.image_url + "?rb_probe=" + Date.now() + "_" + (++probeSerial);
    }
    function ensureProbes() {
      if (!connected) return;
      const ready = [...statuses.values()].filter(s =>
        s.status === "live" || s.status === "reachable").length;
      while (inFlight < MAX_PARALLEL && ready + inFlight < MAX_VIEWS &&
             attempts < MAX_ATTEMPTS && cursor < candidates.length) {
        const next = candidates[cursor++];
        attempts++;
        if (!staleAgency(next, Date.now())) probe(next);
      }
      render();
    }
    function retry(feed) {
      if (!connected || !feed.availableSource) return;
      probe(feed); // explicit operator action, not background polling
      render();
    }
    function recheck() {
      if (!connected) return;
      for (const state of statuses.values()) drop(state);
      statuses.clear();
      cursor = 0; attempts = 0; inFlight = 0;
      ensureProbes();
    }
    // Rendering the view is the operator's network intent. Nothing is fetched
    // on app boot, role preview, or while the camera tool is hidden.
    render();
    ensureProbes();
    return {destroy() {
      if (!connected) return;
      connected = false;
      for (const state of statuses.values()) drop(state);
      statuses.clear();
      root.replaceChildren();
    }};
  }
  window.RustBoyCameras = Object.freeze({mount, rankFeeds, pickNearestActive, distanceMiles});
})();
