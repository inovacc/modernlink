import com.modernlink.messaging.ModernAcknowledgementMode;
import com.modernlink.messaging.ModernConnection;
import com.modernlink.messaging.ModernConnectionFactory;
import com.modernlink.messaging.ModernMessageProducer;
import com.modernlink.messaging.ModernMessagingMode;
import com.modernlink.messaging.ModernMessagingProvider;
import com.modernlink.messaging.ModernSession;
import com.modernlink.messaging.ModernTextMessage;
import javax.jws.WebMethod;
import javax.jws.WebService;

/**
 * Illustrative Java 6 JAX-WS endpoint. Generated service plumbing is owned by the host;
 * only the handoff into ModernLink is shown here.
 */
@WebService
public final class JaxWsToRabbitMq {
    private final String rabbitUrl;

    public JaxWsToRabbitMq(String rabbitUrl) {
        this.rabbitUrl = rabbitUrl;
    }

    @WebMethod
    public String submitOrder(String orderXml) throws Exception {
        ModernConnectionFactory factory = new ModernConnectionFactory(
            rabbitUrl,
            "orders.incoming",
            ModernMessagingMode.REDIRECT,
            ModernMessagingProvider.RABBITMQ);
        ModernConnection connection = null;
        ModernSession session = null;
        ModernMessageProducer producer = null;
        try {
            connection = factory.createConnection();
            session = connection.createSession(ModernAcknowledgementMode.CLIENT);
            producer = session.createProducer("orders.incoming");
            ModernTextMessage message = session.createTextMessage(orderXml);
            return producer.send(message).getMessageId();
        } finally {
            if (producer != null) producer.close();
            if (session != null) session.close();
            if (connection != null) connection.close();
        }
    }
}
