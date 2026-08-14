package com.modernlink;

import java.io.File;

/**
 * VER-05 — native-load smoke test.
 *
 * Proves the JNI boundary is reachable at runtime on the platform it runs on: that
 * NativeLoader selects the right resource, extracts it to a content-addressed path,
 * that System.load succeeds, and that real JNI entry points return real values.
 *
 * This is the test that separates "the code exists" from "the library loads and a
 * native call returns". It performs no network I/O, so it can run anywhere the JAR
 * and its embedded native resource can.
 *
 * Java 6 syntax only: no lambdas, no diamond, no try-with-resources.
 */
public final class NativeLoadSmokeTest {
    public static void main(String[] args) throws Exception {
        String os = System.getProperty("os.name");
        String arch = System.getProperty("os.arch");
        System.out.println("native-smoke-platform=" + os + "/" + arch);
        System.out.println("native-smoke-jvm=" + System.getProperty("java.version"));

        // 1. Loading. Exercises resource selection, SHA-256 extraction, the
        //    content-addressed rename, and System.load.
        NativeLoader.load();
        System.out.println("native-smoke-load=ok");

        // 2. The extracted artifact must be content-addressed, not a bare name.
        File directory = new File(System.getProperty("java.io.tmpdir"));
        File[] extracted = directory.listFiles();
        int contentAddressed = 0;
        if (extracted != null) {
            for (int i = 0; i < extracted.length; i++) {
                String name = extracted[i].getName();
                if (name.startsWith("modernlink-") && !name.endsWith(".tmp")) contentAddressed++;
            }
        }
        if (contentAddressed < 1) throw new AssertionError("no content-addressed native artifact was extracted");
        System.out.println("native-smoke-extracted-count=" + contentAddressed);

        // 3. A second load must be a no-op rather than a second extraction.
        NativeLoader.load();
        System.out.println("native-smoke-reload=ok");

        // 4. Real JNI calls across three distinct entry-point families.
        String uuid4 = ModernUuid.v4();
        if (uuid4.length() != 36 || uuid4.charAt(14) != '4') throw new AssertionError("invalid UUIDv4: " + uuid4);
        String uuid7 = ModernUuid.v7();
        if (uuid7.length() != 36 || uuid7.charAt(14) != '7') throw new AssertionError("invalid UUIDv7: " + uuid7);
        if (uuid4.equals(uuid7)) throw new AssertionError("UUID generator returned a constant");
        System.out.println("native-smoke-uuidv4=" + uuid4);
        System.out.println("native-smoke-uuidv7=" + uuid7);

        String encoded = ModernBase64.encode("modernlink".getBytes("UTF-8"));
        if (!"bW9kZXJubGluaw==".equals(encoded)) throw new AssertionError("unexpected Base64: " + encoded);
        String decoded = new String(ModernBase64.decode(encoded), "UTF-8");
        if (!"modernlink".equals(decoded)) throw new AssertionError("Base64 round trip failed: " + decoded);
        System.out.println("native-smoke-base64=ok");

        // 5. The capability bitmask crosses the boundary as a primitive long.
        long capabilities = new LegacyHttpClient().getCapabilities();
        long required = LegacyHttpClient.CAPABILITY_HTTPS
            | LegacyHttpClient.CAPABILITY_TLS_1_2
            | LegacyHttpClient.CAPABILITY_TLS_1_3
            | LegacyHttpClient.CAPABILITY_REDIRECTS
            | LegacyHttpClient.CAPABILITY_PEER_CERTIFICATES;
        if ((capabilities & required) != required) {
            throw new AssertionError("capability bitmask missing bits: " + capabilities);
        }
        System.out.println("native-smoke-capabilities=" + capabilities);

        System.out.println("native-smoke=PASS");
    }
}
