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
                    reason: format!("invalid glob pattern: {e}. Check glob syntax (e.g. wildcards must not include unmatched brackets)"),
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
#[path = "../tests/unit/parser.rs"]
mod tests;
