package com.modernlink.messaging;

import com.modernlink.LegacyHttpException;

/** Synchronous and listener-based JMS-shaped consumer backed by native receive. */
public final class ModernMessageConsumer {
    private final ModernSession session;
    private final ModernMessagingClient client;
    private final String destination;
    private final ModernAcknowledgementMode acknowledgementMode;
    private ModernMessageListener listener;
    private boolean closed;
    private boolean listenerStarted;

    ModernMessageConsumer(ModernSession session, ModernMessagingClient client, String destination,
        ModernAcknowledgementMode acknowledgementMode) {
        if (destination == null || destination.length() == 0) throw new IllegalArgumentException("destination is required");
        this.session = session;
        this.client = client;
        this.destination = destination;
        this.acknowledgementMode = acknowledgementMode;
    }

    public synchronized ModernReceivedMessage receive() throws Exception {
        requireOpen();
        ModernReceivedMessage received = client.receive();
        if (!destination.equals(received.getMessage().getDestination())) {
            throw new LegacyHttpException("received destination does not match consumer");
        }
        session.connection().recordReceived(received.getReceipt().getTraceId());
        if (acknowledgementMode == ModernAcknowledgementMode.AUTO) acknowledge(received.getReceipt());
        return received;
    }

    public synchronized void setMessageListener(ModernMessageListener listener) throws Exception {
        requireOpen();
        this.listener = listener;
        if (session.connection().isStarted()) startListener();
    }

    synchronized void startListener() throws Exception {
        if (listenerStarted || listener == null || closed) return;
        listenerStarted = true;
        final ModernMessageListener callback = listener;
        Thread thread = new Thread(new Runnable() {
            public void run() {
                try {
                    ModernReceivedMessage received = receive();
                    ModernTextMessage message = new ModernTextMessage(received.getMessage());
                    callback.onMessage(message, received.getReceipt());
                } catch (Exception ignored) {
                    // Listener failures are isolated from the legacy application thread.
                }
            }
        }, "modernlink-messaging-listener");
        thread.setDaemon(true);
        thread.start();
    }

    public synchronized ModernDeliveryReceipt acknowledge(ModernDeliveryReceipt receipt) throws Exception {
        requireOpen();
        ModernDeliveryReceipt acknowledged = client.acknowledge(receipt);
        session.connection().recordAcknowledged(acknowledged.getTraceId());
        return acknowledged;
    }

    public String getDestination() { return destination; }
    public ModernAcknowledgementMode getAcknowledgementMode() { return acknowledgementMode; }
    public synchronized void close() { closed = true; }
    private void requireOpen() throws LegacyHttpException { if (closed) throw new LegacyHttpException("message consumer is closed"); }
}
