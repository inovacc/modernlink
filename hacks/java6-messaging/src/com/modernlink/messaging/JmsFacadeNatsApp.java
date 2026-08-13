package com.modernlink.messaging;

import java.lang.management.ManagementFactory;
import javax.management.MBeanServer;
import javax.management.ObjectName;
import javax.management.StandardMBean;

/** JMS-shaped Java 6 fixture using the shared JMX contract and native NATS. */
public final class JmsFacadeNatsApp {
    private JmsFacadeNatsApp() { }

    public static void main(String[] args) throws Exception {
        String url = args.length > 0 ? args[0] : "nats://127.0.0.1:4222";
        String subject = args.length > 1 ? args[1] : "modernlink.java6.jms.facade";
        ModernConnectionFactory factory = new ModernConnectionFactory(url, subject,
            ModernMessagingMode.REDIRECT, ModernMessagingProvider.NATS);
        ModernConnection connection = factory.createConnection();
        try {
            MBeanServer server = ManagementFactory.getPlatformMBeanServer();
            server.registerMBean(new StandardMBean(connection.getMetrics(), ModernMessagingMetricsMBean.class),
                new ObjectName("com.modernlink.messaging:type=Metrics,role=JmsFacade"));
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
            System.out.println("provider=" + factoryProvider(connection) + " mode=REDIRECT"
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
