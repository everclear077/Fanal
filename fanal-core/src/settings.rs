use crate::model::AgentId;
use serde::{Deserialize, Serialize};

/// One entry of the built-in catalog: how to recognise an agent on disk.
#[derive(Debug, Clone, Copy)]
pub struct AgentProfile {
    pub id: AgentId,
    /// Process image names, lowercase, without `.exe`.
    pub processes: &'static [&'static str],
    /// Whether a session file can say more than "the window is open".
    pub session: bool,
}

pub fn builtin_catalog() -> Vec<AgentProfile> {
    vec![
        AgentProfile {
            id: AgentId::ClaudeCode,
            processes: &["claude"],
            session: true,
        },
        AgentProfile {
            id: AgentId::Cursor,
            processes: &["cursor"],
            session: false,
        },
        AgentProfile {
            id: AgentId::DeepseekHarness,
            processes: &["deepseek harness"],
            session: false,
        },
        AgentProfile {
            id: AgentId::Codex,
            processes: &["codex", "chatgpt"],
            session: true,
        },
        AgentProfile {
            id: AgentId::Kimi,
            processes: &["kimi"],
            session: false,
        },
        AgentProfile {
            id: AgentId::KimiCode,
            processes: &["kimi code", "kimi-code"],
            session: true,
        },
        AgentProfile {
            id: AgentId::Orca,
            processes: &["orca"],
            session: false,
        },
        AgentProfile {
            id: AgentId::ChatGpt,
            processes: &["chatgpt"],
            session: false,
        },
        AgentProfile {
            id: AgentId::Gemini,
            processes: &["gemini"],
            session: false,
        },
        AgentProfile {
            id: AgentId::Trae,
            processes: &["trae", "trae cn", "trae solo cn"],
            session: false,
        },
        AgentProfile {
            id: AgentId::Kiro,
            processes: &["kiro"],
            session: false,
        },
        AgentProfile {
            id: AgentId::Qoder,
            processes: &["qoder"],
            session: false,
        },
        AgentProfile {
            id: AgentId::Antigravity,
            processes: &["antigravity"],
            session: false,
        },
        AgentProfile {
            id: AgentId::OpenCode,
            processes: &["opencode"],
            session: false,
        },
        AgentProfile {
            id: AgentId::Devin,
            processes: &["devin"],
            session: false,
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    System,
}

/// Which language the panel writes in. `System` follows the OS language.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Language {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh")]
    Chinese,
    #[serde(rename = "system")]
    System,
}

impl Default for Language {
    fn default() -> Self {
        Language::System
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dark
    }
}

/// Whether one catalog entry is watched at all.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentConfig {
    pub id: AgentId,
    pub enabled: bool,
}

/// Everything the settings panel can change. Stored as JSON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    pub theme: Theme,
    #[serde(default)]
    pub language: Language,
    /// Grow the lip on its own when something needs the user.
    pub alert_on_attention: bool,
    /// Also raise a Windows notification for an amber lamp.
    pub notify_on_amber: bool,
    pub agents: Vec<AgentConfig>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: Theme::Dark,
            language: Language::System,
            alert_on_attention: true,
            notify_on_amber: true,
            agents: AgentId::ALL
                .into_iter()
                .map(|id| AgentConfig { id, enabled: true })
                .collect(),
        }
    }
}

impl Settings {
    pub fn enabled(&self, id: AgentId) -> bool {
        self.agents
            .iter()
            .find(|agent| agent.id == id)
            .map(|agent| agent.enabled)
            .unwrap_or(true)
    }
}
