package com.modernlink;

public final class LegacyTlsInfo {
    private final String protocol;
    private final String cipherSuite;
    private final byte[][] peerCertificateChain;

    public LegacyTlsInfo(String protocol, String cipherSuite) {
        this(protocol, cipherSuite, new byte[0][]);
    }

    public LegacyTlsInfo(String protocol, String cipherSuite, byte[][] peerCertificateChain) {
        this.protocol = protocol;
        this.cipherSuite = cipherSuite;
        this.peerCertificateChain = copy(peerCertificateChain);
    }

    public String getProtocol() { return protocol; }
    public String getCipherSuite() { return cipherSuite; }
    public byte[][] getPeerCertificateChain() { return copy(peerCertificateChain); }

    private byte[][] copy(byte[][] source) {
        if (source == null) return null;
        byte[][] result = new byte[source.length][];
        for (int i = 0; i < source.length; i++) {
            result[i] = source[i] == null ? null : source[i].clone();
        }
        return result;
    }
}
