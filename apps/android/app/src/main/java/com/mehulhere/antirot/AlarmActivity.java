package com.mehulhere.antirot;

import android.app.NotificationManager;
import android.content.Context;
import android.content.Intent;
import android.media.MediaPlayer;
import android.os.Bundle;
import android.view.Gravity;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.widget.Toast;

import java.util.ArrayList;
import java.util.List;

public class AlarmActivity extends android.app.Activity {
    private MediaPlayer player;
    private AlarmJob alarm;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        getWindow().addFlags(
                WindowManager.LayoutParams.FLAG_SHOW_WHEN_LOCKED |
                        WindowManager.LayoutParams.FLAG_TURN_SCREEN_ON |
                        WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON
        );
        alarm = AlarmReceiver.fromIntent(getIntent());
        startSound();
        setContentView(buildView());
    }

    @Override
    protected void onDestroy() {
        stopSound();
        super.onDestroy();
    }

    private LinearLayout buildView() {
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setGravity(Gravity.CENTER);
        root.setPadding(48, 48, 48, 48);
        root.setBackgroundColor(0xFF101418);

        TextView title = new TextView(this);
        title.setText(alarm.title == null ? "Antirot" : alarm.title);
        title.setTextColor(0xFFF5F7FA);
        title.setTextSize(28);
        title.setGravity(Gravity.CENTER);
        root.addView(title);

        TextView message = new TextView(this);
        message.setText(alarm.message == null ? "Come back." : alarm.message);
        message.setTextColor(0xFFA7B0BA);
        message.setTextSize(18);
        message.setGravity(Gravity.CENTER);
        message.setPadding(0, 24, 0, 32);
        root.addView(message);

        root.addView(button("I'm awake", () -> finishWithAction("ack", 0)));
        root.addView(button("Snooze", () -> finishWithAction("snooze", 9)));
        root.addView(button("Need more time", () -> finishWithAction("snooze", 15)));
        return root;
    }

    private Button button(String text, Runnable action) {
        Button button = new Button(this);
        button.setText(text);
        button.setAllCaps(false);
        button.setOnClickListener(view -> action.run());
        return button;
    }

    private void finishWithAction(String action, int minutes) {
        stopSound();
        NotificationManager manager = getSystemService(NotificationManager.class);
        if (manager != null) {
            manager.cancel(alarm.id.hashCode());
        }
        new AntirotApiClient(this).acknowledge(alarm.id, action, minutes, message -> runOnUiThread(() -> {
            if (message.startsWith("🔴 FALLBACK:")) {
                Toast.makeText(this, "Alarm action failed; tap again to retry. " + message, Toast.LENGTH_LONG).show();
                startSound();
                return;
            }
            new AlarmScheduler(this).cancelSeries(java.util.Collections.singleton(alarm.seriesId));
            reconcileAfterAction();
        }));
    }

    private void reconcileAfterAction() {
        AntirotApiClient api = new AntirotApiClient(this);
        api.fetchPendingAlarms(new AntirotApiClient.AlarmCallback() {
            @Override
            public void onAlarms(List<AlarmJob> alarms, List<String> cancelledSeriesIds, List<String> cancelledAlarmIds) {
                AlarmScheduler scheduler = new AlarmScheduler(AlarmActivity.this);
                scheduler.cancelSeries(cancelledSeriesIds);
                for (String alarmId : cancelledAlarmIds) {
                    scheduler.cancelAlarm(alarmId);
                }
                List<AlarmJob> scheduled = new ArrayList<>();
                for (AlarmJob pending : alarms) {
                    AlarmScheduler.ScheduleResult result = scheduler.schedule(pending);
                    if (result.scheduled) {
                        scheduled.add(pending);
                    }
                }
                api.reconcileAlarms(scheduled, cancelledSeriesIds, result -> runOnUiThread(() -> {
                    if (result.startsWith("🔴 FALLBACK:")) {
                        Toast.makeText(AlarmActivity.this, "Alarm reconciliation will retry. " + result, Toast.LENGTH_LONG).show();
                        return;
                    }
                    finish();
                }));
            }

            @Override
            public void onResult(String message) {
                runOnUiThread(() -> Toast.makeText(
                        AlarmActivity.this,
                        "Alarm replacement will retry. " + message,
                        Toast.LENGTH_LONG
                ).show());
            }
        });
    }

    private void startSound() {
        try {
            player = MediaPlayer.create(this, NotificationHelper.alarmSound(this, alarm.severity));
            if (player != null) {
                player.setLooping(!"normal".equals(alarm.severity));
                player.start();
            }
        } catch (Exception error) {
            System.err.println("🔴 FALLBACK: alarm playback failed - Reason: " + error.getMessage() + " - Impact: visual alarm only");
        }
    }

    private void stopSound() {
        if (player != null) {
            player.stop();
            player.release();
            player = null;
        }
    }

    public static Intent intent(Context context, AlarmJob alarm) {
        Intent intent = new Intent(context, AlarmActivity.class);
        AlarmReceiver.putAlarm(intent, alarm);
        return intent;
    }
}
