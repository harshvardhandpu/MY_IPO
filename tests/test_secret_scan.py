import json
import tempfile
import unittest
from pathlib import Path

from scripts.scan_secrets import FindingKind, scan_paths


class SecretScannerTests(unittest.TestCase):
    def test_rejects_generated_plaintext_pan_pattern(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            sample = Path(directory) / "event.json"
            generated_pan = "TESTP" + "1234" + "Z"
            sample.write_text(json.dumps({"pan": generated_pan}), encoding="utf-8")

            findings = scan_paths([sample])

            self.assertEqual([FindingKind.PAN], [item.kind for item in findings])

    def test_allows_masked_pan(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            sample = Path(directory) / "profile.md"
            sample.write_text("TESTP****Z", encoding="utf-8")

            self.assertEqual([], scan_paths([sample]))

    def test_rejects_generated_obvious_secret_assignment(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            sample = Path(directory) / "settings.txt"
            generated_key = "sk" + "-" + "example" + "1234567890"
            sample.write_text(f"api_key = '{generated_key}'", encoding="utf-8")

            findings = scan_paths([sample])

            self.assertEqual([FindingKind.API_KEY], [item.kind for item in findings])


if __name__ == "__main__":
    unittest.main()
