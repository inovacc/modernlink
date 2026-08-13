package com.modernlink.messaging;

import java.lang.management.ManagementFactory;
import javax.management.MBeanServer;
import javax.management.ObjectName;
import javax.management.StandardMBean;

import com.modernlink.LegacyHttpException;
import com.modernlink.ModernUuid;
import com.modernlink.ModernBase64;

/**
 * Java 6 demo publisher: JMS-shaped message creation plus a real local JMX
 * MBean. Its stdout is a small process-to-process fixture protocol.
 */
public final class LegacyJmsJmxDemo {
    private LegacyJmsJmxDemo() { }

    public static void main(String[] args) throws Exception {
        String mode = args.length > 0 ? args[0].toUpperCase() : "TRANSPARENT";
        String provider = args.length > 1 ? args[1].toUpperCase() : "LEGACY_JMS";
        ModernMessagingMode.valueOf(mode);
        ModernMessagingProvider.valueOf(provider);
        if ("TRANSPARENT".equals(mode) && !"LEGACY_JMS".equals(provider)) {
            throw new IllegalArgumentException("transparent mode requires LEGACY_JMS");
        }
        if (!"TRANSPARENT".equals(mode) && "LEGACY_JMS".equals(provider)) {
            throw new IllegalArgumentException("modern mode requires a modern provider");
        }
        ModernTraceContext tracing = ModernTraceContext.create();
        ModernMessage message = new ModernMessage(ModernUuid.v7(), "orders.created", "order-from-jms", tracing);
        DemoMetrics metrics = new DemoMetrics(mode, provider, tracing.getTraceId());
        MBeanServer server = ManagementFactory.getPlatformMBeanServer();
        server.registerMBean(new StandardMBean(metrics, ModernMessagingMetricsMBean.class),
            new ObjectName("com.modernlink.messaging:type=Metrics,role=Publisher"));
        metrics.published = 1;
        ModernDeliveryReceipt receipt = new ModernDeliveryReceipt(message.getMessageId(),
            ModernMessagingProvider.valueOf(provider), ModernDeliveryState.PUBLISHED,
            message.getTracing().getTraceId());
        System.out.println(mode + "|" + provider + "|" + ModernBase64.encode(message.encode().getBytes("UTF-8")) + "|" + receipt.encode());
    }

    public static final class DemoMetrics implements ModernMessagingMetricsMBean {
        private final String mode;
        private final String provider;
        private final String traceId;
        private long published;

        private DemoMetrics(String mode, String provider, String traceId) {
            this.mode = mode;
            this.provider = provider;
            this.traceId = traceId;
        }

        public String getMode() { return mode; }
        public String getProvider() { return provider; }
        public long getPublished() { return published; }
        public long getReceived() { return 0; }
        public long getAcknowledged() { return 0; }
        public String getLastTraceId() { return traceId; }
    }
}
