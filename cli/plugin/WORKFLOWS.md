# ModernLink Canonical Lifecycle

This bundle is harness-neutral. Adapters may translate names and syntax, but must preserve the
evidence and approval rules below.

```text
SETUP → DISCOVER → UNDERSTAND → MODEL → ASSESS → DESIGN → PLAN → PREPARE
      → MODERNIZE → MIGRATE → VERIFY → CUTOVER → DETACH → CLEANUP
```

The Rust CLI owns facts, report schemas, and lifecycle transitions. Agents may add an
`INFERENCE` or `HYPOTHESIS`, but must cite CLI evidence and cannot promote it to a fact.
`MODERNIZE`, `CUTOVER`, and `DETACH` require an explicit human approval recorded through
`modernlink lifecycle advance --approve`.

Canonical intentions are `analyze`, `architecture`, `domains`, `assess`, `plan`, `prepare`,
`modernize`, `verify`, `migrate`, and `status`. Before an adapter exposes an intention it must
have a harness-specific ownership contract; no adapter may overwrite user-authored instructions.
