#!/usr/bin/env python3
"""Offline tests for Northeast DOT metadata normalization and review catalog."""
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import build_northeast_camera_index as index  # noqa: E402


class NormalizeTests(unittest.TestCase):
    def test_maryland_arcgis_is_location_only_not_live(self):
        sample = {"features": [
            {"attributes": {"OBJECTID": 12, "location": "I-95 at MD 43",
                            "feedID": "C012", "lat": 39.42, "long": -76.4,
                            "url": "https://www.chart.maryland.gov/cameras/12"}},
            {"attributes": {"OBJECTID": 13, "location": "Wrong geometry",
                            "lat": 45, "long": -76, "url": "https://example.org/cam"}},
            {"attributes": {"OBJECTID": 14, "location": "Unsafe URL",
                            "lat": 39.1, "long": -76.1,
                            "url": "http://127.0.0.1/internal"}}
        ]}
        items = index.normalize("md-imap", index.bounded_json(json.dumps(sample)))
        self.assertEqual([x["id"] for x in items], ["md-imap-12", "md-imap-14"])
        self.assertEqual(items[0]["upstream_id"], "C012")
        self.assertEqual(items[0]["media_reference_review_only"],
                         "https://www.chart.maryland.gov/cameras/12")
        self.assertIsNone(items[1]["media_reference_review_only"])
        self.assertTrue(all(x["image_url"] is None and not x["live_verified"] for x in items))

    def test_dc_camera_status_is_not_footage_health(self):
        sample = {"features": [{"attributes": {
            "OBJECTID": 1, "CameraID": 101, "Location": "DC I-395",
            "Latitude": 38.89, "Longitude": -77.03, "Operation_Status": 1
        }}]}
        item = index.normalize("ddot", index.bounded_json(json.dumps(sample)))[0]
        self.assertEqual(item["lat"], 38.89)
        self.assertEqual(item["media_status"], "agency_camera_location_only")
        self.assertIsNone(item["frame_captured_at"])
        self.assertFalse(item["live_verified"])

    def test_delaware_requires_wgs84_coordinates(self):
        sample = {"features": [
            {"attributes": {"OBJECTID": 5, "PUBLIC_NAME": "I-95 @ DE-1"},
             "geometry": {"x": -75.58, "y": 39.68}},
            {"attributes": {"OBJECTID": 6, "PUBLIC_NAME": "Projected x/y"},
             "geometry": {"x": 210000, "y": 135000}}
        ]}
        items = index.normalize("deldot", index.bounded_json(json.dumps(sample)))
        self.assertEqual(len(items), 1)
        self.assertEqual(items[0]["id"], "deldot-5")
        self.assertEqual(items[0]["media_status"], "test_host_location_only")

    def test_vdot_has_no_automatic_media_without_user_agreement(self):
        sample = {"features": [
            {"type": "Feature", "properties": {
                "id": "3379", "description": "US 50 @ Patrick Henry",
                "active": True,
                "image_url": "https://snapshot.vdotcameras.com/thumbs/abc.flv.png"},
             "geometry": {"type": "Point", "coordinates": [-77.14676, 38.868556]}},
            {"properties": {"id": "333", "description": "Outside metro", "active": True},
             "geometry": {"coordinates": [-80, 36]}}
        ]}
        items = index.normalize("vdot", index.bounded_json(json.dumps(sample)))
        self.assertEqual(len(items), 1)
        self.assertEqual(items[0]["media_status"], "third_party_agreement_required")
        self.assertTrue(items[0]["media_reference_review_only"].startswith("https://"))
        self.assertIsNone(items[0]["image_url"])
        self.assertFalse(items[0]["live_verified"])

    def test_input_limits_and_malformed_payload_fail_closed(self):
        for payload in ('{}', '{"features": null}', '{"error": {"message": "bad"}, "features":[]}',
                        '{"features": [null]}'):
            if payload == '{"features": [null]}':
                self.assertEqual(index.normalize("ddot", index.bounded_json(payload)), [])
            else:
                with self.assertRaises(ValueError):
                    index.bounded_json(payload)
        self.assertIsNone(index.coordinate(39.1, float("nan"), "MD"))
        self.assertIsNone(index.coordinate(True, -76, "MD"))
        self.assertIsNone(index.safe_url("http://127.0.0.1/secret"))
        self.assertIsNone(index.safe_url("https://user:pass@example.org/photo.jpg"))

    def test_curated_named_sites_have_no_claims_of_live_access(self):
        sites = json.loads((ROOT / "catalog/northeast_camera_sites.json").read_text())
        providers = json.loads((ROOT / "catalog/camera_feeders.json").read_text())
        ids = {p["id"] for p in providers["providers"]}
        cameras = sites["cameras"]
        self.assertGreaterEqual(len(cameras), 35)
        self.assertEqual(len(set(c["id"] for c in cameras)), len(cameras))
        self.assertTrue(all(c["provider_id"] in ids for c in cameras))
        self.assertTrue(all(c["media_url"] is None and
                            c["last_image_verified_at"] is None for c in cameras))
        self.assertTrue(all((c["lat"] is None and c["lon"] is None) or
                            index.coordinate(c["lat"], c["lon"], c["state"]) for c in cameras))
        self.assertNotIn("youtube", json.dumps(sites).lower())


if __name__ == "__main__":
    unittest.main()
