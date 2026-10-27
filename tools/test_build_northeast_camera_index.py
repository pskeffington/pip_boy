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

    def test_maryland_socrata_metadata_uses_actual_camera_coordinates(self):
        sample = [{"objectid":"55", "location":"I-270 & Montgomery Village Ave (MD 124)",
                   "feedid":"3d010dec036a0075004d823633235daa",
                   "lat":"39.148937", "long":"-77.21702",
                   "url":"https://chart.maryland.gov/video/video.php?feed=3d010dec036a0075004d823633235daa"}]
        items = index.normalize("md-socrata", index.bounded_json(json.dumps(sample)))
        self.assertEqual(len(items), 1)
        self.assertEqual(items[0]["id"], "md-socrata-55")
        self.assertEqual(items[0]["lat"], 39.148937)
        self.assertEqual(items[0]["lon"], -77.21702)
        self.assertIsNone(items[0]["image_url"])
        self.assertFalse(items[0]["live_verified"])

    def test_geolocated_seed_records_are_real_coordinates_but_not_live(self):
        data = json.loads((ROOT / "catalog/maryland_geolocated_camera_seeds.json").read_text())
        self.assertGreaterEqual(data["candidate_count"], 20)
        self.assertEqual(data["candidate_count"], len(data["cameras"]))
        self.assertEqual(len({x["id"] for x in data["cameras"]}), len(data["cameras"]))
        for camera in data["cameras"]:
            self.assertIsNotNone(index.coordinate(camera["lat"], camera["lon"], "MD"))
            self.assertIsNone(camera["image_url"])
            self.assertFalse(camera["live_verified"])

    def test_ridot_official_camera_layer_imports_location_not_stream(self):
        sample = {"features": [
            {"attributes": {"OBJECTID": 100, "EquipmentID": 211, "Description": "I-95 at Thurbers Ave",
                            "Latitude": 41.804, "Longitude": -71.423, "Enabled": 1,
                            "CCVEWebURL": "https://example.org/video/211"}},
            {"attributes": {"OBJECTID": 101, "EquipmentID": 212, "Description": "Disabled camera",
                            "Latitude": 41.81, "Longitude": -71.43, "Enabled": 0}},
            {"attributes": {"OBJECTID": 102, "EquipmentID": 213, "Description": "Out of Rhode Island",
                            "Latitude": 42.5, "Longitude": -71.4, "Enabled": 1}}
        ]}
        rows = index.normalize("ridot", index.bounded_json(json.dumps(sample)))
        self.assertEqual(len(rows), 2)
        self.assertEqual(rows[0]["id"], "ridot-211")
        self.assertEqual(rows[0]["media_status"], "agency_enabled_metadata_only")
        self.assertEqual(rows[1]["media_status"], "agency_disabled_or_unknown")
        self.assertTrue(all(row["image_url"] is None and row["frame_captured_at"] is None
                            and row["media_reference_review_only"] is None for row in rows))
        self.assertTrue(all(not row["live_verified"] for row in rows))

    def test_massdot_staging_is_offline_only_and_rejects_global_outliers(self):
        sample = {"features": [
            {"attributes": {"objectid": 21, "publicid": "MA21", "name": "I-93 Boston",
                            "halfjpeg": "https://unapproved.example/frame.jpg"},
             "geometry": {"x": -71.06, "y": 42.36}},
            {"attributes": {"objectid": 22, "publicid": "WORLD22", "name": "Foreign coordinate"},
             "geometry": {"x": 4.74, "y": 52.31}}
        ]}
        rows = index.normalize("massdot-staging", index.bounded_json(json.dumps(sample)))
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["id"], "massdot-staging-MA21")
        self.assertEqual(rows[0]["media_status"], "staging_only_production_source_unverified")
        self.assertIsNone(rows[0]["image_url"])
        self.assertIsNone(rows[0]["media_reference_review_only"])
        with self.assertRaisesRegex(ValueError, "offline-review only"):
            index.fetch_public("massdot-staging")

    def test_ctroads_and_511ny_developer_exports_are_offline_only(self):
        for source, lat, lon in (("ctroads", 41.76, -72.68),
                                 ("511ny", 40.75, -73.98)):
            payload = [{"ID": "CAM-001", "Name": "I-95 public camera",
                        "Latitude": lat, "Longitude": lon,
                        "Url": "https://images.example.org/camera.jpg",
                        "VideoUrl": "https://images.example.org/movie.m3u8",
                        "Disabled": False, "Blocked": False},
                       {"ID": "CAM-002", "Name": "Blocked camera",
                        "Latitude": lat, "Longitude": lon,
                        "Disabled": False, "Blocked": True}]
            rows = index.normalize(source, index.bounded_json(json.dumps(payload)))
            self.assertEqual(len(rows), 2)
            self.assertEqual(rows[0]["media_status"], "developer_agreement_metadata_only")
            self.assertEqual(rows[1]["media_status"], "developer_feed_disabled_or_blocked")
            for item in rows:
                self.assertIsNone(item["image_url"])
                self.assertIsNone(item["media_reference_review_only"])
                self.assertFalse(item["live_verified"])
            with self.assertRaisesRegex(ValueError, "offline-review only"):
                index.fetch_public(source)

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
        self.assertIsNone(index.safe_url("https://127.0.0.1/internal"))
        self.assertIsNone(index.safe_url("https://unknown-public.example.org/camera"))
        self.assertIsNone(index.safe_url("https://chart.maryland.gov/video/?to" + "ken=fixture"))
        self.assertEqual(index.safe_url("https://chart.maryland.gov/video/video.php?feed=abc"),
                         "https://chart.maryland.gov/video/video.php?feed=abc")

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
