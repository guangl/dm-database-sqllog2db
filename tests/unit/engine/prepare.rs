use super::*;
use std::path::PathBuf;

fn write_temp_file(dir: &std::path::Path, name: &str, size: usize) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, vec![b'a'; size]).unwrap();
    path
}

#[test]
fn test_requested_jobs_of_one_is_unaffected() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_temp_file(dir.path(), "a.log", 10 * 1024 * 1024);
    assert_eq!(
        effective_jobs_for_memory_budget(&[f], 1, 2 * 1024 * 1024 * 1024),
        1
    );
}

#[test]
fn test_small_files_keep_requested_jobs() {
    let dir = tempfile::tempdir().unwrap();
    let files: Vec<_> = (0..8)
        .map(|i| write_temp_file(dir.path(), &format!("f{i}.log"), 1024 * 1024))
        .collect();
    // 1MB files * 3x amplification = 3MB per task; well within a 2GB budget at 8 jobs.
    assert_eq!(
        effective_jobs_for_memory_budget(&files, 8, 2 * 1024 * 1024 * 1024),
        8
    );
}

#[test]
fn test_large_files_cap_jobs_below_requested() {
    let dir = tempfile::tempdir().unwrap();
    // Each file is 500MB; amplified to 1.5GB per task. A 2GB budget allows only 1 concurrent task.
    let files: Vec<_> = (0..8)
        .map(|i| write_temp_file(dir.path(), &format!("big{i}.log"), 500 * 1024 * 1024))
        .collect();
    let jobs = effective_jobs_for_memory_budget(&files, 8, 2 * 1024 * 1024 * 1024);
    assert_eq!(jobs, 1);
}

#[test]
fn test_single_huge_file_falls_back_to_one_job_not_zero() {
    let dir = tempfile::tempdir().unwrap();
    // 4GB nominal size (amplified to 12GB) vastly exceeds a 2GB budget; must not return 0.
    let f = write_temp_file(dir.path(), "huge.log", 4096); // small actual file, fake huge via metadata is not possible portably
    let jobs = effective_jobs_for_memory_budget(&[f], 8, 1); // budget of 1 byte forces the floor
    assert_eq!(jobs, 1, "must always return at least 1 job");
}

#[test]
fn test_missing_files_are_ignored_and_default_to_requested_jobs() {
    let missing = PathBuf::from("/nonexistent/path/should/not/exist.log");
    assert_eq!(
        effective_jobs_for_memory_budget(&[missing], 8, 2 * 1024 * 1024 * 1024),
        8
    );
}
