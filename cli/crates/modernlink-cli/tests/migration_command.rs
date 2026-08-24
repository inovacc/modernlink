use std::{fs, process::Command};

#[test]
fn migration_create_preserves_a_validated_plan_with_its_lifecycle_link() {
    let repository = tempfile::tempdir().expect("temporary repository");
    let input = tempfile::tempdir().expect("temporary input");
    let plan = input.path().join("plan.json");
    fs::write(
        &plan,
        r#"{
  "schema_version": "modernlink.migration-plan/v1alpha1",
  "target_version": 21,
  "tasks": [
    {
      "id": "task:isolate",
      "state": "INFERENCE",
      "kind": "seam-isolation",
      "summary": "isolate",
      "depends_on": [],
      "evidence_ids": ["evidence:gateway"],
      "approval_required": false
    },
    {
      "id": "task:migrate",
      "state": "HYPOTHESIS",
      "kind": "seam-migration",
      "summary": "migrate",
      "depends_on": ["task:isolate"],
      "evidence_ids": ["evidence:gateway"],
      "approval_required": true
    }
  ]
}"#,
    )
    .expect("write migration plan");
    let binary = env!("CARGO_BIN_EXE_modernlink");
    let setup = Command::new(binary)
        .arg("setup")
        .arg(repository.path())
        .args(["--tools", "none"])
        .output()
        .expect("setup workspace");
    assert!(setup.status.success());

    let create = Command::new(binary)
        .args(["migration", "create", "--repository"])
        .arg(repository.path())
        .args(["--id", "MIG-100", "--plan"])
        .arg(&plan)
        .output()
        .expect("create migration record");
    assert!(
        create.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&create.stderr)
    );
    let record_root = repository.path().join("modernlink/migrations/MIG-100");
    let record: serde_json::Value = serde_json::from_slice(
        &fs::read(record_root.join("status.json")).expect("migration record"),
    )
    .expect("record JSON");
    assert_eq!(record["status"], "PLANNED");
    assert_eq!(record["target_version"], 21);
    assert_eq!(
        record["lifecycle_journal"],
        ".modernlink/state/migrations/MIG-100.jsonl"
    );
    assert_eq!(
        record["approval_required_task_ids"],
        serde_json::json!(["task:migrate"])
    );
    let saved_plan: serde_json::Value =
        serde_json::from_slice(&fs::read(record_root.join("plan.json")).expect("saved plan"))
            .expect("plan JSON");
    assert_eq!(saved_plan["tasks"].as_array().unwrap().len(), 2);

    let duplicate = Command::new(binary)
        .args(["migration", "create", "--repository"])
        .arg(repository.path())
        .args(["--id", "MIG-100", "--plan"])
        .arg(&plan)
        .output()
        .expect("refuse duplicate migration record");
    assert!(!duplicate.status.success());
    assert!(String::from_utf8_lossy(&duplicate.stderr).contains("refusing to overwrite"));
}

#[test]
fn migration_create_refuses_a_plan_with_a_missing_prerequisite() {
    let repository = tempfile::tempdir().expect("temporary repository");
    let input = tempfile::tempdir().expect("temporary input");
    let plan = input.path().join("invalid-plan.json");
    fs::write(
        &plan,
        r#"{
  "schema_version": "modernlink.migration-plan/v1alpha1",
  "target_version": 21,
  "tasks": [{
    "id": "task:migrate",
    "state": "HYPOTHESIS",
    "kind": "seam-migration",
    "summary": "migrate",
    "depends_on": ["missing"],
    "evidence_ids": [],
    "approval_required": true
  }]
}"#,
    )
    .expect("write invalid migration plan");
    let binary = env!("CARGO_BIN_EXE_modernlink");
    let setup = Command::new(binary)
        .arg("setup")
        .arg(repository.path())
        .args(["--tools", "none"])
        .output()
        .expect("setup workspace");
    assert!(setup.status.success());

    let create = Command::new(binary)
        .args(["migrate", "create", "--repository"])
        .arg(repository.path())
        .args(["--id", "MIG-101", "--plan"])
        .arg(&plan)
        .output()
        .expect("refuse invalid migration record");
    assert!(!create.status.success());
    assert!(String::from_utf8_lossy(&create.stderr).contains("invalid plan"));
    assert!(
        !repository
            .path()
            .join("modernlink/migrations/MIG-101")
            .exists()
    );
}
