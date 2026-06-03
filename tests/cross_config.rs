use std::fs;

fn load_non_comment_lines(content: &str) -> Vec<String> {
    content
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter(|line| !line.trim().is_empty())
        .map(std::string::ToString::to_string)
        .collect()
}

fn is_valid_sha256_hex(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

#[test]
fn cross_toml_has_exactly_one_sha256_image_reference() {
    let content = fs::read_to_string("Cross.toml").expect("Cross.toml must exist at project root");

    let non_comment_lines = load_non_comment_lines(&content);
    let sha256_lines: Vec<&String> = non_comment_lines
        .iter()
        .filter(|line| line.contains("@sha256:"))
        .collect();

    assert_eq!(
        sha256_lines.len(),
        1,
        "expected exactly 1 non-comment line containing '@sha256:', found {}:\n{:?}",
        sha256_lines.len(),
        sha256_lines,
    );
}

#[test]
fn cross_toml_has_no_floating_edge_tag() {
    let content = fs::read_to_string("Cross.toml").expect("Cross.toml must exist at project root");

    let non_comment_lines = load_non_comment_lines(&content);
    let edge_lines: Vec<&String> = non_comment_lines
        .iter()
        .filter(|line| line.contains(":edge"))
        .collect();

    assert_eq!(
        edge_lines.len(),
        0,
        "expected zero non-comment lines containing ':edge', found {}:\n{:?}",
        edge_lines.len(),
        edge_lines,
    );
}

#[test]
fn cross_toml_sha256_digest_is_valid_64_hex_chars() {
    let content = fs::read_to_string("Cross.toml").expect("Cross.toml must exist at project root");

    let non_comment_lines = load_non_comment_lines(&content);
    let sha256_line = non_comment_lines
        .iter()
        .find(|line| line.contains("@sha256:"))
        .expect("expected one non-comment line containing '@sha256:', found none");

    let digest_start = sha256_line
        .find("@sha256:")
        .expect("line must contain '@sha256:'")
        + "@sha256:".len();

    let raw_after_prefix = &sha256_line[digest_start..];
    let digest = raw_after_prefix
        .trim_end_matches('"')
        .trim_end_matches('\'')
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches('"')
        .trim_matches('\'');

    assert!(
        is_valid_sha256_hex(digest),
        "expected a 64-char lowercase hex SHA256 digest after '@sha256:', got: {:?} (length {})",
        digest,
        digest.len(),
    );
}
