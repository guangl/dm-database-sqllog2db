//! Watch 单元测试 + WATCH-07/08/09 集成测试（从 mod.rs 迁移）。

use super::debounce::should_trigger;
use super::dirs::collect_watch_dirs;
use super::handler::handle_watch;
use super::state::{DEBOUNCE_WINDOW, WatchLoopState};
use super::status::{format_elapsed_hms, render_active_status};
use super::trigger_full::trigger_full_file;
use super::trigger_incremental::{read_bytes_to_tempfile, trigger_incremental};
use crate::config::Config;
use crate::error::Error;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

#[test]
fn test_interrupted_flag_exits_immediately() {
    let cfg = Config::default();
    let interrupted = Arc::new(AtomicBool::new(true));
    let result = handle_watch(&cfg, true, false, &interrupted);
    // 默认 Config 的 sqllog.inputs = ["sqllogs"]，该目录不存在时返回 Err
    // 但 interrupted=true 时如果目录存在则跳出 loop 并返回 Err(Interrupted)（WATCH-09）
    // 我们只验证函数能正常返回（不 panic）
    let _ = result;
}

#[test]
fn test_watch_loop_state_new_with_offsets() {
    let mut initial_offsets = HashMap::new();
    initial_offsets.insert(PathBuf::from("/tmp/test.log"), 1024u64);
    let state = WatchLoopState::new(initial_offsets.clone(), Some("test.db".to_string()));
    assert_eq!(
        state.file_offsets.get(&PathBuf::from("/tmp/test.log")),
        Some(&1024u64)
    );
    assert_eq!(state.sqlite_db_url, Some("test.db".to_string()));
    assert_eq!(state.trigger_count, 0);
}

#[test]
fn test_watch_loop_state_new_without_sqlite() {
    let state = WatchLoopState::new(HashMap::new(), None);
    assert!(state.file_offsets.is_empty());
    assert!(state.sqlite_db_url.is_none());
}

#[test]
fn test_collect_watch_dirs_nonexistent_glob_returns_empty() {
    let dirs = collect_watch_dirs(&["nonexistent_dir_xyz/*.log".to_string()]);
    assert!(
        dirs.is_empty(),
        "nonexistent glob parent should yield empty Vec, got: {dirs:?}"
    );
}

#[test]
fn test_collect_watch_dirs_existing_dir_returns_itself() {
    let tmp = tempfile::TempDir::new().expect("failed to create tempdir");
    let dir_path = tmp.path().to_string_lossy().into_owned();
    let dirs = collect_watch_dirs(&[dir_path]);
    assert_eq!(dirs.len(), 1, "existing dir should yield exactly 1 entry");
    assert_eq!(
        dirs[0].canonicalize().ok(),
        tmp.path().canonicalize().ok(),
        "returned dir should match the temp dir"
    );
}

#[test]
fn test_format_elapsed_hms_zero() {
    assert_eq!(
        format_elapsed_hms(Duration::from_secs(0)),
        "00:00:00",
        "zero seconds should format as 00:00:00"
    );
}

#[test]
fn test_format_elapsed_hms_3661_seconds() {
    assert_eq!(
        format_elapsed_hms(Duration::from_secs(3661)),
        "01:01:01",
        "3661 seconds should format as 01:01:01"
    );
}

// --- 新增 4 个测试（Task 1 gap closure） ---

#[test]
fn test_should_trigger_first_call_admits_and_records() {
    let mut map: HashMap<PathBuf, Instant> = HashMap::new();
    let path = PathBuf::from("/tmp/test.log");
    let now = Instant::now();
    let result = should_trigger(&path, &mut map, now, DEBOUNCE_WINDOW);
    assert!(result, "首次调用应返回 true（允许触发）");
    assert!(
        map.contains_key(&path),
        "首次触发后，路径应写入 debounce_map"
    );
    let recorded = map[&path];
    assert_eq!(recorded, now, "记录的时间戳应等于传入的 now");
}

#[test]
fn test_should_trigger_within_window_rejects() {
    let mut map: HashMap<PathBuf, Instant> = HashMap::new();
    let path = PathBuf::from("/tmp/test2.log");
    let t0 = Instant::now();

    // 首次触发
    let first = should_trigger(&path, &mut map, t0, DEBOUNCE_WINDOW);
    assert!(first, "首次调用应返回 true");

    // 100ms 内再次触发（窗口期内应被抑制）
    let t1 = t0 + Duration::from_millis(100);
    let second = should_trigger(&path, &mut map, t1, DEBOUNCE_WINDOW);
    assert!(!second, "窗口期 100ms 内再次触发应被抑制（返回 false）");

    // 600ms 后触发（窗口期外应被允许）
    let t2 = t0 + Duration::from_millis(600);
    let third = should_trigger(&path, &mut map, t2, DEBOUNCE_WINDOW);
    assert!(third, "窗口期外（600ms > 500ms）应允许触发（返回 true）");
    // 表项应更新为 t2
    assert_eq!(map[&path], t2, "允许触发后，表项应更新为最新时间戳");
}

#[test]
fn test_should_trigger_isolates_distinct_paths() {
    let mut map: HashMap<PathBuf, Instant> = HashMap::new();
    let path1 = PathBuf::from("/tmp/a.log");
    let path2 = PathBuf::from("/tmp/b.log");
    let t0 = Instant::now();

    // P1 在 t0 触发
    let _ = should_trigger(&path1, &mut map, t0, DEBOUNCE_WINDOW);

    // P2 在 t0+100ms 触发（不同路径，不受 P1 窗口影响）
    let t1 = t0 + Duration::from_millis(100);
    let result = should_trigger(&path2, &mut map, t1, DEBOUNCE_WINDOW);
    assert!(result, "不同路径（P2）不受 P1 防抖窗口影响，应返回 true");
}

#[test]
fn test_render_active_status_includes_human_duration() {
    // Duration::from_secs(0) => HumanDuration 输出 "0s" 或 "just now"
    let status_zero = render_active_status("/foo", 3, 42, Duration::from_secs(0));
    assert!(
        status_zero.contains("triggers: 3"),
        "状态行应包含 triggers: 3，实际: {status_zero}"
    );
    assert!(
        status_zero.contains("processed: 42 rows"),
        "状态行应包含 processed: 42 rows，实际: {status_zero}"
    );
    // HumanDuration(0) 输出 "0s" 或 "just now"，均含 "s" 或 "now"
    assert!(
        status_zero.contains("last: "),
        "状态行应包含 last: 前缀，实际: {status_zero}"
    );

    // Duration::from_secs(125) => HumanDuration 应包含 "m"（分钟单位）
    let status_125 = render_active_status("/foo", 1, 1, Duration::from_secs(125));
    assert!(
        status_125.contains('m'),
        "125 秒的 HumanDuration 输出应包含 'm'（分钟），实际: {status_125}"
    );
}

// --- Task 2: trigger_incremental + read_bytes_to_tempfile 测试 ---

/// 测试 `read_bytes_to_tempfile`：从 `start_offset` 处读取，临时文件仅含新增字节。
#[test]
fn test_read_bytes_to_tempfile_reads_from_offset() {
    use std::io::Read;
    let mut source_file = tempfile::NamedTempFile::new().expect("failed to create tmp");
    let content = b"Hello World! This is a test file with 50 bytes content!";
    std::io::Write::write_all(&mut source_file, content).unwrap();
    let source_path = source_file.path().to_path_buf();
    let start_offset = 13u64; // skip "Hello World! "
    let tmp_result = read_bytes_to_tempfile(&source_path, start_offset)
        .expect("read_bytes_to_tempfile should succeed");
    let mut actual_content = Vec::new();
    std::fs::File::open(tmp_result.path())
        .unwrap()
        .read_to_end(&mut actual_content)
        .unwrap();
    let start_idx = usize::try_from(start_offset).expect("offset fits in usize");
    assert_eq!(
        actual_content,
        &content[start_idx..],
        "临时文件应仅包含 start_offset 之后的字节"
    );
}

/// 测试 `read_bytes_to_tempfile`：`start_offset` == 文件大小时，临时文件为空。
#[test]
fn test_read_bytes_to_tempfile_at_eof_produces_empty() {
    use std::io::Read;
    let mut source_file = tempfile::NamedTempFile::new().expect("failed to create tmp");
    let content = b"some content here";
    std::io::Write::write_all(&mut source_file, content).unwrap();
    let source_path = source_file.path().to_path_buf();
    let start_offset = content.len() as u64; // at EOF
    let tmp_result = read_bytes_to_tempfile(&source_path, start_offset)
        .expect("read_bytes_to_tempfile at EOF should succeed");
    let mut actual_content = Vec::new();
    std::fs::File::open(tmp_result.path())
        .unwrap()
        .read_to_end(&mut actual_content)
        .unwrap();
    assert!(
        actual_content.is_empty(),
        "offset == 文件大小时，临时文件应为空，实际: {actual_content:?}"
    );
}

/// 测试 `trigger_incremental`：`new_size` <= `start_offset` 时跳过，`trigger_count` 不增。
#[test]
fn test_trigger_incremental_skips_if_no_new_bytes() {
    let source_file = tempfile::NamedTempFile::new().expect("failed to create tmp");
    let content = vec![0u8; 100];
    std::fs::write(source_file.path(), &content).unwrap();
    let canonical_path = source_file.path().canonicalize().unwrap();
    let mut initial_offsets = HashMap::new();
    // file_offsets 记录 offset = 100（等于文件大小）
    initial_offsets.insert(canonical_path, 100u64);
    let mut state = WatchLoopState::new(initial_offsets, None);
    let cfg = Config::default();
    let interrupted = Arc::new(AtomicBool::new(false));
    let pb = indicatif::ProgressBar::hidden();
    trigger_incremental(
        source_file.path(),
        &cfg,
        true,
        false,
        &interrupted,
        &mut state,
        &pb,
    );
    assert_eq!(
        state.trigger_count, 0,
        "new_size == start_offset 时 trigger_count 不应增加"
    );
}

/// 测试 `trigger_incremental`：首次 Modify 无 `file_offsets` 记录时，记录基线并跳过处理。
#[test]
fn test_trigger_incremental_first_seen_records_baseline() {
    let source_file = tempfile::NamedTempFile::new().expect("failed to create tmp");
    let content = vec![0u8; 100];
    std::fs::write(source_file.path(), &content).unwrap();
    let canonical_path = source_file.path().canonicalize().unwrap();
    // file_offsets 中无该路径记录
    let mut state = WatchLoopState::new(HashMap::new(), None);
    let cfg = Config::default();
    let interrupted = Arc::new(AtomicBool::new(false));
    let pb = indicatif::ProgressBar::hidden();
    trigger_incremental(
        source_file.path(),
        &cfg,
        true,
        false,
        &interrupted,
        &mut state,
        &pb,
    );
    // 应记录基线 offset = 文件大小（100），但不处理
    assert_eq!(
        state.trigger_count, 0,
        "首次 Modify 应跳过处理，trigger_count 不增"
    );
    assert_eq!(
        state.file_offsets.get(&canonical_path),
        Some(&100u64),
        "首次 Modify 应将当前文件大小作为基线写入 file_offsets"
    );
}

// ── WATCH-07/08/09 Wave 0 Tests ─────────────────────────────────────────

/// 一条合法的达梦 SQL 日志行（含 header + footer），用于 WATCH-07/08 测试。
const DM_LOG_LINE_SEL: &str = "2024-01-15 10:00:00.123 (EP[0] sess:0x0001 thrd:456 user:SYSDBA trxid:1001 stmt:select_001 appname:test ip:127.0.0.1) [SEL] select * from t1. EXECTIME: 5(ms) ROWCOUNT: 10(rows) EXEC_ID: 1.\n";
const DM_LOG_LINE_SEL2: &str = "2024-01-15 10:00:01.456 (EP[0] sess:0x0002 thrd:457 user:SYSDBA trxid:1002 stmt:select_002 appname:test ip:127.0.0.1) [SEL] select * from t2. EXECTIME: 3(ms) ROWCOUNT: 5(rows) EXEC_ID: 2.\n";
const DM_LOG_LINE_GARBAGE: &str = "this is not a valid dm sql log line at all\n";

// WATCH-07: CSV watch 追加，多次 trigger_full_file 后 CSV 行数累计，header 仅一行
#[test]
fn test_watch_csv_append() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_a = dir.path().join("a.log");
    let log_b = dir.path().join("b.log");
    let csv_path = dir.path().join("out.csv");

    std::fs::write(&log_a, DM_LOG_LINE_SEL).unwrap();
    std::fs::write(&log_b, DM_LOG_LINE_SEL2).unwrap();

    // Config: CSV exporter，初始 append=false、overwrite=true（trigger_full_file 内部会注入 append=true）
    let toml = format!(
        "[sqllog]\ninputs = [\"{logdir}\"]\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n",
        logdir = dir.path().to_string_lossy().replace('\\', "/"),
        csv = csv_path.to_string_lossy().replace('\\', "/"),
    );
    let cfg: crate::config::Config = toml::from_str(&toml).unwrap();
    let interrupted = Arc::new(AtomicBool::new(false));
    let pb = indicatif::ProgressBar::hidden();
    let mut state = WatchLoopState::new(HashMap::new(), None);

    trigger_full_file(&log_a, &cfg, true, false, &interrupted, &mut state, &pb);
    trigger_full_file(&log_b, &cfg, true, false, &interrupted, &mut state, &pb);

    assert!(csv_path.exists(), "CSV 文件应在触发后存在");
    let content = std::fs::read_to_string(&csv_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    // 应有 1 header + 2 data rows（来自 A 和 B 各 1 条）
    assert!(
        lines.len() >= 3,
        "CSV 应有 header + 2 data rows，实际: {} 行，内容:\n{content}",
        lines.len()
    );
    // header 只出现一次（append 模式不重复写 header）
    let header = lines[0];
    let header_count = lines.iter().filter(|&&l| l == header).count();
    assert_eq!(
        header_count, 1,
        "header 行应只出现一次，实际出现 {header_count} 次"
    );
}

// WATCH-08: error log 追加，两次 trigger_full_file 后 error log 含所有历史错误
#[test]
fn test_watch_error_log_append() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_a = dir.path().join("a.log");
    let log_b = dir.path().join("b.log");
    let csv_path = dir.path().join("out.csv");
    let error_log_path = dir.path().join("errors.log");

    // 每个日志文件都有一条解析失败行（触发 write_error_log）+ 一条合法行（保证 handle_run 不提前失败）
    let content_a = format!("{DM_LOG_LINE_GARBAGE}{DM_LOG_LINE_SEL}");
    let content_b = format!("{DM_LOG_LINE_GARBAGE}{DM_LOG_LINE_SEL2}");
    std::fs::write(&log_a, &content_a).unwrap();
    std::fs::write(&log_b, &content_b).unwrap();

    let toml = format!(
        "[sqllog]\ninputs = [\"{logdir}\"]\n[error]\nfile = \"{errlog}\"\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n",
        logdir = dir.path().to_string_lossy().replace('\\', "/"),
        errlog = error_log_path.to_string_lossy().replace('\\', "/"),
        csv = csv_path.to_string_lossy().replace('\\', "/"),
    );
    let cfg: crate::config::Config = toml::from_str(&toml).unwrap();
    let interrupted = Arc::new(AtomicBool::new(false));
    let pb = indicatif::ProgressBar::hidden();
    let mut state = WatchLoopState::new(HashMap::new(), None);

    trigger_full_file(&log_a, &cfg, true, false, &interrupted, &mut state, &pb);
    trigger_full_file(&log_b, &cfg, true, false, &interrupted, &mut state, &pb);

    assert!(error_log_path.exists(), "error log 应在有解析错误时创建");
    let error_content = std::fs::read_to_string(&error_log_path).unwrap();
    // 两次触发各有一条解析失败行，error log 应包含两条 [ERROR] 记录
    let error_line_count = error_content
        .lines()
        .filter(|l| l.starts_with("[ERROR]"))
        .count();
    assert!(
        error_line_count >= 2,
        "error log 应包含 2 条 [ERROR] 行（来自 A 和 B 各 1 次触发），实际: {error_line_count} 条\n内容:\n{error_content}"
    );
}

// WATCH-09: interrupted=true 时 handle_watch 返回 Err(Error::Interrupted)
#[test]
fn test_handle_watch_returns_interrupted() {
    let dir = tempfile::TempDir::new().unwrap();
    let csv_path = dir.path().join("out.csv");
    let toml = format!(
        "[sqllog]\ninputs = [\"{logdir}\"]\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n",
        logdir = dir.path().to_string_lossy().replace('\\', "/"),
        csv = csv_path.to_string_lossy().replace('\\', "/"),
    );
    let cfg: crate::config::Config = toml::from_str(&toml).unwrap();
    let interrupted = Arc::new(AtomicBool::new(true));
    let result = handle_watch(&cfg, true, false, &interrupted);
    assert!(
        matches!(result, Err(Error::Interrupted)),
        "interrupted=true 时 handle_watch 应返回 Err(Error::Interrupted)，实际: {result:?}"
    );
}
