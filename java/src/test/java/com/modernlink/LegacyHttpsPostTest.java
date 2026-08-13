package com.modernlink;

public final class LegacyHttpsPostTest {
    public static void main(String[] args) throws Exception {
        LegacyHttpRequest request = new LegacyHttpRequest("https://example.com")
            .method("POST")
            .header("Content-Type", "application/octet-stream")
            .body(new byte[] {10, 20, 30})
            .connectTimeoutMillis(15000)
            .readTimeoutMillis(15000);
        LegacyHttpResponse response = new LegacyHttpClient().execute(request);
        if (response.getStatus() < 100 || response.getStatus() > 599) {
            throw new AssertionError("invalid HTTP status: " + response.getStatus());
        }
        if (response.getBody().length == 0) throw new AssertionError("POST response body is empty");
        System.out.println("post-status=" + response.getStatus());
        System.out.println("post-body-bytes=" + response.getBody().length);
    }
}
