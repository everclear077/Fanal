//! Pure detection logic for Fanal.
//!
//! Everything that decides a lamp colour lives here and takes plain data, so it
//! can be tested without a Windows session, a running agent, or a GUI. The
//! `fanal` binary is only responsible for collecting that data.

mod detect;
mod model;
mod parse;
mod settings;

pub use detect::{classify, Activity, Evidence, Observation};
pub use model::{AgentId, AgentSnapshot, Lamp, Severity};
pub use parse::{claude_status, codex_activity, kimi_activity, SessionReading};
pub use settings::{builtin_catalog, AgentConfig, AgentProfile, Language, Settings, Theme};
