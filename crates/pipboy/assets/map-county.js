/* Rust-Boy county source pipeline. Only approved local endpoints are called.
 * Every drawn vertex originates in an actual Census TIGER .shp record.
 * GPS fixes are never sent to this module, only Census-resolved GEOIDs.
 */
(function () {
  "use strict";
  const VINTAGE = "2025";
  const MAX_ZIP = 12 * 1024 * 1024;
  const MAX_UNPACKED = 64 * 1024 * 1024;
  const MAX_FEATURES = 60000;
  const MAX_VERTICES = 450000;
  const SOURCES = Object.freeze(["tiger-roads", "tiger-areawater", "tiger-linearwater"]);
  const bad = (message) => { throw new Error(message); };
  const finite = (n) => typeof n === "number" && Number.isFinite(n);
  const validGeoid = (id) => typeof id === "string" && /^\d{5}$/.test(id) &&
    id.slice(0,2) !== "00" && id.slice(2) !== "000";
  const uint16 = (dv, offset, le = true) => {
    if (offset < 0 || offset + 2 > dv.byteLength) bad("County ZIP index truncated");
    return dv.getUint16(offset, le);
  };
  const uint32 = (dv, offset, le = true) => {
    if (offset < 0 || offset + 4 > dv.byteLength) bad("County ZIP index truncated");
    return dv.getUint32(offset, le);
  };
  const f64 = (dv, offset) => {
    if (offset < 0 || offset + 8 > dv.byteLength) bad("County SHP coordinate truncated");
    return dv.getFloat64(offset, true);
  };
  let crcTable = null;
  function crc32(bytes) {
    if (!crcTable) {
      crcTable = new Uint32Array(256);
      for (let i = 0; i < 256; i++) {
        let value = i;
        for (let j = 0; j < 8; j++) value = (value >>> 1) ^
          ((value & 1) ? 0xEDB88320 : 0);
        crcTable[i] = value >>> 0;
      }
    }
    let value = 0xFFFFFFFF;
    for (let i = 0; i < bytes.length; i++)
      value = (value >>> 8) ^ crcTable[(value ^ bytes[i]) & 255];
    return (value ^ 0xFFFFFFFF) >>> 0;
  }

  function zipDirectory(buffer, geoid, layer) {
    if (!(buffer instanceof ArrayBuffer) || buffer.byteLength < 22 ||
        buffer.byteLength > MAX_ZIP) bad("County ZIP outside 12 MiB bound");
    const dv = new DataView(buffer);
    let tail = -1;
    for (let i = buffer.byteLength - 22; i >= Math.max(0, buffer.byteLength - 65557); i--) {
      if (uint32(dv, i) === 0x06054B50) { tail = i; break; }
    }
    if (tail < 0 || uint16(dv, tail + 4) || uint16(dv, tail + 6))
      bad("County ZIP must be single-volume");
    const count = uint16(dv, tail + 10);
    if (!count || count > 24 || count !== uint16(dv, tail + 8))
      bad("Unexpected county ZIP entry count");
    const end = uint32(dv, tail + 16) + uint32(dv, tail + 12);
    if (end > tail) bad("County ZIP directory outside archive");
    let offset = uint32(dv, tail + 16);
    const matches = Object.create(null);
    const expected = "tl_" + VINTAGE + "_" + geoid + "_" + layer + ".";
    for (let i = 0; i < count; i++) {
      if (uint32(dv, offset) !== 0x02014B50) bad("Broken county ZIP directory");
      const flags = uint16(dv, offset + 8), method = uint16(dv, offset + 10);
      const crc = uint32(dv, offset + 16);
      const packed = uint32(dv, offset + 20), unpacked = uint32(dv, offset + 24);
      const nameLength = uint16(dv, offset + 28);
      const extraLength = uint16(dv, offset + 30);
      const commentLength = uint16(dv, offset + 32);
      const local = uint32(dv, offset + 42);
      const next = offset + 46 + nameLength + extraLength + commentLength;
      if (next > end || offset + 46 > end) bad("County ZIP directory truncated");
      const name = new TextDecoder("utf-8", {fatal:true}).decode(
        new Uint8Array(buffer, offset + 46, nameLength));
      offset = next;
      if (name.includes("/") || name.includes("\\") || name.includes("..") ||
          !name.startsWith(expected)) continue;
      const ext = name.slice(expected.length);
      if (ext !== "shp" && ext !== "dbf") continue;
      if (matches[ext]) bad("Duplicate county ZIP data member");
      if (flags & 1 || (method !== 0 && method !== 8) ||
          unpacked > (ext === "shp" ? MAX_UNPACKED : 24 * 1024 * 1024))
        bad("Unsupported county ZIP member");
      if (uint32(dv, local) !== 0x04034B50) bad("Broken local ZIP member");
      const start = local + 30 + uint16(dv, local + 26) + uint16(dv, local + 28);
      if (start + packed > buffer.byteLength) bad("County ZIP member truncated");
      matches[ext] = {method, crc, unpacked, packed: new Uint8Array(buffer, start, packed)};
    }
    if (!matches.shp) bad("County ZIP has no matching SHP");
    return matches;
  }

  async function unpack(entry) {
    if (entry.method === 0) {
      if (entry.packed.byteLength !== entry.unpacked || crc32(entry.packed) !== entry.crc)
        bad("County stored ZIP CRC mismatch");
      return entry.packed;
    }
    if (typeof DecompressionStream === "undefined" || typeof Blob === "undefined")
      bad("Browser lacks ZIP Deflate support; use a supported browser");
    let stream;
    try {
      stream = new Blob([entry.packed]).stream()
        .pipeThrough(new DecompressionStream("deflate-raw"));
    } catch (_) { bad("Raw ZIP Deflate unavailable"); }
    const reader = stream.getReader();
    const parts = [];
    let bytes = 0;
    try {
      while (true) {
        const chunk = await reader.read();
        if (chunk.done) break;
        bytes += chunk.value.byteLength;
        if (bytes > entry.unpacked || bytes > MAX_UNPACKED) {
          await reader.cancel();
          bad("County ZIP decompress limit exceeded");
        }
        parts.push(chunk.value);
      }
    } finally { reader.releaseLock(); }
    if (bytes !== entry.unpacked) bad("County ZIP unpacked size mismatch");
    const out = new Uint8Array(bytes);
    let offset = 0;
    for (const part of parts) { out.set(part, offset); offset += part.byteLength; }
    if (crc32(out) !== entry.crc) bad("County ZIP CRC failed");
    return out;
  }

  function fieldsFromDbf(raw) {
    if (!raw) return null;
    if (raw.byteLength < 65) bad("Invalid county DBF");
    const dv = new DataView(raw.buffer, raw.byteOffset, raw.byteLength);
    const records = uint32(dv, 4), header = uint16(dv, 8), width = uint16(dv, 10);
    if (records > MAX_FEATURES || header < 65 || header > raw.length ||
        width < 2 || header + records * width > raw.length)
      bad("County DBF table invalid or too large");
    const fields = [];
    let start = 1;
    for (let offset = 32; offset + 32 <= header; offset += 32) {
      if (raw[offset] === 0x0D) break;
      const name = new TextDecoder("ascii").decode(raw.subarray(offset, offset + 11))
        .replace(/\0.*$/, "").trim().toUpperCase();
      const len = raw[offset + 16];
      if (!len || start + len > width) bad("County DBF field exceeds record");
      fields.push({name, start, len});
      start += len;
    }
    return function labelAt(row) {
      if (row >= records) return {name:"",mtfcc:""};
      const startAt = header + row * width;
      if (raw[startAt] === 0x2A) return {name:"",mtfcc:""};
      const get = (name) => {
        const field = fields.find(f => f.name === name);
        if (!field) return "";
        return new TextDecoder("latin1").decode(raw.subarray(
          startAt + field.start, startAt + field.start + field.len))
          .replace(/[\x00-\x1F\x7F]/g,"").trim().slice(0,64);
      };
      return {name:get("FULLNAME") || get("NAME"), mtfcc:get("MTFCC")};
    };
  }

  function readShp(raw, layer, dbf) {
    if (raw.byteLength < 100) bad("County SHP header truncated");
    const dv = new DataView(raw.buffer, raw.byteOffset, raw.byteLength);
    if (uint32(dv, 0, false) !== 9994 || uint32(dv, 28) !== 1000 ||
        uint32(dv, 24, false) * 2 !== raw.byteLength)
      bad("Invalid county SHP header");
    const expected = layer === "areawater" ? 5 : 3;
    const fileType = uint32(dv, 32);
    if (fileType !== expected) bad("County layer has wrong geometry type");
    const shapes = [];
    let offset = 100, vertices = 0, records = 0;
    const bbox = [Infinity, Infinity, -Infinity, -Infinity];
    const label = fieldsFromDbf(dbf);
    while (offset < raw.length) {
      if (offset + 12 > raw.length) bad("County SHP record truncated");
      const recordNumber = uint32(dv, offset, false);
      const length = uint32(dv, offset + 4, false) * 2;
      const end = offset + 8 + length;
      if (end > raw.length || length < 4 || recordNumber !== records + 1)
        bad("County SHP record malformed");
      const kind = uint32(dv, offset + 8);
      if (kind === 0) {offset=end; records++;continue;}
      if (kind !== expected || length < 44) bad("Unexpected county SHP feature");
      const countParts = uint32(dv, offset + 44);
      const countPoints = uint32(dv, offset + 48);
      const partsAt = offset + 52, pointsAt = partsAt + 4 * countParts;
      if (!countParts || !countPoints || countParts > 600 ||
          vertices + countPoints > MAX_VERTICES ||
          pointsAt + countPoints * 16 > end)
        bad("County SHP exceeds handheld geometry bound");
      const parts = [];
      let last = -1;
      for (let i = 0; i < countParts; i++) {
        const idx = uint32(dv, partsAt + 4 * i);
        if (idx <= last || idx >= countPoints) bad("County SHP part order invalid");
        parts.push(idx); last=idx;
      }
      const coords = [];
      let west=Infinity,south=Infinity,east=-Infinity,north=-Infinity;
      for (let i=0;i<countPoints;i++) {
        const x = f64(dv, pointsAt + i*16);
        const y = f64(dv, pointsAt + i*16 + 8);
        if (!finite(x)||!finite(y)||Math.abs(x)>180||Math.abs(y)>90)
          bad("County SHP contains invalid geographic NAD83 coordinates");
        coords.push([x,y]);
        west=Math.min(west,x);east=Math.max(east,x);
        south=Math.min(south,y);north=Math.max(north,y);
      }
      const paths = [];
      for (let i=0; i<parts.length; i++) {
        const path = coords.slice(parts[i], i+1<parts.length ? parts[i+1] : countPoints);
        if (path.length < (layer==="areawater"?4:2)) bad("County SHP short geometry");
        if (layer === "areawater" &&
            (path[0][0] !== path.at(-1)[0] || path[0][1] !== path.at(-1)[1]))
          bad("County water polygon is not closed");
        paths.push(path);
      }
      vertices+=countPoints;
      const record = label ? label(records) : {name:"",mtfcc:""};
      const major = /^(S1100|S1200)$/.test(record.mtfcc);
      shapes.push({
        kind: layer==="areawater" ? "polygon" : "line",
        paths, label: record.name.slice(0,36),
        countyBase:true, countyLayer:layer, countyMajor:major,
        countyMtfcc:record.mtfcc,
        bounds:[west,south,east,north],
        style:layer==="roads"
          ? {stroke:major?"#b9dda1":"#729b7c",width:major?1.6:0.7,opacity:0.9}
          : layer==="linearwater" ? {stroke:"#81b1b8",width:1.0,opacity:0.86}
          : {stroke:"#7ca6a6",fill:"rgba(43,104,111,.35)",width:0.85,opacity:0.86}
      });
      bbox[0]=Math.min(bbox[0],west);bbox[1]=Math.min(bbox[1],south);
      bbox[2]=Math.max(bbox[2],east);bbox[3]=Math.max(bbox[3],north);
      offset=end;records++;
      if (records > MAX_FEATURES) bad("County SHP feature count exceeds handheld limit");
    }
    if (offset!==raw.length || !shapes.length)
      bad("County SHP contains no supported real geometry");
    return {shapes,features:shapes.length,vertices,bounds:bbox,layer,records,
      sourceCrs:"GCS_NAD83",datumTransform:"not_applied"};
  }

  async function parsePackage(buffer, geoid, source) {
    if (!validGeoid(geoid) || !SOURCES.includes(source))
      bad("Invalid county package source or GEOID");
    const layer = source === "tiger-roads" ? "roads" :
      source === "tiger-areawater" ? "areawater" : "linearwater";
    const directory = zipDirectory(buffer, geoid, layer);
    const shp = await unpack(directory.shp);
    const dbf = directory.dbf ? await unpack(directory.dbf) : null;
    return readShp(shp, layer, dbf);
  }

  async function boundedResponse(response) {
    const length = Number(response.headers?.get("content-length") || 0);
    if (length > MAX_ZIP) bad("County package exceeds network budget");
    if (response.body && typeof response.body.getReader === "function") {
      const reader = response.body.getReader();
      const chunks = [];
      let bytes = 0;
      try {
        while (true) {
          const chunk = await reader.read();
          if (chunk.done) break;
          bytes+=chunk.value.byteLength;
          if (bytes > MAX_ZIP) {await reader.cancel();bad("County download exceeds limit");}
          chunks.push(chunk.value);
        }
      } finally {reader.releaseLock();}
      const buffer = new Uint8Array(bytes);
      let index=0;
      for (const chunk of chunks){buffer.set(chunk,index);index+=chunk.byteLength;}
      return buffer.buffer;
    }
    const buffer = await response.arrayBuffer();
    if (buffer.byteLength > MAX_ZIP) bad("County download exceeds limit");
    return buffer;
  }

  async function localJson(url, signal) {
    const response = await fetch(url,{cache:"no-store",credentials:"omit",signal});
    if (!response.ok) bad("Local county catalog/cache unavailable ("+response.status+")");
    return response.json();
  }

  async function active(signal) {
    const data = await localJson("/api/map/cache",signal);
    if (data.schema!==1 || (data.active_geoid!==null &&
        !validGeoid(data.active_geoid)) ||
        !Array.isArray(data.cached_sources) || data.network_used!==false)
      bad("Unrecognized local county cache manifest");
    return data;
  }

  async function sources(signal) {
    const data = await localJson("/api/map/sources",signal);
    if (data.schema!==1 || data.vintage!==VINTAGE ||
        !Array.isArray(data.sources) || data.sources.length!==SOURCES.length ||
        !data.sources.every(s=>SOURCES.includes(s.id)) ||
        !SOURCES.every(id=>data.sources.some(s=>s.id===id)))
      bad("County source list is not the approved Census catalog");
    return data.sources;
  }

  async function packageFor(geoid, id, cacheOnly, signal) {
    if (!validGeoid(geoid)||!SOURCES.includes(id)) bad("Invalid county source selection");
    const response=await fetch("/api/map/package",{
      method:"POST",headers:{"Content-Type":"application/json"},
      credentials:"omit",cache:"no-store",signal,
      body:JSON.stringify({geoid,source:id,confirm:!cacheOnly,cache_only:cacheOnly})
    });
    if (!response.ok) bad((cacheOnly?"OFFLINE PACKAGE ":"CENSUS PACKAGE ")+
      id.toUpperCase()+" UNAVAILABLE ("+response.status+")");
    return parsePackage(await boundedResponse(response),geoid,id);
  }

  async function load(geoid, {cacheOnly=false,signal,onLayer,onStatus}={}) {
    if (!validGeoid(geoid)) bad("County GEOID not valid");
    if (!cacheOnly) await sources(signal);
    const report={geoid,sourceIds:[],errors:[],features:0};
    for (const id of SOURCES) {
      if (signal?.aborted) bad("County loading cancelled");
      onStatus?.("COUNTY "+geoid+" / "+id.toUpperCase()+(cacheOnly?" / CACHE":" / CENSUS"));
      try {
        const parsed=await packageFor(geoid,id,cacheOnly,signal);
        if (signal?.aborted) bad("County loading cancelled");
        report.sourceIds.push(id);report.features+=parsed.features;
        onLayer?.(id,parsed);
      } catch (error) {
        if (signal?.aborted) throw error;
        report.errors.push(id+": "+(error instanceof Error?error.message:"unavailable"));
      }
    }
    if (!report.sourceIds.length)
      bad("No county source loaded: "+report.errors.join(" | ").slice(0,130));
    return report;
  }

  window.RustBoyCounty=Object.freeze({
    load,active,sources,parsePackage,validGeoid,sourceIds:SOURCES
  });
})();
