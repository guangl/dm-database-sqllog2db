use super::*;

#[test]
fn test_pipeline_empty() {
    let p = Pipeline::new();
    assert!(p.is_empty());
}

#[test]
fn test_pipeline_add() {
    let mut p = Pipeline::new();

    #[derive(Debug)]
    struct AlwaysPass;
    impl LogProcessor for AlwaysPass {
        fn process(&self, _: &dm_database_parser_sqllog::Sqllog) -> bool {
            true
        }
    }

    p.add(Box::new(AlwaysPass));
    assert!(!p.is_empty());
}

#[test]
fn test_placeholder_override_question() {
    let cfg = NormalizeConfig {
        enable: true,
        placeholders: vec!["?".into()],
    };
    assert_eq!(cfg.placeholder_override(), Some(false));
}

#[test]
fn test_placeholder_override_colon() {
    let cfg = NormalizeConfig {
        enable: true,
        placeholders: vec![":1".into()],
    };
    assert_eq!(cfg.placeholder_override(), Some(true));
}

#[test]
fn test_placeholder_override_auto() {
    let cfg = NormalizeConfig {
        enable: true,
        placeholders: vec![],
    };
    assert_eq!(cfg.placeholder_override(), None);
}

#[test]
fn test_placeholder_override_both_is_auto() {
    let cfg = NormalizeConfig {
        enable: true,
        placeholders: vec!["?".into(), ":1".into()],
    };
    assert_eq!(cfg.placeholder_override(), None);
}

#[test]
fn test_normalize_config_default() {
    let cfg = NormalizeConfig::default();
    assert!(cfg.enable);
    assert!(cfg.placeholders.is_empty());
}

#[test]
fn test_output_config_field_mask_default() {
    let cfg = OutputConfig::default();
    assert_eq!(cfg.field_mask(), FieldMask::ALL);
}

#[test]
fn test_output_config_field_mask_with_names() {
    let cfg = OutputConfig {
        fields: Some(vec!["sql".into(), "username".into()]),
    };
    let mask = cfg.field_mask();
    // sql=10, username=4
    assert!(mask.is_active(10));
    assert!(mask.is_active(4));
    assert!(!mask.is_active(0)); // ts not included
}

#[test]
fn test_output_config_ordered_indices_preserves_user_order() {
    let cfg = OutputConfig {
        fields: Some(vec!["sql".into(), "username".into(), "ts".into()]),
    };
    let indices = cfg.ordered_field_indices();
    assert_eq!(indices, vec![10_usize, 4, 0]);
}

#[test]
fn test_output_config_ordered_indices_none_returns_all() {
    let cfg = OutputConfig::default();
    let indices = cfg.ordered_field_indices();
    assert_eq!(indices, (0..15_usize).collect::<Vec<_>>());
}

#[test]
fn test_output_config_ordered_indices_empty_equals_all() {
    let cfg = OutputConfig {
        fields: Some(vec![]),
    };
    let indices = cfg.ordered_field_indices();
    assert_eq!(indices.len(), 15);
    assert_eq!(indices, (0..15_usize).collect::<Vec<_>>());
}

#[test]
fn test_default_true_via_serde() {
    let cfg: NormalizeConfig = toml::from_str("").unwrap();
    assert!(cfg.enable);
}

#[test]
fn test_process_with_meta_default_delegates_to_process() {
    use dm_database_parser_sqllog::LogParserBuilder;

    #[derive(Debug)]
    struct AlwaysPass;
    impl LogProcessor for AlwaysPass {
        fn process(&self, _: &dm_database_parser_sqllog::Sqllog) -> bool {
            true
        }
    }

    #[derive(Debug)]
    struct AlwaysFail;
    impl LogProcessor for AlwaysFail {
        fn process(&self, _: &dm_database_parser_sqllog::Sqllog) -> bool {
            false
        }
    }

    let dir = tempfile::TempDir::new().unwrap();
    let log = dir.path().join("t.log");
    std::fs::write(&log, "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n").unwrap();
    let parser = LogParserBuilder::new(log.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser.iter().unwrap().flatten().collect();
    assert!(!records.is_empty());

    let record = &records[0];

    let mut p = Pipeline::new();
    p.add(Box::new(AlwaysPass));
    assert!(p.run_with_meta(record));

    let mut p2 = Pipeline::new();
    p2.add(Box::new(AlwaysFail));
    assert!(!p2.run_with_meta(record));
}
