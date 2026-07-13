package com.mehulhere.antirot;

import org.json.JSONException;
import org.json.JSONObject;

import java.util.Arrays;
import java.util.HashSet;
import java.util.Set;

public class AlarmJob {
    public static final Set<String> KNOWN_KINDS = new HashSet<>(Arrays.asList(
            "normal_wake", "loud_wake", "routine_overdue", "session_overdue",
            "non_response", "session_alarm", "break_alarm", "wake_alarm", "idle_alarm", "test"
    ));
    public final String id;
    public final String kind;
    public final String seriesId;
    public final long generation;
    public final String deliveryToken;
    public final String severity;
    public final String title;
    public final String message;
    public final long fireAtMillis;
    public final boolean requiresAcknowledgement;

    public AlarmJob(
            String id,
            String kind,
            String seriesId,
            long generation,
            String deliveryToken,
            String severity,
            String title,
            String message,
            long fireAtMillis,
            boolean requiresAcknowledgement
    ) {
        this.id = id;
        this.kind = kind;
        this.seriesId = seriesId;
        this.generation = generation;
        this.deliveryToken = deliveryToken;
        this.severity = severity;
        this.title = title;
        this.message = message;
        this.fireAtMillis = fireAtMillis;
        this.requiresAcknowledgement = requiresAcknowledgement;
    }

    public AlarmJob(
            String id,
            String kind,
            String severity,
            String title,
            String message,
            long fireAtMillis,
            boolean requiresAcknowledgement
    ) {
        this(id, kind, id, 1L, null, severity, title, message, fireAtMillis, requiresAcknowledgement);
    }

    public static AlarmJob test(String severity) {
        boolean loud = "loud".equals(severity) || "urgent".equals(severity);
        return new AlarmJob(
                "local-test-" + System.currentTimeMillis(),
                "test",
                "local-test",
                1L,
                null,
                severity,
                loud ? "Antirot loud test" : "Antirot test",
                loud ? "Loud test. Enough disappearing." : "Normal alarm test. Wake up, champ.",
                System.currentTimeMillis() + 5_000L,
                true
        );
    }

    public static AlarmJob fromJson(JSONObject json) throws JSONException {
        return new AlarmJob(
                json.getString("id"),
                json.optString("kind", "test"),
                json.optString("seriesId", json.getString("id")),
                json.optLong("generation", 1L),
                json.optString("deliveryToken", null),
                json.optString("severity", "normal"),
                json.optString("title", "Antirot"),
                json.optString("message", "Come back. Enough vanishing."),
                IsoDates.parse(json.optString("fireAt"), System.currentTimeMillis() + 5_000L),
                json.optBoolean("requiresAcknowledgement", true)
        );
    }
}
