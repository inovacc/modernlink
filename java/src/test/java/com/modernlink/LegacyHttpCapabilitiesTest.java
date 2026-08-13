package com.modernlink;

public final class LegacyHttpCapabilitiesTest {
    public static void main(String[] args) throws Exception {
        long capabilities = new LegacyHttpClient().getCapabilities();
        long required = LegacyHttpClient.CAPABILITY_HTTPS
            | LegacyHttpClient.CAPABILITY_TLS_1_2
            | LegacyHttpClient.CAPABILITY_TLS_1_3
            | LegacyHttpClient.CAPABILITY_REDIRECTS
            | LegacyHttpClient.CAPABILITY_PEER_CERTIFICATES;
        if ((capabilities & required) != required) {
            throw new AssertionError("required native capabilities are missing: " + capabilities);
        }
        System.out.println("capabilities=" + capabilities);
    }
}
