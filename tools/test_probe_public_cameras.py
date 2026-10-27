#!/usr/bin/env python3
"""Offline regressions for the bounded public DOT camera connectivity probe."""
import io
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import probe_public_cameras as camera  # noqa: E402


class FakeResponse:
    def __init__(self, body, kind="image/png", status=200):
        self.body = body
        self.status = status
        self.headers = {"Content-Type": kind, "Last-Modified": "Tue, 01 Jan 2024 00:00:00 GMT"}

    def __enter__(self):
        return self

    def __exit__(self, *args):
        return False

    def read(self, limit):
        return self.body[:limit]


class FakeOpener:
    def __init__(self, response):
        self.response = response
        self.urls = []

    def open(self, request, timeout=None):
        self.urls.append(request.full_url)
        return self.response


class PublicCameraProbeTests(unittest.TestCase):
    def test_only_bounded_approved_https_sources(self):
        good = "https://snapshot.vdotcameras.com/thumbs/a_B2.flv.png"
        self.assertTrue(camera.approved(good))
        self.assertFalse(camera.approved(good + "?secret=1"))
        self.assertFalse(camera.approved("http://snapshot.vdotcameras.com/thumbs/a_B2.flv.png"))
        self.assertFalse(camera.approved("https://bad.example/thumbs/a_B2.flv.png"))
        self.assertFalse(camera.approved("https://snapshot.vdotcameras.com.evil.net/thumbs/a.flv.png"))
        self.assertFalse(camera.approved("https://someone@snapshot.vdotcameras.com/thumbs/a.flv.png"))
        self.assertFalse(camera.approved("https://cwwp2.dot.ca.gov/private/camera.png"))
        self.assertTrue(camera.approved(
            "https://cwwp2.dot.ca.gov/data/d4/cctv/image/tv001/tv001.jpg"
        ))

    def test_probe_checks_actual_bytes_without_claiming_live(self):
        original = camera.OPENER
        try:
            fake = FakeOpener(FakeResponse(b"\x89PNG\r\n\x1a\n" + b"image" * 10))
            camera.OPENER = fake
            result = camera.probe({
                "id": "test",
                "city": "Test Town",
                "publisher": "VDOT",
                "image_url": "https://snapshot.vdotcameras.com/thumbs/fixture.flv.png",
            }, now=1700000000)
            self.assertEqual(result["status"], "reachable_freshness_unknown")
            self.assertEqual(result["content_type"], "image/png")
            self.assertIn("rb_probe=", fake.urls[0])
            self.assertNotIn("live", result["status"])
            camera.OPENER = FakeOpener(FakeResponse(b"no_image", "text/html"))
            self.assertEqual(camera.probe({
                "id": "bad", "image_url":
                "https://snapshot.vdotcameras.com/thumbs/fixture.flv.png"
            }, now=1700000000)["status"], "invalid_media_type")
            camera.OPENER = FakeOpener(FakeResponse(b"no_image", "image/jpeg"))
            self.assertEqual(camera.probe({
                "id": "bad", "image_url":
                "https://snapshot.vdotcameras.com/thumbs/fixture.flv.png"
            }, now=1700000000)["status"], "invalid_image_signature")
        finally:
            camera.OPENER = original

    def test_missing_and_unapproved_cameras_are_never_fetched(self):
        original = camera.OPENER
        try:
            fake = FakeOpener(FakeResponse(b""))
            camera.OPENER = fake
            self.assertEqual(camera.probe({"id": "gated", "image_url": ""}, 100)["status"],
                             "access_required")
            self.assertEqual(camera.probe({"id": "bad", "image_url": "https://bad.example/"}, 100)["status"],
                             "blocked_unapproved_endpoint")
            self.assertEqual(fake.urls, [])
        finally:
            camera.OPENER = original

    def test_local_state_api_only(self):
        for url in ("https://127.0.0.1:8765/api/state",
                    "http://10.0.0.1:8765/api/state",
                    "http://127.0.0.1:8765/internal",
                    "http://127.0.0.1:8765/api/state?url=https://example.org/"):
            with self.assertRaises(ValueError):
                camera.load_local_state(url)


if __name__ == "__main__":
    unittest.main()
