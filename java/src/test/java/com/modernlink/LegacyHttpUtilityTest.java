package com.modernlink;

public final class LegacyHttpUtilityTest {
    public static void main(String[] args) throws Exception {
        LegacyHttpClient client = new LegacyHttpClient();
        String uuid = client.newUuidV7();
        if (uuid.length() != 36 || uuid.charAt(14) != '7') throw new AssertionError("invalid UUIDv7: " + uuid);
        if (!"bW9kZXJubGluaw==".equals(client.base64Encode("modernlink".getBytes("UTF-8")))) {
            throw new AssertionError("unexpected Base64 result");
        }
        String json = client.requestJson(new LegacyHttpRequest("https://example.com")
            .method("POST")
            .header("Content-Type", "application/json")
            .body("payload".getBytes("UTF-8")));
        if (json.indexOf("bodyBase64") < 0 || json.indexOf("cGF5bG9hZA==") < 0) {
            throw new AssertionError("request JSON does not contain encoded body");
        }
        System.out.println("uuidv7=" + uuid);
        System.out.println("json-bytes=" + json.length());
    }
}
