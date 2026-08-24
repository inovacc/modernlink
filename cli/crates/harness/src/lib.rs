use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HarnessError {
    #[error("duplicate harness identifier: {0}")]
    DuplicateId(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarnessCapability {
    Skills,
    SlashCommands,
    Agents,
    Mcp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessDefinition {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub detection_markers: Vec<String>,
    /// Deliberately empty until an adapter's install root is verified against its official contract.
    #[serde(default)]
    pub installation_paths: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<HarnessCapability>,
    pub adapter_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessRegistry {
    definitions: Vec<HarnessDefinition>,
}

impl HarnessRegistry {
    pub fn builtin() -> Self {
        Self {
            definitions: vec![
                HarnessDefinition {
                    id: "claude".to_owned(),
                    display_name: "Claude Code".to_owned(),
                    detection_markers: vec![".claude".to_owned()],
                    installation_paths: Vec::new(),
                    capabilities: vec![
                        HarnessCapability::Skills,
                        HarnessCapability::SlashCommands,
                        HarnessCapability::Agents,
                        HarnessCapability::Mcp,
                    ],
                    adapter_status: "bundled-plugin-contract".to_owned(),
                },
                HarnessDefinition {
                    id: "codex".to_owned(),
                    display_name: "Codex".to_owned(),
                    detection_markers: vec![".codex".to_owned()],
                    installation_paths: Vec::new(),
                    capabilities: vec![HarnessCapability::Skills, HarnessCapability::Agents],
                    adapter_status: "bundled-plugin-contract".to_owned(),
                },
            ],
        }
    }

    pub fn new(mut definitions: Vec<HarnessDefinition>) -> Result<Self, HarnessError> {
        definitions.sort_by(|left, right| left.id.cmp(&right.id));
        for pair in definitions.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(HarnessError::DuplicateId(pair[0].id.clone()));
            }
        }
        Ok(Self { definitions })
    }

    pub fn all(&self) -> &[HarnessDefinition] {
        &self.definitions
    }

    pub fn get(&self, id: &str) -> Option<&HarnessDefinition> {
        self.definitions
            .iter()
            .find(|definition| definition.id == id)
    }

    pub fn canonical_json(&self) -> Result<String, serde_json::Error> {
        let mut definitions = self.definitions.clone();
        definitions.sort_by(|left, right| left.id.cmp(&right.id));
        let mut json = serde_json::to_string_pretty(&definitions)?;
        json.push('\n');
        Ok(json)
    }
}
