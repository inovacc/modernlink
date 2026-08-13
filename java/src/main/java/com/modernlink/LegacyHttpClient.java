package com.modernlink;

import java.util.LinkedHashMap;
import java.util.Map;

public final class LegacyHttpClient {
    private static final boolean LOAD_NATIVE = Boolean.valueOf(System.getProperty("modernlink.loadNative", "true")).booleanValue();

    public LegacyHttpResponse execute(LegacyHttpRequest request) throws LegacyHttpException {
        if (request == null) throw new IllegalArgumentException("request is required");
        if (LOAD_NATIVE) NativeLoader.load();
        long handle = nativeGet(request.getUrl());
        if (handle == 0L) throw new LegacyHttpException("native request failed");
        try {
            return decode(handle);
        } finally {
            nativeRelease(handle);
        }
    }

    private static native long nativeGet(String url);
    private static native int nativeStatus(long handle);
    private static native String[] nativeHeaders(long handle);
    private static native byte[] nativeBody(long handle);
    private static native void nativeRelease(long handle);

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
        return new LegacyHttpResponse(status, headers, body, new LegacyTlsInfo(null, null));
    }
}
