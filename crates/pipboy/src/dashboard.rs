//! Rust-Boy graphical dashboard: a loopback-only, dependency-free HTTP UI.
//!
//! This is a local companion window, not an external web service. Profiles are
//! declarative. No hardware, camera, console, GNSS or TAK transport is opened
//! by browsing the dashboard or selecting a role.
mod messenger_routes;
mod network_routes;
mod task_routes;

use crate::county_cache::{self, CountyCache};
use crate::radio_output::available_outputs;
use crate::radio_playback::NativeRadioEngine;
use crate::zip_index::ZipIndex;
use pipboy_core::network_health;
use pipboy_core::network_suite;
use pipboy_core::radio_directory::curated_stations;
use pipboy_core::radio_presets::{MoveDirection, RadioPresetStore, RadioPresets};
use pipboy_core::reclamation::{NewSite, SiteStatus, SiteStore};
use pipboy_core::scout_gis::{distance_meters, waypoint, Point};
use pipboy_core::workbench::{inspect_hex, tool, WorkbenchStore, PALETTE, PRESETS};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::error::Error;
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

const DEFAULT_PORT: u16 = 8765;
const MAX_HEADER: usize = 8192;
const MAX_BODY: usize = 4096;
const HTML: &str = include_str!("../assets/dashboard.html");
const CSS: &str = include_str!("../assets/dashboard.css");
const JS: &str = include_str!("../assets/dashboard.js");
const MICRO_HTML: &str = include_str!("../assets/micro.html");
const MICRO_CSS: &str = include_str!("../assets/micro.css");
const MICRO_JS: &str = include_str!("../assets/micro.js");
const MAP_JS: &str = include_str!("../assets/map.js");
const MAP_IMPORT_JS: &str = include_str!("../assets/map-import.js");
const MAP_ONLINE_JS: &str = include_str!("../assets/map-online.js");
const COUNTY_JS: &str = include_str!("../assets/map-county.js");
const ZIP_JS: &str = include_str!("../assets/map-zip.js");
const MAP_CSS: &str = include_str!("../assets/map.css");
const NETWORK_JS: &str = include_str!("../assets/network.js");
const NETWORK_CSS: &str = include_str!("../assets/network.css");
const TASKS_JS: &str = include_str!("../assets/tasks.js");
const TASKS_CSS: &str = include_str!("../assets/tasks.css");
const MESSENGER_JS: &str = include_str!("../assets/messenger.js");
const MESSENGER_CSS: &str = include_str!("../assets/messenger.css");
const RECLAMATION_JS: &str = include_str!("../assets/reclamation.js");
const RECLAMATION_CSS: &str = include_str!("../assets/reclamation.css");
const CAMERAS_JS: &str = include_str!("../assets/cameras.js");
const CAMERAS_CSS: &str = include_str!("../assets/cameras.css");
const RADIO_UI_JS: &str = include_str!("../assets/radio-ui.js");
const RADIO_UI_CSS: &str = include_str!("../assets/radio-ui.css");

static RADIO_ENGINE: LazyLock<Mutex<Option<NativeRadioEngine>>> =
    LazyLock::new(|| Mutex::new(None));
static RADIO_OUTPUT: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new("system".into()));

struct Request {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }
}

#[derive(Debug)]
struct Response {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}

impl Response {
    fn new(status: u16, content_type: &'static str, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            content_type,
            body: body.into(),
        }
    }

    fn json(status: u16, data: &Value) -> Self {
        Self::new(
            status,
            "application/json; charset=utf-8",
            serde_json::to_vec(data).expect("serializing a validated JSON value"),
        )
    }

    fn error(status: u16, message: &str) -> Self {
        Self::json(status, &json!({ "error": message }))
    }

    fn write_to(&self, stream: &mut TcpStream) -> io::Result<()> {
        let label = match self.status {
            200 => "OK",
            204 => "No Content",
            400 => "Bad Request",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            409 => "Conflict",
            411 => "Length Required",
            413 => "Content Too Large",
            415 => "Unsupported Media Type",
            422 => "Unprocessable Content",
            431 => "Request Header Fields Too Large",
            500 => "Internal Server Error",
            _ => "Error",
        };
        let head = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
             Cache-Control: no-store, max-age=0\r\n\
             Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self' https://overpass-api.de https://tigerweb.geo.census.gov; frame-src https://www.youtube-nocookie.com; frame-ancestors 'none'; base-uri 'none'; form-action 'none'\r\n\
             X-Content-Type-Options: nosniff\r\n\
             X-Frame-Options: DENY\r\n\
             Referrer-Policy: no-referrer\r\n\
             Cross-Origin-Resource-Policy: same-origin\r\n\
             Connection: close\r\n\r\n",
            self.status,
            label,
            self.content_type,
            self.body.len()
        );
        stream.write_all(head.as_bytes())?;
        stream.write_all(&self.body)?;
        stream.flush()
    }
}

fn receive(stream: &mut TcpStream) -> Result<Request, Response> {
    let mut data = Vec::new();
    let boundary = loop {
        if let Some(offset) = data.windows(4).position(|part| part == b"\r\n\r\n") {
            if offset + 4 > MAX_HEADER {
                return Err(Response::error(431, "HTTP headers exceed 8 KiB"));
            }
            break offset + 4;
        }
        if data.len() >= MAX_HEADER {
            return Err(Response::error(431, "HTTP headers exceed 8 KiB"));
        }
        let mut chunk = [0u8; 1024];
        let count = stream
            .read(&mut chunk)
            .map_err(|_| Response::error(400, "Incomplete HTTP request"))?;
        if count == 0 {
            return Err(Response::error(400, "Incomplete HTTP headers"));
        }
        data.extend_from_slice(&chunk[..count]);
        if data.len() > MAX_HEADER + MAX_BODY {
            return Err(Response::error(413, "HTTP request too large"));
        }
    };

    let head = std::str::from_utf8(&data[..boundary])
        .map_err(|_| Response::error(400, "Invalid HTTP header text"))?;
    let mut lines = head.split("\r\n");
    let first = lines.next().unwrap_or_default();
    let mut words = first.split_whitespace();
    let method = words.next().unwrap_or_default().to_owned();
    let path = words.next().unwrap_or_default().to_owned();
    let version = words.next().unwrap_or_default();
    if words.next().is_some()
        || !matches!(method.as_str(), "GET" | "POST")
        || !path.starts_with('/')
        || version != "HTTP/1.1"
    {
        return Err(Response::error(400, "Unsupported HTTP request line"));
    }
    let mut headers = BTreeMap::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            return Err(Response::error(400, "Folded HTTP headers are not allowed"));
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| Response::error(400, "Malformed HTTP header"))?;
        if name.is_empty() || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-') {
            return Err(Response::error(400, "Malformed HTTP header name"));
        }
        let key = name.to_ascii_lowercase();
        if headers.contains_key(&key) {
            return Err(Response::error(
                400,
                "Duplicate HTTP headers are not allowed",
            ));
        }
        let value = value.trim();
        if value.bytes().any(|c| c.is_ascii_control()) {
            return Err(Response::error(400, "Malformed HTTP header value"));
        }
        headers.insert(key, value.to_owned());
    }
    if headers.contains_key("transfer-encoding") {
        return Err(Response::error(
            400,
            "Chunked HTTP uploads are not accepted",
        ));
    }
    let declared_length = headers
        .get("content-length")
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|_| Response::error(400, "Invalid Content-Length"))?;
    if method == "POST" && declared_length.is_none() {
        return Err(Response::error(411, "Content-Length required"));
    }
    let length = declared_length.unwrap_or(0);
    if length > MAX_BODY {
        return Err(Response::error(413, "JSON request body exceeds 4 KiB"));
    }
    if method == "GET" && length != 0 {
        return Err(Response::error(400, "GET requests cannot contain a body"));
    }
    let received = data.len() - boundary;
    if received > length {
        return Err(Response::error(
            400,
            "Unexpected bytes after HTTP request body",
        ));
    }
    while data.len() - boundary < length {
        let mut chunk = [0u8; 1024];
        let required = (boundary + length - data.len()).min(chunk.len());
        let count = stream
            .read(&mut chunk[..required])
            .map_err(|_| Response::error(400, "Incomplete request body"))?;
        if count == 0 {
            return Err(Response::error(400, "Incomplete request body"));
        }
        data.extend_from_slice(&chunk[..count]);
    }
    Ok(Request {
        method,
        path,
        headers,
        body: data[boundary..].to_vec(),
    })
}

fn parse_json_body(req: &Request, permitted: &[&str]) -> Result<Value, Response> {
    if req.header("content-type") != Some("application/json") {
        return Err(Response::error(
            415,
            "Require Content-Type: application/json",
        ));
    }
    let value: Value = serde_json::from_slice(&req.body)
        .map_err(|_| Response::error(400, "Invalid JSON request body"))?;
    let object = value
        .as_object()
        .ok_or_else(|| Response::error(400, "Expected a JSON object"))?;
    if object.keys().any(|key| !permitted.contains(&key.as_str())) {
        return Err(Response::error(400, "Unexpected JSON field"));
    }
    Ok(value)
}

fn str_field<'a>(body: &'a Value, key: &str) -> Result<&'a str, Response> {
    body.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| Response::error(422, "Missing or invalid string field"))
}

fn positive_integer(body: &Value, key: &str) -> Result<u64, Response> {
    body.get(key)
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .ok_or_else(|| Response::error(422, "Missing or invalid task ID/revision"))
}

fn state(store: &WorkbenchStore) -> Result<Response, Response> {
    let active = store.load().map_err(|_| {
        Response::error(
            409,
            "Cannot read the local workbench; original file preserved",
        )
    })?;
    Ok(Response::json(
        200,
        &json!({
            "app": "Rust-Boy",
            "mode": "local_dashboard",
            "active": active,
            "profiles": PRESETS,
            "camera_feeds": pipboy_core::public_cameras::PUBLIC_CAMERA_FEEDS,
            "palette": PALETTE,
            "net_operations": network_suite::OPERATIONS,
            "engine": {
                "rustgis": "not_integrated",
                "camera_wall": "public_fallback_embeds",
                "private_cameras": "not_configured",
                "tak": "not_integrated",
                "game_link": "explicit_cli_only"
            },
            "storage": "local",
            "network": "loopback_only"
        }),
    ))
}

fn native_result(value: io::Result<String>) -> Response {
    match value {
        Ok(output) => Response::json(200, &json!({"kind": "computed", "output": output})),
        Err(_) => Response::error(422, "Invalid input for this offline tool"),
    }
}

fn experiment(body: &Value, store: &WorkbenchStore) -> Result<Response, Response> {
    let tool_id = str_field(body, "tool")?;
    let active = store
        .load()
        .map_err(|_| Response::error(409, "Workbench state unavailable"))?;
    if !active.slots.iter().any(|slot| slot.tool == tool_id) {
        return Err(Response::error(
            409,
            "Load a role containing this tool first",
        ));
    }
    let output = match tool_id {
        "byte-lab" => {
            let input = str_field(body, "input")?;
            if input.len() > 4096 {
                return Err(Response::error(422, "Hex input exceeds 4 KiB"));
            }
            return Ok(native_result(inspect_hex(input)));
        }
        "net-lab" => {
            let mode = body.get("mode").and_then(Value::as_str).unwrap_or("cidr4");
            let input = str_field(body, "input")?;
            let extra = body.get("extra").and_then(Value::as_str);
            if input.len() > 256 || extra.is_some_and(|x| x.len() > 1024) {
                return Err(Response::error(422, "Network bench input exceeds limit"));
            }
            if mode == "icmp" {
                if body.get("confirm").and_then(Value::as_bool) != Some(true) {
                    return Err(Response::error(422, "ICMP ping requires confirmation"));
                }
                let count = network_health::count(extra.unwrap_or("5"))
                    .map_err(|_| Response::error(422, "ICMP sample count must be 3..8"))?;
                let metrics = crate::endpoint_probe::icmp_health(input, count, true)
                    .map_err(|_| Response::error(409, "Native ping unavailable, incomplete or invalid"))?;
                return Ok(Response::json(200, &json!({
                    "kind":"icmp", "output":metrics.console(), "metrics":metrics
                })));
            }
            if mode == "health" {
                if body.get("confirm").and_then(Value::as_bool) != Some(true) {
                    return Err(Response::error(422, "Network Health requires confirmation"));
                }
                let port = extra.ok_or_else(|| Response::error(422, "TCP port required"))?;
                let number = match body.get("samples") {
                    Some(value) => value
                        .as_str()
                        .ok_or_else(|| Response::error(422, "Sample count must be a string"))?,
                    None => "5",
                };
                let count = network_health::count(number)
                    .map_err(|_| Response::error(422, "Sample count must be 3..8"))?;
                let metrics = crate::endpoint_probe::tcp_health(input, port, count, true)
                    .map_err(|_| Response::error(422, "Invalid Network Health target"))?;
                return Ok(Response::json(200, &json!({
                    "kind":"health", "output": metrics.console(), "metrics": metrics
                })));
            }
            if mode == "tcp" {
                if body.get("confirm").and_then(Value::as_bool) != Some(true) {
                    return Err(Response::error(422, "TCP check requires confirmation"));
                }
                let port = extra.ok_or_else(|| Response::error(422, "TCP port required"))?;
                let output = crate::endpoint_probe::tcp_once(input, port, true)
                    .map_err(|_| Response::error(422, "Invalid TCP endpoint or input"))?;
                return Ok(Response::json(
                    200, &json!({"kind":"probe", "output":output, "connected_automatically":false})
                ));
            }
            return Ok(native_result(network_suite::run_offline(mode, input, extra)));
        }
        "geo-lab" => {
            let mode = str_field(body, "mode")?;
            let a = Point::from_text(str_field(body, "lat")?, str_field(body, "lon")?)
                .map_err(|_| Response::error(422, "Invalid WGS84 starting coordinate"))?;
            if mode == "waypoint" {
                let name = str_field(body, "name")?;
                waypoint(name, a)
                    .and_then(|feature| serde_json::to_string_pretty(&feature).map_err(io::Error::other))
                    .map_err(|_| Response::error(422, "Invalid waypoint name or coordinates"))?
            } else if mode == "distance" {
                let b = Point::from_text(str_field(body, "lat2")?, str_field(body, "lon2")?)
                    .map_err(|_| Response::error(422, "Invalid WGS84 destination coordinate"))?;
                let meters = distance_meters(a, b);
                format!(
                    "SCOUT GIS / OFFLINE\nGreat-circle distance: {:.2} m ({:.3} km)\n\
                     Model: mean-radius spherical Earth\n\
                     No map renderer, GPS or location sharing is active.",
                    meters,
                    meters / 1000.0
                )
            } else {
                return Err(Response::error(422, "Unknown GIS experiment mode"));
            }
        }
        "diag" => {
            let snapshot = super::status::collect()
                .map_err(|_| Response::error(409, "Local host telemetry unavailable"))?;
            pipboy_core::diagnostics::analyze_host(&snapshot)
                .map_err(|_| Response::error(500, "Cannot evaluate local host readings"))?
                .render_text()
        }
        "host" => {
            let report = super::status::collect_details()
                .map_err(|_| Response::error(409, "Host telemetry not available"))?;
            serde_json::to_string_pretty(&report)
                .map_err(|_| Response::error(500, "Cannot format host data"))?
        }
        "library" => {
            "Reference catalog: pipboy sources\nBundled source documents only. No network fetch performed."
                .to_string()
        }
        "notes" => {
            "Local notes are available via: pipboy notes list\n\
             No note contents are read or exposed by this dashboard."
                .to_string()
        }
        "simulator" => {
            "Use: pipboy simulate list\nSynthetic fixture environment only. No hardware access."
                .to_string()
        }
        "game" => {
            "PLAYSTATION / MANUAL OPT-IN\nUse: pipboy game --ps4 <PS4_IP> --confirm\n\
             This dashboard does not connect to a console or poll the LAN."
                .to_string()
        }
        _ => {
            return Err(Response::error(422, "Unsupported or unavailable tool"));
        }
    };
    Ok(Response::json(
        200,
        &json!({
            "kind": if matches!(tool_id, "geo-lab" | "host" | "diag") { "computed" } else { "information" },
            "output": output
        }),
    ))
}

fn county_cache(store: &WorkbenchStore) -> CountyCache {
    CountyCache::new(
        store
            .path()
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("county_cache"),
    )
}

fn county_package(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    let body = parse_json_body(req, &["geoid", "source", "confirm", "cache_only"])?;
    let geoid = str_field(&body, "geoid")?;
    let source = str_field(&body, "source")?;
    let cache_only = body.get("cache_only").and_then(Value::as_bool) == Some(true);
    let confirm = body.get("confirm").and_then(Value::as_bool) == Some(true);
    if !confirm && !cache_only {
        return Err(Response::error(
            403,
            "Confirm Census county network acquisition",
        ));
    }
    let bytes = county_cache(store)
        .fetch(geoid, source, !cache_only && confirm)
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => Response::error(404, "No cached county package"),
            io::ErrorKind::InvalidInput => Response::error(422, "Invalid county GEOID or source"),
            io::ErrorKind::WouldBlock => {
                Response::error(409, "County download already in progress")
            }
            _ => Response::error(409, "County archive unavailable; original cache preserved"),
        })?;
    Ok(Response::new(200, "application/zip", bytes))
}

fn county_boundary(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    let body = parse_json_body(req, &["geoid", "confirm", "cache_only"])?;
    let geoid = str_field(&body, "geoid")?;
    let cache_only = body.get("cache_only").and_then(Value::as_bool) == Some(true);
    let confirm = body.get("confirm").and_then(Value::as_bool) == Some(true);
    if !confirm && !cache_only {
        return Err(Response::error(
            403,
            "Confirm Census county boundary acquisition",
        ));
    }
    let boundary = county_cache(store)
        .boundary(geoid, !cache_only && confirm)
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => Response::error(404, "No cached county boundary"),
            io::ErrorKind::InvalidInput => Response::error(422, "Invalid county boundary or GEOID"),
            io::ErrorKind::WouldBlock => {
                Response::error(409, "County download already in progress")
            }
            _ => Response::error(409, "Census boundary unavailable; existing cache preserved"),
        })?;
    Ok(Response::json(200, &boundary))
}

fn zip_index(store: &WorkbenchStore) -> ZipIndex {
    ZipIndex::new(
        store
            .path()
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("geo_index"),
    )
}

fn zip_lookup(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    let body = parse_json_body(req, &["zip", "confirm", "cache_only"])?;
    let zip = str_field(&body, "zip")?;
    let cache_only = body.get("cache_only").and_then(Value::as_bool) == Some(true);
    let confirm = body.get("confirm").and_then(Value::as_bool) == Some(true);
    if !confirm && !cache_only {
        return Err(Response::error(
            403,
            "ZIP relationship lookup requires confirmation",
        ));
    }
    let result = zip_index(store)
        .resolve_zip(zip, !cache_only && confirm)
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => Response::error(404, "ZCTA index not cached"),
            io::ErrorKind::InvalidInput => Response::error(422, "Invalid ZIP or Census ZCTA index"),
            io::ErrorKind::WouldBlock => Response::error(409, "Census lookup download in progress"),
            _ => Response::error(
                409,
                "Census ZCTA source unavailable; existing cache preserved",
            ),
        })?;
    Ok(Response::json(200, &result))
}

fn county_zips(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    let body = parse_json_body(req, &["geoid", "offset", "confirm", "cache_only"])?;
    let geoid = str_field(&body, "geoid")?;
    let offset = match body.get("offset") {
        None => 0,
        Some(value) => value
            .as_u64()
            .filter(|offset| *offset <= 80_000)
            .ok_or_else(|| Response::error(422, "Invalid county ZIP page offset"))?
            as usize,
    };
    let cache_only = body.get("cache_only").and_then(Value::as_bool) == Some(true);
    let confirm = body.get("confirm").and_then(Value::as_bool) == Some(true);
    if !confirm && !cache_only {
        return Err(Response::error(
            403,
            "County ZCTA lookup requires confirmation",
        ));
    }
    let result = zip_index(store)
        .county_zctas(geoid, offset, !cache_only && confirm)
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => Response::error(404, "ZCTA index not cached"),
            io::ErrorKind::InvalidInput => {
                Response::error(422, "Invalid county or Census ZCTA index")
            }
            io::ErrorKind::WouldBlock => Response::error(409, "Census lookup download in progress"),
            _ => Response::error(
                409,
                "Census ZCTA source unavailable; existing cache preserved",
            ),
        })?;
    Ok(Response::json(200, &result))
}

fn reclamation_store(store: &WorkbenchStore) -> SiteStore {
    SiteStore::new(
        store
            .path()
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf(),
    )
}

fn reclamation_allowed(store: &WorkbenchStore) -> Result<(), Response> {
    let board = store
        .load()
        .map_err(|_| Response::error(409, "Workbench unavailable"))?;
    if board.slots.iter().any(|slot| slot.tool == "reclamation") {
        Ok(())
    } else {
        Err(Response::error(
            403,
            "Pin Reclamation or load Reclamation Scout first",
        ))
    }
}

fn reclamation_read(store: &WorkbenchStore, export: bool) -> Result<Response, Response> {
    reclamation_allowed(store)?;
    let book = reclamation_store(store)
        .load()
        .map_err(|_| Response::error(409, "Site register unavailable; original preserved"))?;
    if export {
        let geojson = book
            .geojson()
            .map_err(|_| Response::error(409, "Cannot export site register"))?;
        Ok(Response::json(200, &geojson))
    } else {
        Ok(Response::json(
            200,
            &json!({"book": book, "location": "manual_only", "network_used": false}),
        ))
    }
}

fn reclamation_map_read(store: &WorkbenchStore) -> Result<Response, Response> {
    reclamation_allowed(store)?;
    let book = reclamation_store(store)
        .load()
        .map_err(|_| Response::error(409, "Site register unavailable; original preserved"))?;
    let sites: Vec<Value> = book
        .sites
        .iter()
        .map(|site| {
            json!({
                "id": site.id,
                "name": site.name,
                "latitude": site.latitude,
                "longitude": site.longitude,
                "accuracy_m": site.accuracy_m,
                "position_source": site.position_source,
                "priority": site.priority,
                "condition": site.condition,
                "status": site.status,
                "kind": site.kind
            })
        })
        .collect();
    Ok(Response::json(
        200,
        &json!({
            "sites": sites,
            "position_source": "manual",
            "network_used": false
        }),
    ))
}

fn reclamation_create(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    reclamation_allowed(store)?;
    let body = parse_json_body(
        req,
        &[
            "name",
            "kind",
            "condition",
            "priority",
            "latitude",
            "longitude",
            "accuracy_m",
            "notes",
        ],
    )?;
    let input: NewSite =
        serde_json::from_value(body).map_err(|_| Response::error(422, "Invalid site fields"))?;
    let site = reclamation_store(store)
        .create(input)
        .map_err(|_| Response::error(422, "Invalid site or local storage unavailable"))?;
    Ok(Response::json(200, &json!({"site": site})))
}

fn reclamation_change(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    reclamation_allowed(store)?;
    let body = parse_json_body(req, &["id", "revision", "status", "notes"])?;
    let id = positive_integer(&body, "id")?;
    let revision = positive_integer(&body, "revision")?;
    let status: SiteStatus = serde_json::from_value(
        body.get("status")
            .cloned()
            .ok_or_else(|| Response::error(422, "Missing status"))?,
    )
    .map_err(|_| Response::error(422, "Invalid site status"))?;
    let notes = match body.get("notes") {
        Some(Value::String(value)) => Some(value.as_str()),
        None => None,
        _ => return Err(Response::error(422, "Invalid notes")),
    };
    let site = reclamation_store(store)
        .change(id, revision, status, notes)
        .map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                Response::error(409, "Site changed elsewhere; refresh before updating")
            } else if error.kind() == io::ErrorKind::NotFound {
                Response::error(404, "Site not found")
            } else {
                Response::error(422, "Invalid site update or storage unavailable")
            }
        })?;
    Ok(Response::json(200, &json!({"site": site})))
}

/// Task records are in a separate local file and survive job-template changes.
fn radio_preset_store(store: &WorkbenchStore) -> RadioPresetStore {
    RadioPresetStore::new(
        store
            .path()
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf(),
    )
}

fn radio_snapshot(engine: Option<&NativeRadioEngine>, presets: &RadioPresets) -> Value {
    let selected_output = RADIO_OUTPUT
        .lock()
        .map(|id| id.clone())
        .unwrap_or_else(|_| "system".into());
    let outputs = available_outputs().unwrap_or_default();
    let output_available = outputs
        .iter()
        .any(|device| device.id == selected_output && device.available);
    let devices: Vec<Value> = outputs
        .iter()
        .map(|device| {
            json!({
                "id": device.id,
                "name": device.name,
                "default": device.is_default,
                "available": device.available,
            })
        })
        .collect();
    let stations: Vec<Value> = curated_stations()
        .iter()
        .map(|s| {
            json!({"id": s.id, "name": s.name, "genre": s.genre,
                "category": s.category, "format": s.format,
                "nominal_kbps": s.nominal_kbps})
        })
        .collect();
    match engine {
        Some(player) => {
            let state = player.status();
            json!({
                "source": "native_ip",
                "status": format!("{:?}", state.state).to_ascii_lowercase(),
                "station": state.station_id,
                "volume": state.volume_percent,
                "error": state.last_error,
                "stations": stations,
                "presets": presets,
                "sdr": "not_connected",
                "outputs": devices,
                "output": selected_output,
                "output_available": output_available,
                "output_failure": player.output_failed()
            })
        }
        None => json!({
            "source": "none", "status": "stopped",
            "station": null, "volume": 60, "error": null,
            "stations": stations, "presets": presets,
            "sdr": "not_connected",
            "outputs": devices, "output": selected_output,
            "output_available": output_available, "output_failure": false
        }),
    }
}

fn radio_state(store: &WorkbenchStore) -> Response {
    let presets = match radio_preset_store(store).load() {
        Ok(presets) => presets,
        Err(_) => return Response::error(409, "Local radio presets unavailable; file preserved"),
    };
    match RADIO_ENGINE.lock() {
        Ok(lock) => Response::json(200, &radio_snapshot(lock.as_ref(), &presets)),
        Err(_) => Response::error(500, "Radio controller unavailable"),
    }
}

fn radio_command(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    let body = parse_json_body(
        req,
        &[
            "action",
            "station",
            "volume",
            "confirm",
            "output",
            "enabled",
            "direction",
        ],
    )?;
    let action = str_field(&body, "action")?;
    let presets = radio_preset_store(store);
    let mut engine = RADIO_ENGINE
        .lock()
        .map_err(|_| Response::error(500, "Radio controller unavailable"))?;
    match action {
        "select" => {
            let station = str_field(&body, "station")?;
            if engine.as_ref().is_some_and(|current| {
                !matches!(
                    current.status().state,
                    pipboy_core::radio_controller::PlaybackState::Failed
                        | pipboy_core::radio_controller::PlaybackState::Stopped
                )
            }) {
                return Err(Response::error(
                    409,
                    "Stop the active station before selecting",
                ));
            }
            presets
                .select(station)
                .map_err(|error| match error.kind() {
                    io::ErrorKind::InvalidInput => Response::error(422, "Unknown curated station"),
                    _ => Response::error(409, "Cannot save radio selection"),
                })?;
        }
        "favorite" => {
            let station = str_field(&body, "station")?;
            let enabled = body
                .get("enabled")
                .and_then(Value::as_bool)
                .ok_or_else(|| Response::error(422, "Favorite requires boolean enabled"))?;
            presets
                .favorite(station, enabled)
                .map_err(|error| match error.kind() {
                    io::ErrorKind::InvalidInput => Response::error(422, "Invalid radio favorite"),
                    _ => Response::error(409, "Cannot save radio favorite"),
                })?;
        }
        "move_favorite" => {
            let station = str_field(&body, "station")?;
            let direction = match str_field(&body, "direction")? {
                "earlier" => MoveDirection::Earlier,
                "later" => MoveDirection::Later,
                _ => return Err(Response::error(422, "Direction must be earlier or later")),
            };
            presets
                .move_favorite(station, direction)
                .map_err(|error| match error.kind() {
                    io::ErrorKind::InvalidInput => Response::error(422, "Invalid radio favorite"),
                    _ => Response::error(409, "Cannot reorder radio favorites"),
                })?;
        }
        "play" => {
            if body.get("confirm").and_then(Value::as_bool) != Some(true) {
                return Err(Response::error(
                    422,
                    "Radio playback requires explicit confirmation",
                ));
            }
            let station = str_field(&body, "station")?;
            if !curated_stations().iter().any(|s| s.id == station) {
                return Err(Response::error(422, "Unknown curated radio station"));
            }
            if engine.as_ref().is_some_and(|current| {
                !matches!(
                    current.status().state,
                    pipboy_core::radio_controller::PlaybackState::Failed
                        | pipboy_core::radio_controller::PlaybackState::Stopped
                )
            }) {
                return Err(Response::error(
                    409,
                    "Stop the active station before switching",
                ));
            }
            // Save last selected *before* starting native I/O. A corrupt or
            // unavailable preferences file must not be silently overwritten.
            presets
                .select(station)
                .map_err(|_| Response::error(409, "Cannot save radio selection"))?;
            // Stop old failed session before opening a replacement. The native
            // decoder owns audio; no external player or web media element.
            if let Some(mut old) = engine.take() {
                old.stop();
            }
            *engine = Some(
                NativeRadioEngine::start_internet(
                    station,
                    60,
                    &RADIO_OUTPUT
                        .lock()
                        .map_err(|_| Response::error(500, "Output controller unavailable"))?,
                )
                .map_err(|_| Response::error(409, "Cannot start native radio worker"))?,
            );
        }
        "stop" => {
            if let Some(mut old) = engine.take() {
                old.stop();
            }
        }
        "output" => {
            let output_id = str_field(&body, "output")?;
            if !available_outputs()
                .map_err(|_| Response::error(409, "Audio outputs unavailable"))?
                .iter()
                .any(|device| device.id == output_id && device.available)
            {
                return Err(Response::error(
                    422,
                    "Output disconnected; refresh audio device list",
                ));
            }
            // Changing hardware output must be explicitly requested. Restart
            // the same stream/volume using the newly selected native endpoint.
            let previous = engine.as_ref().map(|player| player.status());
            let recover_failed_output = engine
                .as_ref()
                .is_some_and(NativeRadioEngine::output_failed);
            if let Some(mut active) = engine.take() {
                active.stop();
            }
            *RADIO_OUTPUT
                .lock()
                .map_err(|_| Response::error(500, "Output controller unavailable"))? =
                output_id.to_owned();
            if let Some(state) = previous {
                if pipboy_core::radio_recovery::restart_after_output_change(
                    state.state,
                    recover_failed_output,
                ) {
                    if let Some(station) = state.station_id {
                        *engine = Some(
                            NativeRadioEngine::start_internet(
                                &station,
                                state.volume_percent,
                                output_id,
                            )
                            .map_err(|_| {
                                Response::error(409, "Cannot restart radio on selected output")
                            })?,
                        );
                    }
                }
            }
        }
        "volume" => {
            let volume = body
                .get("volume")
                .and_then(Value::as_u64)
                .filter(|n| *n <= 100)
                .ok_or_else(|| Response::error(422, "Volume must be between 0 and 100"))?;
            let player = engine
                .as_ref()
                .ok_or_else(|| Response::error(409, "Start a station before changing volume"))?;
            player
                .set_volume(volume as u8)
                .map_err(|_| Response::error(409, "Cannot change radio volume"))?;
        }
        _ => return Err(Response::error(422, "Unknown radio command")),
    }
    let saved = presets
        .load()
        .map_err(|_| Response::error(409, "Local radio presets unavailable; file preserved"))?;
    Ok(Response::json(
        200,
        &radio_snapshot(engine.as_ref(), &saved),
    ))
}

fn dispatch(req: &Request, store: &WorkbenchStore, port: u16) -> Response {
    let expected_host = format!("127.0.0.1:{port}");
    if req.header("host") != Some(expected_host.as_str()) {
        return Response::error(403, "Only the exact loopback host is allowed");
    }
    if req.method == "POST" {
        let expected_origin = format!("http://{expected_host}");
        if req.header("origin") != Some(expected_origin.as_str()) {
            return Response::error(403, "Mutations require same-origin browser requests");
        }
    }
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/") => Response::new(200, "text/html; charset=utf-8", HTML.as_bytes()),
        ("GET", "/micro") | ("GET", "/micro/") => {
            Response::new(200, "text/html; charset=utf-8", MICRO_HTML.as_bytes())
        }
        ("GET", "/assets/micro.css") => {
            Response::new(200, "text/css; charset=utf-8", MICRO_CSS.as_bytes())
        }
        ("GET", "/assets/radio-ui.js") => Response::new(
            200,
            "text/javascript; charset=utf-8",
            RADIO_UI_JS.as_bytes(),
        ),
        ("GET", "/assets/radio-ui.css") => {
            Response::new(200, "text/css; charset=utf-8", RADIO_UI_CSS.as_bytes())
        }
        ("GET", "/assets/cameras.js") => {
            Response::new(200, "text/javascript; charset=utf-8", CAMERAS_JS.as_bytes())
        }
        ("GET", "/assets/cameras.css") => {
            Response::new(200, "text/css; charset=utf-8", CAMERAS_CSS.as_bytes())
        }
        ("GET", "/assets/micro.js") => {
            Response::new(200, "text/javascript; charset=utf-8", MICRO_JS.as_bytes())
        }
        ("GET", "/assets/map-import.js") => Response::new(
            200,
            "text/javascript; charset=utf-8",
            MAP_IMPORT_JS.as_bytes(),
        ),
        ("GET", "/assets/map-online.js") => Response::new(
            200,
            "text/javascript; charset=utf-8",
            MAP_ONLINE_JS.as_bytes(),
        ),
        ("GET", "/assets/map-county.js") => {
            Response::new(200, "text/javascript; charset=utf-8", COUNTY_JS.as_bytes())
        }
        ("GET", "/assets/map-zip.js") => {
            Response::new(200, "text/javascript; charset=utf-8", ZIP_JS.as_bytes())
        }
        ("GET", "/assets/map.js") => {
            Response::new(200, "text/javascript; charset=utf-8", MAP_JS.as_bytes())
        }
        ("GET", "/assets/map.css") => {
            Response::new(200, "text/css; charset=utf-8", MAP_CSS.as_bytes())
        }
        ("GET", "/assets/messenger.js") => Response::new(
            200,
            "text/javascript; charset=utf-8",
            MESSENGER_JS.as_bytes(),
        ),
        ("GET", "/assets/messenger.css") => {
            Response::new(200, "text/css; charset=utf-8", MESSENGER_CSS.as_bytes())
        }
        ("GET", "/assets/tasks.js") => {
            Response::new(200, "text/javascript; charset=utf-8", TASKS_JS.as_bytes())
        }
        ("GET", "/assets/network.js") => {
            Response::new(200, "text/javascript; charset=utf-8", NETWORK_JS.as_bytes())
        }
        ("GET", "/assets/network.css") => {
            Response::new(200, "text/css; charset=utf-8", NETWORK_CSS.as_bytes())
        }
        ("GET", "/assets/reclamation.js") => Response::new(
            200,
            "text/javascript; charset=utf-8",
            RECLAMATION_JS.as_bytes(),
        ),
        ("GET", "/assets/reclamation.css") => {
            Response::new(200, "text/css; charset=utf-8", RECLAMATION_CSS.as_bytes())
        }
        ("GET", "/assets/tasks.css") => {
            Response::new(200, "text/css; charset=utf-8", TASKS_CSS.as_bytes())
        }
        ("GET", "/assets/dashboard.css") => {
            Response::new(200, "text/css; charset=utf-8", CSS.as_bytes())
        }
        ("GET", "/assets/dashboard.js") => {
            Response::new(200, "text/javascript; charset=utf-8", JS.as_bytes())
        }
        ("GET", "/favicon.ico") => Response::new(204, "text/plain; charset=utf-8", Vec::new()),
        ("GET", "/api/map/sources") => Response::json(200, &county_cache::sources()),
        ("GET", "/api/map/cache") => county_cache(store)
            .active()
            .map(|value| Response::json(200, &value))
            .unwrap_or_else(|_| Response::error(409, "County cache unavailable")),
        ("POST", "/api/map/package") => county_package(req, store).unwrap_or_else(|error| error),
        ("POST", "/api/map/boundary") => county_boundary(req, store).unwrap_or_else(|error| error),
        ("POST", "/api/map/zip") => zip_lookup(req, store).unwrap_or_else(|error| error),
        ("POST", "/api/map/county-zips") => county_zips(req, store).unwrap_or_else(|error| error),
        ("GET", "/api/state") => state(store).unwrap_or_else(|error| error),
        ("GET", "/api/radio") => radio_state(store),
        ("POST", "/api/radio") => radio_command(req, store).unwrap_or_else(|error| error),
        ("GET", "/api/reclamation") => reclamation_read(store, false).unwrap_or_else(|error| error),
        ("GET", "/api/reclamation/map") => {
            reclamation_map_read(store).unwrap_or_else(|error| error)
        }
        ("GET", "/api/reclamation/geojson") => {
            reclamation_read(store, true).unwrap_or_else(|error| error)
        }
        ("POST", "/api/reclamation/create") => {
            reclamation_create(req, store).unwrap_or_else(|error| error)
        }
        ("POST", "/api/reclamation/change") => {
            reclamation_change(req, store).unwrap_or_else(|error| error)
        }
        ("GET", "/api/tasks") => task_routes::tasks_read(store).unwrap_or_else(|error| error),
        ("GET", "/api/peers") => task_routes::peers_read(store).unwrap_or_else(|error| error),
        ("GET", "/api/messages") => {
            messenger_routes::messages_read(store).unwrap_or_else(|error| error)
        }
        ("POST", "/api/messages/send") => {
            messenger_routes::messages_send(req, store).unwrap_or_else(|error| error)
        }
        ("POST", "/api/messages/retry") => {
            messenger_routes::messages_retry(req, store).unwrap_or_else(|error| error)
        }
        ("POST", "/api/net/scan") => {
            network_routes::scan_network(req, store).unwrap_or_else(|error| error)
        }
        ("POST", "/api/net/unifi") => {
            network_routes::unifi_request(req, store).unwrap_or_else(|error| error)
        }
        ("POST", "/api/peer/send") => {
            task_routes::device_send(req, store).unwrap_or_else(|error| error)
        }
        ("POST", "/api/peer/check") => {
            task_routes::device_check(req, store).unwrap_or_else(|error| error)
        }
        ("POST", "/api/peer/sync") => {
            task_routes::device_sync(req, store).unwrap_or_else(|error| error)
        }
        ("POST", "/api/tasks/create") => {
            task_routes::tasks_create(req, store).unwrap_or_else(|error| error)
        }
        ("POST", "/api/tasks/change") => {
            task_routes::tasks_change(req, store).unwrap_or_else(|error| error)
        }
        ("POST", "/api/tasks/handoff") => {
            task_routes::tasks_handoff(req, store).unwrap_or_else(|error| error)
        }
        ("POST", "/api/profile") => {
            let result = (|| {
                let body = parse_json_body(req, &["id", "confirm"])?;
                if body.get("confirm").and_then(Value::as_bool) != Some(true) {
                    return Err(Response::error(
                        422,
                        "Confirm profile replacement explicitly",
                    ));
                }
                let id = str_field(&body, "id")?;
                if !PRESETS.iter().any(|profile| profile.id == id) {
                    return Err(Response::error(422, "Unknown role profile"));
                }
                store.apply_preset(id).map_err(|_| {
                    Response::error(409, "Cannot replace workbench; original preserved")
                })?;
                state(store)
            })();
            result.unwrap_or_else(|error| error)
        }
        ("POST", "/api/custom/add") => {
            let result = (|| {
                let body = parse_json_body(req, &["id"])?;
                let id = str_field(&body, "id")?;
                if store
                    .load()
                    .map_err(|_| Response::error(409, "Workbench unavailable"))?
                    .profile
                    != "custom"
                {
                    return Err(Response::error(409, "Select Custom before pinning modules"));
                }
                let candidate = tool(id).ok_or_else(|| Response::error(422, "Unknown tool"))?;
                if !candidate.availability.can_pin() {
                    return Err(Response::error(
                        422,
                        "Unimplemented hardware tools cannot be pinned",
                    ));
                }
                store.add(id).map_err(|_| {
                    Response::error(409, "Tool already pinned or storage unavailable")
                })?;
                state(store)
            })();
            result.unwrap_or_else(|error| error)
        }
        ("POST", "/api/custom/remove") => {
            let result = (|| {
                let body = parse_json_body(req, &["id"])?;
                let id = body
                    .get("id")
                    .and_then(Value::as_u64)
                    .filter(|value| *value != 0)
                    .ok_or_else(|| Response::error(422, "Invalid slot number"))?;
                if store
                    .load()
                    .map_err(|_| Response::error(409, "Workbench unavailable"))?
                    .profile
                    != "custom"
                {
                    return Err(Response::error(409, "Select Custom before removing a slot"));
                }
                store
                    .remove(id)
                    .map_err(|_| Response::error(409, "Cannot remove slot"))?;
                state(store)
            })();
            result.unwrap_or_else(|error| error)
        }
        ("POST", "/api/experiment") => {
            let result = (|| {
                let body = parse_json_body(
                    req,
                    &[
                        "tool", "mode", "input", "extra", "samples", "confirm", "lat", "lon",
                        "lat2", "lon2", "name",
                    ],
                )?;
                experiment(&body, store)
            })();
            result.unwrap_or_else(|error| error)
        }
        ("GET", _) | ("POST", _) => Response::error(404, "Unknown dashboard endpoint"),
        _ => Response::error(405, "Unsupported method"),
    }
}

fn serve_client(mut stream: TcpStream, store: &WorkbenchStore, port: u16) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;
    let response = match receive(&mut stream) {
        Ok(request) => dispatch(&request, store, port),
        Err(response) => response,
    };
    response.write_to(&mut stream)
}

#[cfg(target_os = "macos")]
fn open_browser(url: &str) {
    let mut command = pipboy_core::process_safety::isolated_command("open");
    if let Err(error) = command.arg(url).spawn() {
        eprintln!("Could not open the browser automatically: {error}");
    }
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let usage = "usage: pipboy dashboard [--port 0..65535] [--open (macOS)] [--micro]";
    let mut port = DEFAULT_PORT;
    let mut open = false;
    let mut micro = false;
    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--port" => port = iter.next().ok_or(usage)?.parse::<u16>()?,
            "--open" if !open => open = true,
            "--micro" if !micro => micro = true,
            _ => return Err(usage.into()),
        }
    }
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
    let port = listener.local_addr()?.port();
    let url = if micro {
        format!("http://127.0.0.1:{port}/micro")
    } else {
        format!("http://127.0.0.1:{port}/")
    };
    println!("RUST-BOY DASHBOARD / LOCAL ONLY");
    println!("Open {url}");
    println!("Eight job templates, Custom last; live hardware and RustGIS are not connected.");
    println!("Press Ctrl+C in this terminal to stop the dashboard.");
    io::stdout().flush()?;
    #[cfg(target_os = "macos")]
    if open {
        open_browser(&url);
    }
    #[cfg(not(target_os = "macos"))]
    if open {
        eprintln!("--open is Mac-only; open the printed URL manually.");
    }
    // A network sweep can take seconds. Keep the local UI responsive while
    // bounding concurrent loopback requests to avoid unbounded thread creation.
    const MAX_CLIENTS: usize = 12;
    let store = Arc::new(WorkbenchStore::from_environment()?);
    let active_clients = Arc::new(AtomicUsize::new(0));
    for connection in listener.incoming() {
        match connection {
            Ok(mut stream) => {
                if active_clients.fetch_add(1, Ordering::SeqCst) >= MAX_CLIENTS {
                    active_clients.fetch_sub(1, Ordering::SeqCst);
                    let _ = Response::error(409, "Local dashboard is busy").write_to(&mut stream);
                    continue;
                }
                let store = Arc::clone(&store);
                let active = Arc::clone(&active_clients);
                std::thread::spawn(move || {
                    if let Err(error) = serve_client(stream, &store, port) {
                        eprintln!("Dashboard request ended: {error}");
                    }
                    active.fetch_sub(1, Ordering::SeqCst);
                });
            }
            Err(error) => eprintln!("Dashboard connection refused: {error}"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::task_routes::task_store;
    use super::*;
    use pipboy_core::tasks::Status;
    use std::fs;

    fn request(method: &str, path: &str, body: Value) -> Request {
        let mut headers = BTreeMap::new();
        headers.insert("host".to_owned(), "127.0.0.1:8765".to_owned());
        if method == "POST" {
            headers.insert("origin".to_owned(), "http://127.0.0.1:8765".to_owned());
            headers.insert("content-type".to_owned(), "application/json".to_owned());
        }
        Request {
            method: method.to_owned(),
            path: path.to_owned(),
            headers,
            body: if method == "POST" {
                serde_json::to_vec(&body).unwrap()
            } else {
                Vec::new()
            },
        }
    }

    #[test]
    fn radio_hud_endpoints_require_same_origin_and_valid_commands() {
        let temp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temp.path().join("radio-hud"));
        for asset in ["/assets/radio-ui.js", "/assets/radio-ui.css"] {
            assert_eq!(
                dispatch(&request("GET", asset, Value::Null), &store, 8765).status,
                200
            );
        }
        let state = dispatch(&request("GET", "/api/radio", Value::Null), &store, 8765);
        assert_eq!(state.status, 200);
        let json: Value = serde_json::from_slice(&state.body).unwrap();
        assert_eq!(json["presets"]["favorites"], json!([]));
        assert_eq!(json["presets"]["last_selected"], Value::Null);
        assert!(!radio_preset_store(&store).path().exists());
        let station_list = json["stations"].as_array().expect("radio directory");
        assert_eq!(station_list.len(), 10);
        assert_eq!(station_list[0]["id"], "skylab");
        assert_eq!(station_list[0]["category"], "CHILL");
        assert_eq!(station_list[8]["id"], "defcon");
        assert_eq!(station_list[8]["nominal_kbps"], 128);
        assert_eq!(json["sdr"], "not_connected");
        assert_eq!(json["output"], "system");
        assert!(json["outputs"].is_array());
        assert!(json["output_available"].is_boolean());
        assert_eq!(json["output_failure"], false);
        assert!(json["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|device| { device["id"].is_string() && device["available"].is_boolean() }));
        let invalid = request(
            "POST",
            "/api/radio",
            json!({"action":"play", "station":"unknown", "confirm":true}),
        );
        assert_eq!(dispatch(&invalid, &store, 8765).status, 422);
        let absent = request(
            "POST",
            "/api/radio",
            json!({"action":"play", "station":"skylab"}),
        );
        assert_eq!(dispatch(&absent, &store, 8765).status, 422);
        let unsafe_volume = request(
            "POST",
            "/api/radio",
            json!({"action":"volume", "volume":101}),
        );
        assert_eq!(dispatch(&unsafe_volume, &store, 8765).status, 422);
        let station = request(
            "POST",
            "/api/radio",
            json!({"action":"select", "station":"skylab"}),
        );
        let selection = dispatch(&station, &store, 8765);
        assert_eq!(selection.status, 200);
        let selected: Value = serde_json::from_slice(&selection.body).unwrap();
        assert_eq!(selected["status"], "stopped");
        assert_eq!(selected["presets"]["last_selected"], "skylab");
        assert_eq!(selected["presets"]["favorites"], json!([]));
        for station in ["skylab", "dronezone"] {
            let add = request(
                "POST",
                "/api/radio",
                json!({"action":"favorite", "station":station, "enabled":true}),
            );
            assert_eq!(dispatch(&add, &store, 8765).status, 200);
        }
        let move_preset = request(
            "POST",
            "/api/radio",
            json!({"action":"move_favorite", "station":"dronezone", "direction":"earlier"}),
        );
        let moved = dispatch(&move_preset, &store, 8765);
        assert_eq!(moved.status, 200);
        let body: Value = serde_json::from_slice(&moved.body).unwrap();
        assert_eq!(body["presets"]["favorites"], json!(["dronezone", "skylab"]));
        assert_eq!(body["presets"]["last_selected"], "skylab");
        let reloaded = WorkbenchStore::new(temp.path().join("radio-hud"));
        let persisted = dispatch(&request("GET", "/api/radio", Value::Null), &reloaded, 8765);
        let persisted: Value = serde_json::from_slice(&persisted.body).unwrap();
        assert_eq!(
            persisted["presets"]["favorites"],
            json!(["dronezone", "skylab"])
        );
        assert_eq!(persisted["status"], "stopped"); // No autoplay on reading presets.
        for bad in [
            json!({"action":"select","station":"https://evil.test/audio"}),
            json!({"action":"favorite","station":"unknown","enabled":true}),
            json!({"action":"favorite","station":"skylab","enabled":"true"}),
            json!({"action":"move_favorite","station":"skylab","direction":"up"}),
            json!({"action":"move_favorite","station":"secretagent","direction":"earlier"}),
        ] {
            assert_eq!(
                dispatch(&request("POST", "/api/radio", bad), &store, 8765).status,
                422
            );
        }
        let mut cross_origin = request("POST", "/api/radio", json!({"action":"stop"}));
        cross_origin
            .headers
            .insert("origin".into(), "http://evil.example".into());
        assert_eq!(dispatch(&cross_origin, &store, 8765).status, 403);
        let stop = request("POST", "/api/radio", json!({"action":"stop"}));
        assert_eq!(dispatch(&stop, &store, 8765).status, 200);
        assert!(RADIO_UI_JS.contains("/api/radio"));
        assert!(MICRO_JS.contains("native-radio"));
        assert!(HTML.contains("radio-dock"));
    }

    #[test]
    fn corrupt_radio_presets_are_not_silently_reset() {
        let temp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temp.path().to_path_buf());
        let pref_path = radio_preset_store(&store).path().to_path_buf();
        let corrupted = br#"{"schema_version":1,"favorites":["unapproved"],"last_selected":null}"#;
        std::fs::write(&pref_path, corrupted).unwrap();
        assert_eq!(
            dispatch(&request("GET", "/api/radio", Value::Null), &store, 8765).status,
            409
        );
        let mutation = request(
            "POST",
            "/api/radio",
            json!({"action":"favorite","station":"skylab","enabled":true}),
        );
        assert_eq!(dispatch(&mutation, &store, 8765).status, 409);
        assert_eq!(std::fs::read(&pref_path).unwrap(), corrupted);
    }

    #[test]
    fn all_profiles_are_loaded_from_rust_templates_and_custom_is_last() {
        let temp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temp.path().join("fresh"));
        let response = dispatch(&request("GET", "/api/state", Value::Null), &store, 8765);
        assert_eq!(response.status, 200);
        let state: Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(state["active"]["profile"], "overseer");
        assert_eq!(state["profiles"].as_array().unwrap().len(), 9);
        assert_eq!(state["profiles"][8]["id"], "custom");
        assert_eq!(state["profiles"][8]["tools"], json!([]));
        assert_eq!(state["engine"]["rustgis"], "not_integrated");
        assert_eq!(state["camera_feeds"].as_array().unwrap().len(), 4);
        assert_eq!(state["engine"]["private_cameras"], "not_configured");
        assert!(!temp.path().join("fresh").exists());
        for endpoint in [
            "/",
            "/assets/cameras.js",
            "/assets/cameras.css",
            "/assets/tasks.css",
            "/assets/tasks.js",
            "/assets/dashboard.css",
            "/assets/dashboard.js",
            "/micro",
            "/micro/",
            "/assets/micro.css",
            "/assets/micro.js",
            "/assets/map.js",
            "/assets/map-import.js",
            "/assets/map-online.js",
            "/assets/map.css",
        ] {
            let r = dispatch(&request("GET", endpoint, Value::Null), &store, 8765);
            assert_eq!(r.status, 200);
            assert!(!r.body.is_empty());
        }
    }

    #[test]
    fn cross_origin_and_unknown_host_cannot_write_workbench() {
        let temp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temp.path().join("state"));
        let mut req = request(
            "POST",
            "/api/profile",
            json!({"id":"custom","confirm":true}),
        );
        req.headers
            .insert("origin".to_owned(), "https://example.invalid".to_owned());
        assert_eq!(dispatch(&req, &store, 8765).status, 403);
        req.headers
            .insert("origin".to_owned(), "http://127.0.0.1:8765".to_owned());
        req.headers
            .insert("host".to_owned(), "attacker.local".to_owned());
        assert_eq!(dispatch(&req, &store, 8765).status, 403);
        req.headers
            .insert("host".to_owned(), "127.0.0.1:8765".to_owned());
        req.headers.remove("content-type");
        assert_eq!(dispatch(&req, &store, 8765).status, 415);
        let wrong = request("POST", "/api/profile", json!({"id":"custom"}));
        assert_eq!(dispatch(&wrong, &store, 8765).status, 422);
        assert!(!temp.path().join("state").exists());
    }

    #[test]
    fn lan_scan_requires_pinned_tool_consent_and_private_scope() {
        let temp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temp.path().join("state"));
        let scoped = request(
            "POST",
            "/api/net/scan",
            json!({"cidr":"192.168.1.0/24","confirm":true}),
        );
        assert_eq!(dispatch(&scoped, &store, 8765).status, 409);
        assert_eq!(
            dispatch(
                &request(
                    "POST",
                    "/api/profile",
                    json!({"id":"head-engineer","confirm":true})
                ),
                &store,
                8765
            )
            .status,
            200
        );
        assert_eq!(
            dispatch(
                &request("POST", "/api/net/scan", json!({"cidr":"192.168.1.0/24"})),
                &store,
                8765
            )
            .status,
            422
        );
        assert_eq!(
            dispatch(
                &request(
                    "POST",
                    "/api/net/scan",
                    json!({"cidr":"8.8.8.8/32","confirm":true})
                ),
                &store,
                8765
            )
            .status,
            422
        );
        assert_eq!(
            dispatch(
                &request(
                    "POST",
                    "/api/net/scan",
                    json!({"cidr":"192.168.1.0/16","confirm":true})
                ),
                &store,
                8765
            )
            .status,
            422
        );
        let mut cross_origin = scoped;
        cross_origin
            .headers
            .insert("origin".into(), "https://other.invalid".into());
        assert_eq!(dispatch(&cross_origin, &store, 8765).status, 403);
    }

    #[test]
    fn unifi_inventory_denies_without_role_or_operator_permission() {
        let temp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temp.path().join("state"));
        let clients = request(
            "POST",
            "/api/net/unifi",
            json!({"operation":"clients","site":"00000000-0000-0000-0000-000000000001","confirm":true}),
        );
        assert_eq!(dispatch(&clients, &store, 8765).status, 409);
        assert_eq!(
            dispatch(
                &request(
                    "POST",
                    "/api/profile",
                    json!({"id":"head-engineer","confirm":true})
                ),
                &store,
                8765
            )
            .status,
            200
        );
        assert_eq!(
            dispatch(
                &request(
                    "POST",
                    "/api/net/unifi",
                    json!({"operation":"clients","site":"00000000-0000-0000-0000-000000000001"})
                ),
                &store,
                8765
            )
            .status,
            422
        );
        assert_eq!(
            dispatch(
                &request(
                    "POST",
                    "/api/net/unifi",
                    json!({"operation":"clients","site":"../../secret","confirm":true})
                ),
                &store,
                8765
            )
            .status,
            422
        );
        assert_eq!(
            dispatch(
                &request(
                    "POST",
                    "/api/net/unifi",
                    json!({"operation":"compare","site":"00000000-0000-0000-0000-000000000001",
                "cidr":"8.8.8.8/32","confirm":true})
                ),
                &store,
                8765
            )
            .status,
            422
        );
        assert!(!temp.path().join("state/permissions.json").exists());
    }

    #[test]
    fn role_activation_custom_pinning_and_offline_lab_work_end_to_end() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("state");
        let store = WorkbenchStore::new(root.clone());
        let apply = request(
            "POST",
            "/api/profile",
            json!({"id":"reclamation-scout","confirm":true}),
        );
        assert_eq!(dispatch(&apply, &store, 8765).status, 200);
        assert_eq!(store.load().unwrap().slots[0].tool, "reclamation");
        assert_eq!(store.load().unwrap().slots[1].tool, "geo-lab");
        let lab = request(
            "POST",
            "/api/experiment",
            json!({"tool":"geo-lab","mode":"waypoint","name":"Camp","lat":"42","lon":"-70.75"}),
        );
        let output = dispatch(&lab, &store, 8765);
        assert_eq!(output.status, 200);
        assert!(String::from_utf8_lossy(&output.body).contains("-70.75"));
        let current = request(
            "POST",
            "/api/profile",
            json!({"id":"custom","confirm":true}),
        );
        assert_eq!(dispatch(&current, &store, 8765).status, 200);
        assert!(store.load().unwrap().slots.is_empty());
        assert_eq!(dispatch(&lab, &store, 8765).status, 409);
        let add = request("POST", "/api/custom/add", json!({"id":"net-lab"}));
        assert_eq!(dispatch(&add, &store, 8765).status, 200);
        let net = request(
            "POST",
            "/api/experiment",
            json!({"tool":"net-lab","input":"192.168.1.50/24"}),
        );
        assert!(
            String::from_utf8_lossy(&dispatch(&net, &store, 8765).body).contains("192.168.1.0/24")
        );
        let remove = request("POST", "/api/custom/remove", json!({"id":1}));
        assert_eq!(dispatch(&remove, &store, 8765).status, 200);
        assert!(store.load().unwrap().slots.is_empty());
        assert!(!root.join("notes.json").exists());
        assert!(!root.join("permissions.json").exists());
    }

    #[test]
    fn no_arbitrary_commands_hardware_endpoints_or_saved_secrets() {
        let temp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temp.path().join("state"));
        for (path, body) in [
            ("/api/custom/add", json!({"id":"radio"})),
            ("/api/custom/add", json!({"id":"geo-lab"})),
            ("/api/shell", json!({"cmd":"echo hi"})),
        ] {
            assert_ne!(
                dispatch(&request("POST", path, body), &store, 8765).status,
                200
            );
        }
        // Overseer has a Game Link tile. Its safe handoff is intentionally
        // informational (200), but never opens a LAN/console connection.
        let game = dispatch(
            &request("POST", "/api/experiment", json!({"tool":"game"})),
            &store,
            8765,
        );
        assert_eq!(game.status, 200);
        let handoff: Value = serde_json::from_slice(&game.body).unwrap();
        assert_eq!(handoff["kind"], "information");
        assert!(handoff["output"]
            .as_str()
            .unwrap()
            .contains("does not connect"));
        assert_eq!(
            dispatch(&request("GET", "/api/unknown", Value::Null), &store, 8765).status,
            404
        );
        assert!(!temp.path().join("state").exists());
        let secret = "Bearer random-secret";
        let malformed = request("POST", "/api/profile", json!({"id":secret,"confirm":true}));
        let response = dispatch(&malformed, &store, 8765);
        assert_eq!(response.status, 422);
        assert!(!String::from_utf8_lossy(&response.body).contains(secret));
    }

    #[test]
    fn malformed_existing_store_is_preserved_on_dashboard_mutations() {
        let temp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temp.path().to_path_buf());
        fs::write(store.path(), "invalid").unwrap();
        let req = request(
            "POST",
            "/api/profile",
            json!({"id":"overseer","confirm":true}),
        );
        assert_eq!(dispatch(&req, &store, 8765).status, 409);
        assert_eq!(fs::read_to_string(store.path()).unwrap(), "invalid");
    }

    #[test]
    fn manager_can_create_assign_dispatch_and_track_but_other_roles_cannot_reassign() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("operator");
        let store = WorkbenchStore::new(root.clone());
        let empty = dispatch(&request("GET", "/api/tasks", Value::Null), &store, 8765);
        assert_eq!(empty.status, 200);
        assert!(!root.exists());
        let make = request(
            "POST",
            "/api/tasks/create",
            json!({
                "title": "Check radio room",
                "details": "Record any unusual noise",
                "assignee": "Engineer One",
                "role": "head-engineer",
                "contact": "engineer@example.org",
                "priority": "high",
                "due": "2026-11-03"
            }),
        );
        let created = dispatch(&make, &store, 8765);
        assert_eq!(created.status, 200);
        let data: Value = serde_json::from_slice(&created.body).unwrap();
        assert_eq!(data["task"]["revision"], 1);
        assert_eq!(data["sent"], false);
        let handoff = dispatch(
            &request("POST", "/api/tasks/handoff", json!({"id": 1})),
            &store,
            8765,
        );
        assert_eq!(handoff.status, 200);
        let content: Value = serde_json::from_slice(&handoff.body).unwrap();
        assert_eq!(content["sent"], false);
        assert!(content["text"].as_str().unwrap().contains("RB-0001"));
        assert_eq!(
            dispatch(
                &request(
                    "POST",
                    "/api/tasks/change",
                    json!({"id":1,"revision":1,"operation":"acknowledge"})
                ),
                &store,
                8765,
            )
            .status,
            422
        );
        assert_eq!(
            dispatch(
                &request(
                    "POST",
                    "/api/tasks/change",
                    json!({"id":1,"revision":1,"operation":"sent"})
                ),
                &store,
                8765,
            )
            .status,
            200
        );
        assert_eq!(
            dispatch(
                &request(
                    "POST",
                    "/api/tasks/change",
                    json!({"id":1,"revision":1,"operation":"sent"})
                ),
                &store,
                8765,
            )
            .status,
            409
        );
        store.apply_preset("head-engineer").unwrap();
        let jobs = dispatch(&request("GET", "/api/tasks", Value::Null), &store, 8765);
        let assigned: Value = serde_json::from_slice(&jobs.body).unwrap();
        assert_eq!(assigned["board"]["tasks"][0]["role"], "head-engineer");
        assert_eq!(assigned["manager"], false);
        assert_eq!(dispatch(&make, &store, 8765).status, 403);
        assert_eq!(
            dispatch(
                &request(
                    "POST",
                    "/api/tasks/change",
                    json!({"id":1,"revision":2,"operation":"reassign",
                    "assignee":"Somebody", "role":"overseer"})
                ),
                &store,
                8765,
            )
            .status,
            403
        );
        assert_eq!(
            dispatch(
                &request(
                    "POST",
                    "/api/tasks/change",
                    json!({"id":1,"revision":2,"operation":"acknowledge"})
                ),
                &store,
                8765,
            )
            .status,
            200
        );
        assert_eq!(
            dispatch(
                &request(
                    "POST",
                    "/api/tasks/change",
                    json!({"id":1,"revision":3,"operation":"status","status":"in_progress",
                    "note":"Inspection underway"})
                ),
                &store,
                8765,
            )
            .status,
            200
        );
        assert_eq!(
            task_store(&store).task(1).unwrap().status,
            Status::InProgress
        );
        assert!(!root.join("permissions.json").exists());
    }

    #[test]
    fn task_mutations_require_same_origin_and_do_not_destroy_corrupted_stores() {
        let temp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temp.path().to_path_buf());
        let mut malicious = request(
            "POST",
            "/api/tasks/create",
            json!({
                "title": "Do work", "assignee": "Person", "role": "head-engineer", "priority":"normal"
            }),
        );
        malicious
            .headers
            .insert("origin".to_owned(), "http://attacker.invalid".to_owned());
        assert_eq!(dispatch(&malicious, &store, 8765).status, 403);
        assert!(!task_store(&store).path().exists());
        std::fs::write(task_store(&store).path(), "bad-json").unwrap();
        let valid = request(
            "POST",
            "/api/tasks/create",
            json!({
                "title": "Do work", "assignee": "Person", "role": "head-engineer", "priority":"normal"
            }),
        );
        assert_eq!(dispatch(&valid, &store, 8765).status, 422);
        assert_eq!(
            std::fs::read_to_string(task_store(&store).path()).unwrap(),
            "bad-json"
        );
    }

    #[test]
    fn task_peer_routes_reject_cross_origin_before_any_storage() {
        let dir = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(dir.path().join("private/workbench.json"));
        for path in [
            "/api/tasks/create",
            "/api/tasks/change",
            "/api/tasks/handoff",
            "/api/peer/send",
            "/api/peer/sync",
        ] {
            let mut req = request("POST", path, json!({}));
            req.headers
                .insert("origin".into(), "http://untrusted.invalid".into());
            let response = dispatch(&req, &store, 8765);
            assert_eq!(response.status, 403, "cross-origin route {path}");
            assert!(!dir.path().join("private").exists());
        }
    }

    #[test]
    fn messenger_api_requires_confirmation_and_never_auto_connects() {
        let dir = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(dir.path().join("private/workbench.json"));
        let empty = dispatch(&request("GET", "/api/messages", Value::Null), &store, 8765);
        assert_eq!(empty.status, 200);
        let response: Value = serde_json::from_slice(&empty.body).unwrap();
        assert_eq!(response["messages"], json!([]));
        assert_eq!(response["peers"], json!([]));
        assert_eq!(response["e2ee"], false);
        assert_eq!(response["storage"], "plaintext_local_only");
        assert!(!dir.path().join("private/messages.json").exists());

        for input in [
            json!({"alias":"scout-one","body":"Hi"}),
            json!({"alias":"scout-one","body":"\u{1b}[31m","confirm":true}),
            json!({"alias":"not-paired","body":"Hello","confirm":true}),
            json!({"alias":"scout-one","body":"Hello","confirm":false}),
        ] {
            let response = dispatch(&request("POST", "/api/messages/send", input), &store, 8765);
            assert_ne!(response.status, 200);
            assert!(!dir.path().join("private/messages.json").exists());
        }

        let retry = dispatch(
            &request("POST", "/api/messages/retry", json!({"id":1})),
            &store,
            8765,
        );
        assert_eq!(retry.status, 422);

        let mut cross = request(
            "POST",
            "/api/messages/send",
            json!({"alias":"scout-one","body":"Hello","confirm":true}),
        );
        cross
            .headers
            .insert("origin".into(), "https://hostile.invalid".into());
        assert_eq!(dispatch(&cross, &store, 8765).status, 403);
        assert!(!dir.path().join("private/messages.json").exists());
    }

    #[test]
    fn network_routes_require_same_origin_and_explicit_consent() {
        let temp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temp.path().join("private/workbench.json"));
        for (path, body) in [
            (
                "/api/net/scan",
                json!({"cidr":"192.168.1.0/24","confirm":true}),
            ),
            (
                "/api/net/unifi",
                json!({"operation":"sites","confirm":true}),
            ),
        ] {
            let mut req = request("POST", path, body);
            req.headers
                .insert("origin".into(), "https://untrusted.invalid".into());
            assert_eq!(dispatch(&req, &store, 8765).status, 403, "{path}");
            assert!(!temp.path().join("private").exists());
        }
        for (path, body) in [
            ("/api/net/scan", json!({"cidr":"192.168.1.0/24"})),
            ("/api/net/unifi", json!({"operation":"sites"})),
        ] {
            assert_eq!(
                dispatch(&request("POST", path, body), &store, 8765).status,
                422,
                "missing confirmation: {path}"
            );
            assert!(!temp.path().join("private").exists());
        }
        let mut invalid_type = request(
            "POST",
            "/api/net/scan",
            json!({"cidr":"192.168.1.0/24","confirm":true}),
        );
        invalid_type
            .headers
            .insert("content-type".into(), "text/plain".into());
        assert_eq!(dispatch(&invalid_type, &store, 8765).status, 415);
        assert!(!temp.path().join("private").exists());
    }

    #[test]
    fn network_routes_reject_unpinned_network_tools_before_probing() {
        let temp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temp.path().join("private/workbench.json"));
        assert_eq!(
            dispatch(
                &request(
                    "POST",
                    "/api/net/scan",
                    json!({"cidr":"192.168.1.0/24","confirm":true})
                ),
                &store,
                8765
            )
            .status,
            409
        );
        assert_eq!(
            dispatch(
                &request(
                    "POST",
                    "/api/net/unifi",
                    json!({"operation":"sites","confirm":true})
                ),
                &store,
                8765
            )
            .status,
            409
        );
        assert!(!temp.path().join("private").exists());
    }

    #[test]
    fn network_routes_validate_activated_scopes_without_external_access() {
        let temp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temp.path().join("private/workbench.json"));
        assert!(store.apply_preset("head-engineer").unwrap());
        let test_cases = [
            (
                "/api/net/scan",
                json!({"cidr":"123456789012345678901234567890123","confirm":true}),
            ),
            (
                "/api/net/unifi",
                json!({"operation":"unsupported","confirm":true}),
            ),
            (
                "/api/net/unifi",
                json!({"operation":"clients","site":"invalid","confirm":true}),
            ),
            (
                "/api/net/unifi",
                json!({
                    "operation":"compare",
                    "site":"00000000-0000-0000-0000-000000000001",
                    "cidr":"8.8.8.8/32",
                    "confirm":true
                }),
            ),
        ];
        for (path, body) in test_cases {
            assert_eq!(
                dispatch(&request("POST", path, body), &store, 8765).status,
                422,
                "{path}"
            );
        }
        assert!(!temp.path().join("private/messages.json").exists());
        assert!(!temp.path().join("private/peers.json").exists());
    }

    #[test]
    fn messenger_routes_reject_cross_origin_and_host_before_local_storage() {
        let temp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temp.path().join("private/workbench.json"));
        for path in ["/api/messages/send", "/api/messages/retry"] {
            let mut req = request(
                "POST",
                path,
                json!({"alias":"scout-one", "body":"Hello", "id":1, "confirm":true}),
            );
            req.headers
                .insert("origin".into(), "https://untrusted.invalid".into());
            assert_eq!(dispatch(&req, &store, 8765).status, 403, "{path}");
            assert!(!temp.path().join("private").exists());
        }
        let mut wrong_host = request("GET", "/api/messages", Value::Null);
        wrong_host
            .headers
            .insert("host".into(), "non-loopback.invalid".into());
        assert_eq!(dispatch(&wrong_host, &store, 8765).status, 403);
        assert!(!temp.path().join("private").exists());
    }

    #[test]
    fn messenger_routes_preserve_confirmation_tier_and_payload_validation() {
        let temp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temp.path().join("private/workbench.json"));
        let requests = [
            (
                "/api/messages/send",
                json!({"alias":"scout-one","body":"Hello"}),
                422,
            ),
            (
                "/api/messages/send",
                json!({"alias":"scout-one","body":"Hello","confirm":true,"tier":99}),
                422,
            ),
            (
                "/api/messages/send",
                json!({"alias":"scout-one","body":"Hello","confirm":true,"tier":"unknown"}),
                422,
            ),
            (
                "/api/messages/send",
                json!({"alias":"scout-one","body":"\u{1b}","confirm":true}),
                422,
            ),
            ("/api/messages/retry", json!({"id":1}), 422),
            ("/api/messages/retry", json!({"id":0,"confirm":true}), 422),
            ("/api/messages/retry", json!({"id":1,"confirm":true}), 404),
        ];
        for (path, body, expected) in requests {
            assert_eq!(
                dispatch(&request("POST", path, body), &store, 8765).status,
                expected,
                "{path}"
            );
            assert!(!temp.path().join("private").exists());
        }
        let mut wrong_type = request(
            "POST",
            "/api/messages/send",
            json!({"alias":"scout-one","body":"Hello","confirm":true}),
        );
        wrong_type
            .headers
            .insert("content-type".into(), "text/plain".into());
        assert_eq!(dispatch(&wrong_type, &store, 8765).status, 415);
        assert!(!temp.path().join("private").exists());
    }

    #[test]
    fn county_source_api_is_allowlisted_local_and_explicit() {
        let temp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temp.path().join("state"));
        let index = dispatch(
            &request("GET", "/api/map/sources", Value::Null),
            &store,
            8765,
        );
        assert_eq!(index.status, 200);
        let sources: Value = serde_json::from_slice(&index.body).unwrap();
        assert_eq!(sources["sources"].as_array().unwrap().len(), 4);
        assert_eq!(sources["sources"][3]["id"], "tiger-pointlm");
        let cache = dispatch(&request("GET", "/api/map/cache", Value::Null), &store, 8765);
        assert_eq!(cache.status, 200);
        let manifest: Value = serde_json::from_slice(&cache.body).unwrap();
        assert_eq!(manifest["active_geoid"], Value::Null);
        assert!(!temp.path().join("state/county_cache").exists());

        let cases = [
            json!({"geoid":"25025","source":"tiger-roads"}),
            json!({"geoid":"../../secret","source":"tiger-roads","confirm":true}),
            json!({"geoid":"25025","source":"https://evil.invalid","confirm":true}),
        ];
        for (i, body) in cases.into_iter().enumerate() {
            let response = dispatch(&request("POST", "/api/map/package", body), &store, 8765);
            assert_eq!(response.status, if i == 0 { 403 } else { 422 });
            assert!(!temp.path().join("state/county_cache").exists());
        }
        let response = dispatch(
            &request(
                "POST",
                "/api/map/package",
                json!({"geoid":"25025","source":"tiger-roads","cache_only":true}),
            ),
            &store,
            8765,
        );
        assert_eq!(response.status, 404);
        assert!(!temp.path().join("state/county_cache").exists());

        let mut malicious = request(
            "POST",
            "/api/map/package",
            json!({"geoid":"25025","source":"tiger-roads","confirm":true}),
        );
        malicious
            .headers
            .insert("origin".to_owned(), "https://untrusted.invalid".to_owned());
        assert_eq!(dispatch(&malicious, &store, 8765).status, 403);
        assert!(!temp.path().join("state/county_cache").exists());

        let cached_dir = temp.path().join("state/county_cache");
        std::fs::create_dir_all(&cached_dir).unwrap();
        std::fs::write(
            cached_dir.join("tl_2025_25025_roads.zip"),
            b"PK\x03\x04local-cached-test",
        )
        .unwrap();
        let response = dispatch(
            &request(
                "POST",
                "/api/map/package",
                json!({"geoid":"25025","source":"tiger-roads","cache_only":true}),
            ),
            &store,
            8765,
        );
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "application/zip");
        assert_eq!(response.body, b"PK\x03\x04local-cached-test");
    }

    #[test]
    fn county_boundary_api_requires_consent_and_restores_validated_cache() {
        let temp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temp.path().join("state"));
        let cases = [
            (json!({"geoid":"25025"}), 403),
            (json!({"geoid":"../../bad","confirm":true}), 422),
            (json!({"geoid":"25025","cache_only":true}), 404),
        ];
        for (body, expected) in cases {
            let response = dispatch(&request("POST", "/api/map/boundary", body), &store, 8765);
            assert_eq!(response.status, expected);
            assert!(!temp.path().join("state/county_cache").exists());
        }
        let mut hostile = request(
            "POST",
            "/api/map/boundary",
            json!({"geoid":"25025","confirm":true}),
        );
        hostile
            .headers
            .insert("origin".to_owned(), "https://untrusted.invalid".to_owned());
        assert_eq!(dispatch(&hostile, &store, 8765).status, 403);
        assert!(!temp.path().join("state/county_cache").exists());

        let dir = temp.path().join("state/county_cache");
        std::fs::create_dir_all(&dir).unwrap();
        let fixture = json!({
            "type":"FeatureCollection",
            "features":[{
                "type":"Feature", "properties":{"GEOID":"25025","NAME":"Suffolk"},
                "geometry":{"type":"Polygon","coordinates":[
                    [[-71.2,42.3],[-71.0,42.3],[-71.0,42.5],[-71.2,42.3]]
                ]}
            }]
        });
        std::fs::write(dir.join("boundary_2025_25025.geojson"), fixture.to_string()).unwrap();
        let cached_req = request(
            "POST",
            "/api/map/boundary",
            json!({"geoid":"25025","cache_only":true}),
        );
        let cached = dispatch(&cached_req, &store, 8765);
        assert_eq!(cached.status, 200);
        let data: Value = serde_json::from_slice(&cached.body).unwrap();
        assert_eq!(data["geoid"], "25025");
        assert_eq!(data["cache_hit"], true);
        assert_eq!(data["network_used"], false);
        assert_eq!(data["paths"][0].as_array().unwrap().len(), 4);
        assert_eq!(dispatch(&cached_req, &store, 8765).status, 200);
        let selected = dispatch(&request("GET", "/api/map/cache", Value::Null), &store, 8765);
        assert_eq!(selected.status, 200);
        // Reading a cached boundary never silently activates a county selection.
        let metadata: Value = serde_json::from_slice(&selected.body).unwrap();
        assert_eq!(metadata["active_geoid"], Value::Null);
    }

    #[test]
    fn zip_lookup_api_is_local_consent_gated_and_offline_first() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(tmp.path().join("state/data.json"));
        let cases = [
            ("/api/map/zip", json!({"zip":"02360"}), 403),
            ("/api/map/zip", json!({"zip":"0236","confirm":true}), 422),
            (
                "/api/map/zip",
                json!({"zip":"02360","cache_only":true}),
                404,
            ),
            ("/api/map/county-zips", json!({"geoid":"25023"}), 403),
            (
                "/api/map/county-zips",
                json!({"geoid":"../../","confirm":true}),
                422,
            ),
            (
                "/api/map/county-zips",
                json!({"geoid":"25023","offset":-1,"cache_only":true}),
                422,
            ),
            (
                "/api/map/county-zips",
                json!({"geoid":"25023","cache_only":true}),
                404,
            ),
        ];
        for (path, body, expected) in cases {
            let response = dispatch(&request("POST", path, body), &store, 8765);
            assert_eq!(response.status, expected, "{path}");
            assert!(!tmp.path().join("state/geo_index").exists());
        }
        let mut bad_origin = request(
            "POST",
            "/api/map/zip",
            json!({"zip":"02360","confirm":true}),
        );
        bad_origin
            .headers
            .insert("origin".to_owned(), "https://untrusted.invalid".to_owned());
        assert_eq!(dispatch(&bad_origin, &store, 8765).status, 403);
        assert!(!tmp.path().join("state/geo_index").exists());
    }

    #[test]
    fn csp_headers_deny_external_scripts_and_framing() {
        let response = Response::new(200, "text/plain", b"ok".to_vec());
        let header = format!(
            "HTTP/1.1 {}\r\nContent-Security-Policy: default-src 'none'; frame-ancestors 'none'",
            response.status
        );
        assert!(header.contains("frame-ancestors 'none'"));
        assert!(HTML.contains("/assets/dashboard.css"));
        assert!(HTML.contains("/assets/dashboard.js"));
        assert!(JS.contains("/api/state"));
        assert!(CSS.contains("@media"));
        assert!(!JS.contains("eval("));
        assert!(MICRO_HTML.contains("/assets/map.js"));
        assert!(HTML.contains("/assets/map.js"));
        assert!(MAP_ONLINE_JS.contains("https://overpass-api.de/api/interpreter"));
        assert!(MICRO_HTML.contains("/assets/map-online.js"));
        assert!(HTML.contains("/assets/map-online.js"));
        assert!(HTML.contains("/assets/map-zip.js"));
        assert!(MICRO_HTML.contains("/assets/map-zip.js"));
        assert!(ZIP_JS.contains("/api/map/zip"));
        assert!(MAP_IMPORT_JS.contains("parseRustGIS"));
        assert!(MAP_IMPORT_JS.contains("readPackage"));
        assert!(MICRO_HTML.contains("/assets/map-import.js"));
        assert!(HTML.contains("/assets/map-import.js"));
        assert!(MAP_JS.contains("watchPosition"));
        assert!(MAP_JS.contains("parseGeoJSON"));
        assert!(MICRO_HTML.contains("/assets/micro.css"));
        assert!(MICRO_HTML.contains("/assets/micro.js"));
        assert!(MICRO_HTML.contains("id=\"micro-rail\""));
        assert!(MICRO_CSS.contains("@media"));
        assert!(MICRO_CSS.contains("480px"));
        assert!(MICRO_JS.contains("/api/state"));
        assert!(MICRO_JS.contains("/api/profile"));
        assert!(MICRO_JS.contains("/api/custom/add"));
        assert!(!MICRO_JS.contains("eval("));
    }
}
