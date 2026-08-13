package com.modernlink;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;

public final class NativeLoader {
    private static boolean loaded;

    private NativeLoader() { }

    public static synchronized void load() throws LegacyHttpException {
        if (loaded) return;
        String os = System.getProperty("os.name").toLowerCase();
        String arch = System.getProperty("os.arch").toLowerCase();
        String resource;
        String filename;
        if (os.indexOf("win") >= 0 && (arch.equals("amd64") || arch.equals("x86_64"))) {
            resource = "/native/windows-x86_64/modernlink.dll";
            filename = "modernlink.dll";
        } else if (os.indexOf("linux") >= 0 && (arch.equals("amd64") || arch.equals("x86_64"))) {
            resource = "/native/linux-x86_64/libmodernlink.so";
            filename = "libmodernlink.so";
        } else if (os.indexOf("linux") >= 0 && (arch.equals("aarch64") || arch.equals("arm64"))) {
            resource = "/native/linux-aarch64/libmodernlink.so";
            filename = "libmodernlink.so";
        } else {
            throw new LegacyHttpException("unsupported native platform: " + os + "/" + arch);
        }
        InputStream input = NativeLoader.class.getResourceAsStream(resource);
        if (input == null) throw new LegacyHttpException("native resource not found: " + resource);
        File directory = new File(System.getProperty("java.io.tmpdir"));
        File temporary = null;
        try {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            temporary = File.createTempFile("modernlink-", ".tmp", directory);
            FileOutputStream output = new FileOutputStream(temporary);
            try {
                byte[] buffer = new byte[8192];
                int read;
                while ((read = input.read(buffer)) != -1) {
                    output.write(buffer, 0, read);
                    digest.update(buffer, 0, read);
                }
            } finally {
                output.close();
            }
            File extracted = new File(directory, "modernlink-" + hex(digest.digest()) + "-" + filename);
            if (extracted.isFile()) {
                if (!temporary.delete()) throw new IOException("native temporary file cleanup failed");
            } else if (!temporary.renameTo(extracted)) {
                if (!extracted.isFile()) throw new IOException("native extraction rename failed");
                if (!temporary.delete()) throw new IOException("native temporary file cleanup failed");
            }
            System.load(extracted.getAbsolutePath());
            loaded = true;
        } catch (NoSuchAlgorithmException error) {
            throw new LegacyHttpException("native digest unavailable: " + error.getMessage());
        } catch (IOException error) {
            throw new LegacyHttpException("native extraction failed: " + error.getMessage());
        } catch (UnsatisfiedLinkError error) {
            throw new LegacyHttpException("native load failed: " + error.getMessage());
        } finally {
            try {
                input.close();
            } catch (IOException ignored) {
                // Preserve the native-load error when cleanup also fails.
            }
            if (temporary != null && temporary.exists()) temporary.delete();
        }
    }

    private static String hex(byte[] value) {
        char[] digits = "0123456789abcdef".toCharArray();
        char[] result = new char[value.length * 2];
        for (int i = 0; i < value.length; i++) {
            int current = value[i] & 255;
            result[i * 2] = digits[current >>> 4];
            result[i * 2 + 1] = digits[current & 15];
        }
        return new String(result);
    }
}
