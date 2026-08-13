package com.modernlink.messaging;

import com.modernlink.ModernBase64;

/** Java 6 demo consumer for the same message wire fields. */
public final class ModernProviderDemo {
    private ModernProviderDemo() { }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) throw new IllegalArgumentException("provider is required");
        ModernMessagingProvider provider = ModernMessagingProvider.valueOf(args[0].toUpperCase());
        if (provider == ModernMessagingProvider.LEGACY_JMS) throw new IllegalArgumentException("modern provider required");
        String line = readLine();
        String[] frame = line.split("\\|", 4);
        if (frame.length != 4) throw new IllegalArgumentException("invalid messaging frame");
        ModernMessagingMode mode = ModernMessagingMode.valueOf(frame[0]);
        ModernMessagingProvider routed = ModernMessagingProvider.valueOf(frame[1]);
        if (routed != provider) throw new IllegalArgumentException("route provider does not match consumer");
        ModernMessage message = ModernMessage.decode(new String(ModernBase64.decode(frame[2]), "UTF-8"));
        ModernDeliveryReceipt receipt = ModernDeliveryReceipt.decode(frame[3]);
        if (receipt.getProvider() != provider || receipt.getState() != ModernDeliveryState.PUBLISHED
            || !receipt.getMessageId().equals(message.getMessageId()) || !receipt.getTraceId().equals(message.getTracing().getTraceId())) {
            throw new IllegalArgumentException("delivery receipt does not match message");
        }
        if (message.getTracing().getTraceId().length() != 32 || message.getTracing().getSpanId().length() != 16) {
            throw new IllegalArgumentException("invalid trace context");
        }
        System.out.println("provider=" + provider + " mode=" + mode + " destination=" + message.getDestination()
            + " message-id=" + message.getMessageId() + " trace-id=" + message.getTracing().getTraceId()
            + " acknowledgement=" + message.getAcknowledgementMode() + " receipt=" + receipt.getState());
    }

    private static String readLine() throws Exception {
        java.io.BufferedReader reader = new java.io.BufferedReader(new java.io.InputStreamReader(System.in, "UTF-8"));
        String line = reader.readLine();
        if (line == null) throw new IllegalArgumentException("messaging frame is missing");
        return line;
    }
}
