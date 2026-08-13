package com.modernlink;

public final class LegacyHttpRequestRedirectTest {
    public static void main(String[] args) {
        LegacyHttpRequest request = new LegacyHttpRequest("https://example.com")
            .followRedirects(false)
            .maxRedirects(3);
        if (request.getFollowRedirects()) throw new AssertionError("redirects should be disabled");
        if (request.getMaxRedirects() != 3) throw new AssertionError("redirect limit was not retained");
        boolean rejected = false;
        try {
            request.maxRedirects(-1);
        } catch (IllegalArgumentException expected) {
            rejected = true;
        }
        if (!rejected) throw new AssertionError("negative redirect limit was accepted");
    }
}
