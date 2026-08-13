package com.modernlink.messaging;

import com.modernlink.ModernUuid;

/** Java 6 process fixture for the native NATS messaging boundary. */
public final class NativeNatsApp {
    private NativeNatsApp() { }

    public static void main(String[] args) throws Exception {
        String url = args.length > 0 ? args[0] : "nats://127.0.0.1:4222";
        String subject = args.length > 1 ? args[1] : "modernlink.java6.orders";
        ModernMessagingClient client = new ModernMessagingClient(url, subject,
            ModernMessagingMode.REDIRECT, ModernMessagingProvider.NATS);
        try {
            ModernTraceContext tracing = ModernTraceContext.create();
            ModernMessage message = new ModernMessage(ModernUuid.v7(), subject,
                "order-from-java6-native", tracing, ModernAcknowledgementMode.CLIENT);
            ModernDeliveryReceipt published = client.publish(message);
            ModernReceivedMessage received = client.receive();
            ModernDeliveryReceipt acknowledged = client.acknowledge(received.getReceipt());
            if (!published.getMessageId().equals(received.getMessage().getMessageId())
                || !published.getMessageId().equals(acknowledged.getMessageId())
                || !published.getTraceId().equals(received.getMessage().getTracing().getTraceId())) {
                throw new IllegalStateException("native NATS identity or trace context changed");
            }
            System.out.println("provider=" + client.getProvider() + " mode=" + client.getMode()
                + " subject=" + subject + " message-id=" + received.getMessage().getMessageId()
                + " trace-id=" + received.getMessage().getTracing().getTraceId()
                + " published=" + published.getState() + " received=" + received.getReceipt().getState()
                + " acknowledged=" + acknowledged.getState());
        } finally {
            client.close();
        }
    }
}
