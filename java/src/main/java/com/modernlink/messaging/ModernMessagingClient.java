package com.modernlink.messaging;

import com.modernlink.LegacyHttpException;
import com.modernlink.NativeLoader;

/** Java 6 messaging client backed by the native provider adapter. */
public final class ModernMessagingClient {
    private long handle;
    private final ModernMessagingMode mode;
    private final ModernMessagingProvider provider;

    public ModernMessagingClient(String url, String subject, ModernMessagingMode mode,
        ModernMessagingProvider provider) throws LegacyHttpException {
        if (url == null || subject == null || mode == null || provider == null) {
            throw new IllegalArgumentException("messaging connection fields are required");
        }
        NativeLoader.load();
        this.mode = mode;
        this.provider = provider;
        this.handle = nativeOpen(url, subject, mode.name(), provider.name());
        if (this.handle == 0) throw nativeError("native messaging client unavailable");
    }

    public ModernMessagingMode getMode() { return mode; }
    public ModernMessagingProvider getProvider() { return provider; }

    public synchronized ModernDeliveryReceipt publish(ModernMessage message) throws Exception {
        requireOpen();
        if (message == null) throw new IllegalArgumentException("message is required");
        String value = nativePublish(handle, message.getMessageId(), message.getDestination(), message.getPayload(),
            message.getTracing().getTraceId(), message.getTracing().getSpanId(),
            message.getTracing().getParentSpanId() == null ? "" : message.getTracing().getParentSpanId(),
            message.getTracing().getTraceState() == null ? "" : message.getTracing().getTraceState(),
            message.getTracing().isSampled(), message.getAcknowledgementMode().name());
        if (value == null) throw nativeError("native messaging publish unavailable");
        return ModernDeliveryReceipt.decode(value);
    }

    public synchronized ModernReceivedMessage receive() throws Exception {
        requireOpen();
        String value = nativeReceive(handle);
        if (value == null) throw nativeError("native messaging receive unavailable");
        String[] fields = value.split("\\n", 2);
        if (fields.length != 2) throw new IllegalArgumentException("invalid native messaging frame");
        return new ModernReceivedMessage(ModernMessage.decode(fields[0]), ModernDeliveryReceipt.decode(fields[1]));
    }

    public synchronized ModernDeliveryReceipt acknowledge(ModernDeliveryReceipt receipt) throws Exception {
        requireOpen();
        if (receipt == null) throw new IllegalArgumentException("receipt is required");
        String value = nativeAcknowledge(handle, receipt.getMessageId(), receipt.getProvider().name(),
            receipt.getState().name(), receipt.getTraceId());
        if (value == null) throw nativeError("native messaging acknowledgement unavailable");
        return ModernDeliveryReceipt.decode(value);
    }

    public synchronized void close() {
        if (handle != 0) {
            nativeClose(handle);
            handle = 0;
        }
    }

    private void requireOpen() throws LegacyHttpException {
        if (handle == 0) throw new LegacyHttpException("messaging client is closed");
    }

    private LegacyHttpException nativeError(String fallback) {
        String detail = nativeLastError();
        return new LegacyHttpException(detail == null || detail.length() == 0 ? fallback : detail);
    }

    private static native long nativeOpen(String url, String subject, String mode, String provider);
    private static native String nativePublish(long handle, String messageId, String destination, String payload,
        String traceId, String spanId, String parentSpanId, String traceState, boolean sampled,
        String acknowledgementMode);
    private static native String nativeReceive(long handle);
    private static native String nativeAcknowledge(long handle, String messageId, String provider,
        String state, String traceId);
    private static native void nativeClose(long handle);
    private static native String nativeLastError();
}
