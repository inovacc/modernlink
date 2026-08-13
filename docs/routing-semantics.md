# Routing semantics

`RouteConfig::dry_run` evaluates the first matching rule and returns its
`RouteDecision` without requiring a transport or publishing a message. This
includes denied decisions, so callers can explain why a route would be held.
`RouteConfig::dispatch` remains the applying operation: it rejects denied
decisions, requires the selected provider, adds mode/provider metadata for
transform and redirect, and returns a delivery receipt.

The behavior is covered by `dry_run_returns_denied_decision_without_publishing`
in `crates/messaging/src/lib.rs`. This is local routing behavior; it does not
prove broker delivery, JMS compatibility, JMX exposure, or provider semantics.
