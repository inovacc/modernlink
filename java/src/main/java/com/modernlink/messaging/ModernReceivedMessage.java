package com.modernlink.messaging;

/** Message and receipt returned by the provider-neutral receive boundary. */
public final class ModernReceivedMessage {
    private final ModernMessage message;
    private final ModernDeliveryReceipt receipt;

    public ModernReceivedMessage(ModernMessage message, ModernDeliveryReceipt receipt) {
        if (message == null || receipt == null) throw new IllegalArgumentException("message and receipt are required");
        this.message = message;
        this.receipt = receipt;
    }

    public ModernMessage getMessage() { return message; }
    public ModernDeliveryReceipt getReceipt() { return receipt; }
}
