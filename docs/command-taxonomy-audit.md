# ModernLink CLI Command Taxonomy Audit

Observed against `cli/crates/modernlink-cli/src/main.rs` and `modernlink --help` on 2026-08-23.

## Current command tree

```text
modernlink
├── setup
├── inspect
├── analyze
├── architecture
├── seams
├── compatibility
├── plan
├── history
├── evolution
├── doctor
├── status
├── lifecycle advance
├── harness list
├── plugin install|bind
└── runtime <subcommand>
```

## Findings

1. `lifecycle`, `harness`, `plugin`, and `runtime` form coherent noun groups. Analysis and
   planning commands remain root-level because they are primary lifecycle intentions.
2. `boundaries`, `readiness`, and `verify` appear in future design examples but are not executable
   commands. They are planned, not present.
3. `analyze` retains its legacy analysis artifact while `inspect` is the shared evidence-graph
   workflow. Keep their output distinction explicit until a compatibility decision merges them.
4. `history` collects raw structured facts; `evolution` derives a read-only x-ray from them. They
   are complementary, not duplicates.

## Standing rules

- Preserve lowercase kebab-case command names.
- Put a domain noun before a verb where a family exists (`lifecycle advance`, `plugin install`).
- Keep an analysis command at root only while it remains a primary lifecycle intention.
- Reuse canonical flags: `--output` for report creation, `--target` for Java major version,
  `--history` for inclusion of Git evidence.
- Any future rename or regroup requires an explicit migration decision and old-to-new mapping;
  this audit does not authorize a breaking taxonomy change.
