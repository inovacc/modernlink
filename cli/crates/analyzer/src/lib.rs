use std::{collections::BTreeMap, fs, path::Path};

use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tree_sitter::{Node, Parser};

const SCHEMA_VERSION: &str = "modernlink.analysis/v1alpha1";
const COLLECTOR: &str = "java-tree-sitter";
const COLLECTOR_VERSION: &str = "0.1.0";

#[derive(Debug, Error)]
pub enum AnalysisError {
    #[error("repository path is not a directory: {0}")]
    NotDirectory(String),
    #[error("cannot read repository artifact {path}: {source}")]
    ReadArtifact {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot configure Java parser: {0}")]
    Parser(String),
    #[error("cannot serialize analysis report: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub schema_version: String,
    pub repository_digest: String,
    pub summary: AnalysisSummary,
    pub artifacts: Vec<Artifact>,
    pub evidence: Vec<Evidence>,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl AnalysisReport {
    pub fn canonical_json(&self) -> Result<String, AnalysisError> {
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        Ok(json)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisSummary {
    pub java_files: usize,
    pub parse_errors: usize,
    pub evidence_items: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub path: String,
    pub digest: String,
    pub language: String,
    pub parse_health: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub artifact_id: String,
    pub path: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub collector: String,
    pub collector_version: String,
    pub observation_kind: String,
    pub observed_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub qualified_name: String,
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub kind: String,
    pub source_id: String,
    pub target_name: String,
    pub evidence_ids: Vec<String>,
}

#[derive(Default)]
struct FileFacts {
    package: Option<Fact>,
    imports: Vec<Fact>,
    types: Vec<Fact>,
}

struct Fact {
    name: String,
    start_byte: usize,
    end_byte: usize,
}

pub fn analyze_repository(repository: &Path) -> Result<AnalysisReport, AnalysisError> {
    if !repository.is_dir() {
        return Err(AnalysisError::NotDirectory(
            repository.display().to_string(),
        ));
    }

    let mut paths = WalkBuilder::new(repository)
        .hidden(false)
        .follow_links(false)
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .map(|entry| entry.into_path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "java")
        })
        .collect::<Vec<_>>();
    paths.sort_by_key(|path| repository_relative(repository, path));

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .map_err(|error| AnalysisError::Parser(error.to_string()))?;

    let mut artifacts = Vec::new();
    let mut evidence = Vec::new();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut parse_errors = 0;

    for path in paths {
        let relative = repository_relative(repository, &path);
        let source = fs::read(&path).map_err(|source| AnalysisError::ReadArtifact {
            path: relative.clone(),
            source,
        })?;
        let digest = format!("sha256:{}", hex::encode(Sha256::digest(&source)));
        let artifact_id = stable_id("artifact", [relative.as_str(), digest.as_str()]);
        let tree = parser
            .parse(&source, None)
            .ok_or_else(|| AnalysisError::Parser(format!("parser cancelled for {relative}")))?;
        let has_error = tree.root_node().has_error();
        parse_errors += usize::from(has_error);
        artifacts.push(Artifact {
            id: artifact_id.clone(),
            path: relative.clone(),
            digest,
            language: "java".to_owned(),
            parse_health: if has_error {
                "recovered-error"
            } else {
                "complete"
            }
            .to_owned(),
        });

        let mut facts = FileFacts::default();
        collect_facts(tree.root_node(), &source, &mut facts);
        add_file_facts(
            &relative,
            &artifact_id,
            facts,
            &mut evidence,
            &mut nodes,
            &mut edges,
        );
    }

    artifacts.sort_by(|a, b| a.path.cmp(&b.path));
    evidence.sort_by(|a, b| a.id.cmp(&b.id));
    let mut nodes = coalesce_nodes(nodes);
    let mut edges = coalesce_edges(edges);
    nodes.sort_by(|a, b| {
        (&a.qualified_name, &a.kind, &a.id).cmp(&(&b.qualified_name, &b.kind, &b.id))
    });
    edges.sort_by(|a, b| (&a.kind, &a.target_name, &a.id).cmp(&(&b.kind, &b.target_name, &b.id)));

    let repository_digest = stable_id(
        "repository",
        artifacts
            .iter()
            .flat_map(|artifact| [artifact.path.as_str(), artifact.digest.as_str()]),
    );
    let summary = AnalysisSummary {
        java_files: artifacts.len(),
        parse_errors,
        evidence_items: evidence.len(),
        graph_nodes: nodes.len(),
        graph_edges: edges.len(),
    };

    Ok(AnalysisReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        repository_digest,
        summary,
        artifacts,
        evidence,
        nodes,
        edges,
    })
}

fn collect_facts(node: Node<'_>, source: &[u8], facts: &mut FileFacts) {
    match node.kind() {
        "package_declaration" => {
            if let Some(name) = declaration_value(node, source, "package") {
                facts.package = Some(name);
            }
        }
        "import_declaration" => {
            if let Some(name) = declaration_value(node, source, "import") {
                facts.imports.push(name);
            }
        }
        "class_declaration"
        | "interface_declaration"
        | "enum_declaration"
        | "record_declaration"
        | "annotation_type_declaration" => {
            if let Some(name_node) = node.child_by_field_name("name")
                && let Ok(name) = name_node.utf8_text(source)
            {
                facts.types.push(Fact {
                    name: name.to_owned(),
                    start_byte: name_node.start_byte(),
                    end_byte: name_node.end_byte(),
                });
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_facts(child, source, facts);
    }
}

fn declaration_value(node: Node<'_>, source: &[u8], keyword: &str) -> Option<Fact> {
    let text = node.utf8_text(source).ok()?;
    let value = text
        .trim()
        .strip_prefix(keyword)?
        .trim()
        .strip_prefix("static ")
        .unwrap_or_else(|| text.trim().strip_prefix(keyword).unwrap_or_default().trim())
        .trim_end_matches(';')
        .trim();
    if value.is_empty() {
        return None;
    }
    let offset = text.find(value)?;
    Some(Fact {
        name: value.to_owned(),
        start_byte: node.start_byte() + offset,
        end_byte: node.start_byte() + offset + value.len(),
    })
}

fn add_file_facts(
    path: &str,
    artifact_id: &str,
    mut facts: FileFacts,
    evidence: &mut Vec<Evidence>,
    nodes: &mut Vec<GraphNode>,
    edges: &mut Vec<GraphEdge>,
) {
    facts.imports.sort_by(|a, b| a.name.cmp(&b.name));
    facts.types.sort_by(|a, b| a.name.cmp(&b.name));
    let package_name = facts
        .package
        .as_ref()
        .map_or_else(|| "<default>".to_owned(), |fact| fact.name.clone());
    let package_id = facts.package.as_ref().map(|package| {
        let evidence_id = push_evidence("package", package, path, artifact_id, evidence);
        let node_id = stable_id("node", ["package", package.name.as_str()]);
        nodes.push(GraphNode {
            id: node_id.clone(),
            kind: "package".to_owned(),
            name: package.name.clone(),
            qualified_name: package.name.clone(),
            evidence_ids: vec![evidence_id],
        });
        node_id
    });

    for import in facts.imports {
        let evidence_id = push_evidence("import", &import, path, artifact_id, evidence);
        let source_id = package_id.clone().unwrap_or_else(|| artifact_id.to_owned());
        edges.push(GraphEdge {
            id: stable_id(
                "edge",
                ["imports", source_id.as_str(), import.name.as_str()],
            ),
            kind: "imports".to_owned(),
            source_id,
            target_name: import.name,
            evidence_ids: vec![evidence_id],
        });
    }

    for declared_type in facts.types {
        let evidence_id = push_evidence("type", &declared_type, path, artifact_id, evidence);
        let qualified_name = if package_name == "<default>" {
            declared_type.name.clone()
        } else {
            format!("{package_name}.{}", declared_type.name)
        };
        let node_id = stable_id("node", ["type", qualified_name.as_str()]);
        nodes.push(GraphNode {
            id: node_id.clone(),
            kind: "type".to_owned(),
            name: declared_type.name,
            qualified_name,
            evidence_ids: vec![evidence_id.clone()],
        });
        if let Some(source_id) = &package_id {
            edges.push(GraphEdge {
                id: stable_id("edge", ["contains", source_id, &node_id]),
                kind: "contains".to_owned(),
                source_id: source_id.clone(),
                target_name: node_id,
                evidence_ids: vec![evidence_id],
            });
        }
    }
}

fn push_evidence(
    kind: &str,
    fact: &Fact,
    path: &str,
    artifact_id: &str,
    evidence: &mut Vec<Evidence>,
) -> String {
    let start = fact.start_byte.to_string();
    let end = fact.end_byte.to_string();
    let id = stable_id(
        "evidence",
        [artifact_id, kind, &start, &end, fact.name.as_str()],
    );
    evidence.push(Evidence {
        id: id.clone(),
        artifact_id: artifact_id.to_owned(),
        path: path.to_owned(),
        start_byte: fact.start_byte,
        end_byte: fact.end_byte,
        collector: COLLECTOR.to_owned(),
        collector_version: COLLECTOR_VERSION.to_owned(),
        observation_kind: kind.to_owned(),
        observed_value: fact.name.clone(),
    });
    id
}

fn coalesce_nodes(nodes: Vec<GraphNode>) -> Vec<GraphNode> {
    let mut coalesced = BTreeMap::<String, GraphNode>::new();
    for mut node in nodes {
        match coalesced.get_mut(&node.id) {
            Some(existing) => existing.evidence_ids.append(&mut node.evidence_ids),
            None => {
                coalesced.insert(node.id.clone(), node);
            }
        }
    }
    for node in coalesced.values_mut() {
        node.evidence_ids.sort();
        node.evidence_ids.dedup();
    }
    coalesced.into_values().collect()
}

fn coalesce_edges(edges: Vec<GraphEdge>) -> Vec<GraphEdge> {
    let mut coalesced = BTreeMap::<String, GraphEdge>::new();
    for mut edge in edges {
        match coalesced.get_mut(&edge.id) {
            Some(existing) => existing.evidence_ids.append(&mut edge.evidence_ids),
            None => {
                coalesced.insert(edge.id.clone(), edge);
            }
        }
    }
    for edge in coalesced.values_mut() {
        edge.evidence_ids.sort();
        edge.evidence_ids.dedup();
    }
    coalesced.into_values().collect()
}

fn repository_relative(repository: &Path, path: &Path) -> String {
    path.strip_prefix(repository)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn stable_id<'a>(prefix: &str, components: impl IntoIterator<Item = &'a str>) -> String {
    let mut digest = Sha256::new();
    for component in components {
        digest.update((component.len() as u64).to_be_bytes());
        digest.update(component.as_bytes());
    }
    format!("{prefix}:sha256:{}", hex::encode(digest.finalize()))
}
