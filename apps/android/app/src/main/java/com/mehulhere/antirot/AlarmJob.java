package com.mehulhere.antirot;

import org.json.JSONException;
import org.json.JSONObject;

public class AlarmJob {
    public final String id;
    public final String kind;
    public final String severity;
    public final String title;
    public final String message;
    public final long fireAtMillis;
    public final boolean requiresAcknowledgement;

    public AlarmJob(
            String id,
            String kind,
            String severity,
            String title,
            String message,
            long fireAtMillis,
            boolean requiresAcknowledgement
    ) {
        this.id = id;
        this.kind = kind;
        this.severity = severity;
        this.title = title;
        this.message = message;
        this.fireAtMillis = fireAtMillis;
        this.requiresAcknowledgement = requiresAcknowledgement;
    }

    public static AlarmJob test(String severity) {
        boolean loud = "loud".equals(severity) || "urgent".equals(severity);
        return new AlarmJob(
                "local-test-" + System.currentTimeMillis(),
                "test",
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
                json.optString("severity", "normal"),
                json.optString("title", "Antirot"),
                json.optString("message", "Come back. Enough vanishing."),
                IsoDates.parse(json.optString("fireAt"), System.currentTimeMillis() + 5_000L),
                json.optBoolean("requiresAcknowledgement", true)
        );
    }
}
