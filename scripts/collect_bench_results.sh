#!/usr/bin/env bash
# 收集 criterion 输出的 estimates.json，合并为单一 JSON artifact。
# 输出文件：bench-results-${SHA短}.json，包含 timestamp、commit_sha、benchmarks（每组 mean_ns + stddev_ns）。
# 依赖：ubuntu-latest 内置 jq、find、awk（无需额外安装）。

set -euo pipefail

SHA="${GITHUB_SHA:-$(git rev-parse HEAD)}"
SHORT_SHA="${SHA:0:8}"
TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
OUTPUT="bench-results-${SHORT_SHA}.json"

if [ ! -d "target/criterion" ]; then
    echo "ERROR: target/criterion not found; run cargo bench first" >&2
    exit 1
fi

# 每个 estimates.json 路径形如：target/criterion/<group>/<bench_id>/new/estimates.json
# 用 awk 提取末尾 4 段中的 <group>=NF-3, <bench_id>=NF-2，避开硬编码路径深度。
benchmarks_json="$(
    find target/criterion -name 'estimates.json' -path '*/new/estimates.json' \
    | while IFS= read -r f; do
        group="$(echo "$f" | awk -F/ '{print $(NF-3)}')"
        bench_id="$(echo "$f" | awk -F/ '{print $(NF-2)}')"
        jq --arg g "$group" --arg i "$bench_id" \
            '{key: ($g + "/" + $i), mean_ns: .mean.point_estimate, stddev_ns: .std_dev.point_estimate}' \
            "$f"
      done \
    | jq -s 'map({(.key): {mean_ns: .mean_ns, stddev_ns: .stddev_ns}}) | add // {}'
)"

jq -n \
    --arg ts "$TIMESTAMP" \
    --arg sha "$SHA" \
    --argjson benchmarks "$benchmarks_json" \
    '{timestamp: $ts, commit_sha: $sha, benchmarks: $benchmarks}' \
    > "$OUTPUT"

echo "Benchmark results written to $OUTPUT"
cat "$OUTPUT"
