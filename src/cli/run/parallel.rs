use crate::error::{Error, ErrorStats, Result};
use crate::exporter::{CsvExporter, ExporterManager};
use crate::pipeline::{FieldMask, Pipeline};
use dm_database_parser_sqllog::Sqllog;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::collector;

/// 每个并行任务的返回值：`Some((orig_path, temp_path, count, file_stats))` 或 `None`（跳过/中断）。
type TaskResult = Option<(PathBuf, PathBuf, usize, ErrorStats)>;

/// 将 N 个已处理的临时 CSV 文件按顺序拼接到最终输出路径。
/// 第一个文件保留 header；后续文件跳过第一行。
/// `append_to_existing`=true 时所有文件都跳过 header（目标文件已有 header）。
pub(super) fn concat_csv_parts(
    parts: &[(PathBuf, usize)],
    output_path: &Path,
    overwrite: bool,
    append_to_existing: bool,
) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::BufReader;

    // 无任何 part 时不触碰输出文件，避免 overwrite=true 把已有数据清空。
    if parts.is_empty() {
        return Ok(());
    }

    let file = if append_to_existing {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(output_path)?
    } else {
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(overwrite)
            .open(output_path)?
    };
    let mut writer = std::io::BufWriter::with_capacity(2 * 1024 * 1024, file);

    let mut parts_to_remove: Vec<&Path> = Vec::with_capacity(parts.len());
    for (idx, (part_path, _)) in parts.iter().enumerate() {
        let part_file = std::fs::File::open(part_path)?;
        let mut reader = BufReader::new(part_file);

        // 第一个 part（且非追加模式）保留 header；其余情况跳过 header 行
        let skip_header = idx > 0 || append_to_existing;
        if skip_header {
            // 用 Vec<u8> + read_until 而非 String + read_line：
            // 省去 UTF-8 验证，预分配避免 header 超 capacity 时的二次分配。
            let mut discard = Vec::with_capacity(256);
            std::io::BufRead::read_until(&mut reader, b'\n', &mut discard)?;
        }

        std::io::copy(&mut reader, &mut writer)?;
        // reader 在此处离开作用域并 drop，文件句柄关闭后才删除（Windows 兼容性）
        parts_to_remove.push(part_path.as_path());
    }

    use std::io::Write as _;
    // 先 flush 确保所有数据已落盘，再删除临时文件；flush 失败时保留临时文件便于诊断
    writer.flush()?;
    for p in parts_to_remove {
        if let Err(e) = std::fs::remove_file(p) {
            log::warn!("failed to remove temp part {}: {e}", p.display());
        }
    }
    Ok(())
}

/// 准备临时 CSV parts 目录，与输出文件相邻（避免跨设备 copy）。
/// 若父目录不可写，退回到系统临时目录。
fn setup_parts_dir(output_path: &Path) -> Result<PathBuf> {
    let stem = output_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let dir_name = format!(".{stem}_parts_{}", std::process::id());
    let preferred = output_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(preferred)?;
    let candidate = preferred.join(&dir_name);
    if std::fs::create_dir_all(&candidate).is_ok() {
        Ok(candidate)
    } else {
        let fallback = std::env::temp_dir().join(&dir_name);
        std::fs::create_dir_all(&fallback)?;
        Ok(fallback)
    }
}

/// 将单文件收集到的记录 Vec 写入临时 CSV 文件。
///
/// 返回实际写入的记录数（等于 `rows.len()`）。
fn write_records_to_csv(
    rows: Vec<(Sqllog, Option<String>)>,
    temp_path: &Path,
    include_performance_metrics: bool,
    do_normalize: bool,
    field_mask: FieldMask,
    ordered_indices: &[usize],
) -> Result<usize> {
    let mut exporter = CsvExporter::new(temp_path);
    exporter.normalize = do_normalize;
    exporter.field_mask = field_mask;
    exporter.ordered_indices = ordered_indices.to_vec();
    exporter.include_performance_metrics = include_performance_metrics;
    let mut em = ExporterManager::from_csv(exporter);
    em.initialize()?;
    let include_pm = em.csv_include_performance_metrics();
    let count = rows.len();
    for (record, normalized) in rows {
        em.export_one_preparsed(&record, include_pm, normalized.as_deref())?;
    }
    em.finalize()?;
    Ok(count)
}

/// 并行解析所有文件并将每个文件记录写入独立临时 CSV。
///
/// # 内存注意
/// 按 D-11，CSV 并行路径先通过 `collector::collect_log_file` 将单文件所有过滤后记录
/// 收集到 `Vec<(Sqllog, Option<String>)>`，再写入临时 CSV。单文件内存占用约
/// `records × (sizeof(Sqllog) + Option<String>)` 字节；超大文件场景可在后续
/// `ParallelRunConfig` 重构时切换回流式写入。
#[allow(clippy::too_many_arguments)]
fn run_parallel_tasks(
    log_files: &[PathBuf],
    csv_include_performance_metrics: bool,
    pipeline: &Pipeline,
    jobs: usize,
    interrupted: &Arc<AtomicBool>,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    field_mask: FieldMask,
    ordered_indices: &[usize],
    parts_dir: &Path,
) -> Result<Vec<Result<TaskResult>>> {
    use rayon::prelude::*;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .map_err(|e| Error::Io(std::io::Error::other(e)))?;
    let results: Vec<Result<TaskResult>> = pool.install(|| {
        log_files
            .par_iter()
            .enumerate()
            .map(|(idx, file)| {
                if interrupted.load(Ordering::Relaxed) {
                    return Ok(None);
                }
                let temp_path = parts_dir.join(format!("{idx:08}.csv"));
                let (rows, parse_errors) = collector::collect_log_file(
                    file,
                    pipeline,
                    do_normalize,
                    placeholder_override,
                    interrupted,
                )?;
                let count = write_records_to_csv(
                    rows,
                    &temp_path,
                    csv_include_performance_metrics,
                    do_normalize,
                    field_mask,
                    ordered_indices,
                )?;
                let mut file_stats = ErrorStats::default();
                for _ in 0..parse_errors {
                    file_stats.add_parse_error();
                }
                Ok(Some((file.clone(), temp_path, count, file_stats)))
            })
            .collect()
    });
    Ok(results)
}

/// 收集并行任务结果，分离成功项与错误，合并错误统计。
///
/// 若任何任务失败，清理已生成的临时 part 文件并返回首个错误（`parts_dir` 目录本身由调用方清理）。
fn collect_parallel_results(
    results: Vec<Result<TaskResult>>,
) -> Result<(Vec<(PathBuf, PathBuf, usize)>, ErrorStats, usize)> {
    let mut parts_info: Vec<(PathBuf, PathBuf, usize)> = Vec::with_capacity(results.len());
    let mut parallel_stats = ErrorStats::default();
    let mut first_err: Option<Error> = None;
    let mut skipped = 0usize;
    for result in results {
        match result {
            Ok(Some((orig, temp, count, file_stats))) => {
                parallel_stats.merge(&file_stats);
                parts_info.push((orig, temp, count));
            }
            Ok(None) => skipped += 1,
            Err(e) => {
                log::warn!("parallel collect error: {e}");
                if first_err.is_none() {
                    first_err = Some(e);
                }
            }
        }
    }
    if let Some(e) = first_err {
        for (_, temp, _) in &parts_info {
            let _ = std::fs::remove_file(temp);
        }
        return Err(e);
    }
    Ok((parts_info, parallel_stats, skipped))
}

/// 拼接并行生成的 CSV parts 到最终输出文件，并清理临时目录。
///
/// 返回 `(per_file_counts, skipped, parallel_stats)` 供 `handle_run` 消费。
fn finalize_concat(
    parts_info: Vec<(PathBuf, PathBuf, usize)>,
    output_path: &Path,
    overwrite: bool,
    append_to_existing: bool,
    parts_dir: &Path,
    skipped: usize,
    parallel_stats: ErrorStats,
) -> Result<(Vec<(PathBuf, usize)>, usize, ErrorStats)> {
    let parts_for_concat: Vec<(PathBuf, usize)> = parts_info
        .iter()
        .map(|(_, temp, count)| (temp.clone(), *count))
        .collect();
    let concat_result = concat_csv_parts(
        &parts_for_concat,
        output_path,
        overwrite,
        append_to_existing,
    );
    // 无论拼接成功与否都清理临时目录，避免磁盘满等错误导致残留
    let _ = std::fs::remove_dir_all(parts_dir);
    // 拼接失败且非追加模式时，删除已部分写入的输出文件，避免遗留截断的 CSV
    if concat_result.is_err() && !append_to_existing {
        let _ = std::fs::remove_file(output_path);
    }
    concat_result?;
    Ok((
        parts_info
            .into_iter()
            .map(|(orig, _, count)| (orig, count))
            .collect(),
        skipped,
        parallel_stats,
    ))
}

/// 并行 CSV 处理：每个文件独立跑在 rayon 线程上，各写一个临时 CSV，
/// 最终按文件原始顺序拼接成一个完整 CSV。
///
/// 返回：`(已处理文件列表, 跳过文件数, 解析错误统计)`，已处理列表顺序与 `log_files` 一致。
/// 适用条件：CSV 导出 + 多文件 + jobs > 1 + 无 limit。
/// 注意：并行模式暂不支持每文件进度显示；外层 `run_csv_parallel` 的 verbose eprintln 已足够。
#[allow(clippy::too_many_arguments)]
pub(super) fn process_csv_parallel(
    log_files: &[PathBuf],
    cfg: &crate::config::Config,
    pipeline: &Pipeline,
    jobs: usize,
    interrupted: &Arc<AtomicBool>,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    field_mask: FieldMask,
    ordered_indices: &[usize],
) -> Result<(Vec<(PathBuf, usize)>, usize, ErrorStats)> {
    let csv_cfg = cfg
        .exporter
        .csv
        .as_ref()
        // infallible: process_csv_parallel is only called when CSV exporter is present
        .expect("parallel CSV requires CSV exporter");
    let output_path = Path::new(&csv_cfg.file);
    let append_to_existing = csv_cfg.append && output_path.exists();
    let parts_dir = setup_parts_dir(output_path)?;
    let results = run_parallel_tasks(
        log_files,
        csv_cfg.include_performance_metrics,
        pipeline,
        jobs,
        interrupted,
        do_normalize,
        placeholder_override,
        field_mask,
        ordered_indices,
        &parts_dir,
    )?;
    let (parts_info, parallel_stats, skipped) = match collect_parallel_results(results) {
        Ok(v) => v,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&parts_dir);
            return Err(e);
        }
    };
    finalize_concat(
        parts_info,
        output_path,
        csv_cfg.overwrite,
        append_to_existing,
        &parts_dir,
        skipped,
        parallel_stats,
    )
}
