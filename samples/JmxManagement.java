import com.modernlink.messaging.ModernConnection;
import com.modernlink.messaging.ModernMessagingMetricsMBean;
import java.lang.management.ManagementFactory;
import javax.management.MBeanServer;
import javax.management.ObjectName;

/**
 * JMX is an observability surface, not a payload channel. Never publish message bodies,
 * credentials, or broker URLs as JMX attributes.
 */
public final class JmxManagement {
    private JmxManagement() { }

    public static void printMetrics(ModernConnection connection) throws Exception {
        MBeanServer server = ManagementFactory.getPlatformMBeanServer();
        ObjectName name = connection.getMetricsObjectName();
        ModernMessagingMetricsMBean metrics = javax.management.JMX.newMBeanProxy(
            server, name, ModernMessagingMetricsMBean.class);

        System.out.println("mode=" + metrics.getMode());
        System.out.println("provider=" + metrics.getProvider());
        System.out.println("published=" + metrics.getPublished());
        System.out.println("received=" + metrics.getReceived());
        System.out.println("acknowledged=" + metrics.getAcknowledged());
        // A trace identifier is diagnostic metadata; the message body is deliberately absent.
        System.out.println("lastTraceId=" + metrics.getLastTraceId());
    }
}
