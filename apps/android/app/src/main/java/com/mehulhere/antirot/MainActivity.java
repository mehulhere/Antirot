package com.mehulhere.antirot;

import android.Manifest;
import android.app.AlarmManager;
import android.content.pm.PackageManager;
import android.os.Build;
import android.os.Bundle;
import android.view.Gravity;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import java.util.List;

public class MainActivity extends android.app.Activity {
    private SettingsStore settings;
    private TextView status;
    private EditText serverUrl;
    private EditText apiToken;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        settings = new SettingsStore(this);
        NotificationHelper.ensureChannels(this);
        setContentView(buildView());
        requestNotificationPermissionIfNeeded();
    }

    private ScrollView buildView() {
        ScrollView scroll = new ScrollView(this);
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(36, 48, 36, 48);
        root.setBackgroundColor(0xFF101418);
        scroll.addView(root);

        TextView title = new TextView(this);
        title.setText("Antirot");
        title.setTextColor(0xFFF5F7FA);
        title.setTextSize(32);
        title.setGravity(Gravity.CENTER_HORIZONTAL);
        root.addView(title);

        TextView subtitle = new TextView(this);
        subtitle.setText("Phone alarm client. The coach is on the VPS; this device makes noise.");
        subtitle.setTextColor(0xFFA7B0BA);
        subtitle.setPadding(0, 8, 0, 24);
        root.addView(subtitle);

        serverUrl = input("Antirot VPS URL", settings.getServerUrl());
        apiToken = input("API token", settings.getApiToken());
        root.addView(serverUrl);
        root.addView(apiToken);
        root.addView(button("Save settings", this::saveSettings));
        root.addView(button("Register device", this::registerDevice));
        root.addView(button("Schedule normal test alarm", () -> scheduleTest("normal")));
        root.addView(button("Schedule loud test alarm", () -> scheduleTest("loud")));
        root.addView(button("Poll pending VPS alarms", this::pollPending));
        root.addView(button("Open usage access settings", () -> new UsageStatsHelper(this).openUsageAccessSettings()));
        root.addView(button("Show last 30 min usage", this::showUsage));

        status = new TextView(this);
        status.setTextColor(0xFFF5F7FA);
        status.setPadding(0, 24, 0, 0);
        status.setText("Device: " + settings.getDeviceId());
        root.addView(status);
        return scroll;
    }

    private EditText input(String hint, String value) {
        EditText editText = new EditText(this);
        editText.setHint(hint);
        editText.setText(value);
        editText.setSingleLine(true);
        editText.setTextColor(0xFFF5F7FA);
        editText.setHintTextColor(0xFFA7B0BA);
        return editText;
    }

    private Button button(String text, Runnable action) {
        Button button = new Button(this);
        button.setText(text);
        button.setAllCaps(false);
        button.setOnClickListener(view -> action.run());
        return button;
    }

    private void saveSettings() {
        settings.setServerUrl(serverUrl.getText().toString());
        settings.setApiToken(apiToken.getText().toString());
        status.setText("Settings saved. Device: " + settings.getDeviceId());
    }

    private void registerDevice() {
        saveSettings();
        new AntirotApiClient(this).registerDevice(message -> runOnUiThread(() -> status.setText(message)));
    }

    private void scheduleTest(String severity) {
        String message = new AlarmScheduler(this).schedule(AlarmJob.test(severity));
        status.setText(message);
    }

    private void pollPending() {
        saveSettings();
        new AntirotApiClient(this).fetchPendingAlarms(new AntirotApiClient.AlarmCallback() {
            @Override
            public void onAlarms(List<AlarmJob> alarms) {
                AlarmScheduler scheduler = new AlarmScheduler(MainActivity.this);
                for (AlarmJob alarm : alarms) {
                    scheduler.schedule(alarm);
                }
                runOnUiThread(() -> status.setText("Scheduled " + alarms.size() + " pending alarm(s)."));
            }

            @Override
            public void onResult(String message) {
                runOnUiThread(() -> status.setText(message));
            }
        });
    }

    private void showUsage() {
        UsageSummary summary = new UsageStatsHelper(this).lastThirtyMinutes();
        status.setText(summary.format());
    }

    private void requestNotificationPermissionIfNeeded() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
                checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[] { Manifest.permission.POST_NOTIFICATIONS }, 10);
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            AlarmManager manager = getSystemService(AlarmManager.class);
            if (manager != null && !manager.canScheduleExactAlarms()) {
                statusMessageLater("Exact alarm permission may be needed for reliable alarms.");
            }
        }
    }

    private void statusMessageLater(String message) {
        getWindow().getDecorView().postDelayed(() -> {
            if (status != null) {
                status.setText(message);
            }
        }, 300);
    }
}
