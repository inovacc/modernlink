package com.modernlink;

import java.util.LinkedHashMap;
import java.util.Map;

public final class LegacyHttpResponseStructuredTest {
    public static void main(String[] args) {
        Map<String, String> headers = new LinkedHashMap<String, String>();
        headers.put("content-type", "application/octet-stream");
        LegacyHttpResponse response = new LegacyHttpResponse("https://example.com", 200, headers, new byte[] {0, 1, 2}, null);
        if (!"https://example.com".equals(response.getFinalUrl())) throw new AssertionError("final URL was not retained");
        if (!"application/octet-stream".equals(response.getHeader("content-type"))) {
            throw new AssertionError("response header was not retained");
        }
        if (response.getBody()[0] != 0 || response.getBody()[2] != 2) {
            throw new AssertionError("binary response body was not retained");
        }
    }
}
