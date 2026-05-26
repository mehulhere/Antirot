package com.mehulhere.antirot;

import android.app.AlarmManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.os.Build;
import android.provider.Settings;

public class AlarmScheduler {
    private final Context context;
    private final AlarmManager alarmManager;

    public AlarmScheduler(Context context) {
        this.context = context.getApplicationContext();
        this.alarmManager = this.context.getSystemService(AlarmManager.class);
    }

    public String schedule(AlarmJob alarm) {
        if (alarmManager == null) {
            return "AlarmManager unavailable.";
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S && !alarmManager.canScheduleExactAlarms()) {
            context.startActivity(new Intent(Settings.ACTION_REQUEST_SCHEDULE_EXACT_ALARM).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
            return "Exact alarm permission needed. Opened settings.";
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
        return "Scheduled " + alarm.title;
    }
}
