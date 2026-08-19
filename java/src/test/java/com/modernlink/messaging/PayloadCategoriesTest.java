package com.modernlink.messaging;

import com.modernlink.ModernUuid;
import java.util.Map;
import java.util.TreeMap;

/**
 * MSG-05 — payload categories beyond text must survive the JNI boundary.
 *
 * Before this, `messaging_message_frame` rejected every non-text payload with "Java 6
 * messaging facade currently supports text payloads only", so a JMS application sending
 * a BytesMessage or a MapMessage had nowhere to go. Every assertion below fails against
 * that code.
 *
 * Uses LEGACY_JMS so it needs no broker.
 *
 * Java 6 syntax only.
 */
public final class PayloadCategoriesTest {
    private static final String URL = "legacy-jms://in-process";
    private static final String SUBJECT = "modernlink.payload.queue";

    public static void main(String[] args) throws Exception {
        textStillRoundTripsUnchanged();
        arbitraryBytesSurviveTheBoundary();
        mapSurvivesTheBoundary();
        mapDelimitersDoNotCorruptTheMap();
        wrongAccessorRefusesRatherThanGuessing();
        objectAndStreamAreRefusedWithAReason();
        payloadsAreCopiedNotShared();
        System.out.println("payload-categories=PASS");
    }

    private static ModernMessagingClient open() throws Exception {
        return new ModernMessagingClient(URL, SUBJECT, ModernMessagingMode.TRANSPARENT,
            ModernMessagingProvider.LEGACY_JMS);
    }

    private static ModernMessage roundTrip(ModernPayload payload) throws Exception {
        ModernMessagingClient client = open();
        try {
            ModernMessage sent = new ModernMessage(ModernUuid.v7(), SUBJECT, payload,
                ModernTraceContext.create(), ModernAcknowledgementMode.AUTO);
            client.publish(sent);
            ModernReceivedMessage received = client.receive();
            if (received == null) {
                throw new IllegalStateException("nothing was received back");
            }
            return received.getMessage();
        } finally {
            client.close();
        }
    }

    /** The category that already worked must keep working, byte for byte. */
    private static void textStillRoundTripsUnchanged() throws Exception {
        ModernMessage received = roundTrip(ModernPayload.text("hello from Java 6"));
        require(received.getBody().getKind() == ModernPayloadKind.TEXT, "category must survive");
        require("hello from Java 6".equals(received.getPayload()), "text must survive intact");
    }

    /**
     * The reason the category field exists at all: these bytes are not valid UTF-8, so a
     * text-only boundary had to mangle or reject them.
     */
    private static void arbitraryBytesSurviveTheBoundary() throws Exception {
        byte[] raw = new byte[] { (byte) 0x00, (byte) 0xff, (byte) 0xfe, (byte) 0x41, (byte) 0x0a, (byte) 0x80 };
        ModernMessage received = roundTrip(ModernPayload.bytes(raw));
        require(received.getBody().getKind() == ModernPayloadKind.BYTES, "category must survive");
        byte[] back = received.getBody().asBytes();
        require(back.length == raw.length, "byte length changed in flight");
        for (int index = 0; index < raw.length; index++) {
            require(back[index] == raw[index], "byte " + index + " changed in flight");
        }
    }

    private static void mapSurvivesTheBoundary() throws Exception {
        TreeMap sent = new TreeMap();
        sent.put("alpha", "one");
        sent.put("beta", "two");
        ModernMessage received = roundTrip(ModernPayload.map(sent));
        require(received.getBody().getKind() == ModernPayloadKind.MAP, "category must survive");
        Map back = received.getBody().asMap();
        require(back.size() == 2, "map lost entries");
        require("one".equals(back.get("alpha")), "alpha changed in flight");
        require("two".equals(back.get("beta")), "beta changed in flight");
    }

    /**
     * Both halves of every pair are base64 so a key or value containing a delimiter
     * cannot forge a pair boundary. Without that this map decodes into a DIFFERENT map,
     * which is silent corruption rather than an error.
     *
     * Note the separator is ':' and not '=': '=' is base64 padding, so splitting on it
     * cut encoded keys in half. That bug was real and this shape of test caught it.
     */
    private static void mapDelimitersDoNotCorruptTheMap() throws Exception {
        TreeMap sent = new TreeMap();
        sent.put("key=with,delimiters", "value,with=both");
        sent.put("colon:key", "colon:value");
        sent.put("empty", "");
        ModernMessage received = roundTrip(ModernPayload.map(sent));
        Map back = received.getBody().asMap();
        require(back.size() == 3, "map lost entries: " + back.size());
        require("value,with=both".equals(back.get("key=with,delimiters")), "delimiter key corrupted");
        require("colon:value".equals(back.get("colon:key")), "colon key corrupted");
        require("".equals(back.get("empty")), "empty value corrupted");
    }

    /** Asking for the wrong shape must fail loudly, not return mojibake. */
    private static void wrongAccessorRefusesRatherThanGuessing() {
        ModernPayload bytes = ModernPayload.bytes(new byte[] { 1, 2, 3 });
        boolean refused = false;
        try {
            bytes.asText();
        } catch (IllegalStateException expected) {
            refused = true;
        }
        require(refused, "asText on a BYTES payload must refuse");
    }

    /**
     * OBJECT is the security-relevant one: reconstructing it means deserializing
     * broker-supplied bytes into Java objects, a remote-code-execution surface. The
     * refusal must name the risk so nobody helpfully enables it later.
     */
    private static void objectAndStreamAreRefusedWithAReason() {
        String object = ModernPayload.refusalReason(ModernPayloadKind.OBJECT);
        require(object.indexOf("remote-code-execution") >= 0, "OBJECT refusal must name the risk");
        String stream = ModernPayload.refusalReason(ModernPayloadKind.STREAM);
        require(stream.indexOf("typed field ordering") >= 0, "STREAM refusal must give the reason");

        boolean refused = false;
        try {
            ModernPayload.decode(ModernPayloadKind.OBJECT, new byte[] { (byte) 0xac, (byte) 0xed });
        } catch (IllegalArgumentException expected) {
            refused = true;
        }
        require(refused, "decoding an OBJECT payload must refuse");
    }

    /**
     * A shared array would let a caller mutate a payload after publishing it, producing a
     * message that does not match what the application believes it sent.
     */
    private static void payloadsAreCopiedNotShared() {
        byte[] raw = new byte[] { 1, 2, 3 };
        ModernPayload payload = ModernPayload.bytes(raw);
        raw[0] = 99;
        require(payload.asBytes()[0] == 1, "the payload must not alias the caller array");
        byte[] out = payload.asBytes();
        out[0] = 42;
        require(payload.asBytes()[0] == 1, "asBytes must not expose the internal array");
    }

    private static void require(boolean condition, String message) {
        if (!condition) {
            throw new IllegalStateException(message);
        }
    }
}
