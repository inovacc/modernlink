import com.modernlink.messaging.ModernAcknowledgementMode;
import com.modernlink.messaging.ModernConnection;
import com.modernlink.messaging.ModernConnectionFactory;
import com.modernlink.messaging.ModernDeliveryReceipt;
import com.modernlink.messaging.ModernMessageConsumer;
import com.modernlink.messaging.ModernMessageProducer;
import com.modernlink.messaging.ModernMessagingMode;
import com.modernlink.messaging.ModernMessagingProvider;
import com.modernlink.messaging.ModernReceivedMessage;
import com.modernlink.messaging.ModernSession;
import com.modernlink.messaging.ModernTextMessage;

/**
 * Java 6-compatible JMS-shaped flow. The provider is explicit: this example uses Kafka.
 * The URL is a placeholder and must come from deployment configuration.
 */
public final class JmsToKafka {
    private JmsToKafka() { }

    public static void sendAndReceive(String kafkaUrl) throws Exception {
        ModernConnectionFactory factory = new ModernConnectionFactory(
            kafkaUrl,
            "orders.created",
            ModernMessagingMode.REDIRECT,
            ModernMessagingProvider.KAFKA);
        ModernConnection connection = null;
        ModernSession session = null;
        ModernMessageProducer producer = null;
        ModernMessageConsumer consumer = null;
        try {
            connection = factory.createConnection();
            session = connection.createSession(ModernAcknowledgementMode.CLIENT);
            producer = session.createProducer("orders.created");
            consumer = session.createConsumer("orders.created");

            ModernTextMessage outgoing = session.createTextMessage("<order id=\"42\"/>");
            ModernDeliveryReceipt sent = producer.send(outgoing);

            ModernReceivedMessage received = consumer.receive();
            if (!"<order id=\"42\"/>".equals(received.getMessage().getPayload())) {
                throw new IllegalStateException("received message did not match the sent order");
            }
            System.out.println("received-message-id=" + received.getReceipt().getMessageId());
            consumer.acknowledge(received.getReceipt());
            System.out.println("receipt=" + sent.getMessageId());
        } finally {
            if (consumer != null) consumer.close();
            if (producer != null) producer.close();
            if (session != null) session.close();
            if (connection != null) connection.close();
        }
    }
}
