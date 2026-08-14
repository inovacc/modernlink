package com.modernlink.messaging;

import com.modernlink.LegacyHttpException;

/**
 * MSG-06 / BUGS B-002 — the routing policy engine must be reachable from Java.
 *
 * Before this, crates/jni built every RouteConfig with an empty rule list, so rule
 * matching, allow/deny, dry-run evaluation and rule_id receipts existed only inside
 * Rust and no Java caller could configure or observe any of them. Every assertion
 * below fails against that code: with no rules, every decision returns the default
 * route with a null ruleId, so the ruleId and denial checks cannot pass.
 *
 * Uses LEGACY_JMS so it needs no broker. Note the policy invariant enforced natively:
 * TRANSPARENT requires LEGACY_JMS, and TRANSFORM/REDIRECT require a modern provider.
 *
 * Java 6 syntax only.
 */
public final class RoutingPolicyTest {
    private static final String URL = "legacy-jms://in-process";
    private static final String SUBJECT = "modernlink.routing.queue";

    public static void main(String[] args) throws Exception {
        defaultRouteWhenNoRuleMatches();
        exactDestinationRuleWins();
        prefixAndTenantConstraints();
        deniedRouteIsReportedNotThrown();
        firstMatchWins();
        malformedPolicyIsRejected();
        System.out.println("routing-policy=PASS");
    }

    private static ModernRouteRule transparentRule(String id) {
        return new ModernRouteRule(id, ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.LEGACY_JMS);
    }

    private static ModernConnection open(ModernRouteRule[] rules) throws LegacyHttpException {
        return new ModernConnectionFactory(URL, SUBJECT, ModernMessagingMode.TRANSPARENT,
            ModernMessagingProvider.LEGACY_JMS, rules).createConnection();
    }

    /** With rules present but none matching, the default route applies and ruleId is null. */
    private static void defaultRouteWhenNoRuleMatches() throws Exception {
        ModernConnection connection = open(new ModernRouteRule[] {
            transparentRule("never-matches").destination("some.other.queue")
        });
        try {
            ModernRouteDecision decision = connection.evaluateRoute(SUBJECT);
            if (decision.getRuleId() != null) throw new AssertionError("unexpected rule matched: " + decision.getRuleId());
            if (!decision.isAllowed()) throw new AssertionError("default route should be allowed");
            if (decision.getMode() != ModernMessagingMode.TRANSPARENT) throw new AssertionError("wrong default mode");
            System.out.println("default-route=" + decision);
        } finally {
            connection.close();
        }
    }

    /** An exact-destination rule must match and be reported by id. */
    private static void exactDestinationRuleWins() throws Exception {
        ModernConnection connection = open(new ModernRouteRule[] {
            transparentRule("exact-hit").destination(SUBJECT)
        });
        try {
            ModernRouteDecision decision = connection.evaluateRoute(SUBJECT);
            if (!"exact-hit".equals(decision.getRuleId())) {
                throw new AssertionError("expected exact-hit, got " + decision.getRuleId());
            }
            System.out.println("exact-destination=" + decision);
        } finally {
            connection.close();
        }
    }

    /** Prefix and tenant constrain independently; a wrong tenant must not match. */
    private static void prefixAndTenantConstraints() throws Exception {
        ModernConnection connection = open(new ModernRouteRule[] {
            transparentRule("prefix-tenant").destinationPrefix("modernlink.").tenant("acme")
        });
        try {
            ModernRouteDecision matched = connection.evaluateRoute(SUBJECT, "acme", null, null);
            if (!"prefix-tenant".equals(matched.getRuleId())) {
                throw new AssertionError("prefix+tenant rule did not match: " + matched.getRuleId());
            }
            ModernRouteDecision wrongTenant = connection.evaluateRoute(SUBJECT, "other", null, null);
            if (wrongTenant.getRuleId() != null) {
                throw new AssertionError("rule matched despite a different tenant: " + wrongTenant.getRuleId());
            }
            ModernRouteDecision noTenant = connection.evaluateRoute(SUBJECT);
            if (noTenant.getRuleId() != null) {
                throw new AssertionError("tenant-constrained rule matched an untenanted message");
            }
            System.out.println("prefix-tenant-matched=" + matched.getRuleId());
            System.out.println("prefix-tenant-wrong-tenant-ruleid=" + wrongTenant.getRuleId());
        } finally {
            connection.close();
        }
    }

    /** A denied route is a decision the caller can explain, not an exception. */
    private static void deniedRouteIsReportedNotThrown() throws Exception {
        ModernConnection connection = open(new ModernRouteRule[] {
            transparentRule("blocked").destination(SUBJECT).allowed(false)
        });
        try {
            ModernRouteDecision decision = connection.evaluateRoute(SUBJECT);
            if (decision.isAllowed()) throw new AssertionError("denied rule reported the route as allowed");
            if (!"blocked".equals(decision.getRuleId())) {
                throw new AssertionError("denial did not name the rule: " + decision.getRuleId());
            }
            System.out.println("denied-route=" + decision);
        } finally {
            connection.close();
        }
    }

    /** Rules are evaluated in order; the first match wins even if a later one also matches. */
    private static void firstMatchWins() throws Exception {
        ModernConnection connection = open(new ModernRouteRule[] {
            transparentRule("first").destinationPrefix("modernlink."),
            transparentRule("second").destination(SUBJECT)
        });
        try {
            ModernRouteDecision decision = connection.evaluateRoute(SUBJECT);
            if (!"first".equals(decision.getRuleId())) {
                throw new AssertionError("expected first-match-wins, got " + decision.getRuleId());
            }
            System.out.println("first-match-wins=" + decision.getRuleId());
        } finally {
            connection.close();
        }
    }

    /**
     * A policy the engine cannot honour must be rejected at open, not silently dropped.
     * TRANSPARENT mode requires LEGACY_JMS, so a TRANSPARENT/KAFKA rule is invalid.
     */
    private static void malformedPolicyIsRejected() throws Exception {
        // A rule is only evaluated when it matches, so constrain it to the subject the
        // native side probes at open time.
        ModernRouteRule invalid = new ModernRouteRule("bad-combo",
            ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.KAFKA).destination(SUBJECT);
        ModernConnection connection = null;
        try {
            connection = open(new ModernRouteRule[] {invalid});
            throw new AssertionError("an inconsistent TRANSPARENT/KAFKA rule was accepted");
        } catch (LegacyHttpException expected) {
            System.out.println("rejected-invalid-policy=" + expected.getMessage());
        } finally {
            if (connection != null) connection.close();
        }

        try {
            new ModernRouteRule("", ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.LEGACY_JMS);
            throw new AssertionError("an empty rule id was accepted");
        } catch (IllegalArgumentException expected) {
            System.out.println("rejected-empty-rule-id=ok");
        }

        try {
            transparentRule("half-header").header("only-name", null);
            throw new AssertionError("a header name without a value was accepted");
        } catch (IllegalArgumentException expected) {
            System.out.println("rejected-half-header=ok");
        }
    }
}
