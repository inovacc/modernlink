package com.modernlink.messaging;

/** Java 6-compatible standard-MBean implementation with no payload exposure. */
public final class ModernMessagingMetrics implements ModernMessagingMetricsMBean {
    private final String mode;
    private final String provider;
    private long published;
    private long received;
    private long acknowledged;
    private String lastTraceId;

    public ModernMessagingMetrics(ModernMessagingMode mode, ModernMessagingProvider provider) {
        if (mode == null || provider == null) throw new IllegalArgumentException("mode and provider are required");
        this.mode = mode.name();
        this.provider = provider.name();
    }

    public synchronized void published(String traceId) {
        published++;
        lastTraceId = traceId;
    }

    public synchronized void received(String traceId) {
        received++;
        lastTraceId = traceId;
    }

    public synchronized void acknowledged(String traceId) {
        acknowledged++;
        lastTraceId = traceId;
    }

    public String getMode() { return mode; }
    public String getProvider() { return provider; }
    public synchronized long getPublished() { return published; }
    public synchronized long getReceived() { return received; }
    public synchronized long getAcknowledged() { return acknowledged; }
    public synchronized String getLastTraceId() { return lastTraceId; }
}
