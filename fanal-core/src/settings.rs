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
        // The desktop app ships the same image name as the CLI. The collector
        // tells them apart by where the executable lives.
        // The desktop app writes the same per-process session file as the CLI,
        // under its own data directory rather than the user's home.
        AgentProfile {
            id: AgentId::ClaudeDesktop,
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
        // The desktop app is one program with two image names: `chatgpt` owns
        // the window, `codex` is its helper. The window name comes first, so
        // opening the row finds the window rather than the helper.
        AgentProfile {
            id: AgentId::Codex,
            processes: &["chatgpt", "codex"],
            session: true,
        },
        // The desktop app records each conversation's status. `running` is a
        // task; `completed` is an open client with nothing executing.
        AgentProfile {
            id: AgentId::Kimi,
            processes: &["kimi"],
            session: true,
        },
        AgentProfile {
            id: AgentId::KimiCode,
            processes: &["kimi code", "kimi-code"],
            session: true,
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
    /// Unknown ids are dropped. A file that still names an agent this version
    /// no longer watches, such as one removed from the catalog, still loads.
    #[serde(deserialize_with = "deserialize_agents")]
    pub agents: Vec<AgentConfig>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: Theme::Dark,
            language: Language::System,
            alert_on_attention: true,
            agents: AgentId::ALL
                .into_iter()
                .map(|id| AgentConfig { id, enabled: true })
                .collect(),
        }
    }
}

impl Settings {
    /// An id the saved file does not mention stays off. A file written before
    /// an agent was added to the catalog would otherwise start watching it.
    pub fn enabled(&self, id: AgentId) -> bool {
        self.agents
            .iter()
            .find(|agent| agent.id == id)
            .map(|agent| agent.enabled)
            .unwrap_or(false)
    }

    /// Add catalog entries the file does not have, switched off, and drop ids
    /// the catalog no longer knows. The user's choices for known ids stay.
    pub fn reconcile(&mut self) {
        let known: Vec<AgentConfig> = AgentId::ALL
            .into_iter()
            .map(|id| AgentConfig {
                id,
                enabled: self.enabled(id),
            })
            .collect();
        self.agents = known;
    }
}

fn deserialize_agents<'de, D>(deserializer: D) -> Result<Vec<AgentConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Entry {
        id: String,
        #[serde(default)]
        enabled: bool,
    }

    Ok(Vec::<Entry>::deserialize(deserializer)?
        .into_iter()
        .filter_map(|entry| {
            AgentId::from_key(&entry.id).map(|id| AgentConfig {
                id,
                enabled: entry.enabled,
            })
        })
        .collect())
}
