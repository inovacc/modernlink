package com.modernlink.messaging;

public interface ModernMessagingMetricsMBean {
    String getMode();
    String getProvider();
    long getPublished();
    String getLastTraceId();
}
