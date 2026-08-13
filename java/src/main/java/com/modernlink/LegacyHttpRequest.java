package com.modernlink;

import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;

public final class LegacyHttpRequest {
    private final String url;
    private final String method;
    private final Map<String, String> headers = new LinkedHashMap<String, String>();

    public LegacyHttpRequest(String url) {
        if (url == null || url.trim().length() == 0) {
            throw new IllegalArgumentException("URL must not be empty");
        }
        if (!url.startsWith("https://")) {
            throw new IllegalArgumentException("only https:// URLs are supported");
        }
        this.url = url;
        this.method = "GET";
    }

    public String getUrl() { return url; }
    public String getMethod() { return method; }

    public LegacyHttpRequest header(String name, String value) {
        if (name == null || value == null) {
            throw new IllegalArgumentException("header name and value are required");
        }
        headers.put(name, value);
        return this;
    }

    public Map<String, String> getHeaders() {
        return Collections.unmodifiableMap(headers);
    }
}
