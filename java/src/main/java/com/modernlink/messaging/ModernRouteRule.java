package com.modernlink.messaging;

/**
 * One routing policy rule, evaluated in order with first match winning.
 *
 * A rule constrains on any combination of exact destination, destination prefix,
 * tenant, and a single header name/value pair. An unset constraint means "not
 * constrained". The mode and provider it selects must be mutually consistent:
 * TRANSPARENT requires LEGACY_JMS, and TRANSFORM/REDIRECT require a modern provider.
 * The native side rejects an inconsistent rule rather than quietly correcting it.
 *
 * Java 6 syntax only.
 */
public final class ModernRouteRule {
    private final String id;
    private String destination;
    private String destinationPrefix;
    private String tenant;
    private String headerName;
    private String headerValue;
    private final ModernMessagingMode mode;
    private final ModernMessagingProvider provider;
    private boolean allowed = true;

    public ModernRouteRule(String id, ModernMessagingMode mode, ModernMessagingProvider provider) {
        if (id == null || id.length() == 0) throw new IllegalArgumentException("rule id is required");
        if (mode == null || provider == null) throw new IllegalArgumentException("rule mode and provider are required");
        if (id.indexOf('|') >= 0 || id.indexOf('#') >= 0) {
            throw new IllegalArgumentException("rule id must not contain '|' or '#'");
        }
        this.id = id;
        this.mode = mode;
        this.provider = provider;
    }

    public ModernRouteRule destination(String value) { this.destination = check(value, "destination"); return this; }
    public ModernRouteRule destinationPrefix(String value) { this.destinationPrefix = check(value, "destinationPrefix"); return this; }
    public ModernRouteRule tenant(String value) { this.tenant = check(value, "tenant"); return this; }

    /** Both name and value are required together; the native side rejects one without the other. */
    public ModernRouteRule header(String name, String value) {
        if ((name == null) != (value == null)) {
            throw new IllegalArgumentException("header name and value must be set together");
        }
        this.headerName = check(name, "headerName");
        this.headerValue = check(value, "headerValue");
        return this;
    }

    /** A denied rule is still evaluated; it reports the route as not allowed. */
    public ModernRouteRule allowed(boolean value) { this.allowed = value; return this; }

    public String getId() { return id; }
    public ModernMessagingMode getMode() { return mode; }
    public ModernMessagingProvider getProvider() { return provider; }
    public boolean isAllowed() { return allowed; }

    /** Wire form consumed by the native boundary. Field order is part of the contract. */
    public String encode() {
        StringBuilder builder = new StringBuilder();
        builder.append(id).append('|');
        builder.append(nullToEmpty(destination)).append('|');
        builder.append(nullToEmpty(destinationPrefix)).append('|');
        builder.append(nullToEmpty(tenant)).append('|');
        builder.append(nullToEmpty(headerName)).append('|');
        builder.append(nullToEmpty(headerValue)).append('|');
        builder.append(mode.name()).append('|');
        builder.append(provider.name()).append('|');
        builder.append(allowed ? '1' : '0');
        return builder.toString();
    }

    private static String nullToEmpty(String value) { return value == null ? "" : value; }

    private static String check(String value, String field) {
        if (value != null && value.indexOf('|') >= 0) {
            throw new IllegalArgumentException(field + " must not contain '|'");
        }
        return value;
    }
}
