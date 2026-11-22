use crate::model::{AgentId, AgentSnapshot, Lamp};
use crate::settings::{builtin_catalog, AgentProfile, Settings};
use serde::{Deserialize, Serialize};

/// What a session file last said. Collected by the binary, judged here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Activity {
    /// Output is still arriving.
    Working { updated_at_ms: u64 },
    /// The session stopped on a confirmation and has not moved since.
    NeedsConfirmation { updated_at_ms: u64 },
    /// The session ended on its own.
    Idle { updated_at_ms: u64 },
}

/// One observed agent, before any policy is applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub id: AgentId,
    pub process_running: bool,
    /// The program is on this machine, whether or not it is open. An agent that
    /// is neither running nor installed is not connected, and the panel leaves
    /// it out.
    pub installed: bool,
    pub activity: Option<Activity>,
    /// A short, already-sanitised description such as a file name. Optional.
    pub subject: Option<String>,
}

/// The clock the judgement is given. A session reading is either passed in or
/// it is not: the collector drops a file left by a previous run of the process,
/// and a reading that arrives here stands for as long as that process lives.
#[derive(Debug, Clone, Copy)]
pub struct Evidence {
    pub now_ms: u64,
    pub stale_after_ms: u64,
}

impl Evidence {
    pub const DEFAULT_STALE_MS: u64 = 45_000;
}

fn judge(profile: &AgentProfile, observation: &Observation, evidence: Evidence) -> AgentSnapshot {
    let _ = evidence;
    let activity = observation.activity;

    // Grey is "no process". Yellow is "the client is open and idle", including
    // a session that said the last turn finished. Green needs a session that
    // says a task is executing, so an open window with no reading stays yellow.
    // Red is only a confirmation the session reported. `from_session` on grey
    // means the program is installed, which is what keeps a closed row visible.
    let (lamp, from_session) = match (observation.process_running, activity) {
        (true, Some(Activity::NeedsConfirmation { .. })) => (Lamp::Red, true),
        (true, Some(Activity::Working { .. })) => (Lamp::Green, true),
        (true, Some(Activity::Idle { .. })) => (Lamp::Yellow, true),
        (true, None) => (Lamp::Yellow, false),
        (false, _) => (Lamp::Grey, observation.installed),
    };

    let detail = if from_session && lamp != Lamp::Grey {
        observation.subject.clone().unwrap_or_default()
    } else {
        String::new()
    };

    AgentSnapshot {
        id: profile.id,
        lamp,
        detail,
        from_session,
    }
}

/// Apply settings and the catalog to a set of observations.
///
/// Agents the user switched off never appear. An agent with no observation is
/// reported as not installed, which is a grey lamp; the panel drops those.
pub fn classify(observations: &[Observation], settings: &Settings, evidence: Evidence) -> Vec<AgentSnapshot> {
    builtin_catalog()
        .into_iter()
        .filter(|profile| settings.enabled(profile.id))
        .map(|profile| {
            let observation = observations
                .iter()
                .find(|observation| observation.id == profile.id)
                .cloned()
                .unwrap_or(Observation {
                    id: profile.id,
                    process_running: false,
                    installed: false,
                    activity: None,
                    subject: None,
                });
            judge(&profile, &observation, evidence)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Severity;

    fn evidence() -> Evidence {
        Evidence {
            now_ms: 1_000_000,
            stale_after_ms: 45_000,
        }
    }

    fn observe(id: AgentId, running: bool, activity: Option<Activity>) -> Observation {
        Observation {
            id,
            process_running: running,
            installed: true,
            activity,
            subject: Some("auth.ts".to_string()),
        }
    }

    #[test]
    fn no_process_is_grey_even_with_a_fresh_session() {
        let snapshots = classify(
            &[observe(
                AgentId::ClaudeCode,
                false,
                Some(Activity::Working {
                    updated_at_ms: 1_000_000 - 1_000,
                }),
            )],
            &Settings::default(),
            evidence(),
        );
        let claude = snapshots.iter().find(|s| s.id == AgentId::ClaudeCode).unwrap();
        assert_eq!(claude.lamp, Lamp::Grey);
        assert!(claude.detail.is_empty());
    }

    #[test]
    fn a_fresh_confirmation_is_red_and_names_the_subject() {
        let snapshots = classify(
            &[observe(
                AgentId::Codex,
                true,
                Some(Activity::NeedsConfirmation {
                    updated_at_ms: 1_000_000 - 5_000,
                }),
            )],
            &Settings::default(),
            evidence(),
        );
        let codex = snapshots.iter().find(|s| s.id == AgentId::Codex).unwrap();
        assert_eq!(codex.lamp, Lamp::Red);
        assert_eq!(codex.detail, "auth.ts");
        assert!(codex.from_session);
    }

    #[test]
    fn a_running_task_is_green() {
        let snapshots = classify(
            &[observe(
                AgentId::ClaudeCode,
                true,
                Some(Activity::Working {
                    updated_at_ms: 1_000_000 - 5_000,
                }),
            )],
            &Settings::default(),
            evidence(),
        );
        let claude = snapshots.iter().find(|s| s.id == AgentId::ClaudeCode).unwrap();
        assert_eq!(claude.lamp, Lamp::Green);
        assert_eq!(claude.detail, "auth.ts");
    }

    #[test]
    fn an_old_confirmation_stays_red_while_the_process_lives() {
        let snapshots = classify(
            &[observe(
                AgentId::Codex,
                true,
                Some(Activity::NeedsConfirmation {
                    updated_at_ms: 1_000_000 - 120_000,
                }),
            )],
            &Settings::default(),
            evidence(),
        );
        let codex = snapshots.iter().find(|s| s.id == AgentId::Codex).unwrap();
        assert_eq!(codex.lamp, Lamp::Red);
        assert!(codex.from_session);
    }

    #[test]
    fn an_open_client_without_a_task_is_yellow() {
        let snapshots = classify(
            &[observe(AgentId::Kimi, true, None)],
            &Settings::default(),
            evidence(),
        );
        let kimi = snapshots.iter().find(|s| s.id == AgentId::Kimi).unwrap();
        assert_eq!(kimi.lamp, Lamp::Yellow);
        assert!(!kimi.from_session);
        assert!(kimi.detail.is_empty());
    }

    #[test]
    fn an_idle_session_stays_yellow_and_keeps_its_subject() {
        let snapshots = classify(
            &[observe(
                AgentId::ClaudeCode,
                true,
                Some(Activity::Idle {
                    updated_at_ms: 1_000_000 - 5_000,
                }),
            )],
            &Settings::default(),
            evidence(),
        );
        let claude = snapshots.iter().find(|s| s.id == AgentId::ClaudeCode).unwrap();
        assert_eq!(claude.lamp, Lamp::Yellow);
        assert!(claude.from_session);
        assert_eq!(claude.detail, "auth.ts");
    }

    #[test]
    fn a_disabled_agent_disappears() {
        let mut settings = Settings::default();
        settings
            .agents
            .iter_mut()
            .find(|agent| agent.id == AgentId::Devin)
            .unwrap()
            .enabled = false;
        let snapshots = classify(&[], &settings, evidence());
        assert!(snapshots.iter().all(|s| s.id != AgentId::Devin));
        assert_eq!(snapshots.len(), AgentId::ALL.len() - 1);
    }

    #[test]
    fn an_unobserved_agent_is_not_installed() {
        let snapshots = classify(&[], &Settings::default(), evidence());
        assert!(snapshots.iter().all(|s| s.lamp == Lamp::Grey && !s.from_session));
    }

    #[test]
    fn an_installed_agent_that_is_closed_is_grey() {
        let mut observation = observe(AgentId::Cursor, false, None);
        observation.installed = true;
        let snapshots = classify(&[observation], &Settings::default(), evidence());
        let cursor = snapshots.iter().find(|s| s.id == AgentId::Cursor).unwrap();
        assert_eq!(cursor.lamp, Lamp::Grey);
        assert!(cursor.from_session);
    }

    #[test]
    fn severity_lets_a_confirmation_win() {
        let snapshots = vec![
            AgentSnapshot {
                id: AgentId::ClaudeCode,
                lamp: Lamp::Green,
                detail: String::new(),
                from_session: true,
            },
            AgentSnapshot {
                id: AgentId::Cursor,
                lamp: Lamp::Red,
                detail: String::new(),
                from_session: true,
            },
            AgentSnapshot {
                id: AgentId::Kimi,
                lamp: Lamp::Yellow,
                detail: String::new(),
                from_session: false,
            },
            AgentSnapshot {
                id: AgentId::Devin,
                lamp: Lamp::Grey,
                detail: String::new(),
                from_session: false,
            },
        ];
        let severity = Severity::from_snapshots(&snapshots);
        assert_eq!(severity.lamp, Lamp::Red);
        assert_eq!(
            (severity.attention, severity.running, severity.idle, severity.stopped),
            (1, 1, 1, 1)
        );
    }

    #[test]
    fn settings_round_trip() {
        let settings = Settings::default();
        let encoded = serde_json::to_string(&settings).unwrap();
        let decoded: Settings = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, settings);
    }

    #[test]
    fn an_agent_missing_from_a_saved_file_stays_off() {
        let mut settings = Settings::default();
        settings.agents.retain(|agent| agent.id != AgentId::ClaudeDesktop);
        assert!(!settings.enabled(AgentId::ClaudeDesktop));
        settings.reconcile();
        assert!(!settings.enabled(AgentId::ClaudeDesktop));
        assert!(settings.enabled(AgentId::ClaudeCode));
        assert_eq!(settings.agents.len(), AgentId::ALL.len());
    }
}
