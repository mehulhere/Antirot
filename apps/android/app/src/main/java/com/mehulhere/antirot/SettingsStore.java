package com.mehulhere.antirot;

import android.content.Context;
import android.content.SharedPreferences;

import androidx.security.crypto.EncryptedSharedPreferences;
import androidx.security.crypto.MasterKey;

import java.util.UUID;

public class SettingsStore {
    public static final String DEFAULT_SERVER_URL = "https://api.antirot.org";

    private static final String LEGACY_PREFS = "antirot";
    private static final String SECURE_PREFS = "antirot_secure";
    private static final String MIGRATED = "encrypted_migration_complete";
    private static final String SERVER_URL = "server_url";
    private static final String API_TOKEN = "api_token";
    private static final String DEVICE_ID = "device_id";
    private static final String REGISTERED = "registered";
    private static final String ALARM_SOUND_URI = "alarm_sound_uri";
    private static final String ALARM_SOUND_MODE = "alarm_sound_mode";

    public static final String SOUND_AUTO = "auto";
    public static final String SOUND_NORMAL = "normal";
    public static final String SOUND_LOUD = "loud";
    public static final String SOUND_CUSTOM = "custom";

    private final SharedPreferences preferences;

    public SettingsStore(Context context) {
        try {
            MasterKey masterKey = new MasterKey.Builder(context)
                    .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                    .build();
            preferences = EncryptedSharedPreferences.create(
                    context,
                    SECURE_PREFS,
                    masterKey,
                    EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                    EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
            );
        } catch (Exception error) {
            throw new IllegalStateException("Secure credential storage is unavailable", error);
        }
        migrateLegacyPreferences(context);
        if (getDeviceId().isEmpty()) {
            preferences.edit().putString(DEVICE_ID, UUID.randomUUID().toString()).apply();
        }
    }

    private void migrateLegacyPreferences(Context context) {
        if (preferences.getBoolean(MIGRATED, false)) {
            return;
        }
        SharedPreferences legacy = context.getSharedPreferences(LEGACY_PREFS, Context.MODE_PRIVATE);
        SharedPreferences.Editor encrypted = preferences.edit();
        copyString(legacy, encrypted, SERVER_URL);
        copyString(legacy, encrypted, API_TOKEN);
        copyString(legacy, encrypted, DEVICE_ID);
        copyString(legacy, encrypted, ALARM_SOUND_URI);
        copyString(legacy, encrypted, ALARM_SOUND_MODE);
        if (legacy.contains(REGISTERED)) {
            encrypted.putBoolean(REGISTERED, legacy.getBoolean(REGISTERED, false));
        }
        encrypted.putBoolean(MIGRATED, true);
        if (!encrypted.commit()) {
            throw new IllegalStateException("Could not migrate credentials into secure storage");
        }
        if (!legacy.edit().clear().commit()) {
            throw new IllegalStateException("Could not delete legacy plaintext credentials");
        }
    }

    private void copyString(
            SharedPreferences source,
            SharedPreferences.Editor destination,
            String key
    ) {
        if (source.contains(key)) {
            destination.putString(key, source.getString(key, ""));
        }
    }

    public String getServerUrl() {
        String value = preferences.getString(SERVER_URL, DEFAULT_SERVER_URL);
        if (value == null || value.trim().isEmpty()) {
            return DEFAULT_SERVER_URL;
        }
        String normalized = value.trim();
        if (!BuildConfig.DEBUG && !normalized.startsWith("https://")) {
            return DEFAULT_SERVER_URL;
        }
        return normalized;
    }

    public void setServerUrl(String value) {
        String normalized = value == null || value.trim().isEmpty() ? DEFAULT_SERVER_URL : value.trim();
        if (!BuildConfig.DEBUG && !normalized.startsWith("https://")) {
            normalized = DEFAULT_SERVER_URL;
        }
        preferences.edit().putString(SERVER_URL, normalized).apply();
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

    public String getAlarmSoundMode() {
        return preferences.getString(ALARM_SOUND_MODE, SOUND_AUTO);
    }

    public void setAlarmSoundMode(String value) {
        preferences.edit().putString(ALARM_SOUND_MODE, value == null ? SOUND_AUTO : value).apply();
    }

    public void resetBackendSession() {
        preferences.edit()
                .putString(SERVER_URL, DEFAULT_SERVER_URL)
                .putString(API_TOKEN, "")
                .putString(DEVICE_ID, UUID.randomUUID().toString())
                .putBoolean(REGISTERED, false)
                .apply();
    }
}
