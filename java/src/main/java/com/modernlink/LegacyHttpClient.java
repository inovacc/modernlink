package com.modernlink;

import java.util.LinkedHashMap;
import java.util.Map;

public final class LegacyHttpClient {
    private static final boolean LOAD_NATIVE = Boolean.valueOf(System.getProperty("modernlink.loadNative", "true")).booleanValue();

    public LegacyHttpResponse execute(LegacyHttpRequest request) throws LegacyHttpException {
        if (request == null) throw new IllegalArgumentException("request is required");
        if (LOAD_NATIVE) NativeLoader.load();
        String payload = nativeGet(request.getUrl());
        if (payload == null) throw new LegacyHttpException("native request failed");
        return decode(payload);
    }

    private static native String nativeGet(String url);

    private LegacyHttpResponse decode(String payload) throws LegacyHttpException {
        int separator = payload.indexOf("\n\n");
        if (separator < 0) throw new LegacyHttpException("invalid native response");
        String[] lines = payload.substring(0, separator).split("\n");
        int status;
        try { status = Integer.parseInt(lines[0]); }
        catch (NumberFormatException error) { throw new LegacyHttpException("invalid native status"); }
        Map<String, String> headers = new LinkedHashMap<String, String>();
        int i;
        for (i = 1; i < lines.length; i++) {
            int equals = lines[i].indexOf('=');
            if (equals > 0) headers.put(lines[i].substring(0, equals), lines[i].substring(equals + 1));
        }
        byte[] body = decodeBase64(payload.substring(separator + 2));
        return new LegacyHttpResponse(status, headers, body, new LegacyTlsInfo(null, null));
    }

    private byte[] decodeBase64(String value) throws LegacyHttpException {
        int padding = 0;
        if (value.endsWith("=")) padding++;
        if (value.endsWith("==")) padding++;
        int outputLength = (value.length() * 6 / 8) - padding;
        byte[] output = new byte[outputLength];
        int accumulator = 0;
        int bits = 0;
        int index = 0;
        for (int i = 0; i < value.length(); i++) {
            char character = value.charAt(i);
            if (character == '=') break;
            int digit = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".indexOf(character);
            if (digit < 0) throw new LegacyHttpException("invalid native response body");
            accumulator = (accumulator << 6) | digit;
            bits += 6;
            if (bits >= 8) {
                bits -= 8;
                output[index++] = (byte) ((accumulator >> bits) & 0xff);
            }
        }
        return output;
    }
}
