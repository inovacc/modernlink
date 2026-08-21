package com.modernlink.messaging;

import java.lang.management.ManagementFactory;
import javax.management.MBeanServer;
import javax.management.ObjectName;

/**
 * VER-06 — first Java-side messaging test.
 *
 * Until this existed, all ten Java test classes covered HTTP and utilities only, so the
 * entire JMS-shaped messaging facade had zero Java coverage. This drives the full
 * ConnectionFactory -> Connection -> Session -> Producer/Consumer path across the JNI
 * boundary using the in-process LEGACY_JMS transport, so it needs no broker and can run
 * anywhere the native library loads.
 *
 * Scope is deliberately honest: LEGACY_JMS is backed by an in-process transport
 * (ISSUES I-009), so this proves the FACADE and the JNI boundary carry messages,
 * receipts, trace context and acknowledgements correctly. It proves nothing about
 * broker-backed durability, ordering, reconnect or failure semantics (ISSUES I-010) —
 * that is VER-01/VER-02.
 *
 * Java 6 syntax only: no lambdas, no diamond, no try-with-resources.
 */
public final class LegacyJmsMessagingTest {
    private static final String URL = "legacy-jms://in-process";
    private static final String DESTINATION = "modernlink.ver06.queue";

    public static void main(String[] args) throws Exception {
        roundTrip();
        clientAcknowledgement();
        jmxMetrics();
        listenerDelivery();
        System.out.println("legacy-jms-messaging=PASS");
    }

    /** AUTO acknowledgement: send one message, receive it, confirm identity and tracing. */
    private static void roundTrip() throws Exception {
        ModernConnectionFactory factory = new ModernConnectionFactory(
            URL, DESTINATION, ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.LEGACY_JMS);
        ModernConnection connection = factory.createConnection();
        try {
            ModernSession session = connection.createSession(ModernAcknowledgementMode.AUTO);
            ModernMessageProducer producer = session.createProducer(DESTINATION);
            ModernMessageConsumer consumer = session.createConsumer(DESTINATION);

            ModernTextMessage outbound = session.createTextMessage("hello from Java 6");
            String traceId = outbound.getTracing().getTraceId();
            if (traceId == null || traceId.length() == 0) throw new AssertionError("no trace id was minted");

            ModernDeliveryReceipt sent = producer.send(outbound);
            if (sent.getState() != ModernDeliveryState.PUBLISHED) {
                throw new AssertionError("unexpected publish state: " + sent.getState());
            }
            if (sent.getProvider() != ModernMessagingProvider.LEGACY_JMS) {
                throw new AssertionError("receipt provider was rewritten: " + sent.getProvider());
            }
            if (!traceId.equals(sent.getTraceId())) {
                throw new AssertionError("trace id was not preserved through publish");
            }

            ModernReceivedMessage received = consumer.receive();
            if (!"hello from Java 6".equals(received.getMessage().getPayload())) {
                throw new AssertionError("payload was altered: " + received.getMessage().getPayload());
            }
            if (!DESTINATION.equals(received.getMessage().getDestination())) {
                throw new AssertionError("destination was altered: " + received.getMessage().getDestination());
            }
            if (!sent.getMessageId().equals(received.getReceipt().getMessageId())) {
                throw new AssertionError("message id was not preserved");
            }
            if (!traceId.equals(received.getMessage().getTracing().getTraceId())) {
                throw new AssertionError("trace context was not preserved across the boundary");
            }
            System.out.println("roundtrip-trace-id=" + traceId);
            System.out.println("roundtrip-message-id=" + sent.getMessageId());
        } finally {
            connection.close();
        }
    }

    /** CLIENT acknowledgement: receive must NOT auto-acknowledge; an explicit ack must. */
    private static void clientAcknowledgement() throws Exception {
        ModernConnectionFactory factory = new ModernConnectionFactory(
            URL, DESTINATION, ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.LEGACY_JMS);
        ModernConnection connection = factory.createConnection();
        try {
            ModernSession session = connection.createSession(ModernAcknowledgementMode.CLIENT);
            ModernMessageProducer producer = session.createProducer(DESTINATION);
            ModernMessageConsumer consumer = session.createConsumer(DESTINATION);

            producer.send(session.createTextMessage("needs an explicit ack"));
            ModernReceivedMessage received = consumer.receive();

            if (connection.getMetrics().getAcknowledged() != 0L) {
                throw new AssertionError("CLIENT mode acknowledged without an explicit call");
            }
            ModernDeliveryReceipt acknowledged = consumer.acknowledge(received.getReceipt());
            if (acknowledged.getState() != ModernDeliveryState.ACKNOWLEDGED) {
                throw new AssertionError("unexpected acknowledgement state: " + acknowledged.getState());
            }
            if (connection.getMetrics().getAcknowledged() != 1L) {
                throw new AssertionError("acknowledgement was not counted");
            }
            System.out.println("client-ack-state=" + acknowledged.getState());
        } finally {
            connection.close();
        }
    }

    /** The metrics MBean must be really registered, readable, and must leak nothing. */
    private static void jmxMetrics() throws Exception {
        ModernConnectionFactory factory = new ModernConnectionFactory(
            URL, DESTINATION, ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.LEGACY_JMS);
        ModernConnection connection = factory.createConnection();
        ObjectName name = connection.getMetricsObjectName();
        MBeanServer server = ManagementFactory.getPlatformMBeanServer();
        try {
            if (!server.isRegistered(name)) throw new AssertionError("metrics MBean is not registered: " + name);

            ModernSession session = connection.createSession(ModernAcknowledgementMode.AUTO);
            ModernMessageProducer producer = session.createProducer(DESTINATION);
            ModernMessageConsumer consumer = session.createConsumer(DESTINATION);
            producer.send(session.createTextMessage("secret-payload-must-not-appear"));
            consumer.receive();

            long published = ((Long) server.getAttribute(name, "Published")).longValue();
            long receivedCount = ((Long) server.getAttribute(name, "Received")).longValue();
            String mode = (String) server.getAttribute(name, "Mode");
            String provider = (String) server.getAttribute(name, "Provider");
            String lastTraceId = (String) server.getAttribute(name, "LastTraceId");

            if (published != 1L) throw new AssertionError("published counter wrong: " + published);
            if (receivedCount != 1L) throw new AssertionError("received counter wrong: " + receivedCount);
            if (!"TRANSPARENT".equals(mode)) throw new AssertionError("mode attribute wrong: " + mode);
            if (!"LEGACY_JMS".equals(provider)) throw new AssertionError("provider attribute wrong: " + provider);
            if (lastTraceId == null || lastTraceId.length() == 0) throw new AssertionError("no trace id recorded");

            // Standing rule: no credential, endpoint or payload may reach a JMX attribute.
            String[] attributes = new String[] {mode, provider, lastTraceId};
            for (int i = 0; i < attributes.length; i++) {
                if (attributes[i].indexOf("secret-payload-must-not-appear") >= 0) {
                    throw new AssertionError("a message payload leaked into a JMX attribute");
                }
                if (attributes[i].indexOf(URL) >= 0) {
                    throw new AssertionError("the broker endpoint leaked into a JMX attribute");
                }
            }
            System.out.println("jmx-object-name=" + name);
            System.out.println("jmx-published=" + published + " jmx-received=" + receivedCount);
        } finally {
            connection.close();
        }
        if (server.isRegistered(name)) throw new AssertionError("metrics MBean survived connection close: " + name);
        System.out.println("jmx-unregistered-on-close=ok");
    }

    /** A registered listener must receive asynchronously once the connection is started. */
    private static void listenerDelivery() throws Exception {
        ModernConnectionFactory factory = new ModernConnectionFactory(
            URL, DESTINATION, ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.LEGACY_JMS);
        ModernConnection connection = factory.createConnection();
        try {
            ModernSession session = connection.createSession(ModernAcknowledgementMode.AUTO);
            ModernMessageProducer producer = session.createProducer(DESTINATION);
            ModernMessageConsumer consumer = session.createConsumer(DESTINATION);

            final String[] delivered = new String[1];
            consumer.setMessageListener(new ModernMessageListener() {
                public void onMessage(ModernTextMessage message, ModernDeliveryReceipt receipt) {
                    synchronized (delivered) {
                        delivered[0] = message.getText();
                        delivered.notifyAll();
                    }
                }
            });
            connection.start();
            if (!connection.isStarted()) throw new AssertionError("connection did not report started");

            producer.send(session.createTextMessage("async delivery"));

            synchronized (delivered) {
                long deadline = System.currentTimeMillis() + 10000L;
                while (delivered[0] == null && System.currentTimeMillis() < deadline) {
                    delivered.wait(250L);
                }
            }
            if (!"async delivery".equals(delivered[0])) {
                throw new AssertionError("listener did not receive within 10s, got: " + delivered[0]);
            }
            System.out.println("listener-delivered=true");
        } finally {
            connection.close();
        }
    }
}
