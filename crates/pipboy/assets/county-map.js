/* Real county geography only. Census TIGERweb 2026 vector services.
 * Every coordinate-bearing request follows an explicit operator AREA action.
 * No GPS, cookies, polling, retries, persistence or external scripts.
 */
(function () {
  "use strict";
  const ROOT = "https://tigerweb.geo.census.gov/arcgis/rest/services/TIGERweb/";
  const LIMIT_RESPONSE = 4 * 1024 * 1024;
  const LIMIT_TOTAL = 32 * 1024 * 1024;
  const LIMIT_FEATURES = 28000;
  const LIMIT_VERTICES = 230000;
  const LIMIT_BOUNDARY_VERTICES = 12000;
  const PAGE = 1200;
  const MAX_PAGES = 25;
  const classes = Object.freeze([
    ["roads-primary", "Transportation/MapServer/2", "line"],
    ["roads-secondary", "Transportation/MapServer/6", "line"],
    ["roads-local", "Transportation/MapServer/8", "line"],
    ["railroads", "Transportation/MapServer/9", "line"],
    ["water-lines", "Hydro/MapServer/0", "line"],
    ["water-areas", "Hydro/MapServer/1", "polygon"]
  ]);
  const styles = Object.freeze({
    "roads-primary": {stroke:"#dfdca3", width:2.25},
    "roads-secondary": {stroke:"#b4d5a4", width:1.65},
    "roads-local": {stroke:"#6b9676", width:1},
    railroads: {stroke:"#d1aa83", width:1.2},
    "water-lines": {stroke:"#6caeaa", width:1.25},
    "water-areas": {stroke:"#76b8b0",fill:"rgba(38,105,102,0.35)",width:0.8}
  });

  function fail(message) { throw new Error(message); }
  function validPoint(value) {
    return Array.isArray(value) && value.length >= 2 &&
      Number.isFinite(value[0]) && Number.isFinite(value[1]) &&
      Math.abs(value[0]) <= 180 && Math.abs(value[1]) <= 90;
  }
  function coordinates(a, limit) {
    if (!Array.isArray(a) || a.length > limit || a.length < 2) fail("INVALID CENSUS GEOMETRY");
    for (const p of a) if (!validPoint(p)) fail("INVALID CENSUS COORDINATE");
    return a.map(p => [p[0],p[1]]);
  }
  function regionPolygons(geometry) {
    if (!geometry || !["Polygon","MultiPolygon"].includes(geometry.type))
      fail("COUNTY BOUNDARY MISSING");
    const polys = geometry.type === "Polygon" ? [geometry.coordinates] : geometry.coordinates;
    if (!Array.isArray(polys) || !polys.length || polys.length > 300) fail("INVALID COUNTY PARTS");
    let vertices = 0;
    const result = polys.map(polygon => {
      if (!Array.isArray(polygon) || !polygon.length) fail("INVALID COUNTY RINGS");
      return polygon.map(ring => {
        const coords = coordinates(ring, LIMIT_BOUNDARY_VERTICES);
        vertices += coords.length;
        if (vertices > LIMIT_BOUNDARY_VERTICES || coords.length < 4 ||
            coords[0][0] !== coords.at(-1)[0] || coords[0][1] !== coords.at(-1)[1])
          fail("INVALID OR OVERSIZED COUNTY RING");
        return coords;
      });
    });
    return result;
  }
  function boundsOf(polygons) {
    const bounds = [180,90,-180,-90];
    for (const p of polygons) for (const r of p) for (const c of r) {
      bounds[0] = Math.min(bounds[0],c[0]);
      bounds[1] = Math.min(bounds[1],c[1]);
      bounds[2] = Math.max(bounds[2],c[0]);
      bounds[3] = Math.max(bounds[3],c[1]);
    }
    const [west,south,east,north] = bounds;
    if (west>=east||south>=north||east-west>2||north-south>2 ||
        (east-west)*(north-south)>2.2 || west < -179.9 || east > 179.9)
      fail("COUNTY TOO LARGE FOR HANDHELD FETCH; IMPORT OFFLINE RUSTGIS PACK");
    return bounds;
  }
  function insideRing(point, ring) {
    let inside = false;
    const [x,y] = point;
    for (let i=0,j=ring.length-1;i<ring.length;j=i++) {
      const a=ring[i],b=ring[j];
      if ((a[1]>y)!==(b[1]>y) &&
          x < (b[0]-a[0])*(y-a[1])/(b[1]-a[1])+a[0]) inside=!inside;
    }
    return inside;
  }
  function inCounty(point,polygons) {
    return polygons.some(p=>insideRing(point,p[0]) && !p.slice(1).some(r=>insideRing(point,r)));
  }
  function touches(a,b,c,d) {
    const ux=b[0]-a[0],uy=b[1]-a[1],vx=d[0]-c[0],vy=d[1]-c[1];
    const det=ux*vy-uy*vx;
    if (Math.abs(det)<1e-12) return null;
    const dx=c[0]-a[0],dy=c[1]-a[1];
    const t=(dx*vy-dy*vx)/det,u=(dx*uy-dy*ux)/det;
    return t>1e-8&&t<1-1e-8&&u>=0&&u<=1?t:null;
  }
  function interp(a,b,t) { return [a[0]+(b[0]-a[0])*t,a[1]+(b[1]-a[1])*t]; }
  // Clip county roads to the *sourced polygon*. Generated endpoints are
  // mathematical road/boundary intersections, not invented road alignments.
  function clipLine(line,polygons) {
    const out=[];
    let current=[];
    function flush() { if(current.length>=2)out.push(current);current=[]; }
    for(let i=1;i<line.length;i++) {
      const a=line[i-1],b=line[i];
      const t=[0,1];
      for(const poly of polygons) for(const ring of poly) for(let j=1;j<ring.length;j++){
        const c=ring[j-1],d=ring[j];
        if(Math.max(a[0],b[0])<Math.min(c[0],d[0])||
           Math.min(a[0],b[0])>Math.max(c[0],d[0])||
           Math.max(a[1],b[1])<Math.min(c[1],d[1])||
           Math.min(a[1],b[1])>Math.max(c[1],d[1]))continue;
        const cut=touches(a,b,c,d);
        if(cut!==null)t.push(cut);
      }
      t.sort((x,y)=>x-y);
      for(let j=1;j<t.length;j++) {
        const lo=t[j-1],hi=t[j];
        if(hi-lo<1e-9)continue;
        if(!inCounty(interp(a,b,(lo+hi)/2),polygons)){flush();continue;}
        const p0=interp(a,b,lo),p1=interp(a,b,hi);
        if(current.length && (Math.abs(current.at(-1)[0]-p0[0])>1e-8 ||
                              Math.abs(current.at(-1)[1]-p0[1])>1e-8))flush();
        if(!current.length)current.push(p0);
        current.push(p1);
      }
    }
    flush();
    return out;
  }
  function pointInBounds(p,b) {return p[0]>=b[0]&&p[0]<=b[2]&&p[1]>=b[1]&&p[1]<=b[3];}
  function polygonHasCountyPoint(rings,polygons) {
    return rings.some(r=>r.some(p=>inCounty(p,polygons))) ||
      polygons.some(poly=>poly.some(r=>r.some(p=>insideRing(p,rings[0]))));
  }
  function endpoint(layer) {return ROOT+layer+"/query";}
  function queryParams(fields) {
    return Object.entries({...fields,f:"geojson"}).map(
      ([key,value])=>encodeURIComponent(key)+"="+encodeURIComponent(String(value))).join("&");
  }
  async function boundedJson(response,account) {
    if (!response.ok) fail("CENSUS HTTP "+response.status);
    const size=Number(response.headers?.get?.("content-length"));
    if (Number.isFinite(size) && size>LIMIT_RESPONSE) fail("CENSUS RESPONSE TOO LARGE");
    let source;
    if(response.body && typeof response.body.getReader==="function"){
      const reader=response.body.getReader(),chunks=[];
      let len=0;
      try{
        while(true){
          const {done,value}=await reader.read();
          if(done)break;
          len+=value.byteLength;
          if(len>LIMIT_RESPONSE){await reader.cancel();fail("CENSUS RESPONSE TOO LARGE");}
          chunks.push(value);
        }
      }finally{reader.releaseLock();}
      const bytes=new Uint8Array(len);let offset=0;
      for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.byteLength;}
      source=new TextDecoder("utf-8",{fatal:true}).decode(bytes);
    }else if(typeof response.text==="function")source=await response.text();
    else fail("UNSUPPORTED CENSUS RESPONSE");
    const bytes=new TextEncoder().encode(source).byteLength;
    account.bytes+=bytes;
    if(bytes>LIMIT_RESPONSE||account.bytes>LIMIT_TOTAL)fail("CENSUS DOWNLOAD LIMIT REACHED");
    let value;
    try{value=JSON.parse(source);}catch(_){fail("INVALID CENSUS JSON");}
    if(value?.error)fail("CENSUS QUERY REJECTED");
    if(value?.type!=="FeatureCollection"||!Array.isArray(value.features))fail("UNEXPECTED CENSUS FORMAT");
    return value;
  }
  async function request(layer,params,signal,account) {
    const url=endpoint(layer)+"?"+queryParams(params);
    const response=await fetch(url,{method:"GET",mode:"cors",cache:"no-store",
      credentials:"omit",referrerPolicy:"no-referrer",signal});
    return boundedJson(response,account);
  }
  async function countyForPosition(lat,lon,signal,account) {
    if (!Number.isFinite(lat)||!Number.isFinite(lon)||Math.abs(lat)>90||Math.abs(lon)>180)
      fail("INVALID LOCATION COORDINATES");
    const data=await request("State_County/MapServer/1",{
      where:"1=1",geometry:lon.toFixed(5)+","+lat.toFixed(5),
      geometryType:"esriGeometryPoint",spatialRel:"esriSpatialRelIntersects",
      inSR:4326,outSR:4326,outFields:"GEOID,NAME,STATE,COUNTY",returnGeometry:true
    },signal,account);
    if(data.features.length!==1)fail("NO UNIQUE US COUNTY MATCH; EXISTING MAP PRESERVED");
    const feature=data.features[0],props=feature.properties||{};
    if(!/^\d{5}$/.test(props.GEOID)||typeof props.NAME!=="string"||
       props.NAME.length>100)fail("INVALID COUNTY IDENTIFIER");
    const polygons=regionPolygons(feature.geometry);
    return {id:props.GEOID,name:props.NAME,polygons,bounds:boundsOf(polygons)};
  }
  async function allPages(layer,bounds,signal,account) {
    const features=[];
    const [w,s,e,n]=bounds;
    for(let page=0;page<MAX_PAGES;page++){
      const response=await request(layer,{
        where:"1=1",geometry:[w,s,e,n].map(x=>x.toFixed(6)).join(","),
        geometryType:"esriGeometryEnvelope",spatialRel:"esriSpatialRelIntersects",
        inSR:4326,outSR:4326,outFields:"NAME,MTFCC,OBJECTID",returnGeometry:true,
        orderByFields:"OBJECTID ASC",resultOffset:page*PAGE,resultRecordCount:PAGE
      },signal,account);
      if(features.length+response.features.length>LIMIT_FEATURES)
        fail("COUNTY TOO DENSE; IMPORT COUNTY RUSTGIS MAP");
      features.push(...response.features);
      if(response.features.length<PAGE&&!response.exceededTransferLimit)return features;
      if(response.features.length===0)return features;
    }
    fail("COUNTY PAGINATION LIMIT REACHED; EXISTING MAP PRESERVED");
  }
  async function load(lat,lon,signal) {
    const account={bytes:0,vertices:0,features:0,requests:0};
    const county=await countyForPosition(lat,lon,signal,account);
    const shapes=[];
    const count={roads:0,railroads:0,water:0};
    for(const [kind,layer,shapeKind] of classes){
      const features=await allPages(layer,county.bounds,signal,account);
      for(const f of features){
        const geometry=f?.geometry;
        if(!geometry)continue;
        const label=typeof f.properties?.NAME==="string"?f.properties.NAME.slice(0,36):"";
        let coords=[];
        if(shapeKind==="line"){
          if(geometry.type==="LineString")coords=[geometry.coordinates];
          else if(geometry.type==="MultiLineString")coords=geometry.coordinates;
          else continue;
        }else{
          if(geometry.type==="Polygon")coords=[geometry.coordinates];
          else if(geometry.type==="MultiPolygon")coords=geometry.coordinates;
          else continue;
        }
        if(!Array.isArray(coords))fail("INVALID COUNTY FEATURE");
        for(const group of coords){
          if(shapeKind==="line"){
            const line=coordinates(group,10000);
            account.vertices+=line.length;
            if(account.vertices>LIMIT_VERTICES)fail("COUNTY GEOMETRY LIMIT EXCEEDED");
            for(const path of clipLine(line,county.polygons)){
              shapes.push({kind:"line",paths:[path],label,layerName:"CENSUS COUNTY "+
                county.id,countyBase:true,sourceType:kind,style:styles[kind]});
            }
          }else{
            if(!Array.isArray(group)||!group.length)continue;
            const rings=group.map(ring=>{
              const p=coordinates(ring,12000);account.vertices+=p.length;
              if(p.length<4)fail("INVALID WATER POLYGON");
              return p;
            });
            if(account.vertices>LIMIT_VERTICES)fail("COUNTY GEOMETRY LIMIT EXCEEDED");
            if(polygonHasCountyPoint(rings,county.polygons)){
              shapes.push({kind:"polygon",paths:rings,label,
                layerName:"CENSUS COUNTY "+county.id,countyBase:true,sourceType:kind,
                style:styles[kind]});
            }
          }
          if(shapes.length>LIMIT_FEATURES)fail("COUNTY FEATURE LIMIT EXCEEDED");
        }
        account.features++;
      }
      if(kind.startsWith("roads"))count.roads+=features.length;
      else if(kind==="railroads")count.railroads+=features.length;
      else count.water+=features.length;
    }
    if(!shapes.some(s=>s.sourceType.startsWith("roads")))
      fail("NO USABLE COUNTY ROADS; BASEMAP PRESERVED");
    return {
      id:county.id,name:county.name,bounds:county.bounds,polygons:county.polygons,
      shapes,counts:count,bytes:account.bytes,year:2026,
      provider:"US Census Bureau TIGERweb / 2026 county, roads and hydrography",
      source:"https://tigerweb.geo.census.gov/arcgis/rest/services/TIGERweb",
      license:"Public U.S. Census geography",
      sourcePosition:{type:"operator_requested",transmittedTo:"US Census TIGERweb"}
    };
  }
  window.RustBoyCountyMap=Object.freeze({
    load,countyForPosition,regionPolygons,clipLine,inCounty,boundsOf,
    provider:ROOT,classes:classes.map(row=>row[0])
  });
})();
