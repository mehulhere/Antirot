package com.mehulhere.antirot;

import android.app.AlarmManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.os.Build;

public class AlarmScheduler {
    public static final class ScheduleResult {
        public final boolean scheduled;
        public final String message;

        ScheduleResult(boolean scheduled, String message) {
            this.scheduled = scheduled;
            this.message = message;
        }
    }

    private final Context context;
    private final AlarmManager alarmManager;
    private final SharedPreferences scheduled;

    public AlarmScheduler(Context context) {
        this.context = context.getApplicationContext();
        this.alarmManager = this.context.getSystemService(AlarmManager.class);
        this.scheduled = this.context.getSharedPreferences("scheduled_alarms", Context.MODE_PRIVATE);
    }

    public ScheduleResult schedule(AlarmJob alarm) {
        if (alarmManager == null) {
            return new ScheduleResult(false, "AlarmManager unavailable.");
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S && !alarmManager.canScheduleExactAlarms()) {
            return new ScheduleResult(false, "Exact alarm permission is required in the foreground.");
        }
        PendingIntent receiver = PendingIntent.getBroadcast(
                context,
                alarm.id.hashCode(),
                AlarmReceiver.intent(context, alarm),
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE
        );
        PendingIntent showIntent = PendingIntent.getActivity(
                context,
                alarm.id.hashCode(),
                AlarmActivity.intent(context, alarm),
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE
        );
        AlarmManager.AlarmClockInfo info = new AlarmManager.AlarmClockInfo(alarm.fireAtMillis, showIntent);
        alarmManager.setAlarmClock(info, receiver);
        scheduled.edit().putString("alarm:" + alarm.id, alarm.seriesId).apply();
        return new ScheduleResult(true, "Scheduled " + alarm.title);
    }

    public int cancelSeries(java.util.Collection<String> seriesIds) {
        int count = 0;
        for (java.util.Map.Entry<String, ?> entry : scheduled.getAll().entrySet()) {
            if (!entry.getKey().startsWith("alarm:") || !seriesIds.contains(String.valueOf(entry.getValue()))) {
                continue;
            }
            cancelAlarm(entry.getKey().substring("alarm:".length()));
            count += 1;
        }
        return count;
    }

    public void cancelAlarm(String alarmId) {
        if (alarmManager != null) {
            PendingIntent receiver = PendingIntent.getBroadcast(
                    context,
                    alarmId.hashCode(),
                    AlarmReceiver.intent(context, new AlarmJob(alarmId, "test", "normal", "", "", 0L, false)),
                    PendingIntent.FLAG_NO_CREATE | PendingIntent.FLAG_IMMUTABLE
            );
            if (receiver != null) {
                alarmManager.cancel(receiver);
                receiver.cancel();
            }
        }
        scheduled.edit().remove("alarm:" + alarmId).apply();
    }
}
