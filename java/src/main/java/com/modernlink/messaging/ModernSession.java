package com.modernlink.messaging;

import com.modernlink.LegacyHttpException;
import java.util.Vector;

/** Java 6-compatible session boundary for producer and consumer creation. */
public final class ModernSession {
    private final ModernConnection connection;
    private final ModernMessagingClient client;
    private final ModernAcknowledgementMode acknowledgementMode;
    private final Vector consumers = new Vector();
    private boolean closed;

    ModernSession(ModernConnection connection, ModernMessagingClient client,
        ModernAcknowledgementMode acknowledgementMode) {
        this.connection = connection;
        this.client = client;
        this.acknowledgementMode = acknowledgementMode;
    }

    public ModernMessageProducer createProducer(String destination) throws LegacyHttpException {
        requireOpen();
        return new ModernMessageProducer(this, client, destination, acknowledgementMode);
    }

    public synchronized ModernMessageConsumer createConsumer(String destination) throws LegacyHttpException {
        requireOpen();
        ModernMessageConsumer consumer = new ModernMessageConsumer(this, client, destination, acknowledgementMode);
        consumers.addElement(consumer);
        return consumer;
    }

    public ModernTextMessage createTextMessage(String text) {
        requireOpenUnchecked();
        return new ModernTextMessage(text, acknowledgementMode);
    }

    public synchronized void close() {
        closed = true;
        for (int index = 0; index < consumers.size(); index++) {
            ((ModernMessageConsumer) consumers.elementAt(index)).close();
        }
    }

    void startListeners() throws Exception {
        for (int index = 0; index < consumers.size(); index++) {
            ((ModernMessageConsumer) consumers.elementAt(index)).startListener();
        }
    }

    ModernConnection connection() { return connection; }

    private synchronized void requireOpen() throws LegacyHttpException {
        if (closed) throw new LegacyHttpException("messaging session is closed");
    }

    private synchronized void requireOpenUnchecked() {
        if (closed) throw new IllegalStateException("messaging session is closed");
    }
}
