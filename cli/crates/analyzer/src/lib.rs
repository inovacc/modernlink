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
            artifacts.push(Artifact {
                id: artifact_id.clone(),
                path: relative.clone(),
                digest,
                language: "java-bytecode".to_owned(),
                parse_health: "header-only".to_owned(),
            });
            add_classfile_facts(
                &relative,
                &artifact_id,
                &source,
                &mut evidence,
                &mut signals,
            );
        } else if let Some(kind) = archive_type(&path) {
            archive_files += 1;
            let parse_health = add_archive_facts(
                &relative,
                &artifact_id,
                &source,
                kind,
                &mut evidence,
                &mut signals,
            );
            artifacts.push(Artifact {
                id: artifact_id.clone(),
                path: relative.clone(),
                digest,
                language: format!("java-{kind}"),
                parse_health,
            });
        } else if let Some(rule) = descriptor_rule(&relative) {
            configuration_files += 1;
            let parse_health = if source.is_empty() {
                "empty".to_owned()
            } else {
                add_descriptor_content_facts(
                    &relative,
                    &artifact_id,
                    &source,
                    &mut evidence,
                    &mut signals,
                )
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
    signals: &mut Vec<TechnologySignal>,
) -> String {
    let evidence_start = evidence.len();
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
                } else {
                    None
                };
                if let Some(rule) = rule.filter(|_| !value.is_empty()) {
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
                        "descriptor-reference",
                        &fact,
                        path,
                        artifact_id,
                        "xml-stream",
                        evidence,
                    );
                    signals.push(signal_from_rule(rule, evidence_id));
                }
            }
            Ok(Event::Eof) => {
                if depth == 0 {
                    return "xml-streamed".to_owned();
                }
                evidence.truncate(evidence_start);
                signals.truncate(signals_start);
                return "xml-malformed".to_owned();
            }
            Err(_) => {
                evidence.truncate(evidence_start);
                signals.truncate(signals_start);
                return "xml-malformed".to_owned();
            }
            _ => {}
        }
        buffer.clear();
    }
}

fn add_classfile_facts(
    path: &str,
    artifact_id: &str,
    source: &[u8],
    evidence: &mut Vec<Evidence>,
    signals: &mut Vec<TechnologySignal>,
) {
    let Some(major_version) = classfile_major_version(source) else {
        return;
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
        if entry.is_dir() || !entry.name().ends_with(".class") {
            continue;
        }
        let entry_name = entry.name().to_owned();
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
    }
    "indexed".to_owned()
}

fn classfile_major_version(source: &[u8]) -> Option<u16> {
    (source.len() >= 8 && source[..4] == [0xCA, 0xFE, 0xBA, 0xBE])
        .then(|| u16::from_be_bytes([source[6], source[7]]))
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
    } else if import.starts_with("org.jboss.") {
        ("vendor-api", "jboss", "java.import-prefix.org-jboss")
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
