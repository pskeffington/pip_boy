/* Rust-Boy local map viewport.
 * WGS84 GeoJSON (RFC 7946) -> Web Mercator screen coordinates (EPSG:3857).
 * No third-party scripts, tiles, network calls, location persistence or background GPS.
 * RustGIS layers are imported explicitly as GeoJSON; this is not its native renderer.
 */
(function () {
  "use strict";

  const TILE = 256;
  const MAX_LAT = 85.05112878;
  const MIN_ZOOM = 1;
  const MAX_ZOOM = 19;
  const MAX_FILE_BYTES = 2 * 1024 * 1024;
  const MAX_FEATURES = 2000;
  const MAX_VERTICES = 30000;
  const EARTH_RADIUS_M = 6378137;
  let active = null;
  let mapMemory = null; // volatile geographic features only; never store GPS fixes
  let webRequestCount = 0;
  let webNextRequestAt = 0;
  let webProbeAt = 0;
  let webProbeSuccess = null;

  function clamp(value, lower, upper) {
    return Math.min(upper, Math.max(lower, value));
  }

  function wrapLongitude(value) {
    return ((value + 180) % 360 + 360) % 360 - 180;
  }

  function project(lon, lat) {
    const phi = clamp(lat, -MAX_LAT, MAX_LAT) * Math.PI / 180;
    return [(lon + 180) / 360 * TILE,
      (1 - Math.asinh(Math.tan(phi)) / Math.PI) / 2 * TILE];
  }

  function inverse(x, y) {
    const latitude = Math.atan(Math.sinh(Math.PI * (1 - 2 * clamp(y, 0, TILE) / TILE))) * 180 / Math.PI;
    return { lon: wrapLongitude(x / TILE * 360 - 180), lat: latitude };
  }

  function coordinate(value) {
    if (!Array.isArray(value) || value.length < 2 ||
        typeof value[0] !== "number" || typeof value[1] !== "number" ||
        !Number.isFinite(value[0]) || !Number.isFinite(value[1]) ||
        value[0] < -180 || value[0] > 180 || value[1] < -90 || value[1] > 90) {
      throw new Error("Layer needs valid WGS84 [longitude, latitude] coordinates");
    }
    return [value[0], value[1]];
  }

  function parseGeoJSON(text) {
    const source = JSON.parse(text);
    if (!source || typeof source !== "object" || source.crs) {
      throw new Error("Export RustGIS layers as WGS84 GeoJSON (EPSG:4326)");
    }
    const shapes = [];
    let vertices = 0;
    let features = 0;

    function path(values, minLength) {
      if (!Array.isArray(values) || values.length < minLength) {
        throw new Error("Invalid or empty GeoJSON path");
      }
      vertices += values.length;
      if (vertices > MAX_VERTICES) throw new Error("Layer exceeds 30,000 vertices");
      return values.map(coordinate);
    }

    function geometry(item, label, depth) {
      if (depth > 6 || !item || typeof item !== "object") {
        throw new Error("Unsupported GeoJSON geometry");
      }
      const c = item.coordinates;
      switch (item.type) {
        case "Point":
          shapes.push({ kind: "point", paths: [[coordinate(c)]], label });
          vertices++;
          break;
        case "MultiPoint":
          shapes.push({ kind: "point", paths: [path(c, 1)], label });
          break;
        case "LineString":
          shapes.push({ kind: "line", paths: [path(c, 2)], label });
          break;
        case "MultiLineString":
          if (!Array.isArray(c) || !c.length) throw new Error("Empty MultiLineString");
          shapes.push({ kind: "line", paths: c.map((part) => path(part, 2)), label });
          break;
        case "Polygon":
          if (!Array.isArray(c) || !c.length) throw new Error("Empty Polygon");
          shapes.push({ kind: "polygon", paths: c.map((ring) => path(ring, 4)), label });
          break;
        case "MultiPolygon":
          if (!Array.isArray(c) || !c.length) throw new Error("Empty MultiPolygon");
          for (const polygon of c) {
            if (!Array.isArray(polygon) || !polygon.length) throw new Error("Empty polygon part");
            shapes.push({ kind: "polygon", paths: polygon.map((ring) => path(ring, 4)), label });
          }
          break;
        case "GeometryCollection":
          if (!Array.isArray(item.geometries) || !item.geometries.length) {
            throw new Error("Empty GeometryCollection");
          }
          item.geometries.forEach((child) => geometry(child, label, depth + 1));
          break;
        default:
          throw new Error("Unsupported geometry: " + String(item.type).slice(0, 30));
      }
      if (vertices > MAX_VERTICES) throw new Error("Layer exceeds 30,000 vertices");
    }

    function feature(item) {
      if (!item || item.type !== "Feature") throw new Error("Expected GeoJSON Feature");
      features++;
      if (features > MAX_FEATURES) throw new Error("Layer exceeds 2,000 features");
      const raw = item.properties && item.properties.name;
      const label = typeof raw === "string" ? raw.slice(0, 36) : "";
      geometry(item.geometry, label, 0);
    }

    if (source.type === "FeatureCollection") {
      if (!Array.isArray(source.features) || !source.features.length ||
          source.features.length > MAX_FEATURES) {
        throw new Error("FeatureCollection must contain 1 to 2,000 features");
      }
      source.features.forEach(feature);
    } else if (source.type === "Feature") {
      feature(source);
    } else {
      geometry(source, "", 0);
    }
    if (!vertices || !shapes.length) throw new Error("No drawable GeoJSON geometry");
    return { shapes, vertices, features };
  }

  function niceStep(value) {
    const v = Math.max(1e-6, value);
    const power = Math.pow(10, Math.floor(Math.log10(v)));
    const normalized = v / power;
    return (normalized <= 1 ? 1 : normalized <= 2 ? 2 : normalized <= 5 ? 5 : 10) * power;
  }

  function screenPoint(view, lon, lat) {
    const p = project(lon, lat);
    const c = project(view.center.lon, view.center.lat);
    let dx = p[0] - c[0];
    dx -= Math.round(dx / TILE) * TILE;
    const scale = Math.pow(2, view.zoom);
    return [view.width / 2 + dx * scale, view.height / 2 + (p[1] - c[1]) * scale];
  }

  function drawGrid(ctx, view) {
    const scale = Math.pow(2, view.zoom);
    const center = project(view.center.lon, view.center.lat);
    const minLon = (center[0] - view.width / (2 * scale)) / TILE * 360 - 180;
    const maxLon = (center[0] + view.width / (2 * scale)) / TILE * 360 - 180;
    const north = inverse(center[0], center[1] - view.height / (2 * scale)).lat;
    const south = inverse(center[0], center[1] + view.height / (2 * scale)).lat;
    const lonStep = niceStep((maxLon - minLon) / Math.max(2, view.width / 85));
    const latStep = niceStep((north - south) / Math.max(2, view.height / 85));
    ctx.lineWidth = 1;
    ctx.strokeStyle = "rgba(111, 174, 113, 0.19)";
    ctx.fillStyle = "rgba(161, 219, 147, 0.55)";
    ctx.font = "10px monospace";
    const startLon = Math.ceil(minLon / lonStep) * lonStep;
    for (let index = 0; index < 60; index++) {
      const lon = startLon + index * lonStep;
      if (lon > maxLon + 1e-8) break;
      const x = view.width / 2 + (lon - view.center.lon) * TILE * scale / 360;
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, view.height);
      ctx.stroke();
      ctx.fillText(wrapLongitude(lon).toFixed(lonStep < 1 ? 3 : 1) + "\u00b0", x + 3, 12);
    }
    const startLat = Math.ceil(south / latStep) * latStep;
    for (let index = 0; index < 60; index++) {
      const lat = startLat + index * latStep;
      if (lat > north + 1e-8) break;
      const y = screenPoint(view, view.center.lon, lat)[1];
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(view.width, y);
      ctx.stroke();
      ctx.fillText(lat.toFixed(latStep < 1 ? 3 : 1) + "\u00b0", 4, y - 4);
    }
  }

  function draw(view) {
    const ctx = view.canvas.getContext("2d");
    if (!ctx || !view.width || !view.height) return;
    ctx.setTransform(view.dpr, 0, 0, view.dpr, 0, 0);
    ctx.clearRect(0, 0, view.width, view.height);
    ctx.fillStyle = "#0b1a12";
    ctx.fillRect(0, 0, view.width, view.height);
    drawGrid(ctx, view);
    ctx.lineJoin = "round";
    ctx.lineCap = "round";
    let labels = 0;
    // Reference OSM geometry sits under authoritative local RustGIS layers.
    for (const shape of [...view.onlineShapes, ...view.shapes]) {
      const style = shape.style || {};
      ctx.globalAlpha = Number.isFinite(style.opacity) ? style.opacity : 1;
      if (shape.kind === "point") {
        ctx.fillStyle = style.fill || "#bced99";
        for (const point of shape.paths[0]) {
          const p = screenPoint(view, point[0], point[1]);
          if (p[0] < -8 || p[0] > view.width + 8 || p[1] < -8 || p[1] > view.height + 8) continue;
          ctx.beginPath();
          ctx.arc(p[0], p[1], 3, 0, Math.PI * 2);
          ctx.fill();
          if (shape.label && labels < 12) {
            ctx.font = "10px monospace";
            ctx.fillText(shape.label, p[0] + 6, p[1] - 6);
            labels++;
          }
        }
      } else {
        ctx.beginPath();
        for (const path of shape.paths) {
          path.forEach((point, index) => {
            const p = screenPoint(view, point[0], point[1]);
            if (index === 0) ctx.moveTo(p[0], p[1]);
            else ctx.lineTo(p[0], p[1]);
          });
          if (shape.kind === "polygon") ctx.closePath();
        }
        if (shape.kind === "polygon") {
          ctx.fillStyle = style.fill || "rgba(116, 178, 112, 0.19)";
          ctx.fill("evenodd");
        }
        ctx.lineWidth = style.width || (shape.kind === "polygon" ? 1 : 1.5);
        ctx.strokeStyle = style.stroke || (shape.kind === "polygon" ? "#6b9f6b" : "#b5db90");
        ctx.stroke();
      }
    }

    ctx.globalAlpha = 1;
    // Center crosshair is a camera reference, never passed off as a GPS fix.
    const midX = view.width / 2;
    const midY = view.height / 2;
    ctx.strokeStyle = "rgba(190, 224, 162, 0.45)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(midX - 6, midY);
    ctx.lineTo(midX + 6, midY);
    ctx.moveTo(midX, midY - 6);
    ctx.lineTo(midX, midY + 6);
    ctx.stroke();

    if (view.fix) {
      const point = screenPoint(view, view.fix.lon, view.fix.lat);
      const metersPerPixel = Math.cos(view.fix.lat * Math.PI / 180) *
        2 * Math.PI * EARTH_RADIUS_M / (TILE * Math.pow(2, view.zoom));
      if (view.fix.accuracy > 0 && metersPerPixel > 0) {
        ctx.beginPath();
        ctx.arc(point[0], point[1], Math.min(30000, view.fix.accuracy / metersPerPixel), 0, Math.PI * 2);
        ctx.fillStyle = "rgba(121, 204, 154, 0.13)";
        ctx.fill();
        ctx.strokeStyle = "rgba(126, 229, 159, 0.65)";
        ctx.stroke();
      }
      ctx.beginPath();
      ctx.arc(point[0], point[1], 6, 0, Math.PI * 2);
      ctx.fillStyle = "#e5ffb2";
      ctx.fill();
      ctx.lineWidth = 2;
      ctx.strokeStyle = "#153b21";
      ctx.stroke();
    }

    if (!view.shapes.length && !view.onlineShapes.length) {
      ctx.textAlign = "center";
      ctx.font = "11px monospace";
      ctx.fillStyle = "#dfa96b";
      ctx.fillText("LOAD RUSTGIS MAP / NO GEOGRAPHY", view.width / 2, view.height - 16);
      ctx.textAlign = "start";
    }
  }

  function setStatus(view) {
    if (active !== view) return;
    const center = view.center.lat.toFixed(5) + ", " + view.center.lon.toFixed(5);
    const fix = view.fix
      ? "FIX " + view.fix.lat.toFixed(5) + ", " + view.fix.lon.toFixed(5) +
        " \u00b1" + Math.round(view.fix.accuracy) + "m"
      : view.gpsState;
    view.status.textContent = fix + "  |  Z" + view.zoom.toFixed(1) + "  |  " +
      (view.layerName || "LOAD RUSTGIS MAP") + "  |  CENTER " + center;
    view.locate.textContent = view.watchId === null ? "GPS ON" : "GPS OFF";
    view.followButton.disabled = !view.fix;
    view.followButton.textContent = view.follow ? "FOLLOW" : "RECENTER";
    view.webButton.textContent = view.webEnabled ? "WEB OFF" : "WEB ON";
    view.webRefresh.hidden = !view.webEnabled;
    view.webStatusNode.textContent = view.webStatus;
    view.attribution.hidden = !view.onlineShapes.length;
    view.sourceBox.hidden = !view.layers.length && !view.webMeta;
    if (view.layers.length || view.webMeta) {
      view.sourceSummary.textContent = "DATA SOURCES / " + view.layers.length + " NATIVE LAYERS" +
        (view.webMeta ? " / OSM WEB" : "") +
        (view.warnings.length ? " / " + view.warnings.length + " LIMITATIONS" : "");
      view.sourceLines.textContent = view.mapName + " / " + view.mapFormat + "\n" +
        view.layers.map((layer) => layer.name + " (" + layer.features + " features)\n" +
          "  Source: " + layer.source +
          (layer.originalCrs ? "\n  Original CRS: " + JSON.stringify(layer.originalCrs).slice(0, 120) : "")
        ).join("\n") +
        (view.warnings.length ? "\nLIMITS: " + view.warnings.join(" | ") : "") +
        (view.webMeta
          ? "\nREFERENCE ONLY: © OpenStreetMap contributors / ODbL" +
            "\nProvider: overpass-api.de (RustGIS OSM query source)" +
            "\nCoverage: " + view.webMeta.bounds.join(", ") +
            "\nFetched: " + new Date(view.webMeta.fetchedAt).toISOString() +
            (view.webMeta.timestamp ? "\nOSM base timestamp: " + view.webMeta.timestamp : "") +
            "\nFeatures: " + Object.entries(view.webMeta.count).map(([k,v])=>k+"="+v).join(", ") +
            (view.webMeta.warnings.length ? "\nWEB LIMITS: " + view.webMeta.warnings.join(" | ") : "")
          : "") +
        "\nCoordinates drawn: WGS84 longitude/latitude.";
    }
  }

  function refresh(view) {
    if (active !== view) return;
    setStatus(view);
    draw(view);
  }



  // The only automatic outbound request: an infrequent, coordinate-free status
  // ping upon entering MAP. Actual bbox queries require WEB ON.
  async function probeWeb(view) {
    if (active !== view || view.webEnabled || view.webProbePromise) return;
    if (navigator.onLine === false) {
      view.webStatus = "OFFLINE / WEB MAP UNAVAILABLE";
      refresh(view);
      return;
    }
    if (Date.now() - webProbeAt < 10 * 60 * 1000 && webProbeSuccess !== null) {
      view.webStatus = webProbeSuccess ? "WEB AVAILABLE / TAP WEB ON" : "WEB STATUS UNAVAILABLE";
      refresh(view);
      return;
    }
    const service = window.RustBoyOnline;
    if (!service) return;
    const controller = new AbortController();
    view.webProbeAbort = controller;
    const timeout = setTimeout(() => controller.abort(), 5000);
    view.webStatus = "PING WEB STATUS / NO COORDINATES SENT";
    refresh(view);
    const pending = (async () => {
      try {
        await service.ping(controller.signal);
        webProbeSuccess = true;
        webProbeAt = Date.now();
        if (active === view && !view.webEnabled) {
          view.webStatus = "WEB AVAILABLE / TAP WEB ON";
        }
      } catch (_) {
        if (!controller.signal.aborted || active === view) {
          webProbeSuccess = false;
          webProbeAt = Date.now();
        }
        if (active === view && !view.webEnabled) {
          view.webStatus = "WEB STATUS UNAVAILABLE / OFFLINE MAP READY";
        }
      } finally {
        clearTimeout(timeout);
        if (view.webProbeAbort === controller) view.webProbeAbort = null;
        view.webProbePromise = null;
        if (active === view && !view.webEnabled) refresh(view);
      }
    })();
    view.webProbePromise = pending;
    await pending;
  }

  function centerInside(bounds, center) {
    if (!bounds || !center) return false;
    const dx = (bounds[2] - bounds[0]) * 0.23;
    const dy = (bounds[3] - bounds[1]) * 0.23;
    return center.lon >= bounds[0] + dx && center.lon <= bounds[2] - dx &&
      center.lat >= bounds[1] + dy && center.lat <= bounds[3] - dy;
  }

  function webOff(view, preserveCache = true) {
    view.webEnabled = false;
    if (view.webAbort) view.webAbort.abort();
    view.webStatus = preserveCache && view.onlineShapes.length
      ? "WEB OFF / CACHED ROAD REFERENCE" : "WEB OFF";
    if (!preserveCache) {
      view.onlineShapes = [];
      view.webMeta = null;
    }
    refresh(view);
  }

  async function webFetch(view) {
    if (active !== view || !view.webEnabled || view.webBusy) return;
    const service = window.RustBoyOnline;
    if (!service) { view.webStatus = "WEB CONNECTOR MISSING"; refresh(view); return; }
    if (navigator.onLine === false) {
      view.webStatus = "OFFLINE / EXISTING MAP PRESERVED";
      refresh(view);
      return;
    }
    let bbox;
    try {
      bbox = service.boundsFor(view);
    } catch (error) {
      view.webStatus = error instanceof Error ? error.message : "INVALID WEB MAP AREA";
      refresh(view);
      return;
    }
    const now = Date.now();
    if (view.webMeta && now - view.webMeta.fetchedAt < 10 * 60 * 1000 &&
        centerInside(view.webMeta.bounds, view.center)) {
      view.webStatus = "OSM CACHE / " + view.onlineShapes.length + " FEATURES";
      refresh(view);
      return;
    }
    if (now < webNextRequestAt) {
      view.webStatus = "WEB COOLDOWN / MAP PRESERVED";
      refresh(view);
      return;
    }
    if (webRequestCount >= 8) {
      view.webStatus = "SESSION WEB LIMIT / RESTART TO RESET";
      refresh(view);
      return;
    }
    view.webBusy = true;
    webRequestCount++;
    webNextRequestAt = now + 2 * 60 * 1000;
    const abort = new AbortController();
    view.webAbort = abort;
    const timeout = setTimeout(() => abort.abort(), 25000);
    try {
      view.webStatus = "PING OVERPASS / NO LOCATION IN PING";
      refresh(view);
      if (view.webProbePromise) await view.webProbePromise;
      if (active !== view || !view.webEnabled || abort.signal.aborted) return;
      if (!webProbeSuccess || Date.now() - webProbeAt >= 10 * 60 * 1000) {
        await service.ping(abort.signal);
        webProbeSuccess = true;
        webProbeAt = Date.now();
      }
      if (active !== view || !view.webEnabled || abort.signal.aborted) return;
      view.webStatus = "FETCH ROADS + LANDMARKS / OSM";
      refresh(view);
      const result = await service.roads(view, abort.signal);
      if (active !== view || !view.webEnabled || abort.signal.aborted) return;
      if (!result.shapes.length) {
        view.webStatus = "NO OSM FEATURES IN AREA / LOCAL MAP PRESERVED";
      } else {
        // Atomic replacement: a failed fetch never discards the prior offline cache.
        view.onlineShapes = result.shapes;
        view.webMeta = {
          bounds: result.bounds, count: result.count, timestamp: result.timestamp,
          warnings: result.warnings, fetchedAt: Date.now()
        };
        view.webStatus = result.warnings.length
          ? "OSM PARTIAL / CHECK DATA SOURCES"
          : "OSM REFERENCE READY / " + result.shapes.length;
      }
    } catch (error) {
      if (active !== view || !view.webEnabled) return;
      const msg = error instanceof Error ? error.message : "OVERPASS UNAVAILABLE";
      if (/BUSY|429|503|504/.test(msg)) webNextRequestAt = Date.now() + 5 * 60 * 1000;
      view.webStatus = abort.signal.aborted ? "WEB TIMEOUT / MAP PRESERVED"
        : msg.slice(0, 85) + " / MAP PRESERVED";
    } finally {
      clearTimeout(timeout);
      if (view.webAbort === abort) view.webAbort = null;
      view.webBusy = false;
      if (active === view) refresh(view);
    }
  }

  function webToggle(view) {
    if (view.webEnabled) {
      webOff(view);
    } else {
      view.webEnabled = true; // Explicit consent is never persisted across tabs.
      void webFetch(view);
    }
  }

  function fitLayer(view) {
    const points = view.shapes.flatMap((shape) => shape.paths.flat());
    const anchor = points[0][0];
    let left = Infinity, right = -Infinity, top = Infinity, bottom = -Infinity;
    for (const point of points) {
      let offset = point[0] - anchor;
      offset -= Math.round(offset / 360) * 360;
      const projected = project(anchor + offset, point[1]);
      left = Math.min(left, projected[0]);
      right = Math.max(right, projected[0]);
      top = Math.min(top, projected[1]);
      bottom = Math.max(bottom, projected[1]);
    }
    const width = Math.max((right - left), 1e-5) * TILE / TILE;
    const height = Math.max((bottom - top), 1e-5) * TILE / TILE;
    const usableWidth = Math.max(80, view.width - 50);
    const usableHeight = Math.max(60, view.height - 45);
    view.zoom = clamp(Math.floor(Math.log2(Math.min(usableWidth / width,
      usableHeight / height))), MIN_ZOOM, 17);
    view.center = inverse((left + right) / 2, (top + bottom) / 2);
    view.follow = false;
  }

  function stopGps(view, status) {
    if (view.watchId !== null && navigator.geolocation) {
      navigator.geolocation.clearWatch(view.watchId);
    }
    view.watchId = null;
    view.fix = null;
    view.gpsState = status || "GPS OFF";
    refresh(view);
  }

  function startGps(view) {
    if (!window.isSecureContext || !navigator.geolocation) {
      view.gpsState = "BROWSER LOCATION UNAVAILABLE";
      refresh(view);
      return;
    }
    view.follow = true;
    view.gpsState = "WAITING FOR LOCATION";
    try {
      view.watchId = navigator.geolocation.watchPosition((position) => {
        if (active !== view) return;
        const c = position.coords;
        const timestamp = position.timestamp;
        if (!c || !Number.isFinite(c.latitude) || !Number.isFinite(c.longitude) ||
            Math.abs(c.latitude) > 90 || Math.abs(c.longitude) > 180 ||
            !Number.isFinite(timestamp) || Date.now() - timestamp > 60000 ||
            (view.fix && timestamp < view.fix.timestamp)) {
          view.gpsState = "INVALID OR STALE FIX";
          refresh(view);
          return;
        }
        view.fix = {
          lat: c.latitude, lon: c.longitude, accuracy:
            Number.isFinite(c.accuracy) && c.accuracy >= 0 ? c.accuracy : 0,
          timestamp
        };
        view.gpsState = "BROWSER LOCATION";
        if (view.follow) {
          view.center = { lat: c.latitude, lon: c.longitude };
          view.zoom = Math.max(view.zoom, 14);
        }
        refresh(view);
        if (view.webEnabled && (!view.webMeta ||
            !centerInside(view.webMeta.bounds, view.center))) {
          void webFetch(view);
        }
      }, (error) => {
        if (active !== view) return;
        const message = error && error.code === 1 ? "LOCATION DENIED" :
          error && error.code === 3 ? "LOCATION TIMEOUT" : "LOCATION UNAVAILABLE";
        stopGps(view, message);
      }, { enableHighAccuracy: true, maximumAge: 0, timeout: 12000 });
      refresh(view);
    } catch (_) {
      stopGps(view, "LOCATION UNAVAILABLE");
    }
  }

  function resize(view) {
    if (active !== view) return;
    const rect = view.stage.getBoundingClientRect();
    view.width = Math.max(1, Math.round(rect.width));
    view.height = Math.max(1, Math.round(rect.height));
    view.dpr = Math.min(2, window.devicePixelRatio || 1);
    view.canvas.width = Math.round(view.width * view.dpr);
    view.canvas.height = Math.round(view.height * view.dpr);
    refresh(view);
  }

  function make(tag, className, label) {
    const node = document.createElement(tag);
    node.className = className;
    if (label !== undefined) node.textContent = label;
    return node;
  }

  function control(text, action, title) {
    const node = make("button", "rb-map-button", text);
    node.type = "button";
    node.title = title || text;
    node.setAttribute("aria-label", title || text);
    node.addEventListener("click", action);
    return node;
  }

  function unmount() {
    if (!active) return;
    const view = active;
    if (view.watchId !== null && navigator.geolocation) navigator.geolocation.clearWatch(view.watchId);
    if (view.observer) view.observer.disconnect();
    if (view.windowResize) window.removeEventListener("resize", view.windowResize);
    if (view.webReconnect) window.removeEventListener("online", view.webReconnect);
    if (view.webDisconnect) window.removeEventListener("offline", view.webDisconnect);
    if (view.webAbort) view.webAbort.abort();
    if (view.webProbeAbort) view.webProbeAbort.abort();
    mapMemory = (view.shapes.length || view.onlineShapes.length) ? {
      center: { lat: view.center.lat, lon: view.center.lon }, zoom: view.zoom,
      shapes: view.shapes, layers: view.layers, warnings: view.warnings,
      layerName: view.layerName, mapName: view.mapName, mapFormat: view.mapFormat,
      onlineShapes: view.onlineShapes, webMeta: view.webMeta
    } : null;
    active = null;
    if (view.host.isConnected) view.host.replaceChildren();
  }

  function mount(host, options) {
    unmount();
    if (!host) return;
    const root = make("section", "rb-map");
    root.setAttribute("aria-label", "Local WGS84 map with imported RustGIS GeoJSON layers");
    const toolbar = make("div", "rb-map-toolbar");
    const stage = make("div", "rb-map-stage");
    const canvas = make("canvas", "rb-map-canvas");
    canvas.setAttribute("role", "img");
    canvas.setAttribute("aria-label", "Georeferenced WGS84 map. Pan with a finger or pointer; use zoom buttons.");
    stage.append(canvas);
    const status = make("div", "rb-map-status");
    status.setAttribute("role", "status");
    status.setAttribute("aria-live", "polite");
    const file = make("input", "");
    file.type = "file";
    file.accept = ".rgismap,.rgis,.rgislyr,.rgismpk,.rgispkg,.geojson,.json,application/geo+json";
    file.hidden = true;
    const attribution = make("a", "rb-map-attribution",
      "© OpenStreetMap contributors · ODbL");
    attribution.href = "https://www.openstreetmap.org/copyright";
    attribution.target = "_blank";
    attribution.rel = "noopener noreferrer";
    attribution.hidden = true;
    stage.append(attribution);
    const webStatusNode = make("div", "rb-map-web-state", "WEB OFF");
    webStatusNode.setAttribute("role", "status");
    const sourceBox = make("details", "rb-map-sources");
    sourceBox.hidden = true;
    const sourceSummary = make("summary", "", "DATA SOURCES");
    const sourceLines = make("pre", "rb-map-source-text");
    const clearMap = control("CLEAR MAP", () => {
      mapMemory = null;
      webOff(view, false);
      view.shapes = [];
      view.layers = [];
      view.warnings = [];
      view.layerName = "";
      view.mapName = "";
      view.mapFormat = "";
      view.gpsState = "MAP DATA CLEARED";
      refresh(view);
    }, "Discard imported geography from this browser session");
    sourceBox.append(sourceSummary, sourceLines, clearMap);
    const manual = make("form", "rb-map-manual");
    manual.hidden = true;
    const latitude = make("input", "", "");
    latitude.type = "number";
    latitude.step = "any";
    latitude.min = "-90";
    latitude.max = "90";
    latitude.required = true;
    latitude.placeholder = "Latitude";
    latitude.setAttribute("aria-label", "Manual center latitude");
    const longitude = make("input", "", "");
    longitude.type = "number";
    longitude.step = "any";
    longitude.min = "-180";
    longitude.max = "180";
    longitude.required = true;
    longitude.placeholder = "Longitude";
    longitude.setAttribute("aria-label", "Manual center longitude");
    const apply = make("button", "rb-map-button", "SET CENTER");
    apply.type = "submit";
    manual.append(latitude, longitude, apply);
    const view = {
      host, root, stage, canvas, status, file, sourceBox, sourceSummary, sourceLines,
      manual, latitude, longitude,
      width: 0, height: 0, dpr: 1,
      center: mapMemory ? { ...mapMemory.center } : { lat: 0, lon: 0 },
      zoom: mapMemory ? mapMemory.zoom : 2,
      shapes: mapMemory ? mapMemory.shapes : [],
      layers: mapMemory ? mapMemory.layers : [],
      warnings: mapMemory ? mapMemory.warnings : [],
      layerName: mapMemory ? mapMemory.layerName : "",
      mapName: mapMemory ? mapMemory.mapName : "",
      mapFormat: mapMemory ? mapMemory.mapFormat : "",
      onlineShapes: mapMemory ? mapMemory.onlineShapes || [] : [],
      webMeta: mapMemory ? mapMemory.webMeta || null : null,
      webEnabled: false, webBusy: false, webAbort: null,
      webProbeAbort: null, webProbePromise: null,
      webReconnect: null, webDisconnect: null, webStatus: "WEB OFF",
      attribution, webStatusNode, webButton: null, webRefresh: null,
      fix: null, watchId: null, gpsState: "GPS OFF",
      follow: !mapMemory, observer: null, windowResize: null, drag: null,
      locate: null, followButton: null
    };
    active = view;
    view.locate = control("GPS ON", () => {
      if (view.watchId === null) startGps(view);
      else stopGps(view);
    }, "Enable or stop browser location with permission");
    view.followButton = control("FOLLOW", () => {
      view.follow = true;
      if (view.fix) view.center = { lat: view.fix.lat, lon: view.fix.lon };
      refresh(view);
    }, "Center map on latest location fix");
    view.webButton = control("WEB ON", () => webToggle(view),
      "Enable online road/landmark retrieval; sends map area to Overpass (OSM), not official GIS");
    view.webRefresh = control("SYNC", () => void webFetch(view),
      "Check online access and refresh nearby OSM road reference (rate limited)");
    view.webRefresh.hidden = true;
    const importButton = control("RUSTGIS", () => file.click(),
      "Import native RustGIS map/project/mobile package or GeoJSON");
    const manualButton = control("CENTER", () => {
      manual.hidden = !manual.hidden;
      if (!manual.hidden) latitude.focus();
    }, "Enter map center manually; not a GPS fix");
    const zoomIn = control("+", () => { view.zoom = clamp(view.zoom + 1, MIN_ZOOM, MAX_ZOOM); refresh(view); },
      "Zoom in");
    const zoomOut = control("-", () => { view.zoom = clamp(view.zoom - 1, MIN_ZOOM, MAX_ZOOM); refresh(view); },
      "Zoom out");
    toolbar.append(view.locate, view.followButton, importButton, view.webButton,
      view.webRefresh, manualButton, zoomIn, zoomOut);
    if (options && typeof options.onTools === "function") {
      toolbar.append(control("GIS", options.onTools, "Open Scout GIS tools"));
    }
    root.append(toolbar, stage, webStatusNode, status, sourceBox, manual, file);
    host.replaceChildren(root);

    manual.addEventListener("submit", (event) => {
      event.preventDefault();
      const lat = Number(latitude.value), lon = Number(longitude.value);
      if (!latitude.value || !longitude.value || !Number.isFinite(lat) || !Number.isFinite(lon) ||
          Math.abs(lat) > 90 || Math.abs(lon) > 180) {
        view.gpsState = "INVALID MANUAL CENTER";
      } else {
        view.center = { lat, lon };
        view.follow = false;
        view.zoom = Math.max(view.zoom, 13);
        view.gpsState = "MANUAL CENTER / NOT A FIX";
        manual.hidden = true;
      }
      refresh(view);
      if (view.webEnabled) void webFetch(view);
    });

    file.addEventListener("change", async () => {
      const selected = file.files && file.files[0];
      if (!selected) return;
      try {
        const importer = window.RustBoyMapImport;
        const native = importer && importer.isRustGIS(selected.name);
        let parsed;
        if (native) {
          parsed = await importer.readFile(selected);
        } else {
          if (selected.size > MAX_FILE_BYTES) throw new Error("GeoJSON exceeds 2 MiB");
          const text = await selected.text();
          if (new TextEncoder().encode(text).length > MAX_FILE_BYTES) {
            throw new Error("GeoJSON exceeds 2 MiB");
          }
          parsed = parseGeoJSON(text);
        }
        if (active !== view) return;
        view.shapes = parsed.shapes;
        view.layers = native ? parsed.layers : [{
          name: selected.name.slice(0, 60), source: "User-selected GeoJSON file",
          features: parsed.features, originalCrs: "WGS84"
        }];
        view.warnings = native ? parsed.warnings : [];
        view.mapName = native ? parsed.mapName : selected.name.slice(0, 50);
        view.mapFormat = native ? parsed.format : "GeoJSON";
        view.layerName = view.mapName + " (" + parsed.features + " features)";
        // A current GPS follow fix is authoritative for the viewport center.
        if (view.fix && view.follow) {
          view.center = { lat: view.fix.lat, lon: view.fix.lon };
          view.zoom = Math.max(view.zoom, 14);
        } else if (native && parsed.camera) {
          view.center = { lat: parsed.camera.lat, lon: parsed.camera.lon };
          view.zoom = parsed.camera.zoom;
          view.follow = false;
        } else {
          fitLayer(view);
        }
        view.gpsState = native && parsed.warnings.length ?
          "RUSTGIS VECTOR PARTIAL / VIEW SOURCES" : "LOCAL MAP LOADED";
        refresh(view);
        if (view.webEnabled) void webFetch(view);
      } catch (error) {
        if (active === view) {
          view.gpsState = "MAP ERROR: " + (error instanceof Error ? error.message : "Invalid file");
          refresh(view);
        }
      } finally {
        file.value = "";
      }
    });

    canvas.addEventListener("pointerdown", (event) => {
      if (event.button !== 0 && event.pointerType === "mouse") return;
      view.drag = { x: event.clientX, y: event.clientY, pointer: event.pointerId };
      canvas.setPointerCapture(event.pointerId);
    });
    canvas.addEventListener("pointermove", (event) => {
      if (!view.drag || view.drag.pointer !== event.pointerId) return;
      const dx = event.clientX - view.drag.x;
      const dy = event.clientY - view.drag.y;
      view.drag.x = event.clientX;
      view.drag.y = event.clientY;
      const center = project(view.center.lon, view.center.lat);
      view.center = inverse(center[0] - dx / Math.pow(2, view.zoom),
        center[1] - dy / Math.pow(2, view.zoom));
      view.follow = false;
      refresh(view);
    });
    const endDrag = () => { view.drag = null; };
    canvas.addEventListener("pointerup", endDrag);
    canvas.addEventListener("pointercancel", endDrag);
    canvas.addEventListener("lostpointercapture", endDrag);
    canvas.addEventListener("wheel", (event) => {
      event.preventDefault();
      view.zoom = clamp(view.zoom + (event.deltaY < 0 ? 0.5 : -0.5), MIN_ZOOM, MAX_ZOOM);
      refresh(view);
    }, { passive: false });

    view.webReconnect = () => {
      if (active !== view) return;
      if (view.webEnabled) void webFetch(view);
      else void probeWeb(view);
    };
    view.webDisconnect = () => {
      if (active === view && view.webEnabled) {
        if (view.webAbort) view.webAbort.abort();
        view.webStatus = "OFFLINE / CACHED REFERENCE KEPT";
        refresh(view);
      }
    };
    window.addEventListener("online", view.webReconnect);
    window.addEventListener("offline", view.webDisconnect);

    if ("ResizeObserver" in window) {
      view.observer = new ResizeObserver(() => resize(view));
      view.observer.observe(stage);
    } else {
      view.windowResize = () => resize(view);
      window.addEventListener("resize", view.windowResize);
    }
    window.requestAnimationFrame(() => {
      resize(view);
      void probeWeb(view);
    });
    return view;
  }

  // Public surface is intentionally small. Import and GNSS data stay in memory.
  window.RustBoyMap = Object.freeze({ mount, unmount });
})();
