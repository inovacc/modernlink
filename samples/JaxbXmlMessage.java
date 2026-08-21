import com.modernlink.messaging.ModernAcknowledgementMode;
import com.modernlink.messaging.ModernSession;
import com.modernlink.messaging.ModernTextMessage;
import java.io.StringReader;
import java.io.StringWriter;
import javax.xml.bind.JAXBContext;
import javax.xml.bind.Marshaller;
import javax.xml.bind.Unmarshaller;
import javax.xml.bind.annotation.XmlRootElement;

/** JAXB stays at the application edge; ModernLink carries the resulting XML as text. */
public final class JaxbXmlMessage {
    private JaxbXmlMessage() { }

    public static ModernTextMessage toMessage(ModernSession session, Order order) throws Exception {
        JAXBContext context = JAXBContext.newInstance(Order.class);
        Marshaller marshaller = context.createMarshaller();
        StringWriter output = new StringWriter();
        marshaller.marshal(order, output);
        return session.createTextMessage(output.toString());
    }

    public static Order fromMessage(ModernTextMessage message) throws Exception {
        JAXBContext context = JAXBContext.newInstance(Order.class);
        Unmarshaller unmarshaller = context.createUnmarshaller();
        return (Order) unmarshaller.unmarshal(new StringReader(message.getText()));
    }

    @XmlRootElement(name = "order")
    public static final class Order {
        public String id;
        public String customer;

        public Order() { }

        public Order(String id, String customer) {
            this.id = id;
            this.customer = customer;
        }
    }
}
