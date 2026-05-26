package com.mehulhere.antirot;

import android.app.Application;

public class AntirotApplication extends Application {
    @Override
    public void onCreate() {
        super.onCreate();
        NotificationHelper.ensureChannels(this);
    }
}
