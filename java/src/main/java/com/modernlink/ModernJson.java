package com.modernlink;

/** Standalone JSON builders for Java 6 applications. */
public final class ModernJson {
    private ModernJson() { }

    public static String object(String[] fields) throws LegacyHttpException {
        if (fields == null) throw new IllegalArgumentException("fields are required");
        NativeLoader.load();
        String json = nativeObject(fields.clone());
        if (json == null) throw new LegacyHttpException("native JSON object encoding unavailable");
        return json;
    }

    public static String array(String[] values) throws LegacyHttpException {
        if (values == null) throw new IllegalArgumentException("values are required");
        NativeLoader.load();
        String json = nativeArray(values.clone());
        if (json == null) throw new LegacyHttpException("native JSON array encoding unavailable");
        return json;
    }

    /** Parses and returns normalized JSON text; Java 6 has no standard JSON object model. */
    public static String decode(String json) throws LegacyHttpException {
        if (json == null) throw new IllegalArgumentException("json is required");
        NativeLoader.load();
        String normalized = nativeDecode(json);
        if (normalized == null) throw new LegacyHttpException("invalid JSON");
        return normalized;
    }

    private static native String nativeObject(String[] fields);
    private static native String nativeArray(String[] values);
    private static native String nativeDecode(String json);
}
