package com.modernlink.messaging;

public final class ModernDeliveryReceipt {
    private final String messageId;
    private final ModernMessagingProvider provider;
    private final ModernDeliveryState state;
    private final String traceId;

    public ModernDeliveryReceipt(String messageId, ModernMessagingProvider provider,
        ModernDeliveryState state, String traceId) {
        this.messageId = messageId;
        this.provider = provider;
        this.state = state;
        this.traceId = traceId;
    }

    public String getMessageId() { return messageId; }
    public ModernMessagingProvider getProvider() { return provider; }
    public ModernDeliveryState getState() { return state; }
    public String getTraceId() { return traceId; }

    public String encode() {
        return messageId + "#" + provider.name() + "#" + state.name() + "#" + traceId;
    }

    public static ModernDeliveryReceipt decode(String value) {
        String[] fields = value.split("#", -1);
        if (fields.length != 4) throw new IllegalArgumentException("invalid delivery receipt");
        return new ModernDeliveryReceipt(fields[0], ModernMessagingProvider.valueOf(fields[1]),
            ModernDeliveryState.valueOf(fields[2]), fields[3]);
    }
}
