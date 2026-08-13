package com.modernlink.messaging;

/** Read-only JMX contract for messaging operational counters. */
public interface ModernMessagingMetricsMBean {
    String getMode();
    String getProvider();
    long getPublished();
    long getReceived();
    long getAcknowledged();
    String getLastTraceId();
}
