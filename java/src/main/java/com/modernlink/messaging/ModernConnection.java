package com.modernlink.messaging;

import com.modernlink.LegacyHttpException;
import java.lang.management.ManagementFactory;
import java.util.Vector;
import javax.management.JMException;
import javax.management.MBeanServer;
import javax.management.ObjectName;
import javax.management.StandardMBean;

/** Java 6-compatible connection lifecycle and listener start boundary. */
public final class ModernConnection {
    private final ModernMessagingClient client;
    private final ModernMessagingMode mode;
    private final ModernMessagingProvider provider;
    private final ModernMessagingMetrics metrics;
    private final MBeanServer mBeanServer;
    private final ObjectName metricsObjectName;
    private final Vector sessions = new Vector();
    private boolean started;
    private boolean closed;
    private static long nextConnectionId;

    ModernConnection(ModernMessagingClient client, ModernMessagingMode mode, ModernMessagingProvider provider) {
        this.client = client;
        this.mode = mode;
        this.provider = provider;
        this.metrics = new ModernMessagingMetrics(mode, provider);
        this.mBeanServer = ManagementFactory.getPlatformMBeanServer();
        this.metricsObjectName = registerMetrics();
    }

    private ObjectName registerMetrics() {
        try {
            ObjectName name = new ObjectName("com.modernlink.messaging:type=Metrics,mode="
                + mode.name() + ",provider=" + provider.name() + ",connection=" + connectionId());
            mBeanServer.registerMBean(new StandardMBean(metrics, ModernMessagingMetricsMBean.class), name);
            return name;
        } catch (JMException error) {
            throw new IllegalStateException("unable to register messaging metrics MBean", error);
        }
    }

    private static synchronized long connectionId() {
        return ++nextConnectionId;
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
            try {
                mBeanServer.unregisterMBean(metricsObjectName);
            } catch (JMException ignored) {
                // Management registration must not prevent native transport shutdown.
            }
            client.close();
        }
    }

    /**
     * Evaluate the routing policy for a hypothetical message without publishing it.
     * A denied route comes back as a decision, so callers can report which rule denied it.
     */
    public synchronized ModernRouteDecision evaluateRoute(String destination, String tenant,
        String headerName, String headerValue) throws LegacyHttpException {
        requireOpen();
        return client.dryRun(destination, tenant, headerName, headerValue);
    }

    public synchronized ModernRouteDecision evaluateRoute(String destination) throws LegacyHttpException {
        return evaluateRoute(destination, null, null, null);
    }

    public synchronized boolean isStarted() { return started; }
    public ModernMessagingMetrics getMetrics() { return metrics; }
    public ObjectName getMetricsObjectName() { return metricsObjectName; }

    void requireOpen() throws LegacyHttpException {
        if (closed) throw new LegacyHttpException("messaging connection is closed");
    }

    void recordPublished(String traceId) { metrics.published(traceId); }
    void recordReceived(String traceId) { metrics.received(traceId); }
    void recordAcknowledged(String traceId) { metrics.acknowledged(traceId); }
}
