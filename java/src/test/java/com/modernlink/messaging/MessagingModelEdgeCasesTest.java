package com.modernlink.messaging;

import java.util.HashMap;
import java.util.Map;

/**
 * Deterministic coverage for the Java 6 messaging facade and wire-model classes.
 *
 * Java 6 syntax only: standalone main-style test, no JUnit or external dependencies.
 */
public final class MessagingModelEdgeCasesTest {
    public static void main(String[] args) throws Exception {
        deliveryReceiptsRoundTripAndRejectMalformedFrames();
        routeRulesEncodeAndValidateConstraints();
        routeDecisionsDecodeAndValidateFields();
        guaranteeFramesAndEnumDecodersCoverAllBranches();
        metricsAndMessageWrappersValidateInputs();
        payloadDecodingAndAccessorGuards();
        System.out.println("messaging-model-edge-cases=PASS");
    }

    private static void deliveryReceiptsRoundTripAndRejectMalformedFrames() {
        ModernDeliveryReceipt receipt = new ModernDeliveryReceipt(
            "message-1", ModernMessagingProvider.RABBITMQ,
            ModernDeliveryState.ACKNOWLEDGED, "trace-1");
        require("message-1#RABBITMQ#ACKNOWLEDGED#trace-1".equals(receipt.encode()),
            "receipt wire encoding changed");
        ModernDeliveryReceipt decoded = ModernDeliveryReceipt.decode(receipt.encode());
        require("message-1".equals(decoded.getMessageId()), "receipt message id was not decoded");
        require(decoded.getProvider() == ModernMessagingProvider.RABBITMQ,
            "receipt provider was not decoded");
        require(decoded.getState() == ModernDeliveryState.ACKNOWLEDGED,
            "receipt state was not decoded");
        require("trace-1".equals(decoded.getTraceId()), "receipt trace id was not decoded");

        expectIllegalArgument("short receipt", new CheckedAction() {
            public void run() { ModernDeliveryReceipt.decode("message-1#RABBITMQ#PUBLISHED"); }
        });
        expectIllegalArgument("receipt with extra field", new CheckedAction() {
            public void run() { ModernDeliveryReceipt.decode("m#RABBITMQ#PUBLISHED#t#extra"); }
        });
        expectIllegalArgument("receipt with unknown provider", new CheckedAction() {
            public void run() { ModernDeliveryReceipt.decode("m#UNKNOWN#PUBLISHED#t"); }
        });
        expectIllegalArgument("receipt with unknown state", new CheckedAction() {
            public void run() { ModernDeliveryReceipt.decode("m#RABBITMQ#UNKNOWN#t"); }
        });
    }

    private static void routeRulesEncodeAndValidateConstraints() {
        ModernRouteRule rule = new ModernRouteRule(
            "rule-1", ModernMessagingMode.REDIRECT, ModernMessagingProvider.KAFKA)
            .destination("orders")
            .destinationPrefix("ord")
            .tenant("acme")
            .header("X-Tenant", "acme")
            .allowed(false);
        require("rule-1|orders|ord|acme|X-Tenant|acme|REDIRECT|KAFKA|0".equals(rule.encode()),
            "route rule wire encoding changed");
        require("rule-1".equals(rule.getId()), "route rule id was not retained");
        require(rule.getMode() == ModernMessagingMode.REDIRECT, "route rule mode was not retained");
        require(rule.getProvider() == ModernMessagingProvider.KAFKA, "route rule provider was not retained");
        require(!rule.isAllowed(), "denied rule was not retained");
        rule.header(null, null);
        require("rule-1|orders|ord|acme|||REDIRECT|KAFKA|0".equals(rule.encode()),
            "clearing a header did not clear both fields");

        expectIllegalArgument("null rule id", new CheckedAction() {
            public void run() { new ModernRouteRule(null, ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.LEGACY_JMS); }
        });
        expectIllegalArgument("null rule mode", new CheckedAction() {
            public void run() { new ModernRouteRule("id", null, ModernMessagingProvider.LEGACY_JMS); }
        });
        expectIllegalArgument("null rule provider", new CheckedAction() {
            public void run() { new ModernRouteRule("id", ModernMessagingMode.TRANSPARENT, null); }
        });
        expectIllegalArgument("pipe in rule id", new CheckedAction() {
            public void run() { new ModernRouteRule("bad|id", ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.LEGACY_JMS); }
        });
        expectIllegalArgument("hash in rule id", new CheckedAction() {
            public void run() { new ModernRouteRule("bad#id", ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.LEGACY_JMS); }
        });
        expectIllegalArgument("pipe in destination", new CheckedAction() {
            public void run() { new ModernRouteRule("id", ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.LEGACY_JMS).destination("bad|destination"); }
        });
        expectIllegalArgument("pipe in header name", new CheckedAction() {
            public void run() { new ModernRouteRule("id", ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.LEGACY_JMS).header("bad|name", "value"); }
        });
        expectIllegalArgument("half header", new CheckedAction() {
            public void run() { new ModernRouteRule("id", ModernMessagingMode.TRANSPARENT, ModernMessagingProvider.LEGACY_JMS).header("name", null); }
        });
    }

    private static void routeDecisionsDecodeAndValidateFields() {
        ModernRouteDecision defaultDecision = ModernRouteDecision.decode("TRANSFORM#KAFKA##1");
        require(defaultDecision.getMode() == ModernMessagingMode.TRANSFORM, "route mode was not decoded");
        require(defaultDecision.getProvider() == ModernMessagingProvider.KAFKA,
            "route provider was not decoded");
        require(defaultDecision.getRuleId() == null, "empty rule id should decode as null");
        require(defaultDecision.isAllowed(), "allowed route flag was not decoded");

        ModernRouteDecision denied = ModernRouteDecision.decode("REDIRECT#PULSAR#blocked#0");
        require("blocked".equals(denied.getRuleId()), "route rule id was not decoded");
        require(!denied.isAllowed(), "denied route flag was not decoded");
        require("ModernRouteDecision[mode=REDIRECT,provider=PULSAR,ruleId=blocked,allowed=false]"
            .equals(denied.toString()), "route decision summary changed");

        expectIllegalArgument("null route mode", new CheckedAction() {
            public void run() { new ModernRouteDecision(null, ModernMessagingProvider.KAFKA, null, true); }
        });
        expectIllegalArgument("null route provider", new CheckedAction() {
            public void run() { new ModernRouteDecision(ModernMessagingMode.TRANSFORM, null, null, true); }
        });
        expectIllegalArgument("short route decision", new CheckedAction() {
            public void run() { ModernRouteDecision.decode("TRANSFORM#KAFKA#only-three"); }
        });
        expectIllegalArgument("unknown route mode", new CheckedAction() {
            public void run() { ModernRouteDecision.decode("UNKNOWN#KAFKA#id#1"); }
        });
    }

    private static void guaranteeFramesAndEnumDecodersCoverAllBranches() {
        final ModernProviderGuarantees unsupported = ModernProviderGuarantees.decode(
            "KAFKA|DECLARED|VERIFIED|DECLARED|UNSUPPORTED|UNSUPPORTED|DECLARED|UNSUPPORTED|DECLARED|BLOCKS_INDEFINITELY");
        require(unsupported.getProvider() == ModernMessagingProvider.KAFKA, "provider frame field changed");
        require(unsupported.getOrdering() == ModernGuaranteeSupport.VERIFIED, "ordering frame field changed");
        require(unsupported.getClientAcknowledgement() == ModernGuaranteeSupport.UNSUPPORTED,
            "client acknowledgement frame field changed");
        require(unsupported.getRedelivery() == ModernGuaranteeSupport.DECLARED,
            "redelivery frame field changed");
        require(unsupported.getDeadLettering() == ModernGuaranteeSupport.UNSUPPORTED,
            "dead-letter frame field changed");
        require(unsupported.getReceiveSemantics() == ModernReceiveSemantics.BLOCKS_INDEFINITELY,
            "receive semantics frame field changed");
        require(unsupported.supportsAcknowledgementMode(ModernAcknowledgementMode.AUTO), "AUTO should be supported");
        require(unsupported.supportsAcknowledgementMode(ModernAcknowledgementMode.DUPLICATE_OK),
            "DUPLICATE_OK should be supported");
        require(!unsupported.supportsAcknowledgementMode(ModernAcknowledgementMode.CLIENT),
            "unsupported CLIENT acknowledgement was accepted");
        require(!unsupported.supportsAcknowledgementMode(ModernAcknowledgementMode.TRANSACTED),
            "unsupported transactions were accepted");

        ModernProviderGuarantees declared = ModernProviderGuarantees.decode(
            "PULSAR|DECLARED|DECLARED|DECLARED|DECLARED|VERIFIED|DECLARED|DECLARED|DECLARED|NON_BLOCKING");
        require(declared.supportsAcknowledgementMode(ModernAcknowledgementMode.CLIENT),
            "declared CLIENT acknowledgement should be selectable");
        require(declared.supportsAcknowledgementMode(ModernAcknowledgementMode.TRANSACTED),
            "verified transactions should be selectable");
        expectIllegalArgument("null acknowledgement mode", new CheckedAction() {
            public void run() { unsupported.supportsAcknowledgementMode(null); }
        });
        require(unsupported.toString().indexOf("receive=BLOCKS_INDEFINITELY") >= 0,
            "guarantee summary omitted receive semantics");

        require(ModernGuaranteeSupport.decode("VERIFIED") == ModernGuaranteeSupport.VERIFIED,
            "VERIFIED decoder branch missing");
        require(ModernGuaranteeSupport.decode("DECLARED") == ModernGuaranteeSupport.DECLARED,
            "DECLARED decoder branch missing");
        require(ModernGuaranteeSupport.decode("UNSUPPORTED") == ModernGuaranteeSupport.UNSUPPORTED,
            "UNSUPPORTED decoder branch missing");
        require(ModernReceiveSemantics.NON_BLOCKING.isSafeForPolling(), "non-blocking polling changed");
        require(!ModernReceiveSemantics.BLOCKS_INDEFINITELY.isSafeForPolling(),
            "blocking receive was marked safe for polling");
        require(ModernPayloadKind.decode("TEXT") == ModernPayloadKind.TEXT, "TEXT decoder branch missing");
        require(ModernPayloadKind.decode("BYTES") == ModernPayloadKind.BYTES, "BYTES decoder branch missing");
        require(ModernPayloadKind.decode("MAP") == ModernPayloadKind.MAP, "MAP decoder branch missing");
        require(ModernPayloadKind.decode("STREAM") == ModernPayloadKind.STREAM, "STREAM decoder branch missing");
        require(ModernPayloadKind.decode("OBJECT") == ModernPayloadKind.OBJECT, "OBJECT decoder branch missing");

        expectIllegalArgument("null guarantee frame", new CheckedAction() {
            public void run() { ModernProviderGuarantees.decode(null); }
        });
        expectIllegalArgument("short guarantee frame", new CheckedAction() {
            public void run() { ModernProviderGuarantees.decode("KAFKA|DECLARED"); }
        });
        expectIllegalArgument("unknown guarantee support", new CheckedAction() {
            public void run() { ModernGuaranteeSupport.decode("UNKNOWN"); }
        });
        expectIllegalArgument("unknown receive semantics", new CheckedAction() {
            public void run() { ModernReceiveSemantics.decode("UNKNOWN"); }
        });
        expectIllegalArgument("unknown payload kind", new CheckedAction() {
            public void run() { ModernPayloadKind.decode("UNKNOWN"); }
        });
        expectIllegalArgument("null guarantee support", new CheckedAction() {
            public void run() { ModernGuaranteeSupport.decode(null); }
        });
        expectIllegalArgument("null receive semantics", new CheckedAction() {
            public void run() { ModernReceiveSemantics.decode(null); }
        });
        expectIllegalArgument("null payload kind", new CheckedAction() {
            public void run() { ModernPayloadKind.decode(null); }
        });
    }

    private static void metricsAndMessageWrappersValidateInputs() throws Exception {
        ModernMessagingMetrics metrics = new ModernMessagingMetrics(
            ModernMessagingMode.TRANSFORM, ModernMessagingProvider.NATS);
        require("TRANSFORM".equals(metrics.getMode()), "metrics mode was not retained");
        require("NATS".equals(metrics.getProvider()), "metrics provider was not retained");
        require(metrics.getPublished() == 0L && metrics.getReceived() == 0L
            && metrics.getAcknowledged() == 0L, "metrics did not start at zero");
        require(metrics.getLastTraceId() == null, "metrics trace id did not start empty");
        metrics.published("trace-published");
        metrics.received("trace-received");
        metrics.acknowledged("trace-acknowledged");
        require(metrics.getPublished() == 1L, "published metric was not incremented");
        require(metrics.getReceived() == 1L, "received metric was not incremented");
        require(metrics.getAcknowledged() == 1L, "acknowledged metric was not incremented");
        require("trace-acknowledged".equals(metrics.getLastTraceId()), "last trace metric was not updated");

        expectIllegalArgument("null metrics mode", new CheckedAction() {
            public void run() { new ModernMessagingMetrics(null, ModernMessagingProvider.NATS); }
        });
        expectIllegalArgument("null metrics provider", new CheckedAction() {
            public void run() { new ModernMessagingMetrics(ModernMessagingMode.TRANSFORM, null); }
        });

        final ModernTraceContext tracing = new ModernTraceContext("trace", "span", "parent", "state", false);
        final ModernMessage message = new ModernMessage("message", "subject", "text", tracing,
            ModernAcknowledgementMode.CLIENT);
        require("text".equals(message.getPayload()), "text message payload was not retained");
        require(message.getAcknowledgementMode() == ModernAcknowledgementMode.CLIENT,
            "acknowledgement mode was not retained");
        require(!message.getTracing().isSampled(), "trace sampling flag was not retained");
        ModernMessage autoMessage = new ModernMessage("auto-message", "subject", "body", tracing);
        require(autoMessage.getAcknowledgementMode() == ModernAcknowledgementMode.AUTO,
            "default acknowledgement mode changed");
        ModernMessage autoDecoded = ModernMessage.decode(autoMessage.encode());
        require("auto-message".equals(autoDecoded.getMessageId()), "default message id did not round trip");
        require("subject".equals(autoDecoded.getDestination()), "default message destination did not round trip");
        require("body".equals(autoDecoded.getPayload()), "default message payload did not round trip");
        final ModernTextMessage textMessage = new ModernTextMessage(message);
        require("message".equals(textMessage.getJMSMessageID()), "JMS message id was not copied");
        require("subject".equals(textMessage.getJMSDestination()), "JMS destination was not copied");
        textMessage.setText("changed");
        require("changed".equals(textMessage.getText()), "text setter did not update the message");

        expectIllegalArgument("null message id", new CheckedAction() {
            public void run() { new ModernMessage(null, "subject", "text", tracing); }
        });
        expectIllegalArgument("null message payload", new CheckedAction() {
            public void run() { new ModernMessage("id", "subject", (String) null, tracing); }
        });
        expectIllegalArgument("null acknowledgement mode", new CheckedAction() {
            public void run() { new ModernMessage("id", "subject", "text", tracing, null); }
        });
        expectIllegalArgument("null text message", new CheckedAction() {
            public void run() { new ModernTextMessage(null, ModernAcknowledgementMode.AUTO); }
        });
        expectIllegalArgument("null text setter", new CheckedAction() {
            public void run() { textMessage.setText(null); }
        });
        expectIllegalArgument("null received message", new CheckedAction() {
            public void run() { new ModernReceivedMessage(null, new ModernDeliveryReceipt("id", ModernMessagingProvider.NATS, ModernDeliveryState.RECEIVED, "trace")); }
        });
        expectIllegalArgument("null received receipt", new CheckedAction() {
            public void run() { new ModernReceivedMessage(message, null); }
        });
    }

    private static void payloadDecodingAndAccessorGuards() throws Exception {
        ModernPayload text = ModernPayload.decode(ModernPayloadKind.TEXT, new byte[] {'o', 'k'});
        require(text.getKind() == ModernPayloadKind.TEXT, "decoded payload kind was not retained");
        require("ok".equals(text.asText()), "decoded text payload was not retained");
        expectIllegalState("asMap on text payload", new CheckedAction() {
            public void run() throws Exception { ModernPayload.text("text").asMap(); }
        });
        expectIllegalArgument("null payload kind", new CheckedAction() {
            public void run() { ModernPayload.decode(null, new byte[] {1}); }
        });
        expectIllegalArgument("null payload body", new CheckedAction() {
            public void run() { ModernPayload.decode(ModernPayloadKind.TEXT, null); }
        });
        expectIllegalArgument("stream payload decode", new CheckedAction() {
            public void run() { ModernPayload.decode(ModernPayloadKind.STREAM, new byte[] {1}); }
        });
        expectIllegalArgument("object payload decode", new CheckedAction() {
            public void run() { ModernPayload.decode(ModernPayloadKind.OBJECT, new byte[] {1}); }
        });
        expectIllegalArgument("null text payload", new CheckedAction() {
            public void run() { ModernPayload.text(null); }
        });
        expectIllegalArgument("null bytes payload", new CheckedAction() {
            public void run() { ModernPayload.bytes(null); }
        });
        expectIllegalArgument("null map payload", new CheckedAction() {
            public void run() throws Exception { ModernPayload.map(null); }
        });

        final Map nullValue = new HashMap();
        nullValue.put("key", null);
        expectIllegalArgument("null map value", new CheckedAction() {
            public void run() throws Exception { ModernPayload.map(nullValue); }
        });

        Map empty = ModernPayload.map(new HashMap()).asMap();
        require(empty.isEmpty(), "empty map payload did not round trip");
        expectIllegalArgument("map entry without separator", new CheckedAction() {
            public void run() throws Exception {
                ModernPayload.decode(ModernPayloadKind.MAP, new byte[] {'x'}).asMap();
            }
        });
    }

    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private interface CheckedAction {
        void run() throws Exception;
    }

    private static void expectIllegalArgument(String name, CheckedAction action) {
        try {
            action.run();
            throw new AssertionError(name + " was accepted");
        } catch (IllegalArgumentException expected) {
            // expected
        } catch (Exception error) {
            throw new AssertionError(name + " failed with the wrong exception: " + error);
        }
    }

    private static void expectIllegalState(String name, CheckedAction action) {
        try {
            action.run();
            throw new AssertionError(name + " was accepted");
        } catch (IllegalStateException expected) {
            // expected
        } catch (Exception error) {
            throw new AssertionError(name + " failed with the wrong exception: " + error);
        }
    }
}
