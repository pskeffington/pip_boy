#!/usr/bin/env python3
"""Offline validation of metadata-only Northeast DOT endpoint checks."""
import unittest
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import probe_northeast_sources as probe
import build_northeast_camera_index as index


class EndpointProbeTests(unittest.TestCase):
    def test_good_metadata_is_not_misclassified_as_live(self):
        original = index.fetch_public
        try:
            index.fetch_public = lambda name: {"features": [{
                "attributes": {"OBJECTID": 1, "location": "I-95 at MD 43",
                               "lat": 39.42, "long": -76.4, "feedID": "C012"}
            }]}
            record = probe.check_source("md-imap")
            self.assertEqual(record["status"], "metadata_reachable")
            self.assertEqual(record["camera_locations"], 1)
            self.assertEqual(record["image_streams_verified"], 0)
            self.assertFalse(record["upstream_may_have_more"])
        finally:
            index.fetch_public = original

    def test_unavailable_provider_reports_type_without_credentials(self):
        original = index.fetch_public
        try:
            def blocked(name):
                raise OSError("source unreachable")
            index.fetch_public = blocked
            record = probe.check_source("ridot")
            self.assertEqual(record["status"], "metadata_error")
            self.assertEqual(record["error_type"], "OSError")
            self.assertNotIn("source unreachable", str(record))
        finally:
            index.fetch_public = original

    def test_staging_and_unlisted_sources_are_not_probed(self):
        self.assertNotIn("massdot-staging", probe.SOURCES)
        for name in ["massdot-staging", "nycdot", "private-router", ""]:
            with self.assertRaises(ValueError):
                probe.check_source(name)

    def test_all_default_probes_are_metadata_only(self):
        self.assertEqual(probe.DEFAULT_SOURCES, ("md-socrata", "md-imap", "ddot", "ridot"))
        self.assertTrue(set(probe.DEFAULT_SOURCES).issubset(index.SOURCES))
        self.assertTrue(all(index.SOURCES[x].get("fetch_allowed") is not False
                            for x in probe.SOURCES))


if __name__ == "__main__":
    unittest.main()
