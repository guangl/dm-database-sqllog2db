//! Watch 监听目录收集与格式化显示。

use std::path::{Path, PathBuf};

/// 从 cfg.sqllog.inputs 收集实际存在的监听目录，去重后返回。
/// 路径经 canonicalize 处理以解决 macOS /var → /private/var 等符号链接问题。
#[must_use]
pub fn collect_watch_dirs(inputs: &[String]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    for input_str in inputs {
        let is_glob = input_str.contains('*') || input_str.contains('?') || input_str.contains('[');
        if is_glob {
            if let Some(ancestor) = Path::new(input_str).ancestors().find(|p| p.exists()) {
                let dir = ancestor
                    .canonicalize()
                    .unwrap_or_else(|_| ancestor.to_path_buf());
                if seen.insert(dir.clone()) {
                    dirs.push(dir);
                }
            }
        } else {
            let path = Path::new(input_str);
            if path.is_file() {
                if let Some(parent) = path.parent() {
                    let dir = parent
                        .canonicalize()
                        .unwrap_or_else(|_| parent.to_path_buf());
                    if seen.insert(dir.clone()) {
                        dirs.push(dir);
                    }
                }
            } else if path.is_dir() {
                let dir = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                if seen.insert(dir.clone()) {
                    dirs.push(dir);
                }
            }
        }
    }
    dirs
}

/// 格式化监听目录列表用于状态行显示。
pub(super) fn format_paths_display(dirs: &[PathBuf]) -> String {
    if dirs.len() > 3 {
        format!("{} directories", dirs.len())
    } else {
        dirs.iter()
            .map(|d| d.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}
