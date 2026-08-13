package com.modernlink;

public class LegacyHttpException extends Exception {
    private static final long serialVersionUID = 1L;

    public LegacyHttpException(String message) {
        super(message);
    }
}
