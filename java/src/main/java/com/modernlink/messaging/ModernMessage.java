package com.modernlink.messaging;

import com.modernlink.LegacyHttpException;
import com.modernlink.ModernBase64;

/** Java 6 mirror of the provider-neutral message fields used by the demo boundary. */
public final class ModernMessage {
    private final String messageId;
    private final String destination;
    private final String payload;
    private final ModernTraceContext tracing;
    private final ModernAcknowledgementMode acknowledgementMode;

    public ModernMessage(String messageId, String destination, String payload, ModernTraceContext tracing) {
        this(messageId, destination, payload, tracing, ModernAcknowledgementMode.AUTO);
    }

    public ModernMessage(String messageId, String destination, String payload, ModernTraceContext tracing,
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
    public String getPayload() { return payload; }
    public ModernTraceContext getTracing() { return tracing; }
    public ModernAcknowledgementMode getAcknowledgementMode() { return acknowledgementMode; }

    public String encode() throws LegacyHttpException {
        byte[] encodedPayload;
        try {
            encodedPayload = payload.getBytes("UTF-8");
        } catch (java.io.UnsupportedEncodingException impossible) {
            throw new IllegalStateException("UTF-8 is unavailable", impossible);
        }
        return join("|", messageId, destination, ModernBase64.encode(encodedPayload),
            tracing.getTraceId(), tracing.getSpanId(), tracing.getParentSpanId() == null ? "" : tracing.getParentSpanId(),
            tracing.getTraceState() == null ? "" : tracing.getTraceState(), acknowledgementMode.name());
    }

    public static ModernMessage decode(String value) throws Exception {
        if (value == null) throw new IllegalArgumentException("message is required");
        String[] fields = value.split("\\|", -1);
        if (fields.length != 8) throw new IllegalArgumentException("invalid message field count");
        String payload = new String(ModernBase64.decode(fields[2]), "UTF-8");
        ModernTraceContext tracing = new ModernTraceContextForDecode(fields[3], fields[4], fields[5].length() == 0 ? null : fields[5], fields[6].length() == 0 ? null : fields[6]).value();
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

        private ModernTraceContextForDecode(String traceId, String spanId, String parentSpanId, String traceState) {
            value = new ModernTraceContext(traceId, spanId, parentSpanId, traceState, true);
        }

        private ModernTraceContext value() { return value; }
    }
}
