use crate::config::{LOG_LEVELS, LoggingConfig};
use crate::error::{Error, FileError, Result};
use log::{Level, LevelFilter, Metadata, Record};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::fs::OpenOptions;
use std::io::Write as IoWrite;
use std::path::Path;
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// 使用 LazyLock 缓存日志级别映射表，避免每次查找时重新构建
static LOG_LEVEL_MAP: LazyLock<HashMap<&'static str, LevelFilter>> = LazyLock::new(|| {
    let mut map = HashMap::with_capacity(5);
    map.insert("trace", LevelFilter::Trace);
    map.insert("debug", LevelFilter::Debug);
    map.insert("info", LevelFilter::Info);
    map.insert("warn", LevelFilter::Warn);
    map.insert("error", LevelFilter::Error);
    map
});

/// Manual UTC timestamp formatter using `std::time::SystemTime`.
///
/// Replaces an external datetime dependency with a pure-std implementation.
/// Uses Howard Hinnant's civil-from-days algorithm to convert Unix seconds
/// to year/month/day without any external crates.
fn format_utc_timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // try_from 溢出仅在 Unix 秒数超过 i64::MAX（约 2.9×10^11 年）时发生；回退到 epoch 起点。
    let days = i64::try_from(secs / 86400).unwrap_or(0);
    let rem = i64::try_from(secs % 86400).unwrap_or(0);

    let hours = rem / 3600;
    let rem = rem % 3600;
    let mins = rem / 60;
    let secs_part = rem % 60;

    // civil_from_days: days since Unix epoch (1970-01-01) → year/month/day
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let y = y + i64::from(m <= 2);

    let mut buf = String::with_capacity(19);
    write!(
        buf,
        "{y:04}-{m:02}-{d:02} {hours:02}:{mins:02}:{secs_part:02}",
    )
    .unwrap(); // infallible: writing to a String never fails
    buf
}

/// 自定义简单 Logger：写入文件，可选同时输出到 stdout。
struct SimpleLogger {
    level: LevelFilter,
    file: Mutex<std::fs::File>,
    log_to_stdout: bool,
}

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        match self.level {
            LevelFilter::Off => false,
            LevelFilter::Error => metadata.level() == Level::Error,
            LevelFilter::Warn => metadata.level() <= Level::Warn,
            LevelFilter::Info => metadata.level() <= Level::Info,
            LevelFilter::Debug => metadata.level() <= Level::Debug,
            LevelFilter::Trace => true,
        }
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let now = format_utc_timestamp();
        let msg = format!(
            "[{}][{}] {} - {}\n",
            now,
            record.level(),
            record.target(),
            record.args()
        );
        if self.log_to_stdout {
            let _ = std::io::stdout().write_all(msg.as_bytes());
        }

        // 写到文件
        if let Ok(mut f) = self.file.lock() {
            let _ = f.write_all(msg.as_bytes());
        }
    }

    fn flush(&self) {}
}

/// 初始化日志系统
///
/// `log_to_stdout`: 是否同时向 stdout 输出日志。进度条模式下应传 `false`，
/// 避免日志输出干扰进度条渲染。
pub fn init_logging(config: &LoggingConfig, log_to_stdout: bool) -> Result<()> {
    // 解析日志级别
    let level = parse_log_level(&config.level)?;
    // 获取日志文件路径和目录
    let log_path = Path::new(&config.file);
    let parent_dir = log_path.parent().ok_or_else(|| {
        Error::File(FileError::CreateDirectoryFailed {
            path: log_path.to_path_buf(),
            reason: "Failed to get parent directory".to_string(),
        })
    })?;

    // 创建日志目录（如果不存在）
    if !parent_dir.exists() {
        std::fs::create_dir_all(parent_dir).map_err(|e| {
            Error::File(FileError::CreateDirectoryFailed {
                path: parent_dir.to_path_buf(),
                reason: e.to_string(),
            })
        })?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|e| {
            Error::File(FileError::CreateDirectoryFailed {
                path: log_path.to_path_buf(),
                reason: e.to_string(),
            })
        })?;

    let logger = SimpleLogger {
        level,
        file: Mutex::new(file),
        log_to_stdout,
    };

    // 注册 logger
    match log::set_boxed_logger(Box::new(logger)) {
        Ok(()) => {
            log::set_max_level(level);
        }
        Err(_) => {
            eprintln!(
                "warning: logging already initialized; config {:?} ignored",
                config.file
            );
        }
    }

    log::info!(
        "Logging initialized - level: {:?}, file: {}, retention_days: {}",
        level,
        config.file,
        config.retention_days
    );

    Ok(())
}

/// 解析日志级别字符串
fn parse_log_level(level_str: &str) -> Result<LevelFilter> {
    let lower = level_str.to_lowercase();
    LOG_LEVEL_MAP.get(lower.as_str()).copied().ok_or_else(|| {
        Error::Config(crate::error::ConfigError::InvalidLogLevel {
            level: level_str.to_string(),
            valid_levels: LOG_LEVELS.iter().map(|s| (*s).to_string()).collect(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LoggingConfig;

    fn make_logging_config(dir: &std::path::Path, level: &str) -> LoggingConfig {
        LoggingConfig {
            file: dir.join("app.log").to_str().unwrap().to_string(),
            level: level.to_string(),
            retention_days: 7,
        }
    }

    #[test]
    fn test_init_logging_valid_level_info() {
        let dir = tempfile::TempDir::new().unwrap();
        let cfg = make_logging_config(dir.path(), "info");
        // May fail silently if logger already registered in this process; that is intentional
        let result = init_logging(&cfg, false);
        assert!(result.is_ok());
        assert!(dir.path().join("app.log").exists());
    }

    #[test]
    fn test_init_logging_invalid_level_returns_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let cfg = make_logging_config(dir.path(), "nonsense_level");
        let result = init_logging(&cfg, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_init_logging_creates_parent_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let nested = dir.path().join("sub/nested");
        let cfg = LoggingConfig {
            file: nested.join("app.log").to_str().unwrap().to_string(),
            level: "warn".to_string(),
            retention_days: 7,
        };
        let result = init_logging(&cfg, false);
        assert!(result.is_ok());
        assert!(nested.exists());
    }

    #[test]
    fn test_init_logging_all_valid_levels() {
        let dir = tempfile::TempDir::new().unwrap();
        for level in &["trace", "debug", "info", "warn", "error"] {
            let cfg = LoggingConfig {
                file: dir
                    .path()
                    .join(format!("{level}.log"))
                    .to_str()
                    .unwrap()
                    .to_string(),
                level: (*level).to_string(),
                retention_days: 7,
            };
            assert!(init_logging(&cfg, false).is_ok());
        }
    }

    #[test]
    fn test_init_logging_with_stdout() {
        let dir = tempfile::TempDir::new().unwrap();
        let cfg = make_logging_config(dir.path(), "warn");
        // log_to_stdout=true exercises the stdout write path in SimpleLogger::log
        let result = init_logging(&cfg, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_log_level_all() {
        for level in &["trace", "debug", "info", "warn", "error"] {
            assert!(parse_log_level(level).is_ok());
        }
    }

    #[test]
    fn test_parse_log_level_uppercase() {
        // parse_log_level lowercases, so uppercase should also work
        assert!(parse_log_level("INFO").is_ok());
        assert!(parse_log_level("DEBUG").is_ok());
    }

    #[test]
    fn test_parse_log_level_invalid() {
        assert!(parse_log_level("verbose").is_err());
        assert!(parse_log_level("").is_err());
    }
}
