package com.modernlink;

public final class LegacyHttpRequestConfigurationTest {
    public static void main(String[] args) {
        LegacyHttpRequest request = new LegacyHttpRequest("https://example.com")
            .method("POST")
            .header("X-ModernLink-Test", "request-config")
            .body(new byte[] {10, 20, 30})
            .connectTimeoutMillis(1234)
            .readTimeoutMillis(5678)
            .minimumTlsVersion(LegacyHttpRequest.TLS_1_3);
        if (!"POST".equals(request.getMethod())) throw new AssertionError("method not retained");
        if (!"request-config".equals(request.getHeaders().get("X-ModernLink-Test"))) {
            throw new AssertionError("header not retained");
        }
        if (request.getBody()[1] != 20) throw new AssertionError("body not retained");
        if (request.getConnectTimeoutMillis() != 1234L) throw new AssertionError("connect timeout not retained");
        if (request.getReadTimeoutMillis() != 5678L) throw new AssertionError("read timeout not retained");
        if (request.getMinimumTlsVersion() != LegacyHttpRequest.TLS_1_3) throw new AssertionError("TLS version not retained");
        boolean rejected = false;
        try {
            request.minimumTlsVersion(11);
        } catch (IllegalArgumentException expected) {
            rejected = true;
        }
        if (!rejected) throw new AssertionError("unsupported TLS version was accepted");
    }
}
