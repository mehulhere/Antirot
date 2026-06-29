package com.mehulhere.antirot;

import android.Manifest;
import android.app.AlarmManager;
import android.app.AlertDialog;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.media.MediaPlayer;
import android.media.RingtoneManager;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.Gravity;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import java.io.File;
import java.util.ArrayDeque;
import java.util.List;

public class MainActivity extends android.app.Activity {
    private static final int PICK_ALARM_SOUND_REQUEST = 42;
    private static final long QUICK_ACTION_REFRESH_MS = 60_000L;

    private SettingsStore settings;
    private TextView status;
    private EditText serverUrl;
    private EditText apiToken;
    private EditText coachDraft;
    private TextView coachTranscript;
    private Button voiceButton;
    private LinearLayout quickActionRow;
    private LinearLayout root;
    private boolean showDeveloperSettings = false;
    private boolean namePromptSent = false;
    private String onboardingName = "";
    private String runtimeState = "unknown";
    private final StringBuilder coachLog = new StringBuilder();
    private final ArrayDeque<QueuedChatMessage> chatQueue = new ArrayDeque<>();
    private final ArrayDeque<File> speechQueue = new ArrayDeque<>();
    private boolean chatQueueProcessing = false;
    private boolean speechQueueProcessing = false;
    private GentleVoiceRecorder voiceRecorder;
    private final Handler quickActionRefreshHandler = new Handler(Looper.getMainLooper());
    private final Runnable quickActionRefreshRunnable = new Runnable() {
        @Override
        public void run() {
            renderQuickActions();
            quickActionRefreshHandler.postDelayed(this, QUICK_ACTION_REFRESH_MS);
        }
    };

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        settings = new SettingsStore(this);
        voiceRecorder = new GentleVoiceRecorder(this);
        NotificationHelper.ensureChannels(this);
        setContentView(buildView());
        requestNotificationPermissionIfNeeded();
        refreshRuntimeState();
    }

    @Override
    protected void onResume() {
        super.onResume();
        quickActionRefreshHandler.removeCallbacks(quickActionRefreshRunnable);
        quickActionRefreshHandler.post(quickActionRefreshRunnable);
    }

    @Override
    protected void onPause() {
        quickActionRefreshHandler.removeCallbacks(quickActionRefreshRunnable);
        super.onPause();
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

        renderCoachSurface();

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

    private void renderCoachSurface() {
        TextView heading = new TextView(this);
        heading.setText("Coach chat");
        heading.setTextColor(0xFFF5F7FA);
        heading.setTextSize(22);
        heading.setPadding(0, 18, 0, 8);
        root.addView(heading);

        TextView state = new TextView(this);
        state.setText("State: " + runtimeState);
        state.setTextColor(0xFFA7B0BA);
        state.setPadding(0, 0, 0, 8);
        root.addView(state);

        quickActionRow = new LinearLayout(this);
        quickActionRow.setOrientation(LinearLayout.VERTICAL);
        root.addView(quickActionRow);
        renderQuickActions();

        coachTranscript = new TextView(this);
        coachTranscript.setText(coachLog.length() == 0
                ? "Antirot is ready. Speak or send a short check-in."
                : coachLog.toString());
        coachTranscript.setTextColor(0xFFF5F7FA);
        coachTranscript.setPadding(0, 10, 0, 8);
        root.addView(coachTranscript);

        voiceButton = button("Speak", this::toggleVoice);
        root.addView(voiceButton);

        coachDraft = input("Speak or type the user's next message", "");
        root.addView(coachDraft);
        root.addView(button("Send to coach", this::sendDraftToCoach));
        root.addView(button("Refresh state options", this::refreshRuntimeState));
    }

    private void renderQuickActions() {
        if (quickActionRow == null) {
            return;
        }
        quickActionRow.removeAllViews();
        List<CoachQuickAction> actions = CoachQuickAction.forState(runtimeState);
        if (actions.isEmpty()) {
            TextView empty = new TextView(this);
            empty.setText("No quick actions for this state.");
            empty.setTextColor(0xFFA7B0BA);
            quickActionRow.addView(empty);
            return;
        }
        for (CoachQuickAction action : actions) {
            quickActionRow.addView(button(action.title, () -> handleQuickAction(action)));
        }
    }

    private void handleQuickAction(CoachQuickAction action) {
        if (action.fillsDraft) {
            coachDraft.setText(action.message);
            coachDraft.setSelection(coachDraft.getText().length());
            status.setText("Finish the sentence.");
            return;
        }
        sendCoachMessage(action.message);
    }

    private void sendDraftToCoach() {
        String text = coachDraft.getText().toString().trim();
        if (text.isEmpty()) {
            return;
        }
        coachDraft.setText("");
        sendCoachMessage(text);
    }

    private void sendCoachMessage(String message) {
        sendCoachMessage(message, message);
    }

    private void sendCoachMessage(String message, String visibleMessage) {
        String trimmed = message == null ? "" : message.trim();
        if (trimmed.isEmpty()) {
            return;
        }
        String visible = visibleMessage == null ? null : visibleMessage.trim();
        chatQueue.add(new QueuedChatMessage(trimmed, visible == null || visible.isEmpty() ? null : visible));
        processChatQueue();
    }

    private void processChatQueue() {
        if (chatQueueProcessing || chatQueue.isEmpty()) {
            return;
        }
        chatQueueProcessing = true;
        QueuedChatMessage queued = chatQueue.poll();
        String message = queued.message;
        if (queued.visibleMessage != null) {
            appendCoach("you", queued.visibleMessage);
        }
        status.setText(chatQueue.isEmpty() ? "Thinking" : "Thinking (" + chatQueue.size() + " queued)");
        new AntirotApiClient(this).chat(message, reply -> runOnUiThread(() -> {
            if (reply.startsWith("Failed:")) {
                status.setText(reply);
                appendCoach("system", reply);
                chatQueueProcessing = false;
                processChatQueue();
                return;
            }
            appendCoach("coach", reply);
            status.setText("Ready");
            chatQueueProcessing = false;
            refreshRuntimeState();
            processChatQueue();
        }));
    }

    private void appendCoach(String speaker, String message) {
        coachLog.append(speaker).append(": ").append(message).append("\n\n");
        if (coachTranscript != null) {
            coachTranscript.setText(coachLog.toString());
        }
    }

    private void toggleVoice() {
        if (voiceRecorder.isRecording()) {
            File file = voiceRecorder.stop();
            voiceButton.setText("Speak");
            if (file != null) {
                transcribeAndSend(file);
            }
            return;
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M &&
                checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[] { Manifest.permission.RECORD_AUDIO }, 11);
            status.setText("Grant microphone permission, then tap Speak again.");
            return;
        }

        try {
            voiceRecorder.start(file -> runOnUiThread(() -> {
                voiceButton.setText("Speak");
                transcribeAndSend(file);
            }));
            voiceButton.setText("Stop");
            status.setText("Listening: 10s minimum, gentle silence cutoff.");
        } catch (Exception error) {
            status.setText("Voice failed: " + error.getMessage());
        }
    }

    private void transcribeAndSend(File file) {
        speechQueue.add(file);
        processSpeechQueue();
    }

    private void processSpeechQueue() {
        if (speechQueueProcessing || speechQueue.isEmpty()) {
            return;
        }
        speechQueueProcessing = true;
        File file = speechQueue.poll();
        status.setText(speechQueue.isEmpty() ? "Transcribing" : "Transcribing (" + speechQueue.size() + " queued)");
        new AntirotApiClient(this).transcribeAudio(file, text -> runOnUiThread(() -> {
            file.delete();
            String trimmed = text == null ? "" : text.trim();
            if (trimmed.startsWith("Failed:")) {
                status.setText(trimmed);
                appendCoach("system", trimmed);
                file.delete();
                speechQueueProcessing = false;
                processSpeechQueue();
                return;
            }
            if (trimmed.isEmpty()) {
                status.setText("No speech detected.");
                file.delete();
                speechQueueProcessing = false;
                processSpeechQueue();
                return;
            }
            appendVoiceMessage(file);
            sendCoachMessage(trimmed, null);
            speechQueueProcessing = false;
            processSpeechQueue();
        }));
    }

    private void refreshRuntimeState() {
        new AntirotApiClient(this).fetchRuntimeState(new AntirotApiClient.RuntimeStateCallback() {
            @Override
            public void onRuntimeState(String state) {
                runOnUiThread(() -> {
                    runtimeState = state == null || state.trim().isEmpty() ? "unknown" : state.trim();
                    setContentView(buildView());
                    status.setText("State refreshed: " + runtimeState);
                    promptNameIfNeeded();
                });
            }

            @Override
            public void onResult(String message) {
                runOnUiThread(() -> {
                    runtimeState = "unknown";
                    renderQuickActions();
                    if (status != null) {
                        status.setText("State unavailable: " + message);
                    }
                });
            }
        });
    }

    private void promptNameIfNeeded() {
        if (namePromptSent || (!"onboarding".equals(runtimeState) && !"unknown".equals(runtimeState))) {
            return;
        }
        EditText input = input("Name", onboardingName);
        new AlertDialog.Builder(this)
                .setTitle("Your name")
                .setMessage("The rest can be handled by voice.")
                .setView(input)
                .setPositiveButton("Continue", (dialog, which) -> {
                    String name = input.getText().toString().trim();
                    if (name.isEmpty()) {
                        status.setText("Add the user's name first.");
                        promptNameIfNeeded();
                        return;
                    }
                    onboardingName = name;
                    namePromptSent = true;
                    sendCoachMessage(onboardingMessage(name), null);
                })
                .show();
    }

    private String onboardingMessage(String name) {
        return "The user just shared their name during onboarding. Return the deterministic Antirot first onboarding message exactly.\n" +
                "First onboarding message: I’m Antirot. I’ve coached plenty of people like you: smart, intense, full of plans, and somehow still one bad hour away from drifting off the thing they claim matters.\n\n" +
                "So let’s see what you’ve got. I need to build your profile. Give me a gist of your long-term and short-term goals. You can update this later as well. Because obviously, ambition is not a gift everyone has.\n\n" +
                "Tell me what your day looks like and what you’re planning to get done today.\n" +
                "Name: " + name;
    }

    private void appendVoiceMessage(File file) {
        appendCoach("you", "Voice message");
        if (root == null) {
            return;
        }
        Button playButton = button("Play voice message", () -> playVoiceMessage(file));
        root.addView(playButton);
    }

    private void playVoiceMessage(File file) {
        try {
            MediaPlayer player = new MediaPlayer();
            player.setDataSource(file.getAbsolutePath());
            player.setOnCompletionListener(MediaPlayer::release);
            player.prepare();
            player.start();
        } catch (Exception error) {
            status.setText("Could not play voice message: " + error.getMessage());
        }
    }

    private static final class QueuedChatMessage {
        final String message;
        final String visibleMessage;

        QueuedChatMessage(String message, String visibleMessage) {
            this.message = message;
            this.visibleMessage = visibleMessage;
        }
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
