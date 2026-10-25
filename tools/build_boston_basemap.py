#!/usr/bin/env python3
"""Rebuild only true City of Boston SAM road geometry from a pinned GeoJSON blob.

No network, geocoding, invented vertices, or synthetic locations. A complete
byte-identical source snapshot must be supplied locally before regeneration.
"""
import argparse
import hashlib
import json
import math
import re
from pathlib import Path

SOURCE_BLOB_SHA = "a86dc5f6913987585e429e625bdc61781d03d40e"
EXTENT = [-71.14, 42.309, -70.99, 42.399]
SOURCE = {
    "dataset": "City of Boston Street Segments (SAM System)",
    "origin_url": "https://data.boston.gov/dataset/boston-street-segments-sam-system",
    "geometry_service": "https://gisportal.boston.gov/arcgis/rest/services/Infrastructure/OpenData/MapServer/8",
    "source_mirror": "https://github.com/olincollege/night-light/blob/33b91f4179c5c910ad3b79934bc457e86c829c31/datasets/boston_street_segments.geojson",
    "source_blob_sha": SOURCE_BLOB_SHA,
    "license_note": "Boston Street Segments metadata references ODC PDDL 1.0; verify City terms for distribution.",
    "position": "Actual stored City Boston WGS84 line geometry, rounded to six decimals; no fictional features.",
}
HEADER = (
    "/* GENERATED from pinned Boston SAM source; real WGS84 street segments only.\n"
    " * See docs/BOSTON_BASEMAP.md and tools/build_boston_basemap.py. Do not hand-edit coordinates.\n"
    " */\nwindow.RustBoyBostonBase = Object.freeze("
)


def compile_source(raw_bytes: bytes) -> dict:
    sha = hashlib.sha1(f"blob {len(raw_bytes)}\0".encode("ascii") + raw_bytes).hexdigest()
    if sha != SOURCE_BLOB_SHA:
        raise ValueError("Input is not the pinned City Boston street GeoJSON source blob")
    src = json.loads(raw_bytes)
    if src.get("type") != "FeatureCollection":
        raise ValueError("Expected source GeoJSON FeatureCollection")
    names = []
    lookup = {}
    seen = set()
    roads = []
    for feature in src["features"]:
        geometry = feature.get("geometry", {})
        points = geometry.get("coordinates", [])
        if geometry.get("type") != "LineString" or len(points) < 2:
            continue
        if not any(
            len(pt) >= 2 and EXTENT[0] <= pt[0] <= EXTENT[2]
            and EXTENT[1] <= pt[1] <= EXTENT[3] for pt in points
        ):
            continue
        p = feature["properties"]
        ident = p.get("SEGMENT_ID")
        if not isinstance(ident, int) or isinstance(ident, bool) or ident <= 0 or ident in seen:
            continue
        if not all(
            len(pt) >= 2 and all(isinstance(v, (int, float)) and math.isfinite(v) for v in pt[:2])
            and -180 <= pt[0] <= 180 and -90 <= pt[1] <= 90 for pt in points
        ):
            continue
        name = " ".join(str(p[k]).strip() for k in ("ST_NAME", "ST_TYPE")
                        if p.get(k) and str(p[k]).strip())
        if len(name) > 64:
            continue
        seen.add(ident)
        if name not in lookup:
            lookup[name] = len(names)
            names.append(name)
        code = str(p.get("CFCC") or "")
        street_class = int(code[1:]) if re.fullmatch(r"A\d{1,2}", code) else 0
        line = [ident, street_class, lookup[name]]
        for lon, lat, *_ in points:
            # Quantization only: source point, not interpolated or fabricated.
            line.extend([math.floor((lon + 71) * 1_000_000 + 0.5),
                         math.floor((lat - 42) * 1_000_000 + 0.5)])
        roads.append(line)
    if len(roads) != 10634 or len(names) != 2522:
        raise ValueError("Pinned extract feature/name counts changed")
    if sum((len(line) - 3) // 2 for line in roads) != 47994:
        raise ValueError("Pinned WGS84 vertex count changed")
    return {
        "schema": 1,
        "name": "Boston Street Geometry",
        "region": "Central Boston, Massachusetts",
        "bounds": EXTENT,
        "center": [-71.065, 42.356],
        "encoding": "WGS84: lon = -71 + x/1e6; lat = 42 + y/1e6",
        "source": SOURCE,
        "names": names,
        "roads": roads,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True, help="Pinned original GeoJSON")
    parser.add_argument("--output", type=Path,
                        default=Path("crates/pipboy/assets/boston-base.js"))
    parser.add_argument("--check", action="store_true",
                        help="Validate checked-in payload without modifying anything")
    args = parser.parse_args()
    compiled = compile_source(args.input.read_bytes())
    content = HEADER + json.dumps(compiled, ensure_ascii=False, separators=(",", ":")) + ");\n"
    if args.check:
        actual = args.output.read_text(encoding="utf-8")
        match = re.search(r"window\.RustBoyBostonBase = Object\.freeze\((\{.*\})\);\s*$",
                          actual, re.DOTALL)
        if not match or json.loads(match.group(1)) != compiled:
            raise SystemExit("FAIL: bundled Boston source differs from pinned upstream")
        print(f"PASS: {len(compiled['roads'])} pinned real Boston street segments")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(content, encoding="utf-8")
        print(f"Wrote {len(compiled['roads'])} real Boston segments to {args.output}")


if __name__ == "__main__":
    main()
