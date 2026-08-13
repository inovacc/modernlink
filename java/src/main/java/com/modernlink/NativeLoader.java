package com.modernlink;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;

final class NativeLoader {
    private static boolean loaded;

    private NativeLoader() { }

    static synchronized void load() throws LegacyHttpException {
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
        File extracted = new File(System.getProperty("java.io.tmpdir"), "modernlink-" + resource.hashCode() + "-" + filename);
        try {
            FileOutputStream output = new FileOutputStream(extracted);
            try {
                byte[] buffer = new byte[8192];
                int read;
                while ((read = input.read(buffer)) != -1) output.write(buffer, 0, read);
            } finally {
                output.close();
                input.close();
            }
            System.load(extracted.getAbsolutePath());
            loaded = true;
        } catch (IOException error) {
            throw new LegacyHttpException("native extraction failed: " + error.getMessage());
        } catch (UnsatisfiedLinkError error) {
            throw new LegacyHttpException("native load failed: " + error.getMessage());
        }
    }
}
