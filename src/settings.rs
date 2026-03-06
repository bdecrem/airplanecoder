use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const SETTINGS_DIR: &str = ".airplane";
const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ResumeModel {
    LastUsed,
    Default,
}

impl std::fmt::Display for ResumeModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResumeModel::LastUsed => write!(f, "last-used"),
            ResumeModel::Default => write!(f, "default"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub resume_model: ResumeModel,
    #[serde(default)]
    pub last_model: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            resume_model: ResumeModel::Default,
            last_model: None,
        }
    }
}

fn settings_path() -> PathBuf {
    let dir = std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(SETTINGS_DIR))
        .unwrap_or_else(|_| PathBuf::from("."));
    dir.join(SETTINGS_FILE)
}

impl Settings {
    pub fn load() -> Self {
        let path = settings_path();
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = settings_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    /// Returns the model to start with based on settings and env.
    pub fn startup_model(&self) -> String {
        let default = std::env::var("AIRPLANE_MODEL").unwrap_or_else(|_| "qwen3.5:4b".to_string());

        match self.resume_model {
            ResumeModel::LastUsed => self.last_model.clone().unwrap_or(default),
            ResumeModel::Default => default,
        }
    }
}
