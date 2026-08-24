use std::{fs, process::Command};

#[test]
fn verify_command_reports_static_plan_gaps_without_claiming_migration_safety() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let plan_path = directory.path().join("plan.json");
    let output_path = directory.path().join("verification.json");
    let plan = inference::MigrationPlan {
        schema_version: "modernlink.migration-plan/v1alpha1".to_owned(),
        target_version: 21,
        tasks: vec![inference::MigrationPlanTask {
            id: "migration".to_owned(),
            state: model::ClaimState::Hypothesis,
            kind: "seam-migration".to_owned(),
            summary: "fixture".to_owned(),
            depends_on: vec!["missing".to_owned()],
            evidence_ids: Vec::new(),
            approval_required: false,
        }],
    };
    fs::write(&plan_path, plan.canonical_json().expect("plan JSON")).expect("write plan");
    let output = Command::new(env!("CARGO_BIN_EXE_modernlink"))
        .args(["verify", "--plan"])
        .arg(&plan_path)
        .args(["--output"])
        .arg(&output_path)
        .output()
        .expect("run verify");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(output_path).expect("report")).expect("JSON");
    assert_eq!(report["schema_version"], "modernlink.verification/v1alpha1");
    assert!(
        report["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["status"] == "GAP")
    );
    assert!(
        report["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item
                .as_str()
                .unwrap()
                .contains("does not verify code behavior"))
    );
}
