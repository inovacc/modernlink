package com.modernlink;

import java.io.InputStream;
import java.net.URL;
import java.security.cert.Certificate;

public final class ModernHttpsURLConnectionTest {
    public static void main(String[] args) throws Exception {
        ModernHttpsURLConnection connection = new ModernHttpsURLConnection(new URL("https://example.com"))
            .minimumTlsVersion(LegacyHttpRequest.TLS_1_3)
            .maxRedirects(2);
        if (connection.getMaxRedirects() != 2) throw new AssertionError("redirect limit was not retained");
        boolean limitRejected = false;
        try {
            connection.maxRedirects(-1);
        } catch (IllegalArgumentException expected) {
            limitRejected = true;
        }
        if (!limitRejected) throw new AssertionError("negative redirect limit was accepted");
        connection.setInstanceFollowRedirects(false);
        if (connection.getInstanceFollowRedirects()) throw new AssertionError("redirect policy was not retained");
        boolean verifierRejected = false;
        try {
            connection.setHostnameVerifier(new javax.net.ssl.HostnameVerifier() {
                public boolean verify(String host, javax.net.ssl.SSLSession session) { return true; }
            });
        } catch (UnsupportedOperationException expected) {
            verifierRejected = true;
        }
        if (!verifierRejected) throw new AssertionError("custom hostname verifier was silently accepted");
        connection.setRequestProperty("X-ModernLink-Test", "adapter");
        if (!"GET".equals(connection.getRequestMethod())) throw new AssertionError("default method is not GET");
        InputStream input = connection.getInputStream();
        int bytes = 0;
        byte[] buffer = new byte[256];
        int read;
        while ((read = input.read(buffer)) != -1) bytes += read;
        input.close();
        if (connection.getResponseCode() != 200) throw new AssertionError("unexpected response code");
        if (!"OK".equals(connection.getResponseMessage())) throw new AssertionError("response message is unavailable");
        if (connection.getHeaderFieldKey(0) != null || connection.getHeaderField(0) == null
            || connection.getHeaderField(0).indexOf("HTTP/") != 0) {
            throw new AssertionError("HTTP status line is not exposed at indexed header zero");
        }
        if (!connection.getHeaderFields().containsKey(null)) throw new AssertionError("status line is missing from header fields");
        if (connection.getContentType() == null) throw new AssertionError("content type is unavailable");
        if (connection.getContentLength() < -1) throw new AssertionError("invalid content length");
        if (bytes == 0) throw new AssertionError("response body is empty");
        if (connection.getCipherSuite() == null) throw new AssertionError("cipher suite is unavailable");
        if (connection.getMinimumTlsVersion() != LegacyHttpRequest.TLS_1_3) throw new AssertionError("TLS policy was not retained");
        Certificate[] certificates = connection.getServerCertificates();
        if (certificates == null || certificates.length == 0) throw new AssertionError("server certificates are unavailable");
        if (!"https://example.com".equals(connection.getFinalUrl())) throw new AssertionError("final URL was not retained");
        connection.disconnect();
        System.out.println("adapter-status=" + connection.getResponseCode());
        System.out.println("adapter-body-bytes=" + bytes);
        System.out.println("adapter-cipher=" + connection.getCipherSuite());
        System.out.println("adapter-certificates=" + certificates.length);
    }
}
