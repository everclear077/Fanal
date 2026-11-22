//! Readers for the session files whose shape was checked against a real machine.
//!
//! Each reader takes the file's text and its modification time, and returns the
//! most it can honestly say. A file it does not understand yields `None`, which
//! the classifier treats as "process only" rather than as a guess.

use crate::detect::Activity;
use serde::Deserialize;

/// What one session file contributed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionReading {
    pub activity: Activity,
    pub subject: Option<String>,
}

/// Claude Code writes one small JSON file per live session under
/// `~/.claude/sessions`. `status` is the field that matters; everything else in
/// the file is metadata we deliberately do not surface.
pub fn claude_status(text: &str, modified_at_ms: u64) -> Option<SessionReading> {
    #[derive(Deserialize)]
    struct Session {
        status: Option<String>,
        cwd: Option<String>,
    }

    let session: Session = serde_json::from_str(text).ok()?;
    let activity = match session.status.as_deref() {
        Some("busy" | "running") => Activity::Working {
            updated_at_ms: modified_at_ms,
        },
        Some("needs_approval" | "waiting" | "blocked") => Activity::NeedsConfirmation {
            updated_at_ms: modified_at_ms,
        },
        Some("idle" | "ended" | "stopped") => Activity::Idle {
            updated_at_ms: modified_at_ms,
        },
        _ => return None,
    };

    Some(SessionReading {
        activity,
        subject: session.cwd.as_deref().and_then(leaf),
    })
}

/// Codex records a thread as JSONL. A thread is waiting on the user when a
/// turn started and the file ends before that turn completes or aborts.
pub fn codex_activity(text: &str, modified_at_ms: u64) -> Option<SessionReading> {
    let mut cwd: Option<String> = None;
    let mut open_turn = false;
    let mut saw_event = false;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line).ok()?;
        let kind = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let payload = value.get("payload");
        let payload_type = payload
            .and_then(|p| p.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if kind == "session_meta" {
            if let Some(dir) = payload.and_then(|p| p.get("cwd")).and_then(|v| v.as_str()) {
                cwd = Some(dir.to_string());
            }
        }
        match payload_type {
            "task_started" => {
                saw_event = true;
                open_turn = true;
            }
            "task_complete" | "turn_aborted" => {
                saw_event = true;
                open_turn = false;
            }
            _ => {}
        }
    }

    if !saw_event {
        return None;
    }
    let activity = if open_turn {
        Activity::Working {
            updated_at_ms: modified_at_ms,
        }
    } else {
        Activity::Idle {
            updated_at_ms: modified_at_ms,
        }
    };
    Some(SessionReading {
        activity,
        subject: cwd.as_deref().and_then(leaf),
    })
}

/// Kimi Code appends events to `wire.jsonl`. A turn that ended without a later
/// message is idle; a trailing message means it is still working.
pub fn kimi_activity(text: &str, modified_at_ms: u64) -> Option<SessionReading> {
    let mut saw_turn = false;
    let mut working = false;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line).ok()?;
        match value.get("type").and_then(|v| v.as_str()).unwrap_or("") {
            "agent.turn.started" | "agent.message.appended" => {
                saw_turn = true;
                working = true;
            }
            "agent.turn.ended" => {
                saw_turn = true;
                working = false;
            }
            _ => {}
        }
    }

    if !saw_turn {
        return None;
    }
    let activity = if working {
        Activity::Working {
            updated_at_ms: modified_at_ms,
        }
    } else {
        Activity::Idle {
            updated_at_ms: modified_at_ms,
        }
    };
    Some(SessionReading {
        activity,
        subject: None,
    })
}

/// Kimi Desktop writes one status per conversation to
/// `%APPDATA%\kimi-desktop\kimi-agent\conversation-statuses.json`. The values
/// are status words. A conversation id is not a name, so nothing else is kept.
pub fn kimi_desktop_activity(text: &str, modified_at_ms: u64) -> Option<SessionReading> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let statuses = value.as_object()?;
    let mut running = false;
    let mut idle = false;
    for status in statuses.values().filter_map(|status| status.as_str()) {
        match status {
            "running" => running = true,
            "completed" => idle = true,
            _ => {}
        }
    }
    let activity = if running {
        Activity::Working {
            updated_at_ms: modified_at_ms,
        }
    } else if idle {
        Activity::Idle {
            updated_at_ms: modified_at_ms,
        }
    } else {
        return None;
    };
    Some(SessionReading {
        activity,
        subject: None,
    })
}

/// The last path segment, so a working directory becomes a name rather than a
/// full path in the panel.
fn leaf(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches(['\\', '/']);
    let name = trimmed.rsplit(['\\', '/']).next().unwrap_or(trimmed);
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_busy_is_working_and_names_the_directory() {
        let text = r#"{"status":"busy","cwd":"D:\\work\\fanal"}"#;
        let reading = claude_status(text, 100).unwrap();
        assert_eq!(
            reading.activity,
            Activity::Working { updated_at_ms: 100 }
        );
        assert_eq!(reading.subject.as_deref(), Some("fanal"));
    }

    #[test]
    fn claude_waiting_is_a_confirmation() {
        let text = r#"{"status":"needs_approval"}"#;
        let reading = claude_status(text, 100).unwrap();
        assert_eq!(
            reading.activity,
            Activity::NeedsConfirmation { updated_at_ms: 100 }
        );
    }

    #[test]
    fn an_unknown_claude_status_says_nothing() {
        assert!(claude_status(r#"{"status":"pondering"}"#, 100).is_none());
        assert!(claude_status("not json", 100).is_none());
    }

    #[test]
    fn codex_with_an_open_turn_is_working() {
        let text = "\
{\"type\":\"session_meta\",\"payload\":{\"cwd\":\"C:\\\\repo\\\\app\"}}\n\
{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\"}}\n";
        let reading = codex_activity(text, 200).unwrap();
        assert_eq!(reading.activity, Activity::Working { updated_at_ms: 200 });
        assert_eq!(reading.subject.as_deref(), Some("app"));
    }

    #[test]
    fn codex_completed_turn_is_idle() {
        let text = "\
{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\"}}\n\
{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\"}}\n";
        let reading = codex_activity(text, 200).unwrap();
        assert_eq!(reading.activity, Activity::Idle { updated_at_ms: 200 });
    }

    #[test]
    fn kimi_trailing_message_is_working_and_a_closed_turn_is_idle() {
        let working = "{\"type\":\"agent.message.appended\"}\n";
        assert_eq!(
            kimi_activity(working, 300).unwrap().activity,
            Activity::Working { updated_at_ms: 300 }
        );
        let idle = "\
{\"type\":\"agent.message.appended\"}\n\
{\"type\":\"agent.turn.ended\"}\n";
        assert_eq!(
            kimi_activity(idle, 300).unwrap().activity,
            Activity::Idle { updated_at_ms: 300 }
        );
    }

    #[test]
    fn kimi_desktop_running_wins_over_completed_and_keeps_no_subject() {
        let text = r#"{"agent:one":"completed","agent:two":"running"}"#;
        let reading = kimi_desktop_activity(text, 400).unwrap();
        assert_eq!(reading.activity, Activity::Working { updated_at_ms: 400 });
        assert!(reading.subject.is_none());
    }

    #[test]
    fn kimi_desktop_with_only_finished_conversations_is_idle() {
        let text = r#"{"agent:one":"completed"}"#;
        assert_eq!(
            kimi_desktop_activity(text, 400).unwrap().activity,
            Activity::Idle { updated_at_ms: 400 }
        );
        assert!(kimi_desktop_activity("{}", 400).is_none());
        assert!(kimi_desktop_activity(r#"{"agent:one":"elsewhere"}"#, 400).is_none());
    }
}
