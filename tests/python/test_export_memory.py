"""Failure-path tests for the release gate (no large fixtures required)."""
import contextlib
import hashlib
import io
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2] / "scripts"))
import check_export_memory as gate


class GateTests(unittest.TestCase):
    def result(self, **updates):
        result = dict(peak_rss_bytes=32 * gate.MIB, records=10,
                      expected_records=10, content_valid=True)
        result.update(updates)
        return result

    def test_accepts_exact_limits(self):
        self.assertEqual([], gate.evaluate(self.result(peak_rss_bytes=128 * gate.MIB),
                                         96 * gate.MIB, 128 * gate.MIB, 32 * gate.MIB))

    def test_rejects_peak_even_without_growth(self):
        rss = 128 * gate.MIB + 1
        self.assertIn("peak RSS outside allowed range",
                      gate.evaluate(self.result(peak_rss_bytes=rss), rss,
                                    128 * gate.MIB, 32 * gate.MIB))

    def test_rejects_growth_below_absolute_limit(self):
        self.assertIn("RSS growth exceeds limit",
                      gate.evaluate(self.result(peak_rss_bytes=65 * gate.MIB),
                                    32 * gate.MIB, 128 * gate.MIB, 32 * gate.MIB))

    def test_rejects_bad_data_and_missing_rss(self):
        for changes in (dict(records=9), dict(records=11), dict(content_valid=False),
                        dict(peak_rss_bytes=0)):
            with self.subTest(changes=changes):
                self.assertTrue(gate.evaluate(self.result(**changes),
                                              32 * gate.MIB, 128 * gate.MIB, 32 * gate.MIB))

    def test_expected_digest_across_block_boundary(self):
        for rows in (0, 1, 8191, 8192, 8193):
            self.assertEqual(hashlib.sha256(gate.HEADER + gate.CSV_ROW * rows).hexdigest(),
                             gate.expected_csv_digest(rows))

    def test_process_nonzero_exit_fails(self):
        with self.assertRaisesRegex(RuntimeError, "exited 7"):
            gate.run_timed([sys.executable, "-c", "raise SystemExit(7)"], 10)

    def test_timeout_fails_and_terminates_exporter(self):
        with self.assertRaises(subprocess.TimeoutExpired):
            gate.run_timed([sys.executable, "-c", "import time; time.sleep(60)"], 0.1)

    def test_missing_measurement_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            with mock.patch.object(gate, "run_timed", return_value="no measurement"):
                with self.assertRaisesRegex(RuntimeError, "Peak RSS missing"):
                    gate.measure(pathlib.Path(sys.executable), pathlib.Path(directory),
                                 [], "csv", 1, 10)

    def test_cli_fails_closed_and_writes_report(self):
        for error in (RuntimeError("missing RSS"), OSError("disk full"),
                      subprocess.TimeoutExpired("export", 1)):
            with self.subTest(error=error), tempfile.TemporaryDirectory() as directory:
                report = pathlib.Path(directory) / "report.json"
                argv = ["check", sys.executable, "--file-mib", "1", "--files", "1",
                        "--report", str(report)]
                with mock.patch.object(sys, "argv", argv), \
                     mock.patch.object(gate, "measure", side_effect=error), \
                     contextlib.redirect_stderr(io.StringIO()):
                    self.assertEqual(1, gate.main())
                result = json.loads(report.read_text())
                self.assertFalse(result["passed"])
                self.assertIn("error", result)


if __name__ == "__main__":
    unittest.main()
