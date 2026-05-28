package com.mehulhere.antirot;

import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.content.Context;
import android.media.AudioAttributes;
import android.net.Uri;
import android.os.Build;

public final class NotificationHelper {
    public static final String NORMAL_CHANNEL_ID = "antirot_normal_alarm";
    public static final String LOUD_CHANNEL_ID = "antirot_loud_alarm";

    private NotificationHelper() {}

    public static void ensureChannels(Context context) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) {
            return;
        }
        NotificationManager manager = context.getSystemService(NotificationManager.class);
        if (manager == null) {
            return;
        }
        AudioAttributes attributes = new AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_ALARM)
                .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                .build();
        NotificationChannel normal = new NotificationChannel(
                NORMAL_CHANNEL_ID,
                "Antirot normal alarms",
                NotificationManager.IMPORTANCE_HIGH
        );
        normal.setDescription("Wake checks and normal Antirot alarms.");
        normal.setSound(bundledAlarmSound(context, "normal"), attributes);

        NotificationChannel loud = new NotificationChannel(
                LOUD_CHANNEL_ID,
                "Antirot loud alarms",
                NotificationManager.IMPORTANCE_HIGH
        );
        loud.setDescription("Strict escalation alarms for Antirot.");
        loud.setSound(bundledAlarmSound(context, "loud"), attributes);
        loud.enableVibration(true);

        manager.createNotificationChannel(normal);
        manager.createNotificationChannel(loud);
    }

    public static Uri alarmSound(Context context, String severity) {
        SettingsStore settings = new SettingsStore(context);
        String mode = settings.getAlarmSoundMode();
        if (SettingsStore.SOUND_NORMAL.equals(mode)) {
            return bundledAlarmSound(context, "normal");
        }
        if (SettingsStore.SOUND_LOUD.equals(mode)) {
            return bundledAlarmSound(context, "loud");
        }
        if (SettingsStore.SOUND_CUSTOM.equals(mode)) {
            String selectedUri = settings.getAlarmSoundUri();
            if (!selectedUri.isEmpty()) {
                return Uri.parse(selectedUri);
            }
        }
        return bundledAlarmSound(context, severity);
    }

    public static String soundLabel(Context context) {
        SettingsStore settings = new SettingsStore(context);
        String mode = settings.getAlarmSoundMode();
        if (SettingsStore.SOUND_NORMAL.equals(mode)) {
            return "Bundled normal";
        }
        if (SettingsStore.SOUND_LOUD.equals(mode)) {
            return "Bundled loud";
        }
        if (SettingsStore.SOUND_CUSTOM.equals(mode)) {
            return settings.getAlarmSoundUri().isEmpty() ? "Custom not selected" : "Custom selected";
        }
        return "Auto: normal + loud";
    }

    private static Uri bundledAlarmSound(Context context, String severity) {
        int resource = "normal".equals(severity) ? R.raw.antirot_normal : R.raw.antirot_loud;
        return Uri.parse("android.resource://" + context.getPackageName() + "/" + resource);
    }
}
