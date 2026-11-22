//! Turns the local machine into the observations the core can judge.
//!
//! Process detection covers every catalog entry. Session files are only read
//! for the agents whose format was verified: Claude Code, Claude Desktop, Codex,
//! Kimi, and Kimi Code. An agent with nothing but a process still gets a lamp;
//! it is just a quieter one.

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
        let running = running_exes(&self.system);

        let mut observations: Vec<Observation> = builtin_catalog()
            .into_iter()
            .map(|profile| {
                let live: Vec<&LiveProcess> = running
                    .iter()
                    .filter(|process| {
                        profile.processes.contains(&process.name.as_str())
                            && belongs_to(profile.id, &process.exe)
                    })
                    .collect();
                let process_running = !live.is_empty();
                // ChatGPT satisfies Codex's process list, and a helper can share
                // an image name. Neither one owns the session file.
                let owners: Vec<&LiveProcess> = live
                    .iter()
                    .copied()
                    .filter(|process| session_owner(profile.id, process))
                    .collect();
                let reading = if owners.is_empty() {
                    None
                } else {
                    session_reading(profile.id, &self.home, &owners)
                };
                Observation {
                    id: profile.id,
                    installed: process_running || program_installed(profile.id, &profile.processes),
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

/// Codex waiting on a confirmation, Claude Code mid-task, Claude Desktop open
/// with nothing running, everything else closed.
#[cfg(debug_assertions)]
fn apply_demo(observations: &mut [Observation]) {
    use fanal_core::AgentId;
    let now = now_ms();
    for observation in observations {
        let (running, installed, activity, subject) = match observation.id {
            AgentId::Codex => (
                true,
                true,
                Some(Activity::NeedsConfirmation { updated_at_ms: now }),
                Some("auth.ts".to_string()),
            ),
            AgentId::ClaudeCode => (
                true,
                true,
                Some(Activity::Working { updated_at_ms: now }),
                Some("fanal".to_string()),
            ),
            AgentId::ClaudeDesktop => (true, true, None, None),
            AgentId::Cursor => (false, true, None, None),
            _ => (false, false, None, None),
        };
        observation.installed = installed;
        observation.process_running = running;
        observation.activity = activity;
        observation.subject = subject;
    }
}

/// Whether one of the program's executables can be started from this machine.
/// A running process already answers that; this covers the one that is closed.
fn program_installed(id: fanal_core::AgentId, names: &[&str]) -> bool {
    #[cfg(windows)]
    {
        let _ = id;
        names.iter().any(|name| app_registered(name))
    }
    #[cfg(not(windows))]
    {
        let _ = (id, names);
        false
    }
}

/// Windows records a GUI program under its display name in the uninstall list,
/// and a CLI under its bare name in App Paths. Either one means it is here.
#[cfg(windows)]
fn app_registered(name: &str) -> bool {
    registered_executable(name).is_some()
}

/// The program a display name points at: its icon, then its install folder.
#[cfg(windows)]
fn registered_executable(name: &str) -> Option<PathBuf> {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ,
    };

    let wanted = name.to_ascii_lowercase();
    let roots = [
        (HKEY_CURRENT_USER, "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\0"),
        (HKEY_LOCAL_MACHINE, "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\0"),
        (HKEY_LOCAL_MACHINE, "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\0"),
    ];
    for (root, path) in roots {
        let mut parent = std::ptr::null_mut();
        if unsafe { RegOpenKeyExW(root, wide(path).as_ptr(), 0, KEY_READ, &mut parent) } != 0 {
            continue;
        }
        let mut index = 0u32;
        let found = loop {
            let mut sub = [0u16; 256];
            let mut len = sub.len() as u32;
            let status = unsafe {
                RegEnumKeyExW(parent, index, sub.as_mut_ptr(), &mut len, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut())
            };
            // 259 is no more keys. 234 means this one name did not fit in 255
            // characters; skip it, because stopping here hides every later entry.
            if status == 259 {
                break None;
            }
            if status == 234 {
                index += 1;
                continue;
            }
            if status != 0 {
                break None;
            }
            index += 1;
            let mut child = std::ptr::null_mut();
            if unsafe { RegOpenKeyExW(parent, sub.as_ptr(), 0, KEY_READ, &mut child) } != 0 {
                continue;
            }
            let display = reg_string(child, "DisplayName\0");
            let icon = reg_string(child, "DisplayIcon\0");
            let location = reg_string(child, "InstallLocation\0");
            unsafe { RegCloseKey(child) };
            if display_matches(&display, &wanted)
                && let Some(path) = executable_from(&icon, &location, name)
            {
                break Some(path);
            }
        };
        unsafe { RegCloseKey(parent) };
        if let Some(path) = found {
            return Some(path);
        }
    }

    let app = wide(&format!("Software\\Microsoft\\Windows\\CurrentVersion\\App Paths\\{wanted}.exe\0"));
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let mut key = std::ptr::null_mut();
        if unsafe { RegOpenKeyExW(root, app.as_ptr(), 0, KEY_READ, &mut key) } != 0 {
            continue;
        }
        let target = reg_string(key, "\0");
        unsafe { RegCloseKey(key) };
        let path = PathBuf::from(target.trim().trim_matches('"'));
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

/// A product name matches when the words of the image name are its leading
/// words. "Kimi Code" is not Kimi, and "Cursor Theme Pack" is not Cursor.
/// A trailing version, "(User)", or a channel word is ignored.
#[cfg(windows)]
fn display_matches(display: &str, wanted: &str) -> bool {
    let display = display.to_ascii_lowercase();
    let wanted_words: Vec<&str> = wanted.split_whitespace().collect();
    if wanted_words.is_empty() {
        return false;
    }
    let words: Vec<&str> = display
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    if words.len() < wanted_words.len() {
        return false;
    }
    if words[..wanted_words.len()] != wanted_words[..] {
        return false;
    }
    words[wanted_words.len()..].iter().all(|word| {
        word.chars().all(|ch| ch.is_ascii_digit())
            || matches!(*word, "user" | "cn" | "beta" | "preview" | "insider")
    })
}

#[cfg(windows)]
fn executable_from(icon: &str, location: &str, name: &str) -> Option<PathBuf> {
    let icon = icon.split(',').next().unwrap_or(icon).trim().trim_matches('"');
    let icon_path = PathBuf::from(icon);
    if icon_path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.eq_ignore_ascii_case("exe")) == Some(true)
        && icon_path.is_file()
    {
        return Some(icon_path);
    }
    let folder = PathBuf::from(location.trim().trim_matches('"'));
    let candidate = folder.join(format!("{name}.exe"));
    if candidate.is_file() {
        return Some(candidate);
    }
    None
}

#[cfg(windows)]
fn reg_string(key: windows_sys::Win32::System::Registry::HKEY, value: &str) -> String {
    use windows_sys::Win32::System::Registry::{RegGetValueW, RRF_RT_REG_SZ};
    let mut bytes = 0u32;
    let value = wide(value);
    let first = unsafe {
        RegGetValueW(key, std::ptr::null(), value.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), std::ptr::null_mut(), &mut bytes)
    };
    if first != 0 || bytes < 2 {
        return String::new();
    }
    let mut buf = vec![0u16; (bytes as usize) / 2];
    let second = unsafe {
        RegGetValueW(key, std::ptr::null(), value.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut bytes)
    };
    if second != 0 {
        return String::new();
    }
    let end = buf.iter().position(|unit| *unit == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

struct LiveProcess {
    name: String,
    exe: String,
    pid: u32,
    started_ms: u64,
}

fn process_name(process: &sysinfo::Process) -> String {
    process
        .name()
        .to_string_lossy()
        .trim_end_matches(".exe")
        .to_ascii_lowercase()
}

fn running_exes(system: &System) -> Vec<LiveProcess> {
    system
        .processes()
        .values()
        .map(|process| LiveProcess {
            name: process_name(process),
            exe: process
                .exe()
                .map(|path| path.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default(),
            pid: process.pid().as_u32(),
            started_ms: process.start_time().saturating_mul(1000),
        })
        .collect()
}

/// Claude Code and Claude Desktop share the image name `claude`. The desktop
/// app is either the Store package `WindowsApps\Claude_<version>_...` or the
/// runtime it keeps under `Claude-3p`. The CLI is everything else under that
/// name. A directory that merely contains the word is not the package.
pub fn belongs_to(id: fanal_core::AgentId, exe: &str) -> bool {
    use fanal_core::AgentId;
    let parts: Vec<String> = exe
        .split(['\\', '/'])
        .map(|part| part.to_ascii_lowercase())
        .collect();
    let desktop = parts.iter().any(|part| part.starts_with("claude_") && part.contains('_'))
        || parts.iter().any(|part| part == "claude-3p");
    match id {
        AgentId::ClaudeDesktop => desktop,
        AgentId::ClaudeCode => !desktop,
        _ => true,
    }
}

/// The process whose session file this agent may read. A shared image name is
/// not enough: ChatGPT keeps no Codex rollout, and a helper is not the client.
fn session_owner(id: fanal_core::AgentId, process: &LiveProcess) -> bool {
    use fanal_core::AgentId;
    match id {
        AgentId::Codex => process.name == "codex",
        AgentId::KimiCode => process.name == "kimi code" || process.name == "kimi-code",
        _ => true,
    }
}

/// Every live process of this agent. The window may belong to a later one:
/// Codex's window is `chatgpt`, while `codex` itself is a helper.
pub fn running_pids(id: fanal_core::AgentId) -> Vec<u32> {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    let wanted = process_names(id);
    let mut found = Vec::new();
    for process in system.processes().values() {
        let name = process_name(process);
        let exe = process
            .exe()
            .map(|path| path.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if wanted.contains(&name.as_str()) && belongs_to(id, &exe) {
            found.push(process.pid().as_u32());
        }
    }
    found
}

fn process_names(id: fanal_core::AgentId) -> &'static [&'static str] {
    builtin_catalog()
        .into_iter()
        .find(|profile| profile.id == id)
        .map(|profile| profile.processes)
        .unwrap_or(&[])
}

fn session_reading(id: fanal_core::AgentId, home: &Path, live: &[&LiveProcess]) -> Option<SessionReading> {
    use fanal_core::AgentId;
    match id {
        AgentId::ClaudeCode => claude_reading(&[home.join(".claude").join("sessions")], live),
        AgentId::ClaudeDesktop => claude_reading(&claude_desktop_sessions(), live),
        AgentId::Codex => file_reading(&home.join(".codex").join("sessions"), live, 4, codex_file, fanal_core::codex_activity),
        AgentId::KimiCode => {
            file_reading(&home.join(".kimi-code").join("sessions"), live, 6, kimi_file, fanal_core::kimi_activity)
        }
        AgentId::Kimi => kimi_desktop_reading(live),
        _ => None,
    }
}

fn codex_file(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("jsonl")
}

fn kimi_file(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some("wire.jsonl")
}

/// Claude Desktop keeps one session directory per conversation. A new one is
/// created when a task starts, so the newest directory can be empty while an
/// older one still has a task running. Every directory is read.
fn claude_desktop_sessions() -> Vec<PathBuf> {
    let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
        return Vec::new();
    };
    let root = local.join("Claude-3p").join("local-agent-mode-sessions");
    let mut sessions = Vec::new();
    collect_named(&root, "sessions", 5, &mut sessions);
    sessions.into_iter().map(|(_, path)| path).collect()
}

fn collect_named(dir: &Path, wanted: &str, depth: usize, found: &mut Vec<(u64, PathBuf)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some(wanted) {
            found.push((modified_ms(&path), path));
            continue;
        }
        if depth > 0 {
            collect_named(&path, wanted, depth - 1, found);
        }
    }
}

/// Kimi Desktop keeps one status word per conversation. Only that word is read.
fn kimi_desktop_reading(live: &[&LiveProcess]) -> Option<SessionReading> {
    let path = std::env::var_os("APPDATA")
        .map(PathBuf::from)?
        .join("kimi-desktop")
        .join("kimi-agent")
        .join("conversation-statuses.json");
    if !current_file(&path, live) {
        return None;
    }
    let text = fs::read_to_string(&path).ok()?;
    fanal_core::kimi_desktop_activity(&text, modified_ms(&path))
}

/// A file belongs to this run when it was written after one of these processes
/// started. No grace before the start: a file left by the previous run must
/// not keep the lamp red or green. An unknown start time does not count.
fn current_file(path: &Path, live: &[&LiveProcess]) -> bool {
    let Some(started) = live.iter().map(|process| process.started_ms).filter(|started| *started > 0).min() else {
        return false;
    };
    modified_ms(path) >= started
}

/// One colour for every live Claude Code process. A confirmation outranks a
/// running task, which outranks an idle session. The status stands until the
/// file changes: a long task does not rewrite it every minute.
fn claude_reading(dirs: &[PathBuf], live: &[&LiveProcess]) -> Option<SessionReading> {
    use fanal_core::Activity;
    let mut best: Option<(u8, u64, SessionReading)> = None;
    for dir in dirs {
        for process in live {
            let path = dir.join(format!("{}.json", process.pid));
            if !current_file(&path, &[process]) {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let Some(reading) = fanal_core::claude_status(&text, modified_ms(&path)) else {
                continue;
            };
            let rank = match reading.activity {
                Activity::NeedsConfirmation { .. } => 2,
                Activity::Working { .. } => 1,
                Activity::Idle { .. } => 0,
            };
            let at = modified_ms(&path);
            let newer = best
                .as_ref()
                .map(|(best_rank, best_at, _)| rank > *best_rank || (rank == *best_rank && at > *best_at))
                .unwrap_or(true);
            if newer {
                best = Some((rank, at, reading));
            }
        }
    }
    best.map(|(_, _, reading)| reading)
}

fn file_reading(
    dir: &Path,
    live: &[&LiveProcess],
    depth: usize,
    pred: impl Fn(&Path) -> bool,
    parse: impl Fn(&str, u64) -> Option<SessionReading>,
) -> Option<SessionReading> {
    let newest = newest_file(dir, depth, pred)?;
    if !current_file(&newest, live) {
        return None;
    }
    // A rollout grows without bound. The tail is enough: a turn boundary is one line.
    let text = read_tail(&newest, 256 * 1024).ok()?;
    parse(&text, modified_ms(&newest))
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

/// The executable to launch. A running process answers first. A closed one is
/// taken from the uninstall entry or App Paths, so a grey row can still open.
pub fn executable_for(id: fanal_core::AgentId) -> Option<PathBuf> {
    let wanted = process_names(id);
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    // The first matching image name owns the window. A later one, such as
    // Codex's helper, is only a fallback for launching.
    let running = wanted.iter().find_map(|wanted_name| {
        system.processes().values().find_map(|process| {
            let name = process_name(process);
            if name != *wanted_name {
                return None;
            }
            let exe = process.exe().map(PathBuf::from)?;
            let path = exe.to_string_lossy().to_ascii_lowercase();
            if belongs_to(id, &path) { Some(exe) } else { None }
        })
    });
    if running.is_some() {
        return running;
    }
    #[cfg(windows)]
    {
        wanted.iter().find_map(|name| {
            let path = registered_executable(name)?;
            let text = path.to_string_lossy().to_ascii_lowercase();
            let owned = session_owner(
                id,
                &LiveProcess {
                    name: (*name).to_string(),
                    exe: text.clone(),
                    pid: 0,
                    started_ms: 0,
                },
            );
            if belongs_to(id, &text) && owned { Some(path) } else { None }
        })
    }
    #[cfg(not(windows))]
    {
        None
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    use super::display_matches;
    use super::belongs_to;
    use fanal_core::AgentId;

    #[cfg(windows)]
    #[test]
    fn a_display_name_matches_its_own_words_only() {
        assert!(display_matches("Cursor (User)", "cursor"));
        assert!(display_matches("Kimi Code 1.2.1", "kimi code"));
        assert!(display_matches("Antigravity 2.32.1", "antigravity"));
        assert!(display_matches("OpenCode 2.2.5", "opencode"));
        assert!(display_matches("TRAE SOLO CN (User)", "trae solo cn"));
        assert!(!display_matches("Kimi Code 1.2.1", "kimi"));
        assert!(!display_matches("Cursor Theme Pack", "cursor"));
        assert!(!display_matches("DeepSeek Harness", "deepseek"));
    }

    #[test]
    fn the_store_package_is_claude_desktop_and_a_folder_named_claude_is_not() {
        let store = r"c:\program files\windowsapps\claude_1.0.0_x64__publisher\app\claude.exe";
        let runtime = r"c:\users\me\appdata\local\claude-3p\claude-code\1.0.0\build\claude.exe";
        let folder = r"c:\users\me\windowsapps\claude\claude.exe";
        let cli = r"c:\users\me\.local\bin\claude.exe";
        assert!(belongs_to(AgentId::ClaudeDesktop, store));
        assert!(belongs_to(AgentId::ClaudeDesktop, runtime));
        assert!(!belongs_to(AgentId::ClaudeCode, store));
        assert!(!belongs_to(AgentId::ClaudeCode, runtime));
        assert!(!belongs_to(AgentId::ClaudeDesktop, folder));
        assert!(belongs_to(AgentId::ClaudeCode, folder));
        assert!(belongs_to(AgentId::ClaudeCode, cli));
    }
}

#[cfg(test)]
fn live_process(pid: u32) -> LiveProcess {
    LiveProcess {
        name: "claude".to_string(),
        exe: r"c:\users\me\appdata\local\claude-3p\claude.exe".to_string(),
        pid,
        started_ms: 1,
    }
}

#[cfg(test)]
mod session_tests {
    use super::{claude_reading, live_process};
    use fanal_core::Activity;

    #[test]
    fn a_running_session_in_an_older_directory_still_wins() {
        let root = std::env::temp_dir().join(format!("fanal-claude-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let older = root.join("old-profile").join("sessions");
        let newer = root.join("new-profile").join("sessions");
        let empty = root.join("empty-profile").join("sessions");
        std::fs::create_dir_all(&older).unwrap();
        std::fs::create_dir_all(&newer).unwrap();
        std::fs::create_dir_all(&empty).unwrap();
        std::fs::write(older.join("42.json"), r#"{"status":"busy","cwd":"C:/work/notes"}"#).unwrap();
        std::fs::write(newer.join("99.json"), r#"{"status":"idle","cwd":"C:/work/finished"}"#).unwrap();

        let running = live_process(42);
        let idle = live_process(99);
        // Newest first, and an empty directory in front of that. Neither may hide
        // the task that is still running in the older directory.
        let reading = claude_reading(&[empty.clone(), newer.clone(), older.clone()], &[&idle, &running])
            .expect("a live session");
        assert!(matches!(reading.activity, Activity::Working { .. }));
        assert_eq!(reading.subject.as_deref(), Some("notes"));

        std::fs::write(
            newer.join("7.json"),
            r#"{"status":"needs_approval","cwd":"C:/work/auth"}"#,
        )
        .unwrap();
        let waiting = live_process(7);
        let reading = claude_reading(&[empty, newer, older], &[&idle, &running, &waiting]).expect("a confirmation");
        assert!(matches!(reading.activity, Activity::NeedsConfirmation { .. }));
        assert_eq!(reading.subject.as_deref(), Some("auth"));
        let _ = std::fs::remove_dir_all(&root);
    }
}

