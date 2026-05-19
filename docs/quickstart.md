# QuickStart Guide

This guide walks you through common sqllog2db usage scenarios. Each scenario shows a complete workflow from config generation to output verification. For a minimal 3-command overview, see the [README](../README.md).

## Environment Preparation

Install Rust via rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Install sqllog2db:

```bash
cargo install dm-database-sqllog2db
```

Verify the installation:

```bash
sqllog2db --version
```

Alternative: build from source:

```bash
git clone https://github.com/guangl/sqllog2db
cd sqllog2db
cargo build --release
```

The binary is ~5 MB, statically linked, and located at `target/release/sqllog2db`.

Sample log files are included in the `sqllogs/` directory when cloned from source. Use your own DM SQL logs for production workloads.

---

## Scenario 1: Export SQL Logs to CSV

Export Dameng SQL logs to CSV for analysis or archival.

**Step 1: Generate a default config**

```bash
sqllog2db init -o config.toml --force
```

Expected output:

```
Config written to config.toml
```

**Step 2: Configure CSV export**

Edit `config.toml`:

```toml
[sqllog]
path = "sqllogs"

[exporter.csv]
file = "output/sqllog.csv"
overwrite = true
```

**Step 3: Validate the config**

```bash
sqllog2db validate -c config.toml
```

Expected output:

```
Config validation passed
```

**Step 4: Run the export**

```bash
sqllog2db run -c config.toml
```

Expected output:

```
[INFO] Starting export...
[INFO] Processing: sqllogs/DM_DMSQL_202504_01.log
[INFO] Processed 100000 records...
[INFO] Processed 200000 records...
[INFO] Export complete: 2,372,459 records in 8.87s (267,525 records/sec)
```

**Step 5: Verify the output**

```bash
wc -l output/sqllog.csv
head -5 output/sqllog.csv
```

**Troubleshooting:** If you see "Config validation failed: sqllog.path", ensure the `path` points to an existing directory or file. Use absolute paths for automation scripts.

---

## Scenario 2: Export to SQLite Database

Export to SQLite for SQL-based analysis.

**Step 1: Configure SQLite export**

Edit `config.toml`:

```toml
[sqllog]
path = "sqllogs"

[exporter.sqlite]
file = "output/sqllog.db"
table = "sqllog_records"
overwrite = true
```

**Step 2: Validate and run**

```bash
sqllog2db validate -c config.toml
sqllog2db run -c config.toml
```

**Step 3: Verify and query**

```bash
# Count total records
sqlite3 output/sqllog.db "SELECT COUNT(*) FROM sqllog_records;"

# Top 5 users by query count
sqlite3 output/sqllog.db "SELECT USERNAME, COUNT(*) AS cnt FROM sqllog_records GROUP BY USERNAME ORDER BY cnt DESC LIMIT 5;"

# Top 10 slowest queries
sqlite3 output/sqllog.db "SELECT SQL_TEXT, ELAPSED FROM sqllog_records ORDER BY ELAPSED DESC LIMIT 10;"
```

Expected output for top users:

```
HIHIS|1748411
BBP|513287
HINIS|83921
BLC|18342
SYSDBA|5234
```

**Troubleshooting:** If `sqlite3` is not found, install it with `brew install sqlite` (macOS) or your system package manager.

---

## Scenario 3: Per-File Statistics and Slow-Query Analysis

Analyze export results to identify performance patterns.

**Step 1: Run the stats command**

```bash
sqllog2db stats output/sqllog.csv --top-slow 10
```

Expected per-file statistics table:

```
File                                          Lines      Parsed    Errors    Elapsed
sqllogs/DM_DMSQL_202504_01.log                1,523,421  1,523,421  12        3.42s
sqllogs/DM_DMSQL_202504_03.log                1,487,233  1,487,233  8         3.21s
sqllogs/DM_DMSQL_202504_05.log                1,521,876  1,521,876  15        3.35s
```

Expected top-10 slowest queries:

```
Rank  SQL_TEXT                         ELAPSED(ms)  USERNAME   START_TIME
1     SELECT * FROM ORDERS WHERE ...   12,345       APP_USER   2025-04-15 14:23:01
2     INSERT INTO PAYMENTS ...         8,901        SYS_USER   2025-04-15 14:25:33
```

**Step 2: Group by dimensions**

```bash
# Group by user
sqllog2db stats output/sqllog.csv --group-by user

# Group by application
sqllog2db stats output/sqllog.csv --group-by app

# Filter by time range
sqllog2db stats output/sqllog.csv --from "2025-04-15" --to "2025-04-16"
```

Note: The `--group-by` flag uses lowercase values (`user`, `app`, `ip`). This differs from the `[filter]` config section which uses uppercase field names (`USERNAME`, `APPGROUP`, `IP_ADDRESS`). Refer to the [Config Reference](config-reference.md) for filter field naming.

Use this to identify performance bottlenecks, most active users, and error-prone log files.

---

## Scenario 4: SQL Template Aggregation and Chart Generation

Normalize SQL queries to identify structural patterns and generate SVG charts.

**Step 1: Enable template analysis and charts**

```toml
[sqllog]
path = "sqllogs"

[template]
enable = true
normalize_template = true
aggregator_mode = "hdrhistogram"
latency_buckets = [1, 5, 10, 50, 100, 500, 1000, 5000]

[charts]
output_dir = "charts/"
top_n = 10
frequency_bar = true
latency_hist = true
trend_line = true
user_pie = true

[exporter.csv]
file = "output/sqllog.csv"
overwrite = true
```

**Step 2: Run the export with template aggregation**

```bash
sqllog2db run -c config.toml
```

Expected additional output:

```
[INFO] Template aggregation: 245 unique SQL fingerprints
[INFO] Chart generated: charts/top_n_frequency.svg
[INFO] Chart generated: charts/latency_histogram_*.svg
[INFO] Chart generated: charts/frequency_trend.svg
[INFO] Chart generated: charts/user_schema_pie.svg
```

**Step 3: View the template summary**

```bash
# If using SQLite output
sqllog2db digest output/sqllog.db
```

Expected template summary:

```
Template                                        Count   Avg(ms)   P50(ms)   P95(ms)   P99(ms)
SELECT * FROM HI_BD_TASK_FU WHERE ID_TASK = ?   12,345  342       215       891       2,341
INSERT INTO HI_BD_SIPA_FU_RULE ...              8,901   156       120       445       980
```

**Step 4: Explore the output**

- `output/template_summary.csv` — CSV summary (if CSV exporter)
- `output/sqllog.db` — SQLite with `sqllog_records` and `_templates` tables
- `charts/` — SVG chart files (frequency bar, latency histogram, trend line, user pie)

Use template aggregation to understand SQL execution patterns, identify hot queries, and visualize workload distribution.

---

## Troubleshooting

### Config validation failed

- Run `sqllog2db validate -c config.toml` to see specific errors
- Check that `sqllog.path` exists and is readable
- Ensure output directories exist (sqllog2db does not create intermediate directories)
- Verify TOML syntax: section headers use `[brackets]`, values use `=`

### "No .log files found"

- Verify the path in `[sqllog].path` is correct
- Use absolute paths: `/home/user/logs/` instead of `../logs/`
- Check that files have the `.log` extension

### Slow export performance

- Ensure the pipeline is empty (no `[filter]`, `[template]`, or `[charts]` sections) for maximum speed
- CSV export is faster than SQLite (~5.2M vs ~1.1M records/sec)
- Use NVMe SSDs for best throughput
- File I/O is the primary bottleneck for large datasets

### Parse errors in output

- Parse errors are non-fatal: the tool continues processing
- Errors are logged to the application log (check `[logging]` config)
- GB18030/GBK encoded files are automatically detected and decoded
- Use `sqllog2db stats` to see per-file error counts

### Template aggregation produces too many templates

- Increase `top_n` in `[charts]` to show more templates
- Use `sqllog2db digest --min-count 100` to filter rare templates
- Add filters to narrow the data before template aggregation
