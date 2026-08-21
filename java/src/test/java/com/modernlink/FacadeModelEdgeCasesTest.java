package com.modernlink;

import java.net.URL;
import java.util.LinkedHashMap;
import java.util.Map;

/**
 * Deterministic coverage for the Java-only HTTP facade models.
 *
 * Java 6 syntax only: this is a standalone main-style test with no test framework.
 */
public final class FacadeModelEdgeCasesTest {
    public static void main(String[] args) throws Exception {
        requestDefaultsValidationAndCopies();
        responseAndTlsMetadataAreCopied();
        urlConnectionPreConnectBranches();
        utilityNullArgumentsFailBeforeNativeLoading();
        System.out.println("facade-model-edge-cases=PASS");
    }

    private static void requestDefaultsValidationAndCopies() {
        expectIllegalArgument("null URL", new CheckedAction() {
            public void run() { new LegacyHttpRequest(null); }
        });
        expectIllegalArgument("empty URL", new CheckedAction() {
            public void run() { new LegacyHttpRequest("   "); }
        });
        expectIllegalArgument("non-HTTPS URL", new CheckedAction() {
            public void run() { new LegacyHttpRequest("http://example.com"); }
        });

        final LegacyHttpRequest request = new LegacyHttpRequest("https://example.com");
        require("https://example.com".equals(request.getUrl()), "URL was not retained");
        require("GET".equals(request.getMethod()), "GET was not the default method");
        require(request.getBody().length == 0, "default body was not empty");
        require(request.getConnectTimeoutMillis() == 0L, "connect timeout default changed");
        require(request.getReadTimeoutMillis() == 0L, "read timeout default changed");
        require(request.getFollowRedirects(), "redirects should be enabled by default");
        require(request.getMaxRedirects() == 10, "redirect limit default changed");
        require(request.getMinimumTlsVersion() == LegacyHttpRequest.TLS_1_2,
            "TLS 1.2 should be the default floor");

        expectIllegalArgument("null method", new CheckedAction() {
            public void run() { request.method(null); }
        });
        expectIllegalArgument("blank method", new CheckedAction() {
            public void run() { request.method(" "); }
        });
        request.method("post");
        require("POST".equals(request.getMethod()), "method was not normalized to upper case");

        expectIllegalArgument("null header name", new CheckedAction() {
            public void run() { request.header(null, "value"); }
        });
        expectIllegalArgument("null header value", new CheckedAction() {
            public void run() { request.header("X-Test", null); }
        });
        request.header("X-Test", "value");
        require("value".equals(request.getHeaders().get("X-Test")), "header was not retained");
        expectUnsupported("request headers are read-only", new CheckedAction() {
            public void run() { request.getHeaders().put("X-Other", "value"); }
        });

        final byte[] body = new byte[] {1, 2, 3};
        request.body(body);
        body[0] = 99;
        require(request.getBody()[0] == 1, "request body aliases caller input");
        byte[] returned = request.getBody();
        returned[1] = 88;
        require(request.getBody()[1] == 2, "request body getter exposes internal storage");

        expectIllegalArgument("null body", new CheckedAction() {
            public void run() { request.body(null); }
        });
        expectIllegalArgument("negative connect timeout", new CheckedAction() {
            public void run() { request.connectTimeoutMillis(-1L); }
        });
        expectIllegalArgument("negative read timeout", new CheckedAction() {
            public void run() { request.readTimeoutMillis(-1L); }
        });
        expectIllegalArgument("negative redirect limit", new CheckedAction() {
            public void run() { request.maxRedirects(-1); }
        });
        expectIllegalArgument("unsupported TLS version", new CheckedAction() {
            public void run() { request.minimumTlsVersion(11); }
        });
    }

    private static void responseAndTlsMetadataAreCopied() {
        Map<String, String> headers = new LinkedHashMap<String, String>();
        headers.put("Content-Type", "application/octet-stream");
        byte[] body = new byte[] {10, 20, 30};
        byte[][] chain = new byte[][] {new byte[] {1, 2}, null};
        LegacyTlsInfo tls = new LegacyTlsInfo("TLSv1.3", "TLS_AES_128_GCM_SHA256", chain);
        final LegacyHttpResponse response = new LegacyHttpResponse(
            "https://example.com/final", 204, "No Content", headers, body, tls);

        headers.put("Injected", "header");
        body[0] = 99;
        chain[0][0] = 99;
        require(response.getHeaders().size() == 1, "response headers alias constructor input");
        require(response.getBody()[0] == 10, "response body aliases constructor input");
        require(response.getTlsInfo().getPeerCertificateChain()[0][0] == 1,
            "TLS certificate chain aliases constructor input");
        require(response.getStatus() == 204, "response status was not retained");
        require("No Content".equals(response.getStatusMessage()), "status message was not retained");
        require(response.getHeader("content-type").equals("application/octet-stream"),
            "header lookup should be case-insensitive");
        require(response.getHeader(null) == null, "null header lookup should return null");
        require(response.getHeader("missing") == null, "missing header should return null");

        byte[] responseBody = response.getBody();
        responseBody[1] = 88;
        require(response.getBody()[1] == 20, "response body getter exposes internal storage");
        expectUnsupported("response headers are read-only", new CheckedAction() {
            public void run() { response.getHeaders().put("X", "y"); }
        });

        byte[][] returnedChain = response.getTlsInfo().getPeerCertificateChain();
        returnedChain[0][0] = 77;
        require(response.getTlsInfo().getPeerCertificateChain()[0][0] == 1,
            "TLS certificate getter exposes internal storage");
        LegacyTlsInfo noChain = new LegacyTlsInfo("TLSv1.2", "cipher");
        require(noChain.getPeerCertificateChain().length == 0, "default certificate chain was not empty");
        LegacyTlsInfo nullChain = new LegacyTlsInfo("TLSv1.2", "cipher", (byte[][]) null);
        require(nullChain.getPeerCertificateChain() == null, "null certificate chain was not preserved");
    }

    private static void urlConnectionPreConnectBranches() throws Exception {
        expectIllegalArgument("non-HTTPS connection URL", new CheckedAction() {
            public void run() throws Exception { new ModernHttpsURLConnection(new URL("http://example.com")); }
        });

        final ModernHttpsURLConnection connection = new ModernHttpsURLConnection(new URL("https://example.com"));
        require(connection.getMinimumTlsVersion() == LegacyHttpRequest.TLS_1_2,
            "connection TLS default changed");
        require(connection.getMaxRedirects() == 10, "connection redirect default changed");
        require(!connection.usingProxy(), "connection unexpectedly reports a proxy");
        require(connection.getFinalUrl() == null, "final URL exists before connecting");
        require(connection.getCipherSuite() == null, "cipher suite exists before connecting");
        require(connection.getErrorStream() == null, "error stream exists before connecting");
        require(connection.getLocalCertificates() == null, "local certificates should be unavailable");
        require(connection.getLocalPrincipal() == null, "local principal should be unavailable");
        expectIllegalArgument("connection TLS validation", new CheckedAction() {
            public void run() { connection.minimumTlsVersion(10); }
        });
        expectIllegalArgument("connection redirect validation", new CheckedAction() {
            public void run() { connection.maxRedirects(-1); }
        });
        connection.disconnect();
    }

    private static void utilityNullArgumentsFailBeforeNativeLoading() throws Exception {
        expectIllegalArgument("Base64 encode null", new CheckedAction() {
            public void run() throws Exception { ModernBase64.encode(null); }
        });
        expectIllegalArgument("Base64 decode null", new CheckedAction() {
            public void run() throws Exception { ModernBase64.decode(null); }
        });
        expectIllegalArgument("JSON object null", new CheckedAction() {
            public void run() throws Exception { ModernJson.object(null); }
        });
        expectIllegalArgument("JSON array null", new CheckedAction() {
            public void run() throws Exception { ModernJson.array(null); }
        });
        expectIllegalArgument("JSON decode null", new CheckedAction() {
            public void run() throws Exception { ModernJson.decode(null); }
        });
    }

    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private interface CheckedAction {
        void run() throws Exception;
    }

    private static void expectIllegalArgument(String name, CheckedAction action) {
        try {
            action.run();
            throw new AssertionError(name + " was accepted");
        } catch (IllegalArgumentException expected) {
            // expected
        } catch (Exception error) {
            throw new AssertionError(name + " failed with the wrong exception: " + error);
        }
    }

    private static void expectUnsupported(String name, CheckedAction action) {
        try {
            action.run();
            throw new AssertionError(name + " was mutable");
        } catch (UnsupportedOperationException expected) {
            // expected
        } catch (Exception error) {
            throw new AssertionError(name + " failed with the wrong exception: " + error);
        }
    }
}
