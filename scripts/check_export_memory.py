#!/usr/bin/env python3
"""Measure CLI peak RSS on macOS/Linux; fail on data loss or excess memory.

Example: python3 scripts/check_export_memory.py target/release/sqllog2db
Defaults reproduce 1, 4 and 16 input files of approximately 256 MiB each.
Fixtures and outputs are generated in a temporary directory and removed on exit.
"""
import argparse
import hashlib
import json
import os
import pathlib
import platform
import re
import signal
import subprocess
import sys
import tempfile
import time

LINE = (
    b"2025-01-15 10:30:28.001 (EP[0] sess:0x1 user:BENCH trxid:1 stmt:0x1 "
    b"appname:BenchApp ip:10.0.0.1) [SEL] SELECT col1, col2 FROM bench_table "
    b"WHERE id=1 AND status='active'. EXECTIME: 13(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n"
)
MIB = 1024 * 1024
HEADER = b"ts,ep,sess_id,thrd_id,username,trx_id,statement,appname,client_ip,tag,sql,exec_time_ms,row_count,exec_id\n"
CSV_ROW = (
    b'2025-01-15 10:30:28.001,0,0x1,,BENCH,1,0x1,BenchApp,10.0.0.1,SEL,'
    b'"SELECT col1, col2 FROM bench_table WHERE id=1 AND status=\'active\'. ",13,1,1\n'
)
def expected_csv_digest(rows):
    hasher = hashlib.sha256(HEADER)
    block = CSV_ROW * 8192
    for _ in range(rows // 8192):
        hasher.update(block)
    hasher.update(CSV_ROW * (rows % 8192))
    return hasher.hexdigest()


def run_timed(command, timeout):
    # Kill the whole time + exporter process group on timeout, not only time.
    with subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                          text=True, start_new_session=True) as proc:
        try:
            stdout, stderr = proc.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            os.killpg(proc.pid, signal.SIGKILL)
            proc.communicate()
            raise
        if proc.returncode:
            raise RuntimeError(f"export exited {proc.returncode}: {stdout} {stderr}")
    return stderr


def evaluate(result, baseline, max_rss, max_growth):
    reasons = []
    rss = result["peak_rss_bytes"]
    if rss <= 0 or rss > max_rss:
        reasons.append("peak RSS outside allowed range")
    if rss - baseline > max_growth:
        reasons.append("RSS growth exceeds limit")
    if result["records"] != result["expected_records"]:
        reasons.append("record count mismatch")
    if not result["content_valid"]:
        reasons.append("exported content mismatch")
    return reasons


def measure(binary, root, inputs, fmt, expected_rows, timeout):
    output = root / f"output.{fmt}"
    output.unlink(missing_ok=True)
    exporter = f'[exporter.csv]\nfile = "{output}"'
    cfg = root / "config.toml"
    cfg.write_text(
        '[sqllog]\ninputs = ' + json.dumps([str(p) for p in inputs])
        + '\n[logging]\nlevel = "warn"\n'
        + f'file = "{root / "app.log"}"\n'
        + exporter + '\noverwrite = true\nappend = false\n', encoding="utf-8"
    )
    if sys.platform == "darwin":
        command = ["/usr/bin/time", "-l"]
        pattern, unit = r"(\d+)\s+maximum resident set size", 1
    else:
        command = ["/usr/bin/time", "-f", "PEAK_RSS_KIB=%M"]
        pattern, unit = r"PEAK_RSS_KIB=(\d+)", 1024
    started = time.monotonic()
    stderr = run_timed(
        command + [str(binary), "run", "-c", str(cfg), "-q"], timeout,
    )
    elapsed = time.monotonic() - started
    match = re.search(pattern, stderr)
    if match is None:
        raise RuntimeError(f"Peak RSS missing from time output: {stderr}")
    count, hasher = -1, hashlib.sha256()  # Exclude header.
    with output.open("rb") as stream:
        while block := stream.read(MIB):
            count += block.count(b"\n")
            hasher.update(block)
    digest = hasher.hexdigest()
    content_valid = digest == expected_csv_digest(expected_rows)
    output.unlink()
    return dict(format=fmt, files=len(inputs), records=count,
                peak_rss_bytes=int(match[1]) * unit, csv_sha256=digest,
                content_valid=content_valid, elapsed_seconds=elapsed)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=pathlib.Path)
    parser.add_argument("--file-mib", type=int, default=256)
    parser.add_argument("--files", type=int, default=16)
    parser.add_argument("--max-rss-mib", type=int, default=128)
    parser.add_argument("--max-growth-mib", type=int, default=32)
    parser.add_argument("--timeout-seconds", type=int, default=600)
    parser.add_argument("--report", type=pathlib.Path)
    args = parser.parse_args()
    if sys.platform not in ("darwin", "linux"):
        parser.error("requires macOS time or GNU time on Linux")
    if min(args.file_mib, args.files, args.max_rss_mib, args.timeout_seconds) < 1:
        parser.error("sizes, timeout and file count must be positive")
    if args.max_growth_mib < 0:
        parser.error("growth limit must be nonnegative")
    binary = args.binary.resolve(strict=True)
    rows = args.file_mib * MIB // len(LINE)
    report = dict(
        schema_version=1, platform=platform.platform(),
        revision=os.environ.get("GITHUB_SHA"),
        binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
        file_mib=args.file_mib, file_counts=sorted({1, min(4, args.files), args.files}),
        max_rss_mib=args.max_rss_mib, max_growth_mib=args.max_growth_mib,
        timeout_seconds=args.timeout_seconds, cases=[], passed=False,
    )
    try:
        with tempfile.TemporaryDirectory(prefix="sqllog-memory-") as directory:
            root = pathlib.Path(directory)
            inputs = [root / f"{i}.log" for i in range(args.files)]
            block = LINE * 8192
            for path in inputs:
                with path.open("wb") as stream:
                    for _ in range(rows // 8192):
                        stream.write(block)
                    stream.write(LINE * (rows % 8192))
            for fmt in ("csv",):
                baseline = None
                for count in report["file_counts"]:
                    result = measure(binary, root, inputs[:count], fmt,
                                     rows * count, args.timeout_seconds)
                    result["expected_records"] = rows * count
                    if baseline is None:
                        baseline = result["peak_rss_bytes"]
                    result["growth_bytes"] = max(0, result["peak_rss_bytes"] - baseline)
                    result["failures"] = evaluate(
                        result, baseline, args.max_rss_mib * MIB, args.max_growth_mib * MIB,
                    )
                    result["passed"] = not result["failures"]
                    report["cases"].append(result)
                    print(json.dumps(result), flush=True)
            report["passed"] = all(case["passed"] for case in report["cases"])
    except (OSError, RuntimeError, subprocess.TimeoutExpired) as exc:
        report["error"] = str(exc)
        print(str(exc), file=sys.stderr)
    finally:
        if args.report:
            args.report.parent.mkdir(parents=True, exist_ok=True)
            args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
