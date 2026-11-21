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
    pub activity: Option<Activity>,
    /// A short, already-sanitised description such as a file name. Optional.
    pub subject: Option<String>,
}

/// The clock and the staleness window the judgement needs.
#[derive(Debug, Clone, Copy)]
pub struct Evidence {
    pub now_ms: u64,
    /// A session reading older than this is ignored. The lamp then follows the
    /// process alone: green while it runs, red once it has exited.
    pub stale_after_ms: u64,
}

impl Evidence {
    pub const DEFAULT_STALE_MS: u64 = 45_000;
}

fn fresh(activity: Activity, evidence: Evidence) -> bool {
    let updated = match activity {
        Activity::Working { updated_at_ms }
        | Activity::NeedsConfirmation { updated_at_ms }
        | Activity::Idle { updated_at_ms } => updated_at_ms,
    };
    evidence.now_ms.saturating_sub(updated) <= evidence.stale_after_ms
}

fn judge(profile: &AgentProfile, observation: &Observation, evidence: Evidence) -> AgentSnapshot {
    let activity = observation.activity.filter(|activity| fresh(*activity, evidence));

    let (lamp, from_session) = match (observation.process_running, activity) {
        (_, Some(Activity::NeedsConfirmation { .. })) => (Lamp::Amber, true),
        (true, Some(Activity::Working { .. })) => (Lamp::Green, true),
        (true, _) => (Lamp::Green, false),
        (false, _) => (Lamp::Red, activity.is_some()),
    };

    let detail = match lamp {
        Lamp::Amber | Lamp::Green if from_session => observation.subject.clone().unwrap_or_default(),
        Lamp::Green | Lamp::Red | Lamp::Amber => String::new(),
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
/// Agents the user switched off never appear. Agents with no observation are
/// reported as stopped, so a missing process and a missing reading look the
/// same: a red lamp.
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
            activity,
            subject: Some("auth.ts".to_string()),
        }
    }

    #[test]
    fn no_process_is_red_even_with_a_stale_session() {
        let snapshots = classify(
            &[observe(
                AgentId::ClaudeCode,
                false,
                Some(Activity::Working {
                    updated_at_ms: 1_000_000 - 10_000,
                }),
            )],
            &Settings::default(),
            evidence(),
        );
        let claude = snapshots.iter().find(|s| s.id == AgentId::ClaudeCode).unwrap();
        assert_eq!(claude.lamp, Lamp::Red);
        assert!(claude.detail.is_empty());
    }

    #[test]
    fn a_fresh_confirmation_is_amber_and_names_the_subject() {
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
        assert_eq!(codex.lamp, Lamp::Amber);
        assert_eq!(codex.detail, "auth.ts");
        assert!(codex.from_session);
    }

    #[test]
    fn a_stale_confirmation_falls_back_to_the_process() {
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
        assert_eq!(codex.lamp, Lamp::Green);
        assert!(!codex.from_session);
        assert!(codex.detail.is_empty());
    }

    #[test]
    fn a_live_process_without_a_session_is_a_quiet_green() {
        let snapshots = classify(
            &[observe(AgentId::Kimi, true, None)],
            &Settings::default(),
            evidence(),
        );
        let kimi = snapshots.iter().find(|s| s.id == AgentId::Kimi).unwrap();
        assert_eq!(kimi.lamp, Lamp::Green);
        assert!(!kimi.from_session);
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
    fn an_unobserved_agent_is_stopped() {
        let snapshots = classify(&[], &Settings::default(), evidence());
        assert!(snapshots.iter().all(|s| s.lamp == Lamp::Red));
    }

    #[test]
    fn severity_lets_amber_win() {
        let snapshots = vec![
            AgentSnapshot {
                id: AgentId::ClaudeCode,
                lamp: Lamp::Green,
                detail: String::new(),
                from_session: true,
            },
            AgentSnapshot {
                id: AgentId::Cursor,
                lamp: Lamp::Amber,
                detail: String::new(),
                from_session: true,
            },
            AgentSnapshot {
                id: AgentId::Kimi,
                lamp: Lamp::Red,
                detail: String::new(),
                from_session: false,
            },
        ];
        let severity = Severity::from_snapshots(&snapshots);
        assert_eq!(severity.lamp, Lamp::Amber);
        assert_eq!((severity.attention, severity.running, severity.stopped), (1, 1, 1));
    }

    #[test]
    fn settings_round_trip() {
        let settings = Settings::default();
        let encoded = serde_json::to_string(&settings).unwrap();
        let decoded: Settings = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, settings);
    }
}
