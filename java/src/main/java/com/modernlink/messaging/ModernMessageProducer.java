package com.modernlink.messaging;

import com.modernlink.LegacyHttpException;

/** Synchronous JMS-shaped producer backed by the native messaging client. */
public final class ModernMessageProducer {
    private final ModernSession session;
    private final ModernMessagingClient client;
    private final String destination;
    private final ModernAcknowledgementMode acknowledgementMode;
    private boolean closed;

    ModernMessageProducer(ModernSession session, ModernMessagingClient client, String destination,
        ModernAcknowledgementMode acknowledgementMode) {
        if (destination == null || destination.length() == 0) throw new IllegalArgumentException("destination is required");
        this.session = session;
        this.client = client;
        this.destination = destination;
        this.acknowledgementMode = acknowledgementMode;
    }

    public ModernDeliveryReceipt send(ModernTextMessage message) throws Exception {
        if (closed) throw new LegacyHttpException("message producer is closed");
        if (message == null) throw new IllegalArgumentException("message is required");
        ModernDeliveryReceipt receipt = client.publish(message.toModernMessage(destination));
        session.connection().recordPublished(receipt.getTraceId());
        return receipt;
    }

    public ModernAcknowledgementMode getAcknowledgementMode() { return acknowledgementMode; }
    public String getDestination() { return destination; }
    public void close() { closed = true; }
}
