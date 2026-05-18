use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ResumeConfig {
    /// 状态文件路径，`--resume` 模式下用于记录已处理文件的指纹
    #[serde(default = "default_state_file")]
    pub state_file: String,
}

fn default_state_file() -> String {
    ".sqllog2db_state.toml".to_string()
}

impl Default for ResumeConfig {
    fn default() -> Self {
        Self {
            state_file: default_state_file(),
        }
    }
}
