//! Turns the local machine into the observations the core can judge.
//!
//! Process detection covers every catalog entry. Session files are only read
//! for the agents whose format was verified: Claude Code, Codex, and Kimi Code.
//! An agent with nothing but a process still gets a lamp; it is just a quieter one.

use fanal_core::{builtin_catalog, Activity, Observation, SessionReading};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{ProcessesToUpdate, System};

pub struct Collector {
    system: System,
    home: PathBuf,
}

impl Collector {
    pub fn new() -> Self {
        Collector {
            system: System::new(),
            home: home_dir(),
        }
    }

    pub fn observe(&mut self) -> Vec<Observation> {
        self.system.refresh_processes(ProcessesToUpdate::All, true);
        let running = running_names(&self.system);

        let mut observations: Vec<Observation> = builtin_catalog()
            .into_iter()
            .map(|profile| {
                let process_running = profile
                    .processes
                    .iter()
                    .any(|name| running.iter().any(|running| running == name));
                let reading = if process_running {
                    session_reading(profile.id, &self.home)
                } else {
                    None
                };
                Observation {
                    id: profile.id,
                    process_running,
                    activity: reading.as_ref().map(|reading| reading.activity),
                    subject: reading.and_then(|reading| reading.subject),
                }
            })
            .collect();

        // A fixed board for screenshots and for trying the amber state. Release
        // builds ignore it, so a stray variable cannot turn the lamps into fiction.
        #[cfg(debug_assertions)]
        if std::env::var_os("FANAL_DEMO").is_some() {
            apply_demo(&mut observations);
        }
        observations
    }
}

/// Codex waiting on a file, Claude Code mid-task, everything else stopped.
#[cfg(debug_assertions)]
fn apply_demo(observations: &mut [Observation]) {
    use fanal_core::AgentId;
    let now = now_ms();
    for observation in observations {
        let (running, activity, subject) = match observation.id {
            AgentId::Codex => (
                true,
                Some(Activity::NeedsConfirmation { updated_at_ms: now }),
                Some("auth.ts".to_string()),
            ),
            AgentId::ClaudeCode => (
                true,
                Some(Activity::Working { updated_at_ms: now }),
                Some("fanal".to_string()),
            ),
            _ => (false, None, None),
        };
        observation.process_running = running;
        observation.activity = activity;
        observation.subject = subject;
    }
}

fn running_names(system: &System) -> Vec<String> {
    system
        .processes()
        .values()
        .map(|process| {
            process
                .name()
                .to_string_lossy()
                .trim_end_matches(".exe")
                .to_ascii_lowercase()
        })
        .collect()
}

fn session_reading(id: fanal_core::AgentId, home: &Path) -> Option<SessionReading> {
    use fanal_core::AgentId;
    match id {
        AgentId::ClaudeCode => claude_reading(&home.join(".claude").join("sessions")),
        AgentId::Codex => codex_reading(&home.join(".codex").join("sessions")),
        AgentId::KimiCode => kimi_reading(&home.join(".kimi-code").join("sessions")),
        _ => None,
    }
}

/// The most recently touched Claude Code session wins. Several can be live at
/// once; the lip only has room for one colour per agent.
fn claude_reading(dir: &Path) -> Option<SessionReading> {
    let newest = newest_file(dir, 1, |path| {
        path.extension().and_then(|ext| ext.to_str()) == Some("json")
    })?;
    let text = fs::read_to_string(&newest).ok()?;
    fanal_core::claude_status(&text, modified_ms(&newest))
}

fn codex_reading(dir: &Path) -> Option<SessionReading> {
    let newest = newest_file(dir, 4, |path| {
        path.extension().and_then(|ext| ext.to_str()) == Some("jsonl")
    })?;
    // A rollout grows without bound. The tail is enough: a turn boundary is one line.
    let text = read_tail(&newest, 256 * 1024).ok()?;
    fanal_core::codex_activity(&text, modified_ms(&newest))
}

fn kimi_reading(dir: &Path) -> Option<SessionReading> {
    let newest = newest_file(dir, 6, |path| path.file_name().and_then(|n| n.to_str()) == Some("wire.jsonl"))?;
    let text = read_tail(&newest, 256 * 1024).ok()?;
    fanal_core::kimi_activity(&text, modified_ms(&newest))
}

fn newest_file(dir: &Path, depth: usize, pred: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    fn walk(dir: &Path, depth: usize, pred: &dyn Fn(&Path) -> bool, best: &mut Option<(SystemTime, PathBuf)>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if depth > 0 {
                    walk(&path, depth - 1, pred, best);
                }
            } else if pred(&path) {
                if let Ok(modified) = entry.metadata().and_then(|meta| meta.modified()) {
                    let replace = best.as_ref().map(|(time, _)| modified > *time).unwrap_or(true);
                    if replace {
                        *best = Some((modified, path));
                    }
                }
            }
        }
    }

    let mut best = None;
    walk(dir, depth, &pred, &mut best);
    best.map(|(_, path)| path)
}

fn read_tail(path: &Path, max_bytes: u64) -> std::io::Result<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = fs::File::open(path)?;
    let length = file.metadata()?.len();
    if length > max_bytes {
        file.seek(SeekFrom::Start(length - max_bytes))?;
    }
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    // A seek lands mid-line. Drop that fragment so the parser sees whole records.
    if length > max_bytes {
        if let Some(index) = buf.find('\n') {
            buf.replace_range(..=index, "");
        }
    }
    Ok(buf)
}

fn modified_ms(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Where Fanal keeps its own settings. This is Fanal's directory, never an
/// agent's.
pub fn settings_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(home_dir);
    base.join("Fanal").join("settings.json")
}

pub fn load_settings() -> fanal_core::Settings {
    let path = settings_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save_settings(settings: &fanal_core::Settings) -> std::io::Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(settings).expect("settings always serialize");
    fs::write(path, text)
}

/// The executable to launch for an agent, resolved from its running process so
/// it follows wherever the user installed it.
pub fn executable_for(id: fanal_core::AgentId) -> Option<PathBuf> {
    use fanal_core::AgentId;
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    let wanted: &[&str] = match id {
        AgentId::ClaudeCode => &["claude"],
        AgentId::Cursor => &["cursor"],
        AgentId::DeepseekHarness => &["deepseek harness"],
        AgentId::Codex => &["codex", "chatgpt"],
        AgentId::Kimi => &["kimi"],
        AgentId::KimiCode => &["kimi code"],
        AgentId::Orca => &["orca"],
        AgentId::ChatGpt => &["chatgpt"],
        AgentId::Gemini => &["gemini"],
        AgentId::Trae => &["trae"],
        AgentId::Kiro => &["kiro"],
        AgentId::Qoder => &["qoder"],
        AgentId::Antigravity => &["antigravity"],
        AgentId::OpenCode => &["opencode"],
        AgentId::Devin => &["devin"],
    };
    system.processes().values().find_map(|process| {
        let name = process.name().to_string_lossy().trim_end_matches(".exe").to_ascii_lowercase();
        if wanted.contains(&name.as_str()) {
            process.exe().map(PathBuf::from)
        } else {
            None
        }
    })
}

/// The timestamp the staleness window is measured against.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

