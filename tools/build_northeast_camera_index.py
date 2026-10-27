#!/usr/bin/env python3
"""Review-only import of Northeast DOT camera *metadata*, never auto-playback.

Examples:
  python3 tools/build_northeast_camera_index.py --source md-imap --fetch --output /tmp/md-camera-review.json
  python3 tools/build_northeast_camera_index.py --source ddot --input /tmp/ddot-response.json --output /tmp/ddot-review.json

Reads only fixed official public endpoints, one explicit request per invocation.
No private IP discovery, redirects, camera media requests, recording or credentials.
Output is review-only: never consumed by the live /api/state camera wall.
"""
import argparse
import datetime as dt
import json
import math
import pathlib
import re
import urllib.error
import urllib.parse
import urllib.request

MAX_BYTES = 6 * 1024 * 1024
MAX_RECORDS = 5000  # supports full authorized CT/NY JSON exports; still capped at 6 MiB
SOURCES = {
    "md-socrata": {
        "state": "MD", "kind": "socrata", "agency": "Maryland Open Data / CHART",
        "url": "https://opendata.maryland.gov/resource/4z3c-43ce.json?$limit=500",
        "page": "https://opendata.maryland.gov/resource/4z3c-43ce.json",
    },
    "md-imap": {
        "state": "MD", "kind": "arcgis", "agency": "MD iMAP / CHART",
        "url": "https://mdgeodata.md.gov/imap/rest/services/Transportation/MD_TrafficCameras/FeatureServer/0/query"
               "?where=1%3D1&outFields=OBJECTID%2Clocation%2Ccounty%2CfeedID%2Curl%2Clat%2Clong"
               "&returnGeometry=false&resultRecordCount=500&f=json",
        "page": "https://mdgeodata.md.gov/imap/rest/services/Transportation/MD_TrafficCameras/FeatureServer/0",
    },
    "ddot": {
        "state": "DC", "kind": "arcgis", "agency": "District DOT / DC GIS",
        "url": "https://maps2.dcgis.dc.gov/dcgis/rest/services/DDOT/TrafficOperations/FeatureServer/2/query"
               "?where=1%3D1&outFields=OBJECTID%2CLocation%2CLatitude%2CLongitude%2CCameraID%2COperation_Status"
               "&returnGeometry=false&resultRecordCount=500&f=json",
        "page": "https://maps2.dcgis.dc.gov/dcgis/rest/services/DDOT/TrafficOperations/FeatureServer/2",
    },
    "deldot": {
        "state": "DE", "kind": "arcgis", "agency": "DelDOT FirstMap (test host)",
        "url": "https://enterprise.firstmaptest.delaware.gov/arcgis/rest/services/Transportation/"
               "DE_Boundary_and_Point/FeatureServer/26/query"
               "?where=1%3D1&outFields=OBJECTID%2CPUBLIC_NAME%2CMAINROAD%2CINTERSECTION%2CSTATUS"
               "&returnGeometry=true&outSR=4326&resultRecordCount=500&f=json",
        "page": "https://enterprise.firstmaptest.delaware.gov/arcgis/rest/services/Transportation/"
                "DE_Boundary_and_Point/FeatureServer/26",
    },
    "ctroads": {
        "state": "CT", "kind": "restricted-json", "agency": "Connecticut DOT / CTroads",
        "url": "https://www.ctroads.org/developers/help",
        "page": "https://www.ctroads.org/developers/help",
        "fetch_allowed": False,  # a registered developer key and approval are required
    },
    "511ny": {
        "state": "NY", "kind": "restricted-json", "agency": "New York State 511 / NYSDOT",
        "url": "https://prod-ny.ibi511.com/developers/help",
        "page": "https://prod-ny.ibi511.com/developers/resources",
        "fetch_allowed": False,  # a registered developer key and agreement are required
    },
    "ridot": {
        "state": "RI", "kind": "arcgis", "agency": "Rhode Island DOT / Rhodeways",
        "url": "https://risegis.ri.gov/hosting/rest/services/RIDOT/Rhodeways/MapServer/6/query"
               "?where=1%3D1&outFields=OBJECTID%2CEquipmentID%2CDescription%2CLatitude%2CLongitude"
               "%2CEnabled%2CCategory%2CDirection&returnGeometry=false&resultRecordCount=500&f=json",
        "page": "https://risegis.ri.gov/hosting/rest/services/RIDOT/Rhodeways/MapServer/6",
    },
    "massdot-staging": {
        "state": "MA", "kind": "arcgis", "agency": "MassDOT traffic-camera GIS staging (unapproved)",
        "url": "https://gisstg.massdot.state.ma.us/rh/rest/services/Hosted/MassDOT_Traffic_Cams/FeatureServer/0/query"
               "?where=1%3D1&outFields=objectid%2Cpublicid%2Cname%2Cdescription%2Cprovider%2Cstatecode"
               "&returnGeometry=true&outSR=4326&resultRecordCount=500&f=json",
        "page": "https://gisstg.massdot.state.ma.us/rh/rest/services/Hosted/MassDOT_Traffic_Cams/FeatureServer/0",
        "fetch_allowed": False,  # staging host: offline review only pending production confirmation
    },
    "vdot": {
        "state": "VA", "kind": "geojson", "agency": "VDOT 511",
        "url": "https://511.vdot.virginia.gov/services/map/layers/map/cams",
        "page": "https://www.vdot.virginia.gov/news-events/media/",
    },
}
BOUNDS = {
    "MD": (37.8, 39.75, -79.5, -75.0),
    "DC": (38.7, 39.05, -77.2, -76.85),
    "DE": (38.35, 39.9, -75.85, -74.75),
    "VA": (38.4, 39.25, -77.7, -76.7),  # DC metro only, not all Virginia
    "RI": (41.1, 42.05, -72.05, -71.05),
    "CT": (40.9, 42.1, -73.75, -71.75),
    "NY": (40.45, 45.15, -79.9, -71.8),
    "MA": (41.15, 42.95, -73.6, -69.8),
}


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, msg, headers, newurl):
        return None


OPENER = urllib.request.build_opener(NoRedirect)


def bounded_json(text):
    if len(text) > MAX_BYTES:
        raise ValueError("Provider metadata exceeds 6 MiB")
    payload = json.loads(text)
    if isinstance(payload, list):
        if len(payload) > MAX_RECORDS:
            raise ValueError("Socrata records exceed one-pass limit")
        payload = {"features": [{"attributes":item} for item in payload if isinstance(item, dict)]}
    if not isinstance(payload, dict) or not isinstance(payload.get("features"), list):
        raise ValueError("Expected ArcGIS, Socrata or GeoJSON camera features")
    if len(payload["features"]) > MAX_RECORDS:
        raise ValueError("Provider features exceed one-pass limit")
    if payload.get("error"):
        raise ValueError("Upstream returned an API error")
    return payload


def fetch_public(source):
    if SOURCES[source].get("fetch_allowed") is False:
        raise ValueError("Staging source is offline-review only; --fetch is forbidden")
    url = SOURCES[source]["url"]  # user cannot provide a URL
    parts = urllib.parse.urlsplit(url)
    if parts.scheme != "https" or not parts.hostname or parts.username or parts.password:
        raise ValueError("Insecure source definition")
    if parts.hostname not in {
        "mdgeodata.md.gov", "maps2.dcgis.dc.gov",
        "enterprise.firstmaptest.delaware.gov", "511.vdot.virginia.gov",
        "opendata.maryland.gov", "risegis.ri.gov"
    }:
        raise ValueError("Unapproved metadata provider")
    request = urllib.request.Request(url, headers={"Accept": "application/json"})
    with OPENER.open(request, timeout=10) as response:
        if response.status != 200:
            raise ValueError("Provider HTTP status not OK")
        value = response.read(MAX_BYTES + 1)
    return bounded_json(value)


def coordinate(lat, lon, state):
    # Socrata publishes numeric coordinates as short decimal strings.
    if isinstance(lat, str) and re.fullmatch(r"-?\d{1,3}(?:\.\d{1,12})?", lat):
        lat = float(lat)
    if isinstance(lon, str) and re.fullmatch(r"-?\d{1,3}(?:\.\d{1,12})?", lon):
        lon = float(lon)
    if isinstance(lat, bool) or isinstance(lon, bool):
        return None
    if not isinstance(lat, (int, float)) or not isinstance(lon, (int, float)):
        return None
    if not (math.isfinite(lat) and math.isfinite(lon)):
        return None
    s, n, w, e = BOUNDS[state]
    if not (s <= lat <= n and w <= lon <= e):
        return None
    return round(lat, 7), round(lon, 7)


def clean_name(value):
    return re.sub(r"\s+", " ", value.strip())[:110] if isinstance(value, str) else ""


REVIEW_URL_HOSTS = {
    "chart.maryland.gov", "www.chart.maryland.gov",
    "snapshot.vdotcameras.com", "511.vdot.virginia.gov",
}
SENSITIVE_QUERY_KEYS = {"key", "token", "access_token", "auth", "authorization",
                        "session", "password", "secret", "sig", "signature"}

def safe_url(value):
    """Keep only publisher-hosted, noncredential metadata links; never fetch them."""
    if not isinstance(value, str) or len(value) > 500:
        return None
    try:
        u = urllib.parse.urlsplit(value)
        if (u.scheme != "https" or u.hostname not in REVIEW_URL_HOSTS
                or u.username or u.password or u.port or u.fragment):
            return None
        if any(k.lower() in SENSITIVE_QUERY_KEYS
               for k, _ in urllib.parse.parse_qsl(u.query, keep_blank_values=True)):
            return None
        return value
    except ValueError:
        return None


def normalize(source, payload):
    spec = SOURCES[source]
    records = []
    for item in payload["features"]:
        if not isinstance(item, dict):
            continue
        attrs = item.get("attributes") if spec["kind"] in ("arcgis", "socrata", "restricted-json") else item.get("properties")
        if not isinstance(attrs, dict):
            continue
        if source == "md-socrata":
            ident = attrs.get("objectid")
            name = clean_name(attrs.get("location"))
            coords = coordinate(attrs.get("lat"), attrs.get("long"), "MD")
            evidence = safe_url(attrs.get("url"))
            status = "metadata_only_media_requires_review"
            provider_id = attrs.get("feedid")
        elif source == "md-imap":
            ident = attrs.get("OBJECTID")
            name = clean_name(attrs.get("location"))
            coords = coordinate(attrs.get("lat"), attrs.get("long"), "MD")
            evidence = safe_url(attrs.get("url"))
            status = "metadata_only_media_requires_review"
            provider_id = attrs.get("feedID")
        elif source == "ddot":
            ident = attrs.get("CameraID") or attrs.get("OBJECTID")
            name = clean_name(attrs.get("Location"))
            coords = coordinate(attrs.get("Latitude"), attrs.get("Longitude"), "DC")
            evidence = None
            status = "agency_camera_location_only"
            provider_id = attrs.get("CameraID")
        elif source in ("ctroads", "511ny"):
            ident = attrs.get("ID")
            name = clean_name(attrs.get("Name") or attrs.get("RoadwayName"))
            coords = coordinate(attrs.get("Latitude"), attrs.get("Longitude"), spec["state"])
            # Authenticated developer exports are review-only. Never persist raw
            # Url, VideoUrl, API key, or a source request URL containing a key.
            evidence = None
            status = ("developer_feed_disabled_or_blocked" if
                      attrs.get("Disabled") is True or attrs.get("Blocked") is True
                      else "developer_agreement_metadata_only")
            provider_id = ident
        elif source == "ridot":
            ident = attrs.get("EquipmentID") or attrs.get("OBJECTID")
            name = clean_name(attrs.get("Description"))
            coords = coordinate(attrs.get("Latitude"), attrs.get("Longitude"), "RI")
            # GIS enabled status reports configuration, not image health.
            evidence = None
            status = "agency_enabled_metadata_only" if attrs.get("Enabled") == 1 else "agency_disabled_or_unknown"
            provider_id = attrs.get("EquipmentID")
        elif source == "massdot-staging":
            ident = attrs.get("publicid") or attrs.get("objectid")
            name = clean_name(attrs.get("name") or attrs.get("description"))
            geom = item.get("geometry") or {}
            coords = coordinate(geom.get("y"), geom.get("x"), "MA")
            # Exclude world-scale outliers. No staging JPEG or media URLs are promoted.
            evidence = None
            status = "staging_only_production_source_unverified"
            provider_id = attrs.get("publicid")
        elif source == "deldot":
            ident = attrs.get("OBJECTID")
            name = clean_name(attrs.get("PUBLIC_NAME") or
                              " ".join(str(attrs.get(k) or "") for k in ("MAINROAD", "INTERSECTION")))
            geom = item.get("geometry") or {}
            # Only request-derived WGS84 geometry; no arbitrary EPSG projection guess.
            coords = coordinate(geom.get("y"), geom.get("x"), "DE")
            evidence = None
            status = "test_host_location_only"
            provider_id = attrs.get("OBJECTID")
        else:
            ident = attrs.get("id")
            name = clean_name(attrs.get("description") or attrs.get("route"))
            geom = item.get("geometry") or {}
            raw = geom.get("coordinates")
            coords = coordinate(raw[1], raw[0], "VA") if isinstance(raw, list) and len(raw) == 2 else None
            evidence = safe_url(attrs.get("image_url"))
            status = "third_party_agreement_required"
            provider_id = ident
        if not coords or not name or not isinstance(ident, (str, int)) or not re.fullmatch(r"[\w-]{1,64}", str(ident)):
            continue
        records.append({
            "id": source + "-" + str(ident),
            "provider_id": source,
            "upstream_id": str(provider_id)[:80] if provider_id is not None else None,
            "state": spec["state"],
            "name": name,
            "lat": coords[0], "lon": coords[1],
            "source_url": spec["page"],
            "media_reference_review_only": evidence,
            "media_status": status,
            "image_url": None,
            "frame_captured_at": None,
            "live_verified": False,
        })
    unique = {item["id"]: item for item in records}
    return [unique[k] for k in sorted(unique)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, choices=sorted(SOURCES))
    inputs = parser.add_mutually_exclusive_group(required=True)
    inputs.add_argument("--input", type=pathlib.Path, help="offline agency JSON/GeoJSON file")
    inputs.add_argument("--fetch", action="store_true", help="one explicit allowlisted metadata HTTP GET")
    parser.add_argument("--output", type=pathlib.Path, required=True, help="review JSON (not active feed list)")
    args = parser.parse_args()
    if args.fetch:
        payload = fetch_public(args.source)
    else:
        if args.input.stat().st_size > MAX_BYTES:
            raise ValueError("Input file exceeds 6 MiB")
        payload = bounded_json(args.input.read_bytes())
    records = normalize(args.source, payload)
    review = {
        "schema_version": 1,
        "review_only": True,
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "source": args.source,
        "source_url": SOURCES[args.source]["page"],
        "camera_count": len(records),
        "upstream_exceeded_limit": bool(payload.get("exceededTransferLimit")),
        "cameras": records,
        "warning": "Camera locations are not proof of media access, permission or live freshness.",
    }
    args.output.write_text(json.dumps(review, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print("REVIEW ONLY: " + str(len(records)) + " valid Northeast camera locations -> " + str(args.output))


if __name__ == "__main__":
    main()
