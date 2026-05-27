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
        new AntirotApiClient(this).acknowledge(alarm.id, action, minutes, message -> {});
        finish();
    }

    private void startSound() {
        try {
            player = MediaPlayer.create(this, NotificationHelper.alarmSound(this));
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
