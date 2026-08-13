package com.modernlink;

import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;

public final class LegacyHttpResponse {
    private final int status;
    private final Map<String, String> headers;
    private final byte[] body;
    private final LegacyTlsInfo tlsInfo;

    public LegacyHttpResponse(int status, Map<String, String> headers, byte[] body, LegacyTlsInfo tlsInfo) {
        this.status = status;
        this.headers = Collections.unmodifiableMap(new LinkedHashMap<String, String>(headers));
        this.body = body.clone();
        this.tlsInfo = tlsInfo;
    }

    public int getStatus() { return status; }
    public Map<String, String> getHeaders() { return headers; }
    public String getHeader(String name) { return headers.get(name); }
    public byte[] getBody() { return body.clone(); }
    public LegacyTlsInfo getTlsInfo() { return tlsInfo; }
}
