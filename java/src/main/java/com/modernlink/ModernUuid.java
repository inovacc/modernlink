package com.modernlink;

/** Standalone UUID utilities for Java 6 applications. */
public final class ModernUuid {
    private ModernUuid() { }

    public static String v4() throws LegacyHttpException {
        NativeLoader.load();
        String value = nativeV4();
        if (value == null) throw new LegacyHttpException("native UUIDv4 unavailable");
        return value;
    }

    public static String v7() throws LegacyHttpException {
        NativeLoader.load();
        String value = nativeV7();
        if (value == null) throw new LegacyHttpException("native UUIDv7 unavailable");
        return value;
    }

    private static native String nativeV7();
    private static native String nativeV4();
}
