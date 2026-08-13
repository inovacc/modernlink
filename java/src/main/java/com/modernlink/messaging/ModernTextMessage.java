package com.modernlink.messaging;

import com.modernlink.LegacyHttpException;
import com.modernlink.ModernUuid;

/** Text-message subset shared by the Java 6 JMS-shaped facade. */
public final class ModernTextMessage {
    private String messageId;
    private String destination;
    private String text;
    private ModernTraceContext tracing;
    private final ModernAcknowledgementMode acknowledgementMode;

    ModernTextMessage(String text, ModernAcknowledgementMode acknowledgementMode) {
        if (text == null || acknowledgementMode == null) throw new IllegalArgumentException("text and acknowledgement mode are required");
        this.messageId = null;
        this.destination = null;
        this.text = text;
        this.acknowledgementMode = acknowledgementMode;
    }

    ModernTextMessage(ModernMessage message) {
        if (message == null) throw new IllegalArgumentException("message is required");
        this.messageId = message.getMessageId();
        this.destination = message.getDestination();
        this.text = message.getPayload();
        this.tracing = message.getTracing();
        this.acknowledgementMode = message.getAcknowledgementMode();
    }

    public String getJMSMessageID() { return messageId; }
    public String getJMSDestination() { return destination; }
    public String getText() { return text; }
    public void setText(String text) { if (text == null) throw new IllegalArgumentException("text is required"); this.text = text; }
    public ModernTraceContext getTracing() throws LegacyHttpException {
        if (tracing == null) tracing = ModernTraceContext.create();
        return tracing;
    }

    ModernMessage toModernMessage(String destination) throws Exception {
        this.destination = destination;
        if (messageId == null) messageId = ModernUuid.v7();
        return new ModernMessage(messageId, destination, text, getTracing(), acknowledgementMode);
    }
}
