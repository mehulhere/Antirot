package com.mehulhere.antirot;

import android.app.Notification;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

public class AlarmReceiver extends BroadcastReceiver {
    public static final String EXTRA_ID = "id";
    public static final String EXTRA_KIND = "kind";
    public static final String EXTRA_SERIES_ID = "seriesId";
    public static final String EXTRA_GENERATION = "generation";
    public static final String EXTRA_SEVERITY = "severity";
    public static final String EXTRA_TITLE = "title";
    public static final String EXTRA_MESSAGE = "message";
    public static final String EXTRA_FIRE_AT = "fireAt";

    @Override
    public void onReceive(Context context, Intent intent) {
        AlarmJob alarm = fromIntent(intent);
        Intent activityIntent = AlarmActivity.intent(context, alarm);
        activityIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        context.startActivity(activityIntent);

        PendingIntent fullScreen = PendingIntent.getActivity(
                context,
                alarm.id.hashCode(),
                activityIntent,
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE
        );
        String channel = "normal".equals(alarm.severity)
                ? NotificationHelper.NORMAL_CHANNEL_ID
                : NotificationHelper.LOUD_CHANNEL_ID;
        Notification notification = new Notification.Builder(context, channel)
                .setSmallIcon(android.R.drawable.ic_lock_idle_alarm)
                .setContentTitle(alarm.title)
                .setContentText(alarm.message)
                .setCategory(Notification.CATEGORY_ALARM)
                .setPriority(Notification.PRIORITY_MAX)
                .setOngoing(true)
                .setFullScreenIntent(fullScreen, true)
                .build();
        NotificationManager manager = context.getSystemService(NotificationManager.class);
        if (manager != null) {
            manager.notify(alarm.id.hashCode(), notification);
        }
    }

    public static Intent intent(Context context, AlarmJob alarm) {
        Intent intent = new Intent(context, AlarmReceiver.class);
        putAlarm(intent, alarm);
        return intent;
    }

    static void putAlarm(Intent intent, AlarmJob alarm) {
        intent.putExtra(EXTRA_ID, alarm.id);
        intent.putExtra(EXTRA_KIND, alarm.kind);
        intent.putExtra(EXTRA_SERIES_ID, alarm.seriesId);
        intent.putExtra(EXTRA_GENERATION, alarm.generation);
        intent.putExtra(EXTRA_SEVERITY, alarm.severity);
        intent.putExtra(EXTRA_TITLE, alarm.title);
        intent.putExtra(EXTRA_MESSAGE, alarm.message);
        intent.putExtra(EXTRA_FIRE_AT, alarm.fireAtMillis);
    }

    static AlarmJob fromIntent(Intent intent) {
        return new AlarmJob(
                intent.getStringExtra(EXTRA_ID),
                intent.getStringExtra(EXTRA_KIND),
                intent.getStringExtra(EXTRA_SERIES_ID),
                intent.getLongExtra(EXTRA_GENERATION, 1L),
                null,
                intent.getStringExtra(EXTRA_SEVERITY),
                intent.getStringExtra(EXTRA_TITLE),
                intent.getStringExtra(EXTRA_MESSAGE),
                intent.getLongExtra(EXTRA_FIRE_AT, System.currentTimeMillis()),
                true
        );
    }
}
