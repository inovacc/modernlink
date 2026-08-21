package com.modernlink.messaging;

/**
 * One broker-backed round trip through the public Java 6 facade and JNI boundary.
 *
 * Usage:
 *   java ... BrokerBackedMessagingTest PROVIDER URL [destination-prefix]
 *
 * The caller owns broker lifecycle. A connection or delivery failure is an error; this
 * probe never skips merely because a broker is unavailable. Java 6 syntax only.
 */
public final class BrokerBackedMessagingTest {
    private BrokerBackedMessagingTest() { }

    public static void main(String[] args) throws Exception {
        if (args.length < 2 || args.length > 3) {
            throw new IllegalArgumentException(
                "usage: BrokerBackedMessagingTest PROVIDER URL [destination-prefix]");
        }

        ModernMessagingProvider provider = ModernMessagingProvider.valueOf(args[0]);
        String prefix = args.length == 3 ? args[2] : "modernlink_java_boundary";
        String destination = prefix + "_" + Long.toString(System.nanoTime());
        String messageId = "message-" + Long.toString(System.nanoTime());
        String payload = "java6-jni-" + provider.name();

        ModernMessagingClient client = new ModernMessagingClient(
            args[1], destination, ModernMessagingMode.REDIRECT, provider);
        try {
            ModernMessage sent = new ModernMessage(
                messageId,
                destination,
                payload,
                ModernTraceContext.create(),
                ModernAcknowledgementMode.CLIENT);

            ModernDeliveryReceipt published = client.publish(sent);
            require(published.getProvider() == provider, "publish provider changed");
            require(published.getState() == ModernDeliveryState.PUBLISHED,
                "publish did not return PUBLISHED");

            ModernReceivedMessage received = client.receive();
            require(messageId.equals(received.getMessage().getMessageId()),
                "received message id changed");
            require(destination.equals(received.getMessage().getDestination()),
                "received destination changed");
            require(payload.equals(received.getMessage().getBody().asText()),
                "received text body changed");
            require(received.getReceipt().getProvider() == provider,
                "receive provider changed");
            require(received.getReceipt().getState() == ModernDeliveryState.RECEIVED,
                "receive did not return RECEIVED");

            ModernDeliveryReceipt acknowledged = client.acknowledge(received.getReceipt());
            require(acknowledged.getProvider() == provider, "acknowledge provider changed");
            require(acknowledged.getState() == ModernDeliveryState.ACKNOWLEDGED,
                "acknowledge did not return ACKNOWLEDGED");
            require(messageId.equals(acknowledged.getMessageId()),
                "acknowledge message id changed");
        } finally {
            client.close();
        }

        System.out.println("broker-backed-messaging=PASS provider=" + provider.name()
            + " destination=" + destination);
    }

    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }
}
