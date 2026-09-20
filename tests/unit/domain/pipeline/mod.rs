use super::*;

#[test]
fn test_pipeline_empty() {
    let p = Pipeline::new();
    assert!(p.is_empty());
}

#[test]
fn test_pipeline_add() {
    #[derive(Debug)]
    struct AlwaysPass;
    impl LogProcessor for AlwaysPass {
        fn process(&self, _: &crate::model::LogRecord) -> bool {
            true
        }
    }

    let mut p = Pipeline::new();
    p.add(Box::new(AlwaysPass));
    assert!(!p.is_empty());
}

#[test]
fn test_placeholder_override_question() {
    let cfg = NormalizeConfig {
        tags: vec!["SEL".into()],
        placeholders: vec!["?".into()],
    };
    assert_eq!(cfg.placeholder_override(), Some(false));
}

#[test]
fn test_placeholder_override_colon() {
    let cfg = NormalizeConfig {
        tags: vec!["SEL".into()],
        placeholders: vec![":1".into()],
    };
    assert_eq!(cfg.placeholder_override(), Some(true));
}

#[test]
fn test_placeholder_override_auto() {
    let cfg = NormalizeConfig {
        tags: vec!["SEL".into()],
        placeholders: vec![],
    };
    assert_eq!(cfg.placeholder_override(), None);
}

#[test]
fn test_placeholder_override_both_is_auto() {
    let cfg = NormalizeConfig {
        tags: vec!["SEL".into()],
        placeholders: vec!["?".into(), ":1".into()],
    };
    assert_eq!(cfg.placeholder_override(), None);
}

#[test]
fn test_normalize_config_default() {
    let cfg = NormalizeConfig::default();
    assert_eq!(cfg.tags, vec!["SEL"]);
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
fn test_empty_replace_parameters_section() {
    let cfg: NormalizeConfig = toml::from_str("").unwrap();
    assert_eq!(cfg.tags, vec!["SEL"]);
    assert!(cfg.placeholders.is_empty());
}

#[test]
fn test_process_with_meta_default_delegates_to_process() {
    #[derive(Debug)]
    struct AlwaysPass;
    impl LogProcessor for AlwaysPass {
        fn process(&self, _: &crate::model::LogRecord) -> bool {
            true
        }
    }

    #[derive(Debug)]
    struct AlwaysFail;
    impl LogProcessor for AlwaysFail {
        fn process(&self, _: &crate::model::LogRecord) -> bool {
            false
        }
    }

    let record = crate::model::LogRecord {
        ts: "2025-01-15 10:30:28.001".into(),
        tag: Some("SEL".into()),
        sql: "SELECT 1".into(),
        ..Default::default()
    };

    let mut p = Pipeline::new();
    p.add(Box::new(AlwaysPass));
    assert!(p.run_with_meta(&record));

    let mut p2 = Pipeline::new();
    p2.add(Box::new(AlwaysFail));
    assert!(!p2.run_with_meta(&record));
}
