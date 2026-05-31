/// SQL 日志解析模块
/// 使用 dm-database-parser-sqllog 库解析达梦数据库的 SQL 日志文件
use crate::error::{Error, ParserError, Result};
use log::{debug, info, warn};
use std::path::{Path, PathBuf};

/// SQL 日志解析器
#[derive(Debug)]
pub(crate) struct SqllogParser {
    /// 日志输入列表（每条可为文件路径、目录路径或 glob 模式）
    inputs: Vec<String>,
}

impl SqllogParser {
    /// 创建新的 SQL 日志解析器，接受多个输入路径/模式
    pub(crate) fn new(inputs: Vec<String>) -> Self {
        Self { inputs }
    }

    /// 返回所有日志文件的路径列表（已合并去重排序）
    /// 空结果返回 Ok(空 Vec)；NoFilesFound 由 `handle_run` 层触发
    pub(crate) fn log_files(&self) -> Result<Vec<PathBuf>> {
        let mut all = Vec::new();
        for input in &self.inputs {
            let mut files = Self::expand_single(input)?;
            all.append(&mut files);
        }
        all.sort();
        all.dedup();
        Ok(all)
    }

    /// 展开单条 input（文件/目录/glob 模式）为日志文件列表
    fn expand_single(input: &str) -> Result<Vec<PathBuf>> {
        // Glob 模式检测
        if input.contains('*') || input.contains('?') || input.contains('[') {
            return Self::scan_glob(input);
        }

        let path = Path::new(input);

        if !path.exists() {
            return Err(Error::Parser(ParserError::PathNotFound {
                path: PathBuf::from(input),
            }));
        }

        let mut log_files = Vec::new();

        if path.is_file() {
            // 单个文件
            info!("Parsing single log file: {}", path.display());
            log_files.push(path.to_path_buf());
        } else if path.is_dir() {
            // 目录：扫描所有 .log 文件
            info!("Scanning log directory: {}", path.display());

            let entries = std::fs::read_dir(path).map_err(|e| {
                Error::Parser(ParserError::ReadDirFailed {
                    path: PathBuf::from(input),
                    reason: e.to_string(),
                })
            })?;

            for entry in entries {
                let entry = entry.map_err(|e| {
                    Error::Parser(ParserError::ReadDirFailed {
                        path: PathBuf::from(input),
                        reason: e.to_string(),
                    })
                })?;

                let entry_path = entry.path();

                if entry_path.is_file() && entry_path.extension().is_some_and(|ext| ext == "log") {
                    debug!("Found log file: {}", entry_path.display());
                    log_files.push(entry_path);
                }
            }

            if log_files.is_empty() {
                warn!("No .log files found in directory {}", path.display());
            } else {
                info!("Found {} log files", log_files.len());
            }
        } else {
            return Err(Error::Parser(ParserError::InvalidPath {
                path: PathBuf::from(input),
                reason: "既不是文件也不是目录".to_string(),
                line_number: None,
            }));
        }

        log_files.sort();
        Ok(log_files)
    }

    /// 使用 glob 模式扫描日志文件
    fn scan_glob(pattern: &str) -> Result<Vec<PathBuf>> {
        // Windows 路径用反斜杠，glob crate 只接受正斜杠，统一替换
        #[cfg(windows)]
        let pattern_normalized = pattern.replace('\\', "/");
        #[cfg(not(windows))]
        let pattern_normalized = pattern.to_owned();
        let pattern = pattern_normalized.as_str();

        let mut log_files: Vec<PathBuf> = glob::glob(pattern)
            .map_err(|e| {
                Error::Parser(ParserError::InvalidPath {
                    path: PathBuf::from(pattern),
                    reason: format!("invalid glob pattern: {e}"),
                    line_number: None,
                })
            })?
            .filter_map(std::result::Result::ok)
            .filter(|p| p.is_file() && p.extension().is_some_and(|ext| ext == "log"))
            .collect();

        log_files.sort();

        if log_files.is_empty() {
            warn!("No .log files matched glob pattern: {pattern}");
        } else {
            info!(
                "Glob matched {} log files for pattern: {pattern}",
                log_files.len()
            );
        }

        Ok(log_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_files_nonexistent_path() {
        let p = SqllogParser::new(vec!["/this/does/not/exist/at/all".to_string()]);
        assert!(p.log_files().is_err());
    }

    #[test]
    fn test_log_files_empty_directory() {
        let dir = tempfile::TempDir::new().unwrap();
        let p = SqllogParser::new(vec![dir.path().to_string_lossy().into_owned()]);
        let files = p.log_files().unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_log_files_with_log_file() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.log"), "").unwrap();
        let p = SqllogParser::new(vec![dir.path().to_string_lossy().into_owned()]);
        let files = p.log_files().unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_log_files_ignores_non_log_files() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.log"), "").unwrap();
        std::fs::write(dir.path().join("test.txt"), "").unwrap();
        std::fs::write(dir.path().join("test.csv"), "").unwrap();
        let p = SqllogParser::new(vec![dir.path().to_string_lossy().into_owned()]);
        let files = p.log_files().unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_log_files_single_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("single.log");
        std::fs::write(&file_path, "").unwrap();
        let p = SqllogParser::new(vec![file_path.to_string_lossy().into_owned()]);
        let files = p.log_files().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], file_path);
    }

    #[test]
    fn test_log_files_sorted() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("c.log"), "").unwrap();
        std::fs::write(dir.path().join("a.log"), "").unwrap();
        std::fs::write(dir.path().join("b.log"), "").unwrap();
        let p = SqllogParser::new(vec![dir.path().to_string_lossy().into_owned()]);
        let files = p.log_files().unwrap();
        assert_eq!(files.len(), 3);
        let names: Vec<_> = files
            .iter()
            .map(|f| f.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["a.log", "b.log", "c.log"]);
    }

    #[test]
    fn test_log_files_glob_pattern() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("2025-01.log"), "").unwrap();
        std::fs::write(dir.path().join("2025-02.log"), "").unwrap();
        std::fs::write(dir.path().join("other.txt"), "").unwrap();
        let pattern = format!("{}/*.log", dir.path().display());
        let p = SqllogParser::new(vec![pattern]);
        let files = p.log_files().unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_log_files_glob_no_match() {
        let dir = tempfile::TempDir::new().unwrap();
        let pattern = format!("{}/nomatch*.log", dir.path().display());
        let p = SqllogParser::new(vec![pattern]);
        let files = p.log_files().unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_log_files_invalid_glob_pattern() {
        // '[' without closing ']' is an invalid glob pattern
        let p = SqllogParser::new(vec!["/tmp/[invalid".to_string()]);
        let result = p.log_files();
        assert!(result.is_err());
    }

    #[test]
    fn test_log_files_multi_input_merge_and_dedup() {
        let base = tempfile::TempDir::new().unwrap();
        let dir_a = base.path().join("a");
        let dir_b = base.path().join("b");
        std::fs::create_dir_all(&dir_a).unwrap();
        std::fs::create_dir_all(&dir_b).unwrap();
        std::fs::write(dir_a.join("x.log"), "").unwrap();
        std::fs::write(dir_b.join("y.log"), "").unwrap();

        let input_a = dir_a.to_string_lossy().into_owned();
        let input_b = dir_b.to_string_lossy().into_owned();
        // 重复 dir_a：dedup 应过滤掉
        let p = SqllogParser::new(vec![input_a.clone(), input_b, input_a]);
        let files = p.log_files().unwrap();
        assert_eq!(
            files.len(),
            2,
            "dedup should produce 2 files, got: {files:?}"
        );
        // 验证字典序排序
        assert!(files[0] < files[1]);
    }

    #[test]
    fn test_log_files_multi_input_mixes_file_dir_glob() {
        let base = tempfile::TempDir::new().unwrap();
        let dir1 = base.path().join("dir1");
        let dir2 = base.path().join("dir2");
        std::fs::create_dir_all(&dir1).unwrap();
        std::fs::create_dir_all(&dir2).unwrap();

        let single_log = base.path().join("single.log");
        std::fs::write(&single_log, "").unwrap();
        std::fs::write(dir1.join("a.log"), "").unwrap();
        std::fs::write(dir2.join("c.log"), "").unwrap();

        let glob_pattern = format!("{}/*.log", dir2.display());
        let p = SqllogParser::new(vec![
            single_log.to_string_lossy().into_owned(),
            dir1.to_string_lossy().into_owned(),
            glob_pattern,
        ]);
        let files = p.log_files().unwrap();
        assert_eq!(files.len(), 3, "expected 3 files, got: {files:?}");
    }
}
