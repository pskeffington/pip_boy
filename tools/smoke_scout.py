#!/usr/bin/env python3
"""Cross-platform offline Scout GIS/GeoJSON CLI acceptance (never opens a network)."""
import json
import math
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target" / "debug" / ("pipboy.exe" if os.name == "nt" else "pipboy")


def call(env, *args, ok=True):
    result = subprocess.run(
        [str(BINARY), *args],
        cwd=ROOT,
        env=env,
        capture_output=True,
        text=True,
        timeout=12,
    )
    if ok:
        assert result.returncode == 0, (args, result.stderr)
    else:
        assert result.returncode != 0, (args, result.stdout)
    return result


with tempfile.TemporaryDirectory() as temporary:
    home = Path(temporary) / "workbench"
    env = dict(os.environ, PIPBOY_HOME=str(home))
    feature = json.loads(call(env, "scout", "geo", "waypoint",
                              'Trailhead "East"', "42.0", "-70.75").stdout)
    assert feature["type"] == "Feature"
    assert feature["geometry"] == {"type": "Point", "coordinates": [-70.75, 42]}
    assert feature["properties"]["name"] == 'Trailhead "East"'
    assert not home.exists(), "GIS CLI unexpectedly persisted a location"

    out = call(env, "scout", "geo", "distance",
               "0", "0", "0", "1").stdout
    assert "111195." in out and "No network or GNSS used" in out
    out = call(env, "scout", "geo", "distance",
               "0", "179.9", "0", "-179.9").stdout
    meters = float(out.split("Distance: ")[1].split(" m ")[0])
    assert math.isclose(meters, 22239, abs_tol=5)

    file = Path(temporary) / "test.geojson"
    file.write_text(json.dumps({
        "type": "FeatureCollection",
        "features": [
            feature,
            {"type": "Feature",
             "geometry": {"type": "Point", "coordinates": [-71.5, 41.0]},
             "properties": {"name": "West"}}
        ],
    }), encoding="utf-8")
    report = json.loads(call(env, "scout", "geo", "inspect", str(file)).stdout)
    assert report["features"] == 2
    assert report["coordinate_order"] == "longitude,latitude"
    assert report["bbox"] == [-71.5, 41, -70.75, 42]
    assert report["network_used"] is False
    assert report["geometry"] == "Point"

    for geometry, label in [
        ({"type": "LineString", "coordinates": [[0, 0], [1, 1]]}, "LineString"),
        ({"type": "Polygon", "coordinates": [[[0, 0], [1, 0], [1, 1], [0, 0]]]}, "Polygon"),
        ({"type": "GeometryCollection", "geometries": [
            {"type": "Point", "coordinates": [0, 0]},
            {"type": "LineString", "coordinates": [[0, 0], [1, 1]]}
        ]}, "Mixed"),
    ]:
        file.write_text(json.dumps(geometry), encoding="utf-8")
        report = json.loads(call(env, "scout", "geo", "inspect", str(file)).stdout)
        assert report["geometry"] == label and report["bbox"] == [0, 0, 1, 1], report

    invalids = [
        ("geo", "waypoint", "Bad", "91", "0"),
        ("geo", "waypoint", "Bad", "0", "-181"),
        ("geo", "waypoint", "Bad", "nan", "0"),
        ("geo", "waypoint", "Bad", "0", "inf"),
        ("geo", "waypoint", "Bad", "a", "b"),
        ("geo", "waypoint", "Bad", "0", "0", "extra"),
        ("geo", "distance", "0", "0", "1"),
        ("geo", "inspect"),
        ("geo", "inspect", str(file), "--anything"),
        ("invalid",),
    ]
    for case in invalids:
        call(env, "scout", *case, ok=False)
    file.write_text('{"type":"Point","coordinates":[181,0]}',
                    encoding="utf-8")
    call(env, "scout", "geo", "inspect", str(file), ok=False)
    file.write_text('{"type":"Point","coordinates":[0,0],"crs":{"name":"EPSG:3857"}}',
                    encoding="utf-8")
    call(env, "scout", "geo", "inspect", str(file), ok=False)
    file.write_text(json.dumps({"type":"LineString", "coordinates":[[0,0]]*30001}),
                    encoding="utf-8")
    call(env, "scout", "geo", "inspect", str(file), ok=False)
    file.write_text(" " * (1024 * 1024 + 1), encoding="utf-8")
    call(env, "scout", "geo", "inspect", str(file), ok=False)
    assert not home.exists(), "Rejected GIS inputs must not create state"

    call(env, "workbench", "use", "reclamation-scout", "--replace")
    profile = json.loads(call(env, "workbench", "list", "--json").stdout)
    assert profile["profile"] == "reclamation-scout"
    assert [x["tool"] for x in profile["slots"]] == [
        "reclamation", "geo-lab", "game", "net-lab", "notes", "host"
    ]
    assert not (home / "notes.json").exists()
    assert not (home / "permissions.json").exists()

print("PASS: Scout GeoJSON geometry families, WGS84 bounds, finite coordinates, size limits, dateline distance, offline behavior and ready-made role")
