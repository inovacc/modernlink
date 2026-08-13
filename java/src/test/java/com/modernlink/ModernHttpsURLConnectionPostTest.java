package com.modernlink;

import java.io.InputStream;
import java.io.OutputStream;
import java.net.URL;

public final class ModernHttpsURLConnectionPostTest {
    public static void main(String[] args) throws Exception {
        ModernHttpsURLConnection connection = new ModernHttpsURLConnection(new URL("https://example.com"));
        connection.setDoOutput(true);
        connection.setRequestProperty("Content-Type", "application/octet-stream");
        OutputStream output = connection.getOutputStream();
        output.write(new byte[] {1, 2, 3});
        output.close();
        int status = connection.getResponseCode();
        if (status < 400 || status > 599) throw new AssertionError("expected an HTTP error response from example.com");
        InputStream error = connection.getErrorStream();
        if (error == null || error.read() == -1) throw new AssertionError("error stream is empty");
        System.out.println("adapter-post-status=" + status);
    }
}
