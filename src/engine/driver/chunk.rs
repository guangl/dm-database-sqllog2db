//! 单文件切块：将一个超大日志文件按行边界切分为多个临时分片文件，
//! 使其可以复用现有的多文件 rayon 并行管线（`parallel::process_csv_parallel`），
//! 从而在"只有一个输入文件"的场景下也能获得文件内并行解析的收益。
//!
//! 安全前提（由调用方在切块前校验）：
//! - 未启用参数替换（`replace_parameters`）：PARAMS 缓存按 session/stmt 在整份文件范围内
//!   跨记录查找，切块会破坏跨块引用。
//! - 未配置事务级过滤器（`indicators` / `sql` includes/excludes）：这两类过滤器要求基于
//!   完整文件的预扫描结果做事务边界判断，切块后单个事务可能跨越分片边界。
//!
//! 每行即一条完整日志记录（无多行记录），因此按 `\n` 切分不会破坏记录完整性。

use crate::error::Result;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// 触发切块所需的最小文件大小：小文件切块的调度开销可能超过并行收益。
const MIN_CHUNK_SPLIT_SIZE: u64 = 64 * 1024 * 1024;

/// 切块产生的临时分片文件；`Drop` 时自动清理临时目录。
pub(crate) struct FileChunks {
    pub paths: Vec<PathBuf>,
    dir: PathBuf,
}

impl Drop for FileChunks {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// 判断文件是否足够大、值得切块；返回文件大小供 [`split_file_into_chunks`] 复用。
pub(crate) fn should_split(file: &Path, jobs: usize) -> Option<u64> {
    if jobs <= 1 {
        return None;
    }
    let size = std::fs::metadata(file).ok()?.len();
    if size < MIN_CHUNK_SPLIT_SIZE {
        return None;
    }
    Some(size)
}

/// 将 `file` 按行边界切分为最多 `jobs` 个临时分片，每片大小约为 `size / jobs`。
/// 最后一片吸收所有剩余行（包括因取整产生的余数）。
pub(crate) fn split_file_into_chunks(file: &Path, jobs: usize, size: u64) -> Result<FileChunks> {
    let stem = file.file_stem().unwrap_or_default().to_string_lossy();
    let dir_name = format!(".{stem}_chunks_{}", std::process::id());
    let preferred = file
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let dir = {
        let candidate = preferred.join(&dir_name);
        if std::fs::create_dir_all(&candidate).is_ok() {
            candidate
        } else {
            let fallback = std::env::temp_dir().join(&dir_name);
            std::fs::create_dir_all(&fallback)?;
            fallback
        }
    };

    let f = std::fs::File::open(file)?;
    let mut reader = BufReader::with_capacity(1024 * 1024, f);
    let target = (size / jobs as u64).max(1);
    let mut paths = Vec::with_capacity(jobs);
    let mut idx = 0usize;

    loop {
        let is_last_chunk = idx + 1 >= jobs;
        let chunk_path = dir.join(format!("{idx:04}.log"));
        let out = std::fs::File::create(&chunk_path)?;
        let mut writer = std::io::BufWriter::with_capacity(1024 * 1024, out);
        let mut written_this_chunk: u64 = 0;
        let mut line = Vec::with_capacity(256);
        loop {
            line.clear();
            let n = reader.read_until(b'\n', &mut line)?;
            if n == 0 {
                break;
            }
            writer.write_all(&line)?;
            written_this_chunk += n as u64;
            if !is_last_chunk && written_this_chunk >= target {
                break;
            }
        }
        writer.flush()?;
        drop(writer);
        if written_this_chunk == 0 {
            let _ = std::fs::remove_file(&chunk_path);
            break;
        }
        paths.push(chunk_path);
        idx += 1;
        if is_last_chunk {
            break;
        }
    }
    Ok(FileChunks { paths, dir })
}
