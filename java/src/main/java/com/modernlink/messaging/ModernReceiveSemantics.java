package com.modernlink.messaging;

/**
 * What {@code receive()} does when no message is waiting (H-16).
 *
 * The Java and Rust signatures both suggest "there may be no message". That is true for two
 * of six providers. The other four block until one arrives, through a JNI call the calling
 * application cannot cancel.
 *
 * A legacy JMS application polling for work expects {@code receiveNoWait()} semantics: call,
 * get null, do something else. On a blocking provider that call never returns. Check this
 * before writing such a loop.
 */
public enum ModernReceiveSemantics {
    /** Returns promptly with no message when nothing is waiting. */
    NON_BLOCKING,
    /** Blocks until a message arrives. There is no timeout and no way to cancel. */
    BLOCKS_INDEFINITELY;

    /** True when a polling loop is safe against this provider. */
    public boolean isSafeForPolling() {
        return this == NON_BLOCKING;
    }

    static ModernReceiveSemantics decode(String value) {
        if (value == null) {
            throw new IllegalArgumentException("receive semantics value is required");
        }
        // No switch on String: that is Java 7, and java/src compiles at -source 1.6.
        if (value.equals("NON_BLOCKING")) {
            return NON_BLOCKING;
        }
        if (value.equals("BLOCKS_INDEFINITELY")) {
            return BLOCKS_INDEFINITELY;
        }
        throw new IllegalArgumentException("unknown receive semantics value: " + value);
    }
}
