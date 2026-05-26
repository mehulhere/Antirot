package com.mehulhere.antirot;

import java.time.Instant;

public final class IsoDates {
    private IsoDates() {}

    public static long parse(String value, long fallback) {
        try {
            if (value == null || value.trim().isEmpty()) {
                return fallback;
            }
            return Instant.parse(value).toEpochMilli();
        } catch (Exception ignored) {
            return fallback;
        }
    }

    public static String now() {
        return Instant.now().toString();
    }
}
