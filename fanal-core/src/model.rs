use serde::{Deserialize, Serialize};

/// Stable identifier for one monitored agent. Matches the catalog and the
/// settings file, so renaming a product does not orphan a user's choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentId {
    #[serde(rename = "claude-code")]
    ClaudeCode,
    #[serde(rename = "cursor")]
    Cursor,
    #[serde(rename = "deepseek-harness")]
    DeepseekHarness,
    #[serde(rename = "codex")]
    Codex,
    #[serde(rename = "kimi")]
    Kimi,
    #[serde(rename = "kimi-code")]
    KimiCode,
    #[serde(rename = "orca")]
    Orca,
    #[serde(rename = "chatgpt")]
    ChatGpt,
    #[serde(rename = "gemini")]
    Gemini,
    #[serde(rename = "trae")]
    Trae,
    #[serde(rename = "kiro")]
    Kiro,
    #[serde(rename = "qoder")]
    Qoder,
    #[serde(rename = "antigravity")]
    Antigravity,
    #[serde(rename = "opencode")]
    OpenCode,
    #[serde(rename = "devin")]
    Devin,
}

impl AgentId {
    pub const ALL: [AgentId; 15] = [
        AgentId::ClaudeCode,
        AgentId::Cursor,
        AgentId::DeepseekHarness,
        AgentId::Codex,
        AgentId::Kimi,
        AgentId::KimiCode,
        AgentId::Orca,
        AgentId::ChatGpt,
        AgentId::Gemini,
        AgentId::Trae,
        AgentId::Kiro,
        AgentId::Qoder,
        AgentId::Antigravity,
        AgentId::OpenCode,
        AgentId::Devin,
    ];

    pub fn as_str(self) -> &'static str {
        self.profile().1
    }

    /// The name shown to a person, matching the catalog.
    pub fn display_name(self) -> &'static str {
        self.profile().0
    }

    fn profile(self) -> (&'static str, &'static str) {
        match self {
            AgentId::ClaudeCode => ("Claude Code", "claude-code"),
            AgentId::Cursor => ("Cursor", "cursor"),
            AgentId::DeepseekHarness => ("DeepSeek Harness", "deepseek-harness"),
            AgentId::Codex => ("Codex", "codex"),
            AgentId::Kimi => ("Kimi", "kimi"),
            AgentId::KimiCode => ("Kimi Code", "kimi-code"),
            AgentId::Orca => ("Orca", "orca"),
            AgentId::ChatGpt => ("ChatGPT", "chatgpt"),
            AgentId::Gemini => ("Gemini", "gemini"),
            AgentId::Trae => ("Trae", "trae"),
            AgentId::Kiro => ("Kiro", "kiro"),
            AgentId::Qoder => ("Qoder", "qoder"),
            AgentId::Antigravity => ("Antigravity", "antigravity"),
            AgentId::OpenCode => ("OpenCode", "opencode"),
            AgentId::Devin => ("Devin", "devin"),
        }
    }
}

/// What the lamp says. There are exactly three colours; a fourth would be a
/// product decision, not a detection nuance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lamp {
    /// The agent is running and producing output.
    Green,
    /// The agent is stopped, waiting for the user to confirm something.
    Amber,
    /// The agent is not running.
    Red,
}

impl Lamp {
    /// Amber outranks green, green outranks red. This is the colour the tray
    /// icon and the collapsed lip use.
    pub fn rank(self) -> u8 {
        match self {
            Lamp::Amber => 2,
            Lamp::Green => 1,
            Lamp::Red => 0,
        }
    }
}

/// One row of the panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSnapshot {
    pub id: AgentId,
    pub lamp: Lamp,
    /// One short sentence about what it is doing. Never a status word: the
    /// colour already says that.
    pub detail: String,
    /// True when the colour comes from a session file rather than merely from
    /// the process existing. The panel renders a process-only reading quieter.
    pub from_session: bool,
}

/// The single fact the lip and the tray icon need.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Severity {
    pub lamp: Lamp,
    pub attention: usize,
    pub running: usize,
    pub stopped: usize,
}

impl Severity {
    pub fn from_snapshots(snapshots: &[AgentSnapshot]) -> Self {
        let mut severity = Severity {
            lamp: Lamp::Red,
            attention: 0,
            running: 0,
            stopped: 0,
        };
        for snapshot in snapshots {
            match snapshot.lamp {
                Lamp::Amber => severity.attention += 1,
                Lamp::Green => severity.running += 1,
                Lamp::Red => severity.stopped += 1,
            }
            if snapshot.lamp.rank() > severity.lamp.rank() {
                severity.lamp = snapshot.lamp;
            }
        }
        severity
    }
}
