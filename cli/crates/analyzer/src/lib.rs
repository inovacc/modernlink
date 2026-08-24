use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Cursor, Read},
    path::Path,
};

use ignore::WalkBuilder;
use quick_xml::{Reader, events::Event};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tree_sitter::{Node, Parser};

const SCHEMA_VERSION: &str = "modernlink.analysis/v1alpha1";
const COLLECTOR: &str = "java-tree-sitter";
const DESCRIPTOR_COLLECTOR: &str = "repository-descriptor";
const BYTECODE_COLLECTOR: &str = "classfile-header";
const SQL_COLLECTOR: &str = "sql-lexical";
const COLLECTOR_VERSION: &str = "0.1.0";
const MAX_BYTECODE_ENTRY_BYTES: u64 = 16 * 1024 * 1024;
const MAX_DESCRIPTOR_ENTRY_BYTES: u64 = 4 * 1024 * 1024;

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
    #[error("cannot adapt analysis report to evidence graph: {0}")]
    EvidenceGraph(#[from] model::GraphError),
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
    pub signals: Vec<TechnologySignal>,
}

impl AnalysisReport {
    pub fn canonical_json(&self) -> Result<String, AnalysisError> {
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        Ok(json)
    }

    pub fn to_evidence_graph(&self) -> Result<model::EvidenceGraph, AnalysisError> {
        let evidence = self
            .evidence
            .iter()
            .map(|item| model::Evidence {
                id: item.id.clone(),
                kind: item.observation_kind.clone(),
                value: item.observed_value.clone(),
                source: model::SourceLocation {
                    path: item.path.clone(),
                    start_byte: item.start_byte,
                    end_byte: item.end_byte,
                },
            })
            .collect::<Vec<_>>();
        let mut nodes = self
            .artifacts
            .iter()
            .map(|artifact| model::GraphNode {
                id: artifact.id.clone(),
                kind: "artifact".to_owned(),
                name: artifact.path.clone(),
                evidence_ids: Vec::new(),
            })
            .collect::<Vec<_>>();
        nodes.extend(self.nodes.iter().map(|node| model::GraphNode {
            id: node.id.clone(),
            kind: node.kind.clone(),
            name: node.qualified_name.clone(),
            evidence_ids: node.evidence_ids.clone(),
        }));

        let mut known_ids = nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<BTreeSet<_>>();
        let names_to_ids = nodes
            .iter()
            .map(|node| (node.name.clone(), node.id.clone()))
            .collect::<BTreeMap<_, _>>();
        let mut edges = Vec::new();
        for edge in &self.edges {
            let target_id = if known_ids.contains(&edge.target_name) {
                edge.target_name.clone()
            } else if let Some(id) = names_to_ids.get(&edge.target_name) {
                id.clone()
            } else {
                let id = model::stable_id("external-node", [edge.target_name.as_str()]);
                if known_ids.insert(id.clone()) {
                    nodes.push(model::GraphNode {
                        id: id.clone(),
                        kind: "external-reference".to_owned(),
                        name: edge.target_name.clone(),
                        evidence_ids: Vec::new(),
                    });
                }
                id
            };
            edges.push(model::GraphEdge {
                id: edge.id.clone(),
                kind: edge.kind.clone(),
                source_id: edge.source_id.clone(),
                target_id,
                evidence_ids: edge.evidence_ids.clone(),
            });
        }
        let claims = self
            .signals
            .iter()
            .map(|signal| model::Claim {
                id: signal.id.clone(),
                state: model::ClaimState::Inference,
                summary: format!(
                    "{} signal detected by deterministic rule {}",
                    signal.technology, signal.rule_id
                ),
                evidence_ids: signal.evidence_ids.clone(),
            })
            .collect();
        let graph = model::EvidenceGraph {
            schema_version: model::EVIDENCE_SCHEMA_VERSION.to_owned(),
            evidence,
            nodes,
            edges,
            claims,
        };
        graph.validate()?;
        Ok(graph)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisSummary {
    pub java_files: usize,
    pub class_files: usize,
    pub archive_files: usize,
    pub configuration_files: usize,
    pub sql_files: usize,
    pub parse_errors: usize,
    pub evidence_items: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub technology_signals: usize,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TechnologySignal {
    pub id: String,
    pub category: String,
    pub technology: String,
    pub rule_id: String,
    pub epistemic_state: String,
    pub evidence_ids: Vec<String>,
}

#[derive(Default)]
struct FileFacts {
    package: Option<Fact>,
    imports: Vec<Fact>,
    annotations: Vec<Fact>,
    types: Vec<Fact>,
    method_calls: Vec<Fact>,
    sql_literals: Vec<Fact>,
    jms_destinations: Vec<Fact>,
    type_relations: Vec<TypeRelation>,
}

struct Fact {
    name: String,
    start_byte: usize,
    end_byte: usize,
}

struct TypeRelation {
    source_type: String,
    kind: &'static str,
    target: Fact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SqlAccessKind {
    Read,
    Write,
    SchemaWrite,
}

impl SqlAccessKind {
    fn edge_kind(self) -> &'static str {
        match self {
            Self::Read => "reads",
            Self::Write | Self::SchemaWrite => "writes",
        }
    }

    fn observation_kind(self) -> &'static str {
        match self {
            Self::Read => "sql-table-read",
            Self::Write => "sql-table-write",
            Self::SchemaWrite => "sql-table-schema-write",
        }
    }

    fn rule_id(self) -> &'static str {
        match self {
            Self::Read => "sql.table-read",
            Self::Write => "sql.table-write",
            Self::SchemaWrite => "sql.table-schema-write",
        }
    }
}

struct SqlToken {
    text: String,
    start_byte: usize,
    end_byte: usize,
}

struct SignalRule {
    category: &'static str,
    technology: &'static str,
    rule_id: &'static str,
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
                || path
                    .extension()
                    .is_some_and(|extension| extension == "class")
                || path.extension().is_some_and(|extension| {
                    matches!(extension.to_str(), Some("jar" | "war" | "ear"))
                })
                || path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("sql"))
                || descriptor_rule(&repository_relative(repository, path)).is_some()
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
    let mut signals = Vec::new();
    let mut parse_errors = 0;
    let mut java_files = 0;
    let mut class_files = 0;
    let mut archive_files = 0;
    let mut configuration_files = 0;
    let mut sql_files = 0;

    for path in paths {
        let relative = repository_relative(repository, &path);
        let source = fs::read(&path).map_err(|source| AnalysisError::ReadArtifact {
            path: relative.clone(),
            source,
        })?;
        let digest = format!("sha256:{}", hex::encode(Sha256::digest(&source)));
        let artifact_id = stable_id("artifact", [relative.as_str(), digest.as_str()]);
        if path
            .extension()
            .is_some_and(|extension| extension == "java")
        {
            java_files += 1;
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
                &mut signals,
            );
        } else if path
            .extension()
            .is_some_and(|extension| extension == "class")
        {
            class_files += 1;
            let parse_health = add_classfile_facts(
                &relative,
                &artifact_id,
                &source,
                &mut evidence,
                &mut edges,
                &mut signals,
            );
            artifacts.push(Artifact {
                id: artifact_id.clone(),
                path: relative.clone(),
                digest,
                language: "java-bytecode".to_owned(),
                parse_health,
            });
        } else if let Some(kind) = archive_type(&path) {
            archive_files += 1;
            let parse_health = add_archive_facts(
                &relative,
                &artifact_id,
                &source,
                kind,
                &mut evidence,
                &mut edges,
                &mut signals,
            );
            artifacts.push(Artifact {
                id: artifact_id.clone(),
                path: relative.clone(),
                digest,
                language: format!("java-{kind}"),
                parse_health,
            });
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("sql"))
        {
            sql_files += 1;
            let parse_health = add_sql_facts(
                &relative,
                &artifact_id,
                &source,
                &mut evidence,
                &mut nodes,
                &mut edges,
                &mut signals,
            );
            artifacts.push(Artifact {
                id: artifact_id.clone(),
                path: relative.clone(),
                digest,
                language: "sql".to_owned(),
                parse_health,
            });
        } else if let Some(rule) = descriptor_rule(&relative) {
            configuration_files += 1;
            let parse_health = if source.is_empty() {
                "empty".to_owned()
            } else if is_gradle_build_file(&relative) {
                add_gradle_java_version_facts(
                    &relative,
                    &artifact_id,
                    &source,
                    &mut evidence,
                    &mut signals,
                );
                "configuration-lexed".to_owned()
            } else {
                let parse_health = add_descriptor_content_facts(
                    &relative,
                    &artifact_id,
                    &source,
                    &mut evidence,
                    &mut edges,
                    &mut signals,
                );
                if is_maven_pom(&relative) {
                    add_maven_java_version_facts(
                        &relative,
                        &artifact_id,
                        &source,
                        &mut evidence,
                        &mut signals,
                    );
                }
                parse_health
            };
            artifacts.push(Artifact {
                id: artifact_id.clone(),
                path: relative.clone(),
                digest,
                language: descriptor_language(&relative).to_owned(),
                parse_health,
            });
            if !source.is_empty() {
                let fact = Fact {
                    name: relative.clone(),
                    start_byte: 0,
                    end_byte: source.len(),
                };
                let evidence_id = push_evidence_with_collector(
                    "descriptor-path",
                    &fact,
                    &relative,
                    &artifact_id,
                    DESCRIPTOR_COLLECTOR,
                    &mut evidence,
                );
                signals.push(signal_from_rule(rule, evidence_id));
            }
        }
    }

    artifacts.sort_by(|a, b| a.path.cmp(&b.path));
    evidence.sort_by(|a, b| a.id.cmp(&b.id));
    let mut nodes = coalesce_nodes(nodes);
    let mut edges = coalesce_edges(edges);
    let mut signals = coalesce_signals(signals);
    nodes.sort_by(|a, b| {
        (&a.qualified_name, &a.kind, &a.id).cmp(&(&b.qualified_name, &b.kind, &b.id))
    });
    edges.sort_by(|a, b| (&a.kind, &a.target_name, &a.id).cmp(&(&b.kind, &b.target_name, &b.id)));
    signals.sort_by(|a, b| {
        (&a.category, &a.technology, &a.rule_id, &a.id).cmp(&(
            &b.category,
            &b.technology,
            &b.rule_id,
            &b.id,
        ))
    });

    let repository_digest = stable_id(
        "repository",
        artifacts
            .iter()
            .flat_map(|artifact| [artifact.path.as_str(), artifact.digest.as_str()]),
    );
    let summary = AnalysisSummary {
        java_files,
        class_files,
        archive_files,
        configuration_files,
        sql_files,
        parse_errors,
        evidence_items: evidence.len(),
        graph_nodes: nodes.len(),
        graph_edges: edges.len(),
        technology_signals: signals.len(),
    };

    Ok(AnalysisReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        repository_digest,
        summary,
        artifacts,
        evidence,
        nodes,
        edges,
        signals,
    })
}

fn add_descriptor_content_facts(
    path: &str,
    artifact_id: &str,
    source: &[u8],
    evidence: &mut Vec<Evidence>,
    edges: &mut Vec<GraphEdge>,
    signals: &mut Vec<TechnologySignal>,
) -> String {
    let evidence_start = evidence.len();
    let edges_start = edges.len();
    let signals_start = signals.len();
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(true);
    let mut tag = None::<String>;
    let mut buffer = Vec::new();
    let mut depth = 0_usize;
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) => {
                depth += 1;
                tag = std::str::from_utf8(element.name().as_ref())
                    .ok()
                    .map(|name| name.to_ascii_lowercase())
            }
            Ok(Event::End(_)) => {
                depth = depth.saturating_sub(1);
                tag = None;
            }
            Ok(Event::Text(text)) => {
                let Some(tag) = &tag else {
                    buffer.clear();
                    continue;
                };
                let Ok(value) = text.decode() else {
                    buffer.clear();
                    continue;
                };
                let value = value.trim();
                let rule = if tag.contains("queue") || tag.contains("topic") || tag.contains("jms")
                {
                    Some(SignalRule {
                        category: "integration-boundary",
                        technology: "jms",
                        rule_id: "descriptor.xml.jms-destination",
                    })
                } else if tag.contains("jndi") || tag.contains("datasource") {
                    Some(SignalRule {
                        category: "integration-boundary",
                        technology: "jndi",
                        rule_id: "descriptor.xml.jndi-reference",
                    })
                } else if tag.contains("trans-attribute")
                    || tag.contains("transaction-type")
                    || tag.contains("transaction-manager")
                {
                    Some(SignalRule {
                        category: "transaction-boundary",
                        technology: "transaction-descriptor",
                        rule_id: "descriptor.xml.transaction-boundary",
                    })
                } else {
                    None
                };
                if let Some(rule) = rule.filter(|_| !value.is_empty()) {
                    let observation_kind = if rule.rule_id == "descriptor.xml.transaction-boundary"
                    {
                        "transaction-descriptor-reference"
                    } else {
                        "descriptor-reference"
                    };
                    let start = source
                        .windows(value.len())
                        .position(|window| window == value.as_bytes())
                        .unwrap_or(0);
                    let fact = Fact {
                        name: value.to_owned(),
                        start_byte: start,
                        end_byte: start + value.len(),
                    };
                    let evidence_id = push_evidence_with_collector(
                        observation_kind,
                        &fact,
                        path,
                        artifact_id,
                        "xml-stream",
                        evidence,
                    );
                    let target_name = descriptor_target_name(rule.technology, value);
                    edges.push(GraphEdge {
                        id: stable_id(
                            "edge",
                            ["descriptor-references", artifact_id, target_name.as_str()],
                        ),
                        kind: "descriptor-references".to_owned(),
                        source_id: artifact_id.to_owned(),
                        target_name,
                        evidence_ids: vec![evidence_id.clone()],
                    });
                    signals.push(signal_from_rule(rule, evidence_id));
                }
            }
            Ok(Event::Eof) => {
                if depth == 0 {
                    return "xml-streamed".to_owned();
                }
                evidence.truncate(evidence_start);
                edges.truncate(edges_start);
                signals.truncate(signals_start);
                return "xml-malformed".to_owned();
            }
            Err(_) => {
                evidence.truncate(evidence_start);
                edges.truncate(edges_start);
                signals.truncate(signals_start);
                return "xml-malformed".to_owned();
            }
            _ => {}
        }
        buffer.clear();
    }
}

fn descriptor_target_name(technology: &str, value: &str) -> String {
    format!("{technology}:{value}")
}

fn add_classfile_facts(
    path: &str,
    artifact_id: &str,
    source: &[u8],
    evidence: &mut Vec<Evidence>,
    edges: &mut Vec<GraphEdge>,
    signals: &mut Vec<TechnologySignal>,
) -> String {
    let Some(major_version) = classfile_major_version(source) else {
        return "invalid-classfile".to_owned();
    };
    let fact = Fact {
        name: major_version.to_string(),
        start_byte: 6,
        end_byte: 8,
    };
    let evidence_id = push_evidence_with_collector(
        "classfile-major-version",
        &fact,
        path,
        artifact_id,
        BYTECODE_COLLECTOR,
        evidence,
    );
    let technology = classfile_java_version(major_version).map_or_else(
        || format!("classfile-major-{major_version}"),
        |version| format!("java-{version}"),
    );
    signals.push(TechnologySignal {
        id: stable_id(
            "signal",
            [
                "java-bytecode",
                technology.as_str(),
                "classfile.major-version",
            ],
        ),
        category: "java-bytecode".to_owned(),
        technology,
        rule_id: "classfile.major-version".to_owned(),
        epistemic_state: "derived".to_owned(),
        evidence_ids: vec![evidence_id],
    });
    if add_classfile_reference_facts(path, artifact_id, source, None, evidence, edges, signals)
        .is_err()
    {
        return "header-only".to_owned();
    }
    "constant-pool".to_owned()
}

fn add_sql_facts(
    path: &str,
    artifact_id: &str,
    source: &[u8],
    evidence: &mut Vec<Evidence>,
    nodes: &mut Vec<GraphNode>,
    edges: &mut Vec<GraphEdge>,
    signals: &mut Vec<TechnologySignal>,
) -> String {
    let Ok(text) = std::str::from_utf8(source) else {
        return "non-utf8".to_owned();
    };
    if text.is_empty() {
        return "empty".to_owned();
    }

    for (access, table) in sql_table_accesses(text) {
        add_sql_table_access(
            access,
            table,
            0,
            path,
            artifact_id,
            SQL_COLLECTOR,
            evidence,
            nodes,
            edges,
            signals,
        );
    }
    "lexed".to_owned()
}

#[allow(clippy::too_many_arguments)]
fn add_sql_table_access(
    access: SqlAccessKind,
    table: SqlToken,
    offset: usize,
    path: &str,
    artifact_id: &str,
    collector: &str,
    evidence: &mut Vec<Evidence>,
    nodes: &mut Vec<GraphNode>,
    edges: &mut Vec<GraphEdge>,
    signals: &mut Vec<TechnologySignal>,
) {
    let fact = Fact {
        name: table.text.clone(),
        start_byte: offset + table.start_byte,
        end_byte: offset + table.end_byte,
    };
    let evidence_id = push_evidence_with_collector(
        access.observation_kind(),
        &fact,
        path,
        artifact_id,
        collector,
        evidence,
    );
    let node_id = stable_id("database-table", [table.text.as_str()]);
    nodes.push(GraphNode {
        id: node_id,
        kind: "database-table".to_owned(),
        name: table.text.clone(),
        qualified_name: table.text.clone(),
        evidence_ids: vec![evidence_id.clone()],
    });
    edges.push(GraphEdge {
        id: stable_id(
            "edge",
            [access.edge_kind(), artifact_id, table.text.as_str()],
        ),
        kind: access.edge_kind().to_owned(),
        source_id: artifact_id.to_owned(),
        target_name: table.text,
        evidence_ids: vec![evidence_id.clone()],
    });
    signals.push(TechnologySignal {
        id: stable_id("signal", ["data-access", "sql", access.rule_id()]),
        category: "data-access".to_owned(),
        technology: "sql".to_owned(),
        rule_id: access.rule_id().to_owned(),
        epistemic_state: "derived".to_owned(),
        evidence_ids: vec![evidence_id],
    });
}

fn sql_table_accesses(source: &str) -> Vec<(SqlAccessKind, SqlToken)> {
    let tokens = sql_tokens(source);
    let mut accesses = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let keyword = tokens[index].text.to_ascii_uppercase();
        let (access, table_start) = match keyword.as_str() {
            "FROM" | "JOIN" => (SqlAccessKind::Read, index + 1),
            "INSERT" | "MERGE" => match tokens.get(index + 1) {
                Some(next) if next.text.eq_ignore_ascii_case("INTO") => {
                    (SqlAccessKind::Write, index + 2)
                }
                _ => {
                    index += 1;
                    continue;
                }
            },
            "UPDATE" => (SqlAccessKind::Write, index + 1),
            "DELETE" => match tokens.get(index + 1) {
                Some(next) if next.text.eq_ignore_ascii_case("FROM") => {
                    (SqlAccessKind::Write, index + 2)
                }
                _ => {
                    index += 1;
                    continue;
                }
            },
            "CREATE" | "ALTER" | "DROP" | "TRUNCATE" => match tokens.get(index + 1) {
                Some(next) if next.text.eq_ignore_ascii_case("TABLE") => {
                    (SqlAccessKind::SchemaWrite, index + 2)
                }
                _ => {
                    index += 1;
                    continue;
                }
            },
            _ => {
                index += 1;
                continue;
            }
        };
        if let Some((table, last_index)) = sql_table_name(&tokens, table_start) {
            accesses.push((access, table));
            index = last_index + 1;
        } else {
            index += 1;
        }
    }
    accesses
}

fn sql_table_name(tokens: &[SqlToken], start: usize) -> Option<(SqlToken, usize)> {
    let first = tokens.get(start)?;
    if first.text == "(" || first.text == ")" || first.text == "." {
        return None;
    }
    let mut text = first.text.clone();
    let start_byte = first.start_byte;
    let mut end_byte = first.end_byte;
    let mut index = start;
    while tokens.get(index + 1).is_some_and(|token| token.text == ".") {
        let name = tokens.get(index + 2)?;
        if matches!(name.text.as_str(), "(" | ")" | ".") {
            return None;
        }
        text.push('.');
        text.push_str(&name.text);
        end_byte = name.end_byte;
        index += 2;
    }
    Some((
        SqlToken {
            text,
            start_byte,
            end_byte,
        },
        index,
    ))
}

fn sql_tokens(source: &str) -> Vec<SqlToken> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            value if value.is_ascii_whitespace() => index += 1,
            b'-' if bytes.get(index + 1) == Some(&b'-') => {
                index += 2;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index += 2;
                while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/')
                {
                    index += 1;
                }
                index = (index + 2).min(bytes.len());
            }
            b'\'' => index = skip_sql_string(bytes, index, b'\''),
            b'\"' | b'`' | b'[' => {
                let quote = bytes[index];
                let close = if quote == b'[' { b']' } else { quote };
                let start = index;
                index = skip_sql_string(bytes, index, close);
                if index > start + 1 {
                    let end = index.saturating_sub(1);
                    tokens.push(SqlToken {
                        text: source[start + 1..end].to_owned(),
                        start_byte: start,
                        end_byte: index,
                    });
                }
            }
            b'.' | b'(' | b')' => {
                let start = index;
                index += 1;
                tokens.push(SqlToken {
                    text: source[start..index].to_owned(),
                    start_byte: start,
                    end_byte: index,
                });
            }
            value if value.is_ascii_alphabetic() || value == b'_' || value == b'$' => {
                let start = index;
                index += 1;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric()
                        || matches!(bytes[index], b'_' | b'$' | b'#'))
                {
                    index += 1;
                }
                tokens.push(SqlToken {
                    text: source[start..index].to_owned(),
                    start_byte: start,
                    end_byte: index,
                });
            }
            _ => index += 1,
        }
    }
    tokens
}

fn skip_sql_string(bytes: &[u8], start: usize, quote: u8) -> usize {
    let mut index = start + 1;
    while index < bytes.len() {
        if bytes[index] == quote {
            if bytes.get(index + 1) == Some(&quote) {
                index += 2;
            } else {
                return index + 1;
            }
        } else {
            index += 1;
        }
    }
    bytes.len()
}

fn add_classfile_reference_facts(
    path: &str,
    artifact_id: &str,
    source: &[u8],
    entry_name: Option<&str>,
    evidence: &mut Vec<Evidence>,
    edges: &mut Vec<GraphEdge>,
    signals: &mut Vec<TechnologySignal>,
) -> Result<(), ()> {
    let references = classfile_references(source)?;
    for reference in references {
        let fact = Fact {
            name: entry_name.map_or_else(
                || reference.name.clone(),
                |entry| format!("{entry}:{}", reference.name),
            ),
            start_byte: reference.start_byte,
            end_byte: reference.end_byte,
        };
        let evidence_id = push_evidence_with_collector(
            "bytecode-class-reference",
            &fact,
            path,
            artifact_id,
            BYTECODE_COLLECTOR,
            evidence,
        );
        if let Some(rule) = import_rule(&reference.name) {
            signals.push(signal_from_rule(rule, evidence_id.clone()));
        }
        edges.push(GraphEdge {
            id: stable_id(
                "edge",
                ["bytecode-references", artifact_id, reference.name.as_str()],
            ),
            kind: "bytecode-references".to_owned(),
            source_id: artifact_id.to_owned(),
            target_name: reference.name,
            evidence_ids: vec![evidence_id],
        });
    }
    Ok(())
}

fn archive_type(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "jar" => Some("jar"),
        "war" => Some("war"),
        "ear" => Some("ear"),
        _ => None,
    }
}

fn add_archive_facts(
    path: &str,
    artifact_id: &str,
    source: &[u8],
    archive_type: &str,
    evidence: &mut Vec<Evidence>,
    edges: &mut Vec<GraphEdge>,
    signals: &mut Vec<TechnologySignal>,
) -> String {
    let type_fact = Fact {
        name: archive_type.to_owned(),
        start_byte: 0,
        end_byte: 0,
    };
    let evidence_id = push_evidence_with_collector(
        "archive-type",
        &type_fact,
        path,
        artifact_id,
        "zip-central-directory",
        evidence,
    );
    signals.push(TechnologySignal {
        id: stable_id(
            "signal",
            ["deployment-archive", archive_type, "archive.extension"],
        ),
        category: "deployment-archive".to_owned(),
        technology: archive_type.to_owned(),
        rule_id: "archive.extension".to_owned(),
        epistemic_state: "derived".to_owned(),
        evidence_ids: vec![evidence_id],
    });
    let Ok(mut archive) = zip::ZipArchive::new(Cursor::new(source)) else {
        return "unreadable".to_owned();
    };
    for index in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(index) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }
        let entry_name = entry.name().to_owned();
        if let Some(rule) = descriptor_rule(&entry_name) {
            let fact = Fact {
                name: format!("{path}!{entry_name}"),
                start_byte: 0,
                end_byte: 0,
            };
            let evidence_id = push_evidence_with_collector(
                "archive-descriptor-path",
                &fact,
                path,
                artifact_id,
                "zip-central-directory",
                evidence,
            );
            signals.push(signal_from_rule(rule, evidence_id));
            if archive_descriptor_is_xml(&entry_name) {
                let mut descriptor = Vec::new();
                let read = entry
                    .by_ref()
                    .take(MAX_DESCRIPTOR_ENTRY_BYTES + 1)
                    .read_to_end(&mut descriptor);
                if read.is_ok() && descriptor.len() as u64 <= MAX_DESCRIPTOR_ENTRY_BYTES {
                    let descriptor_path = format!("{path}!{entry_name}");
                    add_descriptor_content_facts(
                        &descriptor_path,
                        artifact_id,
                        &descriptor,
                        evidence,
                        edges,
                        signals,
                    );
                }
            }
        }
        if !entry_name.ends_with(".class") {
            continue;
        }
        let mut header = [0_u8; 8];
        let Ok(read) = entry.read(&mut header) else {
            continue;
        };
        let Some(major) = classfile_major_version(&header[..read]) else {
            continue;
        };
        let fact = Fact {
            name: format!("{path}!{entry_name}:{major}"),
            start_byte: 0,
            end_byte: 0,
        };
        let evidence_id = push_evidence_with_collector(
            "archive-classfile-major-version",
            &fact,
            path,
            artifact_id,
            BYTECODE_COLLECTOR,
            evidence,
        );
        let technology = classfile_java_version(major).map_or_else(
            || format!("classfile-major-{major}"),
            |version| format!("java-{version}"),
        );
        signals.push(TechnologySignal {
            id: stable_id(
                "signal",
                [
                    "java-bytecode",
                    technology.as_str(),
                    "classfile.major-version",
                ],
            ),
            category: "java-bytecode".to_owned(),
            technology,
            rule_id: "classfile.major-version".to_owned(),
            epistemic_state: "derived".to_owned(),
            evidence_ids: vec![evidence_id],
        });
        if entry.size() > MAX_BYTECODE_ENTRY_BYTES {
            continue;
        }
        let mut classfile = header[..read].to_vec();
        if entry.read_to_end(&mut classfile).is_err() {
            continue;
        }
        let _ = add_classfile_reference_facts(
            path,
            artifact_id,
            &classfile,
            Some(&entry_name),
            evidence,
            edges,
            signals,
        );
    }
    "indexed".to_owned()
}

fn archive_descriptor_is_xml(entry_name: &str) -> bool {
    let lower = entry_name.to_ascii_lowercase();
    lower.ends_with(".xml") || lower.ends_with(".xmi")
}

fn classfile_major_version(source: &[u8]) -> Option<u16> {
    (source.len() >= 8 && source[..4] == [0xCA, 0xFE, 0xBA, 0xBE])
        .then(|| u16::from_be_bytes([source[6], source[7]]))
}

#[derive(Debug)]
struct ClassReference {
    name: String,
    start_byte: usize,
    end_byte: usize,
}

#[derive(Debug)]
enum ConstantPoolEntry {
    Empty,
    Utf8 {
        value: String,
    },
    Class {
        name_index: usize,
        start_byte: usize,
        end_byte: usize,
    },
}

fn classfile_references(source: &[u8]) -> Result<Vec<ClassReference>, ()> {
    if classfile_major_version(source).is_none() || source.len() < 10 {
        return Err(());
    }
    let constant_pool_count = usize::from(read_u16(source, 8).ok_or(())?);
    let mut entries = (0..constant_pool_count)
        .map(|_| ConstantPoolEntry::Empty)
        .collect::<Vec<_>>();
    let mut cursor = 10;
    let mut index = 1;
    while index < constant_pool_count {
        let tag = *source.get(cursor).ok_or(())?;
        cursor += 1;
        match tag {
            1 => {
                let length = usize::from(read_u16(source, cursor).ok_or(())?);
                cursor += 2;
                let end = cursor.checked_add(length).ok_or(())?;
                let value = std::str::from_utf8(source.get(cursor..end).ok_or(())?)
                    .map_err(|_| ())?
                    .to_owned();
                entries[index] = ConstantPoolEntry::Utf8 { value };
                cursor = end;
            }
            7 => {
                let start_byte = cursor - 1;
                let name_index = usize::from(read_u16(source, cursor).ok_or(())?);
                cursor += 2;
                entries[index] = ConstantPoolEntry::Class {
                    name_index,
                    start_byte,
                    end_byte: cursor,
                };
            }
            3 | 4 => cursor = cursor.checked_add(4).ok_or(())?,
            5 | 6 => {
                cursor = cursor.checked_add(8).ok_or(())?;
                index += 1;
                if index >= constant_pool_count {
                    return Err(());
                }
            }
            8 | 16 | 19 | 20 => cursor = cursor.checked_add(2).ok_or(())?,
            9 | 10 | 11 | 12 | 17 | 18 => cursor = cursor.checked_add(4).ok_or(())?,
            15 => cursor = cursor.checked_add(3).ok_or(())?,
            _ => return Err(()),
        }
        if cursor > source.len() {
            return Err(());
        }
        index += 1;
    }

    let mut references = BTreeMap::new();
    for entry in &entries {
        let ConstantPoolEntry::Class {
            name_index,
            start_byte,
            end_byte,
        } = entry
        else {
            continue;
        };
        let Some(ConstantPoolEntry::Utf8 { value }) = entries.get(*name_index) else {
            return Err(());
        };
        let Some(name) = normalize_class_reference(value) else {
            continue;
        };
        references.entry(name.clone()).or_insert(ClassReference {
            name,
            start_byte: *start_byte,
            end_byte: *end_byte,
        });
    }
    Ok(references.into_values().collect())
}

fn read_u16(source: &[u8], offset: usize) -> Option<u16> {
    let bytes = source.get(offset..offset.checked_add(2)?)?;
    Some(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn normalize_class_reference(value: &str) -> Option<String> {
    (!value.is_empty()
        && !value.starts_with('[')
        && !value.ends_with("module-info")
        && !value.ends_with("package-info"))
    .then(|| value.replace('/', "."))
}

fn classfile_java_version(major: u16) -> Option<u16> {
    match major {
        50 => Some(6),
        52 => Some(8),
        55 => Some(11),
        61 => Some(17),
        65 => Some(21),
        69 => Some(25),
        _ => None,
    }
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
        "marker_annotation" | "annotation" => {
            if let Ok(text) = node.utf8_text(source) {
                let name = text
                    .trim()
                    .trim_start_matches('@')
                    .split('(')
                    .next()
                    .unwrap_or_default()
                    .trim();
                if !name.is_empty() {
                    facts.annotations.push(Fact {
                        name: name.to_owned(),
                        start_byte: node.start_byte(),
                        end_byte: node.end_byte(),
                    });
                    if name.rsplit('.').next() == Some("JmsListener") {
                        if let Some(destination) = annotation_destination(text) {
                            facts.jms_destinations.push(Fact {
                                name: destination.to_owned(),
                                start_byte: node.start_byte() + text.find(destination).unwrap_or(0),
                                end_byte: node.start_byte()
                                    + text.find(destination).unwrap_or(0)
                                    + destination.len(),
                            });
                        }
                    }
                }
            }
        }
        "method_invocation" => {
            if let Some(name_node) = node.child_by_field_name("name")
                && let Ok(name) = name_node.utf8_text(source)
            {
                let target = node
                    .child_by_field_name("object")
                    .and_then(|object| object.utf8_text(source).ok())
                    .map(|object| format!("{object}.{name}"))
                    .unwrap_or_else(|| name.to_owned());
                facts.method_calls.push(Fact {
                    name: target,
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                });
            }
        }
        "string_literal" => {
            if let Ok(text) = node.utf8_text(source)
                && let Some(value) = text
                    .strip_prefix('"')
                    .and_then(|value| value.strip_suffix('"'))
                && !value.contains('\\')
                && !sql_table_accesses(value).is_empty()
            {
                facts.sql_literals.push(Fact {
                    name: value.to_owned(),
                    start_byte: node.start_byte() + 1,
                    end_byte: node.end_byte().saturating_sub(1),
                });
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
                collect_type_relations(node, source, name, facts);
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_facts(child, source, facts);
    }
}

fn annotation_destination(annotation: &str) -> Option<&str> {
    let marker = "destination";
    let start = annotation.find(marker)? + marker.len();
    let value = annotation[start..]
        .trim_start()
        .strip_prefix('=')?
        .trim_start();
    let value = value.strip_prefix('"')?;
    let end = value.find('"')?;
    (!value[..end].contains('\\')).then_some(&value[..end])
}

fn collect_type_relations(node: Node<'_>, source: &[u8], source_type: &str, facts: &mut FileFacts) {
    if let Some(superclass) = node.child_by_field_name("superclass")
        && let Some(target) = superclass.named_child(0)
        && let Ok(name) = target.utf8_text(source)
    {
        facts.type_relations.push(TypeRelation {
            source_type: source_type.to_owned(),
            kind: "extends",
            target: Fact {
                name: name.to_owned(),
                start_byte: target.start_byte(),
                end_byte: target.end_byte(),
            },
        });
    }
    if let Some(interfaces) = node.child_by_field_name("interfaces") {
        let Some(type_list) = interfaces.named_child(0) else {
            return;
        };
        let mut cursor = type_list.walk();
        for target in type_list.named_children(&mut cursor) {
            if let Ok(name) = target.utf8_text(source) {
                facts.type_relations.push(TypeRelation {
                    source_type: source_type.to_owned(),
                    kind: "implements",
                    target: Fact {
                        name: name.to_owned(),
                        start_byte: target.start_byte(),
                        end_byte: target.end_byte(),
                    },
                });
            }
        }
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
    signals: &mut Vec<TechnologySignal>,
) {
    facts.imports.sort_by(|a, b| a.name.cmp(&b.name));
    facts.annotations.sort_by(|a, b| a.name.cmp(&b.name));
    facts.types.sort_by(|a, b| a.name.cmp(&b.name));
    facts.method_calls.sort_by(|a, b| a.name.cmp(&b.name));
    facts.sql_literals.sort_by(|a, b| a.name.cmp(&b.name));
    facts.jms_destinations.sort_by(|a, b| a.name.cmp(&b.name));
    facts.type_relations.sort_by(|a, b| {
        (&a.source_type, a.kind, &a.target.name).cmp(&(&b.source_type, b.kind, &b.target.name))
    });
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
        if let Some(rule) = import_rule(&import.name) {
            signals.push(signal_from_rule(rule, evidence_id.clone()));
        }
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

    for annotation in facts.annotations {
        let evidence_id = push_evidence("annotation", &annotation, path, artifact_id, evidence);
        if let Some(rule) = annotation_rule(&annotation.name) {
            signals.push(signal_from_rule(rule, evidence_id));
        }
    }

    for call in facts.method_calls {
        let evidence_id = push_evidence("method-call", &call, path, artifact_id, evidence);
        let source_id = package_id.clone().unwrap_or_else(|| artifact_id.to_owned());
        edges.push(GraphEdge {
            id: stable_id("edge", ["calls", source_id.as_str(), call.name.as_str()]),
            kind: "calls".to_owned(),
            source_id,
            target_name: call.name,
            evidence_ids: vec![evidence_id],
        });
    }

    for literal in facts.sql_literals {
        for (access, table) in sql_table_accesses(&literal.name) {
            add_sql_table_access(
                access,
                table,
                literal.start_byte,
                path,
                artifact_id,
                "java-static-sql-literal",
                evidence,
                nodes,
                edges,
                signals,
            );
        }
    }

    for destination in facts.jms_destinations {
        let evidence_id = push_evidence(
            "jms-listener-destination",
            &destination,
            path,
            artifact_id,
            evidence,
        );
        edges.push(GraphEdge {
            id: stable_id("edge", ["consumes", artifact_id, destination.name.as_str()]),
            kind: "consumes".to_owned(),
            source_id: artifact_id.to_owned(),
            target_name: format!("jms:{}", destination.name),
            evidence_ids: vec![evidence_id.clone()],
        });
        signals.push(TechnologySignal {
            id: stable_id(
                "signal",
                [
                    "integration-boundary",
                    "jms",
                    "java.annotation.jms-listener-destination",
                ],
            ),
            category: "integration-boundary".to_owned(),
            technology: "jms".to_owned(),
            rule_id: "java.annotation.jms-listener-destination".to_owned(),
            epistemic_state: "derived".to_owned(),
            evidence_ids: vec![evidence_id],
        });
    }

    for relation in facts.type_relations {
        let evidence_id =
            push_evidence(relation.kind, &relation.target, path, artifact_id, evidence);
        let source_name = if package_name == "<default>" {
            relation.source_type
        } else {
            format!("{package_name}.{}", relation.source_type)
        };
        let source_id = stable_id("node", ["type", source_name.as_str()]);
        edges.push(GraphEdge {
            id: stable_id(
                "edge",
                [
                    relation.kind,
                    source_id.as_str(),
                    relation.target.name.as_str(),
                ],
            ),
            kind: relation.kind.to_owned(),
            source_id,
            target_name: relation.target.name,
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
    push_evidence_with_collector(kind, fact, path, artifact_id, COLLECTOR, evidence)
}

fn push_evidence_with_collector(
    kind: &str,
    fact: &Fact,
    path: &str,
    artifact_id: &str,
    collector: &str,
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
        collector: collector.to_owned(),
        collector_version: COLLECTOR_VERSION.to_owned(),
        observation_kind: kind.to_owned(),
        observed_value: fact.name.clone(),
    });
    id
}

fn signal_from_rule(rule: SignalRule, evidence_id: String) -> TechnologySignal {
    TechnologySignal {
        id: stable_id("signal", [rule.category, rule.technology, rule.rule_id]),
        category: rule.category.to_owned(),
        technology: rule.technology.to_owned(),
        rule_id: rule.rule_id.to_owned(),
        epistemic_state: "derived".to_owned(),
        evidence_ids: vec![evidence_id],
    }
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

fn coalesce_signals(signals: Vec<TechnologySignal>) -> Vec<TechnologySignal> {
    let mut coalesced = BTreeMap::<String, TechnologySignal>::new();
    for mut signal in signals {
        match coalesced.get_mut(&signal.id) {
            Some(existing) => existing.evidence_ids.append(&mut signal.evidence_ids),
            None => {
                coalesced.insert(signal.id.clone(), signal);
            }
        }
    }
    for signal in coalesced.values_mut() {
        signal.evidence_ids.sort();
        signal.evidence_ids.dedup();
    }
    coalesced.into_values().collect()
}

fn is_maven_pom(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case("pom.xml"))
}

fn is_gradle_build_file(path: &str) -> bool {
    matches!(
        path.rsplit('/').next().map(|name| name.to_ascii_lowercase()),
        Some(name) if matches!(name.as_str(), "build.gradle" | "build.gradle.kts")
    )
}

struct JavaVersionFactContext<'a> {
    path: &'a str,
    artifact_id: &'a str,
    evidence: &'a mut Vec<Evidence>,
    signals: &'a mut Vec<TechnologySignal>,
}

fn add_maven_java_version_facts(
    path: &str,
    artifact_id: &str,
    source: &[u8],
    evidence: &mut Vec<Evidence>,
    signals: &mut Vec<TechnologySignal>,
) {
    let Ok(text) = std::str::from_utf8(source) else {
        return;
    };
    let mut context = JavaVersionFactContext {
        path,
        artifact_id,
        evidence,
        signals,
    };
    for property in [
        "maven.compiler.source",
        "maven.compiler.target",
        "maven.compiler.release",
    ] {
        let open = format!("<{property}>");
        let close = format!("</{property}>");
        let Some(start) = text.find(&open) else {
            continue;
        };
        let value_start = start + open.len();
        let Some(value_end) = text[value_start..]
            .find(&close)
            .map(|end| value_start + end)
        else {
            continue;
        };
        add_declared_java_version(
            &mut context,
            property,
            &text[value_start..value_end],
            value_start,
            value_end,
        );
    }
}

fn add_gradle_java_version_facts(
    path: &str,
    artifact_id: &str,
    source: &[u8],
    evidence: &mut Vec<Evidence>,
    signals: &mut Vec<TechnologySignal>,
) {
    let Ok(text) = std::str::from_utf8(source) else {
        return;
    };
    let mut context = JavaVersionFactContext {
        path,
        artifact_id,
        evidence,
        signals,
    };
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        for (property, prefix) in [
            ("gradle.source-compatibility", "sourceCompatibility"),
            ("gradle.target-compatibility", "targetCompatibility"),
        ] {
            let Some(value) = trimmed
                .strip_prefix(prefix)
                .and_then(|rest| rest.trim_start().strip_prefix('='))
                .map(str::trim)
            else {
                continue;
            };
            let value = value.trim_matches(|character| character == '\'' || character == '"');
            let Some(relative_start) = line.find(value) else {
                continue;
            };
            add_declared_java_version(
                &mut context,
                property,
                value,
                offset + relative_start,
                offset + relative_start + value.len(),
            );
        }
        offset += line.len();
    }
}

fn add_declared_java_version(
    context: &mut JavaVersionFactContext<'_>,
    rule_id: &str,
    value: &str,
    start_byte: usize,
    end_byte: usize,
) {
    let Some(version) = normalize_declared_java_version(value) else {
        return;
    };
    let fact = Fact {
        name: version.to_string(),
        start_byte,
        end_byte,
    };
    let evidence_id = push_evidence_with_collector(
        "declared-java-version",
        &fact,
        context.path,
        context.artifact_id,
        DESCRIPTOR_COLLECTOR,
        context.evidence,
    );
    let technology = format!("java-{version}");
    context.signals.push(TechnologySignal {
        id: stable_id(
            "signal",
            ["java-configuration", technology.as_str(), rule_id],
        ),
        category: "java-configuration".to_owned(),
        technology,
        rule_id: rule_id.to_owned(),
        epistemic_state: "derived".to_owned(),
        evidence_ids: vec![evidence_id],
    });
}

fn normalize_declared_java_version(value: &str) -> Option<u16> {
    let value = value.trim();
    let value = value.strip_prefix("JavaVersion.VERSION_").unwrap_or(value);
    let value = value.replace('_', ".");
    let value = value.strip_prefix("1.").unwrap_or(&value);
    value.parse::<u16>().ok().filter(|version| *version > 0)
}

fn descriptor_rule(path: &str) -> Option<SignalRule> {
    let lower = path.to_ascii_lowercase();
    let mut components = lower.rsplit('/');
    let file_name = components.next()?;
    let parent = components.next();
    match (file_name, parent) {
        ("pom.xml", _) => Some(SignalRule {
            category: "build-system",
            technology: "maven",
            rule_id: "descriptor.filename.pom-xml",
        }),
        ("build.gradle" | "build.gradle.kts" | "settings.gradle" | "settings.gradle.kts", _) => {
            Some(SignalRule {
                category: "build-system",
                technology: "gradle",
                rule_id: "descriptor.filename.gradle",
            })
        }
        ("build.xml", _) => Some(SignalRule {
            category: "build-system",
            technology: "ant",
            rule_id: "descriptor.filename.build-xml",
        }),
        ("jboss-web.xml", Some("web-inf")) => Some(SignalRule {
            category: "application-server",
            technology: "jboss",
            rule_id: "descriptor.path.jboss-web-xml",
        }),
        (
            "jboss.xml" | "jboss-app.xml" | "jboss-deployment-structure.xml",
            Some("meta-inf" | "web-inf"),
        ) => Some(SignalRule {
            category: "application-server",
            technology: "jboss",
            rule_id: "descriptor.path.jboss-deployment",
        }),
        ("wildfly-config.xml", Some("meta-inf")) => Some(SignalRule {
            category: "application-server",
            technology: "wildfly",
            rule_id: "descriptor.path.wildfly-config-xml",
        }),
        ("weblogic.xml", Some("web-inf")) => Some(SignalRule {
            category: "application-server",
            technology: "weblogic",
            rule_id: "descriptor.path.weblogic-xml",
        }),
        ("weblogic-application.xml", Some("meta-inf")) => Some(SignalRule {
            category: "application-server",
            technology: "weblogic",
            rule_id: "descriptor.path.weblogic-application-xml",
        }),
        ("weblogic-ejb-jar.xml", Some("meta-inf")) => Some(SignalRule {
            category: "application-server",
            technology: "weblogic",
            rule_id: "descriptor.path.weblogic-ejb-jar-xml",
        }),
        ("ibm-web-bnd.xmi" | "ibm-web-ext.xmi", Some("web-inf"))
        | ("ibm-application-bnd.xmi", Some("meta-inf")) => Some(SignalRule {
            category: "application-server",
            technology: "websphere",
            rule_id: "descriptor.path.ibm-websphere",
        }),
        ("web.xml", Some("web-inf")) => Some(SignalRule {
            category: "deployment-model",
            technology: "java-ee-web",
            rule_id: "descriptor.path.web-xml",
        }),
        ("context.xml", Some("meta-inf")) => Some(SignalRule {
            category: "application-server",
            technology: "tomcat",
            rule_id: "descriptor.path.tomcat-context-xml",
        }),
        ("application.xml", Some("meta-inf")) => Some(SignalRule {
            category: "deployment-model",
            technology: "java-ee-ear",
            rule_id: "descriptor.path.application-xml",
        }),
        ("ejb-jar.xml", Some("meta-inf")) => Some(SignalRule {
            category: "deployment-model",
            technology: "ejb",
            rule_id: "descriptor.path.ejb-jar-xml",
        }),
        _ => None,
    }
}

fn descriptor_language(path: &str) -> &'static str {
    if path.to_ascii_lowercase().ends_with(".kts") {
        "kotlin"
    } else if path.to_ascii_lowercase().ends_with(".gradle") {
        "groovy"
    } else {
        "xml"
    }
}

fn import_rule(import: &str) -> Option<SignalRule> {
    let (category, technology, rule_id) = if import.starts_with("weblogic.") {
        ("vendor-api", "weblogic", "java.import-prefix.weblogic")
    } else if import.starts_with("org.wildfly.") {
        ("vendor-api", "wildfly", "java.import-prefix.org-wildfly")
    } else if import.starts_with("org.jboss.") {
        ("vendor-api", "jboss", "java.import-prefix.org-jboss")
    } else if import.starts_with("org.apache.catalina.") {
        ("vendor-api", "tomcat", "java.import-prefix.apache-catalina")
    } else if import.starts_with("com.ibm.websphere.") || import.starts_with("com.ibm.ws.") {
        (
            "vendor-api",
            "websphere",
            "java.import-prefix.ibm-websphere",
        )
    } else if import.starts_with("javax.jms.") {
        (
            "integration-boundary",
            "jms",
            "java.import-prefix.javax-jms",
        )
    } else if import.starts_with("javax.naming.") {
        (
            "integration-boundary",
            "jndi",
            "java.import-prefix.javax-naming",
        )
    } else if import.starts_with("javax.sql.") || import.starts_with("java.sql.") {
        ("data-access", "jdbc", "java.import-prefix.jdbc")
    } else if import.starts_with("javax.persistence.") || import.starts_with("org.hibernate.") {
        (
            "data-access",
            "persistence",
            "java.import-prefix.persistence",
        )
    } else if import.starts_with("javax.ejb.") {
        (
            "application-boundary",
            "ejb",
            "java.import-prefix.javax-ejb",
        )
    } else if import.starts_with("javax.transaction.") || import.starts_with("jakarta.transaction.")
    {
        ("transaction", "jta", "java.import-prefix.transaction")
    } else if import.starts_with("javax.xml.ws.") || import.starts_with("jakarta.xml.ws.") {
        (
            "integration-boundary",
            "jax-ws",
            "java.import-prefix.jax-ws",
        )
    } else if import.starts_with("javax.xml.bind.") || import.starts_with("jakarta.xml.bind.") {
        ("serialization", "jaxb", "java.import-prefix.jaxb")
    } else if import.starts_with("javax.management.") {
        ("management", "jmx", "java.import-prefix.jmx")
    } else if import.starts_with("javax.servlet.") || import.starts_with("jakarta.servlet.") {
        (
            "application-boundary",
            "servlet",
            "java.import-prefix.servlet",
        )
    } else if import.starts_with("java.rmi.") {
        ("integration-boundary", "rmi", "java.import-prefix.rmi")
    } else if import.starts_with("org.springframework.") {
        ("framework", "spring", "java.import-prefix.spring")
    } else {
        return None;
    };
    Some(SignalRule {
        category,
        technology,
        rule_id,
    })
}

fn annotation_rule(annotation: &str) -> Option<SignalRule> {
    let name = annotation.rsplit('.').next().unwrap_or(annotation);
    let (category, technology, rule_id) = match name {
        "Transactional" | "TransactionAttribute" => (
            "transaction-boundary",
            "transaction-annotation",
            "java.annotation.transaction",
        ),
        "MessageDriven" | "JmsListener" => (
            "messaging-boundary",
            "message-consumer-annotation",
            "java.annotation.messaging",
        ),
        "WebService" | "WebMethod" => (
            "integration-boundary",
            "soap-annotation",
            "java.annotation.soap",
        ),
        "Path" | "RequestMapping" => (
            "http-boundary",
            "http-endpoint-annotation",
            "java.annotation.http",
        ),
        "Scheduled" => (
            "batch-boundary",
            "scheduled-annotation",
            "java.annotation.batch",
        ),
        _ => return None,
    };
    Some(SignalRule {
        category,
        technology,
        rule_id,
    })
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
