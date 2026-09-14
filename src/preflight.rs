// 整个模块仅在 binary crate (main.rs) 中使用；lib crate 生产代码不调用。
// 其 `#[cfg(test)]` 单元测试直接引用当前模块中的 items，编译不受影响。

use crate::config::Config;
use crate::parser::SqllogParser;
use std::path::Path;

/// 在 run 命令执行前检查基础条件。
/// 返回所有警告/错误，调用方决定是否中止。
#[must_use]
pub fn check(cfg: &Config) -> PreflightResult {
    let mut result = PreflightResult::default();
    for input in &cfg.sqllog.inputs {
        check_log_path(input, &mut result);
    }
    check_output_writable(cfg, &mut result);
    result
}

fn check_log_path(path_str: &str, result: &mut PreflightResult) {
    let has_glob = path_str.contains('*') || path_str.contains('?') || path_str.contains('[');

    // For non-glob paths, check existence before trying to scan
    if !has_glob {
        let path = Path::new(path_str);
        if !path.exists() {
            result.errors.push(format!(
                "日志路径不存在: {path_str}  (检查 [sqllog].inputs 或 --input 标志)"
            ));
            return;
        }
    }

    match SqllogParser::new(vec![path_str.to_string()]).log_files() {
        Ok(files) if files.is_empty() => {
            result
                .warnings
                .push(format!("路径 {path_str} 中未找到 .log 文件"));
        }
        Ok(_) => {}
        Err(e) => {
            result.errors.push(format!("扫描日志路径失败: {e}"));
        }
    }
}

fn check_output_writable(cfg: &Config, result: &mut PreflightResult) {
    use crate::config::exporter::ActiveExporter;
    let file = match cfg.exporter.active() {
        Some(ActiveExporter::Parquet(config)) => &config.file,
        Some(ActiveExporter::Csv(config)) => &config.file,
        None => return,
    };
    check_path_writable(file, result);
}

fn check_path_writable(file_path: &str, result: &mut PreflightResult) {
    let path = Path::new(file_path);

    // 若父目录不存在，先尝试创建；创建失败则直接报错，无需继续检查文件。
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty())
        && !parent.exists()
    {
        if std::fs::create_dir_all(parent).is_err() {
            result
                .errors
                .push(format!("无法创建输出目录: {}", parent.display()));
        }
        return;
    }

    // 用单次 open（create + write）镜像导出器实际行为，消除 exists() → open() 的 TOCTOU 竞争。
    // truncate(false)：preflight 仅验证可写性，不截断已有文件。
    if std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(path)
        .is_err()
    {
        result.errors.push(format!("输出文件不可写: {file_path}"));
    }
}

#[derive(Debug, Default)]
pub struct PreflightResult {
    pub(crate) errors: Vec<String>,
    pub(crate) warnings: Vec<String>,
}

impl PreflightResult {
    #[must_use]
    pub(crate) fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 打印所有警告和错误，返回是否有致命错误。
    #[must_use]
    pub fn print_and_check(&self) -> bool {
        for warn in &self.warnings {
            eprintln!("Warning: {warn}");
        }
        for err in &self.errors {
            eprintln!("Error: {err}");
        }
        self.has_errors()
    }
}

#[cfg(test)]
#[path = "../tests/unit/preflight.rs"]
mod tests;
