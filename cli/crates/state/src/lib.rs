use std::{fs::OpenOptions, io::Write, path::Path};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("event sequence {actual} does not follow {expected}")]
    InvalidSequence { expected: u64, actual: u64 },
    #[error("transition from {from:?} to {to:?} is not allowed")]
    InvalidTransition {
        from: LifecyclePhase,
        to: LifecyclePhase,
    },
    #[error("transition from {from:?} to {to:?} requires explicit approval")]
    ApprovalRequired {
        from: LifecyclePhase,
        to: LifecyclePhase,
    },
    #[error("cannot serialize lifecycle state: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("cannot access lifecycle journal: {0}")]
    Journal(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LifecyclePhase {
    Setup,
    Discover,
    Understand,
    Model,
    Assess,
    Design,
    Plan,
    Prepare,
    Modernize,
    Migrate,
    Verify,
    Cutover,
    Detach,
    Cleanup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleEvent {
    pub id: String,
    pub sequence: u64,
    pub from: LifecyclePhase,
    pub to: LifecyclePhase,
    pub approved: bool,
    #[serde(default)]
    pub artifact_hashes: Vec<String>,
}

impl LifecycleEvent {
    pub fn new(sequence: u64, from: LifecyclePhase, to: LifecyclePhase, approved: bool) -> Self {
        let id = model::stable_id(
            "lifecycle-event",
            [
                sequence.to_string(),
                format!("{from:?}"),
                format!("{to:?}"),
                approved.to_string(),
            ],
        );
        Self {
            id,
            sequence,
            from,
            to,
            approved,
            artifact_hashes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleSnapshot {
    pub run_id: String,
    pub phase: LifecyclePhase,
    pub last_sequence: u64,
    pub event_count: u64,
    #[serde(default)]
    pub artifact_hashes: Vec<String>,
}

impl LifecycleSnapshot {
    pub fn new(run_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            phase: LifecyclePhase::Setup,
            last_sequence: 0,
            event_count: 0,
            artifact_hashes: Vec::new(),
        }
    }

    pub fn apply(&mut self, event: &LifecycleEvent) -> Result<(), StateError> {
        let expected_sequence = self.last_sequence + 1;
        if event.sequence != expected_sequence {
            return Err(StateError::InvalidSequence {
                expected: expected_sequence,
                actual: event.sequence,
            });
        }
        let expected_next = next_phase(self.phase).ok_or(StateError::InvalidTransition {
            from: self.phase,
            to: event.to,
        })?;
        if event.from != self.phase || event.to != expected_next {
            return Err(StateError::InvalidTransition {
                from: event.from,
                to: event.to,
            });
        }
        if requires_approval(event.to) && !event.approved {
            return Err(StateError::ApprovalRequired {
                from: event.from,
                to: event.to,
            });
        }
        self.phase = event.to;
        self.last_sequence = event.sequence;
        self.event_count += 1;
        self.artifact_hashes.extend(event.artifact_hashes.clone());
        self.artifact_hashes.sort();
        self.artifact_hashes.dedup();
        Ok(())
    }

    pub fn replay(
        run_id: impl Into<String>,
        events: &[LifecycleEvent],
    ) -> Result<Self, StateError> {
        let mut snapshot = Self::new(run_id);
        for event in events {
            snapshot.apply(event)?;
        }
        Ok(snapshot)
    }
}

pub fn event_json_line(event: &LifecycleEvent) -> Result<String, StateError> {
    Ok(format!("{}\n", serde_json::to_string(event)?))
}

pub fn parse_event_json_lines(input: &str) -> Result<Vec<LifecycleEvent>, StateError> {
    input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<Vec<_>, _>>()
        .map_err(StateError::Serialization)
}

pub fn append_event(path: &Path, event: &LifecycleEvent) -> Result<(), StateError> {
    let mut journal = OpenOptions::new().append(true).create(true).open(path)?;
    journal.write_all(event_json_line(event)?.as_bytes())?;
    journal.sync_data()?;
    Ok(())
}

pub fn recover_journal(
    run_id: impl Into<String>,
    path: &Path,
) -> Result<LifecycleSnapshot, StateError> {
    let journal = std::fs::read_to_string(path)?;
    let events = parse_event_json_lines(&journal)?;
    LifecycleSnapshot::replay(run_id, &events)
}

fn next_phase(phase: LifecyclePhase) -> Option<LifecyclePhase> {
    use LifecyclePhase::*;
    match phase {
        Setup => Some(Discover),
        Discover => Some(Understand),
        Understand => Some(Model),
        Model => Some(Assess),
        Assess => Some(Design),
        Design => Some(Plan),
        Plan => Some(Prepare),
        Prepare => Some(Modernize),
        Modernize => Some(Migrate),
        Migrate => Some(Verify),
        Verify => Some(Cutover),
        Cutover => Some(Detach),
        Detach => Some(Cleanup),
        Cleanup => None,
    }
}

fn requires_approval(phase: LifecyclePhase) -> bool {
    matches!(
        phase,
        LifecyclePhase::Modernize | LifecyclePhase::Cutover | LifecyclePhase::Detach
    )
}
