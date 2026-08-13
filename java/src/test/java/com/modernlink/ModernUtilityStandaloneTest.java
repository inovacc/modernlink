package com.modernlink;

public final class ModernUtilityStandaloneTest {
    public static void main(String[] args) throws Exception {
        String uuid = ModernUuid.v7();
        if (uuid.length() != 36 || uuid.charAt(14) != '7') throw new AssertionError("invalid UUIDv7");
        String uuid4 = ModernUuid.v4();
        if (uuid4.length() != 36 || uuid4.charAt(14) != '4') throw new AssertionError("invalid UUIDv4");
        if (!"bW9kZXJubGluaw==".equals(ModernBase64.encode("modernlink".getBytes("UTF-8")))) {
            throw new AssertionError("unexpected Base64 result");
        }
        if (!"modernlink".equals(new String(ModernBase64.decode("bW9kZXJubGluaw=="), "UTF-8"))) {
            throw new AssertionError("unexpected Base64 decode result");
        }
        String object = ModernJson.object(new String[] {"message", "hello \"legacy\"", "kind", "gateway"});
        if (object.indexOf("hello \\\"legacy\\\"") < 0) throw new AssertionError("JSON object was not escaped");
        String array = ModernJson.array(new String[] {"one", "two"});
        if (!"[\"one\",\"two\"]".equals(array)) throw new AssertionError("JSON array was not encoded");
        if (!"{\"message\":\"hello\"}".equals(ModernJson.decode("{ \"message\": \"hello\" }"))) {
            throw new AssertionError("JSON decode did not normalize input");
        }
        System.out.println("standalone-uuidv7=" + uuid);
        System.out.println("standalone-json=" + object);
    }
}
