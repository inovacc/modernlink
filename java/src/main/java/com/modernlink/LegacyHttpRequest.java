package com.modernlink;

import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;

public final class LegacyHttpRequest {
    private final String url;
    private String method;
    private final Map<String, String> headers = new LinkedHashMap<String, String>();
    private byte[] body = new byte[0];
    private long connectTimeoutMillis;
    private long readTimeoutMillis;

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

    public LegacyHttpRequest method(String method) {
        if (method == null || method.trim().length() == 0) {
            throw new IllegalArgumentException("HTTP method is required");
        }
        this.method = method.toUpperCase();
        return this;
    }

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

    public LegacyHttpRequest body(byte[] body) {
        if (body == null) throw new IllegalArgumentException("body is required");
        this.body = body.clone();
        return this;
    }

    public byte[] getBody() { return body.clone(); }

    public LegacyHttpRequest connectTimeoutMillis(long timeout) {
        if (timeout < 0) throw new IllegalArgumentException("connect timeout must not be negative");
        connectTimeoutMillis = timeout;
        return this;
    }

    public LegacyHttpRequest readTimeoutMillis(long timeout) {
        if (timeout < 0) throw new IllegalArgumentException("read timeout must not be negative");
        readTimeoutMillis = timeout;
        return this;
    }

    public long getConnectTimeoutMillis() { return connectTimeoutMillis; }
    public long getReadTimeoutMillis() { return readTimeoutMillis; }
}
