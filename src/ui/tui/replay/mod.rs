use std::collections::{HashMap, VecDeque, hash_map::DefaultHasher};
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};

use ftui::core::event::{KeyCode, KeyEventKind, Modifiers, MouseButton, MouseEventKind};
use ftui::{Frame, GraphemePool, Model};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::bootstrap_config::AppDependencies;
use super::*;
use crate::domain::{PullRequest, PullRequestStatus};

const REPLAY_SCHEMA_VERSION: u64 = 1;
const REPLAY_FIXTURE_DIRECTORY: &str = "tests/fixtures/replay";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReplayOptions {
    pub invariant_only: bool,
    pub snapshot_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayOutcome {
    pub trace_path: PathBuf,
    pub steps_replayed: usize,
    pub states_compared: usize,
    pub frames_compared: usize,
}

include!("types.rs");
include!("fixtures.rs");
include!("trace_parser.rs");
include!("engine.rs");

#[cfg(test)]
mod tests;
