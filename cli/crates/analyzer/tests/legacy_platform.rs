use std::{fs, io::Write};

use modernlink_analyzer::analyze_repository;

#[test]
fn derives_build_and_application_server_signals_from_cited_artifacts() {
    let repository = tempfile::tempdir().expect("temporary repository");
    let web_inf = repository.path().join("orders/src/main/webapp/WEB-INF");
    let meta_inf = repository
        .path()
        .join("orders/src/main/application/META-INF");
    let java_dir = repository
        .path()
        .join("orders/src/main/java/com/acme/orders");
    fs::create_dir_all(&web_inf).expect("WEB-INF");
    fs::create_dir_all(&meta_inf).expect("META-INF");
    fs::create_dir_all(&java_dir).expect("Java package");
    fs::write(
        repository.path().join("pom.xml"),
        "<project><modelVersion>4.0.0</modelVersion></project>",
    )
    .expect("Maven descriptor");
    fs::write(
        web_inf.join("jboss-web.xml"),
        "<jboss-web><context-root>/orders</context-root></jboss-web>",
    )
    .expect("JBoss descriptor");
    fs::write(
        meta_inf.join("weblogic-application.xml"),
        "<weblogic-application><connection-factory-jndi-name>jms/Orders</connection-factory-jndi-name><queue-jndi-name>jms/OrdersQueue</queue-jndi-name></weblogic-application>",
    )
    .expect("WebLogic descriptor");
    fs::write(
        java_dir.join("LegacyQueue.java"),
        "package com.acme.orders; import weblogic.jms.extensions.WLMessage; public class LegacyQueue {}",
    )
    .expect("Java fixture");

    let report = analyze_repository(repository.path()).expect("analysis");

    assert_eq!(report.summary.java_files, 1);
    assert_eq!(report.summary.configuration_files, 3);
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "orders/src/main/application/META-INF/weblogic-application.xml"
            && artifact.parse_health == "xml-streamed"
    }));
    assert_signal(
        &report,
        "build-system",
        "maven",
        "descriptor.filename.pom-xml",
        "pom.xml",
    );
    assert_signal(
        &report,
        "integration-boundary",
        "jndi",
        "descriptor.xml.jndi-reference",
        "orders/src/main/application/META-INF/weblogic-application.xml",
    );
    assert_signal(
        &report,
        "integration-boundary",
        "jms",
        "descriptor.xml.jms-destination",
        "orders/src/main/application/META-INF/weblogic-application.xml",
    );
    assert_signal(
        &report,
        "application-server",
        "jboss",
        "descriptor.path.jboss-web-xml",
        "orders/src/main/webapp/WEB-INF/jboss-web.xml",
    );
    assert_signal(
        &report,
        "application-server",
        "weblogic",
        "descriptor.path.weblogic-application-xml",
        "orders/src/main/application/META-INF/weblogic-application.xml",
    );
    assert_signal(
        &report,
        "vendor-api",
        "weblogic",
        "java.import-prefix.weblogic",
        "orders/src/main/java/com/acme/orders/LegacyQueue.java",
    );
}

#[test]
fn ignores_deployment_descriptor_names_outside_standard_locations() {
    let repository = tempfile::tempdir().expect("temporary repository");
    let docs = repository.path().join("docs");
    fs::create_dir_all(&docs).expect("docs directory");
    fs::write(docs.join("web.xml"), "<example/>").expect("example web.xml");
    fs::write(docs.join("weblogic.xml"), "<example/>").expect("example weblogic.xml");

    let report = analyze_repository(repository.path()).expect("analysis");

    assert_eq!(report.summary.configuration_files, 0);
    assert!(report.signals.is_empty());
}

#[test]
fn records_malformed_recognized_descriptor_without_claiming_content_evidence() {
    let repository = tempfile::tempdir().expect("temporary repository");
    let web_inf = repository.path().join("app/src/main/webapp/WEB-INF");
    fs::create_dir_all(&web_inf).expect("WEB-INF");
    fs::write(
        web_inf.join("weblogic.xml"),
        "<weblogic-web-app><queue-jndi-name>",
    )
    .expect("malformed descriptor");

    let report = analyze_repository(repository.path()).expect("analysis");

    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "app/src/main/webapp/WEB-INF/weblogic.xml"
            && artifact.parse_health == "xml-malformed"
    }));
    assert!(
        !report
            .signals
            .iter()
            .any(|signal| signal.rule_id == "descriptor.xml.jms-destination")
    );
}

#[test]
fn derives_legacy_infrastructure_signals_from_import_evidence() {
    let repository = tempfile::tempdir().expect("temporary repository");
    let java_dir = repository.path().join("src/main/java/com/acme/legacy");
    fs::create_dir_all(&java_dir).expect("Java package");
    fs::write(
        java_dir.join("Infrastructure.java"),
        "package com.acme.legacy;\nimport javax.jms.Queue;\nimport javax.naming.InitialContext;\nimport javax.persistence.EntityManager;\nimport javax.transaction.UserTransaction;\nimport javax.xml.ws.Service;\npublic class Infrastructure {}",
    )
    .expect("Java fixture");

    let report = analyze_repository(repository.path()).expect("analysis");
    for (category, technology, rule_id) in [
        (
            "integration-boundary",
            "jms",
            "java.import-prefix.javax-jms",
        ),
        (
            "integration-boundary",
            "jndi",
            "java.import-prefix.javax-naming",
        ),
        (
            "data-access",
            "persistence",
            "java.import-prefix.persistence",
        ),
        ("transaction", "jta", "java.import-prefix.transaction"),
        (
            "integration-boundary",
            "jax-ws",
            "java.import-prefix.jax-ws",
        ),
    ] {
        assert_signal(
            &report,
            category,
            technology,
            rule_id,
            "src/main/java/com/acme/legacy/Infrastructure.java",
        );
    }
}

#[test]
fn derives_classfile_version_evidence_without_executing_bytecode() {
    let repository = tempfile::tempdir().expect("temporary repository");
    let classes = repository.path().join("target/classes/com/acme");
    fs::create_dir_all(&classes).expect("class directory");
    fs::write(
        classes.join("Legacy.class"),
        [0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 52, 0, 1],
    )
    .expect("class fixture");

    let report = analyze_repository(repository.path()).expect("analysis");
    assert_eq!(report.summary.class_files, 1);
    assert!(
        report
            .artifacts
            .iter()
            .any(|artifact| artifact.language == "java-bytecode"
                && artifact.parse_health == "header-only")
    );
    assert_signal(
        &report,
        "java-bytecode",
        "java-8",
        "classfile.major-version",
        "target/classes/com/acme/Legacy.class",
    );
}

#[test]
fn indexes_nested_class_headers_in_a_war_without_executing_them() {
    let repository = tempfile::tempdir().expect("temporary repository");
    let archive_file = fs::File::create(repository.path().join("legacy.war")).expect("archive");
    let mut archive = zip::ZipWriter::new(archive_file);
    archive
        .start_file(
            "WEB-INF/classes/com/acme/Legacy.class",
            zip::write::SimpleFileOptions::default(),
        )
        .expect("class entry");
    archive
        .write_all(&[0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 61, 0, 1])
        .expect("class header");
    archive.finish().expect("finish archive");

    let report = analyze_repository(repository.path()).expect("analysis");
    assert_eq!(report.summary.archive_files, 1);
    assert!(
        report
            .artifacts
            .iter()
            .any(|artifact| artifact.language == "java-war" && artifact.parse_health == "indexed")
    );
    assert_signal(
        &report,
        "deployment-archive",
        "war",
        "archive.extension",
        "legacy.war",
    );
    assert_signal(
        &report,
        "java-bytecode",
        "java-17",
        "classfile.major-version",
        "legacy.war",
    );
}

#[test]
fn derives_static_boundary_signals_from_recognized_annotations() {
    let repository = tempfile::tempdir().expect("temporary repository");
    let java_dir = repository.path().join("src/main/java/com/acme");
    fs::create_dir_all(&java_dir).expect("Java package");
    fs::write(
        java_dir.join("Endpoints.java"),
        "package com.acme; @Transactional public class PaymentService {} @MessageDriven public class Consumer {} @WebService public class LegacySoap {} @Path(\"/payments\") public class PaymentEndpoint {}",
    )
    .expect("Java fixture");
    let report = analyze_repository(repository.path()).expect("analysis");
    for (category, technology, rule_id) in [
        (
            "transaction-boundary",
            "transaction-annotation",
            "java.annotation.transaction",
        ),
        (
            "messaging-boundary",
            "message-consumer-annotation",
            "java.annotation.messaging",
        ),
        (
            "integration-boundary",
            "soap-annotation",
            "java.annotation.soap",
        ),
        (
            "http-boundary",
            "http-endpoint-annotation",
            "java.annotation.http",
        ),
    ] {
        assert_signal(
            &report,
            category,
            technology,
            rule_id,
            "src/main/java/com/acme/Endpoints.java",
        );
    }
}

fn assert_signal(
    report: &modernlink_analyzer::AnalysisReport,
    category: &str,
    technology: &str,
    rule_id: &str,
    evidence_path: &str,
) {
    let signal = report
        .signals
        .iter()
        .find(|signal| {
            signal.category == category
                && signal.technology == technology
                && signal.rule_id == rule_id
        })
        .unwrap_or_else(|| panic!("missing {category}/{technology}/{rule_id}"));
    assert_eq!(signal.epistemic_state, "derived");
    assert!(signal.evidence_ids.iter().any(|id| {
        report
            .evidence
            .iter()
            .any(|evidence| &evidence.id == id && evidence.path == evidence_path)
    }));
}
