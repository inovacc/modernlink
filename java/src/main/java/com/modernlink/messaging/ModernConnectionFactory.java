package com.modernlink.messaging;

import com.modernlink.LegacyHttpException;

/** Java 6-compatible connection-factory entry point for the messaging facade. */
public final class ModernConnectionFactory {
    private final String url;
    private final String subject;
    private final ModernMessagingMode mode;
    private final ModernMessagingProvider provider;

    public ModernConnectionFactory(String url, String subject, ModernMessagingMode mode,
        ModernMessagingProvider provider) {
        if (url == null || subject == null || mode == null || provider == null) {
            throw new IllegalArgumentException("connection factory fields are required");
        }
        this.url = url;
        this.subject = subject;
        this.mode = mode;
        this.provider = provider;
    }

    public ModernConnection createConnection() throws LegacyHttpException {
        ModernMessagingClient client = new ModernMessagingClient(url, subject, mode, provider);
        try {
            return new ModernConnection(client, mode, provider);
        } catch (RuntimeException error) {
            client.close();
            throw error;
        }
    }
}
