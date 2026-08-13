package com.modernlink.messaging;

/** Java 6 demo consumer for the same message wire fields. */
public final class ModernProviderDemo {
    private ModernProviderDemo() { }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) throw new IllegalArgumentException("provider is required");
        ModernMessagingProvider provider = ModernMessagingProvider.valueOf(args[0].toUpperCase());
        if (provider == ModernMessagingProvider.LEGACY_JMS) throw new IllegalArgumentException("modern provider required");
        String line = readLine();
        String[] frame = line.split("\\|", 3);
        if (frame.length != 3) throw new IllegalArgumentException("invalid messaging frame");
        ModernMessagingMode mode = ModernMessagingMode.valueOf(frame[0]);
        ModernMessagingProvider routed = ModernMessagingProvider.valueOf(frame[1]);
        if (routed != provider) throw new IllegalArgumentException("route provider does not match consumer");
        ModernMessage message = ModernMessage.decode(frame[2]);
        if (message.getTracing().getTraceId().length() != 32 || message.getTracing().getSpanId().length() != 16) {
            throw new IllegalArgumentException("invalid trace context");
        }
        System.out.println("provider=" + provider + " mode=" + mode + " destination=" + message.getDestination()
            + " message-id=" + message.getMessageId() + " trace-id=" + message.getTracing().getTraceId());
    }

    private static String readLine() throws Exception {
        java.io.BufferedReader reader = new java.io.BufferedReader(new java.io.InputStreamReader(System.in, "UTF-8"));
        String line = reader.readLine();
        if (line == null) throw new IllegalArgumentException("messaging frame is missing");
        return line;
    }
}
