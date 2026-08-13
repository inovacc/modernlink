package com.modernlink;

/** Standalone Base64 utilities for Java 6 applications. */
public final class ModernBase64 {
    private ModernBase64() { }

    public static String encode(byte[] value) throws LegacyHttpException {
        if (value == null) throw new IllegalArgumentException("value is required");
        NativeLoader.load();
        String encoded = nativeEncode(value.clone());
        if (encoded == null) throw new LegacyHttpException("native Base64 encoding unavailable");
        return encoded;
    }

    public static byte[] decode(String value) throws LegacyHttpException {
        if (value == null) throw new IllegalArgumentException("value is required");
        NativeLoader.load();
        byte[] decoded = nativeDecode(value);
        if (decoded == null) throw new LegacyHttpException("native Base64 decoding unavailable");
        return decoded;
    }

    private static native String nativeEncode(byte[] value);
    private static native byte[] nativeDecode(String value);
}
