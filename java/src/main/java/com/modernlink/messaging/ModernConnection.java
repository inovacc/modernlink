package com.modernlink.messaging;

import com.modernlink.LegacyHttpException;
import java.util.Vector;

/** Java 6-compatible connection lifecycle and listener start boundary. */
public final class ModernConnection {
    private final ModernMessagingClient client;
    private final ModernMessagingMode mode;
    private final ModernMessagingProvider provider;
    private final ModernMessagingMetrics metrics;
    private final Vector sessions = new Vector();
    private boolean started;
    private boolean closed;

    ModernConnection(ModernMessagingClient client, ModernMessagingMode mode, ModernMessagingProvider provider) {
        this.client = client;
        this.mode = mode;
        this.provider = provider;
        this.metrics = new ModernMessagingMetrics(mode, provider);
    }

    public synchronized ModernSession createSession(ModernAcknowledgementMode acknowledgementMode) throws LegacyHttpException {
        requireOpen();
        if (acknowledgementMode == null) throw new IllegalArgumentException("acknowledgement mode is required");
        ModernSession session = new ModernSession(this, client, acknowledgementMode);
        sessions.addElement(session);
        return session;
    }

    public synchronized void start() throws Exception {
        requireOpen();
        started = true;
        for (int index = 0; index < sessions.size(); index++) {
            ((ModernSession) sessions.elementAt(index)).startListeners();
        }
    }

    public synchronized void close() {
        if (!closed) {
            closed = true;
            client.close();
        }
    }

    public synchronized boolean isStarted() { return started; }
    public ModernMessagingMetrics getMetrics() { return metrics; }

    void requireOpen() throws LegacyHttpException {
        if (closed) throw new LegacyHttpException("messaging connection is closed");
    }

    void recordPublished(String traceId) { metrics.published(traceId); }
    void recordReceived(String traceId) { metrics.received(traceId); }
    void recordAcknowledged(String traceId) { metrics.acknowledged(traceId); }
}
