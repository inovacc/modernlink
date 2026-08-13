package com.modernlink;

public final class LegacyHttpsTest {
    public static void main(String[] args) throws Exception {
        LegacyHttpRequest request = new LegacyHttpRequest("https://example.com");
        LegacyHttpResponse response = new LegacyHttpClient().execute(request);
        if (response.getStatus() < 100 || response.getStatus() > 599) {
            throw new AssertionError("invalid HTTP status: " + response.getStatus());
        }
        if (response.getBody().length == 0) {
            throw new AssertionError("HTTPS response body is empty");
        }
        if (response.getTlsInfo().getPeerCertificateChain() == null
            || response.getTlsInfo().getPeerCertificateChain().length == 0) {
            throw new AssertionError("peer certificate chain is unavailable");
        }
        System.out.println("status=" + response.getStatus());
        System.out.println("body-bytes=" + response.getBody().length);
        System.out.println("peer-certs=" + response.getTlsInfo().getPeerCertificateChain().length);
    }
}
