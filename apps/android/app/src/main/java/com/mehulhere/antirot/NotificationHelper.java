package com.mehulhere.antirot;

import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.content.Context;
import android.media.AudioAttributes;
import android.net.Uri;
import android.os.Build;
import android.provider.Settings;

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
        normal.setSound(Settings.System.DEFAULT_ALARM_ALERT_URI, attributes);

        NotificationChannel loud = new NotificationChannel(
                LOUD_CHANNEL_ID,
                "Antirot loud alarms",
                NotificationManager.IMPORTANCE_HIGH
        );
        loud.setDescription("Strict escalation alarms for Antirot.");
        loud.setSound(Settings.System.DEFAULT_ALARM_ALERT_URI, attributes);
        loud.enableVibration(true);

        manager.createNotificationChannel(normal);
        manager.createNotificationChannel(loud);
    }

    public static Uri alarmSound(Context context) {
        String selectedUri = new SettingsStore(context).getAlarmSoundUri();
        if (!selectedUri.isEmpty()) {
            return Uri.parse(selectedUri);
        }
        return Settings.System.DEFAULT_ALARM_ALERT_URI;
    }
}
