package com.modernlink;

public final class LegacyHttpRequestTest {
    public static void main(String[] args) throws Exception {
        try {
            new LegacyHttpRequest("");
            throw new AssertionError("empty URL must be rejected");
        } catch (IllegalArgumentException expected) {
            // expected
        }
        LegacyHttpRequest request = new LegacyHttpRequest("https://example.com");
        if (!"GET".equals(request.getMethod())) {
            throw new AssertionError("GET must be the default method");
        }
    }
}
