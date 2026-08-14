package com.modernlink.messaging;

/**
 * The outcome of evaluating the routing policy for a message, without publishing it.
 *
 * A denied route is returned as a decision rather than thrown, so a caller can explain
 * why a message would not be delivered — which rule denied it — instead of only that
 * something failed.
 *
 * Java 6 syntax only.
 */
public final class ModernRouteDecision {
    private final ModernMessagingMode mode;
    private final ModernMessagingProvider provider;
    private final String ruleId;
    private final boolean allowed;

    public ModernRouteDecision(ModernMessagingMode mode, ModernMessagingProvider provider,
        String ruleId, boolean allowed) {
        if (mode == null || provider == null) throw new IllegalArgumentException("mode and provider are required");
        this.mode = mode;
        this.provider = provider;
        this.ruleId = ruleId;
        this.allowed = allowed;
    }

    public ModernMessagingMode getMode() { return mode; }
    public ModernMessagingProvider getProvider() { return provider; }

    /** The rule that matched, or null when the default route applied. */
    public String getRuleId() { return ruleId; }

    public boolean isAllowed() { return allowed; }

    public static ModernRouteDecision decode(String value) {
        String[] fields = value.split("#", -1);
        if (fields.length != 4) throw new IllegalArgumentException("invalid route decision: " + value);
        String ruleId = fields[2].length() == 0 ? null : fields[2];
        return new ModernRouteDecision(ModernMessagingMode.valueOf(fields[0]),
            ModernMessagingProvider.valueOf(fields[1]), ruleId, "1".equals(fields[3]));
    }

    public String toString() {
        return "ModernRouteDecision[mode=" + mode.name() + ",provider=" + provider.name()
            + ",ruleId=" + ruleId + ",allowed=" + allowed + "]";
    }
}
