package com.modernlink.messaging;

/**
 * MSG-04 — per-provider guarantees must be queryable from Java before traffic moves.
 *
 * Before this, delivery semantics existed only as prose in docs/BACKLOG.md. A Java 6
 * caller had no way to ask whether the provider it was about to select could honour
 * CLIENT acknowledgement or persist a message, so the "fail closed on unsupported
 * guarantees" rule in AGENTS.md was unenforceable from the side that needed it.
 *
 * Every assertion below fails against the pre-MSG-04 code, where neither
 * ModernMessagingClient.guaranteesFor nor the native entry point behind it existed.
 *
 * Needs no broker and no network: the table is static data compiled into the native
 * library, which is exactly why it can be consulted before connecting.
 *
 * Java 6 syntax only.
 */
public final class ProviderGuaranteesTest {

    public static void main(String[] args) throws Exception {
        everyProviderReturnsATable();
        natsCoreDoesNotClaimWhatItCannotDo();
        jetStreamOffersRealServerSideAcknowledgement();
        transactionsAreRefusedEverywhere();
        declaredIsNotProven();
        summaryLeaksNoConnectionDetail();
        System.out.println("provider-guarantees=PASS");
    }

    private static void everyProviderReturnsATable() throws Exception {
        ModernMessagingProvider[] providers = ModernMessagingProvider.values();
        if (providers.length == 0) {
            throw new IllegalStateException("no providers declared");
        }
        for (int index = 0; index < providers.length; index++) {
            ModernMessagingProvider provider = providers[index];
            ModernProviderGuarantees guarantees = ModernMessagingClient.guaranteesFor(provider);
            require(guarantees.getProvider() == provider,
                "the table must describe the provider it was asked about: " + provider);
            require(guarantees.getPersistence() != null, "persistence must be stated for " + provider);
            require(guarantees.getOrdering() != null, "ordering must be stated for " + provider);
            require(guarantees.getReplay() != null, "replay must be stated for " + provider);
        }
    }

    /**
     * Core NATS is fire-and-forget: no broker-side state, so an unacknowledged message is
     * simply gone. The table must say so, because the dangerous answer is not "no" - it
     * is a confident "yes" that silently degrades to at-most-once.
     */
    private static void natsCoreDoesNotClaimWhatItCannotDo() throws Exception {
        ModernProviderGuarantees nats = ModernMessagingClient.guaranteesFor(ModernMessagingProvider.NATS);
        require(nats.getPersistence() == ModernGuaranteeSupport.UNSUPPORTED,
            "core NATS cannot persist");
        require(nats.getServerSideAcknowledgement() == ModernGuaranteeSupport.UNSUPPORTED,
            "core NATS has no server-side ack state");
        require(!nats.supportsAcknowledgementMode(ModernAcknowledgementMode.CLIENT),
            "core NATS must not accept CLIENT acknowledgement");
        require(nats.supportsAcknowledgementMode(ModernAcknowledgementMode.AUTO),
            "AUTO is what core NATS actually offers");
    }

    private static void jetStreamOffersRealServerSideAcknowledgement() throws Exception {
        ModernProviderGuarantees jetStream =
            ModernMessagingClient.guaranteesFor(ModernMessagingProvider.NATS_JETSTREAM);
        require(jetStream.getServerSideAcknowledgement() == ModernGuaranteeSupport.VERIFIED,
            "JetStream uses an explicit ack policy and a broker-backed test has exercised it");
        require(jetStream.supportsAcknowledgementMode(ModernAcknowledgementMode.CLIENT),
            "JetStream must accept CLIENT acknowledgement");
    }

    /**
     * No transport in crates/messaging implements transactions. Until one does, every
     * provider must refuse TRANSACTED: accepting it and behaving as if AUTO is exactly
     * how a rollback silently becomes a commit.
     */
    private static void transactionsAreRefusedEverywhere() throws Exception {
        ModernMessagingProvider[] providers = ModernMessagingProvider.values();
        for (int index = 0; index < providers.length; index++) {
            ModernProviderGuarantees guarantees = ModernMessagingClient.guaranteesFor(providers[index]);
            require(guarantees.getTransactions() == ModernGuaranteeSupport.UNSUPPORTED,
                "no transport implements transactions yet: " + providers[index]);
            require(!guarantees.supportsAcknowledgementMode(ModernAcknowledgementMode.TRANSACTED),
                "TRANSACTED must be refused by " + providers[index]);
        }
    }

    /** A claim must never read as evidence. */
    private static void declaredIsNotProven() {
        require(ModernGuaranteeSupport.VERIFIED.isProven(), "VERIFIED is proven");
        require(!ModernGuaranteeSupport.DECLARED.isProven(), "DECLARED is a claim, not proof");
        require(!ModernGuaranteeSupport.UNSUPPORTED.isProven(), "UNSUPPORTED is not proof");
    }

    /**
     * AGENTS.md: never put credentials, payloads or message bodies in JMX attributes or
     * logs. The summary is capability metadata only, so it must stay safe to log.
     */
    private static void summaryLeaksNoConnectionDetail() throws Exception {
        String summary = ModernMessagingClient.guaranteesFor(ModernMessagingProvider.RABBITMQ).toString();
        require(summary.indexOf("amqp://") < 0, "summary must not contain an endpoint");
        require(summary.indexOf("guest") < 0, "summary must not contain a credential");
        require(summary.indexOf("RABBITMQ") >= 0, "summary must name the provider");
        require(summary.indexOf("persistence=") >= 0, "summary must state persistence");
    }

    private static void require(boolean condition, String message) {
        if (!condition) {
            throw new IllegalStateException(message);
        }
    }
}
