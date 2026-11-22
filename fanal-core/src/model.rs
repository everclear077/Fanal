use serde::{Deserialize, Serialize};

/// Stable identifier for one monitored agent. Matches the catalog and the
/// settings file, so renaming a product does not orphan a user's choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentId {
    #[serde(rename = "claude-code")]
    ClaudeCode,
    #[serde(rename = "claude-desktop")]
    ClaudeDesktop,
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
    pub const ALL: [AgentId; 14] = [
        AgentId::ClaudeCode,
        AgentId::ClaudeDesktop,
        AgentId::Cursor,
        AgentId::DeepseekHarness,
        AgentId::Codex,
        AgentId::Kimi,
        AgentId::KimiCode,
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

    /// The id stored in settings. An id this version does not know is `None`,
    /// so a file written by a newer or older build does not fail to load.
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|id| id.as_str() == key)
    }

    /// The name shown to a person, matching the catalog.
    pub fn display_name(self) -> &'static str {
        self.profile().0
    }

    fn profile(self) -> (&'static str, &'static str) {
        match self {
            AgentId::ClaudeCode => ("Claude Code", "claude-code"),
            AgentId::ClaudeDesktop => ("Claude Desktop", "claude-desktop"),
            AgentId::Cursor => ("Cursor", "cursor"),
            AgentId::DeepseekHarness => ("DeepSeek Harness", "deepseek-harness"),
            AgentId::Codex => ("Codex", "codex"), // the ChatGPT desktop app
            AgentId::Kimi => ("Kimi", "kimi"),
            AgentId::KimiCode => ("Kimi Code", "kimi-code"),
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

/// What the lamp says. Grey is the absence of a process, not a fourth kind of
/// task: the other three describe a client that is actually open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lamp {
    /// A task is executing.
    Green,
    /// The client is open and no task is running.
    Yellow,
    /// A task has opened a confirmation and is waiting on the user.
    Red,
    /// The program is installed and its process is not running.
    Grey,
}

impl Lamp {
    /// A confirmation outranks a running task, which outranks an idle client,
    /// which outranks a closed one. The tray icon takes the highest.
    pub fn rank(self) -> u8 {
        match self {
            Lamp::Red => 3,
            Lamp::Green => 2,
            Lamp::Yellow => 1,
            Lamp::Grey => 0,
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
    /// True when a session file decided the colour. On a grey lamp it means the
    /// program is installed, which is what keeps a closed row in the list.
    pub from_session: bool,
}

/// The single fact the lip and the tray icon need.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Severity {
    pub lamp: Lamp,
    /// Tasks waiting on a confirmation.
    pub attention: usize,
    /// Tasks actually executing.
    pub running: usize,
    /// Clients that are open with nothing running.
    pub idle: usize,
    /// Installed programs whose process is not running.
    pub stopped: usize,
}

impl Severity {
    pub fn from_snapshots(snapshots: &[AgentSnapshot]) -> Self {
        let mut severity = Severity {
            lamp: Lamp::Grey,
            attention: 0,
            running: 0,
            idle: 0,
            stopped: 0,
        };
        for snapshot in snapshots {
            match snapshot.lamp {
                Lamp::Red => severity.attention += 1,
                Lamp::Green => severity.running += 1,
                Lamp::Yellow => severity.idle += 1,
                Lamp::Grey => severity.stopped += 1,
            }
            if snapshot.lamp.rank() > severity.lamp.rank() {
                severity.lamp = snapshot.lamp;
            }
        }
        severity
    }
}
