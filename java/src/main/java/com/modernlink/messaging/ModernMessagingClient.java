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
        this(url, subject, mode, provider, null);
    }

    /**
     * Open with a routing policy. Rules are evaluated in order, first match wins;
     * a null or empty array behaves exactly like the unrouted constructor.
     */
    public ModernMessagingClient(String url, String subject, ModernMessagingMode mode,
        ModernMessagingProvider provider, ModernRouteRule[] rules) throws LegacyHttpException {
        if (url == null || subject == null || mode == null || provider == null) {
            throw new IllegalArgumentException("messaging connection fields are required");
        }
        NativeLoader.load();
        this.mode = mode;
        this.provider = provider;
        if (rules == null || rules.length == 0) {
            this.handle = nativeOpen(url, subject, mode.name(), provider.name());
        } else {
            String[] encoded = new String[rules.length];
            for (int index = 0; index < rules.length; index++) {
                if (rules[index] == null) throw new IllegalArgumentException("routing rule " + index + " is null");
                encoded[index] = rules[index].encode();
            }
            this.handle = nativeOpenRouted(url, subject, mode.name(), provider.name(), encoded);
        }
        if (this.handle == 0) throw nativeError("native messaging client unavailable");
    }

    /**
     * Evaluate the routing policy for a hypothetical message without publishing it.
     *
     * A denied route is returned as a decision, not an exception, so the caller can
     * report which rule denied it.
     */
    public synchronized ModernRouteDecision dryRun(String destination, String tenant,
        String headerName, String headerValue) throws LegacyHttpException {
        requireOpen();
        if (destination == null || destination.length() == 0) {
            throw new IllegalArgumentException("destination is required");
        }
        if ((headerName == null) != (headerValue == null)) {
            throw new IllegalArgumentException("header name and value must be supplied together");
        }
        String value = nativeDryRun(handle, destination, tenant == null ? "" : tenant,
            headerName == null ? "" : headerName, headerValue == null ? "" : headerValue);
        if (value == null) throw nativeError("native route evaluation unavailable");
        return ModernRouteDecision.decode(value);
    }

    public ModernMessagingMode getMode() { return mode; }
    public ModernMessagingProvider getProvider() { return provider; }

    public synchronized ModernDeliveryReceipt publish(ModernMessage message) throws Exception {
        requireOpen();
        if (message == null) throw new IllegalArgumentException("message is required");
        // MSG-05: the body goes over base64-encoded for EVERY category, not just the
        // non-text ones. A BytesMessage passed as a Java String would be mangled by the
        // UTF-8 round trip, and having one category take a different path is how that bug
        // comes back. The category rides alongside so the native side never guesses.
        ModernPayload body = message.getBody();
        String value = nativePublish(handle, message.getMessageId(), message.getDestination(), body.encodeBody(),
            message.getTracing().getTraceId(), message.getTracing().getSpanId(),
            message.getTracing().getParentSpanId() == null ? "" : message.getTracing().getParentSpanId(),
            message.getTracing().getTraceState() == null ? "" : message.getTracing().getTraceState(),
            message.getTracing().isSampled(), message.getAcknowledgementMode().name(),
            body.getKind().name());
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

    /**
     * The guarantee table for a provider, queryable BEFORE any traffic moves (MSG-04).
     *
     * Static and connectionless on purpose: the whole point is to let a deployment ask
     * "can this provider honour what we need?" without first opening a connection to it.
     *
     * Read a DECLARED field as "claimed, never tested". Most fields are DECLARED today.
     */
    public static ModernProviderGuarantees guaranteesFor(ModernMessagingProvider provider)
        throws LegacyHttpException {
        if (provider == null) {
            throw new IllegalArgumentException("provider is required");
        }
        NativeLoader.load();
        String frame = nativeProviderGuarantees(provider.name());
        if (frame == null) {
            String detail = nativeLastError();
            throw new LegacyHttpException(detail == null || detail.length() == 0
                ? "native provider guarantees unavailable" : detail);
        }
        return ModernProviderGuarantees.decode(frame);
    }


    /**
     * Last-resort reclamation for a client the caller never closed (H-08).
     *
     * Java 6 has no try-with-resources, so the ergonomic guard modern code would use is not
     * available here - which makes a forgotten close MORE likely in this codebase, not less.
     * Without this, the native client stays in the registry for the life of the JVM, holding
     * a transport, a tokio runtime and its threads.
     *
     * This is a backstop, NOT the mechanism. Finalizers run at the GC's discretion, may never
     * run at all before exit, and are removed in later Java versions. Callers must still call
     * {@link #close()}; this only bounds the damage when they do not.
     *
     * Every throwable is swallowed: an exception escaping finalize() aborts finalization for
     * this object and can stall the finalizer thread for every other object in the JVM. A
     * leaked transport is a much smaller problem than a wedged finalizer queue.
     */
    @Override
    protected void finalize() throws Throwable {
        try {
            close();
        } catch (Throwable suppressed) {
            // Deliberately ignored - see above.
        } finally {
            super.finalize();
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
    private static native long nativeOpenRouted(String url, String subject, String mode, String provider,
        String[] rules);
    private static native String nativeDryRun(long handle, String destination, String tenant,
        String headerName, String headerValue);
    private static native String nativePublish(long handle, String messageId, String destination, String payload,
        String traceId, String spanId, String parentSpanId, String traceState, boolean sampled,
        String acknowledgementMode, String payloadKind);
    private static native String nativeReceive(long handle);
    private static native String nativeAcknowledge(long handle, String messageId, String provider,
        String state, String traceId);
    private static native void nativeClose(long handle);
    private static native String nativeProviderGuarantees(String provider);
    private static native String nativeLastError();
}
