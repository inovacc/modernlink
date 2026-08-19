package com.modernlink.messaging;

import com.modernlink.ModernBase64;
import java.io.UnsupportedEncodingException;
import java.util.Iterator;
import java.util.Map;
import java.util.TreeMap;

/**
 * A message body and its category (MSG-05).
 *
 * Before this, the Java 6 boundary carried text only: the native side rejected every
 * other category with "text payloads only". A JMS application that sends a BytesMessage
 * or a MapMessage had nowhere to go.
 *
 * Immutable, and byte arrays are copied in and out. A shared array would let a caller
 * mutate a payload after publishing it, which is unobservable at the call site and
 * produces a message that does not match what the application believes it sent.
 *
 * Java 6 syntax only.
 */
public final class ModernPayload {
    private final ModernPayloadKind kind;
    private final byte[] bytes;

    private ModernPayload(ModernPayloadKind kind, byte[] bytes) {
        this.kind = kind;
        this.bytes = bytes;
    }

    public static ModernPayload text(String value) {
        if (value == null) {
            throw new IllegalArgumentException("text payload is required");
        }
        return new ModernPayload(ModernPayloadKind.TEXT, utf8(value));
    }

    public static ModernPayload bytes(byte[] value) {
        if (value == null) {
            throw new IllegalArgumentException("bytes payload is required");
        }
        byte[] copy = new byte[value.length];
        System.arraycopy(value, 0, copy, 0, value.length);
        return new ModernPayload(ModernPayloadKind.BYTES, copy);
    }

    /**
     * A string-to-string map body.
     *
     * Encoded as {@code base64(key):base64(value)} pairs joined by commas. Both halves
     * are base64 so a key or value containing a delimiter cannot forge a pair boundary.
     * The separators are {@code :} and {@code ,} because neither appears in the base64
     * alphabet -- {@code =} would have been the obvious choice and is wrong, because it
     * is base64 padding.
     *
     * A TreeMap is used so the encoding is deterministic: two equal maps encode
     * identically, which keeps the frame comparable in tests and logs.
     */
    public static ModernPayload map(Map entries) {
        if (entries == null) {
            throw new IllegalArgumentException("map payload is required");
        }
        TreeMap ordered = new TreeMap(entries);
        StringBuilder builder = new StringBuilder();
        Iterator iterator = ordered.entrySet().iterator();
        boolean first = true;
        while (iterator.hasNext()) {
            Map.Entry entry = (Map.Entry) iterator.next();
            if (entry.getKey() == null || entry.getValue() == null) {
                throw new IllegalArgumentException("map payload keys and values must be non-null");
            }
            if (!first) {
                builder.append(",");
            }
            first = false;
            builder.append(ModernBase64.encode(utf8(entry.getKey().toString())));
            builder.append(":");
            builder.append(ModernBase64.encode(utf8(entry.getValue().toString())));
        }
        return new ModernPayload(ModernPayloadKind.MAP, utf8(builder.toString()));
    }

    public ModernPayloadKind getKind() {
        return kind;
    }

    /** The text body. Refuses rather than guessing when this is not a TEXT payload. */
    public String asText() {
        requireKind(ModernPayloadKind.TEXT);
        return fromUtf8(bytes);
    }

    /** A copy of the raw body, valid for any category. */
    public byte[] asBytes() {
        byte[] copy = new byte[bytes.length];
        System.arraycopy(bytes, 0, copy, 0, bytes.length);
        return copy;
    }

    /** The map body. Refuses rather than guessing when this is not a MAP payload. */
    public Map asMap() {
        requireKind(ModernPayloadKind.MAP);
        TreeMap entries = new TreeMap();
        String encoded = fromUtf8(bytes);
        if (encoded.length() == 0) {
            return entries;
        }
        String[] pairs = encoded.split(",", -1);
        for (int index = 0; index < pairs.length; index++) {
            int separator = pairs[index].indexOf(':');
            if (separator < 0) {
                throw new IllegalArgumentException("map payload entry has no separator");
            }
            String key = fromUtf8(ModernBase64.decode(pairs[index].substring(0, separator)));
            String value = fromUtf8(ModernBase64.decode(pairs[index].substring(separator + 1)));
            entries.put(key, value);
        }
        return entries;
    }

    /** The base64 form carried in the native frame. */
    String encodeBody() {
        return ModernBase64.encode(bytes);
    }

    static ModernPayload decode(ModernPayloadKind kind, byte[] body) {
        if (kind == null || body == null) {
            throw new IllegalArgumentException("payload category and body are required");
        }
        if (kind == ModernPayloadKind.STREAM || kind == ModernPayloadKind.OBJECT) {
            throw new IllegalArgumentException(refusalReason(kind));
        }
        return new ModernPayload(kind, body);
    }

    /**
     * Why STREAM and OBJECT are refused rather than delivered as opaque bytes.
     *
     * OBJECT is the important one: reconstructing an ObjectMessage means deserializing
     * broker-supplied bytes into Java objects, which is a well-known remote-code-execution
     * surface. A compatibility layer fronting a locked-down legacy application must not
     * open that by default. Use BYTES and deserialize explicitly if the application
     * accepts the risk.
     */
    static String refusalReason(ModernPayloadKind kind) {
        if (kind == ModernPayloadKind.OBJECT) {
            return "OBJECT payloads are deliberately not carried across the Java 6 boundary: "
                + "reconstructing one means deserializing broker-supplied bytes into Java "
                + "objects, which is a remote-code-execution surface. Use BYTES and "
                + "deserialize explicitly if the application accepts that risk.";
        }
        return "STREAM payloads are not carried across the Java 6 boundary: the frame does "
            + "not encode the typed field ordering a StreamMessage requires, and delivering "
            + "it as opaque bytes would lose that structure silently.";
    }

    private void requireKind(ModernPayloadKind expected) {
        if (kind != expected) {
            throw new IllegalStateException("payload category is " + kind.name()
                + ", not " + expected.name());
        }
    }

    private static byte[] utf8(String value) {
        try {
            return value.getBytes("UTF-8");
        } catch (UnsupportedEncodingException impossible) {
            throw new IllegalStateException("UTF-8 is unavailable", impossible);
        }
    }

    private static String fromUtf8(byte[] value) {
        try {
            return new String(value, "UTF-8");
        } catch (UnsupportedEncodingException impossible) {
            throw new IllegalStateException("UTF-8 is unavailable", impossible);
        }
    }
}
