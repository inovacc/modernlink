package com.modernlink;

import java.util.LinkedHashMap;
import java.util.Map;

public final class LegacyHttpClient {
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
    private static native int nativeStatus(long handle);
    private static native String[] nativeHeaders(long handle);
    private static native byte[] nativeBody(long handle);
    private static native byte[][] nativeTlsCertificates(long handle);
    private static native String nativeTlsProtocol(long handle);
    private static native String nativeTlsCipherSuite(long handle);
    private static native String nativeFinalUrl(long handle);
    private static native void nativeRelease(long handle);

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
        return new LegacyHttpResponse(nativeFinalUrl(handle), status, headers, body,
            new LegacyTlsInfo(nativeTlsProtocol(handle), nativeTlsCipherSuite(handle), nativeTlsCertificates(handle)));
    }
}
