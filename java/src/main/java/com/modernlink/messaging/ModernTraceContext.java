package com.modernlink.messaging;

import com.modernlink.LegacyHttpException;
import com.modernlink.ModernUuid;

public final class ModernTraceContext {
    private final String traceId;
    private final String spanId;
    private final String parentSpanId;
    private final boolean sampled;

    ModernTraceContext(String traceId, String spanId, String parentSpanId, boolean sampled) {
        this.traceId = traceId;
        this.spanId = spanId;
        this.parentSpanId = parentSpanId;
        this.sampled = sampled;
    }

    public static ModernTraceContext create() throws LegacyHttpException {
        String trace = ModernUuid.v4().replace("-", "");
        String span = ModernUuid.v4().replace("-", "").substring(0, 16);
        return new ModernTraceContext(trace, span, null, true);
    }

    public String getTraceId() { return traceId; }
    public String getSpanId() { return spanId; }
    public String getParentSpanId() { return parentSpanId; }
    public boolean isSampled() { return sampled; }
}
