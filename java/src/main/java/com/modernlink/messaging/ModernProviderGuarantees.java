package com.modernlink.messaging;

/**
 * What one provider is declared to offer, queryable before any traffic moves (MSG-04).
 *
 * The table lives in Rust (`crates/messaging`, {@code Provider::guarantees}) and is
 * assembled from the transport implementations themselves, not from vendor
 * documentation. That matters: a guarantee reported here describes what this
 * compatibility layer actually does, which is often narrower than what the broker is
 * capable of.
 *
 * Read {@link ModernGuaranteeSupport#DECLARED} as "claimed, never tested". Most fields
 * are DECLARED today, because only a happy-path send/receive/acknowledge round trip has
 * ever been executed against a real broker.
 */
public final class ModernProviderGuarantees {
    private final ModernMessagingProvider provider;
    private final ModernGuaranteeSupport persistence;
    private final ModernGuaranteeSupport ordering;
    private final ModernGuaranteeSupport serverSideAcknowledgement;
    private final ModernGuaranteeSupport clientAcknowledgement;
    private final ModernGuaranteeSupport transactions;
    private final ModernGuaranteeSupport redelivery;
    private final ModernGuaranteeSupport deadLettering;
    private final ModernGuaranteeSupport replay;

    private ModernProviderGuarantees(ModernMessagingProvider provider,
        ModernGuaranteeSupport persistence, ModernGuaranteeSupport ordering,
        ModernGuaranteeSupport serverSideAcknowledgement, ModernGuaranteeSupport clientAcknowledgement,
        ModernGuaranteeSupport transactions, ModernGuaranteeSupport redelivery,
        ModernGuaranteeSupport deadLettering, ModernGuaranteeSupport replay) {
        this.provider = provider;
        this.persistence = persistence;
        this.ordering = ordering;
        this.serverSideAcknowledgement = serverSideAcknowledgement;
        this.clientAcknowledgement = clientAcknowledgement;
        this.transactions = transactions;
        this.redelivery = redelivery;
        this.deadLettering = deadLettering;
        this.replay = replay;
    }

    /** Decode the pipe-separated frame emitted by the native boundary. */
    static ModernProviderGuarantees decode(String frame) {
        if (frame == null) {
            throw new IllegalArgumentException("guarantee frame is required");
        }
        // -1 keeps trailing empty fields, so a truncated frame is caught by the length
        // check below instead of being silently padded.
        String[] parts = frame.split("\\|", -1);
        if (parts.length != 9) {
            throw new IllegalArgumentException("guarantee frame must have 9 fields, got " + parts.length);
        }
        return new ModernProviderGuarantees(
            ModernMessagingProvider.valueOf(parts[0]),
            ModernGuaranteeSupport.decode(parts[1]),
            ModernGuaranteeSupport.decode(parts[2]),
            ModernGuaranteeSupport.decode(parts[3]),
            ModernGuaranteeSupport.decode(parts[4]),
            ModernGuaranteeSupport.decode(parts[5]),
            ModernGuaranteeSupport.decode(parts[6]),
            ModernGuaranteeSupport.decode(parts[7]),
            ModernGuaranteeSupport.decode(parts[8]));
    }

    public ModernMessagingProvider getProvider() {
        return provider;
    }

    /** Messages survive a broker restart. */
    public ModernGuaranteeSupport getPersistence() {
        return persistence;
    }

    /** Delivery order is preserved within one destination or partition. */
    public ModernGuaranteeSupport getOrdering() {
        return ordering;
    }

    /**
     * The broker tracks acknowledgement state, so an unacknowledged message is
     * redelivered rather than lost along with the consumer.
     */
    public ModernGuaranteeSupport getServerSideAcknowledgement() {
        return serverSideAcknowledgement;
    }

    /** {@link ModernAcknowledgementMode#CLIENT} is honoured end to end. */
    public ModernGuaranteeSupport getClientAcknowledgement() {
        return clientAcknowledgement;
    }

    /** Transactional publish and consume. No transport implements this today. */
    public ModernGuaranteeSupport getTransactions() {
        return transactions;
    }

    /** An unacknowledged message is redelivered. */
    public ModernGuaranteeSupport getRedelivery() {
        return redelivery;
    }

    /** Poison messages can be diverted to a dead-letter destination. */
    public ModernGuaranteeSupport getDeadLettering() {
        return deadLettering;
    }

    /** Already-consumed messages can be re-read. */
    public ModernGuaranteeSupport getReplay() {
        return replay;
    }

    /**
     * Whether this provider can honour the given acknowledgement mode.
     *
     * Callers should check this before moving traffic. A provider that cannot honour a
     * mode will refuse it rather than downgrade it, so treating this as optional turns a
     * capability gap into a runtime failure instead of a deployment decision.
     */
    public boolean supportsAcknowledgementMode(ModernAcknowledgementMode mode) {
        if (mode == null) {
            throw new IllegalArgumentException("acknowledgement mode is required");
        }
        if (mode == ModernAcknowledgementMode.AUTO || mode == ModernAcknowledgementMode.DUPLICATE_OK) {
            return true;
        }
        if (mode == ModernAcknowledgementMode.CLIENT) {
            return clientAcknowledgement != ModernGuaranteeSupport.UNSUPPORTED;
        }
        return transactions != ModernGuaranteeSupport.UNSUPPORTED;
    }

    /**
     * A stable, log-safe summary. Contains only capability metadata: no credentials, no
     * endpoints, no payloads, so it is safe for JMX attributes and logs.
     */
    @Override
    public String toString() {
        StringBuilder builder = new StringBuilder();
        builder.append(provider.name());
        builder.append(" persistence=").append(persistence.name());
        builder.append(" ordering=").append(ordering.name());
        builder.append(" serverAck=").append(serverSideAcknowledgement.name());
        builder.append(" clientAck=").append(clientAcknowledgement.name());
        builder.append(" transactions=").append(transactions.name());
        builder.append(" redelivery=").append(redelivery.name());
        builder.append(" deadLettering=").append(deadLettering.name());
        builder.append(" replay=").append(replay.name());
        return builder.toString();
    }
}
