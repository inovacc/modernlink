package com.modernlink;

public final class LegacyTlsInfo {
    private final String protocol;
    private final String cipherSuite;

    public LegacyTlsInfo(String protocol, String cipherSuite) {
        this.protocol = protocol;
        this.cipherSuite = cipherSuite;
    }

    public String getProtocol() { return protocol; }
    public String getCipherSuite() { return cipherSuite; }
}
