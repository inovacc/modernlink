package com.modernlink.messaging;

import com.modernlink.LegacyHttpException;
import com.modernlink.ModernBase64;

/**
 * Java 6 mirror of the provider-neutral message fields used by the demo boundary.
 *
 * MSG-05 added the payload category. The wire frame gained a tenth field carrying it,
 * because base64 alone is ambiguous: a receiver cannot tell a UTF-8 string from an opaque
 * blob, and guessing would make a BytesMessage arrive silently as text.
 *
 * The String constructors are retained and mean TEXT, so every existing caller keeps
 * working unchanged.
 */
public final class ModernMessage {
    private final String messageId;
    private final String destination;
    private final ModernPayload payload;
    private final ModernTraceContext tracing;
    private final ModernAcknowledgementMode acknowledgementMode;

    public ModernMessage(String messageId, String destination, String payload, ModernTraceContext tracing) {
        this(messageId, destination, payload, tracing, ModernAcknowledgementMode.AUTO);
    }

    public ModernMessage(String messageId, String destination, String payload, ModernTraceContext tracing,
        ModernAcknowledgementMode acknowledgementMode) {
        this(messageId, destination, payload == null ? null : ModernPayload.text(payload), tracing,
            acknowledgementMode);
    }

    public ModernMessage(String messageId, String destination, ModernPayload payload, ModernTraceContext tracing,
        ModernAcknowledgementMode acknowledgementMode) {
        if (messageId == null || destination == null || payload == null || tracing == null) {
            throw new IllegalArgumentException("message fields are required");
        }
        if (acknowledgementMode == null) throw new IllegalArgumentException("acknowledgement mode is required");
        this.messageId = messageId;
        this.destination = destination;
        this.payload = payload;
        this.tracing = tracing;
        this.acknowledgementMode = acknowledgementMode;
    }

    public String getMessageId() { return messageId; }
    public String getDestination() { return destination; }

    /**
     * The text body.
     *
     * Throws when the payload is not TEXT rather than returning a lossy rendering of the
     * bytes, because a BytesMessage silently arriving as mojibake is worse than a failure.
     * Use {@link #getBody()} for category-agnostic access.
     */
    public String getPayload() { return payload.asText(); }

    /** The body and its category. */
    public ModernPayload getBody() { return payload; }

    public ModernTraceContext getTracing() { return tracing; }
    public ModernAcknowledgementMode getAcknowledgementMode() { return acknowledgementMode; }

    public String encode() throws LegacyHttpException {
        return join("|", messageId, destination, payload.encodeBody(),
            tracing.getTraceId(), tracing.getSpanId(), tracing.getParentSpanId() == null ? "" : tracing.getParentSpanId(),
            tracing.getTraceState() == null ? "" : tracing.getTraceState(), acknowledgementMode.name(),
            tracing.isSampled() ? "1" : "0", payload.getKind().name());
    }

    public static ModernMessage decode(String value) throws Exception {
        if (value == null) throw new IllegalArgumentException("message is required");
        String[] fields = value.split("\\|", -1);
        if (fields.length != 10) throw new IllegalArgumentException("invalid message field count");
        if (!"0".equals(fields[8]) && !"1".equals(fields[8])) throw new IllegalArgumentException("invalid trace sampling flag");
        ModernPayload payload = ModernPayload.decode(ModernPayloadKind.decode(fields[9]),
            ModernBase64.decode(fields[2]));
        ModernTraceContext tracing = new ModernTraceContextForDecode(fields[3], fields[4], fields[5].length() == 0 ? null : fields[5], fields[6].length() == 0 ? null : fields[6], "1".equals(fields[8])).value();
        return new ModernMessage(fields[0], fields[1], payload, tracing, ModernAcknowledgementMode.valueOf(fields[7]));
    }

    private static String join(String separator, String... fields) {
        StringBuffer result = new StringBuffer();
        for (int index = 0; index < fields.length; index++) {
            if (index != 0) result.append(separator);
            result.append(fields[index]);
        }
        return result.toString();
    }

    private static final class ModernTraceContextForDecode {
        private final ModernTraceContext value;

        private ModernTraceContextForDecode(String traceId, String spanId, String parentSpanId, String traceState, boolean sampled) {
            value = new ModernTraceContext(traceId, spanId, parentSpanId, traceState, sampled);
        }

        private ModernTraceContext value() { return value; }
    }
}
