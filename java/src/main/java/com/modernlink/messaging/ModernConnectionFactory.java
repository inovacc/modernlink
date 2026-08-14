package com.modernlink.messaging;

import com.modernlink.LegacyHttpException;

/** Java 6-compatible connection-factory entry point for the messaging facade. */
public final class ModernConnectionFactory {
    private final String url;
    private final String subject;
    private final ModernMessagingMode mode;
    private final ModernMessagingProvider provider;

    private final ModernRouteRule[] rules;

    public ModernConnectionFactory(String url, String subject, ModernMessagingMode mode,
        ModernMessagingProvider provider) {
        this(url, subject, mode, provider, null);
    }

    /**
     * Build a factory that applies a routing policy to every connection it creates.
     * Rules are evaluated in order, first match wins; null or empty means no policy.
     */
    public ModernConnectionFactory(String url, String subject, ModernMessagingMode mode,
        ModernMessagingProvider provider, ModernRouteRule[] rules) {
        if (url == null || subject == null || mode == null || provider == null) {
            throw new IllegalArgumentException("connection factory fields are required");
        }
        this.url = url;
        this.subject = subject;
        this.mode = mode;
        this.provider = provider;
        if (rules == null) {
            this.rules = null;
        } else {
            // Defensive copy: the caller must not be able to mutate the policy after
            // the factory is built.
            this.rules = new ModernRouteRule[rules.length];
            System.arraycopy(rules, 0, this.rules, 0, rules.length);
        }
    }

    public ModernConnection createConnection() throws LegacyHttpException {
        ModernMessagingClient client = new ModernMessagingClient(url, subject, mode, provider, rules);
        try {
            return new ModernConnection(client, mode, provider);
        } catch (RuntimeException error) {
            client.close();
            throw error;
        }
    }
}
