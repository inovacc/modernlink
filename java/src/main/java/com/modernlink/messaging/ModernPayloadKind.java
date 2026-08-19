package com.modernlink.messaging;

/**
 * Payload categories carried across the JNI boundary (MSG-05).
 *
 * The category travels with the bytes because base64 alone is ambiguous: the receiver
 * cannot tell a UTF-8 string from an opaque blob, and guessing would make a
 * BytesMessage arrive silently as text.
 *
 * {@link #STREAM} and {@link #OBJECT} are declared so the enum matches the native
 * domain, but publishing either is refused. See {@link ModernPayload} for why.
 */
public enum ModernPayloadKind {
    TEXT,
    BYTES,
    MAP,
    /** Declared, not carried: the frame does not encode typed field ordering. */
    STREAM,
    /** Declared, not carried: reconstructing one means Java deserialization. */
    OBJECT;

    static ModernPayloadKind decode(String value) {
        if (value == null) {
            throw new IllegalArgumentException("payload category is required");
        }
        // No switch on String: that is Java 7, and java/src compiles at -source 1.6.
        if (value.equals("TEXT")) {
            return TEXT;
        }
        if (value.equals("BYTES")) {
            return BYTES;
        }
        if (value.equals("MAP")) {
            return MAP;
        }
        if (value.equals("STREAM")) {
            return STREAM;
        }
        if (value.equals("OBJECT")) {
            return OBJECT;
        }
        throw new IllegalArgumentException("unknown payload category: " + value);
    }
}
