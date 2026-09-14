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
