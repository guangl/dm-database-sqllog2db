/// 去除 IPv4-mapped IPv6 地址前缀（如 `::ffff:192.168.1.1` → `192.168.1.1`）
#[inline]
#[must_use]
pub(crate) fn strip_ip_prefix(ip: &str) -> &str {
    const PREFIX: &str = "::ffff:";
    // 快速路径：IPv4 地址以数字开头，不以 ':' 开头，直接返回
    if ip.as_bytes().first() != Some(&b':') {
        return ip;
    }
    if ip.len() > PREFIX.len() && ip[..PREFIX.len()].eq_ignore_ascii_case(PREFIX) {
        &ip[PREFIX.len()..]
    } else {
        ip
    }
}

/// Saturating cast from f32 milliseconds to i64 milliseconds without precision-loss warnings
#[inline]
#[must_use]
pub(crate) fn f32_ms_to_i64(ms: f32) -> i64 {
    const MAX_I64_F64: f64 = 9_223_372_036_854_775_807.0; // i64::MAX as f64
    const MIN_I64_F64: f64 = -9_223_372_036_854_775_808.0; // i64::MIN as f64

    if !ms.is_finite() {
        return 0;
    }

    let ms_f64 = f64::from(ms);
    if ms_f64 > MAX_I64_F64 {
        i64::MAX
    } else if ms_f64 < MIN_I64_F64 {
        i64::MIN
    } else {
        let clamped = ms_f64.round();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "value is clamped to i64 range above; saturating cast (Rust 1.45+) handles boundary values correctly"
        )]
        {
            clamped as i64
        }
    }
}

/// 确保输出文件的父目录存在
pub(crate) fn ensure_parent_dir(path: &std::path::Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.exists()) {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}
