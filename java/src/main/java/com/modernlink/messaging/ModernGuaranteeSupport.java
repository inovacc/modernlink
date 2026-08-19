package com.modernlink.messaging;

/**
 * How well one messaging guarantee is backed for a given provider (MSG-04).
 *
 * Three levels rather than a boolean, because "the transport implements it" and
 * "a test proved it" are different statements. A caller deciding whether to move
 * production traffic needs to tell them apart, so {@link #isProven()} is true only
 * for {@link #VERIFIED}.
 */
public enum ModernGuaranteeSupport {
    /** Implemented and exercised by a test against a real broker. */
    VERIFIED,
    /** Implemented, but no test has ever exercised it. Treat as a claim. */
    DECLARED,
    /** Not offered. Asking for it is refused, never quietly downgraded. */
    UNSUPPORTED;

    /**
     * True only for {@link #VERIFIED}.
     *
     * Deliberately false for {@link #DECLARED}: handing a caller a claim dressed as
     * evidence is the failure this whole table exists to prevent.
     */
    public boolean isProven() {
        return this == VERIFIED;
    }

    static ModernGuaranteeSupport decode(String value) {
        if (value == null) {
            throw new IllegalArgumentException("guarantee support value is required");
        }
        // No switch on String: that is Java 7, and java/src compiles at -source 1.6.
        if (value.equals("VERIFIED")) {
            return VERIFIED;
        }
        if (value.equals("DECLARED")) {
            return DECLARED;
        }
        if (value.equals("UNSUPPORTED")) {
            return UNSUPPORTED;
        }
        throw new IllegalArgumentException("unknown guarantee support value: " + value);
    }
}
