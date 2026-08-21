package com.modernlink;

import com.modernlink.messaging.ModernMessagingClient;
import com.modernlink.messaging.ModernMessagingMode;
import com.modernlink.messaging.ModernMessagingProvider;
import com.modernlink.messaging.ModernRouteRule;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;

/**
 * Exercises fail-closed JNI branches that the public Java facade rejects earlier.
 *
 * The reflection is deliberate: these methods are private so applications cannot bypass
 * Java-side validation, but the native boundary must still reject malformed values when a
 * stale or foreign facade calls it directly. Java 6 syntax only.
 */
public final class NativeBoundaryEdgeCasesTest {
    public static void main(String[] args) throws Exception {
        NativeLoader.load();
        messagingBoundaryRejectsMalformedFrames();
        httpBoundaryRejectsMalformedRequestsAndHandles();
        utilityBoundaryRejectsMalformedValues();
        System.out.println("native-boundary-edge-cases=PASS");
    }

    private static void messagingBoundaryRejectsMalformedFrames() throws Exception {
        Class owner = ModernMessagingClient.class;
        Method open = method(owner, "nativeOpen", new Class[] {
            String.class, String.class, String.class, String.class
        });
        Method openRouted = method(owner, "nativeOpenRouted", new Class[] {
            String.class, String.class, String.class, String.class, String[].class
        });
        Method dryRun = method(owner, "nativeDryRun", new Class[] {
            Long.TYPE, String.class, String.class, String.class, String.class
        });
        Method publish = method(owner, "nativePublish", new Class[] {
            Long.TYPE, String.class, String.class, String.class, String.class, String.class,
            String.class, String.class, Boolean.TYPE, String.class, String.class
        });
        Method receive = method(owner, "nativeReceive", new Class[] { Long.TYPE });
        Method acknowledge = method(owner, "nativeAcknowledge", new Class[] {
            Long.TYPE, String.class, String.class, String.class, String.class
        });
        Method close = method(owner, "nativeClose", new Class[] { Long.TYPE });
        Method guarantees = method(owner, "nativeProviderGuarantees", new Class[] { String.class });
        Method lastError = method(owner, "nativeLastError", new Class[0]);

        require(zero(call(open, new Object[] { null, "subject", "TRANSPARENT", "LEGACY_JMS" })),
            "native open accepted a null URL");
        requireError(lastError, "null open URL");
        require(zero(call(open, new Object[] {
            "legacy-jms://in-process", "subject", "UNKNOWN", "LEGACY_JMS"
        })), "native open accepted an unknown mode");
        requireError(lastError, "unknown open mode");
        require(zero(call(open, new Object[] {
            "legacy-jms://in-process", "subject", "TRANSPARENT", "UNKNOWN"
        })), "native open accepted an unknown provider");
        requireError(lastError, "unknown open provider");
        require(zero(call(open, new Object[] {
            "legacy-jms://in-process", "", "TRANSPARENT", "LEGACY_JMS"
        })), "native open accepted an empty destination");
        requireError(lastError, "empty open destination");
        require(zero(call(open, new Object[] {
            "legacy-jms://in-process", "subject", "TRANSPARENT", "PULSAR"
        })), "native open accepted an incompatible mode and provider");
        requireError(lastError, "incompatible open route");
        require(zero(call(open, new Object[] {
            "://", "subject", "REDIRECT", "NATS"
        })), "native open accepted an invalid broker endpoint");
        requireError(lastError, "invalid broker endpoint");
        require(zero(call(openRouted, new Object[] {
            "legacy-jms://in-process", "subject", "TRANSPARENT", "LEGACY_JMS", null
        })), "native routed open accepted a null rule array");
        requireError(lastError, "null routing rules");
        require(zero(call(openRouted, new Object[] {
            "legacy-jms://in-process", "subject", "TRANSPARENT", "LEGACY_JMS",
            new String[] { "not-a-route-frame" }
        })), "native routed open accepted a malformed rule");
        requireError(lastError, "malformed routing rule");
        require(zero(call(openRouted, new Object[] {
            "legacy-jms://in-process", "subject", "UNKNOWN", "LEGACY_JMS",
            new String[0]
        })), "native routed open accepted an unknown mode");
        requireError(lastError, "unknown routed mode");
        require(zero(call(openRouted, new Object[] {
            "legacy-jms://in-process", "subject", "TRANSPARENT", "UNKNOWN",
            new String[0]
        })), "native routed open accepted an unknown provider");
        requireError(lastError, "unknown routed provider");
        require(zero(call(openRouted, new Object[] {
            "legacy-jms://in-process", "subject", "TRANSPARENT", "LEGACY_JMS",
            new String[] { null }
        })), "native routed open accepted a null rule");
        requireError(lastError, "null routed rule");
        require(zero(call(openRouted, new Object[] {
            "legacy-jms://in-process", "", "TRANSPARENT", "LEGACY_JMS",
            new String[0]
        })), "native routed open accepted an empty destination");
        requireError(lastError, "empty routed destination");
        require(zero(call(openRouted, new Object[] {
            "legacy-jms://in-process", "subject", "TRANSPARENT", "PULSAR",
            new String[0]
        })), "native routed open accepted an incompatible mode and provider");
        requireError(lastError, "incompatible routed route");
        require(zero(call(openRouted, new Object[] {
            "://", "subject", "REDIRECT", "NATS", new String[0]
        })), "native routed open accepted an invalid broker endpoint");
        requireError(lastError, "invalid routed broker endpoint");

        require(call(dryRun, new Object[] {
            new Long(0L), "subject", "", "", ""
        }) == null, "native dry-run accepted an invalid handle");
        requireError(lastError, "invalid dry-run handle");
        require(call(publish, publishArguments(0L, "message", "subject", "eA==", "AUTO", "TEXT")) == null,
            "native publish accepted an invalid handle");
        requireError(lastError, "invalid publish handle");
        require(call(receive, new Object[] { new Long(0L) }) == null,
            "native receive accepted an invalid handle");
        requireError(lastError, "invalid receive handle");
        require(call(acknowledge, new Object[] {
            new Long(0L), "message", "LEGACY_JMS", "RECEIVED", "trace"
        }) == null, "native acknowledge accepted an invalid handle");
        requireError(lastError, "invalid acknowledge handle");
        call(close, new Object[] { new Long(0L) });

        ModernMessagingClient client = new ModernMessagingClient(
            "legacy-jms://in-process", "native.boundary.queue",
            ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.LEGACY_JMS);
        long handle = handle(client);
        try {
            require(call(dryRun, new Object[] {
                new Long(handle), null, "", "", ""
            }) == null, "native dry-run accepted a null destination");
            requireError(lastError, "null dry-run destination");
            require(call(dryRun, new Object[] {
                new Long(handle), "", "", "", ""
            }) == null, "native dry-run accepted an empty destination");
            requireError(lastError, "empty dry-run destination");
            require(call(receive, new Object[] { new Long(handle) }) == null,
                "native receive invented a message");
            requireError(lastError, "empty receive");

            require(call(publish, publishArguments(handle, null, "subject", "eA==", "AUTO", "TEXT")) == null,
                "native publish accepted a null message field");
            requireError(lastError, "null publish field");
            require(call(publish, publishArguments(handle, "message", "subject", "%%%", "AUTO", "TEXT")) == null,
                "native publish accepted invalid Base64");
            requireError(lastError, "invalid publish Base64");
            require(call(publish, publishArguments(handle, "message", "subject", "eA==", "UNKNOWN", "TEXT")) == null,
                "native publish accepted an unknown acknowledgement mode");
            requireError(lastError, "unknown acknowledgement mode");
            require(call(publish, publishArguments(handle, "message", "subject", "eA==", "AUTO", "OBJECT")) == null,
                "native publish accepted an object payload");
            requireError(lastError, "object payload");
            require(call(publish, publishArguments(handle, "message", "", "eA==", "AUTO", "TEXT")) == null,
                "native publish accepted an empty destination");
            requireError(lastError, "empty publish destination");

            Object[] valid = publishArguments(handle, "message-valid", "native.boundary.queue",
                "Ym9keQ==", "AUTO", "TEXT");
            valid[6] = "parent";
            valid[7] = "vendor=state";
            require(call(publish, valid) != null, "native publish rejected a valid traced message");
            String received = (String) call(receive, new Object[] { new Long(handle) });
            require(received != null && received.indexOf('\n') > 0,
                "native receive did not return a message and receipt frame");
            String receipt = received.substring(received.indexOf('\n') + 1);
            String[] fields = receipt.split("#", -1);
            require(fields.length == 4, "native receipt field count changed");
            require(call(acknowledge, new Object[] {
                new Long(handle), fields[0], fields[1], fields[2], fields[3]
            }) != null, "native acknowledge rejected a received receipt");

            require(call(acknowledge, new Object[] {
                new Long(handle), null, "LEGACY_JMS", "RECEIVED", "trace"
            }) == null, "native acknowledge accepted a null receipt field");
            requireError(lastError, "null acknowledgement field");
            require(call(acknowledge, new Object[] {
                new Long(handle), "message", "UNKNOWN", "RECEIVED", "trace"
            }) == null, "native acknowledge accepted an unknown provider");
            requireError(lastError, "unknown acknowledgement provider");
            require(call(acknowledge, new Object[] {
                new Long(handle), "message", "LEGACY_JMS", "UNKNOWN", "trace"
            }) == null, "native acknowledge accepted an unknown state");
            requireError(lastError, "unknown acknowledgement state");
        } finally {
            client.close();
        }

        ModernRouteRule invalidRoute = new ModernRouteRule(
            "invalid-mode-provider", ModernMessagingMode.TRANSPARENT,
            ModernMessagingProvider.PULSAR).destination("native.boundary.invalid-route");
        ModernMessagingClient invalidRouteClient = new ModernMessagingClient(
            "legacy-jms://in-process", "native.boundary.default",
            ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.LEGACY_JMS,
            new ModernRouteRule[] { invalidRoute });
        try {
            require(call(dryRun, new Object[] {
                new Long(handle(invalidRouteClient)), "native.boundary.invalid-route", "", "", ""
            }) == null, "native dry-run accepted an incompatible selected route");
            requireError(lastError, "incompatible dry-run route");
        } finally {
            invalidRouteClient.close();
        }

        ModernRouteRule deniedRoute = new ModernRouteRule(
            "deny", ModernMessagingMode.TRANSPARENT,
            ModernMessagingProvider.LEGACY_JMS).destination("native.boundary.denied").allowed(false);
        ModernMessagingClient deniedRouteClient = new ModernMessagingClient(
            "legacy-jms://in-process", "native.boundary.default",
            ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.LEGACY_JMS,
            new ModernRouteRule[] { deniedRoute });
        try {
            require(call(publish, publishArguments(handle(deniedRouteClient), "denied-message",
                "native.boundary.denied", "Ym9keQ==", "AUTO", "TEXT")) == null,
                "native publish ignored a denied route");
            requireError(lastError, "denied publish route");
        } finally {
            deniedRouteClient.close();
        }

        ModernRouteRule mismatchedRoute = new ModernRouteRule(
            "mismatch", ModernMessagingMode.REDIRECT,
            ModernMessagingProvider.NATS).destination("native.boundary.mismatch");
        ModernMessagingClient mismatchedRouteClient = new ModernMessagingClient(
            "legacy-jms://in-process", "native.boundary.default",
            ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.LEGACY_JMS,
            new ModernRouteRule[] { mismatchedRoute });
        try {
            require(call(publish, publishArguments(handle(mismatchedRouteClient), "mismatch-message",
                "native.boundary.mismatch", "Ym9keQ==", "AUTO", "TEXT")) == null,
                "native publish ignored a route/transport mismatch");
            requireError(lastError, "mismatched publish route");
        } finally {
            mismatchedRouteClient.close();
        }

        require(call(receive, new Object[] { new Long(handle) }) == null,
            "native receive accepted a closed handle");
        require(call(guarantees, new Object[] { null }) == null,
            "native guarantees accepted a null provider");
        requireError(lastError, "null guarantee provider");
        require(call(guarantees, new Object[] { "UNKNOWN" }) == null,
            "native guarantees accepted an unknown provider");
        requireError(lastError, "unknown guarantee provider");
    }

    private static void httpBoundaryRejectsMalformedRequestsAndHandles() throws Exception {
        Class owner = LegacyHttpClient.class;
        Method execute = method(owner, "nativeExecute", new Class[] {
            String.class, String.class, String[].class, byte[].class,
            Long.TYPE, Long.TYPE, Boolean.TYPE, Integer.TYPE, Integer.TYPE
        });
        Method lastError = method(owner, "nativeLastError", new Class[0]);
        Method status = method(owner, "nativeStatus", new Class[] { Long.TYPE });
        Method statusMessage = method(owner, "nativeStatusMessage", new Class[] { Long.TYPE });
        Method headers = method(owner, "nativeHeaders", new Class[] { Long.TYPE });
        Method body = method(owner, "nativeBody", new Class[] { Long.TYPE });
        Method certificates = method(owner, "nativeTlsCertificates", new Class[] { Long.TYPE });
        Method protocol = method(owner, "nativeTlsProtocol", new Class[] { Long.TYPE });
        Method cipher = method(owner, "nativeTlsCipherSuite", new Class[] { Long.TYPE });
        Method finalUrl = method(owner, "nativeFinalUrl", new Class[] { Long.TYPE });
        Method release = method(owner, "nativeRelease", new Class[] { Long.TYPE });

        require(zero(call(execute, httpArguments(null, "GET", new String[0], 10))),
            "native HTTP accepted a null URL");
        requireHttpError(lastError, "null HTTP URL");
        require(zero(call(execute, httpArguments("https://example.com", null, new String[0], 10))),
            "native HTTP accepted a null method");
        requireHttpError(lastError, "null HTTP method");
        require(zero(call(execute, httpArguments("https://example.com", "BAD METHOD", new String[0], 10))),
            "native HTTP accepted an invalid method");
        requireHttpError(lastError, "invalid HTTP method");
        require(zero(call(execute, httpArguments("https://example.com", "GET", new String[] { "X" }, 10))),
            "native HTTP accepted an odd header array");
        requireHttpError(lastError, "odd HTTP headers");
        require(zero(call(execute, httpArguments("https://example.com", "GET",
            new String[] { "bad name", "value" }, 10))),
            "native HTTP accepted an invalid header name");
        requireHttpError(lastError, "invalid HTTP header name");
        require(zero(call(execute, httpArguments("https://example.com", "GET",
            new String[] { "X-Test", "bad\r\nvalue" }, 10))),
            "native HTTP accepted an invalid header value");
        requireHttpError(lastError, "invalid HTTP header value");
        require(zero(call(execute, httpArguments("https://example.com", "GET",
            new String[] { null, "value" }, 10))),
            "native HTTP accepted a null header name");
        requireHttpError(lastError, "null HTTP header name");
        require(zero(call(execute, httpArguments("https://example.com", "GET",
            new String[] { "X-Test", null }, 10))),
            "native HTTP accepted a null header value");
        requireHttpError(lastError, "null HTTP header value");
        Object[] nullBody = httpArguments("https://example.com", "GET", new String[0], 10);
        nullBody[3] = null;
        require(zero(call(execute, nullBody)), "native HTTP accepted a null body");
        requireHttpError(lastError, "null HTTP body");
        Object[] redirects = httpArguments("https://example.com", "GET", new String[0], 10);
        redirects[7] = new Integer(-1);
        require(zero(call(execute, redirects)), "native HTTP accepted negative redirects");
        requireHttpError(lastError, "negative redirects");
        Object[] tls = httpArguments("https://example.com", "GET", new String[0], 10);
        tls[8] = new Integer(10);
        require(zero(call(execute, tls)), "native HTTP accepted an unsupported TLS floor");
        requireHttpError(lastError, "unsupported TLS floor");
        require(zero(call(execute, httpArguments("not a URL", "GET", new String[0], 10))),
            "native HTTP accepted an invalid URL");
        requireHttpError(lastError, "invalid parsed URL");
        require(zero(call(execute, httpArguments("https://127.0.0.1:1", "GET", new String[0], 1))),
            "native HTTP unexpectedly returned a response from an unreachable endpoint");
        requireHttpError(lastError, "unreachable endpoint");

        require(((Integer) call(status, new Object[] { new Long(0L) })).intValue() == 0,
            "native HTTP status accepted an invalid handle");
        require(call(statusMessage, new Object[] { new Long(0L) }) == null,
            "native HTTP status message accepted an invalid handle");
        require(call(headers, new Object[] { new Long(0L) }) == null,
            "native HTTP headers accepted an invalid handle");
        require(call(body, new Object[] { new Long(0L) }) == null,
            "native HTTP body accepted an invalid handle");
        require(call(certificates, new Object[] { new Long(0L) }) == null,
            "native TLS certificates accepted an invalid handle");
        require(call(protocol, new Object[] { new Long(0L) }) == null,
            "native TLS protocol accepted an invalid handle");
        require(call(cipher, new Object[] { new Long(0L) }) == null,
            "native TLS cipher accepted an invalid handle");
        require(call(finalUrl, new Object[] { new Long(0L) }) == null,
            "native final URL accepted an invalid handle");
        call(release, new Object[] { new Long(0L) });
        call(release, new Object[] { new Long(Long.MAX_VALUE) });
    }

    private static void utilityBoundaryRejectsMalformedValues() throws Exception {
        expectLegacyHttpException("invalid Base64", new CheckedAction() {
            public void run() throws Exception { ModernBase64.decode("%%%"); }
        });
        expectLegacyHttpException("odd JSON object fields", new CheckedAction() {
            public void run() throws Exception { ModernJson.object(new String[] { "key" }); }
        });
        expectLegacyHttpException("malformed JSON", new CheckedAction() {
            public void run() throws Exception { ModernJson.decode("{"); }
        });

        Method nativeEncode = method(ModernBase64.class, "nativeEncode", new Class[] { byte[].class });
        Method nativeDecode = method(ModernBase64.class, "nativeDecode", new Class[] { String.class });
        Method nativeObject = method(ModernJson.class, "nativeObject", new Class[] { String[].class });
        Method nativeArray = method(ModernJson.class, "nativeArray", new Class[] { String[].class });
        Method nativeJsonDecode = method(ModernJson.class, "nativeDecode", new Class[] { String.class });
        require(call(nativeEncode, new Object[] { null }) == null,
            "native Base64 accepted a null byte array");
        require(call(nativeDecode, new Object[] { null }) == null,
            "native Base64 decode accepted a null string");
        require(call(nativeDecode, new Object[] { "%%%" }) == null,
            "native Base64 decode accepted malformed input");
        require(call(nativeObject, new Object[] { null }) == null,
            "native JSON object accepted a null field array");
        require(call(nativeObject, new Object[] { new String[] { null, "value" } }) == null,
            "native JSON object accepted a null field");
        require(call(nativeArray, new Object[] { null }) == null,
            "native JSON accepted a null value array");
        require(call(nativeArray, new Object[] { new String[] { null } }) == null,
            "native JSON accepted a null array element");
        require(call(nativeJsonDecode, new Object[] { null }) == null,
            "native JSON decode accepted a null string");
    }

    private static Object[] publishArguments(long handle, String messageId, String destination,
        String payload, String acknowledgement, String kind) {
        return new Object[] {
            new Long(handle), messageId, destination, payload, "trace", "span", "", "",
            Boolean.TRUE, acknowledgement, kind
        };
    }

    private static Object[] httpArguments(String url, String method, String[] headers, int timeout) {
        return new Object[] {
            url, method, headers, new byte[0], new Long(timeout), new Long(timeout),
            Boolean.FALSE, new Integer(0), new Integer(12)
        };
    }

    private static long handle(ModernMessagingClient client) throws Exception {
        Field field = ModernMessagingClient.class.getDeclaredField("handle");
        field.setAccessible(true);
        return field.getLong(client);
    }

    private static Method method(Class owner, String name, Class[] parameterTypes) throws Exception {
        Method value = owner.getDeclaredMethod(name, parameterTypes);
        value.setAccessible(true);
        return value;
    }

    private static Object call(Method method, Object[] arguments) throws Exception {
        try {
            return method.invoke(null, arguments);
        } catch (InvocationTargetException error) {
            Throwable cause = error.getCause();
            if (cause instanceof Exception) throw (Exception) cause;
            if (cause instanceof Error) throw (Error) cause;
            throw error;
        }
    }

    private static boolean zero(Object value) {
        return value instanceof Long && ((Long) value).longValue() == 0L;
    }

    private static void requireError(Method lastError, String label) throws Exception {
        String error = (String) call(lastError, new Object[0]);
        require(error != null && error.length() > 0, label + " did not set a native error");
    }

    private static void requireHttpError(Method lastError, String label) throws Exception {
        String error = (String) call(lastError, new Object[0]);
        require(error != null && error.length() > 0, label + " did not set an HTTP error");
    }

    private static void expectLegacyHttpException(String label, CheckedAction action) throws Exception {
        try {
            action.run();
            throw new AssertionError(label + " was accepted");
        } catch (LegacyHttpException expected) {
            // Expected fail-closed result.
        }
    }

    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private interface CheckedAction {
        void run() throws Exception;
    }
}
