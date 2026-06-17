package com.mehulhere.antirot;

import android.Manifest;
import android.app.AlarmManager;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.media.RingtoneManager;
import android.net.Uri;
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
    private static final int PICK_ALARM_SOUND_REQUEST = 42;

    private SettingsStore settings;
    private TextView status;
    private EditText serverUrl;
    private EditText apiToken;
    private LinearLayout root;
    private boolean showDeveloperSettings = false;

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
        root = new LinearLayout(this);
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

        TextView backend = new TextView(this);
        backend.setText("Backend: api.antirot.org");
        backend.setTextColor(0xFFA7B0BA);
        backend.setPadding(0, 0, 0, 16);
        root.addView(backend);

        root.addView(button("Register device", this::registerDevice));
        root.addView(button("Reset local login", this::resetBackendSession));
        root.addView(button("Use auto normal/loud sounds", () -> setAlarmSoundMode(SettingsStore.SOUND_AUTO)));
        root.addView(button("Use bundled normal sound", () -> setAlarmSoundMode(SettingsStore.SOUND_NORMAL)));
        root.addView(button("Use bundled loud sound", () -> setAlarmSoundMode(SettingsStore.SOUND_LOUD)));
        root.addView(button("Choose custom alarm sound", this::chooseAlarmSound));
        root.addView(button("Schedule normal test alarm", () -> scheduleTest("normal")));
        root.addView(button("Schedule loud test alarm", () -> scheduleTest("loud")));
        root.addView(button("Poll pending VPS alarms", this::pollPending));
        root.addView(button("Open usage access settings", () -> new UsageStatsHelper(this).openUsageAccessSettings()));
        root.addView(button("Show last 30 min usage", this::showUsage));
        root.addView(button("Developer settings", this::toggleDeveloperSettings));

        status = new TextView(this);
        status.setTextColor(0xFFF5F7FA);
        status.setPadding(0, 24, 0, 0);
        status.setText(statusText());
        root.addView(status);
        renderDeveloperSettings();
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
        if (serverUrl != null) {
            settings.setServerUrl(serverUrl.getText().toString());
        }
        if (apiToken != null) {
            settings.setApiToken(apiToken.getText().toString());
        }
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

    private void chooseAlarmSound() {
        Intent intent = new Intent(RingtoneManager.ACTION_RINGTONE_PICKER);
        intent.putExtra(RingtoneManager.EXTRA_RINGTONE_TYPE, RingtoneManager.TYPE_ALARM);
        intent.putExtra(RingtoneManager.EXTRA_RINGTONE_TITLE, "Choose Antirot alarm sound");
        intent.putExtra(RingtoneManager.EXTRA_RINGTONE_SHOW_SILENT, false);
        intent.putExtra(RingtoneManager.EXTRA_RINGTONE_SHOW_DEFAULT, true);
        String existing = settings.getAlarmSoundUri();
        Uri current = existing.isEmpty() ? RingtoneManager.getDefaultUri(RingtoneManager.TYPE_ALARM) : Uri.parse(existing);
        intent.putExtra(RingtoneManager.EXTRA_RINGTONE_EXISTING_URI, current);
        startActivityForResult(intent, PICK_ALARM_SOUND_REQUEST);
    }

    private void clearAlarmSound() {
        setAlarmSoundMode(SettingsStore.SOUND_AUTO);
    }

    private void setAlarmSoundMode(String mode) {
        settings.setAlarmSoundMode(mode);
        if (!SettingsStore.SOUND_CUSTOM.equals(mode)) {
            settings.setAlarmSoundUri("");
        }
        status.setText("Alarm sound changed. " + statusText());
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

    private void toggleDeveloperSettings() {
        showDeveloperSettings = !showDeveloperSettings;
        serverUrl = null;
        apiToken = null;
        setContentView(buildView());
    }

    private void renderDeveloperSettings() {
        if (root == null) {
            return;
        }
        if (!showDeveloperSettings) {
            return;
        }
        if (serverUrl != null || apiToken != null) {
            return;
        }

        TextView heading = new TextView(this);
        heading.setText("Developer settings");
        heading.setTextColor(0xFFF5F7FA);
        heading.setPadding(0, 28, 0, 8);
        root.addView(heading);

        serverUrl = input("Antirot backend URL", settings.getServerUrl());
        apiToken = input("API token from /etc/antirot/backend.env", settings.getApiToken());
        root.addView(serverUrl);
        root.addView(apiToken);
        root.addView(button("Save developer settings", this::saveSettings));
        root.addView(button("Reset backend to api.antirot.org", this::resetBackendUrl));
        root.addView(button("Reset backend session", this::resetBackendSession));
    }

    private void resetBackendUrl() {
        settings.setServerUrl(SettingsStore.DEFAULT_SERVER_URL);
        if (serverUrl != null) {
            serverUrl.setText(SettingsStore.DEFAULT_SERVER_URL);
        }
        status.setText("Backend reset. " + statusText());
    }

    private void resetBackendSession() {
        settings.resetBackendSession();
        serverUrl = null;
        apiToken = null;
        status.setText("Backend session reset. Sign in again when Google login is wired here.");
        setContentView(buildView());
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != PICK_ALARM_SOUND_REQUEST || resultCode != RESULT_OK || data == null) {
            return;
        }
        Uri picked = data.getParcelableExtra(RingtoneManager.EXTRA_RINGTONE_PICKED_URI);
        if (picked == null) {
            status.setText("No alarm sound selected.");
            return;
        }
        settings.setAlarmSoundUri(picked.toString());
        settings.setAlarmSoundMode(SettingsStore.SOUND_CUSTOM);
        status.setText("Selected alarm sound. " + statusText());
    }

    private String statusText() {
        String sound = NotificationHelper.soundLabel(this);
        return "Device: " + settings.getDeviceId() + "\nAlarm sound: " + sound;
    }

    private void statusMessageLater(String message) {
        getWindow().getDecorView().postDelayed(() -> {
            if (status != null) {
                status.setText(message);
            }
        }, 300);
    }
}
