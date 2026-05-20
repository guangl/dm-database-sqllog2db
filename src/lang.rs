//! Internationalization: Chinese / English switching.
//!
//! Language priority (highest first):
//!   1. `--lang` CLI flag
//!   2. `SQLLOG2DB_LANG` environment variable
//!   3. System `LANG` / `LC_ALL` / `LANGUAGE` environment variables
//!   4. Default: English
//!
//! `detect`, `apply_zh` 及各个 `zh_*` 辅助函数仅在 binary crate (main.rs) 中使用；
//! lib crate 生产代码不调用。`#[cfg(test)]` 中的单元测试直接引用当前模块 items，编译不受影响。
#![allow(dead_code)]

use clap::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Zh,
}

impl Lang {
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().trim_matches('"') {
            "zh" | "zh_cn" | "zh_tw" | "zh_hk" | "chinese" => Some(Self::Zh),
            "en" | "en_us" | "en_gb" | "english" => Some(Self::En),
            _ => None,
        }
    }
}

/// Detect language from environment variables only (no CLI args).
fn from_env() -> Lang {
    if let Ok(v) = std::env::var("SQLLOG2DB_LANG") {
        if let Some(lang) = Lang::parse(&v) {
            return lang;
        }
    }
    let system = std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .or_else(|_| std::env::var("LANGUAGE"))
        .unwrap_or_default();
    if system.to_lowercase().starts_with("zh") {
        Lang::Zh
    } else {
        Lang::En
    }
}

/// Pre-scan raw CLI args for `--lang <value>` before clap parses.
fn from_args(args: &[String]) -> Option<Lang> {
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "--lang" {
            return iter.next().and_then(|v| Lang::parse(v));
        }
        if let Some(v) = arg.strip_prefix("--lang=") {
            return Lang::parse(v);
        }
    }
    None
}

/// Determine the effective language: CLI flag > env var > system locale > English.
#[must_use]
pub(crate) fn detect(args: &[String]) -> Lang {
    from_args(args).unwrap_or_else(from_env)
}

// ── Clap command localization ─────────────────────────────────────────────────

/// Apply Chinese help strings to the clap `Command` tree.
/// Called only when `lang == Lang::Zh`; the default command is already English.
#[must_use]
pub(crate) fn apply_zh(cmd: Command) -> Command {
    cmd.about("解析达梦数据库 SQL 日志并导出到 CSV / SQLite")
        .long_about(
            "高性能 CLI 工具：流式解析达梦（DM）数据库 SQL 日志，导出到 CSV 或 SQLite。\n\n\
             适用场景：日志归档、数据分析预处理、基于日志的审计与问责。",
        )
        .mut_arg("verbose", |a| a.help("详细输出（debug 级别）"))
        .mut_arg("quiet", |a| a.help("静默模式（仅显示错误，隐藏进度条）"))
        .mut_arg("no_color", |a| {
            a.help("禁用颜色输出（也支持 NO_COLOR 环境变量）")
        })
        .mut_arg("lang", |a| {
            a.help("界面语言：zh | en（默认自动检测 LANG 环境变量）")
        })
        .mut_subcommand("run", zh_run)
        .mut_subcommand("init", zh_init)
        .mut_subcommand("validate", zh_validate)
        .mut_subcommand("show-config", zh_show_config)
}

fn zh_common_config_args(s: Command) -> Command {
    s.mut_arg("config", |a| a.help("配置文件路径"))
        .mut_arg("set", |a| {
            a.help("覆盖配置字段，如 --set exporter.csv.file=out.csv")
        })
}

fn zh_run(s: Command) -> Command {
    zh_common_config_args(s)
        .about("运行日志导出任务")
        .mut_arg("limit", |a| a.help("最多处理 N 条记录后停止（跨文件累计）"))
        .mut_arg("dry_run", |a| a.help("只解析不写文件（统计记录数）"))
        .mut_arg("from", |a| a.help("只保留此时间戳之后（含）的记录"))
        .mut_arg("to", |a| a.help("只保留此时间戳之前（含）的记录"))
        .mut_arg("output", |a| {
            a.help("CSV 输出文件（等同于 --set exporter.csv.file=<FILE>）")
        })
        .mut_arg("progress_interval", |a| {
            a.help("进度条刷新间隔（毫秒，默认 80）")
        })
}

fn zh_init(s: Command) -> Command {
    s.about("生成默认配置文件")
        .mut_arg("output", |a| a.help("输出配置文件路径"))
        .mut_arg("force", |a| a.help("若文件已存在则强制覆盖"))
}

fn zh_validate(s: Command) -> Command {
    zh_common_config_args(s).about("验证配置文件是否合法")
}

fn zh_show_config(s: Command) -> Command {
    zh_common_config_args(s)
        .about("显示当前生效配置（含 --set 覆盖后的值）")
        .mut_arg("diff", |a| a.help("高亮与默认配置不同的字段"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::opts::Cli;
    use clap::CommandFactory;

    #[test]
    fn test_lang_parse_zh_variants() {
        assert_eq!(Lang::parse("zh"), Some(Lang::Zh));
        assert_eq!(Lang::parse("ZH"), Some(Lang::Zh));
        assert_eq!(Lang::parse("zh_cn"), Some(Lang::Zh));
        assert_eq!(Lang::parse("zh_tw"), Some(Lang::Zh));
        assert_eq!(Lang::parse("zh_hk"), Some(Lang::Zh));
        assert_eq!(Lang::parse("chinese"), Some(Lang::Zh));
    }

    #[test]
    fn test_lang_parse_en_variants() {
        assert_eq!(Lang::parse("en"), Some(Lang::En));
        assert_eq!(Lang::parse("EN"), Some(Lang::En));
        assert_eq!(Lang::parse("en_us"), Some(Lang::En));
        assert_eq!(Lang::parse("en_gb"), Some(Lang::En));
        assert_eq!(Lang::parse("english"), Some(Lang::En));
    }

    #[test]
    fn test_lang_parse_invalid() {
        assert_eq!(Lang::parse("fr"), None);
        assert_eq!(Lang::parse(""), None);
        assert_eq!(Lang::parse("ja"), None);
    }

    #[test]
    fn test_detect_with_lang_flag_zh() {
        let args: Vec<String> = vec!["--lang".into(), "zh".into()];
        assert_eq!(detect(&args), Lang::Zh);
    }

    #[test]
    fn test_detect_with_lang_flag_en() {
        let args: Vec<String> = vec!["--lang".into(), "en".into()];
        assert_eq!(detect(&args), Lang::En);
    }

    #[test]
    fn test_detect_with_lang_equals_form() {
        let args: Vec<String> = vec!["--lang=zh".into()];
        assert_eq!(detect(&args), Lang::Zh);
    }

    #[test]
    fn test_detect_with_invalid_lang_falls_through() {
        // --lang=invalid → from_args returns None → falls through to from_env
        let args: Vec<String> = vec!["--lang=invalid".into()];
        let result = detect(&args);
        // Result depends on env; just verify it doesn't panic and returns a valid Lang
        assert!(result == Lang::En || result == Lang::Zh);
    }

    #[test]
    fn test_detect_no_args_uses_env() {
        // No --lang flag → calls from_env; result depends on environment
        let result = detect(&[]);
        assert!(result == Lang::En || result == Lang::Zh);
    }

    #[test]
    fn test_detect_skips_unrelated_args() {
        let args: Vec<String> = vec!["run".into(), "-c".into(), "config.toml".into()];
        let result = detect(&args);
        assert!(result == Lang::En || result == Lang::Zh);
    }

    #[test]
    fn test_apply_zh_smoke() {
        // Verify apply_zh can be called without panicking and returns a Command
        let cmd = Cli::command();
        let zh_cmd = apply_zh(cmd);
        // Check that the about string was updated
        assert!(
            zh_cmd
                .get_about()
                .is_some_and(|s| s.to_string().contains("达梦"))
        );
    }

    #[test]
    fn test_lang_default_is_en() {
        assert_eq!(Lang::default(), Lang::En);
    }
}
