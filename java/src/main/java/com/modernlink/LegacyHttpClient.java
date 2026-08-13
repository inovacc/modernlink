package com.modernlink;

import java.util.LinkedHashMap;
import java.util.Map;

public final class LegacyHttpClient {
    public static final long CAPABILITY_HTTPS = 1L;
    public static final long CAPABILITY_TLS_1_2 = 2L;
    public static final long CAPABILITY_TLS_1_3 = 4L;
    public static final long CAPABILITY_REDIRECTS = 8L;
    public static final long CAPABILITY_PEER_CERTIFICATES = 16L;
    private static final boolean LOAD_NATIVE = Boolean.valueOf(System.getProperty("modernlink.loadNative", "true")).booleanValue();

    public LegacyHttpResponse execute(LegacyHttpRequest request) throws LegacyHttpException {
        if (request == null) throw new IllegalArgumentException("request is required");
        if (LOAD_NATIVE) NativeLoader.load();
        String[] headers = flattenHeaders(request.getHeaders());
        long handle = nativeExecute(request.getUrl(), request.getMethod(), headers, request.getBody(),
            request.getConnectTimeoutMillis(), request.getReadTimeoutMillis(),
            request.getFollowRedirects(), request.getMaxRedirects(), request.getMinimumTlsVersion());
        if (handle == 0L) throw new LegacyHttpException(nativeLastError());
        try {
            return decode(handle);
        } finally {
            nativeRelease(handle);
        }
    }

    private static native long nativeExecute(String url, String method, String[] headers, byte[] body,
        long connectTimeoutMillis, long readTimeoutMillis, boolean followRedirects, int maxRedirects,
        int minimumTlsVersion);
    private static native String nativeLastError();
    private static native long nativeCapabilities();
    private static native String nativeUuidV7();
    private static native String nativeBase64Encode(byte[] value);
    private static native String nativeRequestJson(String url, String method, String[] headers, byte[] body);
    private static native int nativeStatus(long handle);
    private static native String nativeStatusMessage(long handle);
    private static native String[] nativeHeaders(long handle);
    private static native byte[] nativeBody(long handle);
    private static native byte[][] nativeTlsCertificates(long handle);
    private static native String nativeTlsProtocol(long handle);
    private static native String nativeTlsCipherSuite(long handle);
    private static native String nativeFinalUrl(long handle);
    private static native void nativeRelease(long handle);

    public long getCapabilities() throws LegacyHttpException {
        if (LOAD_NATIVE) NativeLoader.load();
        long capabilities = nativeCapabilities();
        if (capabilities == 0L) throw new LegacyHttpException("native capabilities unavailable");
        return capabilities;
    }

    public String newUuidV7() throws LegacyHttpException {
        if (LOAD_NATIVE) NativeLoader.load();
        String value = nativeUuidV7();
        if (value == null) throw new LegacyHttpException("native UUIDv7 unavailable");
        return value;
    }

    public String base64Encode(byte[] value) throws LegacyHttpException {
        if (value == null) throw new IllegalArgumentException("value is required");
        if (LOAD_NATIVE) NativeLoader.load();
        String encoded = nativeBase64Encode(value.clone());
        if (encoded == null) throw new LegacyHttpException("native Base64 encoding unavailable");
        return encoded;
    }

    public String requestJson(LegacyHttpRequest request) throws LegacyHttpException {
        if (request == null) throw new IllegalArgumentException("request is required");
        if (LOAD_NATIVE) NativeLoader.load();
        String json = nativeRequestJson(request.getUrl(), request.getMethod(), flattenHeaders(request.getHeaders()), request.getBody());
        if (json == null) throw new LegacyHttpException("native JSON serialization unavailable");
        return json;
    }

    private String[] flattenHeaders(Map<String, String> headers) {
        String[] result = new String[headers.size() * 2];
        int index = 0;
        for (Map.Entry<String, String> entry : headers.entrySet()) {
            result[index++] = entry.getKey();
            result[index++] = entry.getValue();
        }
        return result;
    }

    private LegacyHttpResponse decode(long handle) throws LegacyHttpException {
        int status = nativeStatus(handle);
        if (status < 100 || status > 599) throw new LegacyHttpException("invalid native status");
        String[] encodedHeaders = nativeHeaders(handle);
        if (encodedHeaders == null || (encodedHeaders.length % 2) != 0) {
            throw new LegacyHttpException("invalid native headers");
        }
        Map<String, String> headers = new LinkedHashMap<String, String>();
        for (int i = 0; i < encodedHeaders.length; i += 2) {
            headers.put(encodedHeaders[i], encodedHeaders[i + 1]);
        }
        byte[] body = nativeBody(handle);
        if (body == null) throw new LegacyHttpException("native response body unavailable");
        return new LegacyHttpResponse(nativeFinalUrl(handle), status, nativeStatusMessage(handle), headers, body,
            new LegacyTlsInfo(nativeTlsProtocol(handle), nativeTlsCipherSuite(handle), nativeTlsCertificates(handle)));
    }
}
