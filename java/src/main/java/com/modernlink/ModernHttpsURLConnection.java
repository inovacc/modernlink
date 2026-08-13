package com.modernlink;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.ProtocolException;
import java.net.URL;
import java.security.Principal;
import java.security.cert.Certificate;
import java.security.cert.CertificateException;
import java.security.cert.CertificateFactory;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import javax.net.ssl.HttpsURLConnection;
import javax.net.ssl.HostnameVerifier;
import javax.net.ssl.SSLPeerUnverifiedException;
import javax.net.ssl.SSLSocketFactory;

/** Java 6-compatible HttpsURLConnection-shaped facade over ModernLink. */
public final class ModernHttpsURLConnection extends HttpsURLConnection {
    private final LegacyHttpClient client = new LegacyHttpClient();
    private final ByteArrayOutputStream requestBody = new ByteArrayOutputStream();
    private LegacyHttpResponse response;
    private int minimumTlsVersion = LegacyHttpRequest.TLS_1_2;
    private int maxRedirects = 10;

    public ModernHttpsURLConnection(URL url) {
        super(url);
        if (url == null || !"https".equalsIgnoreCase(url.getProtocol())) {
            throw new IllegalArgumentException("an https URL is required");
        }
    }

    public String getFinalUrl() {
        return response == null ? null : response.getFinalUrl();
    }

    public ModernHttpsURLConnection minimumTlsVersion(int version) {
        if (connected) throw new IllegalStateException("cannot change TLS version after connect");
        if (version != LegacyHttpRequest.TLS_1_2 && version != LegacyHttpRequest.TLS_1_3) {
            throw new IllegalArgumentException("minimum TLS version must be TLS_1_2 or TLS_1_3");
        }
        minimumTlsVersion = version;
        return this;
    }

    public int getMinimumTlsVersion() { return minimumTlsVersion; }

    public ModernHttpsURLConnection maxRedirects(int max) {
        if (connected) throw new IllegalStateException("cannot change redirect limit after connect");
        if (max < 0) throw new IllegalArgumentException("maximum redirects must not be negative");
        maxRedirects = max;
        return this;
    }

    public int getMaxRedirects() { return maxRedirects; }

    public void connect() throws IOException {
        if (connected) return;
        LegacyHttpRequest request = new LegacyHttpRequest(url.toExternalForm())
            .method(getRequestMethod())
            .connectTimeoutMillis(getConnectTimeout())
            .readTimeoutMillis(getReadTimeout())
            .followRedirects(getInstanceFollowRedirects())
            .maxRedirects(maxRedirects)
            .minimumTlsVersion(minimumTlsVersion);
        Map<String, List<String>> properties = getRequestProperties();
        for (Map.Entry<String, List<String>> entry : properties.entrySet()) {
            List<String> values = entry.getValue();
            if (values != null && !values.isEmpty()) request.header(entry.getKey(), values.get(0));
        }
        byte[] body = requestBody.toByteArray();
        if (getDoOutput() && "GET".equals(getRequestMethod())) {
            request.method("POST");
        }
        if (body.length != 0) request.body(body);
        try {
            response = client.execute(request);
        } catch (LegacyHttpException error) {
            IOException exception = new IOException(error.toString());
            exception.initCause(error);
            throw exception;
        }
        connected = true;
    }

    public void disconnect() {
        response = null;
        connected = false;
    }

    public boolean usingProxy() { return false; }

    public void setHostnameVerifier(HostnameVerifier verifier) {
        if (verifier != getDefaultHostnameVerifier()) {
            throw new UnsupportedOperationException("Java hostname verifiers are not supported; Rust hostname verification is mandatory");
        }
        super.setHostnameVerifier(verifier);
    }

    public void setSSLSocketFactory(SSLSocketFactory factory) {
        if (factory != getDefaultSSLSocketFactory()) {
            throw new UnsupportedOperationException("Java SSL socket factories are not supported; TLS is provided by Rust");
        }
        super.setSSLSocketFactory(factory);
    }

    public OutputStream getOutputStream() throws IOException {
        if (connected) throw new ProtocolException("cannot write after connect");
        doOutput = true;
        return requestBody;
    }

    public InputStream getInputStream() throws IOException {
        connect();
        if (response.getStatus() >= 400) throw new FileNotFoundException("HTTP status " + response.getStatus());
        return new ByteArrayInputStream(response.getBody());
    }

    public InputStream getErrorStream() {
        if (!connected || response == null || response.getStatus() < 400) return null;
        return new ByteArrayInputStream(response.getBody());
    }

    public int getResponseCode() throws IOException {
        connect();
        return response.getStatus();
    }

    public String getResponseMessage() throws IOException {
        connect();
        return response.getStatusMessage();
    }

    public String getHeaderField(String name) {
        if (response == null) return null;
        return response.getHeader(name);
    }

    public Map<String, List<String>> getHeaderFields() {
        if (response == null) return Collections.emptyMap();
        Map<String, List<String>> result = new LinkedHashMap<String, List<String>>();
        for (Map.Entry<String, String> entry : response.getHeaders().entrySet()) {
            result.put(entry.getKey(), Collections.singletonList(entry.getValue()));
        }
        return Collections.unmodifiableMap(result);
    }

    public String getCipherSuite() {
        return response == null || response.getTlsInfo() == null ? null : response.getTlsInfo().getCipherSuite();
    }

    public Certificate[] getServerCertificates() throws SSLPeerUnverifiedException {
        if (response == null || response.getTlsInfo() == null) throw new SSLPeerUnverifiedException("TLS response is unavailable");
        byte[][] encoded = response.getTlsInfo().getPeerCertificateChain();
        List<Certificate> certificates = new ArrayList<Certificate>(encoded.length);
        try {
            CertificateFactory factory = CertificateFactory.getInstance("X.509");
            for (int i = 0; i < encoded.length; i++) {
                certificates.add(factory.generateCertificate(new ByteArrayInputStream(encoded[i])));
            }
        } catch (CertificateException error) {
            SSLPeerUnverifiedException exception = new SSLPeerUnverifiedException(error.toString());
            exception.initCause(error);
            throw exception;
        }
        return certificates.toArray(new Certificate[certificates.size()]);
    }

    public Certificate[] getLocalCertificates() { return null; }

    public Principal getPeerPrincipal() throws SSLPeerUnverifiedException {
        Certificate[] certificates = getServerCertificates();
        if (certificates.length == 0) throw new SSLPeerUnverifiedException("server certificate chain is empty");
        return ((java.security.cert.X509Certificate) certificates[0]).getSubjectDN();
    }

    public Principal getLocalPrincipal() { return null; }
}
