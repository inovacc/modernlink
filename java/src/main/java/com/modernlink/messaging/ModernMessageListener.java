package com.modernlink.messaging;

/** Java 6 callback contract corresponding to a JMS message listener. */
public interface ModernMessageListener {
    void onMessage(ModernTextMessage message, ModernDeliveryReceipt receipt);
}
