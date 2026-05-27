package com.mehulhere.antirot;

import android.content.Context;
import android.content.SharedPreferences;

import java.util.UUID;

public class SettingsStore {
    private static final String PREFS = "antirot";
    private static final String SERVER_URL = "server_url";
    private static final String API_TOKEN = "api_token";
    private static final String DEVICE_ID = "device_id";
    private static final String REGISTERED = "registered";
    private static final String ALARM_SOUND_URI = "alarm_sound_uri";

    private final SharedPreferences preferences;

    public SettingsStore(Context context) {
        preferences = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
        if (getDeviceId().isEmpty()) {
            preferences.edit().putString(DEVICE_ID, UUID.randomUUID().toString()).apply();
        }
    }

    public String getServerUrl() {
        return preferences.getString(SERVER_URL, "");
    }

    public void setServerUrl(String value) {
        preferences.edit().putString(SERVER_URL, value.trim()).apply();
    }

    public String getApiToken() {
        return preferences.getString(API_TOKEN, "");
    }

    public void setApiToken(String value) {
        preferences.edit().putString(API_TOKEN, value.trim()).apply();
    }

    public String getDeviceId() {
        return preferences.getString(DEVICE_ID, "");
    }

    public boolean isRegistered() {
        return preferences.getBoolean(REGISTERED, false);
    }

    public void setRegistered(boolean registered) {
        preferences.edit().putBoolean(REGISTERED, registered).apply();
    }

    public String getAlarmSoundUri() {
        return preferences.getString(ALARM_SOUND_URI, "");
    }

    public void setAlarmSoundUri(String value) {
        preferences.edit().putString(ALARM_SOUND_URI, value == null ? "" : value).apply();
    }
}
