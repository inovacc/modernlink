package com.modernlink.messaging;

/** JMS-shaped Java 6 fixture using the shared JMX contract and native transport. */
public final class JmsFacadeNatsApp {
    private JmsFacadeNatsApp() { }

    public static void main(String[] args) throws Exception {
        String url = args.length > 0 ? args[0] : "nats://127.0.0.1:4222";
        String subject = args.length > 1 ? args[1] : "modernlink.java6.jms.facade";
        ModernMessagingProvider provider = args.length > 2
            ? ModernMessagingProvider.valueOf(args[2].toUpperCase())
            : ModernMessagingProvider.NATS;
        ModernMessagingMode mode = args.length > 3
            ? ModernMessagingMode.valueOf(args[3].toUpperCase())
            : ModernMessagingMode.REDIRECT;
        ModernConnectionFactory factory = new ModernConnectionFactory(url, subject,
            mode, provider);
        ModernConnection connection = factory.createConnection();
        try {
            ModernSession session = connection.createSession(ModernAcknowledgementMode.CLIENT);
            ModernMessageProducer producer = session.createProducer(subject);
            ModernMessageConsumer consumer = session.createConsumer(subject);
            ModernTextMessage message = session.createTextMessage("order-from-jms-facade");
            ModernDeliveryReceipt published = producer.send(message);
            ModernReceivedMessage received = consumer.receive();
            ModernDeliveryReceipt acknowledged = consumer.acknowledge(received.getReceipt());
            if (!published.getMessageId().equals(received.getMessage().getMessageId())
                || !published.getMessageId().equals(acknowledged.getMessageId())
                || !published.getTraceId().equals(received.getMessage().getTracing().getTraceId())) {
                throw new IllegalStateException("JMS-shaped facade changed message identity or tracing");
            }
            System.out.println("provider=" + factoryProvider(connection) + " mode=" + mode
                + " destination=" + subject + " message-id=" + received.getMessage().getMessageId()
                + " trace-id=" + received.getMessage().getTracing().getTraceId()
                + " published=" + published.getState() + " received=" + received.getReceipt().getState()
                + " acknowledged=" + acknowledged.getState()
                + " metrics-published=" + connection.getMetrics().getPublished()
                + " metrics-received=" + connection.getMetrics().getReceived()
                + " metrics-acknowledged=" + connection.getMetrics().getAcknowledged());
        } finally {
            connection.close();
        }
    }

    private static String factoryProvider(ModernConnection connection) {
        return connection.getMetrics().getProvider();
    }
}
