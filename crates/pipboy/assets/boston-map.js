/* Rust-Boy Boston offline vector source loader.
 * Geometry is decoded only from pinned public City of Boston SAM street records.
 * This module does not synthesize features, geocode names, or request location.
 */
(function () {
  "use strict";
  const SOURCE_SHA = "a86dc5f6913987585e429e625bdc61781d03d40e";
  const REGION = [-71.14, 42.309, -70.99, 42.399];
  const MAX_ROADS = 12000;
  let cache = null;

  function validRecord(record, names) {
    if (!Array.isArray(record) || record.length < 7 ||
        record.length % 2 !== 1 ||
        !Number.isSafeInteger(record[0]) || record[0] <= 0 ||
        !Number.isInteger(record[1]) || record[1] < 0 || record[1] > 99 ||
        !Number.isInteger(record[2]) || record[2] < 0 || record[2] >= names.length) {
      throw new Error("Invalid Boston geometry record");
    }
    const points = [];
    let within = false;
    for (let i = 3; i < record.length; i += 2) {
      const x = record[i], y = record[i + 1];
      if (!Number.isSafeInteger(x) || !Number.isSafeInteger(y)) {
        throw new Error("Boston road contains non-integer WGS84 coordinates");
      }
      const lon = -71 + x / 1000000;
      const lat = 42 + y / 1000000;
      if (lon < -71.20 || lon > -70.94 || lat < 42.27 || lat > 42.43) {
        throw new Error("Boston road outside verified extract extent");
      }
      within ||= lon >= REGION[0] && lon <= REGION[2] &&
        lat >= REGION[1] && lat <= REGION[3];
      points.push([lon, lat]);
    }
    if (!within) throw new Error("Boston road has no vertex within selected source extent");
    return points;
  }

  function load() {
    if (cache) return cache;
    const data = window.RustBoyBostonBase;
    if (!data || data.schema !== 1 ||
        data.source?.source_blob_sha !== SOURCE_SHA ||
        JSON.stringify(data.bounds) !== JSON.stringify(REGION) ||
        !Array.isArray(data.names) || !Array.isArray(data.roads) ||
        data.roads.length !== 10634 || data.roads.length > MAX_ROADS ||
        data.names.length !== 2522 ||
        !data.names.every((name) => typeof name === "string" &&
          name.length <= 64 && !/[\x00-\x1f\x7f]/.test(name))) {
      throw new Error("Pinned Boston source data missing or invalid");
    }

    const ids = new Set();
    const shapes = [];
    let vertices = 0;
    for (const record of data.roads) {
      const coordinates = validRecord(record, data.names);
      if (ids.has(record[0])) throw new Error("Duplicate Boston road source ID");
      ids.add(record[0]);
      vertices += coordinates.length;
      if (vertices > 55000) throw new Error("Boston basemap exceeds vertex bound");
      const code = record[1];
      let west = Infinity, south = Infinity, east = -Infinity, north = -Infinity;
      for (const [lon, lat] of coordinates) {
        west = Math.min(west, lon); south = Math.min(south, lat);
        east = Math.max(east, lon); north = Math.max(north, lat);
      }
      // Only symbology is original; each centerline follows real source vertices.
      const major = code > 0 && code < 30;
      const collector = code >= 30 && code < 40;
      shapes.push({
        kind: "line", paths: [coordinates], label: data.names[record[2]],
        sourceId: record[0], cfcc: code, bostonBase: true,
        bounds: [west, south, east, north],
        style: {
          stroke: major ? "#a9d693" : collector ? "#719a72" : "#42634b",
          width: major ? 1.6 : collector ? 1.05 : 0.65,
          opacity: major ? 0.95 : collector ? 0.84 : 0.68
        }
      });
    }
    if (vertices !== 47994) throw new Error("Pinned Boston source vertex count changed");
    const labelShapes = shapes.filter((shape) =>
      shape.cfcc > 0 && shape.cfcc <= 35 && shape.label.length > 3
    ).sort((a, b) => a.cfcc - b.cfcc ||
      b.paths[0].length - a.paths[0].length);
    cache = Object.freeze({
      name: data.name, source: data.source, bounds: [...data.bounds],
      center: {lon: data.center[0], lat: data.center[1]},
      shapes, labelShapes, featureCount: shapes.length, vertexCount: vertices
    });
    return cache;
  }

  window.RustBoyBoston = Object.freeze({ load });
})();
