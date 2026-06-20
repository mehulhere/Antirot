package com.mehulhere.antirot;

import android.content.Context;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileInputStream;
import java.io.OutputStream;
import java.io.InputStreamReader;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

public class AntirotApiClient {
    private final SettingsStore settings;

    public AntirotApiClient(Context context) {
        settings = new SettingsStore(context);
    }

    public void registerDevice(Callback callback) {
        runAsync(() -> {
            JSONObject body = new JSONObject();
            body.put("deviceId", settings.getDeviceId());
            body.put("platform", "android");
            body.put("appVersion", "0.1.0");
            body.put("notificationCapability", "foreground_alarm");
            body.put("usageCapability", "recent_summary");
            request("POST", "/devices/register", body);
            settings.setRegistered(true);
            callback.onResult("Registered device");
        }, callback);
    }

    public void acknowledge(String alarmId, String action, int minutes, Callback callback) {
        runAsync(() -> {
            JSONObject body = new JSONObject();
            body.put("deviceId", settings.getDeviceId());
            body.put("action", action);
            body.put("at", IsoDates.now());
            if (minutes > 0) {
                body.put("minutes", minutes);
            }
            request("POST", "/alarms/" + alarmId + "/" + action, body);
            callback.onResult("Alarm " + action + " sent");
        }, callback);
    }

    public void fetchPendingAlarms(AlarmCallback callback) {
        runAsync(() -> {
            String path = "/alarms/pending?deviceId=" + settings.getDeviceId();
            String text = request("GET", path, null);
            JSONArray array = new JSONArray(text);
            List<AlarmJob> alarms = new ArrayList<>();
            for (int i = 0; i < array.length(); i++) {
                alarms.add(AlarmJob.fromJson(array.getJSONObject(i)));
            }
            callback.onAlarms(alarms);
        }, callback);
    }

    public void fetchRuntimeState(RuntimeStateCallback callback) {
        runAsync(() -> {
            String text = request("GET", "/v1/test/state?userId=admin&deviceId=" + settings.getDeviceId(), null);
            JSONObject body = new JSONObject(text);
            JSONObject runtimeState = body.optJSONObject("runtimeState");
            callback.onRuntimeState(runtimeState == null ? "unknown" : runtimeState.optString("state", "unknown"));
        }, callback);
    }

    public void chat(String message, Callback callback) {
        runAsync(() -> {
            JSONObject body = new JSONObject();
            body.put("message", message);
            String text = request("POST", "/v1/chat", body);
            JSONObject response = new JSONObject(text);
            callback.onResult(response.optString("reply", text));
        }, callback);
    }

    public void transcribeAudio(File file, Callback callback) {
        runAsync(() -> {
            String text = multipartAudioRequest("/v1/speech/transcribe", file);
            JSONObject response = new JSONObject(text);
            callback.onResult(response.optString("text", ""));
        }, callback);
    }

    private String request(String method, String path, JSONObject body) throws Exception {
        String serverUrl = settings.getServerUrl();
        if (serverUrl.isEmpty()) {
            serverUrl = SettingsStore.DEFAULT_SERVER_URL;
        }
        URL url = new URL(serverUrl.replaceAll("/+$", "") + path);
        HttpURLConnection connection = (HttpURLConnection) url.openConnection();
        connection.setRequestMethod(method);
        connection.setConnectTimeout(10_000);
        connection.setReadTimeout(15_000);
        String token = settings.getApiToken();
        if (!token.isEmpty()) {
            connection.setRequestProperty("Authorization", "Bearer " + token);
        }
        if (body != null) {
            connection.setDoOutput(true);
            connection.setRequestProperty("Content-Type", "application/json");
            try (OutputStream output = connection.getOutputStream()) {
                output.write(body.toString().getBytes(StandardCharsets.UTF_8));
            }
        }
        int code = connection.getResponseCode();
        BufferedReader reader = new BufferedReader(new InputStreamReader(
                code < 300 ? connection.getInputStream() : connection.getErrorStream(),
                StandardCharsets.UTF_8
        ));
        StringBuilder builder = new StringBuilder();
        String line;
        while ((line = reader.readLine()) != null) {
            builder.append(line);
        }
        if (code >= 300) {
            throw new IllegalStateException("Server returned " + code + ": " + builder);
        }
        return builder.toString();
    }

    private String multipartAudioRequest(String path, File file) throws Exception {
        String serverUrl = settings.getServerUrl();
        if (serverUrl.isEmpty()) {
            serverUrl = SettingsStore.DEFAULT_SERVER_URL;
        }
        String boundary = "Boundary-" + System.currentTimeMillis();
        URL url = new URL(serverUrl.replaceAll("/+$", "") + path);
        HttpURLConnection connection = (HttpURLConnection) url.openConnection();
        connection.setRequestMethod("POST");
        connection.setConnectTimeout(10_000);
        connection.setReadTimeout(60_000);
        connection.setDoOutput(true);
        connection.setRequestProperty("Content-Type", "multipart/form-data; boundary=" + boundary);
        String token = settings.getApiToken();
        if (!token.isEmpty()) {
            connection.setRequestProperty("Authorization", "Bearer " + token);
        }

        try (OutputStream output = connection.getOutputStream();
             FileInputStream input = new FileInputStream(file)) {
            write(output, "--" + boundary + "\r\n");
            write(output, "Content-Disposition: form-data; name=\"file\"; filename=\"" + file.getName() + "\"\r\n");
            write(output, "Content-Type: audio/mp4\r\n\r\n");
            byte[] buffer = new byte[8192];
            int read;
            while ((read = input.read(buffer)) != -1) {
                output.write(buffer, 0, read);
            }
            write(output, "\r\n--" + boundary + "--\r\n");
        }

        int code = connection.getResponseCode();
        BufferedReader reader = new BufferedReader(new InputStreamReader(
                code < 300 ? connection.getInputStream() : connection.getErrorStream(),
                StandardCharsets.UTF_8
        ));
        StringBuilder builder = new StringBuilder();
        String line;
        while ((line = reader.readLine()) != null) {
            builder.append(line);
        }
        if (code >= 300) {
            throw new IllegalStateException("Server returned " + code + ": " + builder);
        }
        return builder.toString();
    }

    private void write(OutputStream output, String value) throws Exception {
        output.write(value.getBytes(StandardCharsets.UTF_8));
    }

    private void runAsync(ThrowingRunnable runnable, Callback callback) {
        new Thread(() -> {
            try {
                runnable.run();
            } catch (Exception error) {
                callback.onResult("Failed: " + error.getMessage());
            }
        }).start();
    }

    public interface Callback {
        void onResult(String message);
    }

    public interface AlarmCallback extends Callback {
        void onAlarms(List<AlarmJob> alarms);
    }

    public interface RuntimeStateCallback extends Callback {
        void onRuntimeState(String state);
    }

    private interface ThrowingRunnable {
        void run() throws Exception;
    }
}
