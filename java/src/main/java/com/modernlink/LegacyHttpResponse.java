package com.modernlink;

import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;

public final class LegacyHttpResponse {
    private final String finalUrl;
    private final int status;
    private final String statusMessage;
    private final Map<String, String> headers;
    private final byte[] body;
    private final LegacyTlsInfo tlsInfo;

    public LegacyHttpResponse(String finalUrl, int status, String statusMessage, Map<String, String> headers, byte[] body, LegacyTlsInfo tlsInfo) {
        this.finalUrl = finalUrl;
        this.status = status;
        this.statusMessage = statusMessage;
        this.headers = Collections.unmodifiableMap(new LinkedHashMap<String, String>(headers));
        this.body = body.clone();
        this.tlsInfo = tlsInfo;
    }

    public int getStatus() { return status; }
    public String getStatusMessage() { return statusMessage; }
    public String getFinalUrl() { return finalUrl; }
    public Map<String, String> getHeaders() { return headers; }
    public String getHeader(String name) {
        if (name == null) return null;
        String exact = headers.get(name);
        if (exact != null) return exact;
        for (Map.Entry<String, String> entry : headers.entrySet()) {
            if (name.equalsIgnoreCase(entry.getKey())) return entry.getValue();
        }
        return null;
    }
    public byte[] getBody() { return body.clone(); }
    public LegacyTlsInfo getTlsInfo() { return tlsInfo; }
}
